# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 SourceExpandedV2 material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v44.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v44/all.tsv"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]
V43_REPORT_SHA256 = "69c44bfae09f60200640758c741d2f0607dd893fa5bbede96fae3bfed96e1a81"


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV44Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_plan_and_fixture_boundary(self):
        contract = self.contract
        self.assertEqual((contract["contract"], contract["version"]), ("forever-product-v44", 44))
        self.assertEqual(contract["status"], "synthetic-plan-cli-tests-passed; real-target-not-run")
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 30)
        self.assertTrue(fixture.startswith("forever-product\t44\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertIn("linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17", fixture)
        self.assertEqual(contract["publication_profile"], "Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1")
        self.assertEqual((contract["builds"], contract["platform"], contract["offline"]), ([70450], "linux", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(
            (
                contract["plan_profile"],
                contract["plan_coverage_profile"],
                contract["codec_profile"],
                contract["domain_profile"],
                contract["resource_profile"]["id"],
                contract["completion"],
                contract["verification_profile"],
                contract["verification_scope"],
            ),
            (
                "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1",
                "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17",
                "linux70450-source-expanded-v2-material-filtered-wmo-v1",
                "source-expanded-v2-material-filtered-wmo-v1",
                "linux-collision-source-expanded-resources-v2",
                "linux70450-available-source-expanded-v2-material-filtered-wmo-collision-v1",
                "linux70450-source-expanded-v2-material-filtered-wmo-verification-v1",
                "artifact_integrity_source_expanded_v2_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
            ),
        )
        plan = contract["plan"]
        self.assertEqual((plan["forever-product"], plan["product_version"], plan["build"]), ("44", 44, 70450))
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertTrue(plan["storage_budgets_downward_only"])
        fixtures = contract["fixtures"]
        self.assertFalse(fixtures["contains_client_data"])
        self.assertFalse(fixtures["real_extraction_evidence"])
        self.assertTrue(fixtures["used_by_plan_admission"])
        self.assertFalse(fixtures["used_as_plan_file_by_cli_test"])
        self.assertEqual(fixtures["source_plan"], "tests/contracts/fixtures/forever-product-v44/all.tsv")
        self.assertEqual(fixtures["cli_source_plan"], "tests/contracts/fixtures/forever-product-v43/all.tsv")
        self.assertIn("derived in memory", plan["source_fixture"])
        self.assertIn("canonical v44 PLAN fixture", plan["source_fixture"])
        self.assertNotIn("no v44 fixture", plan["source_fixture"])

    def test_source_expanded_v2_changes_only_source_limit(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["basis"], "SourceExpandedV2")
        self.assertEqual(resource["only_limit_changed_vs_v43"], "max_source_bytes")
        expected = {
            "max_reads": 1_000_000,
            "max_requests": 16_000_000,
            "max_dependency_requests": 16_000_000,
            "max_source_bytes": 2_147_483_648,
            "max_raw_bytes": 268_435_456,
            "max_models": 65_536,
        }
        for prefix in ("collector", "startup_collision"):
            for name, value in expected.items():
                self.assertEqual(resource[f"{prefix}_{name}"], value, f"{prefix}_{name}")
        for name in (
            "spool_max_model_identity_records",
            "placements_max_models",
            "model_staging_max_models",
            "package_model_inventory_max",
            "model_output_inventory_max",
        ):
            self.assertEqual(resource[name], 65_536, name)
        self.assertEqual(resource["model_staging_max_bytes"], 536_870_912)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        self.assertEqual(
            resource["preserved_non_source_limits_vs_v43"],
            {
                "max_models": 65_536,
                "max_reads": 1_000_000,
                "max_requests": 16_000_000,
                "max_dependency_requests": 16_000_000,
                "max_raw_bytes": 268_435_456,
                "max_model_output_bytes": 536_870_912,
                "max_staged_model_bytes": 536_870_912,
                "max_memory_bytes": 1_073_741_824,
                "max_wdt_cache_entries": 4096,
                "max_callbacks_per_map_tile": 4096,
                "max_model_identity_records": 65_536,
                "max_placements_models": 65_536,
                "max_package_model_inventory": 65_536,
                "max_model_output_inventory": 65_536,
            },
        )
        self.assertTrue(resource["all_model_identity_routes_coherent"])
        self.assertFalse(resource["disk_world_profile_implicit"])

        limits = self.contract["limits"]
        self.assertEqual(
            (limits["source_bytes_max"], limits["models_max"], limits["reads_max"]),
            (2_147_483_648, 65_536, 1_000_000),
        )
        self.assertEqual(
            (limits["request_calls_max"], limits["dependency_requests_max"], limits["raw_bytes_max"]),
            (16_000_000, 16_000_000, 268_435_456),
        )
        self.assertEqual(
            (limits["plan_collision_vmo_bytes_max"], limits["maximum_staged_model_bytes"]),
            (536_870_912, 536_870_912),
        )
        self.assertEqual(limits["model_output_bytes_max"], 536_870_912)
        self.assertEqual(limits["effective_model_output_bytes_budget"], 536_870_912)
        self.assertEqual(
            limits["effective_model_output_bytes_budget_rule"],
            "min(plan_collision_vmo_bytes_max, maximum_staged_model_bytes)",
        )

    def test_v17_codec_preserves_v16_layout_and_five_omission_lists(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(
            codec["path"],
            "vmaps/available-source-expanded-v2-material-filtered-wmo-geometry-coverage-v17.bin",
        )
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 17))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v16 field layout", codec["preserved_layout"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["domain_profile"], self.contract["domain_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        golden = codec["synthetic_golden"]
        self.assertEqual(
            golden["v17"],
            {"byte_length": 1022, "sha256": "6ca62a384a97721adfaba63477df1b95921832d72571bbdc9298ccadbc60b418"},
        )
        self.assertEqual(
            golden["normalized_static_v16_golden"],
            {
                "byte_length": 1033,
                "sha256": "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a",
                "encoding": "MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_V16_GOLDEN_HEX",
            },
        )
        self.assertTrue(golden["dynamic_v16_encode_comparison"])
        self.assertIn("freshly encoded equivalent v16 data", golden["comparison_scope"])
        self.assertEqual(codec["strict_wrappers"]["v43"]["layout_version"], 16)
        self.assertEqual(codec["strict_wrappers"]["v44"]["layout_version"], 17)
        self.assertIn("all ten pairwise overlaps", codec["cross_profile_rejection"])

    def test_acceptance_matrix_records_attempted_failure_and_design_followup(self):
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        expected = {
            "canonical-profile-and-plan",
            "coverage-codec-v17",
            "resource-profile-preservation",
            "cli-lifecycle",
            "v43-v44-cross-version",
            "resealed-coverage-attacks",
            "omission-list-boundaries",
            "reduced-plan-resource-metadata",
            "real-target",
            "v45-placement-expanded-followup",
        }
        self.assertEqual(set(cases), expected)
        for name in expected - {"real-target", "v45-placement-expanded-followup"}:
            self.assertIn("passed", cases[name]["status"], name)
        self.assertEqual(cases["real-target"]["status"], "target-not-run")
        self.assertEqual(cases["real-target"]["attempt_status"], "attempted; failed; acceptance not achieved")
        self.assertEqual(cases["v45-placement-expanded-followup"]["status"], "design-only; target-not-run")
        validation = self.contract["validation_plan"]
        self.assertEqual(validation["target_status"], "target-not-run")
        self.assertEqual(validation["real_target_status"], "target-not-run")
        self.assertTrue(validation["real_target_attempted"])
        self.assertFalse(validation["real_2_gib_boundary"])
        self.assertFalse(validation["real_extraction_accepted"])
        attempt = validation["real_attempt"]
        self.assertEqual(attempt["run_id"], "v44-source-expanded-v2-wmo-20261027T092051Z")
        self.assertEqual(attempt["report_path"], "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v44-source-expanded-v2-wmo-20261027T092051Z-acceptance.json")
        self.assertEqual(attempt["report_sha256"], "2a29530f77a5f6f72415288f6c6c266d8697fc424d95b6e0c7db8e7891a1966d")
        self.assertEqual(attempt["elapsed_seconds"], 426)
        self.assertEqual(attempt["software_sha256"], "312a01d66144e8fbb6ab76b54871c2eb030bd67c43ba9c2762d47b30a865c3ad")
        self.assertEqual(attempt["binary_sha256"], "622b3c88d9c24c30109a90a8f3e09d7f1bfb9fb689ea6e5b0f9ddb74442b65e3")
        qualification = attempt["software_qualification"]
        self.assertEqual((qualification["workspace_test_suites"], qualification["workspace_tests_passed"], qualification["workspace_tests_failed"], qualification["workspace_tests_ignored"], qualification["contracts_passed"]), (88, 4634, 0, 26, 569))
        self.assertEqual(qualification["software_sha256"], attempt["software_sha256"])
        self.assertEqual(qualification["binary_sha256"], attempt["binary_sha256"])
        self.assertEqual((attempt["inspect_exit"], attempt["extract_exit"], attempt["extract_error"]), (0, 9, "product_selected_geometry_limit"))
        self.assertEqual((attempt["probe_exit"], attempt["verify_exit"], attempt["verify_links_exit"]), (-1, -1, -1))
        self.assertTrue(attempt["inputs_unchanged"])
        self.assertTrue(attempt["client_metadata_unchanged"])
        self.assertFalse(attempt["network_attempted"])
        self.assertEqual(attempt["effective_placement_staging_limit_bytes"], 134_217_728)
        self.assertEqual(attempt["plan_collision_placement_bytes_max"], 268_435_456)
        self.assertFalse(attempt["accepted_prefix"])
        self.assertIsNone(attempt["audit_sha256"])
        self.assertIsNone(attempt["geometry"])
        self.assertIsNone(attempt["source_diagnostic"])
        self.assertFalse(attempt["final_counters_available"])
        self.assertFalse(attempt["confirmed_root_cause"])
        self.assertIn("does not establish it as the observed cause", attempt["placement_staging_candidate"])
        followup = self.contract["v45_followup"]
        self.assertEqual(followup["status"], "design-only; target-not-run")
        self.assertEqual((followup["plan_collision_placement_bytes_max"], followup["max_staged_placement_bytes"]), (268_435_456, 268_435_456))
        self.assertEqual(followup["per_map_placement_bytes_max"], 67_108_864)
        self.assertEqual(followup["coverage_layout_version"], 18)
        self.assertEqual(followup["diagnostics"], ["total_placements", "per_map_placements", "placement_domain"])
        self.assertFalse(followup["disk_world_profile_implicit"])
        self.assertFalse(followup["placement_capacity_sufficiency_proven"])
        for suite, count in (("cli", 15), ("plan_admission", 5), ("library", 794), ("v43_cli", 13)):
            result = validation["synthetic_test_results"][suite]
            self.assertEqual((result["status"], result["passed"], result["failed"]), ("passed", count, 0))
            self.assertTrue(result["command"].startswith("cargo test --offline --locked -p forever-extractor"))
        clippy = validation["synthetic_test_results"]["clippy"]
        self.assertEqual((clippy["status"], clippy["scope"]), ("passed", "forever-extractor all targets"))
        self.assertEqual(clippy["command"], "cargo clippy --all-targets")
        self.assertIn("coverage-codec-v17", {case["case"] for case in self.contract["acceptance_matrix"]})
        self.assertEqual(
            (
                validation["source_bytes_max_inspect_value"],
                validation["models_max_inspect_value"],
                validation["reads_max_inspect_value"],
                validation["model_bytes_max_inspect_value"],
            ),
            (2_147_483_648, 65_536, 1_000_000, 536_870_912),
        )
        self.assertIn("4096", cases["omission-list-boundaries"]["expect"])
        self.assertIn("4097", cases["omission-list-boundaries"]["expect"])
        self.assertIn("both verifiers", cases["omission-list-boundaries"]["expect"])
        self.assertIn("under v44", cases["v43-v44-cross-version"]["expect"])
        self.assertIn("under v43", cases["v43-v44-cross-version"]["expect"])
        self.assertIn("resume", cases["cli-lifecycle"]["expect"])
        self.assertIn("deterministic", cases["cli-lifecycle"]["expect"])
        self.assertIn("no real-client extraction", validation["synthetic_test_scope"])
        self.assertIn("or runtime claim", validation["synthetic_test_scope"])
        self.assertFalse(validation["real_2_gib_boundary"])

    def test_v43_failure_is_historical_and_does_not_establish_v44_sufficiency(self):
        prior = self.contract["v43_prior_failure"]
        self.assertEqual(prior["status"], "failed; v43 only; acceptance not transferred")
        self.assertEqual(prior["run_id"], "v43-model-bytes-expanded-wmo-20261027T070025Z")
        self.assertEqual(prior["report_sha256"], V43_REPORT_SHA256)
        self.assertEqual(prior["extract_error"], "local_store_policy_limit")
        self.assertEqual(
            (prior["source_bytes_max"], prior["remaining_source_cap_bytes"], prior["bounded_probe_decoded_bytes"], prior["observed_excess_bytes"]),
            (1_073_741_824, 311_233, 314_478, 3_245),
        )
        self.assertFalse(prior["declared_length_independently_available"])
        for name in (
            "source_capacity_sufficiency_proven",
            "model_output_sufficiency_proven",
            "wmo_462695_admission_proven",
            "full_world_coverage",
            "runtime_acceptance",
        ):
            self.assertFalse(prior[name], name)
        boundary = self.contract["boundary"]
        self.assertTrue(boundary["real_target_attempted"])
        for name in (
            "real_extraction_accepted",
            "real_2_gib_boundary",
            "source_capacity_sufficiency",
            "model_output_sufficiency",
            "wmo_462695_admission",
            "full_world_coverage",
            "complete_vmap_component",
            "runtime_acceptance",
            "format_parity",
            "windows",
            "distribution",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(boundary[name], name)
        self.assertFalse(self.contract["verification"]["full_world_coverage"])
        self.assertFalse(self.contract["verification"]["runtime_claim"])
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
