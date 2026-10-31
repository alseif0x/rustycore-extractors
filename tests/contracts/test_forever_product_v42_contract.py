# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 model-output-expanded material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v42.json"
V42_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v42/all.tsv"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV42Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_plan_and_existing_synthetic_source(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["status"]),
            ("forever-product-v42", 42, "synthetic-implementation-partial; real-target-failed"),
        )
        self.assertEqual(
            (contract["publication_profile"], contract["builds"], contract["platform"]),
            ("Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1", [70450], "linux"),
        )
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
                "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15",
                "linux70450-model-output-expanded-material-filtered-wmo-v1",
                "model-output-expanded-material-filtered-wmo-v1",
                "linux-collision-model-output-expanded-resources-v1",
                "linux70450-available-model-output-expanded-material-filtered-wmo-collision-v1",
                "linux70450-model-output-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_model_output_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
            ),
        )
        plan = contract["plan"]
        self.assertEqual((plan["forever-product"], plan["product_version"], plan["build"]), ("42", 42, 70450))
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertEqual(plan["collision_profile"], contract["plan_profile"])
        self.assertEqual(plan["coverage_profile"], contract["plan_coverage_profile"])
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1_048_576)

        fixtures = contract["fixtures"]
        self.assertEqual(fixtures["canonical_plan"], "tests/contracts/fixtures/forever-product-v42/all.tsv")
        self.assertEqual(plan["source_fixture"].split(";", 1)[0], fixtures["canonical_plan"])
        self.assertTrue(V42_FIXTURE.is_file())
        self.assertTrue(fixtures["used_by_plan_admission_tests"])
        self.assertTrue(fixtures["used_by_cli_test"])
        self.assertFalse(fixtures["contains_client_data"])
        self.assertFalse(fixtures["real_extraction_evidence"])
        self.assertIn("canonical synthetic v42 PLAN fixture", fixtures["classification"])

    def test_source_expanded_acquisition_limits_are_preserved(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["basis"], "SourceExpandedV1")
        self.assertEqual(resource["only_limit_changed_vs_v41"], "model_output_inventory_capacity")
        expected = {
            "max_reads": 1_000_000,
            "max_requests": 16_000_000,
            "max_dependency_requests": 16_000_000,
            "max_source_bytes": 1_073_741_824,
            "max_raw_bytes": 268_435_456,
            "max_models": 65_536,
        }
        for prefix in ("collector", "startup_collision"):
            for key, value in expected.items():
                self.assertEqual(resource[f"{prefix}_{key}"], value)
        for key in (
            "spool_max_model_identity_records",
            "placements_max_models",
            "model_staging_max_models",
            "package_model_inventory_max",
            "model_output_inventory_max",
        ):
            self.assertEqual(resource[key], 65_536, key)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        self.assertTrue(resource["all_model_identity_routes_coherent"])
        self.assertFalse(resource["disk_world_profile_implicit"])

        limits = self.contract["limits"]
        self.assertEqual(
            (limits["source_bytes_max"], limits["models_max"], limits["reads_max"]),
            (1_073_741_824, 65_536, 1_000_000),
        )
        self.assertEqual(
            (limits["request_calls_max"], limits["dependency_requests_max"], limits["raw_bytes_max"]),
            (16_000_000, 16_000_000, 268_435_456),
        )
        self.assertEqual(limits["model_output_inventory_max"], 65_536)
        self.assertEqual(limits["model_output_bytes_max"], 402_653_184)
        self.assertEqual(limits["maximum_staged_model_bytes"], 402_653_184)
        self.assertEqual(limits["plan_collision_vmo_bytes_max"], 268_435_456)
        self.assertEqual(limits["effective_model_output_bytes_budget"], 268_435_456)
        self.assertEqual(
            limits["effective_model_output_bytes_budget_rule"],
            "min(plan_collision_vmo_bytes_max, maximum_staged_model_bytes)",
        )
        self.assertTrue(limits["model_output_inventory_capacity_raised_vs_v41"])
        for name in (
            "source_bytes_max_raised_vs_v41",
            "models_max_raised_vs_v41",
            "reads_max_raised_vs_v41",
            "request_calls_max_raised_vs_v41",
            "dependency_requests_max_raised_vs_v41",
            "raw_bytes_max_raised_vs_v41",
        ):
            self.assertFalse(limits[name], name)

    def test_v15_codec_keeps_v14_field_layout_and_five_lists(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(
            codec["path"],
            "vmaps/available-model-output-expanded-material-filtered-wmo-geometry-coverage-v15.bin",
        )
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 15))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v14 field layout", codec["preserved_layout"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["domain_profile"], self.contract["domain_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        self.assertEqual(codec["strict_wrappers"]["v41"]["layout_version"], 14)
        self.assertEqual(codec["strict_wrappers"]["v42"]["layout_version"], 15)
        self.assertIn("v41 and v42 plans/packages/coverage reject crossing", codec["cross_profile_rejection"])
        self.assertTrue(self.contract["wmo_omissions"]["same_predicates_as_v41"])

    def test_model_output_limits_and_focused_validation_scope(self):
        diagnostics = self.contract["model_output_limit_diagnostics"]
        self.assertEqual(diagnostics["count_code"], "product_selected_geometry_model_count_limit")
        self.assertEqual(diagnostics["byte_code"], "product_selected_geometry_model_bytes_limit")
        self.assertTrue(diagnostics["count_byte_codes_integrated"])
        self.assertTrue(diagnostics["sink_count_byte_diagnostic_tests"])
        self.assertEqual(
            diagnostics["sink_test_source"],
            "crates/forever-extractor/src/driver/collision_geometry_v22/tests.rs",
        )
        self.assertIn("not CLI tests", diagnostics["sink_test_scope"])
        self.assertEqual(diagnostics["byte_diagnostic_validation"], "passed")
        self.assertIn("default VMO conversion first", diagnostics["byte_check_order"])
        self.assertIn("accumulated converted model-output byte budget", diagnostics["byte_check_order"])
        self.assertEqual(diagnostics["stage_model_4097_validation"], "passed")
        self.assertEqual(diagnostics["stage_model_4097_result"], {"v42": "accepts", "v41": "rejects"})
        self.assertIn("not an extractor CLI run", diagnostics["stage_model_test_scope"])
        self.assertFalse(diagnostics["nominal_65536_identity_boundary_proven"])
        self.assertTrue(diagnostics["target_observed"])
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        self.assertEqual(cases["model-output-count-vs-bytes-diagnostics"]["status"], "passed")
        self.assertEqual(cases["stage-model-4097"]["status"], "passed")
        self.assertIn("not characterize it as a CLI extraction", cases["stage-model-4097"]["expect"])
        self.assertIn("65536 identity boundary", cases["stage-model-4097"]["expect"])
        self.assertEqual(cases["coverage-codec-v15"]["status"], "passed")
        self.assertEqual(cases["omission-list-boundaries"]["status"], "passed")

    def test_synthetic_cli_acceptance_and_real_boundary_are_explicit(self):
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        evidence = self.contract["synthetic_validation_evidence"]
        self.assertEqual((evidence["library_tests_passed"], evidence["library_tests_total"]), (782, 782))
        self.assertEqual((evidence["v42_cli_tests_passed"], evidence["v42_cli_tests_total"]), (12, 12))
        self.assertEqual((evidence["v41_cli_tests_passed"], evidence["v41_cli_tests_total"]), (11, 11))
        self.assertEqual(evidence["v15_golden_sha256"], "433d78046326fdc9a2c40eee6b55c139e86ea844695bda1201546fadbba01b41")
        self.assertEqual((evidence["auditor_cases"], evidence["auditor_reports"]), (50, 142))
        self.assertTrue(evidence["auditor_passed"])
        self.assertEqual(evidence["real_inspect_compatibility"]["status"], "passed")
        self.assertIn("inspect-only", evidence["real_inspect_compatibility"]["scope"])
        self.assertEqual(evidence["real_inspect_mutation_cases"], 6)
        self.assertEqual(
            evidence["real_inspect_mutation_fields"],
            ["source_bytes_max", "models_max", "reads_max"],
        )
        self.assertEqual(
            evidence["real_inspect_mutations"],
            [
                "source_bytes_max:missing",
                "source_bytes_max:wrong",
                "models_max:missing",
                "models_max:wrong",
                "reads_max:missing",
                "reads_max:wrong",
            ],
        )
        self.assertFalse(evidence["model_output_inventory_max_stdout_required"])
        self.assertIn("independent metadata", evidence["model_output_inventory_max_stdout_note"])
        self.assertFalse(evidence["nominal_65536_identity_boundary_proven"])
        self.assertFalse(evidence["cli_extraction_with_4097_artifacts_claimed"])
        self.assertFalse(evidence["real_extraction_acceptance"])
        self.assertEqual(cases["cli-lifecycle"]["status"], "passed")
        self.assertIn("12/12", cases["cli-lifecycle"]["evidence"])
        self.assertIn("11/11", cases["cli-lifecycle"]["evidence"])
        self.assertEqual(cases["canonical-profile-and-plan"]["status"], "passed")
        self.assertIn("782/782 passed", cases["canonical-profile-and-plan"]["evidence"])
        self.assertIn("6 passed", cases["canonical-profile-and-plan"]["evidence"])
        self.assertEqual(cases["resource-profile-preservation"]["status"], "passed")
        self.assertEqual(cases["v41-v42-cross-version"]["status"], "passed")
        self.assertEqual(cases["resealed-coverage-attacks"]["status"], "passed")
        for name in (
            "canonical-profile-and-plan",
            "synthetic-plan-source",
            "coverage-codec-v15",
            "resource-profile-preservation",
            "cli-lifecycle",
            "v41-v42-cross-version",
            "resealed-coverage-attacks",
            "omission-list-boundaries",
            "stage-model-4097",
            "model-output-count-vs-bytes-diagnostics",
            "contract-tests",
            "auditor-and-inspect-compatibility",
            "real-acceptance",
            "follow-up-v43-model-bytes-expanded",
        ):
            self.assertIn(name, cases)
        self.assertEqual(cases["real-acceptance"]["status"], "failed")
        self.assertEqual(cases["follow-up-v43-model-bytes-expanded"]["status"], "design-only; target-not-run")
        self.assertIn("536870912", cases["follow-up-v43-model-bytes-expanded"]["expect"])
        self.assertIn("v16", cases["follow-up-v43-model-bytes-expanded"]["expect"])
        self.assertIn("resume", cases["cli-lifecycle"]["expect"])
        self.assertIn("verify and verify-links", cases["cli-lifecycle"]["expect"])
        self.assertIn("both directions", cases["v41-v42-cross-version"]["expect"])
        self.assertTrue(self.contract["boundary"]["real_target_attempted"])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["run_id"], "v42-model-output-expanded-wmo-20261027T044414Z")
        self.assertEqual(real["elapsed_seconds"], 169)
        self.assertEqual((real["inspect_exit"], real["extract_exit"], real["extract_error"]), (0, 9, "product_selected_geometry_model_bytes_limit"))
        self.assertEqual((real["probe_exit"], real["verify_exit"], real["verify_links_exit"]), (-1, -1, -1))
        self.assertFalse(real["network_attempted"])
        self.assertFalse(real["accepted_prefix"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        self.assertFalse(real["diagnostic_present"])
        self.assertFalse(real["final_casc_counters_available"])
        self.assertIsNone(real["observed_source_bytes_at_failure"])
        self.assertIsNone(real["observed_models_at_failure"])
        self.assertIsNone(real["observed_reads_at_failure"])
        self.assertTrue(real["failing_model_passed_count_check"])
        self.assertEqual(real["model_count_check_max"], 65536)
        self.assertEqual(real["plan_collision_vmo_bytes_max"], 268435456)
        self.assertEqual(real["maximum_staged_model_bytes"], 402653184)
        self.assertEqual(real["effective_model_output_bytes_budget"], 268435456)
        self.assertEqual(
            real["effective_model_output_bytes_budget_rule"],
            "min(plan_collision_vmo_bytes_max, maximum_staged_model_bytes)",
        )
        self.assertFalse(real["maximum_staged_model_bytes_reached"])
        self.assertEqual(
            real["report_sha256"],
            "347548262472d679b08efa89b4a988dfa3e05c9cb5307ff4586fe5af24ccc651",
        )
        self.assertEqual(
            real["software_sha256"],
            "4cb7b734d0f92a1a99566f959a5f44ad6ac8ce8ef71fdd73eb0cbb7de09a9c54",
        )
        self.assertEqual(
            real["binary_sha256"],
            "c5eebbeec9e2e4fbd556c0b35f8cead8389465dc1f558047e8b29078a8ff8f7e",
        )
        self.assertEqual(real["software_tests"], {"suites": 86, "passed": 4589, "ignored": 26, "contracts": 558})
        v43 = self.contract["follow_up_v43"]
        self.assertEqual(v43["status"], "design-only; target-not-run")
        self.assertEqual(v43["profile"], "ModelBytesExpandedV1")
        self.assertEqual((v43["model_staging_max_bytes"], v43["plan_collision_vmo_bytes_max"]), (536870912, 536870912))
        self.assertEqual(v43["model_staging_raised_from_bytes"], 402653184)
        self.assertEqual(v43["plan_collision_vmo_bytes_raised_from"], 268435456)
        self.assertEqual(v43["effective_model_output_bytes_budget"], 536870912)
        self.assertEqual(
            v43["effective_model_output_bytes_budget_rule"],
            "min(plan_collision_vmo_bytes_max, model_staging_max_bytes)",
        )
        self.assertEqual(v43["planned_coverage_layout_version"], 16)
        self.assertFalse(v43["other_limits_changed"])
        self.assertFalse(v43["disk_world_profile_implicit"])
        self.assertFalse(v43["source_capacity_sufficiency_proven"])
        self.assertFalse(v43["model_output_inventory_sufficiency_proven"])
        self.assertFalse(v43["wmo_462695_admission_proven"])
        self.assertFalse(v43["full_world_coverage"])
        self.assertFalse(v43["runtime_acceptance"])
        for name in (
            "real_source_1gib_boundary",
            "source_capacity_sufficiency",
            "model_output_inventory_sufficiency",
            "wmo_462695_admission",
            "full_world_coverage",
            "runtime_acceptance",
            "full_extractor_complete",
        ):
            self.assertFalse(self.contract["boundary"][name], name)
        self.assertFalse(self.contract["full_extractor_complete"])
        self.assertFalse(self.contract["synthetic_validation_evidence"]["real_extraction_acceptance"])
        self.assertIn("6/6 current", cases["contract-tests"]["evidence"])
        self.assertFalse(self.contract["synthetic_validation_evidence"]["nominal_65536_identity_boundary_proven"])

    def test_v41_failure_remains_generic_and_is_not_attributed(self):
        prior = self.contract["prior_v41_failure"]
        self.assertEqual(prior["status"], "failed; historical v41 only, no acceptance transferred")
        self.assertEqual(prior["run_id"], "v41-source-expanded-wmo-20261027T020021Z")
        self.assertEqual(
            prior["report_sha256"],
            "d3c2bef14736c0d7523ccf3e7469a2db9c18b92665283a2b9db9e93d753cad72",
        )
        self.assertEqual((prior["extract_exit"], prior["extract_error"]), (9, "product_selected_geometry_limit"))
        self.assertFalse(prior["diagnostic_present"])
        self.assertFalse(prior["final_casc_counters_available"])
        for key in (
            "observed_source_bytes_at_failure",
            "observed_models_at_failure",
            "observed_reads_at_failure",
        ):
            self.assertIsNone(prior[key], key)
        guard = prior["model_output_inventory_guard"]
        self.assertEqual(guard["condition"], "self.models.len() >= 4096")
        self.assertIsNone(guard["triggered_in_v41_run"])
        self.assertFalse(guard["failure_cause_count_vs_bytes_proven"])
        self.assertFalse(prior["source_capacity_sufficiency_proven"])
        self.assertFalse(prior["wmo_462695_admission_proven"])


if __name__ == "__main__":
    unittest.main()
