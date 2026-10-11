# SPDX-License-Identifier: GPL-3.0-or-later
"""DB2 v4 acquisition contracts, synthetic facts and repository-only references."""
import copy
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker, validate

ROOT = Path(__file__).resolve().parents[2]


def load(path):
    return json.loads((ROOT / path).read_text())


class ForeverAcquiredDb2ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = load('schemas/forever-manifest-v4.schema.json')
        cls.contract = load('contracts/forever-contract-v4.json')
        cls.reviewed = load('tests/contracts/fixtures/forever-manifest-v4.reviewed.complete.json')
        cls.catalog = load('tests/contracts/fixtures/forever-manifest-v4.catalog.complete.json')
        cls.failed = load('tests/contracts/fixtures/forever-manifest-v4.catalog.failed.json')
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema, format_checker=FormatChecker())

    def invalid(self, document):
        self.assertTrue(list(self.validator.iter_errors(document)))

    def test_reviewed_catalog_and_failed_synthetic_fixtures(self):
        for fixture in (self.reviewed, self.catalog, self.failed):
            self.assertEqual(list(self.validator.iter_errors(fixture)), [])
            # The public entry point must work without a registry or retrieval.
            validate(fixture, self.schema, format_checker=FormatChecker())
        self.assertEqual(self.contract['manifest']['report_version'], 2)
        self.assertEqual(self.contract['scope']['default_tables'], ['ChrClasses', 'ChrRaces'])

    def test_versions_and_acquisition_remain_separate(self):
        for fixture in (self.reviewed, self.catalog):
            for version in (2, 3, 5):
                altered = copy.deepcopy(fixture)
                altered['manifest_version'] = version
                self.invalid(altered)
        for version in (2, 3):
            schema = load(f'schemas/forever-manifest-v{version}.schema.json')
            self.assertTrue(list(Draft202012Validator(schema).iter_errors(self.reviewed)))
        altered = copy.deepcopy(self.reviewed)
        del altered['acquisition']
        self.invalid(altered)

    def test_acquisition_is_strict_and_offline(self):
        for key, value in (('backend', 'native'), ('profile', 'future'), ('offline', False),
                           ('complete_ckey_md5', False), ('ekey_validation', 'full'),
                           ('root_layout', 'auto'), ('index_layouts', ['paged'] * 15)):
            altered = copy.deepcopy(self.reviewed)
            altered['acquisition'][key] = value
            self.invalid(altered)
        altered = copy.deepcopy(self.reviewed)
        altered['acquisition']['private_path'] = '/private/client'
        self.invalid(altered)
        altered = copy.deepcopy(self.reviewed)
        altered['acquisition']['index_layouts'][0] = 'paged'
        self.invalid(altered)

    def test_all_limits_are_required_and_file_ceiling_can_only_shrink(self):
        for key in self.reviewed['acquisition']['limits']:
            altered = copy.deepcopy(self.reviewed)
            del altered['acquisition']['limits'][key]
            self.invalid(altered)
        for number in (0, 16777217):
            altered = copy.deepcopy(self.reviewed)
            altered['acquisition']['limits']['file_bytes'] = number
            altered['options']['max_file_bytes'] = number
            self.invalid(altered)
        altered = copy.deepcopy(self.reviewed)
        altered['acquisition']['limits']['file_bytes'] = 131072
        altered['options']['max_file_bytes'] = 131072
        self.assertEqual(list(self.validator.iter_errors(altered)), [])

    def test_selection_controls_catalog_provenance_and_claims(self):
        for fixture in (self.reviewed, self.catalog):
            altered = copy.deepcopy(fixture)
            altered['schema_selection'] = 'catalog' if fixture is self.reviewed else 'reviewed'
            self.invalid(altered)
        for key, value in (('table_schema_validated', False), ('record_data_decoded', False),
                           ('decoded_rows', None), ('name', 'liquidtype')):
            altered = copy.deepcopy(self.catalog)
            altered['outputs']['artifacts'][0][key] = value
            self.invalid(altered)

    def test_reviewed_keeps_explicit_metadata_only_semantics(self):
        altered = copy.deepcopy(self.reviewed)
        altered['options']['table_selection'] = 'explicit_manifest'
        altered['options']['manifest_sha256'] = '0' * 64
        for artifact in altered['outputs']['artifacts']:
            artifact['table_schema_validated'] = False
            artifact['record_data_decoded'] = False
            artifact['decoded_rows'] = None
        self.assertEqual(list(self.validator.iter_errors(altered)), [])
        self.assertIn('metadata-only', self.contract['scope']['schema_selection']['reviewed'])

    def test_failed_cannot_claim_complete_output(self):
        altered = copy.deepcopy(self.failed)
        altered['outputs']['incomplete_marker_present'] = False
        self.invalid(altered)
        altered = copy.deepcopy(self.failed)
        altered['outputs']['artifacts'][0]['record_data_decoded'] = True
        self.invalid(altered)

    def test_catalog_reference_uses_the_existing_pinned_identity_set(self):
        v3 = load('schemas/forever-manifest-v3.schema.json')
        self.assertEqual(self.schema['$defs']['catalog_identity'],
                         v3['$defs']['catalog_identity'])
        self.assertEqual(self.catalog['schema_catalog'], self.contract['schema_catalog'])
        acquisition = load('schemas/forever-acquisition-v1.schema.json')
        del acquisition['$id']
        del acquisition['$schema']
        self.assertEqual(self.schema['$defs']['acquisition'], acquisition)
        self.assertEqual(self.schema['properties']['acquisition'],
                         {'$ref': '#/$defs/acquisition'})

    def test_every_reference_is_a_local_fragment_without_nested_resource_ids(self):
        pending = [self.schema]
        while pending:
            node = pending.pop()
            if isinstance(node, dict):
                if '$ref' in node:
                    self.assertTrue(node['$ref'].startswith('#/'), node['$ref'])
                if node is not self.schema:
                    self.assertNotIn('$id', node)
                pending.extend(node.values())
            elif isinstance(node, list):
                pending.extend(node)


if __name__ == '__main__':
    unittest.main()
