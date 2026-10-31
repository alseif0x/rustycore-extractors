# SPDX-License-Identifier: GPL-3.0-or-later
"""Synthetic scope and boundary gates for the v48 missing-ROOT product."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v48.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v48/all.tsv"
EXPECTED_DB2 = [
    {"name": "GameObjectDisplayInfo", "file_data_id": 1266277, "count": 1},
    {"name": "Map", "file_data_id": 1349477, "count": 1},
]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV48Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_and_canonical_plan(self):
        contract = self.contract
        self.assertEqual((contract["contract"], contract["version"]), ("forever-product-v48", 48))
        self.assertEqual(contract["status"], "synthetic-validation-passed; real-target-attempt-failed-unaccepted")
        self.assertEqual(contract["publication_profile"], "Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1")
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["component_mask"], 17)
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 31)
        self.assertTrue(fixture.startswith("forever-product\t48\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertEqual(fixture.splitlines()[29:], ["db2\tGameObjectDisplayInfo\t1266277", "db2\tMap\t1349477"])
        plan = contract["plan"]
        self.assertEqual(plan["required_db2_entries"], EXPECTED_DB2)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["gameobject_scope"], "available-fdid-direct-missing-root-v1")
        self.assertEqual(plan["model_scope"], "placement-and-gameobject-with-missing-root-v1")
        self.assertEqual(plan["line_count"], 31)

    def test_profiles_codec_and_missing_root_ceiling_are_separate_from_v47(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["source_profile"],
                contract["plan_profile"],
                contract["plan_coverage_profile"],
                contract["codec_profile"],
                contract["domain_profile"],
                contract["resource_profile"]["id"],
                contract["completion"],
                contract["verification_profile"],
            ),
            (
                "linux70450-gameobject-missing-root-material-filtered-wmo-v1",
                "linux70450-gameobject-missing-root-material-filtered-wmo-vmap-product-v1",
                "linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21",
                "linux70450-gameobject-missing-root-material-filtered-wmo-v1",
                "gameobject-missing-root-material-filtered-wmo-v1",
                "linux-collision-gameobject-missing-root-expanded-resources-v1",
                "linux70450-available-gameobject-missing-root-material-filtered-wmo-collision-v1",
                "linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1",
            ),
        )
        resource = contract["resource_profile"]
        self.assertEqual(resource["basis"], "GameObjectExpandedV1")
        self.assertFalse(resource["resource_limits_changed_vs_v47"])
        self.assertEqual(resource["gameobject_missing_root_max"], 4096)
        self.assertEqual(
            resource["gameobject_missing_root_max_basis"],
            "maximum direct nonzero GODI candidate requests admitted by the profile",
        )
        missing_root = contract["gameobject_missing_root"]
        self.assertEqual(missing_root["maximum_direct_nonzero_godi_candidate_rows"], 4096)
        self.assertEqual(missing_root["maximum_unique_fdids"], 4096)
        self.assertFalse(missing_root["row_count_is_unique_fdid_count"])
        self.assertEqual(missing_root["candidate_row_count_is_capped_at"], 4096)
        self.assertEqual(missing_root["unique_fdid_count_is_capped_at"], 4096)
        self.assertEqual(
            missing_root["repeated_missing_root_fdids_across_rows"],
            "allowed; counted once in the canonical FDID list",
        )
        codec = contract["coverage_codec"]
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 21))
        self.assertEqual(codec["missing_root_list_max"], 4096)
        self.assertTrue(codec["missing_root_list_canonical"])
        self.assertFalse(codec["row_count_must_match_list_count"])
        self.assertTrue(codec["row_count_counts_missing_root_outcomes"])
        self.assertTrue(codec["list_contains_sorted_unique_missing_root_fdids"])
        self.assertIn("v47", codec["cross_profile_rejection"])

    def test_authentication_and_nonacceptance_boundaries(self):
        contract = self.contract
        missing_root = contract["gameobject_missing_root"]
        self.assertEqual(missing_root["root_absence_authentication"], "producer_reported")
        self.assertEqual(missing_root["maximum_unique_fdids"], 4096)
        self.assertEqual(missing_root["maximum_direct_nonzero_godi_candidate_rows"], 4096)
        self.assertEqual(
            missing_root["maximum_basis"],
            "profile candidate-request ceiling applies to direct nonzero rows; the canonical FDID list deduplicates repeated rows",
        )
        self.assertEqual(missing_root["root_present_without_body"], "fatal; never declared as missing-root")
        verification = contract["verification"]
        self.assertEqual(verification["map_domain_authentication"], "independently_derived")
        self.assertEqual(verification["placement_and_assembly_authentication"], "independently_derived")
        self.assertEqual(verification["gameobject_root_absence_authentication"], "producer_reported")
        self.assertTrue(verification["rejects_root_present_unavailable_bodies"])
        self.assertTrue(verification["rejects_missing_root_list_over_4096"])
        self.assertTrue(verification["rejects_missing_root_candidate_rows_over_4096"])
        self.assertTrue(verification["requires_list_to_equal_unique_missing_root_outcome_fdids"])
        self.assertEqual(
            verification["synthetic_replay_attack_tests"],
            ["4097_missing_root_rows_with_one_unique_fdid_are_rejected_by_candidate_ceiling"],
        )
        boundaries = contract["boundaries"]
        self.assertEqual(boundaries["target_status"], "attempted-failed-unaccepted")
        self.assertTrue(boundaries["synthetic_validation_status"] == "passed")
        self.assertEqual(boundaries["synthetic_v48_cli_tests"], 11)
        self.assertEqual(boundaries["synthetic_v47_cli_regression_tests"], 7)
        self.assertTrue(boundaries["real_target_attempted"])
        attempt = boundaries["real_target_attempt"]
        self.assertEqual(attempt["run_id"], "v48-gameobject-missing-root-wmo-20261028T153819Z")
        self.assertEqual(attempt["extract_exit"], 9)
        self.assertEqual(attempt["component"], "db2")
        self.assertEqual(attempt["code"], "local_store_no_verified_encoding")
        self.assertIsNone(attempt["source_diagnostic"])
        self.assertFalse(attempt["accepted_prefix"])
        self.assertFalse(attempt["package_published"])
        for key in (
            "real_extraction_accepted",
            "full_real_target_acceptance",
            "full_world_coverage",
            "complete_vmap_component",
            "runtime_acceptance",
            "windows",
            "distribution",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(boundaries[key], key)
        self.assertFalse(contract["real_extraction_accepted"])
        self.assertFalse(contract["full_real_target_acceptance"])
        self.assertFalse(contract["full_extractor_complete"])
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])

    def test_recorded_synthetic_gates_match_commands(self):
        validation = self.contract["synthetic_validation"]
        self.assertEqual(
            validation["plan_admission_tests"],
            {
                "status": "passed",
                "command": "source tools/environment.sh && cargo test --offline --locked -p forever-extractor --lib plan::v48_tests",
                "tests": 4,
            },
        )
        self.assertEqual(validation["cli_build_and_execution"]["status"], "passed")
        self.assertEqual(validation["cli_build_and_execution"]["tests"], 11)
        self.assertEqual(
            self.contract["boundaries"]["synthetic_v48_cli_tests"],
            validation["cli_build_and_execution"]["tests"],
        )
        self.assertEqual(
            validation["cli_build_and_execution"]["product_scenarios"],
            [
                "extract_verify_resume_determinism_and_v47_cross_rejection",
                "raw_root_absent_vs_root_present_without_body",
                "repeated_missing_root_fdid_across_two_displays",
                "authenticated_partial_godi_published_and_geometry_continued",
                "authenticated_partial_godi_resume_and_geometry_continuation",
                "initial_db2_acquisition_failure_finishes_redacted_diagnostic_with_db2_component",
                "root_present_model_failure_finishes_redacted_startup_diagnostic_with_vmaps_component",
                "4096_missing_root_ceiling_and_resealed_codec_attacks",
            ],
        )
        self.assertEqual(validation["v47_cli_regression"]["status"], "passed")
        self.assertEqual(validation["v47_cli_regression"]["tests"], 7)
        self.assertEqual(
            validation["software_qualification"],
            {
                "status": "workspace-suite-clippy-and-release-build-passed",
                "workspace_suite": True,
                "workspace_suite_targets": 92,
                "workspace_suite_tests": 4735,
                "workspace_suite_ignored": 26,
                "workspace_suite_command": "source tools/environment.sh && tools/checks rust",
                "clippy": True,
                "release_build": True,
            },
        )


if __name__ == "__main__":
    unittest.main()
