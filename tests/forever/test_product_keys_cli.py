"""GPL-3.0-or-later. Explicit retained synthetic key CLI acceptance, 2026-10-11.

Set RCE_PRODUCT_BINARY and RCE_PRODUCT_KEYS_CASES outside Git. All key material
is authored public test data. Every CLI has syscall network denial and traces.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import unittest
import product_fixture as plain
import product_keys_fixture as fixture

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_KEYS_CASES')


@unittest.skipUnless(BINARY and CASES, 'explicit external synthetic product keys run required')
class ProductKeysCli(unittest.TestCase):
    def setUp(self):
        self.root = Path(CASES) / self._testMethodName
        self.root.mkdir(parents=True, mode=0o700)
        self.assertFalse(any((p / '.git').exists() for p in self.root.resolve().parents))
        self.calls = 0

    def prepare(self, tile=False, components='db2,gt,cameras', build=70205, locale='enUS', metadata=True):
        self.payloads = plain.files(tile)
        self.client = self.root / 'client'
        self.offsets = fixture.store(self.client, self.payloads, build, locale, metadata)
        self.plan = self.root / 'plan.tsv'
        self.plan.write_text(fixture.plan(components, build, locale))
        self.keys = self.root / 'provided.tsv'
        self.keys.write_text(fixture.keyfile())
        self.output = self.root / 'output'

    def options(self, command):
        args = ['--plan', str(self.plan)]
        if command in ('extract', 'resume'):
            args += ['--client', str(self.client), '--output', str(self.output), '--keys', str(self.keys)]
        elif command == 'verify':
            args += ['--output', str(self.output)]
        return args

    def invoke(self, command, options=None, code=0, kill=None):
        self.calls += 1
        trace = self.root / f'{self.calls:02}-{command}.network'
        options = self.options(command) if options is None else options
        args = ['strace', '-f', '-qq', '-e', 'trace=network,fsync' if kill else 'trace=network',
                '-e', 'inject=network:error=ENETUNREACH', '-o', str(trace)]
        if kill:
            args += ['-e', f'inject=fsync:signal=SIGKILL:when={kill}']
        result = subprocess.run(args + [BINARY, command] + options, capture_output=True, text=True, timeout=240)
        (self.root / f'{self.calls:02}-{command}.stdout').write_text(result.stdout)
        (self.root / f'{self.calls:02}-{command}.stderr').write_text(result.stderr)
        for text in (result.stdout, result.stderr):
            self.assertNotIn(str(self.root), text)
            self.assertNotIn('forever-tact-keys', text)
            for name, key in fixture.KEYS.items():
                self.assertNotIn(f'{name:016x}', text)
                self.assertNotIn(key.hex(), text)
        if kill:
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('SIGKILL', trace.read_text())
            # Only fsync is traced in addition to denied network; no network
            # syscall may have been attempted even in an interrupted run.
            self.assertNotIn('ENETUNREACH', trace.read_text())
            return None
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertEqual(trace.read_text(), '')
        report = json.loads(result.stdout if code == 0 else result.stderr)
        if report.get('code') != 'product_usage':
            self.assertEqual(report['contract'], 'forever-product-v2')
        return report

    def assert_no_material_in_package(self):
        for name in ('forever-package.json', '.state/context.json'):
            text = (self.output / name).read_text()
            self.assertNotIn(str(self.keys), text)
            self.assertNotIn('forever-tact-keys', text)
            for key in fixture.KEYS.values():
                self.assertNotIn(key.hex(), text)

    def inventory(self):
        manifest = json.loads((self.output / 'forever-package.json').read_text())
        return manifest['artifacts']

    def test_all_six_actual_tile_matches_plain_v1_bytes(self):
        self.prepare(tile=True, components='db2,gt,cameras,maps,vmaps,mmaps')
        self.invoke('extract'); self.invoke('verify')
        encrypted_inventory = self.inventory()
        self.assertEqual(len(encrypted_inventory), 31)
        self.assertTrue(list((self.output / 'mmaps').glob('*.mmtile')))
        self.assert_no_material_in_package()
        plain_client = self.root / 'plain-client'
        plain.store(plain_client, self.payloads)
        v1 = self.root / 'v1.tsv'; v1.write_text(plain.plan())
        # v1 output contract deliberately differs, so invoke the same retained
        # binary directly through the syscall-denied helper with relaxed label.
        self.plan = v1; self.client = plain_client; self.output = self.root / 'plain-output'
        self.invoke_v1('extract', ['--plan', str(v1), '--client', str(plain_client), '--output', str(self.output)])
        self.assertEqual(encrypted_inventory, self.inventory())

    def invoke_v1(self, command, options, code=0):
        self.calls += 1
        trace = self.root / f'{self.calls:02}-{command}.network'
        result = subprocess.run(['strace', '-f', '-qq', '-e', 'trace=network', '-e',
            'inject=network:error=ENETUNREACH', '-o', str(trace), BINARY, command, *options],
            capture_output=True, text=True, timeout=240)
        (self.root / f'{self.calls:02}-{command}.stdout').write_text(result.stdout)
        (self.root / f'{self.calls:02}-{command}.stderr').write_text(result.stderr)
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertEqual(trace.read_text(), '')
        report = json.loads(result.stdout if code == 0 else result.stderr)
        self.assertEqual(report['contract'], 'forever-product-v1')
        return report

    def test_metadata_and_70170_esES(self):
        self.prepare(build=70170, locale='esES')
        report = self.invoke('extract'); self.invoke('verify')
        self.assertEqual(report['package']['context']['build'], 70170)
        self.assertEqual(report['package']['context']['locale'], 'esES')
        self.assert_no_material_in_package()

    def test_plan_only_inspect_needs_no_keys(self):
        self.prepare()
        self.keys.unlink()
        self.assertEqual(self.invoke('inspect')['status'], 'planned')
        self.assertFalse(self.output.exists())

    def test_client_inspect_with_keys(self):
        self.prepare()
        report = self.invoke('inspect', ['--plan', str(self.plan), '--client', str(self.client), '--keys', str(self.keys)])
        self.assertEqual(report['status'], 'admitted')
        self.assertFalse(report['availability_checked'])
        self.assertFalse(self.output.exists())

    def test_required_keys_and_illegal_verify_inspect_options(self):
        self.prepare()
        args = self.options('extract')[:-2]
        self.assertEqual(self.invoke('extract', args, 2)['code'], 'product_keys_required')
        args = ['--plan', str(self.plan), '--client', str(self.client)]
        self.assertEqual(self.invoke('inspect', args, 2)['code'], 'product_keys_required')
        self.assertEqual(self.invoke('inspect', ['--plan', str(self.plan), '--keys', str(self.keys)], 2)['code'], 'product_usage')
        self.assertEqual(self.invoke('verify', self.options('verify') + ['--keys', str(self.keys)], 2)['code'], 'product_usage')
        self.assertFalse(self.output.exists())

    def test_v1_forbids_key_option_before_opening_fifo(self):
        self.prepare()
        self.plan.write_text(plain.plan('db2,gt,cameras'))
        self.keys.unlink(); os.mkfifo(self.keys)
        report = self.invoke_v1('extract', self.options('extract'), 2)
        self.assertEqual(report['code'], 'product_keys_forbidden')
        self.assertFalse(self.output.exists())

    def test_v1_rejects_encrypted_metadata_without_output(self):
        self.prepare()
        self.plan.write_text(plain.plan('db2,gt,cameras'))
        report = self.invoke_v1('extract', self.options('extract')[:-2], 9)
        self.assertEqual(report['code'], 'local_store_no_verified_encoding')
        self.assertFalse(self.output.exists())

    def test_v1_rejects_encrypted_selected_payload(self):
        self.prepare(metadata=False)
        self.plan.write_text(plain.plan('db2,gt,cameras'))
        report = self.invoke_v1('extract', self.options('extract')[:-2], 9)
        self.assertEqual(report['code'], 'local_store_no_verified_encoding')
        self.assertEqual(report['component'], 'db2')
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.assertFalse(list((self.output / 'dbc').rglob('*.db2')))
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_wrong_metadata_key_rejects_before_output(self):
        self.prepare()
        entries = dict(fixture.KEYS); entries[0x101] = bytes([0xef]) * 16
        self.keys.write_text(fixture.keyfile(entries))
        self.assertEqual(self.invoke('extract', code=9)['code'], 'local_store_no_verified_encoding')
        self.assertFalse(self.output.exists())

    def test_missing_metadata_key_rejects_before_output(self):
        self.prepare()
        self.keys.write_text(fixture.keyfile({0x202: fixture.KEYS[0x202]}))
        self.assertEqual(self.invoke('extract', code=9)['code'], 'local_store_no_verified_encoding')
        self.assertFalse(self.output.exists())

    def test_wrong_selected_db2_key_leaves_no_affected_payload(self):
        self.prepare()
        entries = dict(fixture.KEYS); entries[0x202] = bytes([0xef]) * 16
        self.keys.write_text(fixture.keyfile(entries))
        report = self.invoke('extract', code=9)
        self.assertEqual(report['code'], 'local_store_no_verified_encoding')
        self.assertEqual(report['component'], 'db2')
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.assertFalse(list((self.output / 'dbc').rglob('LiquidType.db2')))
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.invoke('verify', code=8)

    def test_inspect_does_not_assert_selected_key_availability(self):
        self.prepare()
        self.keys.write_text(fixture.keyfile({name: key for name, key in fixture.KEYS.items() if name != 0x202}))
        report = self.invoke('inspect', ['--plan', str(self.plan), '--client', str(self.client), '--keys', str(self.keys)])
        self.assertEqual(report['status'], 'admitted')
        self.assertFalse(report['availability_checked'])
        self.assertEqual(self.invoke('extract', code=9)['code'], 'local_store_no_verified_encoding')
        self.assertFalse(list((self.output / 'dbc').rglob('LiquidType.db2')))

    def test_tampered_headerless_cipher_rejects_complete_ckey(self):
        self.prepare()
        content = self.payloads[301]
        at, size = self.offsets[hashlib.md5(content).digest()]
        archive = self.client / 'Data/data/data.000'
        data = bytearray(archive.read_bytes())
        self.assertEqual(data[at + 30:at + 39], b'BLTE\0\0\0\0E')
        data[at + size - 1] ^= 1
        archive.write_bytes(data)
        report = self.invoke('extract', code=9)
        self.assertEqual(report['code'], 'local_store_no_verified_encoding')
        self.assertEqual(report['component'], 'cameras')
        self.assertFalse(list((self.output / 'cameras').glob('*')))
        self.invoke('verify', code=8)

    def test_key_input_fifo_directory_symlink_and_oversize(self):
        self.prepare()
        original = self.keys
        paths = [(self.root / 'fifo', 'product_keys_input_type'),
                 (self.root / 'directory', 'product_keys_input_type'),
                 (self.root / 'alias', 'product_keys_input_io'),
                 (self.root / 'huge', 'product_keys_input_limit')]
        os.mkfifo(paths[0][0]); paths[1][0].mkdir(); paths[2][0].symlink_to(original)
        paths[3][0].write_bytes(b'x' * (256 * 1024 + 1))
        for path, code in paths:
            self.keys = path
            self.assertEqual(self.invoke('extract', code=9)['code'], code)
            self.assertFalse(self.output.exists())

    def test_key_parser_rejects_empty_order_and_format(self):
        self.prepare()
        for text, code in [('', 'tact_keys_format'), ('forever-tact-keys\t1\n', 'tact_keys_format'),
            (fixture.keyfile().replace('\n', '\r\n'), 'tact_keys_format'),
            ('forever-tact-keys\t1\n' + ''.join(reversed(fixture.keyfile().splitlines(True)[1:])), 'tact_keys_order')]:
            self.keys.write_text(text)
            self.assertEqual(self.invoke('extract', code=9)['code'], code)
            self.assertFalse(self.output.exists())

    def test_real_sigkill_resume_manifest_exact(self):
        self.prepare()
        self.invoke('extract', kill=25)
        self.invoke('verify', code=8)
        self.invoke('resume'); self.invoke('verify')
        recovered = (self.output / 'forever-package.json').read_bytes()
        self.output = self.root / 'baseline'; self.invoke('extract')
        self.assertEqual(recovered, (self.output / 'forever-package.json').read_bytes())

    def test_resume_v1_context_rejects_and_different_supplied_set_succeeds(self):
        # Metadata remains plain so the wrong-v1 attempt reaches package context.
        self.prepare(metadata=False)
        original = self.plan.read_text()
        self.invoke('extract', kill=25)
        self.plan.write_text(plain.plan('db2,gt,cameras'))
        report = self.invoke_v1('resume', self.options('resume')[:-2], 9)
        self.assertEqual(report['code'], 'package_context')
        self.plan.write_text(original)
        entries = dict(fixture.KEYS); entries[0x505] = bytes([0x55]) * 16
        self.keys.write_text(fixture.keyfile(entries))
        self.invoke('resume'); self.invoke('verify')
        self.assert_no_material_in_package()

    def test_resume_skips_sealed_db2_with_key_removed(self):
        self.prepare()
        self.output = self.root / 'baseline'; self.invoke('extract')
        baseline = (self.output / 'forever-package.json').read_bytes()
        self.output = self.root / 'output'; self.invoke('extract', kill=70)
        self.assertTrue((self.output / '.state/sealed/db2').exists())
        self.assertFalse((self.output / '.state/sealed/gt').exists())
        self.keys.write_text(fixture.keyfile({name: key for name, key in fixture.KEYS.items() if name != 0x202}))
        self.invoke('resume'); self.invoke('verify')
        self.assertEqual(baseline, (self.output / 'forever-package.json').read_bytes())

    def test_resume_same_ckey_alternate_prefix_candidate(self):
        self.prepare()
        self.output = self.root / 'baseline'; self.invoke('extract')
        baseline = (self.output / 'forever-package.json').read_bytes()
        self.output = self.root / 'output'; self.invoke('extract', kill=25)
        content = self.payloads[plain.IDS['LiquidType']]
        original_offset, _ = self.offsets[hashlib.md5(content).digest()]
        fixture.add_candidate(self.client, content, original_offset, 0x303)
        self.keys.write_text(fixture.keyfile({name: key for name, key in fixture.KEYS.items() if name != 0x202}))
        self.invoke('resume'); self.invoke('verify')
        self.assertEqual(baseline, (self.output / 'forever-package.json').read_bytes())

    def test_resume_protects_keyfile_inside_output(self):
        self.prepare()
        self.invoke('extract', kill=25)
        original = (self.output / '.state/context.json').read_bytes()
        self.keys = self.output / 'provided.tsv'; self.keys.write_text(fixture.keyfile())
        self.assertEqual(self.invoke('resume', code=9)['code'], 'package_path')
        self.assertEqual(original, (self.output / '.state/context.json').read_bytes())


if __name__ == '__main__':
    unittest.main()
