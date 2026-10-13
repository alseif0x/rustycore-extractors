# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit catalog v3 contracts. All fixtures are synthetic structured facts."""
import copy
import hashlib
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[2]


def load(relative):
    return json.loads((ROOT / relative).read_text())


class ForeverCatalogContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.schema = load('schemas/forever-manifest-v3.schema.json')
        cls.v2 = load('schemas/forever-manifest-v2.schema.json')
        cls.contract = load('contracts/forever-contract-v3.json')
        cls.complete = load('tests/contracts/fixtures/forever-manifest-v3.complete.json')
        cls.failed = load('tests/contracts/fixtures/forever-manifest-v3.failed.json')
        Draft202012Validator.check_schema(cls.schema)
        cls.validator = Draft202012Validator(cls.schema, format_checker=FormatChecker())

    def test_synthetic_complete_and_failed_fixtures(self):
        for fixture in [self.complete, self.failed]:
            with self.subTest(status=fixture['status']):
                self.assertEqual(list(self.validator.iter_errors(fixture)), [])

    def test_v2_v3_contracts_are_distinct(self):
        v2_fixture = load('tests/contracts/fixtures/forever-manifest.complete.json')
        self.assertTrue(list(self.validator.iter_errors(v2_fixture)))
        self.assertTrue(list(Draft202012Validator(self.v2).iter_errors(self.complete)))
        self.assertEqual(self.v2['properties']['manifest_version'], {'const': 2})
        self.assertEqual(self.schema['properties']['manifest_version'], {'const': 3})
        self.assertEqual(self.contract['scope']['default_tables'], [])
        self.assertEqual(load('contracts/forever-contract-v2.json')['scope']['default_tables'], ['ChrClasses', 'ChrRaces'])

    def test_provenance_matches_exact_committed_catalog(self):
        raw = (ROOT / 'contracts/forever-metadata-v1.json').read_bytes()
        catalog = json.loads(raw)
        provenance = self.complete['schema_catalog']
        self.assertEqual(provenance, self.contract['schema_catalog'])
        self.assertEqual(provenance['facts_sha256'], hashlib.sha256(raw).hexdigest())
        self.assertEqual(provenance['source_commit'], catalog['source']['commit'])
        self.assertEqual(provenance['source_blob'], catalog['source']['git_blob'])
        self.assertEqual(provenance['source_sha256'], catalog['source']['sha256'])
        for key in provenance:
            changed = copy.deepcopy(self.complete)
            changed['schema_catalog'][key] = 'invalid'
            self.assertTrue(list(self.validator.iter_errors(changed)), key)

    def test_all_1126_catalog_name_id_pairs_are_in_the_schema(self):
        catalog = load('contracts/forever-metadata-v1.json')
        expected = {(table['name'], table['file_data_id']) for table in catalog['tables']}
        actual = {(item['properties']['name']['const'], item['properties']['file_id']['const']) for item in self.schema['$defs']['catalog_identity']['oneOf']}
        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 1126)
        self.assertEqual(len(self.schema['$defs']['catalog_identity']['oneOf']), 1126)

    def test_complete_cannot_downgrade_schema_or_record_claims(self):
        for key, value in [('table_schema_validated', False), ('record_data_decoded', False), ('decoded_rows', None), ('name', 'liquidtype'), ('file_id', 1371381), ('bytes', 16777217)]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.complete)
                changed['outputs']['artifacts'][0][key] = value
                self.assertTrue(list(self.validator.iter_errors(changed)))

    def test_catalog_selector_explicit_selection_and_budgets_are_required(self):
        for key, value in [('schema_catalog', 'unknown'), ('table_selection', 'default_character_tables'), ('manifest_sha256', None), ('entry_count', 257), ('max_file_bytes', 16777217), ('max_batch_bytes', 268435457)]:
            with self.subTest(key=key):
                changed = copy.deepcopy(self.complete)
                changed['options'][key] = value
                self.assertTrue(list(self.validator.iter_errors(changed)))
        for section, field in [('schema_catalog', None), ('options', 'schema_catalog')]:
            changed = copy.deepcopy(self.complete)
            if field is None:
                del changed[section]
            else:
                del changed[section][field]
            self.assertTrue(list(self.validator.iter_errors(changed)))


if __name__ == '__main__':
    unittest.main()
