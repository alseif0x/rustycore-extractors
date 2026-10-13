# SPDX-License-Identifier: GPL-3.0-or-later
"""Static v6 offmesh contract/source consistency, without runtime acceptance.

No client reads, compilation, native execution, geometry or network. Separate
synthetic CLI evidence is required before changing acceptance status.
"""
import hashlib
import json
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[2]
LEGACY_SHA256 = {
    1: 'e0de29feebab291630b6d6286cae251be1f15633c7c383ecaa9e37f2b8cd793f',
    2: 'acca7a217dfb9e074bf025237f7af6bd5bb23aa95d2627015bf54d955f140481',
    4: '86e090dbe9d11bde1e752435ffed9969d0d1fcbfd8e7224aca8e8c0a697010db',
    5: '925732c9dce67b9a84ac013ac6e91a3cc4fc61e5c2c4d218dd4aa5c00233e229',
}


def read(path):
    return (ROOT / path).read_text()


class ProductOffmeshContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-product-contract-v6.json'))
        cls.plan = read('crates/forever-extractor/src/plan.rs')
        cls.offmesh = read('crates/forever-extractor/src/plan/offmesh.rs')
        cls.driver = read('crates/forever-extractor/src/driver.rs')
        cls.verifier = read('crates/forever-extractor/src/verify.rs')

    def test_new_capability_keeps_platform_and_acceptance_gates_explicit(self):
        c = self.c
        self.assertEqual((c['contract'], c['version']), ('forever-product-v6', 6))
        self.assertEqual((c['backend'], c['platform']), ('rust-local', 'linux'))
        self.assertTrue(c['offline'])
        self.assertFalse(c['full_extractor_complete'])
        self.assertEqual(c['commands'], ['inspect', 'extract', 'resume', 'verify'])
        self.assertEqual(c['verification']['scope'], 'artifact_integrity_and_intrinsic_formats')
        self.assertEqual(c['validation']['static_checks'], 'source and contract consistency only')
        for key in ('native_offmesh_executed', 'real_client_routes_accepted',
                    'runtime_accepted', 'v6_bundle_built', 'publication_authorized'):
            self.assertFalse(c['validation'][key], key)
        self.assertIn('separate retained QA evidence', c['validation']['synthetic_cli_acceptance'])
        self.assertEqual(c['validation']['new_registry_dependencies'], [])

    def test_legacy_catalog_and_build_limits_are_not_relabelled(self):
        p = self.c['plan']
        metadata = json.loads(read(p['schema_catalog']['metadata_contract']))
        self.assertEqual(self.c['builds'], [70170, 70205])
        self.assertEqual(p['schema_catalog']['id'], 'pinned-metadata-v1')
        self.assertEqual(metadata['catalog_id'], p['schema_catalog']['id'])
        self.assertEqual(len(metadata['tables']), 1126)
        self.assertEqual(p['schema_catalog']['declaration_count'], 1126)
        self.assertEqual(p['max_db2_entries'], 1126)
        self.assertIn('forbidden', p['schema_catalog']['schema_catalog_line'])
        self.assertRegex(self.plan, r'pub const MAX_DB2_ENTRIES: usize = 1126;')
        self.assertIn('product_plan_offmesh_scope', self.plan)

    def test_canonical_wire_header_connections_then_existing_db2_tail(self):
        p = self.c['plan']
        self.assertEqual(p['encoding'], 'ASCII canonical TSV with final LF')
        self.assertEqual(p['max_bytes'], 128 * 1024)
        self.assertEqual(p['ordered_keys'], ['forever-product', 'build', 'locale', 'index', 'root',
                         'decryption', 'components', 'terrain', 'pack', 'navigation',
                         'big-base-unit', 'skip-liquids', 'maximum-slope', 'non-steep-slope', 'offmesh'])
        off = p['offmesh_connections']
        self.assertEqual(off['header'], 'offmesh TAB inline-f32-bits-v1 LF')
        self.assertEqual(off['record_key'], 'offmesh-connection')
        self.assertIn('connection records precede all db2 records', off['position'])
        self.assertEqual(off['ordered_fields'], ['MAP', 'ROW', 'COLUMN', 'FROMXbits', 'FROMYbits',
                         'FROMZbits', 'TOXbits', 'TOYbits', 'TOZbits', 'RADIUSbits', 'DIRECTION', 'AREA', 'FLAGS'])
        self.assertEqual((off['field_separator'], off['record_terminator']), ('TAB', 'LF'))
        for literal in ('inline-f32-bits-v1', 'offmesh-connection'):
            self.assertIn(literal, self.plan + self.offmesh)
        self.assertIn('plan.render() != input', self.plan)
        self.assertRegex(self.plan, r'pub const MAX_PLAN_BYTES: usize = 128 \* 1024;')

    def test_all_float_bits_and_integer_admission_are_closed(self):
        off = self.c['plan']['offmesh_connections']
        self.assertEqual(off['float_fields'], ['FROMXbits', 'FROMYbits', 'FROMZbits',
                                              'TOXbits', 'TOYbits', 'TOZbits', 'RADIUSbits'])
        self.assertIn('eight lowercase hexadecimal', off['float_encoding'])
        self.assertIn('canonical unsigned decimal', off['integer_encoding'])
        self.assertEqual(off['map'], 'u32')
        self.assertEqual((off['row'], off['column']), ([0, 63], [0, 63]))
        self.assertEqual((off['direction'], off['area'], off['flags']), ([0, 1], [0, 63], [0, 65535]))
        self.assertIn('finite', off['endpoints'])
        self.assertIn('strictly greater than zero', off['radius'])
        self.assertIn('even when', off['all_records_validated'])
        for literal in ('product_plan_offmesh_bits', 'from_bits', 'is_finite', '08x'):
            self.assertIn(literal, self.offmesh)
        self.assertRegex(self.offmesh, r'\[u32;\s*3\]')
        self.assertIn('to_bits', self.offmesh)

    def test_option_even_empty_selects_v6_and_preserves_legacy_contracts(self):
        off = self.c['plan']['offmesh_connections']
        self.assertEqual((off['minimum_records'], off['maximum_records']), (0, 1024))
        self.assertIn('Some including an empty vector selects v6', off['representation'])
        self.assertRegex(self.plan, r'pub offmesh:\s*Option<Vec<OffMeshConnection>>')
        self.assertRegex(self.plan + self.offmesh, r'MAX_OFFMESH_CONNECTIONS:\s*usize\s*=\s*1024')
        self.assertIn('forever-product-v6', self.plan)
        self.assertIn('self.offmesh.is_some()', self.plan)
        self.assertEqual(self.c['legacy_contracts_unchanged'], [f'forever-product-v{v}' for v in range(1, 6)])
        for version, expected in LEGACY_SHA256.items():
            path = ROOT / f'contracts/forever-product-contract-v{version}.json'
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), expected, path.name)
        self.assertEqual(json.loads(read('contracts/forever-product-disk-contract-v1.json'))['product_contract'],
                         'forever-product-v3')

    def test_navigation_dependencies_remain_required(self):
        p = self.c['plan']
        self.assertTrue(p['requires_navigation'])
        self.assertEqual(p['dependencies']['mmaps'], ['db2', 'maps', 'vmaps', 'Map', 'LiquidType', 'terrain=all'])
        self.assertEqual(p['dependencies']['cameras'], ['db2', 'CinematicCamera'])
        for text in ('Component::Navigation', 'SchemaCatalog::PinnedMetadataV1', 'Selection::All',
                     'has_db2("Map")', 'has_db2("LiquidType")'):
            self.assertIn(text, self.plan)

    def test_decryption_and_optional_disk_storage_keep_existing_grammar(self):
        p = self.c['plan']
        self.assertEqual(p['decryption'], {'required': True, 'position': 'immediately after root',
                                         'values': ['disabled', 'provided-salsa20-v1']})
        disk = json.loads(read('contracts/forever-product-disk-contract-v1.json'))
        storage = p['collision_storage']
        self.assertTrue(storage['optional'])
        self.assertEqual(storage['omitted'], 'resident')
        self.assertEqual(storage['position'], 'after decryption and before components')
        self.assertEqual(storage['explicit_profile'], disk['plan']['storage'])
        self.assertEqual(storage['ordered_budget_keys'], disk['plan']['ordered_budget_keys'])
        for name in ('minimum_spool_bytes', 'maximum_spool_bytes', 'minimum_output_bytes', 'maximum_output_bytes'):
            self.assertEqual(storage[name], disk['plan'][name])
        cli = read('crates/forever-extractor/src/main.rs')
        for code in ('product_keys_required', 'product_keys_forbidden', 'product_scratch_required', 'product_scratch_forbidden'):
            self.assertIn(code, cli)
        self.assertFalse(self.c['provided_keys']['network'])
        self.assertFalse(self.c['provided_keys']['discovery'])

    def test_driver_passes_validated_sources_and_rejects_nonempty_empty_world(self):
        p = self.c['pipeline']
        self.assertEqual(p['driver_helper'], 'navigation_connections')
        self.assertEqual(p['empty_map_error'], 'product_offmesh_empty_map')
        self.assertIn('fn navigation_connections', self.driver)
        self.assertIn('product_offmesh_empty_map', self.driver)
        self.assertGreaterEqual(self.driver.count('navigation_connections('), 2)
        nav_call = self.driver[self.driver.index('    nav::execute('):]
        self.assertIn('&connections,', nav_call.split('nav::Options', 1)[0])
        self.assertIn('inline connection list is empty', self.c['empty_world'])
        self.assertIn('no implicit file, native or network source', p['sources'])

    def test_verifier_compares_exact_map_subset_bits_order_and_multiplicity(self):
        off = self.c['verification']['offmesh']
        self.assertEqual(off['helper'], 'verify_offmesh_sources')
        self.assertEqual(off['mismatch'], 'product_offmesh_mismatch')
        for text in ('verify_offmesh_sources', 'OffMeshConnection::from_source',
                     'product_offmesh_mismatch', 'product_offmesh_unsupported'):
            self.assertIn(text, self.verifier)
        self.assertIn('.filter(', self.verifier)
        self.assertIn('all f32 bits', self.c['verification']['mmaps'])
        self.assertIn('order and multiplicity', self.c['verification']['mmaps'])
        self.assertIn('duplicates and order', off['comparison'])
        self.assertIn('no sorting or deduplication', self.c['plan']['offmesh_connections']['order'])

    def test_canonical_plan_identity_binds_resume_and_exact_source_bits(self):
        self.assertEqual(self.c['identities']['plan'], 'SHA-256 of the complete canonical TSV bytes')
        self.assertIn('signed-zero bits, order and duplicates', self.c['resume'])
        self.assertIn('explicit empty v6 header', self.c['identities']['offmesh'])
        self.assertIn('context.plan_sha256 != integrity::digest(plan.render().as_bytes())', self.verifier)
        self.assertEqual(self.c['package'], {'manifest': 'forever-package.json', 'final_version': 1, 'state_version': 2})
        self.assertIn('generic CLI/plan failures before admission retain forever-product-v1', self.c['failure_envelope'])

    def test_heap_and_work_ceilings_are_unchanged(self):
        memory = self.c['memory_and_work']
        disk = json.loads(read('contracts/forever-product-disk-contract-v1.json'))['memory']
        self.assertFalse(memory['ceilings_raised'])
        self.assertEqual(memory['plan_bytes'], 131072)
        self.assertEqual(memory['maximum_inline_connections'], 1024)
        for name in ('combined_ceiling_bytes', 'common_run_allowance_bytes', 'guard_allowance_bytes', 'conversion_ceiling_bytes'):
            self.assertEqual(memory['disk_world_' + name], disk[name])
        source = read('crates/forever-navigation/src/offmesh.rs')
        self.assertIn('input_records: 65536,', source)
        self.assertIn('output_records: 65536,', source)
        self.assertIn('heap_bytes: 2 * 1024 * 1024,', source)
        self.assertIn('not allocator RSS', memory['scope'])

    def test_sources_and_failure_codes_are_present_without_new_acceptance_claims(self):
        for path in self.c['sources']:
            self.assertTrue((ROOT / path).is_file(), path)
        source = self.plan + self.offmesh + self.driver + self.verifier
        for code in self.c['failure_codes'].values():
            self.assertIn(code, source)
        for claim in ('native offmesh producer byte parity', 'endpoint snapping or routability',
                      'real-client offmesh routes', 'runtime acceptance', 'new v6 reproducible distribution acceptance'):
            self.assertIn(claim, self.c['verification']['not_claimed'])


if __name__ == '__main__':
    unittest.main()
