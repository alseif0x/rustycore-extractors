# SPDX-License-Identifier: GPL-3.0-or-later
"""Authored GT manifest vectors, shared with Rust parser/serializer tests."""
import copy
import hashlib
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / 'tests/contracts/fixtures/forever-gt'


class GT70338Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.validators = {}
        for version in range(1, 5):
            schema = json.loads((ROOT / f'schemas/forever-gt-manifest-v{version}.schema.json').read_text())
            Draft202012Validator.check_schema(schema)
            cls.validators[version] = Draft202012Validator(schema)
        cls.vectors = [json.loads(path.read_text()) for path in sorted(FIXTURES.glob('*.json'))]
        cls.v4 = next(v for v in cls.vectors if v['manifest_version'] == 4)

    def test_shared_vectors_validate_exact_version_and_build(self):
        for vector in self.vectors:
            version = vector['manifest_version']
            self.validators[version].validate(vector)
            for other, validator in self.validators.items():
                if other != version:
                    self.assertFalse(validator.is_valid(vector))
            for build in [70170, 70205, 70338, 70339]:
                changed = dict(vector, build=build)
                self.assertEqual(self.validators[version].is_valid(changed),
                                 build == 70338 if version == 4 else build in [70170, 70205])

    def test_v4_requires_component_profile_and_rust_local_acquisition(self):
        for key in ['component_profile', 'acquisition']:
            v = copy.deepcopy(self.v4)
            del v[key]
            self.assertFalse(self.validators[4].is_valid(v))
        for key, value in [('component_profile', 'auto'), ('plan', 'all'),
                           ('scope', 'explicit_gt_batch'), ('unknown', 0)]:
            self.assertFalse(self.validators[4].is_valid(dict(self.v4, **{key: value})))
        v = copy.deepcopy(self.v4)
        v['acquisition']['backend'] = 'native'
        self.assertFalse(self.validators[4].is_valid(v))

    def test_v4_exact_catalog_order_and_numeric_raw_modes(self):
        self.assertEqual(len(self.v4['artifacts']), 20)
        self.assertEqual(sum(a['validation_mode'] == 'f32' for a in self.v4['artifacts']), 14)
        for artifact in self.v4['artifacts']:
            if artifact['validation_mode'] == 'raw':
                for key in ['value_columns', 'data_rows', 'consumer_rows_including_zero', 'terminated_early']:
                    self.assertIsNone(artifact[key])
        for mutate in [lambda v: v['artifacts'].reverse(), lambda v: v['artifacts'].pop(),
                       lambda v: v['artifacts'][0].update(file_id=0),
                       lambda v: v['artifacts'][4].update(data_rows=1),
                       lambda v: v['artifacts'][0].update(value_columns=2)]:
            v = copy.deepcopy(self.v4)
            mutate(v)
            self.assertFalse(self.validators[4].is_valid(v))

    def test_no_cap_increase_or_generic_product_admission(self):
        c = json.loads((ROOT / 'contracts/forever-gt-contract-v4.json').read_text())
        self.assertEqual(c['admitted_builds'], [70338])
        self.assertEqual(c['backend'], 'rust-local')
        self.assertEqual(c['legacy_builds'], [70170, 70205])
        self.assertEqual(c['limits']['file_bytes'], 4194304)
        self.assertEqual(c['limits']['batch_bytes'], 83886080)
        self.assertEqual(c['unified_product_70338_scope_unchanged'], 'DB2 only')
        acquisition = (ROOT / c['acquisition_schema']).read_bytes()
        self.assertEqual(hashlib.sha256(acquisition).hexdigest(), c['acquisition_schema_sha256'])
        catalog = (ROOT / c['catalog']).read_bytes()
        self.assertEqual(hashlib.sha256(catalog).hexdigest(), c['catalog_sha256'])
        for amount in [0, 4194305]:
            v = copy.deepcopy(self.v4)
            v['acquisition']['limits']['file_bytes'] = amount
            self.assertFalse(self.validators[4].is_valid(v))


if __name__ == '__main__':
    unittest.main()
