# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 selected available-navigation contract and source boundary."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-selected-navigation-contract-v1.json"


class ForeverSelectedNavigationContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_identity_and_scoped_api_profiles(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"]),
            ("forever-selected-navigation-contract-v1", 1),
        )
        self.assertEqual(
            (c["scope"]["product_line"], c["scope"]["client_build"], c["scope"]["platform"]),
            ("forever", 70450, "linux"),
        )
        self.assertTrue(c["scope"]["api_only"])
        self.assertFalse(c["scope"]["cli_profile"])
        self.assertEqual(c["reader"]["profile"], "wowdbdefs-70450-available-navigation-v1")
        self.assertEqual(c["reader"]["catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(c["selected_plan"]["profile"], "navigation-selected-available-70450-linux-v1")

    def test_synthetic_contract_examples_preserve_direct_and_context_semantics(self):
        examples = self.contract["synthetic_contract_examples"]
        duplicate = examples["duplicate_direct_id"]
        rows = duplicate["rows_in_source_order"]
        winners = {}
        for row in rows:
            winners[row["id"]] = row
        self.assertEqual(winners[duplicate["selected_id"]]["ordinal"],
                         duplicate["expected_direct_winner_ordinal"])
        self.assertEqual(winners[duplicate["selected_id"]]["effective_parent"], 11)
        self.assertEqual(duplicate["expected_output_map_ids"], [7])
        self.assertEqual(duplicate["expected_ancestor_context_ids"], [11])
        self.assertEqual(
            examples["copy_only_selected_id"]["expected_result"],
            self.contract["error_codes"]["copy_only_selection"],
        )
        self.assertEqual(
            examples["missing_active_parent"]["expected_result"],
            self.contract["error_codes"]["missing_active_parent"],
        )

    def test_public_symbols_and_error_boundaries_are_present(self):
        availability = (ROOT / "crates/forever-db2/src/availability.rs").read_text()
        inventory = (ROOT / "crates/forever-navigation/src/inventory.rs").read_text()
        pipeline = (ROOT / "crates/forever-navigation/src/pipeline.rs").read_text()
        self.assertIn("pub fn read_navigation_maps_70450", availability)
        self.assertIn("AvailableRecords<source::NavigationMapFields>", availability)
        self.assertIn("SELECTED_AVAILABLE_70450_LINUX_PROFILE", inventory)
        self.assertIn("pub struct AvailableSelectedPlan", inventory)
        self.assertIn("pub fn from_available_70450", inventory)
        self.assertIn("pub fn run_available_selected", pipeline)
        self.assertIn("inventory::AvailableSelectedPlan", pipeline)
        error_codes = inventory.split("pub const fn code", 1)[1].split("impl std::fmt::Display", 1)[0]
        for code in self.contract["error_codes"].values():
            self.assertIn(code, error_codes)

    def test_scope_nonclaims_and_validation_requirements_are_explicit(self):
        c = self.contract
        self.assertTrue(c["scope"]["api_only"])
        self.assertFalse(c["scope"]["cli_profile"])
        self.assertIn("no CLI profile or Map-partial v21 admission", c["non_claims"])
        self.assertIn("no complete-world coverage claim", c["non_claims"])
        self.assertIn("no native parity claim", c["non_claims"])
        self.assertIn("no RustyCore runtime acceptance claim", c["non_claims"])
        self.assertIn("no Windows or release authorization", c["non_claims"])
        requirements = c["validation_requirements"]
        self.assertGreaterEqual(len(requirements), 4)
        self.assertTrue(any("cumulative work/heap limits" in requirement for requirement in requirements))
        self.assertTrue(any("source completeness/authentication" in requirement for requirement in requirements))


if __name__ == "__main__":
    unittest.main()
