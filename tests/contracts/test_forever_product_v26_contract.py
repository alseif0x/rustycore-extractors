# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 DB2-only partial-section plan and contract; synthetic inputs only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v26.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v26"


class ProductV26Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def test_identity_admission_and_canonical_plan_scope(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v26", 26, "Build70450Db2PartialV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual((contract["platform"], contract["backend"], contract["offline"]), ("linux", "rust-local", True))
        self.assertEqual(
            contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"]
        )
        plan = contract["plan"]
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2"], 1))
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertEqual(plan["partial_encrypted_sections"], "declared-skip, required")
        self.assertIn("1161", plan["db2_entries"])
        self.assertEqual(plan["storage"], "resident")
        self.assertEqual(plan["terrain"], ["none"])
        self.assertEqual(plan["navigation"], ["none"])
        self.assertEqual(plan["map_domain"], "absent")
        self.assertEqual(plan["offmesh"], "absent")
        self.assertEqual(contract["admission"]["slopes"], [55, 55])
        self.assertTrue(contract["admission"]["pack"])
        self.assertFalse(contract["admission"]["big_base_unit"])
        self.assertFalse(contract["admission"]["skip_liquids"])

    def test_plan_fixtures_are_canonical_one_or_multiple_entry_db2_plans(self):
        expected = {
            "single.tsv": [["db2", "AccountStoreCategory", "6220124"]],
            "multiple.tsv": [
                ["db2", "AccountStoreCategory", "6220124"],
                ["db2", "AnimationData", "1375431"],
            ],
        }
        for filename, entries in expected.items():
            raw = (FIXTURES / filename).read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            fields = {row[0]: row[1] for row in rows if row[0] != "db2"}
            self.assertEqual(fields["forever-product"], "26")
            self.assertEqual(fields["build"], "70450")
            self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
            self.assertEqual(fields["decryption"], "disabled")
            self.assertEqual(fields["partial_encrypted_sections"], "declared-skip")
            self.assertEqual(fields["components"], "db2")
            self.assertEqual(fields["terrain"], "none")
            self.assertEqual([row for row in rows if row[0] == "db2"], entries)

    def test_per_entry_declaration_shape_limits_and_zero_fill_error_are_explicit(self):
        contract = self.contract
        self.assertIn("For each selected DB2 table", contract["package"]["partial_encrypted_sections_by_entry"])
        self.assertIn("lookup, frame_ordinal, decoded_size", contract["package"]["partial_encrypted_sections_by_entry"])
        self.assertIn("does not add a package manifest field", contract["package"]["declaration_boundary"])
        self.assertEqual(contract["limits"]["max_db2_entries"], 1161)
        self.assertEqual(contract["limits"]["db2_file_bytes"], 16 * 1024 * 1024)
        self.assertEqual(contract["limits"]["db2_total_bytes"], 16 * 1024 * 1024 * 1024)
        self.assertIn("product_partial_zerofill", contract["source_policy"]["partial_sections"])
        self.assertEqual(contract["acceptance_matrix"][1]["case"], "synthetic-plan-fixtures")
        cases = {item["case"]: item["expect"] for item in contract["acceptance_matrix"]}
        self.assertIn("entry 1162", cases["catalog-limit"])
        self.assertIn("product_plan_partial_encrypted_sections", cases["partial-plan-gate"])
        self.assertIn("product_plan_component_scope", cases["scope-and-decryption"])

    def test_legacy_preservation_and_non_claim_boundary(self):
        contract = self.contract
        legacy = contract["legacy_preservation"]
        self.assertTrue(legacy["v16_plan_grammar_unchanged"])
        self.assertTrue(legacy["v25_plan_grammar_unchanged"])
        self.assertFalse(legacy["v16_and_v25_acceptance_transferred_to_v26"])
        for name in (
            "decryption",
            "key_discovery_or_use",
            "complete_db2_or_client_coverage",
            "runtime_acceptance",
            "format_parity",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["boundary"][name])
        self.assertFalse(contract["verification"]["full_extractor_complete"])
        self.assertFalse(contract["verification"]["runtime_claim"])
        self.assertFalse(contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
