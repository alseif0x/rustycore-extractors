# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit Linux70374 triple-component public facts; no client/keys/target calls."""
from pathlib import Path
import hashlib
import importlib.util
import json
import unittest

ROOT = Path(__file__).resolve().parents[2]


def load(name):
    return json.loads((ROOT / 'contracts' / name).read_bytes())


class ProductV11Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.v = load('forever-product-contract-v11.json')

    def test_explicit_profile_exact_triple_and_platform(self):
        v = self.v
        self.assertEqual((v['version'], v['contract'], v['publication_profile']),
                         (11, 'forever-product-v11', 'Build70374Db2GtCamerasV1'))
        self.assertEqual(v['builds'], [70374])
        self.assertEqual(v['plan']['components'], ['db2', 'gt', 'cameras'])
        self.assertEqual(v['plan']['component_mask'], 7)
        self.assertEqual((v['platform'], v['backend']), ('linux', 'rust-local'))
        self.assertEqual(v['commands'], ['inspect', 'extract', 'resume', 'verify', 'verify-links'])
        self.assertEqual(v['plan']['dependencies'], {'cameras': ['db2', 'CinematicCamera']})
        self.assertEqual(v['plan']['selections'], ['none'])
        self.assertEqual(v['plan']['offmesh_connections'],
                         'Field must be absent; even an empty offmesh selection is unsupported.')
        self.assertFalse(v['full_extractor_complete'])

    def test_exact_catalog_and_camera_semantic_provenance(self):
        raw = (ROOT / 'contracts/wowdbdefs-70374-v1.json').read_bytes()
        facts = json.loads(raw)
        target = self.v['plan']['schema_catalog']
        self.assertEqual(target, load('forever-product-contract-v10.json')['plan']['schema_catalog'])
        self.assertEqual(target['facts_sha256'], hashlib.sha256(raw).hexdigest())
        c = self.v['fixed_components']['cameras']['schema_catalog']
        self.assertEqual((c['id'], c['source_commit'], c['source_manifest_blob'],
                          c['source_manifest_sha256'], c['facts_sha256']),
                         (target['id'], facts['source']['commit'], facts['source']['manifest_blob'],
                          facts['source']['manifest_sha256'], target['facts_sha256']))
        table = next(t for t in facts['tables'] if t['name'] == 'CinematicCamera')
        for name in ('file_data_id', 'table_hash', 'layout_hash', 'file_field_count'):
            self.assertEqual(c[name], table[name])
        self.assertEqual((c['physical_field'], c['field_name'], c['field_type'],
                          c['decoded_scalar_count'], c['external_id']), (3, 'FileDataID', 'u32', 7, True))
        self.assertEqual(table['decode_kinds'][3], 'Unsigned(32, 1)')
        self.assertEqual(c['definition_path'], 'definitions/CinematicCamera.dbd')
        self.assertEqual(c['definition_blob'], 'fca2c69a9e7b6017481e624f8ebb51260864507a')
        self.assertEqual(c['definition_sha256'],
                         '6974ca7b12a7eb35b1d961787182e34e49547dbc5e639d9e7b4d6eba8a678252')
        self.assertNotEqual(c['definition_blob'],
                            load('forever-camera-contract-v3.json')['schema_catalog']['definition_blob'])

    def test_fixed_gt_algorithms_do_not_widen_standalone_profiles(self):
        g = self.v['fixed_components']['gt']
        raw = (ROOT / g['catalog']).read_bytes()
        tables = json.loads(raw)['tables']
        self.assertEqual(hashlib.sha256(raw).hexdigest(), g['catalog_sha256'])
        self.assertEqual((len(tables), sum(t['validation_mode'] == 'f32' for t in tables)), (20, 14))
        self.assertEqual((g['files'], g['numeric_files'], g['raw_only_files']), (20, 14, 6))
        self.assertEqual(g['numeric_policy'], 'finite_decimal_f64_to_f32_v1')
        algorithm = (ROOT / g['algorithm_contract']).read_bytes()
        self.assertEqual(hashlib.sha256(algorithm).hexdigest(), g['algorithm_contract_sha256'])
        for component, contract, profile in (
                ('gt', 'forever-gt-contract-v4.json', 'gt-catalog-70338-v1'),
                ('cameras', 'forever-camera-contract-v3.json', 'camera-raw-70338-v1')):
            self.assertEqual(load(contract)['admitted_builds'], [70338])
            self.assertEqual(self.v['fixed_components'][component]['standalone_profile_unchanged'], profile)
            self.assertFalse(self.v['fixed_components'][component]['standalone_admission_extended'])
        self.assertEqual(self.v['fixed_components']['cameras']['discovery_api'], 'cameras::discover_70374')

    def test_legacy_wire_keys_and_component_limits_preserved(self):
        v9 = load('forever-product-contract-v9.json')
        v10 = load('forever-product-contract-v10.json')
        self.assertEqual(self.v['package'], v10['package'])
        self.assertEqual(self.v['provided_keys'], v10['provided_keys'])
        self.assertEqual(self.v['legacy_preservation']['canonical_versions'], [1, 2, 3, 4, 5, 6, 8, 9, 10])
        self.assertTrue(self.v['legacy_preservation']['catalog_does_not_select_v11'])
        self.assertEqual((v9['builds'], v9['plan']['component_mask']), ([70338], 7))
        self.assertEqual((v10['builds'], v10['plan']['component_mask']), ([70374], 1))
        for name, value in v10['limits'].items():
            self.assertEqual(self.v['limits'][name], value)
        for name in ('gt', 'cameras'):
            self.assertEqual(self.v['limits'][name], v9['limits'][name])
        self.assertEqual(self.v['nonempty_artifacts'], v9['nonempty_artifacts'])
        self.assertIn('independent of path order', self.v['verification']['metadata_preflight'])
        self.assertIn('--keys is forbidden for verify and verify-links',
                      self.v['verification']['metadata_preflight'])
        self.assertIn('verify_sealed_artifact', self.v['published_camera_source']['comparison'])

    def test_both_authored_fixtures_are_exact_canonical_plans(self):
        folder = ROOT / 'tests/contracts/fixtures/forever-product-v11'
        disabled = (folder / 'disabled.tsv').read_bytes()
        provided = (folder / 'provided.tsv').read_bytes()
        self.assertEqual(disabled.replace(b'decryption\tdisabled\n',
                                          b'decryption\tprovided-salsa20-v1\n'), provided)
        for raw, policy in ((disabled, 'disabled'), (provided, 'provided-salsa20-v1')):
            self.assertTrue(raw.endswith(b'\n'))
            self.assertNotIn(b'\r', raw)
            lines = raw.decode('ascii').splitlines()
            fields = dict(line.split('\t') for line in lines[:-1])
            self.assertEqual([line.split('\t')[0] for line in lines[:-1]], self.v['plan']['ordered_keys'])
            self.assertEqual(fields['forever-product'], '11')
            self.assertEqual(fields['build'], '70374')
            self.assertEqual(fields['schema-catalog'], 'wowdbdefs-70374-v1')
            self.assertEqual(fields['components'], 'db2,gt,cameras')
            self.assertEqual(fields['decryption'], policy)
            self.assertEqual((fields['terrain'], fields['navigation']), ('none', 'none'))
            self.assertEqual(lines[-1], 'db2\tCinematicCamera\t1294214')
            self.assertNotIn('collision-storage', fields)
            self.assertNotIn('offmesh', fields)

    def test_narrow_bootstrap_and_shared_product_wiring(self):
        admission = (ROOT / 'crates/forever-casc/src/admission.rs').read_text()
        legacy = admission.split('pub const SUPPORTED_CLIENTS:', 1)[1].split('];', 1)[0]
        self.assertNotIn('70374', legacy)
        self.assertIn('admit_db2_70374', admission)
        driver = (ROOT / 'crates/forever-extractor/src/driver.rs').read_text()
        self.assertIn('open_db2_70374', driver)
        publication = (ROOT / 'crates/forever-extractor/src/plan/publication.rs').read_text()
        self.assertIn('Build70374Db2GtCamerasV1', publication)
        plan = (ROOT / 'crates/forever-extractor/src/plan.rs').read_text()
        self.assertIn('fn is_v11(', plan)
        self.assertIn('fn is_gt_cameras(', plan)
        self.assertIn('discover_70374', (ROOT / 'crates/forever-casc/src/cameras.rs').read_text())

    def test_only_linux_hosted_step_selects_v11(self):
        flow = json.loads((ROOT / '.github/workflows/forever-rust.yml').read_bytes())
        command = 'python -m unittest discover -s tests/contracts -p test_forever_product_v11_contract.py -v'
        for name, job in flow['jobs'].items():
            runs = '\n'.join(step.get('run', '') for step in job['steps'])
            self.assertEqual(runs.count(command), 1 if name == 'linux' else 0)

    def test_current_source_selection_includes_v11_and_retains_v10(self):
        spec = importlib.util.spec_from_file_location('product11_packager', ROOT / 'tools/package_forever.py')
        m = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(m)
        self.assertEqual(m.SCHEMA, 'forever-distribution-v1')
        self.assertEqual(m.SOURCE_PROFILE, 'forever-seven-crate-linux-product-db2-gt-cameras-source-v11')
        selected = set(m.selected_paths(ROOT))
        for name in ('contracts/forever-product-contract-v11.json',
                     'tests/contracts/test_forever_product_v11_contract.py',
                     'tests/contracts/fixtures/forever-product-v11/disabled.tsv',
                     'tests/contracts/fixtures/forever-product-v11/provided.tsv',
                     'crates/forever-extractor/src/plan/v11_tests.rs',
                     'crates/forever-extractor/src/driver/v11_tests.rs',
                     'crates/forever-extractor/src/verify/tests/v11.rs',
                     'crates/forever-extractor/tests/product_v11_cli.rs',
                     'crates/forever-casc/src/cameras/tests/build70374.rs',
                     'contracts/forever-product-contract-v10.json',
                     'tests/contracts/fixtures/forever-product-v10/disabled.tsv'):
            self.assertIn(name, selected)
            self.assertTrue(m.bundle_path_allowed('source/' + name))
        self.assertTrue(set(m.PRODUCT_70374_GT_CAMERAS_SOURCE).issubset(m.REQUIRED))
        self.assertTrue(set(m.PRODUCT_70374_GT_CAMERAS_SOURCE).issubset(load('forever-distribution-contract-v1.json')['source']['required']))


if __name__ == '__main__':
    unittest.main()
