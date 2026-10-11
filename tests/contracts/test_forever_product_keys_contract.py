"""GPL-3.0-or-later. Authored 2026-10-11: explicit supplied-key product contract."""
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[2]
class ProvidedKeyProductContract(unittest.TestCase):
    def setUp(self):
        self.old=json.loads((ROOT/'contracts/forever-product-contract-v1.json').read_text())
        self.c=json.loads((ROOT/'contracts/forever-product-contract-v2.json').read_text())
    def test_v2_capability_is_explicit_and_v1_profile_stays_closed(self):
        self.assertEqual((self.c['contract'],self.c['version']),('forever-product-v2',2))
        self.assertEqual(self.c['plan']['ordered_keys'][4:7],['root','decryption','components'])
        self.assertNotIn('decryption',self.old['plan']['ordered_keys'])
        self.assertIn('provided-salsa20-v1 only',self.c['plan']['decryption'])
        self.assertFalse(self.c['full_extractor_complete'])
    def test_key_input_is_explicit_bounded_and_has_no_discovery(self):
        k=self.c['provided_keys'];self.assertEqual(k['option'],'--keys PATH')
        self.assertEqual(k['required_for'],['extract','resume','inspect --client'])
        self.assertEqual(k['forbidden_for'],['v1 plans','verify','inspect without --client'])
        self.assertEqual((k['max_file_bytes'],k['max_entries']),(262144,4096))
        self.assertFalse(k['discovery']);self.assertFalse(k['built_in_keys']);self.assertFalse(k['network'])
        self.assertIn('protected roots',k['source_protection'])
    def test_no_key_material_is_put_in_plan_identity_or_manifest(self):
        self.assertIn('excluded from CLI JSON, plans, package context and manifests',self.c['identities']['keys'])
        self.assertIn('actual keyset is physical availability',self.c['identities']['keys'])
        self.assertEqual(self.c['package'],self.old['package'])
        self.assertIn('different supplied set is allowed',self.c['resume'])
        self.assertIn('complete CKey',self.c['resume'])
    def test_unchanged_producers_formats_and_limits_do_not_claim_runtime(self):
        for key in ['builds','locales','commands','limits','verification']:
            self.assertEqual(self.c[key],self.old[key])
        self.assertEqual(self.c['plan']['dependencies'],self.old['plan']['dependencies'])
        self.assertEqual(self.c['verification']['scope'],'artifact_integrity_and_intrinsic_formats')
        self.assertIn('runtime acceptance',self.c['verification']['not_claimed'])
if __name__=='__main__':unittest.main()
