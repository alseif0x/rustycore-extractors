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

if __name__=='__main__':
    unittest.main()
