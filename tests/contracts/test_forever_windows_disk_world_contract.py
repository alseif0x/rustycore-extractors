# SPDX-License-Identifier: GPL-3.0-or-later
"""Closed source/contract boundaries; these checks are not native execution."""
import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


class WindowsDiskWorldContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.disk = json.loads(read('contracts/forever-windows-disk-world-contract-v1.json'))
        cls.product = json.loads(read('contracts/forever-product-contract-v7.json'))

    def test_explicit_versions_and_profiles(self):
        p, d = self.product, self.disk
        self.assertEqual((p['contract'], p['version']), ('forever-product-v7', 7))
        self.assertEqual((d['contract'], d['version']), ('forever-windows-collision-disk-v1', 1))
        self.assertEqual(d['product_contract'], p['contract'])
        self.assertEqual(p['plan']['collision_storage'], d['storage_profile'])
        self.assertEqual(d['storage_profile'], 'windows-exclusive-append-disk-world-v1')
        for key, path in [('backend_profile', d['sources']['spool']),
                          ('volume_profile', d['sources']['spool']),
                          ('resource_profile', d['sources']['resources'])]:
            self.assertIn(d[key], read(path))
        self.assertIn(d['physical_preflight']['space_profile'], read(d['sources']['space']))
        self.assertIn(d['backend_profile'], read(d['sources']['append']))

    def test_build_catalog_and_offmesh_scope_remain_explicit(self):
        p = self.product
        self.assertEqual(p['builds'], [70170, 70205])
        self.assertFalse(p['plan']['schema_catalog']['build_70338_admitted'])
        self.assertEqual(p['plan']['schema_catalog']['explicit'], 'wowdbdefs-70205-v1')
        self.assertEqual(p['plan']['schema_catalog']['explicit_build'], 70205)
        o = p['plan']['offmesh']
        self.assertTrue(o['requires_pinned_metadata'])
        self.assertTrue(o['requires_navigation'])
        self.assertFalse(o['catalog_extension'])
        v6 = json.loads(read(o['source_contract']))
        self.assertEqual(o['maximum_records'], v6['plan']['offmesh_connections']['maximum_records'])
        self.assertEqual(o['profile'], v6['plan']['offmesh_connections']['profile'])

    def test_existing_budget_bounds_and_whole_package_scope(self):
        p = self.product['plan']
        old = json.loads(read('contracts/forever-product-disk-contract-v1.json'))['plan']
        for field in ('ordered_budget_keys', 'minimum_spool_bytes', 'maximum_spool_bytes',
                      'minimum_output_bytes', 'maximum_output_bytes'):
            self.assertEqual(p[field], old[field])
        self.assertTrue(p['whole_product_output_budget'])
        self.assertTrue(p['collision_required'])
        self.assertFalse(p['automatic_storage_selection'])
        self.assertFalse(p['paths_and_key_material'])

    def test_linux_archive_sources_remain_byte_identical(self):
        for path, expected in self.disk['unchanged_linux_sha256'].items():
            with self.subTest(path=path):
                self.assertEqual(hashlib.sha256((ROOT / path).read_bytes()).hexdigest(), expected)
        source = read('crates/forever-casc/src/collision.rs')
        self.assertIn('#[cfg(target_os = "linux")]\npub mod spool;', source)
        self.assertIn('#[cfg(windows)]\n#[path = "collision/spool/windows.rs"]\npub mod spool;', source)

    def test_existing_product_publication_protocol_is_reused(self):
        p, d = self.product['package'], self.disk['publication']
        self.assertEqual((p['manifest_version'], p['windows_state_version'],
                          p['windows_transaction_version']), (1, 3, 3))
        self.assertEqual(p['filesystem_profile'], 'forever-windows-product-ntfs-v1')
        for key in ('new_context_field', 'state_migration', 'power_loss_durability_claimed'):
            self.assertFalse(p[key])
        self.assertFalse(d['linux_hardlink_publisher_ported'])
        self.assertFalse(d['power_loss_durability_claimed'])
        self.assertNotIn('hard_link(', read('crates/forever-casc/src/package/windows.rs'))

    def test_pending_extent_guarantees_are_bounded_and_distinct_from_kernel_access(self):
        a = self.disk['append']
        self.assertEqual((a['container_header_bytes'], a['record_header_bytes'],
                          a['digest_scratch_bytes']), (32, 64, 8192))
        for key in ('patch_committed_prefix', 'per_record_retained_file_metadata',
                    'fcntl_lease_or_oplock_claimed', 'same_sid_isolation', 'atomic_snapshot'):
            self.assertFalse(a[key])
        self.assertIn('not kernel-enforced', a['guarantee'])
        self.assertIn('SHARE0', a['handle'])
        self.assertIn('Sticky poison', a['uncommitted_drop_error_or_panic'])
        self.assertEqual(a['commit_order'][-1], 'commit ordinal')

    def test_scratch_is_fresh_only_and_failed_attempts_are_retained(self):
        n = self.disk['namespace']
        self.assertTrue(n['fresh_only'])
        self.assertTrue(n['exclusive_creation'])
        self.assertFalse(n['old_failed_scratch_adopted_or_removed'])
        s = self.product['scratch']
        self.assertEqual(s['required_for'], ['extract', 'resume'])
        self.assertEqual(s['not_required_for'], ['inspect', 'verify'])
        self.assertFalse(s['adopt_existing'])
        self.assertFalse(s['remove_old_failed_namespace'])

    def test_space_observation_has_distinct_profile_and_twenty_objects(self):
        p = self.disk['physical_preflight']
        self.assertTrue(p['fixed_local_ntfs_only'])
        self.assertTrue(p['quota_aware_caller_available_bytes'])
        self.assertTrue(p['same_volume_reservations_aggregated'])
        self.assertFalse(p['group_by_drive_letter'])
        self.assertFalse(p['observation_is_atomic_reservation'])
        self.assertFalse(p['capacity_guaranteed_after_preflight'])
        self.assertIsNone(p['inodes'])
        self.assertEqual(p['scratch_namespace_objects'], 20)
        self.assertEqual(p['endpoint_headroom_bytes'], 64 * 1024 * 1024)

    def test_combined_logical_ceiling_is_not_raised(self):
        l = self.disk['limits']
        self.assertTrue(l['downward_only'])
        self.assertEqual(l['combined_logical_bytes'], 2 * 1024**3)
        self.assertEqual(l['common_run_allowance_bytes'] + l['external_guard_allowance_bytes']
                         + l['conversion_ceiling_bytes'], l['combined_logical_bytes'])
        self.assertLessEqual(l['namespace_constructor_logical_bytes'] + l['preflight_allowance_bytes'],
                             l['external_guard_allowance_bytes'])
        self.assertIn('RSS bound', l['not_claimed'])
        self.assertTrue(l['capacity_growth_old_plus_new_charged'])

    def test_native_acceptance_and_full_goal_remain_separate(self):
        for c in (self.disk, self.product):
            self.assertFalse(c['full_extractor_complete'])
            self.assertFalse(c['validation']['native_disk_world_accepted'])
            self.assertFalse(c['validation']['real_client_accepted'])
            self.assertFalse(c['validation']['distribution_accepted'])
        d = self.disk
        for key in ('network_api', 'client_data_in_git', 'publication_authorized'):
            self.assertFalse(d[key])
        self.assertEqual(d['validation']['network_attempts'], 'not_measured')
        self.assertFalse(d['validation']['native_cpp_execution'])
        doc = read(d['documentation'])
        for marker in ('not native Windows/NTFS acceptance', 'during collision',
                       '4,096', '256 MiB', 'No release or client-data publication is authorized'):
            self.assertIn(marker, doc)

    def test_native_rust_inventories_are_closed_and_have_no_ignored_cases(self):
        groups = self.disk['validation']['native_test_groups']
        self.assertEqual(len(groups), 10)
        self.assertEqual(len({g['prefix'] for g in groups}), len(groups))
        for g in groups:
            source = read(g['source'])
            names = re.findall(r'#\[test\]\s*fn (\w+)\(', source)
            self.assertTrue(names)
            self.assertEqual(len(names), len(set(names)))
            self.assertEqual(names, g['names'])
            self.assertNotIn('#[ignore', source)
        all_names = {name for g in groups for name in g['names']}
        self.assertIn('disk_world_4100_unique_models_and_256_mib_sources_complete_all_six_owners', all_names)

    def test_native_cli_inventory_has_a_collision_interruption_boundary(self):
        source = read(self.disk['sources']['native_cli'])
        tree = ast.parse(source, feature_version=(3, 11))
        names = sorted(n.name for cls in tree.body if isinstance(cls, ast.ClassDef)
                       for n in cls.body if isinstance(n, ast.FunctionDef) and n.name.startswith('test_'))
        self.assertEqual(names, self.disk['validation']['cli_test_names'])
        self.assertEqual(len(names), 17)
        for marker in ('process.kill()', 'os.scandir(raw)', 'OWNERS <= entries',
                       "sizes.get('records.archive', 0) > 32",
                       'observed_raw_append_extent_with_six_owners_before_collision_seal',
                       'self.assertEqual(process.returncode, 1)',
                       'FOREVER_WINDOWS_DISK_OTHER_VOLUME_PARENT',
                       'self.assertNotEqual(parent.stat().st_dev, self.scratch.stat().st_dev)',
                       'self.assertEqual(snapshot(after[name]), files)',
                       "'WINDOWS_DISK_PRODUCT_CASE_REPORT '",
                       'not_measured'):
            self.assertIn(marker, source)
        self.assertNotIn('terminate_after_db2()', source)

    def test_native_helper_rejects_skips_and_matches_exact_compiled_counts(self):
        source = read(self.disk['sources']['native_script'])
        for marker in ("$volume.FileSystem -ne 'NTFS'", "$volume.DriveType -ne 'Fixed'",
                       '--offline', '--locked', '--test-threads=1', '-- --list',
                       'native disk test inventory mismatch', '0 failed; 0 ignored;',
                       'native disk CLI would skip', 'native disk CLI inventory mismatch',
                       '--features', 'windows-msvc-seal-v2',
                       '--profile forever-windows-producer-seal-msvc-v2',
                       "'WINDOWS_DISK_PRODUCT_RUN '", "'WINDOWS_DISK_PRODUCT_BINARY '",
                       "'WINDOWS_DISK_PRODUCT_SOURCE '"):
            self.assertIn(marker, source)
        workflow = json.loads(read('.github/workflows/forever-rust.yml'))
        job = workflow['jobs']['windows-disk-world']
        self.assertEqual(job['strategy']['matrix']['profile'], ['debug', 'release'])
        self.assertEqual(job['runs-on'], 'windows-2022')
        self.assertEqual(job['timeout-minutes'], 20)
        self.assertEqual(job['steps'][-1]['run'],
                         ". ./tools/check_forever_windows_disk_world.ps1 -BuildProfile '${{ matrix.profile }}'")

    def test_fixture_plan_bytes_and_collision_window_are_authored_and_bounded(self):
        folder = ROOT / 'tests/forever'
        saved_path = sys.path[:]
        sys.path.insert(0, str(folder))
        try:
            spec = importlib.util.spec_from_file_location('windows_disk_cli_source_check',
                                                         folder / 'test_windows_disk_product_cli.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            for build in (70170, 70205):
                for locale in ('enUS', 'esES'):
                    for supplied in (False, True):
                        text = module.disk_plan(build, locale, supplied)
                        self.assertTrue(text.startswith('forever-product\t7\n'))
                        self.assertIn('collision-storage\twindows-exclusive-append-disk-world-v1\n', text)
                        self.assertNotIn('\r', text)
                        self.assertTrue(text.isascii())
                        self.assertTrue(text.endswith('\n'))
            payloads = module.collision_boundary_payloads()
            for fdid in (400, 401):
                self.assertEqual(len(payloads[fdid]), 16 * 1024 * 1024)
                self.assertTrue(payloads[fdid].startswith(module.world.m2()))
            self.assertNotEqual(payloads[400], payloads[401])
        finally:
            sys.path[:] = saved_path


if __name__ == '__main__':
    unittest.main()
