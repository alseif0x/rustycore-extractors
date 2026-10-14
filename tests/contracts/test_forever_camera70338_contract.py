# SPDX-License-Identifier: GPL-3.0-or-later
"""Authored camera vectors shared with strict Rust readers and serializers."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tests/contracts/fixtures/forever-camera'


class Camera70338Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.validators = {}
        for version in range(1, 4):
            schema = json.loads((ROOT / f'schemas/forever-camera-manifest-v{version}.schema.json').read_text())
            Draft202012Validator.check_schema(schema)
            cls.validators[version] = Draft202012Validator(schema)
        cls.vectors = [json.loads(p.read_text()) for p in sorted(FIXTURES.glob('*.json'))]
        cls.v3 = next(v for v in cls.vectors if v['manifest_version'] == 3)
        cls.contract = json.loads((ROOT / 'contracts/forever-camera-contract-v3.json').read_text())

    def test_shared_vectors_versions_and_build_admission(self):
        for v in self.vectors:
            version = v['manifest_version']
            self.validators[version].validate(v)
            for build in [70170, 70205, 70338, 70339]:
                self.assertEqual(self.validators[version].is_valid(dict(v, build=build)),
                                 build == 70338 if version == 3 else build in [70170, 70205])
            for other in [1, 2, 3]:
                if version != other:
                    self.assertFalse(self.validators[other].is_valid(v))

    def test_target_catalog_table_and_semantic_field_agree(self):
        p = self.v3['schema_catalog']
        self.assertEqual(p, self.contract['schema_catalog'])
        facts = (ROOT / 'contracts/wowdbdefs-70338-v1.json').read_bytes()
        self.assertEqual(hashlib.sha256(facts).hexdigest(), p['facts_sha256'])
        d = json.loads(facts)
        self.assertEqual(d['source']['commit'], p['source_commit'])
        self.assertEqual(d['source']['manifest_blob'], p['source_manifest_blob'])
        t = next(t for t in d['tables'] if t['name'] == 'CinematicCamera')
        for key in ['table_hash', 'layout_hash']:
            self.assertEqual(t[key], p[key])
        self.assertEqual(t['file_data_id'], p['file_data_id'])
        self.assertEqual(p['physical_field'], 3)
        self.assertEqual(t['decode_kinds'][3], 'Unsigned(32, 1)')
        self.assertEqual(t['index'], -1)
        self.assertEqual(p['file_field_count'], 5)
        self.assertEqual(sum(f['array_size'] for f in t['fields']), p['decoded_scalar_count'])
        self.assertEqual(p['field_name'], 'FileDataID')
        self.assertEqual(p['definition_path'], 'definitions/CinematicCamera.dbd')
        self.assertEqual(p['definition_blob'], '3676f4e583d5c53c4eda065db9c322f9d45921f0')
        self.assertEqual(p['definition_sha256'], '21f07cf5a2cdba30bec06895cb68e7467b58ab20cbee474f256473d2f70ca9d6')

    def test_exact_provenance_without_overloading_legacy_metadata_sha(self):
        self.assertNotIn('db2_metadata_sha256', self.v3)
        for key in ['component_profile', 'schema_catalog', 'acquisition']:
            v = copy.deepcopy(self.v3)
            del v[key]
            self.assertFalse(self.validators[3].is_valid(v))
        for key in self.v3['schema_catalog']:
            v = copy.deepcopy(self.v3)
            del v['schema_catalog'][key]
            self.assertFalse(self.validators[3].is_valid(v))
        for key, value in [('physical_field', 4), ('decoded_scalar_count', 5),
                           ('external_id', False), ('id', 'wowdbdefs-70205-v1'),
                           ('definition_sha256', '0' * 64), ('unknown', 0)]:
            v = copy.deepcopy(self.v3)
            v['schema_catalog'][key] = value
            self.assertFalse(self.validators[3].is_valid(v))
        v = dict(self.v3, db2_metadata_sha256='897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def')
        self.assertFalse(self.validators[3].is_valid(v))

    def test_acquisition_bounds_and_explicit_false_scope_claims(self):
        self.assertEqual(self.contract['admitted_builds'], [70338])
        self.assertEqual(self.contract['backend'], 'rust-local')
        self.assertEqual(self.contract['limits']['total_bytes_including_source'], 268435456)
        self.assertEqual(self.contract['limits']['discovered_unique_ids'], 4096)
        for key in ['m2_parsed', 'waypoints_generated', 'runtime_accepted', 'full_client_extraction']:
            self.assertIs(self.v3[key], False)
            self.assertFalse(self.validators[3].is_valid(dict(self.v3, **{key: True})))
        for amount in [0, 16777217]:
            v = copy.deepcopy(self.v3)
            v['acquisition']['limits']['file_bytes'] = amount
            self.assertFalse(self.validators[3].is_valid(v))
        v = copy.deepcopy(self.v3)
        v['acquisition']['backend'] = 'native'
        self.assertFalse(self.validators[3].is_valid(v))

    def test_source_identity_and_exact_camera_path_framing(self):
        self.assertEqual(self.v3['source']['file_id'], 1294214)
        self.assertEqual(self.v3['source']['direct_records'], 3)
        self.assertEqual(self.v3['source']['unique_file_ids'], 2)
        for path in ['cameras/FILE0000000b.xxx', 'cameras/other.xxx', '../escape']:
            v = copy.deepcopy(self.v3)
            v['cameras'][0]['relative_path'] = path
            self.assertFalse(self.validators[3].is_valid(v))
        v = copy.deepcopy(self.v3)
        v['source']['file_id'] = 0
        self.assertFalse(self.validators[3].is_valid(v))

    def test_current_source_recipe_selects_all_new_camera_files(self):
        spec = importlib.util.spec_from_file_location('camera70338_packager', ROOT / 'tools/package_forever.py')
        packager = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(packager)
        selected = set(packager.selected_paths(ROOT))
        for name in ['crates/forever-casc/src/cameras/contract.rs',
                     'crates/forever-casc/src/cameras/tests/build70338.rs',
                     'crates/forever-casc/tests/camera_cli.rs',
                     'contracts/forever-camera-contract-v3.json',
                     'schemas/forever-camera-manifest-v3.schema.json']:
            self.assertIn(name, selected)
        for p in FIXTURES.iterdir():
            self.assertIn(p.relative_to(ROOT).as_posix(), selected)


if __name__ == '__main__':
    unittest.main()
