# SPDX-License-Identifier: GPL-3.0-or-later
"""Public v9 facts/source agreement only; authored TSV, no client data."""
from pathlib import Path
import hashlib,importlib.util,json,unittest
ROOT=Path(__file__).resolve().parents[2]
class ProductV9Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.v=json.loads((ROOT/'contracts/forever-product-contract-v9.json').read_text())
    def test_explicit_exact_profile_build_scope_and_legacy(self):
        v=self.v;self.assertEqual((v['version'],v['contract'],v['publication_profile']),(9,'forever-product-v9','Build70338Db2GtCamerasV1'));self.assertEqual(v['builds'],[70338]);self.assertEqual(v['plan']['components'],['db2','gt','cameras']);self.assertEqual(v['plan']['component_mask'],7);self.assertEqual(v['legacy_preservation']['canonical_versions'],[1,2,3,4,5,6,8]);self.assertEqual(json.loads((ROOT/'contracts/forever-product-contract-v5.json').read_text())['plan']['components'],['db2'])
    def test_fixed_current_component_contract_hashes(self):
        for name,profile in [('gt','gt-catalog-70338-v1'),('cameras','camera-raw-70338-v1')]:
            c=self.v['fixed_components'][name];data=(ROOT/c['contract']).read_bytes();self.assertEqual(hashlib.sha256(data).hexdigest(),c['contract_sha256']);self.assertEqual(json.loads(data)['component_profile'],profile)
    def test_catalog_semantic_provenance_exact(self):
        c=json.loads((ROOT/'contracts/forever-camera-contract-v3.json').read_text());self.assertEqual(self.v['fixed_components']['cameras']['schema_catalog'],c['schema_catalog']);self.assertEqual(self.v['plan']['schema_catalog']['id'],c['schema_catalog']['id']);self.assertEqual(c['schema_catalog']['physical_field'],3)
    def test_existing_budgets_and_nonempty_policy(self):
        l=self.v['limits'];self.assertEqual(l['db2_file_bytes'],16*1024**2);self.assertEqual(l['db2_total_bytes'],16*1024**3);self.assertEqual(l['gt']['files'],20);self.assertEqual(l['gt']['file_bytes'],4*1024**2);self.assertEqual(l['cameras']['total_bytes_including_source_once'],256*1024**2);self.assertEqual(l['cameras']['unique_ids'],4096);self.assertTrue(self.v['nonempty_artifacts']['required']);self.assertFalse(self.v['nonempty_artifacts']['standalone_opaque_empty_admission_transferred'])
    def test_package_formats_keys_and_false_completion_unchanged(self):
        old=json.loads((ROOT/'contracts/forever-product-contract-v5.json').read_text());self.assertEqual(self.v['package'],old['package']);self.assertEqual(self.v['provided_keys'],old['provided_keys']);self.assertFalse(self.v['full_extractor_complete']);self.assertEqual(self.v['backend'],'rust-local');self.assertEqual(self.v['platform'],'linux')
    def test_authored_canonical_fixture_agreement(self):
        data=(ROOT/'tests/contracts/fixtures/forever-product-v9/disabled.tsv').read_bytes();self.assertTrue(data.endswith(b'\n'));lines=data.decode().splitlines();self.assertEqual(lines[0],'forever-product\t9');fields=dict(line.split('\t') for line in lines[:-1]);self.assertEqual(fields['components'],'db2,gt,cameras');self.assertEqual(fields['decryption'],'disabled');self.assertEqual(lines[-1],'db2\tCinematicCamera\t1294214');self.assertEqual(fields['terrain'],fields['navigation']);self.assertEqual(fields['terrain'],'none')
    def test_linux_hosted_contract_step_selects_v9_suite(self):
        flow=json.loads((ROOT/'.github/workflows/forever-rust.yml').read_text());step=next(s for s in flow['jobs']['linux']['steps']if s.get('name')=='Check source contracts and synthetic Python oracle');self.assertIn('python -m unittest discover -s tests/contracts -p test_forever_product_v9_contract.py -v',step['run'].splitlines())
    def test_source_recipe_includes_new_production_contract_tests_and_fixture(self):
        spec=importlib.util.spec_from_file_location('product9_packager',ROOT/'tools/package_forever.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);selected=set(m.selected_paths(ROOT))
        for path in ['contracts/forever-product-contract-v9.json','crates/forever-extractor/src/plan/publication.rs','crates/forever-extractor/src/plan/v9_tests.rs','crates/forever-extractor/src/verify/v9.rs','crates/forever-extractor/src/verify/tests/v9.rs','crates/forever-extractor/tests/product_v9_cli.rs','tests/contracts/fixtures/forever-product-v9/disabled.tsv','tests/forever/product_v9_fixture.py','tests/forever/test_product_v9_cli.py']:
            self.assertIn(path,selected)
if __name__=='__main__':unittest.main()
