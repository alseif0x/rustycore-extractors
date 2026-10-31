# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 source-expanded material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v41.json"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV41Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_contract_identity_and_plan(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v41", 41, "Build70450SourceExpandedMaterialFilteredWmoGeometryV1"),
        )
        self.assertEqual(
            (contract["builds"], contract["platform"], contract["backend"], contract["offline"]),
            ([70450], "linux", "rust-local", True),
        )
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        identities = (
            contract["plan_profile"],
            contract["plan_coverage_profile"],
            contract["codec_profile"],
            contract["domain_profile"],
            contract["resource_profile"]["id"],
            contract["completion"],
            contract["verification_profile"],
            contract["verification_scope"],
        )
        self.assertEqual(
            identities,
            (
                "linux70450-source-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-source-expanded-material-filtered-wmo-coverage-v14",
                "linux70450-source-expanded-material-filtered-wmo-v1",
                "source-expanded-material-filtered-wmo-v1",
                "linux-collision-source-expanded-resources-v1",
                "linux70450-available-source-expanded-material-filtered-wmo-collision-v1",
                "linux70450-source-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_source_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
            ),
        )
        plan = contract["plan"]
        self.assertEqual(plan["forever-product"], "41")
        self.assertEqual(plan["product_version"], 41)
        self.assertEqual(plan["build"], 70450)
        self.assertEqual(plan["collision_profile"], contract["plan_profile"])
        self.assertEqual(plan["coverage_profile"], contract["plan_coverage_profile"])
        self.assertEqual(plan["domain_closure"], contract["domain_profile"])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["component_mask"], 17)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1048576)
        fixture = contract["fixtures"]
        self.assertEqual(
            plan["source_fixture"],
            "tests/contracts/fixtures/forever-product-v41/all.tsv; canonical synthetic PLAN fixture, no client data or real extraction evidence",
        )
        self.assertEqual(
            fixture["canonical_plan"],
            "tests/contracts/fixtures/forever-product-v41/all.tsv",
        )
        self.assertTrue((ROOT / fixture["canonical_plan"]).is_file())
        self.assertTrue(fixture["used_by_plan_admission_tests"])
        self.assertTrue(fixture["used_by_cli_tests"])
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])

    def test_only_source_capacity_is_raised(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-source-expanded-resources-v1")
        self.assertEqual(resource["only_limit_raised_vs_v40"], "max_source_bytes")
        for prefix in ("collector", "startup_collision"):
            self.assertEqual(resource[f"{prefix}_max_source_bytes"], 1_073_741_824)
            self.assertEqual(resource[f"{prefix}_max_models"], 65_536)
            self.assertEqual(resource[f"{prefix}_max_reads"], 1_000_000)
            self.assertEqual(resource[f"{prefix}_max_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_dependency_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_raw_bytes"], 268_435_456)
        self.assertEqual(resource["spool_max_model_identity_records"], 65_536)
        self.assertEqual(resource["placements_max_models"], 65_536)
        self.assertEqual(resource["model_staging_max_models"], 65_536)
        self.assertEqual(resource["package_model_inventory_max"], 65_536)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        self.assertFalse(resource["disk_world_profile_implicit"])

        limits = self.contract["limits"]
        self.assertEqual(limits["source_bytes_max"], 1_073_741_824)
        self.assertEqual(limits["models_max"], 65_536)
        self.assertEqual(limits["reads_max"], 1_000_000)
        self.assertEqual(limits["request_calls_max"], 16_000_000)
        self.assertEqual(limits["dependency_requests_max"], 16_000_000)
        self.assertEqual(limits["raw_bytes_max"], 268_435_456)
        self.assertTrue(limits["source_bytes_max_raised_vs_v40"])
        for name in (
            "models_max_raised_vs_v40",
            "reads_max_raised_vs_v40",
            "request_calls_max_raised_vs_v40",
            "dependency_requests_max_raised_vs_v40",
            "raw_bytes_max_raised_vs_v40",
        ):
            self.assertFalse(limits[name], name)
        for name in LIST_ORDER:
            self.assertEqual(limits[f"{name}_max"], 4096)

    def test_v14_codec_keeps_v13_layout_and_five_lists(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(
            codec["path"],
            "vmaps/available-source-expanded-material-filtered-wmo-geometry-coverage-v14.bin",
        )
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 14))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v13 field layout", codec["preserved_layout"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        self.assertEqual(codec["strict_wrappers"]["v40"]["layout_version"], 13)
        self.assertEqual(codec["strict_wrappers"]["v41"]["layout_version"], 14)
        self.assertIn("cross-version", codec["cross_profile_rejection"])
        self.assertTrue(self.contract["wmo_omissions"]["same_predicates_as_v40"])

    def test_cli_acceptance_scope_and_limits(self):
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        for name in (
            "canonical-profile-and-plan",
            "synthetic-plan-fixture",
            "coverage-codec-v14",
            "source-expanded-resource-profile",
            "reduced-plan-budget",
            "cli-lifecycle",
            "production-verifier-attacks",
            "v40-v41-cross-version",
            "list-limits",
            "cli-resource-reports",
            "real-acceptance",
        ):
            self.assertIn(name, cases)
        self.assertIn("resume", cases["cli-lifecycle"]["expect"])
        self.assertIn("verify and verify-links", cases["cli-lifecycle"]["expect"])
        self.assertIn("11/11", cases["cli-lifecycle"]["evidence"])
        self.assertIn("7/7", cases["canonical-profile-and-plan"]["evidence"])
        self.assertIn("2/2", cases["coverage-codec-v14"]["evidence"])
        self.assertIn("766/766", cases["source-expanded-resource-profile"]["evidence"])
        self.assertIn("all ten pairwise omission-list overlaps", cases["production-verifier-attacks"]["evidence"])
        self.assertIn("resealed", cases["production-verifier-attacks"]["evidence"])
        self.assertIn("all ten pairwise omission-list overlaps", cases["production-verifier-attacks"]["expect"])
        self.assertIn("v40 and v41", cases["v40-v41-cross-version"]["expect"])
        self.assertIn("4096 and rejects 4097", cases["list-limits"]["expect"])
        self.assertIn("no real 1 GiB boundary", cases["cli-resource-reports"]["expect"])
        self.assertEqual(cases["real-acceptance"]["status"], "failed")
        self.assertEqual(cases["follow-up-v42"]["status"], "design-only; target-not-run")

    def test_real_v41_failure_and_v42_followup_are_bounded(self):
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v41")
        self.assertEqual(real["run_id"], "v41-source-expanded-wmo-20261027T020021Z")
        self.assertEqual(real["report_sha256"], "d3c2bef14736c0d7523ccf3e7469a2db9c18b92665283a2b9db9e93d753cad72")
        self.assertEqual(real["software_sha256"], "4ca63bee25f512c13452d129b5bb3fa209170380d98a186c7793b1b43e42497d")
        self.assertEqual(real["binary_sha256"], "18219365db26c56466deaa0257687228475bf7ebc25fd7377a345a464d2700d9")
        self.assertEqual(
            (real["inspect_exit"], real["extract_exit"], real["extract_error"], real["elapsed_seconds"]),
            (0, 9, "product_selected_geometry_limit", 115),
        )
        self.assertEqual((real["probe_exit"], real["verify_exit"], real["verify_links_exit"]), (-1, -1, -1))
        self.assertFalse(real["accepted"])
        self.assertFalse(real["accepted_prefix"])
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["inputs_unchanged"])
        self.assertTrue(real["client_metadata_unchanged"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        self.assertFalse(real["diagnostic_present"])
        self.assertFalse(real["final_casc_counters_available"])
        for counter in ("observed_source_bytes_at_failure", "observed_models_at_failure", "observed_reads_at_failure"):
            self.assertIsNone(real[counter])
        guard = real["model_output_inventory_guard"]
        self.assertEqual(guard["source_path"], "crates/forever-extractor/src/driver/collision_geometry_v22.rs")
        self.assertEqual(guard["condition"], "self.models.len() >= 4096")
        self.assertEqual(guard["source_profile_models_max"], 65_536)
        self.assertIsNone(guard["triggered_in_this_run"])
        self.assertFalse(guard["failure_cause_count_vs_bytes_proven"])
        self.assertEqual(real["software_tests"], {"workspace_test_suites": 85, "passed": 4558, "failed": 0, "ignored": 26, "contract_tests_passed": 552})
        self.assertFalse(real["real_source_1gib_boundary"])
        self.assertFalse(real["source_capacity_sufficiency_proven"])
        self.assertFalse(real["wmo_462695_admission_proven"])
        self.assertFalse(real["full_world_coverage"])
        self.assertFalse(real["runtime_claim"])
        self.assertTrue(self.contract["boundary"]["real_extraction_failed"])

        followup = self.contract["v42_follow_up"]
        self.assertEqual(followup["status"], "design-only; target-not-run")
        self.assertEqual(followup["profile"], "ModelOutputExpandedV1")
        self.assertEqual(followup["coverage_version"], 15)
        self.assertEqual(followup["output_inventory_capacity"], 65_536)
        self.assertTrue(followup["output_inventory_capacity_coherent"])
        self.assertTrue(followup["diagnostics"]["model_count_limit_separate"])
        self.assertTrue(followup["diagnostics"]["model_bytes_limit_separate"])
        self.assertTrue(followup["preserve_all_other_limits"])
        self.assertFalse(followup["disk_world_profile_implicit"])
        self.assertFalse(followup["real_target_attempted"])

        prior = self.contract["prior_v40_failure_evidence"]
        self.assertEqual(prior["status"], "failed; historical v40 only, not v41 acceptance")
        self.assertEqual(prior["run_id"], "v40-model-heavy-wmo-20261027T000044Z")
        self.assertEqual(
            prior["report_sha256"],
            "f1851ba06cf85679f4f83d5c080352c81dff533c655843f78232f2b67b3cc0c9",
        )
        self.assertEqual(
            prior["source_limit_probe_sha256"],
            "93cbe5e506bd21d0f8799001b148c83168001fb718ce5e676ce1f1a5b9d29051",
        )
        self.assertEqual(prior["extract_error"], "local_store_policy_limit")
        self.assertEqual(
            (prior["source_bytes"], prior["source_bytes_max"], prior["source_bytes_remaining"]),
            (536_853_486, 536_870_912, 17_426),
        )
        self.assertEqual(prior["probe_excess"], 4_798)
        self.assertFalse(prior["acceptance_transferred"])
        self.assertFalse(prior["wmo_462695_admission_proven"])
        self.assertFalse(self.contract["boundary"]["real_source_1gib_boundary"])
        self.assertFalse(self.contract["boundary"]["source_capacity_sufficiency"])
        self.assertFalse(self.contract["boundary"]["full_world_coverage"])
        self.assertFalse(self.contract["boundary"]["runtime_acceptance"])
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
