"""GPL-3.0-or-later. Retained explicit synthetic disk product acceptance.

Set RCE_PRODUCT_BINARY and RCE_PRODUCT_DISK_CASES outside Git. Every CLI runs
with denied network syscalls and a retained trace. Scratch and outputs survive.
"""
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import unittest
import product_fixture as plain
import product_keys_fixture as encrypted
import product_disk_fixture as fixture

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_DISK_CASES')


@unittest.skipUnless(BINARY and CASES, 'explicit external synthetic disk product run required')
class ProductDiskCli(unittest.TestCase):
    def setUp(self):
        self.root = Path(CASES) / self._testMethodName
        self.root.mkdir(parents=True, mode=0o700)
        self.assertFalse(any((p / '.git').exists() for p in self.root.resolve().parents))
        self.calls = 0
        self.client = self.root / 'client'
        self.output = self.root / 'output'
        self.scratch = self.root / 'scratch'
        self.scratch.mkdir(mode=0o700)
        self.plan = self.root / 'plan.tsv'
        self.keys = None

    def prepare(self, supplied=False, components='db2,gt,cameras,maps,vmaps,mmaps', build=70205, locale='enUS'):
        self.payloads = fixture.files()
        if supplied:
            encrypted.store(self.client, self.payloads, build, locale)
            self.keys = self.root / 'keys.tsv'
            self.keys.write_text(encrypted.keyfile())
        else:
            plain.store(self.client, self.payloads, build, locale)
        self.plan.write_text(fixture.plan(components, build, locale, supplied))

    def options(self, command):
        args = ['--plan', str(self.plan)]
        if command in ('extract', 'resume'):
            args += ['--client', str(self.client), '--output', str(self.output), '--scratch', str(self.scratch)]
            if self.keys:
                args += ['--keys', str(self.keys)]
        elif command == 'verify':
            args += ['--output', str(self.output)]
        return args

    def invoke(self, command, options=None, code=0, kill=None, contract='forever-product-v3', trace_fsync=False):
        self.calls += 1
        trace = self.root / f'{self.calls:02}-{command}.network'
        args = ['strace', '-f', '-qq', '-yy', '-e', 'trace=network,fsync' if kill or trace_fsync else 'trace=network',
                '-e', 'inject=network:error=ENETUNREACH', '-o', str(trace)]
        if kill:
            args += ['-e', f'inject=fsync:signal=SIGKILL:when={kill}']
        result = subprocess.run(args + [BINARY, command] + (self.options(command) if options is None else options),
                                capture_output=True, text=True, timeout=240)
        (self.root / f'{self.calls:02}-{command}.stdout').write_text(result.stdout)
        (self.root / f'{self.calls:02}-{command}.stderr').write_text(result.stderr)
        self.assertNotIn(str(self.root), result.stdout + result.stderr)
        if kill:
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('SIGKILL', trace.read_text())
            self.assertNotIn('ENETUNREACH', trace.read_text())
            return None
        self.assertEqual(result.returncode, code, result.stderr)
        if trace_fsync:
            for line in trace.read_text().splitlines():
                self.assertRegex(line, r'^\d+\s+fsync\(.*\)\s+= 0$')
        else:
            self.assertEqual(trace.read_text(), '')
        report = json.loads(result.stdout if code == 0 else result.stderr)
        if report.get('code') != 'product_usage':
            self.assertEqual(report['contract'], contract)
        return report

    def inventory(self):
        return json.loads((self.output / 'forever-package.json').read_text())['artifacts']

    def assert_nonempty(self):
        self.assertGreaterEqual(len(list((self.output / 'vmaps').glob('*.vmo'))), 2)
        self.assertTrue(list((self.output / 'vmaps').rglob('*.vmtree')))
        self.assertTrue(list((self.output / 'vmaps').rglob('*.vmtile')))
        self.assertTrue(list((self.output / 'maps').glob('*.map')))
        self.assertTrue(list((self.output / 'mmaps').glob('*.mmtile')))
        self.assertGreater((self.output / 'vmaps/GameObjectModels.dtree').stat().st_size, 8)

    def assert_private_retained_scratch(self):
        namespaces = list(self.scratch.iterdir())
        self.assertTrue(namespaces)
        for namespace in namespaces:
            self.assertTrue(namespace.is_dir())
            self.assertEqual(namespace.stat().st_mode & 0o777, 0o700)
            for directory in namespace.iterdir():
                self.assertTrue(directory.is_dir())
                self.assertEqual(directory.stat().st_mode & 0o777, 0o700)
                self.assertEqual([p.name for p in directory.iterdir()], ['records.archive'])
                self.assertEqual((directory / 'records.archive').stat().st_mode & 0o777, 0o600)
        self.assertFalse(any('scratch' in entry['path'] for entry in self.inventory()))

    def test_all_six_nonempty_world_matches_resident_artifacts(self):
        self.prepare()
        result = self.invoke('extract')
        self.invoke('verify')
        self.assertEqual(result['collision_storage'], 'linux-leased-append-disk-world-v1')
        self.assertEqual(result['package']['context']['components'], 63)
        self.assert_nonempty()
        self.assert_private_retained_scratch()
        disk_inventory = self.inventory()
        disk_context = result['package']['context']
        self.assertEqual(disk_context['plan_sha256'], hashlib.sha256(self.plan.read_bytes()).hexdigest())
        self.output = self.root / 'resident-output'
        self.plan.write_text(plain.plan())
        args = ['--plan', str(self.plan), '--client', str(self.client), '--output', str(self.output)]
        resident = self.invoke('extract', args, contract='forever-product-v1')
        self.assertEqual(disk_inventory, self.inventory())
        self.assertNotEqual(disk_context['plan_sha256'], resident['package']['context']['plan_sha256'])
        self.assertEqual(disk_context['source_sha256'], resident['package']['context']['source_sha256'])
        self.assertEqual(disk_context['producer_sha256'], resident['package']['context']['producer_sha256'])

    def test_cross_volume_scratch_matches_exact_complete_package(self):
        self.prepare(components='vmaps')
        self.invoke('extract'); self.invoke('verify')
        first_inventory = self.inventory()
        first_manifest = (self.output / 'forever-package.json').read_bytes()
        self.assert_private_retained_scratch()
        cross = os.environ.get('RCE_SPOOL_CROSS_VOLUME_PARENT')
        self.assertTrue(cross, 'explicit existing second-filesystem parent required')
        parent = Path(cross)
        self.assertTrue(parent.is_absolute())
        self.assertNotEqual(parent.stat().st_dev, self.client.stat().st_dev)
        self.scratch = parent / self._testMethodName
        self.scratch.mkdir(mode=0o700)
        self.output = self.root / 'cross-volume-output'
        self.invoke('extract'); self.invoke('verify')
        self.assertEqual(first_inventory, self.inventory())
        self.assertEqual(first_manifest, (self.output / 'forever-package.json').read_bytes())
        self.assert_private_retained_scratch()

    def test_encrypted_all_six_nonempty_world_matches_resident_v2(self):
        self.prepare(supplied=True)
        self.invoke('extract'); self.invoke('verify')
        self.assert_nonempty(); self.assert_private_retained_scratch()
        inventory = self.inventory()
        for filename in ('forever-package.json', '.state/context.json'):
            text = (self.output / filename).read_text()
            self.assertNotIn(str(self.keys), text)
            for key in encrypted.KEYS.values():
                self.assertNotIn(key.hex(), text)
        self.output = self.root / 'resident-output'
        self.plan.write_text(encrypted.plan())
        args = ['--plan', str(self.plan), '--client', str(self.client), '--output', str(self.output), '--keys', str(self.keys)]
        self.invoke('extract', args, contract='forever-product-v2')
        self.assertEqual(inventory, self.inventory())

    def test_inspect_and_verify_need_no_scratch(self):
        self.prepare()
        self.assertEqual(self.invoke('inspect')['status'], 'planned')
        self.assertEqual(self.invoke('inspect', ['--plan', str(self.plan), '--client', str(self.client)])['status'], 'admitted')
        self.assertFalse(self.output.exists()); self.assertEqual(list(self.scratch.iterdir()), [])
        self.invoke('extract'); self.invoke('verify')
        self.invoke('inspect', self.options('inspect') + ['--scratch', str(self.scratch)], 2)
        self.invoke('verify', self.options('verify') + ['--scratch', str(self.scratch)], 2)

    def test_missing_scratch_rejects_before_nonexistent_client_or_output(self):
        self.plan.write_text(fixture.plan())
        args = ['--plan', str(self.plan), '--client', str(self.client), '--output', str(self.output)]
        self.assertEqual(self.invoke('extract', args, 2)['code'], 'product_scratch_required')
        self.assertFalse(self.output.exists()); self.assertFalse(self.client.exists())
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_old_plans_forbid_scratch_before_opening_client_or_fifo(self):
        os.mkfifo(self.scratch / 'not-opened')
        for plan, contract in ((plain.plan(), 'forever-product-v1'), (encrypted.plan(), 'forever-product-v2')):
            self.plan.write_text(plan)
            report = self.invoke('extract', code=2, contract=contract)
            self.assertEqual(report['code'], 'product_scratch_forbidden')
            self.assertFalse(self.output.exists()); self.assertFalse(self.client.exists())

    def test_space_shortfall_rejects_before_client_or_output(self):
        budgets = {name: 128 * 1024**3 for name in fixture.FIELDS}
        budgets['output'] = 64 * 1024**3
        self.plan.write_text(fixture.plan(budgets=budgets))
        self.assertEqual(self.invoke('extract', code=9)['code'], 'product_disk_space')
        self.assertFalse(self.output.exists()); self.assertFalse(self.client.exists())
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_scratch_must_exist_be_private_directory_and_not_symlink(self):
        self.plan.write_text(fixture.plan())
        original = self.scratch
        for path, setup in ((self.root / 'absent', None), (self.root / 'file', 'file'),
                            (self.root / 'fifo', 'fifo'), (self.root / 'public', 'public'),
                            (self.root / 'alias', 'alias')):
            if setup == 'file': path.write_bytes(b'authored')
            elif setup == 'fifo': os.mkfifo(path)
            elif setup == 'public': path.mkdir(mode=0o755)
            elif setup == 'alias': path.symlink_to(original, target_is_directory=True)
            self.scratch = path
            report = self.invoke('extract', code=9)
            self.assertIn(report['code'], ('product_disk_path', 'product_disk_privacy', 'product_disk_io'))
            self.assertFalse(self.output.exists()); self.assertFalse(self.client.exists())
        self.scratch = original
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_scratch_and_output_client_boundaries(self):
        self.prepare()
        original_scratch, original_output = self.scratch, self.output
        for scratch, output in ((self.client, original_output), (self.client / 'Data', original_output),
                                (original_scratch, original_scratch / 'output'),
                                (original_scratch, self.root), (self.root, original_output)):
            self.scratch, self.output = scratch, output
            report = self.invoke('extract', code=9)
            self.assertIn(report['code'], ('product_disk_alias', 'product_disk_privacy'))
        self.scratch, self.output = original_scratch, original_output
        self.assertFalse(self.output.exists()); self.assertEqual(list(self.scratch.iterdir()), [])

    def test_real_sigkill_resume_retains_namespace_and_manifest(self):
        self.prepare()
        self.output = self.root / 'uninterrupted'
        self.invoke('extract', trace_fsync=True)
        baseline = (self.output / 'forever-package.json').read_bytes()
        trace = (self.root / f'{self.calls:02}-extract.network').read_text().splitlines()
        # Decode actual fsync FD names from the successful identical producer.
        # The second final-map archive sync is a record commit, after all six
        # owners exist and before any complete collision producer seal.
        commits = [i + 1 for i, line in enumerate(trace)
                   if re.search(r'/maps/records\.archive>\)\s+= 0$', line)]
        self.assertGreaterEqual(len(commits), 2)
        ordinal = commits[1]
        before = {p.name for p in self.scratch.iterdir()}
        self.output = self.root / 'output'
        self.invoke('extract', kill=ordinal)
        interrupted = {p.name for p in self.scratch.iterdir()}
        self.assertTrue(before < interrupted)
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.assertFalse((self.output / '.state/sealed/vmaps').exists())
        self.invoke('verify', code=8)
        self.invoke('resume'); self.invoke('verify')
        self.assert_nonempty()
        self.assertEqual(baseline, (self.output / 'forever-package.json').read_bytes())
        self.assertTrue(interrupted < {p.name for p in self.scratch.iterdir()})
        self.assert_private_retained_scratch()

    def test_resume_changed_budget_rejects_same_context(self):
        self.prepare()
        self.invoke('extract', kill=25)
        original = self.plan.read_text()
        self.plan.write_text(original.replace(f'collision-output-bytes\t{fixture.BUDGET}', f'collision-output-bytes\t{fixture.BUDGET + 1}'))
        self.assertEqual(self.invoke('resume', code=9)['code'], 'package_context')
        self.plan.write_text(original)
        self.invoke('resume'); self.invoke('verify')

    def test_output_budget_caps_whole_package_before_noncollision_db2_writes(self):
        self.prepare()
        self.plan.write_text(self.plan.read_text().replace(
            f'collision-output-bytes\t{fixture.BUDGET}', 'collision-output-bytes\t1'))
        report = self.invoke('extract', code=9)
        self.assertEqual(report['component'], 'db2')
        self.assertEqual(report['code'], 'package_limit')
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.assertFalse((self.output / '.state/sealed/db2').exists())
        self.invoke('verify', code=8)

    def test_keyfile_inside_scratch_rejects_before_output(self):
        self.prepare(supplied=True)
        inside = self.scratch / 'provided.tsv'
        inside.write_bytes(self.keys.read_bytes()); self.keys = inside
        self.assertEqual(self.invoke('extract', code=9)['code'], 'product_disk_alias')
        self.assertFalse(self.output.exists())
        self.assertEqual([p.name for p in self.scratch.iterdir()], ['provided.tsv'])


if __name__ == '__main__':
    unittest.main()
