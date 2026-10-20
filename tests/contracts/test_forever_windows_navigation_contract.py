# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original static Windows navigation source checks.
# Modified 2026-10-12: retain import pins and admit one reviewed test-only delta.
# Modified 2026-10-19: retain exact inverse for Linux-only Windows-lint cfg gates.
# Modified 2026-10-19: retain exact inverse for rooted Linux available inventory factoring.
"""Source consistency only; no runtime, client reads or actual NTFS proof."""
import ast
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-consumer-source-v13'
HISTORICAL_PROFILE = 'forever-seven-crate-windows-navigation-readers-source-v7'


def literal(name):
    for node in ast.parse((ROOT / 'tools/package_forever.py').read_text()).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class WindowsNavigationContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads((ROOT / 'contracts/forever-windows-navigation-contract-v1.json').read_text())
        cls.terrain = (ROOT / 'crates/forever-navigation/src/terrain/load/windows.rs').read_text()
        cls.inventory = (ROOT / 'crates/forever-navigation/src/inventory/windows.rs').read_text()
        cls.guard = (ROOT / 'crates/forever-navigation/src/inventory/windows/guard.rs').read_text()
        cls.run_source = (ROOT / 'crates/forever-navigation/src/inventory/windows/run.rs').read_text()

    def test_source_scope_keeps_product_and_runtime_gates_closed(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-navigation-readers-source-v1', 1))
        self.assertTrue(self.c['original_implementation'])
        for field in ['network', 'windows_product_enabled', 'actual_windows_ntfs_accepted', 'real_geometry_accepted', 'full_runtime_accepted', 'full_extractor_complete', 'publication_authorized']:
            self.assertFalse(self.c[field])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertIn('static source consistency only', self.c['validation']['static_tests'])

    def test_mandatory_closed_source_group_and_prior_profiles(self):
        d = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())
        self.assertEqual(self.c['distribution_source_profile'], HISTORICAL_PROFILE)
        self.assertIn(HISTORICAL_PROFILE, d['source']['prior_profiles'])
        self.assertEqual(literal('SOURCE_PROFILE'), PROFILE)
        self.assertEqual(d['source']['profile'], PROFILE)
        self.assertEqual(tuple(self.c['sources']), literal('WINDOWS_NAVIGATION_SOURCE'))
        self.assertEqual(len(self.c['sources']), 13)
        for name in self.c['sources']:
            self.assertTrue((ROOT / name).is_file(), name)
            self.assertIn(name, d['source']['required'])
        for profile in ['forever-seven-crate-windows-producer-source-v4', 'forever-seven-crate-windows-vmap-diagnostics-source-v5', 'forever-seven-crate-windows-vmap-required-framing-source-v6']:
            self.assertIn(profile, d['source']['prior_profiles'])

    def test_validated_twelve_imports_and_reviewed_fixture_delta_are_byte_exact(self):
        pins = self.c['validated_import_sha256']
        self.assertEqual(len(pins), 12)
        fixture = 'crates/forever-navigation/src/terrain/load/windows/tests.rs'
        correction = self.c['reviewed_fixture_correction']
        self.assertEqual(correction['path'], fixture)
        self.assertEqual(correction['historical_import_sha256'], pins[fixture])
        self.assertEqual(pins[fixture], '56ff0049c76d7cb2039e703b5dba5885dda0f1e4a7c4d733175e7e9a0a63c680')
        self.assertEqual(correction['current_sha256'], '95127d88e88e99ac55c26deb720ad5b14f86f8d367097018d8b491dda21ec760')
        self.assertFalse(correction['production_changed'])
        reviewed = {fixture: correction['current_sha256']}
        additional = self.c['additional_reviewed_fixture_corrections']
        expected_additional = {'crates/forever-navigation/src/inventory/windows/tests.rs': 'a577712f25b2fdf63d74c740a2fb16bab45082aa17535c2ddba35e1b85c428ce', 'crates/forever-navigation/src/inventory/windows/run/tests.rs': 'fdf8cd0330a1b00fc944ccb8b4c8d986f8d060c5c5853d5f7a25de3a6b200176'}
        self.assertEqual({e['path']: e['current_sha256'] for e in additional}, expected_additional)
        self.assertEqual(len(additional), 2)
        for entry in additional:
            self.assertEqual(entry['historical_import_sha256'], pins[entry['path']])
            self.assertFalse(entry['production_changed'])
            reviewed[entry['path']] = entry['current_sha256']
        order = self.c['reviewed_module_order_correction']
        path = 'crates/forever-navigation/src/terrain/load.rs'
        self.assertEqual(order['path'], path)
        self.assertEqual(order['historical_import_sha256'], pins[path])
        self.assertFalse(order['behavior_changed'])
        current = (ROOT / path).read_bytes()
        gate = self.c['reviewed_build70450_map_verifier_correction']
        self.assertEqual(gate['path'], path)
        self.assertEqual(gate['previous_sha256'], order['current_sha256'])
        self.assertEqual(
            gate['current_sha256'],
            '1ea7f6e6c4fad539a113a4e710560e8aa499d1a3513991833ce163d897e91699',
        )
        self.assertEqual(
            gate['scope'],
            'Linux source loading dispatches only build 70450 to the explicit verify_70450 path; every other build, including Windows, retains the generic verifier',
        )
        self.assertEqual(hashlib.sha256(current).hexdigest(), gate['current_sha256'])
        prior_current = current
        for added, previous in [
            (
                b'verify_map(&bytes, plan.build(), limits.file_bytes)',
                b'maps::verify(&bytes, plan.build(), limits.file_bytes)',
            ),
            (
                b'map: verify_map(&file.bytes, build, maps::MAX_MAP_BYTES).expect("complete checked terrain"),',
                b'map: maps::verify(&file.bytes, build, maps::MAX_MAP_BYTES)\n            .expect("complete checked terrain"),',
            ),
            (
                b"fn verify_map(\n    bytes: &[u8],\n    build: u32,\n    max_bytes: usize,\n) -> Result<maps::MapView<'_>, maps::Error> {\n    #[cfg(target_os = \"linux\")]\n    if build == 70450 {\n        maps::verify_70450(bytes, build, max_bytes)\n    } else {\n        maps::verify(bytes, build, max_bytes)\n    }\n    #[cfg(not(target_os = \"linux\"))]\n    {\n        maps::verify(bytes, build, max_bytes)\n    }\n}\n",
                b'',
            ),
        ]:
            self.assertEqual(prior_current.count(added), 1)
            prior_current = prior_current.replace(added, previous, 1)
        self.assertEqual(hashlib.sha256(prior_current).hexdigest(), gate['previous_sha256'])
        before = b'#[cfg(windows)]\npub mod windows;\n#[cfg(test)]\nmod tests;'
        after = b'#[cfg(test)]\nmod tests;\n#[cfg(windows)]\npub mod windows;'
        self.assertEqual(prior_current.count(after), 1)
        self.assertEqual(hashlib.sha256(prior_current.replace(after, before)).hexdigest(), pins[path])
        reviewed[path] = gate['current_sha256']
        admission = self.c['reviewed_build70450_inventory_admission_correction']
        inventory_path = 'crates/forever-navigation/src/inventory.rs'
        self.assertEqual(admission['path'], inventory_path)
        self.assertEqual(
            admission['previous_sha256'],
            '13946d3a1df2e99deed0faebe57a715ff99dff3213c50bee018a772c968f1a00',
        )
        self.assertEqual(admission['current_sha256'], pins[inventory_path])
        self.assertEqual(
            admission['current_sha256'],
            '3953e798ee1b22a1d9bca7646ed9ff2366fe5ab25651af05c6aea447ed4125fd',
        )
        self.assertEqual(
            admission['scope'],
            'Linux Plan admission adds only build 70450; builds 70170 and 70205 retain their prior gate and non-Linux build-70450 admission remains rejected',
        )
        inventory_current = (ROOT / inventory_path).read_bytes()
        fs_delta = self.c['reviewed_linux_selected_filesystem_inventory_delta']
        self.assertEqual(fs_delta['path'], inventory_path)
        self.assertEqual(fs_delta['baseline_revision'], '6eb2f657e6ae90dc4db8de420353ffc042ebafad')
        self.assertEqual(fs_delta['baseline_sha256'], 'dcfa037709d1b4f66ecee282e8effa7442a7e28787a9814e3cbca5603cb35f18')
        self.assertTrue(fs_delta['exact_inverse_required'])
        fs_fixture_bytes = (ROOT / fs_delta['fixture']).read_bytes()
        self.assertEqual(hashlib.sha256(fs_fixture_bytes).hexdigest(), fs_delta['fixture_sha256'])
        fs_fixture = json.loads(fs_fixture_bytes)
        self.assertEqual(fs_fixture['schema'], 'forever-selected-navigation-fs-inventory-source-delta-v1')
        self.assertEqual(fs_fixture['path'], inventory_path)
        self.assertEqual(fs_fixture['baseline_revision'], fs_delta['baseline_revision'])
        self.assertEqual(fs_fixture['baseline_sha256'], fs_delta['baseline_sha256'])
        self.assertEqual(fs_fixture['candidate_sha256'], fs_delta['candidate_sha256'])
        self.assertEqual(fs_fixture['scope'], fs_delta['scope'])
        self.assertEqual(hashlib.sha256(inventory_current).hexdigest(), fs_delta['candidate_sha256'])
        fs_restored_lines = inventory_current.decode('utf-8').splitlines(keepends=True)
        for hunk in reversed(fs_fixture['hunks']):
            after_lines = hunk['after_lines']
            before_lines = hunk['before_lines']
            self.assertTrue(after_lines or before_lines)
            matches = [
                index
                for index in range(len(fs_restored_lines) - len(after_lines) + 1)
                if fs_restored_lines[index:index + len(after_lines)] == after_lines
            ]
            self.assertEqual(len(matches), 1, 'each Linux filesystem delta hunk must invert exactly once')
            start = matches[0]
            fs_restored_lines[start:start + len(after_lines)] = before_lines
        inventory_after_fs = ''.join(fs_restored_lines).encode('utf-8')
        self.assertEqual(hashlib.sha256(inventory_after_fs).hexdigest(), fs_delta['baseline_sha256'])
        cfg_delta = self.c['reviewed_windows_lint_cfg_inventory_delta']
        self.assertEqual(cfg_delta['path'], inventory_path)
        self.assertEqual(cfg_delta['baseline_revision'], 'e93dc7bfe6db8a72a357e759d006fdcea3447d62')
        self.assertEqual(cfg_delta['baseline_sha256'], '1973ea452608f49bbc7aa3cba8910d4282cbdc9e5bf9584836d791f68edf5389')
        self.assertTrue(cfg_delta['exact_inverse_required'])
        cfg_fixture_bytes = (ROOT / cfg_delta['fixture']).read_bytes()
        self.assertEqual(hashlib.sha256(cfg_fixture_bytes).hexdigest(), cfg_delta['fixture_sha256'])
        cfg_fixture = json.loads(cfg_fixture_bytes)
        self.assertEqual(cfg_fixture['schema'], 'forever-windows-lint-cfg-inverse-delta-v1')
        self.assertEqual(cfg_fixture['path'], inventory_path)
        self.assertEqual(cfg_fixture['baseline_revision'], cfg_delta['baseline_revision'])
        self.assertEqual(cfg_fixture['baseline_sha256'], cfg_delta['baseline_sha256'])
        self.assertEqual(cfg_fixture['candidate_sha256'], cfg_delta['candidate_sha256'])
        self.assertEqual(cfg_fixture['scope'], cfg_delta['scope'])
        self.assertEqual(hashlib.sha256(inventory_after_fs).hexdigest(), cfg_delta['candidate_sha256'])
        cfg_restored_lines = inventory_after_fs.decode('utf-8').splitlines(keepends=True)
        for hunk in reversed(cfg_fixture['hunks']):
            after_lines = hunk['after_lines']
            before_lines = hunk['before_lines']
            self.assertTrue(after_lines or before_lines)
            matches = [
                index
                for index in range(len(cfg_restored_lines) - len(after_lines) + 1)
                if cfg_restored_lines[index:index + len(after_lines)] == after_lines
            ]
            self.assertEqual(len(matches), 1, 'each Windows-lint cfg hunk must invert exactly once')
            start = matches[0]
            cfg_restored_lines[start:start + len(after_lines)] = before_lines
        inventory_linux = ''.join(cfg_restored_lines).encode('utf-8')
        self.assertEqual(hashlib.sha256(inventory_linux).hexdigest(), cfg_delta['baseline_sha256'])
        linux_delta = self.c['reviewed_linux_selected_navigation_inventory_delta']
        self.assertEqual(linux_delta['path'], inventory_path)
        self.assertEqual(
            linux_delta['baseline_revision'],
            'c8a86b289c463fca7b9dd5ab6d22c275f75faa81',
        )
        self.assertEqual(linux_delta['baseline_sha256'], admission['current_sha256'])
        self.assertEqual(pins[inventory_path], linux_delta['baseline_sha256'])
        self.assertEqual(
            linux_delta['scope'],
            'Linux build-70450 selected partial-coverage navigation owner and cumulative preprocessing accounting; no Windows behavior or acceptance change',
        )
        self.assertTrue(linux_delta['exact_inverse_required'])
        fixture_path = ROOT / linux_delta['fixture']
        fixture_bytes = fixture_path.read_bytes()
        self.assertEqual(hashlib.sha256(fixture_bytes).hexdigest(), linux_delta['fixture_sha256'])
        fixture = json.loads(fixture_bytes)
        self.assertEqual(fixture['schema'], 'forever-selected-navigation-source-delta-v1')
        self.assertEqual(fixture['path'], inventory_path)
        self.assertEqual(fixture['baseline_revision'], linux_delta['baseline_revision'])
        self.assertEqual(fixture['baseline_sha256'], linux_delta['baseline_sha256'])
        self.assertEqual(fixture['candidate_sha256'], linux_delta['candidate_sha256'])
        self.assertEqual(fixture['scope'], linux_delta['scope'])
        self.assertEqual(hashlib.sha256(inventory_linux).hexdigest(), linux_delta['candidate_sha256'])
        restored_lines = inventory_linux.decode('utf-8').splitlines(keepends=True)
        hunks = fixture['hunks']
        self.assertTrue(hunks)
        for hunk in reversed(hunks):
            after_lines = hunk['after_lines']
            before_lines = hunk['before_lines']
            self.assertTrue(after_lines or before_lines)
            matches = [
                index
                for index in range(len(restored_lines) - len(after_lines) + 1)
                if restored_lines[index:index + len(after_lines)] == after_lines
            ]
            self.assertEqual(len(matches), 1, 'each Linux delta hunk must invert exactly once')
            start = matches[0]
            restored_lines[start:start + len(after_lines)] = before_lines
        inventory = ''.join(restored_lines).encode('utf-8')
        self.assertEqual(hashlib.sha256(inventory).hexdigest(), linux_delta['baseline_sha256'])
        before_admission = b'if !matches!(build, 70170 | 70205) {'
        after_admission = b'if !matches!(build, 70170 | 70205) && !(cfg!(target_os = "linux") && build == 70450) {'
        self.assertEqual(inventory.count(after_admission), 1)
        self.assertEqual(
            hashlib.sha256(inventory.replace(after_admission, before_admission, 1)).hexdigest(),
            admission['previous_sha256'],
        )
        self.assertEqual(hashlib.sha256(inventory).hexdigest(), admission['current_sha256'])
        reviewed[inventory_path] = fs_delta['candidate_sha256']
        consumer = json.loads((ROOT / 'contracts/forever-rustycore-navigation-consumer-contract-v1.json').read_text())
        shared_deltas = {entry['path']: entry for entry in consumer['preserved_windows_source_deltas']}
        self.assertEqual(set(shared_deltas), {'Cargo.lock', 'crates/forever-navigation/Cargo.toml'})
        for name, expected in pins.items():
            expected = reviewed.get(name, expected)
            current = (ROOT / name).read_bytes()
            if name in shared_deltas:
                delta = shared_deltas[name]
                self.assertEqual(delta['baseline_sha256'], expected)
                self.assertEqual(hashlib.sha256(current).hexdigest(), delta['candidate_sha256'])
                after = delta['after'].encode('utf-8')
                self.assertEqual(current.count(after), 1)
                current = current.replace(after, delta['before'].encode('utf-8'), 1)
            self.assertEqual(hashlib.sha256(current).hexdigest(), expected, name)

    def test_only_reviewed_local_dependency_and_exact_registry_closure(self):
        m = tomllib.loads((ROOT / 'crates/forever-navigation/Cargo.toml').read_text())
        self.assertEqual(m['target']['cfg(windows)']['dependencies'], {'forever-platform': {'path': '../forever-platform'}})
        packages = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
        self.assertEqual(len(packages), 9)
        self.assertEqual(len(literal('CRATES')), 7)
        self.assertEqual(next(p for p in packages if p['name'] == 'forever-navigation')['dependencies'], ['forever-casc', 'forever-db2', 'forever-platform', 'forever-terrain', 'forever-vmap'])
        registry = {p['name'] + '-' + p['version']: p['checksum'] for p in packages if 'source' in p}
        self.assertEqual(registry, literal('DEPENDENCIES'))
        self.assertEqual({p['name']: p['version'] for p in packages if 'source' in p}, self.c['dependency_graph']['registry_packages'])

    def test_safe_exports_complete_reads_eof_and_digest(self):
        for name in ['inventory.rs', 'terrain/load.rs']:
            self.assertIn('#[cfg(windows)]\npub mod windows;', (ROOT / 'crates/forever-navigation/src' / name).read_text())
        for text in ['pub struct RootedSource', 'pub struct FsLimits', 'pub struct FsStats', 'impl Source for RootedSource', 'file.read_exact_at(', 'file.verify_eof()', 'hash::Sha256::new()']:
            self.assertIn(text, self.terrain)
        for text in ['pub struct Package', 'pub struct Limits', 'pub struct Statistics']:
            self.assertIn(text, self.inventory)
        self.assertIn('lease.verify_eof()', self.guard)
        self.assertIn('hash::Sha256::new()', self.inventory)
        self.assertEqual((self.c['policy']['serial_handles'], self.c['policy']['resident_handles_between_operations']), (65, 0))

    def test_protected_final_work_and_changed_result_priority(self):
        read = self.terrain.split('let mut cleanup = self.reserve_after', 1)[1]
        self.assertLess(read.index('ReadFile::open'), read.index('self.finish_read'))
        reserve = self.terrain.split('fn reserve_after(', 1)[1].split('fn finish_read', 1)[0]
        self.assertIn('work.used += reservation.used', reserve)
        exact = self.terrain.split('fn exact_name(', 1)[1].split('fn read_present', 1)[0]
        self.assertLess(exact.index('// protected final check'), exact.index('ReadDirectory::open'))
        self.assertIn('finish_directory(lease, result)', exact)
        finish = self.terrain.split('fn finish_read<T>', 1)[1].split('impl Source', 1)[0]
        self.assertLess(finish.index('drop(file)'), finish.index('self.guard_after'))
        self.assertIn('Err(SourceError::InputChanged)', finish)
        tile = self.inventory.split('fn finish_tile<T>', 1)[1].split('fn map_id', 1)[0]
        for text in ['let after = self.verify_unchanged();', 'error.changed()', 'self.failed = true;', 'return result;', 'after?;']:
            self.assertIn(text, tile)
        self.assertLess(tile.index('return result;'), tile.index('after?;'))

    def test_downward_ceilings_and_logical_accounting(self):
        t = self.c['limits']['terrain']
        self.assertEqual((t['metadata_bytes'], t['entries'], t['root_bytes'], t['verify_steps'], t['chunk_bytes']), (128 * 1024 * 1024, 65536, 4096, 262144, 16384))
        for text in ['MAX_METADATA_BYTES: usize = 128 * 1024 * 1024', 'MAX_ENTRIES: usize = 65_536', 'MAX_ROOT_BYTES: usize = 4096', 'MAX_VERIFY_STEPS: usize = 262_144', 'CHUNK_BYTES: usize = 16 * 1024', 'LEASE_BYTES: usize = 256 * 1024']:
            self.assertIn(text, self.terrain)
        for text in ['entries: 1_000_000', 'tilelists: 4096', 'root_bytes: 4096', 'heap_bytes: 256 * 1024 * 1024', 'metadata_calls: 8_000_000']:
            self.assertIn(text, self.inventory)
        self.assertIn('SCRATCH: usize = 256 * 1024', self.guard)
        self.assertTrue(self.c['limits']['only_downward'])
        self.assertIn('RSS', self.c['limits']['scope'])
        self.assertIn('EOF probe bytes excluded', self.c['limits']['read_statistics'])

    def test_strict_filesystem_profile_and_inherited_error_boundary(self):
        for text in ['operator_path(', 'ReadDirectory::open(', 'ReadFile::open(', 'name.len() > 255', '0x20..=0x7e']:
            self.assertIn(text, self.guard)
        self.assertIn('stricter than Linux', self.c['policy']['inventory_names'])
        self.assertIn('not an atomic snapshot', self.c['policy']['identity'])
        self.assertIn('open_rows/source-setup', self.c['policy']['early_error_boundary'])
        self.assertIn('no atomic concurrent-mutation exclusion', self.c['policy']['early_error_boundary'])

    def test_sequential_provisional_sink_and_frozen_case_counts(self):
        for text in ['pub enum Selection', 'pub trait Sink', 'pub fn execute<S: Sink>', 'pub struct Completed']:
            self.assertIn(text, self.run_source)
        self.assertIn('callbacks provisional until Completed', self.c['api']['sink'])
        base = ROOT / 'crates/forever-navigation/src'
        counts = [(base / n).read_text().count('#[test]') for n in ['inventory/windows/run/tests.rs', 'inventory/windows/tests.rs', 'terrain/load/windows/tests.rs', 'terrain/load/fs/hash.rs']]
        self.assertEqual(counts, [22, 17, 14, 3])
        self.assertEqual(sum(counts), 56)

    def test_semantic_success_does_not_erase_network_or_unknown_failure(self):
        w = self.c['validation']['wine']
        self.assertTrue(w['terminal'] and w['semantic_passed'])
        self.assertEqual((w['debug_passed'], w['release_passed'], w['total_passed'], w['timeouts'], w['not_run']), (56, 56, 112, 0, 0))
        for field in ['overall_accepted', 'strict_zero_nonlocal_attempts', 'trace_evidence_complete', 'original_trace_complete_claim_accepted']:
            self.assertFalse(w[field])
        self.assertEqual((w['denied_nonlocal_attempts'], w['unknown_syscall_bodies']), (783, 40))
        self.assertIn('no safe/nonnetwork assumption', w['unknown_scope'])
        for field in ['report_sha256', 'root_semantic_review_sha256', 'root_trace_body_audit_sha256', 'independent_trace_body_audit_sha256', 'network_qualification_sha256']:
            self.assertEqual(len(w[field]), 64)
        d = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())['source']['windows_navigation_evidence']
        self.assertEqual((d['synthetic_tests_passed'], d['unknown_syscall_bodies'], d['denied_nonlocal_attempts']), (112, 40, 783))
        self.assertFalse(d['overall_accepted'] or d['trace_evidence_complete'])
        self.assertIn('rejected by root', d['qualification'])


if __name__ == '__main__':
    unittest.main()
