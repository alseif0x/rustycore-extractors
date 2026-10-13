# SPDX-License-Identifier: GPL-3.0-or-later
"""Authored synthetic v7 product QA on actual Windows/MSVC/fixed local NTFS.

Run only with the same explicit binary/case variables as the resident helper.
The independent toy writers and build-bearing Linux inventories are reused;
this suite does not execute original C++, use client assets or trace networking.
"""
import json
import os
from pathlib import Path
import subprocess
import time
import unittest

import product_fixture as plain
import product_disk_fixture as world
import product_keys_fixture as encrypted
import test_windows_product_cli as resident

BINARY, UNSEALED, CASES = resident.BINARY, resident.UNSEALED, resident.CASES
PROFILE = 'windows-exclusive-append-disk-world-v1'
OWNERS = {'raw', 'wdt', 'cache', 'placements', 'vmo', 'maps'}
sha, snapshot = resident.sha, resident.snapshot


def disk_plan(build=70205, locale='enUS', supplied=False,
              components='db2,gt,cameras,maps,vmaps,mmaps', budgets=None, offmesh=False):
    text = world.plan(components, build, locale, supplied, budgets)
    text = text.replace('forever-product\t3\n', 'forever-product\t7\n', 1)
    text = text.replace('linux-leased-append-disk-world-v1', PROFILE, 1)
    if offmesh:
        text = text.replace('non-steep-slope\t55\n',
                            'non-steep-slope\t55\noffmesh\tinline-f32-bits-v1\n', 1)
    return text


def make_private_directory(path):
    """QA setup only: one noninherited owner full-control ACE, protected DACL."""
    script = '''$ErrorActionPreference = 'Stop'
$path = $env:FOREVER_QA_PRIVATE_DIRECTORY
if (Test-Path -LiteralPath $path) { throw 'synthetic private directory already exists' }
[IO.Directory]::CreateDirectory($path) | Out-Null
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User
$acl = [Security.AccessControl.DirectorySecurity]::new()
$acl.SetOwner($sid)
$acl.SetAccessRuleProtection($true, $false)
$rule = [Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'Allow')
$acl.AddAccessRule($rule)
Set-Acl -LiteralPath $path -AclObject $acl
'''
    env = dict(os.environ, FOREVER_QA_PRIVATE_DIRECTORY=str(path))
    result = subprocess.run(['pwsh', '-NoProfile', '-NonInteractive', '-Command', script],
                            env=env, capture_output=True, timeout=30)
    if result.returncode != 0:
        raise RuntimeError('private synthetic scratch setup failed: '
                           + result.stderr.decode(errors='replace'))


def collision_boundary_payloads():
    """Two complete M2 inputs give a visible first-RAW/second-read kill window.

    Padding is authored ignored source data, not extra geometry. Distinct final
    bytes prevent the toy CASC writer from deduplicating the two source buffers.
    """
    payloads = world.files()
    base = world.m2()
    size = 16 * 1024 * 1024
    payloads[400] = base + bytes(size - len(base) - 1) + b'a'
    payloads[401] = base + bytes(size - len(base) - 1) + b'b'
    payloads[plain.IDS['GameObjectDisplayInfo']] = plain.table(
        'GameObjectDisplayInfo', [{1: 400}, {1: 401}, {1: 501}, {1: 0}], [1, 2, 3, 4])
    return payloads


@unittest.skipUnless(os.name == 'nt' and BINARY and UNSEALED and CASES,
                     'explicit native Windows sealed disk-product run required')
