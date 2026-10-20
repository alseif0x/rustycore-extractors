# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-12: original product disk plan/source consistency checks.
"""Static consistency only; no client, compiler, runtime or bundle acceptance."""
import ast
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'forever-seven-crate-linux-product-collision-disk-source-v9'
CURRENT_PROFILE = 'forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-consumer-source-v13'
HISTORICAL = ('forever-seven-crate-windows-navigation-readers-source-v7',
              'forever-seven-crate-linux-collision-leased-archive-source-v8')


def read(path):
    return (ROOT / path).read_text()


def literal(name):
    for node in ast.parse(read('tools/package_forever.py')).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class ProductDiskContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-product-disk-contract-v1.json'))
        cls.d = json.loads(read('contracts/forever-distribution-contract-v1.json'))

    def test_explicit_product_version_keeps_acceptance_gates_separate(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-product-collision-disk-v1', 1))
        self.assertEqual(self.c['product_contract'], 'forever-product-v3')
        self.assertTrue(self.c['original_implementation'])
        for key in ['network', 'full_extractor_complete', 'real_client_accepted', 'full_runtime_accepted',
                    'windows_product_enabled', 'bundle_built', 'publication_authorized']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        verifier = read('crates/forever-extractor/src/verify.rs')
        self.assertIn('limits.total_bytes = budget.output_bytes;', verifier)
        self.assertIn('package::verify_with(output, expected, limits,', verifier)
        self.assertEqual(self.c['validation']['static_checks'], 'source and contract consistency only')

    def test_mandatory_v9_closure_preserves_historical_profiles(self):
        self.assertEqual(literal('SOURCE_PROFILE'), CURRENT_PROFILE)
        self.assertEqual(self.c['distribution_source_profile'], PROFILE)
        self.assertEqual(self.d['source']['profile'], CURRENT_PROFILE)
        self.assertIn(PROFILE, self.d['source']['prior_profiles'])
        self.assertEqual(tuple(self.c['sources']), literal('PRODUCT_COLLISION_DISK_SOURCE'))
        for profile in HISTORICAL:
            self.assertIn(profile, self.d['source']['prior_profiles'])
        archive = json.loads(read('contracts/forever-linux-collision-archive-contract-v1.json'))
        self.assertEqual(archive['distribution_source_profile'], HISTORICAL[1])
        self.assertFalse(archive['product_cli_enabled'])
        for name in self.c['sources']:
            self.assertTrue((ROOT / name).is_file(), name)
            self.assertIn(name, self.d['source']['required'])

    def test_exact_package_and_registry_closure_and_linux_local_edge(self):
        self.assertEqual(len(literal('CRATES')), 7)
        packages = tomllib.loads(read('Cargo.lock'))['package']
        self.assertEqual(len(packages), 9)
        self.assertEqual({p['name'] + '-' + p['version']: p['checksum'] for p in packages if 'source' in p}, literal('DEPENDENCIES'))
        manifest = tomllib.loads(read('crates/forever-extractor/Cargo.toml'))
        edge = {'forever-platform': {'path': '../forever-platform'}}
        self.assertEqual(manifest['target']['cfg(any(windows, target_os = "linux"))']['dependencies'], edge)
        self.assertEqual(self.d['source']['linux_dependency_edges']['forever-extractor'], ['forever-platform'])
        self.assertIn('forever-platform', next(p for p in packages if p['name'] == 'forever-extractor')['dependencies'])

    def test_v3_canonical_storage_and_seven_downward_budgets(self):
        p = self.c['plan']
        self.assertEqual(p['version'], 3)
        self.assertEqual(p['requires_component'], 'collision')
        self.assertEqual(p['decryption'], ['disabled', 'provided-salsa20-v1'])
        self.assertFalse(p['paths_and_key_material'])
        self.assertTrue(p['old_v1_v2_grammar_and_resident_default_preserved'])
        self.assertTrue(p['canonical_plan_hash_binds_storage_and_budgets'])
        self.assertEqual((p['minimum_spool_bytes'], p['maximum_spool_bytes']), (32, 128 * 1024**3))
        self.assertEqual((p['minimum_output_bytes'], p['maximum_output_bytes']), (1, 64 * 1024**3))
        self.assertIn('whole common package', p['output_budget_scope'])
        source = read('crates/forever-extractor/src/plan/disk.rs')
        for text in ['Resident', 'LeasedDiskV1', 'pub struct DiskBudget', p['storage']]:
            self.assertIn(text, source)
        for key in p['ordered_budget_keys']:
            self.assertIn(key, source)
        self.assertEqual(len(p['ordered_budget_keys']), 7)

    def test_explicit_scratch_and_existing_publication_protocol(self):
        cli = self.c['cli']
        self.assertEqual(cli['scratch_flag'], '--scratch')
        self.assertEqual(cli['scratch_required'], ['extract', 'resume'])
        self.assertTrue(cli['scratch_forbidden_for_resident'] and cli['inspection_and_verification_need_no_scratch'])
        self.assertFalse(cli['automatic_storage_selection'])
        self.assertIn('--scratch', read('crates/forever-extractor/src/main.rs'))
        p = self.c['pipeline']
        self.assertEqual(p['owners'], ['raw', 'wdt', 'cache', 'placement', 'vmo', 'map'])
        self.assertEqual((p['backend'], p['volume_profile'], p['resource_profile']), ('LeasedAppendV1', 'CapturedVolumesV2', 'DiskWorldV1'))
        self.assertTrue(p['complete_assembly_before_collision_put'] and p['native_output_names_preserved'])
        self.assertFalse(p['available_prefix_fallback'])
        self.assertIn('new exclusive private', p['namespace_policy'])
        self.assertIn('neither adopted nor removed', p['namespace_policy'])
        self.assertIn('canonical plan', p['resume_identity'])
        source = read('crates/forever-extractor/src/driver/disk.rs')
        for text in ['CapturedVolumesV2', 'DiskWorldV1', 'LeasedAppendV1',
                     'gameobjects::spooled::extract_startup_fdid', 'staged::extract',
                     'staged::models::prepare', 'staged::package::assemble', 'Cause::Callback']:
            self.assertIn(text, source)
        self.assertLess(source.index('staged::package::assemble'), source.index('run.put('))

    def test_borrowed_space_observation_is_not_reservation_or_snapshot(self):
        p = self.c['physical_preflight']
        source = read('crates/forever-platform/src/linux/space.rs')
        self.assertEqual(p['api_profile'], 'linux-gnu-x64-directory-available-space-v1')
        self.assertIn(p['api_profile'], source)
        self.assertIn('pub fn available(directory: &File)', source)
        self.assertIn('pub allocation_unit: u64', source)
        self.assertIn('pub inodes: Option<u64>', source)
        self.assertIn('checked_mul(fragment_size)', source)
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', source)
        self.assertIn('target_env = "gnu"', source)
        self.assertIn('target_pointer_width = "64"', source)
        for key in ['borrowed_directory_descriptor', 'checked_available_blocks_times_fragment_size',
                    'aggregate_same_device_budgets', 'before_client_acquisition_and_output_creation',
                    'source_output_scratch_disjoint_and_rooted_guards']:
            self.assertTrue(p[key])
        for key in ['atomic_reservation', 'guaranteed_capacity', 'filesystem_metadata_or_rss_measurement']:
            self.assertFalse(p[key])
        self.assertFalse(self.c['memory']['ceiling_raised'])
        m = self.c['memory']
        self.assertTrue(m['common_run_live_capacity_reserved_by_driver'])
        self.assertEqual(m['combined_ceiling_bytes'], 2 * 1024**3)
        self.assertEqual(m['conversion_ceiling_bytes'], m['combined_ceiling_bytes'] - m['common_run_allowance_bytes'] - m['guard_allowance_bytes'])

    def test_complete_real_inputs_runtime_and_distribution_remain_open(self):
        doc = read('docs/forever-product-disk.md')
        for text in ['Map', 'SkillLineAbility', 'GameObjectDisplayInfo', 'not complete',
                     'not an atomic reservation', 'v1/v2', 'new scratch namespace']:
            self.assertIn(text, doc)
        self.assertFalse(self.c['validation']['real_client_or_native_executed'])
        self.assertFalse(self.c['validation']['runtime_accepted'])


if __name__ == '__main__':
    unittest.main()
