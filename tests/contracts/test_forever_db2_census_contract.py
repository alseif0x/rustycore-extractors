# SPDX-License-Identifier: GPL-3.0-or-later
"""Static consistency for the synthetic-only keyless DB2 census contract."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ForeverDb2CensusContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            (ROOT / "contracts/forever-db2-census-contract-v1.json").read_text()
        )
        cls.census = (ROOT / "crates/forever-extractor/src/driver/census_db2.rs").read_text()
        cls.probe = (ROOT / "crates/forever-casc/src/local_store/probe.rs").read_text()
        cls.owned = (ROOT / "crates/forever-casc/src/local_store/owned.rs").read_text()
        cls.catalog = (
            ROOT / "crates/forever-db2/src/records/wowdbdefs_70450_generated.rs"
        ).read_text()

    def test_contract_is_keyless_partial_and_has_no_success_claim(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"]),
            ("forever-db2-keyless-availability-census-v1", 1),
        )
        self.assertFalse(contract["network"])
        self.assertFalse(contract["keys_consulted"])
        self.assertTrue(contract["cli_enabled"])
        self.assertFalse(contract["windows_enabled"])
        self.assertFalse(contract["complete_extraction_claim"])
        self.assertTrue(contract["report"]["partial_result_expected_when_keys_unavailable"])
        self.assertFalse(contract["report"]["complete_extraction_claim"])
        self.assertEqual(contract["catalogue"]["id"], "wowdbdefs-70450-v1")
        self.assertEqual(contract["catalogue"]["declaration_count"], 1161)

    def test_catalogue_enumeration_and_shared_keyless_resolution_are_explicit(self):
        self.assertEqual(self.catalog.count("TableMetadata { name:"), 1161)
        self.assertIn("catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID)", self.census)
        self.assertIn("declarations.iter().enumerate()", self.census)
        self.assertIn("store.probe_file_data_ids(&[declaration.file_data_id]", self.census)
        self.assertIn("resolve_with_ordinal", self.owned)
        self.assertIn("resolve_with_ordinal(id, self.locale", self.probe)
        self.assertIn("open_keyless_store_70450", self.census)

    def test_report_contract_is_redacted_and_preserves_each_state(self):
        states = self.contract["availability_states"]
        self.assertEqual(
            set(states),
            {
                "fully_readable",
                "encrypted_no_available_key",
                "file_unavailable",
                "unsupported_or_unknown",
            },
        )
        self.assertEqual(
            self.contract["state_precedence"],
            [
                "fully_readable",
                "unsupported_or_unknown",
                "encrypted_no_available_key",
                "file_unavailable",
            ],
        )
        predicates = self.contract["state_predicates"]
        self.assertIn("Central.encoded_keys", predicates["fully_readable"])
        self.assertIn("BuildReference", predicates["fully_readable"])
        self.assertIn("precedes encryption", predicates["unsupported_or_unknown"])
        self.assertIn("at least one candidate rejection class is encrypted", predicates["encrypted_no_available_key"])
        self.assertIn("incomplete span", predicates["file_unavailable"])
        self.assertTrue(self.contract["report"]["deterministic"])
        self.assertTrue(self.contract["report"]["all_declarations_retained"])
        fields = set(self.contract["report"]["fields"])
        self.assertTrue(
            {
                "declaration_ordinal",
                "table_name",
                "file_data_id",
                "state",
                "state_reason",
                "trace",
                "root_rows_considered",
                "selected_root_ordinal",
                "encoding_row_count",
                "ckey_central_present",
                "index_prefix_match",
                "rejection_class",
                "blte_code",
            }
            <= fields
        )
        self.assertFalse(fields & {"ckey", "ekey", "key_bytes", "bytes", "payload", "sha256"})
        self.assertIn("key values", self.contract["report"]["forbidden_values"][-1])
        for source in self.contract["sources"]:
            self.assertTrue((ROOT / source).is_file(), source)
        self.assertIn("FileTrace", self.census)
        self.assertIn("candidate_bytes_considered", self.census)
        self.assertIn("MAX_REPORT_BYTES", self.census)


if __name__ == "__main__":
    unittest.main()