class WindowsDiskProductCli(unittest.TestCase):
    record = resident.WindowsProductCli.record
    invoke = resident.WindowsProductCli.invoke
    assert_artifacts = resident.WindowsProductCli.assert_artifacts

    def setUp(self):
        resident.WindowsProductCli.setUp(self)
        self.scratch = self.root / 'scratch'
        make_private_directory(self.scratch)
        self.boundary = None
        self.scratch_unchanged = None

    def tearDown(self):
        report = {
            'synthetic_only': True, 'storage_profile': PROFILE,
            'binary_sha256': self.binary_hash,
            'binary_unchanged': self.binary_hash == sha(Path(BINARY).read_bytes()),
            'unsealed_sha256': self.unsealed_hash,
            'unsealed_unchanged': self.unsealed_hash == sha(Path(UNSEALED).read_bytes()),
            'commands': self.calls, 'terminated_phase': self.terminated_phase,
            'collision_boundary': self.boundary,
            'old_scratch_unchanged_after_resume': self.scratch_unchanged,
            'network_tracing_performed': False, 'network_attempts': 'not_measured',
            'original_cpp_executed': False, 'real_client_acceptance': False,
        }
        encoded = (json.dumps(report, sort_keys=True, indent=2) + '\n').encode()
        (self.root / 'case-report.json').write_bytes(encoded)
        print('WINDOWS_DISK_PRODUCT_CASE ' + json.dumps({
            'case': self._testMethodName, 'report_sha256': sha(encoded),
            'commands': len(self.calls), 'binary_sha256': self.binary_hash}, sort_keys=True))
        print('WINDOWS_DISK_PRODUCT_CASE_REPORT ' + json.dumps(
            {'case': self._testMethodName, 'report': report}, sort_keys=True, separators=(',', ':')))
        self.assertTrue(report['binary_unchanged'])
        self.assertTrue(report['unsealed_unchanged'])
        if self.plan_hash is not None:
            self.assertEqual(sha(self.plan.read_bytes()), self.plan_hash)
        if self.keys_hash is not None:
            self.assertEqual(sha(self.keys.read_bytes()), self.keys_hash)
        if self.inputs is not None:
            self.assertEqual(snapshot(self.client), self.inputs)

    def set_plan(self, text):
        self.plan.write_bytes(text.encode('ascii'))
        self.plan_hash = sha(self.plan.read_bytes())

    def prepare(self, build=70205, locale='enUS', supplied=False,
                components='db2,gt,cameras,maps,vmaps,mmaps', payloads=None,
                budgets=None, offmesh=False):
        self.build, self.locale = build, locale
        payloads = world.files() if payloads is None else payloads
        if supplied:
            encrypted.store(self.client, payloads, build, locale)
            self.keys = self.root / 'keys.tsv'
            self.keys.write_bytes(encrypted.keyfile().encode('ascii'))
            self.keys_hash = sha(self.keys.read_bytes())
        else:
            plain.store(self.client, payloads, build, locale)
        self.set_plan(disk_plan(build, locale, supplied, components, budgets, offmesh))
        self.inputs = snapshot(self.client)

    def arguments(self, command):
        args = [command, '--plan', str(self.plan)]
        if command in ('extract', 'resume'):
            args += ['--client', str(self.client), '--output', str(self.output),
                     '--scratch', str(self.scratch)]
            if self.keys:
                args += ['--keys', str(self.keys)]
        elif command == 'verify':
            args += ['--output', str(self.output)]
        return args

    def namespaces(self):
        entries = list(self.scratch.iterdir())
        self.assertTrue(all(p.is_dir() and p.name.startswith('.forever-collision-') for p in entries))
        return {p.name: p for p in entries}

    def assert_scratch(self):
        namespaces = self.namespaces()
        self.assertTrue(namespaces)
        for path in namespaces.values():
            self.assertEqual({p.name for p in path.iterdir()}, OWNERS | {'.forever-package-lock'})
            for name in OWNERS:
                self.assertEqual({p.name for p in (path / name).iterdir()},
                                 {'records.archive', '.forever-package-lock'})
                self.assertGreaterEqual((path / name / 'records.archive').stat().st_size, 32)
        return namespaces

    def complete(self, command='extract', golden=True):
        extracted = self.invoke(command)
        verified = self.invoke('verify')
        self.assertEqual(extracted['contract'], 'forever-product-v7')
        self.assertEqual(verified['contract'], 'forever-product-v7')
        self.assertEqual(extracted['collision_storage'], PROFILE)
        self.assertEqual(extracted['package'], verified['package'])
        self.assertFalse(extracted['full_extractor_complete'])
        self.assertEqual(extracted['package']['context']['plan_sha256'], self.plan_hash)
        manifest = (self.assert_artifacts(extracted['package']) if golden else
                    json.loads((self.output / 'forever-package.json').read_bytes()))
        self.assert_scratch()
        self.assertFalse(any('scratch' in a['path'] for a in manifest['artifacts']))
        return manifest

    def terminate_during_collision(self):
        before = set(self.namespaces())
        args = self.arguments('extract')
        process = subprocess.Popen([str(BINARY), *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        deadline = time.monotonic() + 90
        try:
            while True:
                self.assertIsNone(process.poll(), 'producer exited before collision boundary')
                self.assertLess(time.monotonic(), deadline, 'collision boundary timed out')
                added = {p.name: p for p in self.scratch.iterdir() if p.name not in before}
                if len(added) == 1:
                    namespace = next(iter(added.values()))
                    entries = {p.name for p in namespace.iterdir()}
                    raw = namespace / 'raw'
                    if OWNERS <= entries and raw.is_dir():
                        # Windows directory enumeration supplies size without
                        # trying to reopen the live SHARE0 archive for reading.
                        with os.scandir(raw) as items:
                            sizes = {p.name: p.stat(follow_symlinks=False).st_size for p in items}
                        if sizes.get('records.archive', 0) > 32:
                            self.assertFalse((self.output / '.state/sealed/vmaps').exists())
                            self.assertIsNone(process.poll(), 'producer exited at collision boundary')
                            self.boundary = {'owners': sorted(OWNERS),
                                             'raw_archive_bytes_observed': sizes['records.archive'],
                                             'collision_seal_absent': True,
                                             'namespace': namespace.name}
                            # Actual TerminateProcess, without orderly unwind.
                            process.kill()
                            break
                time.sleep(0.001)
        finally:
            if process.poll() is None:
                process.kill()
            stdout, stderr = process.communicate(timeout=30)
            self.record(args, subprocess.CompletedProcess(args, process.returncode, stdout, stderr))
        # CPython's Windows kill uses TerminateProcess(handle, 1). A normal
        # product error (2/8/9) that wins the race must not count as termination.
        self.assertEqual(process.returncode, 1)
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.assertFalse((self.output / '.state/sealed/vmaps').exists())
        self.terminated_phase = 'observed_raw_append_extent_with_six_owners_before_collision_seal'
        return {name: snapshot(path) for name, path in self.namespaces().items()}

    def test_70205_enUS_all_six_match_linux_artifact_inventory(self):
        self.prepare()
        self.complete()

    def test_70170_esES_all_six_match_linux_artifact_inventory(self):
        self.prepare(build=70170, locale='esES')
        self.complete()

    def test_supplied_toy_keys_all_six(self):
        self.prepare(supplied=True)
        self.complete()

    def test_optional_empty_offmesh_all_six(self):
        self.prepare(offmesh=True)
        self.complete()

    def test_plan_and_client_inspect_require_no_scratch_or_output(self):
        self.prepare()
        planned = self.invoke('inspect')
        self.assertEqual(planned['status'], 'planned')
        inspected = self.invoke('inspect', args=['inspect', '--plan', str(self.plan),
                                                '--client', str(self.client)])
        self.assertEqual(inspected['status'], 'admitted')
        self.assertEqual(inspected['context']['producer_sha256'], self.binary_hash)
        self.assertFalse(self.output.exists())
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_fresh_outputs_have_exact_same_manifest_and_artifacts(self):
        self.prepare()
        first = self.complete()
        before = snapshot(self.output)
        self.output = self.root / 'second-output'
        self.assertEqual(self.complete(), first)
        self.assertEqual(snapshot(self.output), before)
        self.assertEqual(len(self.namespaces()), 2)

    def test_second_ntfs_volume_scratch_preserves_exact_complete_package(self):
        self.prepare(components='vmaps')
        baseline = self.complete(golden=False)
        alternate = os.environ.get('FOREVER_WINDOWS_DISK_OTHER_VOLUME_PARENT')
        self.assertTrue(alternate, 'explicit second fixed local NTFS volume required')
        parent = Path(alternate)
        self.assertNotEqual(parent.stat().st_dev, self.scratch.stat().st_dev)
        self.scratch = parent / self._testMethodName
        make_private_directory(self.scratch)
        self.output = self.root / 'other-volume-output'
        self.assertEqual(self.complete(golden=False), baseline)
        self.assertEqual(len(self.namespaces()), 1)

    def test_terminate_during_collision_then_resume_fresh_scratch(self):
        self.prepare(payloads=collision_boundary_payloads())
        self.output = self.root / 'uninterrupted'
        baseline = self.complete(golden=False)
        self.output = self.root / 'output'
        old = self.terminate_during_collision()
        self.invoke('verify', code=8)
        self.assertEqual(self.complete('resume', golden=False), baseline)
        after = self.namespaces()
        self.assertEqual(len(after), len(old) + 1)
        for name, files in old.items():
            self.assertEqual(snapshot(after[name]), files)
        self.scratch_unchanged = True

    def test_resume_changed_budget_rejects_before_scratch_mutation(self):
        self.prepare(components='vmaps', payloads=collision_boundary_payloads())
        old = self.terminate_during_collision()
        before = snapshot(self.output)
        original = self.plan.read_text(encoding='ascii')
        self.set_plan(original.replace(f'collision-output-bytes\t{world.BUDGET}\n',
                                       f'collision-output-bytes\t{world.BUDGET + 1}\n', 1))
        self.assertEqual(self.invoke('resume', code=9)['code'], 'package_context')
        self.assertEqual(snapshot(self.output), before)
        self.assertEqual({n: snapshot(p) for n, p in self.namespaces().items()}, old)
        self.set_plan(original)
        self.complete('resume', golden=False)
        for name, files in old.items():
            self.assertEqual(snapshot(self.namespaces()[name]), files)
        self.scratch_unchanged = True

    def test_output_budget_bounds_noncollision_db2_before_scratch_creation(self):
        budgets = dict.fromkeys(world.FIELDS, world.BUDGET)
        budgets['output'] = 1
        self.prepare(budgets=budgets)
        result = self.invoke('extract', code=9)
        self.assertEqual((result['component'], result['code']), ('db2', 'package_limit'))
        self.assertFalse((self.output / '.state/sealed/db2').exists())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.assertEqual(list(self.scratch.iterdir()), [])
        self.invoke('verify', code=8)

    def test_raw_budget_failure_retains_scratch_without_collision_prefix(self):
        budgets = dict.fromkeys(world.FIELDS, world.BUDGET)
        budgets['raw'] = 32
        self.prepare(components='vmaps', budgets=budgets)
        self.invoke('extract', code=9)
        self.assertFalse((self.output / '.state/sealed/vmaps').exists())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.assertFalse((self.output / 'vmaps').exists())
        self.assert_scratch()
        self.invoke('verify', code=8)

    def test_missing_scratch_rejects_before_missing_keys_client_or_output(self):
        self.set_plan(disk_plan(supplied=True))
        args = ['extract', '--plan', str(self.plan), '--client', str(self.client),
                '--keys', str(self.root / 'missing-keys'), '--output', str(self.output)]
        self.assertEqual(self.invoke('extract', code=2, args=args)['code'], 'product_scratch_required')
        self.assertFalse(self.output.exists())
        self.assertFalse(self.client.exists())
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_key_inside_scratch_rejects_without_output_or_namespace(self):
        self.prepare(supplied=True)
        inside = self.scratch / 'provided.tsv'
        inside.write_bytes(self.keys.read_bytes())
        self.keys = inside
        self.assertEqual(self.invoke('extract', code=9)['code'], 'product_disk_alias')
        self.assertFalse(self.output.exists())
        self.assertEqual({p.name for p in self.scratch.iterdir()}, {'provided.tsv'})

    def test_source_scratch_and_output_overlap_reject_before_mutation(self):
        self.prepare()
        scratch, output = self.scratch, self.output
        for selected_scratch, selected_output in ((scratch, self.client / 'output'),
                                                  (scratch, scratch / 'output'),
                                                  (self.client, output), (self.root, output)):
            self.scratch, self.output = selected_scratch, selected_output
            result = self.invoke('extract', code=9)
            self.assertIn(result['code'], ('product_disk_alias', 'product_disk_privacy'))
            self.assertFalse(selected_output.exists())
        self.scratch, self.output = scratch, output
        self.assertEqual(list(scratch.iterdir()), [])

    def test_unsealed_and_changed_loaded_seals_reject_before_scratch_creation(self):
        resident.WindowsProductCli.test_unsealed_and_modified_loaded_seals_reject_before_output(self)
        self.assertEqual(list(self.scratch.iterdir()), [])

    def test_artifact_and_unknown_inventory_tampering_reject_verification(self):
        self.prepare(components='vmaps')
        manifest = self.complete(golden=False)
        path = self.output / manifest['artifacts'][0]['path']
        before = path.read_bytes()
        path.write_bytes(bytes([before[0] ^ 1]) + before[1:])
        self.invoke('verify', code=9)
        path.write_bytes(before)
        self.invoke('verify')
        (self.output / 'unexpected').write_bytes(b'authored foreign entry')
        self.invoke('verify', code=9)

    def test_space_shortfall_rejects_before_source_acquisition_or_output(self):
        self.client.mkdir()
        budgets = dict.fromkeys(world.FIELDS, 128 * 1024**3)
        budgets['output'] = 64 * 1024**3
        self.set_plan(disk_plan(budgets=budgets))
        self.inputs = snapshot(self.client)
        self.assertEqual(self.invoke('extract', code=9)['code'], 'product_disk_space')
        self.assertFalse(self.output.exists())
        self.assertEqual(list(self.scratch.iterdir()), [])


if __name__ == '__main__':
    unittest.main()
