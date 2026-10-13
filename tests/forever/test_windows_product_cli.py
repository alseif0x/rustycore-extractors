"""GPL-3.0-or-later. Authored synthetic native Windows resident-product QA.

The explicit sealed executable runs on a fixed local NTFS volume in CI. Inputs
come only from the independent toy writers. This does not trace network calls,
execute original TrinityCore, authenticate a publisher, or use client assets.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import unittest

import product_fixture as plain
import product_disk_fixture as world
import product_keys_fixture as encrypted
import product_offmesh_fixture as offmesh

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools'))
import seal_forever_windows as pe

BINARY = os.environ.get('FOREVER_WINDOWS_PRODUCT_BINARY')
UNSEALED = os.environ.get('FOREVER_WINDOWS_PRODUCT_UNSEALED')
CASES = os.environ.get('FOREVER_WINDOWS_PRODUCT_CASES')
GOLDEN = ROOT / 'tests/forever/fixtures/windows-resident-product-golden-v1.json'
GOLDEN_70170 = ROOT / 'tests/forever/fixtures/windows-resident-product-70170-golden-v1.json'
sha = lambda data: hashlib.sha256(data).hexdigest()


def snapshot(path):
    return {p.relative_to(path).as_posix(): sha(p.read_bytes())
            for p in path.rglob('*') if p.is_file()}


@unittest.skipUnless(os.name == 'nt' and BINARY and UNSEALED and CASES,
                     'explicit native Windows sealed-product run required')
class WindowsProductCli(unittest.TestCase):
    def setUp(self):
        self.root = Path(CASES) / self._testMethodName
        self.root.mkdir(parents=True)
        self.assertFalse(any((p / '.git').exists() for p in self.root.resolve().parents))
        self.client = self.root / 'client'
        self.plan = self.root / 'plan.tsv'
        self.output = self.root / 'output'
        self.keys = None
        self.calls = []
        self.binary_hash = sha(Path(BINARY).read_bytes())
        self.unsealed_hash = sha(Path(UNSEALED).read_bytes())
        self.plan_hash = None
        self.keys_hash = None
        self.inputs = None
        self.terminated_phase = None

    def tearDown(self):
        report = {'synthetic_only': True, 'binary_sha256': self.binary_hash,
                  'binary_unchanged': self.binary_hash == sha(Path(BINARY).read_bytes()),
                  'unsealed_sha256': self.unsealed_hash,
                  'unsealed_unchanged': self.unsealed_hash == sha(Path(UNSEALED).read_bytes()),
                  'commands': self.calls, 'terminated_phase': self.terminated_phase,
                  'network_tracing_performed': False, 'network_attempts': 'not_measured',
                  'original_cpp_executed': False,
                  'real_client_acceptance': False}
        encoded = (json.dumps(report, sort_keys=True, indent=2) + '\n').encode()
        (self.root / 'case-report.json').write_bytes(encoded)
        print('WINDOWS_PRODUCT_CASE ' + json.dumps({
            'case': self._testMethodName, 'report_sha256': sha(encoded),
            'commands': len(self.calls), 'binary_sha256': self.binary_hash,
            'terminated_phase': self.terminated_phase}, sort_keys=True))
        self.assertTrue(report['binary_unchanged'])
        self.assertTrue(report['unsealed_unchanged'])
        if self.plan_hash is not None:
            self.assertEqual(sha(self.plan.read_bytes()), self.plan_hash)
        if self.keys_hash is not None:
            self.assertEqual(sha(self.keys.read_bytes()), self.keys_hash)
        if self.inputs is not None:
            self.assertEqual(snapshot(self.client), self.inputs)

    def prepare(self, build=70205, locale='enUS', supplied=False, version6=False,
                payloads=None):
        self.locale = locale
        self.build = build
        payloads = world.files() if payloads is None else payloads
        if supplied:
            encrypted.store(self.client, payloads, build, locale)
            self.keys = self.root / 'keys.tsv'
            self.keys.write_bytes(encrypted.keyfile().encode())
            self.keys_hash = sha(self.keys.read_bytes())
        else:
            plain.store(self.client, payloads, build, locale)
        text = (offmesh.plan((), build, locale, 'db2,gt,cameras,maps,vmaps,mmaps')
                if version6 else plain.plan(build=build, locale=locale))
        if supplied:
            text = text.replace('forever-product\t1\n', 'forever-product\t2\n', 1)
            if version6:
                text = text.replace('decryption\tdisabled\n', 'decryption\tprovided-salsa20-v1\n', 1)
            else:
                text = text.replace('root\theader-50893-v2\n',
                                    'root\theader-50893-v2\ndecryption\tprovided-salsa20-v1\n', 1)
        # Canonical plans and key files require LF even on Windows.
        self.plan.write_bytes(text.encode())
        self.plan_hash = sha(self.plan.read_bytes())
        self.inputs = snapshot(self.client)

    def arguments(self, command):
        args = [command, '--plan', str(self.plan)]
        if command in ('extract', 'resume'):
            args += ['--client', str(self.client), '--output', str(self.output)]
            if self.keys:
                args += ['--keys', str(self.keys)]
        elif command == 'verify':
            args += ['--output', str(self.output)]
        return args

    def record(self, args, result, binary=BINARY):
        ordinal = len(self.calls) + 1
        (self.root / f'{ordinal:03}.stdout').write_bytes(result.stdout)
        (self.root / f'{ordinal:03}.stderr').write_bytes(result.stderr)
        self.calls.append({'executable': str(binary), 'executable_sha256': sha(Path(binary).read_bytes()),
                           'arguments': args, 'exit_code': result.returncode,
                           'stdout_sha256': sha(result.stdout),
                           'stderr_sha256': sha(result.stderr)})

    def invoke(self, command, code=0, args=None, binary=BINARY):
        args = self.arguments(command) if args is None else args
        result = subprocess.run([str(binary), *args], capture_output=True, timeout=180)
        self.record(args, result, binary)
        if code is None:
            self.assertNotEqual(result.returncode, 0)
        else:
            self.assertEqual(result.returncode, code, result.stderr.decode(errors='replace'))
        return json.loads(result.stdout if result.returncode == 0 else result.stderr)

    def assert_artifacts(self, package):
        golden = json.loads((GOLDEN_70170 if self.build == 70170 else GOLDEN).read_bytes())
        expected = [dict(a, path=a['path'].replace('dbc/enUS/', f'dbc/{self.locale}/'))
                    for a in golden['artifacts']]
        expected.sort(key=lambda a: a['path'].lower())
        manifest = json.loads((self.output / 'forever-package.json').read_bytes())
        self.assertEqual(manifest['artifacts'], expected)
        self.assertEqual(package['files'], golden['file_count'])
        self.assertEqual(package['bytes'], golden['total_bytes'])
        self.assertEqual(package['context']['components'], 63)
        self.assertEqual(package['context']['producer_sha256'], self.binary_hash)
        self.assertFalse((self.output / '.forever-package-incomplete').exists())
        protocol = json.loads((self.output / '.state/protocol.json').read_bytes())
        self.assertEqual(protocol['state_version'], 3)
        self.assertEqual(protocol['filesystem_profile'], 'forever-windows-product-ntfs-v1')
        for a in expected:
            data = (self.output / a['path']).read_bytes()
            self.assertEqual((len(data), sha(data)), (a['bytes'], a['sha256']))
        self.assertEqual({a['component'] for a in expected},
                         {'db2', 'gt', 'cameras', 'maps', 'vmaps', 'mmaps'})
        return manifest

    def complete(self, command='extract'):
        extracted = self.invoke(command)
        verified = self.invoke('verify')
        self.assertEqual(extracted['package'], verified['package'])
        self.assertFalse(extracted['full_extractor_complete'])
        return self.assert_artifacts(extracted['package'])

    def terminate_after_db2(self):
        args = self.arguments('extract')
        process = subprocess.Popen([str(BINARY), *args], stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE)
        deadline = time.monotonic() + 60
        try:
            while not (self.output / '.state/sealed/db2').is_file():
                self.assertIsNone(process.poll(), 'producer exited before termination boundary')
                self.assertLess(time.monotonic(), deadline, 'termination boundary timed out')
                time.sleep(0.001)
            # This uses TerminateProcess on Windows, without an orderly unwind.
            self.assertIsNone(process.poll(), 'producer completed before termination')
            process.kill()
        finally:
            if process.poll() is None:
                process.kill()
            stdout, stderr = process.communicate(timeout=30)
            result = subprocess.CompletedProcess(args, process.returncode, stdout, stderr)
            self.record(args, result)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue((self.output / '.forever-package-incomplete').is_file())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.terminated_phase = 'observed_db2_seal_before_final_manifest'

    def test_70205_enUS_nonempty_all_six_match_linux_artifact_golden(self):
        self.prepare()
        self.complete()

    def test_70170_esES_nonempty_all_six_match_linux_artifact_golden(self):
        self.prepare(build=70170, locale='esES')
        self.complete()

    def test_v6_explicit_empty_offmesh_preserves_all_six_artifacts(self):
        self.prepare(version6=True)
        self.complete()

    def test_supplied_synthetic_salsa20_keys_all_six(self):
        self.prepare(supplied=True)
        self.complete()

    def test_loaded_sealed_producer_client_inspection_without_output(self):
        self.prepare()
        result = self.invoke('inspect', args=['inspect', '--plan', str(self.plan),
                                           '--client', str(self.client)])
        self.assertEqual(result['status'], 'admitted')
        self.assertEqual(result['context']['producer_sha256'], self.binary_hash)
        self.assertFalse(result['availability_checked'])
        self.assertFalse(self.output.exists())

    def test_two_fresh_outputs_are_byte_identical(self):
        self.prepare()
        first = self.complete()
        before = snapshot(self.output)
        self.output = self.root / 'second'
        self.assertEqual(self.complete(), first)
        self.assertEqual(snapshot(self.output), before)

    def test_terminate_process_then_resume_sealed_components(self):
        self.prepare()
        self.terminate_after_db2()
        self.complete('resume')

    def test_resume_wrong_plan_rejects_without_mutating_incomplete_state(self):
        self.prepare()
        self.terminate_after_db2()
        before = snapshot(self.output)
        self.plan.write_bytes(self.plan.read_bytes().replace(b'maximum-slope\t55', b'maximum-slope\t56'))
        # Explicit authored mutation; all following invocations must retain it.
        self.plan_hash = sha(self.plan.read_bytes())
        self.assertEqual(self.invoke('resume', code=None)['code'], 'package_context')
        self.assertEqual(snapshot(self.output), before)

    def test_completed_output_rejects_new_writer_and_resume(self):
        self.prepare()
        self.complete()
        before = snapshot(self.output)
        self.invoke('extract', code=None)
        self.invoke('resume', code=None)
        self.assertEqual(snapshot(self.output), before)

    def test_missing_model_dependency_prevents_complete_package(self):
        payloads = world.files()
        del payloads[502]
        self.prepare(payloads=payloads)
        self.invoke('extract', code=None)
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.invoke('verify', code=None)

    def test_artifact_tampering_rejects_verification(self):
        self.prepare()
        manifest = self.complete()
        artifact = self.output / manifest['artifacts'][0]['path']
        data = artifact.read_bytes()
        artifact.write_bytes(bytes([data[0] ^ 1]) + data[1:])
        self.invoke('verify', code=None)

    def test_unknown_inventory_entry_rejects_verification(self):
        self.prepare()
        self.complete()
        (self.output / 'unexpected').write_bytes(b'authored unexpected entry')
        self.invoke('verify', code=None)

    def test_unsealed_and_modified_loaded_seals_reject_before_output(self):
        self.prepare()
        result = self.invoke('extract', code=None, binary=UNSEALED)
        self.assertEqual(result['code'], 'producer_unsealed')
        self.assertFalse(self.output.exists())
        data = Path(BINARY).read_bytes()
        layout = pe.locate(data)
        bad = self.root / 'changed-seal.exe'
        bad.write_bytes(data[:layout.slot] + bytes([data[layout.slot] ^ 1]) + data[layout.slot + 1:])
        result = self.invoke('extract', code=None, binary=bad)
        self.assertEqual(result['code'], 'pe_digest_mismatch')
        self.assertFalse(self.output.exists())

    def test_linux_disk_profile_rejects_before_missing_input_paths(self):
        self.plan.write_bytes(world.plan(supplied=True).encode())
        self.plan_hash = sha(self.plan.read_bytes())
        for command in ('inspect', 'extract', 'resume', 'verify'):
            args = [command, '--plan', str(self.plan)]
            if command != 'verify':
                args += ['--client', str(self.root / 'absent-client'),
                         '--keys', str(self.root / 'absent-keys')]
            if command != 'inspect':
                args += ['--output', str(self.output)]
            if command in ('extract', 'resume'):
                args += ['--scratch', str(self.root / 'absent-scratch')]
            self.assertEqual(self.invoke(command, code=2, args=args)['code'],
                             'product_disk_unsupported')
        self.assertFalse(self.output.exists())

    def test_client_and_output_ancestor_overlap_rejects(self):
        self.prepare()
        self.output = self.client / 'output'
        self.invoke('extract', code=None)
        self.assertFalse(self.output.exists())


if __name__ == '__main__':
    unittest.main()
