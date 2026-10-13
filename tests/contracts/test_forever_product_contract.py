"""GPL-3.0-or-later. Authored 2026-10-11: versioned product scope checks."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]

class ProductContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads((ROOT/'contracts/forever-product-contract-v1.json').read_text())

    def test_scope_is_explicit_and_not_full_extraction(self):
        c=self.contract
        self.assertEqual(c['contract'],'forever-product-v1')
        self.assertTrue(c['offline']);self.assertEqual(c['backend'],'rust-local')
        self.assertEqual(c['builds'],[70170,70205]);self.assertFalse(c['full_extractor_complete'])
        self.assertEqual(c['verification']['scope'],'artifact_integrity_and_intrinsic_formats')
        self.assertIn('runtime acceptance',c['verification']['not_claimed'])

    def test_package_and_selection_contracts_remain_explicit(self):
        c=self.contract
        self.assertEqual(c['package'],{'manifest':'forever-package.json','final_version':1,'state_version':2})
        self.assertEqual(c['plan']['components'],['db2','gt','cameras','maps','vmaps','mmaps'])
        self.assertEqual(c['plan']['dependencies']['mmaps'],['db2','maps','vmaps','Map','LiquidType','terrain=all'])
        self.assertEqual(c['plan']['max_bytes'],131072)
        self.assertEqual(c['plan']['max_db2_entries'],1126)

    def test_logical_source_does_not_claim_physical_availability(self):
        c=self.contract
        self.assertIn('not physical archive availability',c['identities']['source_scope'])
        self.assertIn('complete content MD5',c['identities']['source_scope'])
        self.assertIn('/proc/self/exe',c['identities']['producer'])

    def test_v5_is_separately_pinned_to_build_70338(self):
        c=json.loads((ROOT/'contracts/forever-product-contract-v5.json').read_text())
        self.assertEqual(c['contract'],'forever-product-v5')
        self.assertEqual(c['version'],5)
        self.assertEqual(c['builds'],[70338])
        self.assertFalse(c['full_extractor_complete'])
        self.assertEqual(c['plan']['components'],['db2'])
        self.assertIn('DB2-only',c['scope'])
        self.assertEqual(c['plan']['schema_catalog']['id'],'wowdbdefs-70338-v1')
        self.assertEqual(c['plan']['schema_catalog']['required_build'],70338)
        self.assertEqual(c['plan']['schema_catalog']['source_commit'],'d6f50d4f351193e84226aa62a36faa0f58528afa')
        self.assertEqual(c['plan']['schema_catalog']['facts_sha256'],'9fb8852f7ec15a5721ff17da4708970eacb54c5e79c4e78703891a2468905405')
        self.assertEqual(c['limits']['db2_target_high_volume']['catalog'],'wowdbdefs-70338-v1')

    def test_catalog_raw_text_validation_is_byte_preserving_and_target_scoped(self):
        for version, catalog in ((4, 'wowdbdefs-70205-v1'), (5, 'wowdbdefs-70338-v1')):
            c=json.loads((ROOT/f'contracts/forever-product-contract-v{version}.json').read_text())
            raw=c['limits']['db2_raw_text_validation']
            self.assertEqual(raw['catalogs'],[catalog])
            self.assertIn('zero or one',raw['sections'])
            self.assertIn('no UTF-8 conversion',raw['strings'])
            self.assertIn('preserved unchanged',raw['output'])
            self.assertIn('raw-text validator',c['verification']['db2'])

    def test_catalog_sparse_parent_raw_validation_is_exact_and_does_not_project(self):
        expected = [
            'CollectableSourceEncounterSparse',
            'CollectableSourceQuestSparse',
            'CollectableSourceVendorSparse',
        ]
        for version, catalog in ((4, 'wowdbdefs-70205-v1'), (5, 'wowdbdefs-70338-v1')):
            c=json.loads((ROOT/f'contracts/forever-product-contract-v{version}.json').read_text())
            raw=c['limits']['db2_sparse_parent_raw_validation']
            self.assertEqual(raw['catalogs'],[catalog])
            self.assertEqual(raw['tables'],expected)
            self.assertIn('one unencrypted sparse section',raw['sections'])
            self.assertIn('exact byte length',raw['parent_block'])
            self.assertIn('not interpreted',raw['parent_block'])
            self.assertIn('decoded rows returned',raw['strings'])
            self.assertIn('preserved unchanged',raw['output'])
            self.assertIn('sparse-parent raw validator',c['verification']['db2'])
            self.assertIn('pinned native C++ sparse parent materialization',raw['not_claimed'])

if __name__=='__main__':
    unittest.main()
