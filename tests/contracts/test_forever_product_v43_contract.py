# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 model-bytes-expanded material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v43.json"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]
V42_REPORT_SHA256 = "347548262472d679b08efa89b4a988dfa3e05c9cb5307ff4586fe5af24ccc651"
V43_REPORT_SHA256 = "69c44bfae09f60200640758c741d2f0607dd893fa5bbede96fae3bfed96e1a81"
V43_SOFTWARE_SHA256 = "be1ea515cbe30f3989ceeefcac56378385dd3cd24a0ec37fbd7cfb59e71eeac6"
V43_DIAGNOSTIC_SHA256 = "cc44adc0b1d2648ecb8fac9d9d9d5fa4485a61d292c8ee5722775a6a518ad0d7"
V43_PROBE_REPORT_SHA256 = "aad35ce11039b604a01e2d8a1bc4b220147c6dd735d06ee70d07506d882f586d"


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV43Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_plan_and_synthetic_fixture_boundary(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"]),
            ("forever-product-v43", 43),
        )
        self.assertEqual(
            contract["status"],
            "synthetic-cli-integration-passed; real-target-failed; follow-up-v44-design-only",
        )
        self.assertEqual(
            (contract["publication_profile"], contract["builds"], contract["platform"]),
            ("Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1", [70450], "linux"),
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
                "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16",
                "linux70450-model-bytes-expanded-material-filtered-wmo-v1",
                "model-bytes-expanded-material-filtered-wmo-v1",
                "linux-collision-model-bytes-expanded-resources-v1",
                "linux70450-available-model-bytes-expanded-material-filtered-wmo-collision-v1",
                "linux70450-model-bytes-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_model_bytes_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
            ),
        )
        plan = contract["plan"]
        self.assertEqual((plan["forever-product"], plan["product_version"], plan["build"]), ("43", 43, 70450))
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1_048_576)
        fixture = contract["fixtures"]
        self.assertEqual(fixture["canonical_plan"], "tests/contracts/fixtures/forever-product-v43/all.tsv")
        self.assertEqual(
            fixture["source_fixture_for_cli_transform"],
            "tests/contracts/fixtures/forever-product-v42/all.tsv",
        )
        self.assertTrue(fixture["used_by_cli_test"])
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertTrue(fixture["used_by_plan_admission_tests"])
        self.assertIn("in memory", plan["source_fixture"])
        self.assertIn("canonical synthetic v43 PLAN fixture", fixture["classification"])

    def test_resource_limits_and_independent_byte_ceilings(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["basis"], "ModelBytesExpandedV1")
        for prefix in ("collector", "startup_collision"):
            expected = {
                "max_reads": 1_000_000,
                "max_requests": 16_000_000,
                "max_dependency_requests": 16_000_000,
                "max_source_bytes": 1_073_741_824,
                "max_raw_bytes": 268_435_456,
                "max_models": 65_536,
            }
            for name, value in expected.items():
                self.assertEqual(resource[f"{prefix}_{name}"], value)
        self.assertEqual(resource["model_staging_max_models"], 65_536)
        self.assertEqual(resource["model_staging_max_bytes"], 536_870_912)
        self.assertEqual(resource["package_model_inventory_max"], 65_536)
        self.assertEqual(resource["model_output_inventory_max"], 65_536)
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
        self.assertEqual(
            (limits["plan_collision_vmo_bytes_raised_from"], limits["plan_collision_vmo_bytes_max"]),
            (268_435_456, 536_870_912),
        )
        self.assertEqual(
            (limits["maximum_staged_model_bytes_raised_from"], limits["maximum_staged_model_bytes"]),
            (402_653_184, 536_870_912),
        )
        self.assertEqual(limits["model_output_bytes_max"], 536_870_912)
        self.assertEqual(limits["effective_model_output_bytes_budget"], 536_870_912)
        self.assertEqual(
            limits["effective_model_output_bytes_budget_rule"],
            "min(plan_collision_vmo_bytes_max, maximum_staged_model_bytes)",
        )

    def test_v16_codec_preserves_v15_layout_and_five_lists(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(
            codec["path"],
            "vmaps/available-model-bytes-expanded-material-filtered-wmo-geometry-coverage-v16.bin",
        )
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 16))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v15 field layout", codec["preserved_layout"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["domain_profile"], self.contract["domain_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        self.assertEqual(codec["strict_wrappers"]["v42"]["layout_version"], 15)
        self.assertEqual(codec["strict_wrappers"]["v43"]["layout_version"], 16)
        golden = self.contract["validation_plan"]["v43_golden_v16"]
        self.assertEqual(golden["bytes"], 1033)
        self.assertEqual(golden["digest_algorithm"], "SHA-256")
        self.assertEqual(
            golden["sha256"],
            "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a",
        )
        self.assertIn("byte-for-byte", golden["normalization"])
        self.assertIn("synthetic", golden["scope"])
        self.assertIn("v42 and v43", codec["cross_profile_rejection"])
        self.assertIn("all ten pairwise overlaps", codec["cross_profile_rejection"])

    def test_synthetic_cli_cases_record_passed_checks_without_acceptance_claims(self):
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        expected_cases = {
            "canonical-profile-and-plan",
            "coverage-codec-v16",
            "resource-profile-preservation",
            "cli-lifecycle",
            "v42-v43-cross-version",
            "resealed-coverage-attacks",
            "omission-list-boundaries",
            "reduced-plan-resource-metadata",
            "contract-tests",
            "v43-plan-admission",
            "extractor-library",
            "v42-cli-regression",
            "v43-cli",
            "real-target",
            "v44-source-expanded-v2-design",
        }
        self.assertEqual(set(cases), expected_cases)
        for name in expected_cases - {"contract-tests", "real-target", "v44-source-expanded-v2-design"}:
            self.assertTrue(cases[name]["status"].startswith("passed"), name)
        self.assertEqual(cases["contract-tests"]["status"], "passed")
        self.assertIn("1033 bytes", cases["coverage-codec-v16"]["evidence"])
        self.assertIn(
            "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a",
            cases["coverage-codec-v16"]["evidence"],
        )
        self.assertEqual(cases["extractor-library"]["status"], "passed")
        self.assertIn("788/788", cases["extractor-library"]["evidence"])
        self.assertEqual(cases["v43-cli"]["status"], "passed")
        self.assertIn("13/13", cases["v43-cli"]["evidence"])
        self.assertEqual(cases["real-target"]["status"], "failed at configured source-byte limit")
        self.assertIn("local_store_policy_limit", cases["real-target"]["evidence"])
        self.assertEqual(cases["v44-source-expanded-v2-design"]["status"], "design-only; target-not-run")
        validation = self.contract["validation_plan"]
        self.assertIn("synthetic product/CLI tests passed", validation["classification"])
        self.assertEqual(
            validation["v43_plan_admission_tests"],
            {
                "passed": 5,
                "failed": 0,
                "total": 5,
                "scope": "cargo test -p forever-extractor --lib plan::v43_tests",
            },
        )
        self.assertEqual(
            validation["v43_golden_v16"]["sha256"],
            "3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a",
        )
        self.assertEqual(
            validation["v43_cli_execution"],
            {"passed": 13, "failed": 0, "ignored": 0, "total": 13},
        )
        self.assertEqual(
            validation["v42_cli_regression"],
            {"passed": 12, "failed": 0, "ignored": 0, "total": 12},
        )
        self.assertEqual(
            validation["extractor_library_tests"],
            {"passed": 788, "failed": 0, "ignored": 0, "total": 788},
        )
        self.assertEqual(validation["contract_tests"], "6/6 passed")
        self.assertEqual(validation["cli_format_check"], "passed")
        self.assertIn("resume", cases["cli-lifecycle"]["expect"])
        self.assertIn("deterministic", cases["cli-lifecycle"]["expect"])
        self.assertIn("both verifiers", cases["v42-v43-cross-version"]["expect"])
        self.assertIn("ten pairwise", cases["resealed-coverage-attacks"]["expect"])
        self.assertIn("4096", cases["omission-list-boundaries"]["expect"])
        self.assertIn("4097", cases["omission-list-boundaries"]["expect"])
        self.assertIn("model_bytes_max=536870912", cases["reduced-plan-resource-metadata"]["expect"])
        self.assertFalse(self.contract["validation_plan"]["real_512_mib_boundary"])
        self.assertEqual(
            self.contract["validation_plan"]["model_bytes_max_inspect_value"],
            536_870_912,
        )

    def test_v42_failed_predecessor_is_bounded_without_count_inference(self):
        prior = self.contract["v42_prior_failure"]
        self.assertEqual(prior["status"], "failed; v42 only; acceptance not transferred")
        self.assertEqual(prior["run_id"], "v42-model-output-expanded-wmo-20261027T044414Z")
        self.assertEqual(prior["report_sha256"], V42_REPORT_SHA256)
        self.assertEqual(prior["extract_error"], "product_selected_geometry_model_bytes_limit")
        self.assertFalse(prior["final_casc_counters_available"])
        for counter in (
            "observed_source_bytes_at_failure",
            "observed_models_at_failure",
            "observed_reads_at_failure",
        ):
            self.assertIsNone(prior[counter], counter)
        self.assertEqual(prior["model_count_check_max"], 65_536)
        self.assertTrue(prior["failing_model_passed_count_check"])
        self.assertFalse(prior["inference_more_than_4096_models"])
        self.assertEqual(prior["plan_collision_vmo_bytes_max"], 268_435_456)
        self.assertEqual(prior["maximum_staged_model_bytes"], 402_653_184)
        self.assertEqual(prior["effective_model_output_bytes_budget"], 268_435_456)
        self.assertFalse(prior["source_capacity_sufficiency_proven"])
        self.assertFalse(prior["model_output_inventory_sufficiency_proven"])
        self.assertFalse(prior["wmo_462695_admission_proven"])
        self.assertFalse(prior["full_world_coverage"])
        self.assertFalse(prior["runtime_acceptance"])

    def test_v43_failed_real_attempt_and_no_acceptance_boundaries(self):
        contract = self.contract
        self.assertIn("real target attempted and failed", contract["validation_plan"]["classification"])
        self.assertTrue(contract["validation_plan"]["real_target_attempted"])
        self.assertFalse(contract["validation_plan"]["real_extraction_accepted"])
        self.assertFalse(contract["validation_plan"]["real_extraction_acceptance"])
        real = contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["run_id"], "v43-model-bytes-expanded-wmo-20261027T070025Z")
        self.assertEqual(real["report_sha256"], V43_REPORT_SHA256)
        self.assertEqual(real["software_json_sha256"], V43_SOFTWARE_SHA256)
        self.assertEqual(real["software_sha256_as_recorded_in_report"], V43_SOFTWARE_SHA256)
        self.assertEqual(real["diagnostic_jsonl_sha256"], V43_DIAGNOSTIC_SHA256)
        self.assertEqual((real["inspect_exit"], real["extract_exit"]), (0, 9))
        self.assertEqual((real["extract_error"], real["extract_component"]), ("local_store_policy_limit", "vmaps"))
        self.assertEqual((real["probe_exit"], real["verify_exit"], real["verify_links_exit"]), (-1, -1, -1))
        self.assertEqual(real["elapsed_seconds"], 220)
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["inputs_unchanged"])
        self.assertTrue(real["client_metadata_unchanged"])
        self.assertFalse(real["accepted_prefix"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        diagnostic = real["source_diagnostic"]
        self.assertEqual(
            (diagnostic["phase"], diagnostic["request_kind"], diagnostic["file_data_id"], diagnostic["ordinal"]),
            ("models", "wmo_group", 248722, 1034),
        )
        self.assertEqual((diagnostic["source_bytes_max"], diagnostic["source_bytes_at_failure"], diagnostic["remaining_source_cap_bytes"]), (1_073_741_824, 1_073_430_591, 311_233))
        self.assertEqual((diagnostic["models"], diagnostic["reads"], diagnostic["request_calls"], diagnostic["dependency_requests"]), (7_432, 16_805, 638_555, 52_704))
        self.assertEqual((diagnostic["raw_bytes"], diagnostic["resident_bytes"], diagnostic["external_bytes"], diagnostic["external_source_bytes"], diagnostic["store_resident_bytes"]), (174_255_498, 520_358_907, 294_467_663, 11_866_356, 306_605_630))
        probe = contract["source_limit_probe"]
        self.assertEqual(probe["report_sha256"], V43_PROBE_REPORT_SHA256)
        self.assertEqual((probe["remaining_cap_bytes"], probe["decoded_source_bytes_observed"], probe["observed_excess_bytes"], probe["hypothetical_source_total_bytes"]), (311_233, 314_478, 3_245, 1_073_745_069))
        self.assertFalse(probe["declared_length_independently_available"])
        follow_up = contract["follow_up_v44"]
        self.assertEqual(follow_up["status"], "design-only; target-not-run")
        self.assertEqual(follow_up["profile"], "SourceExpandedV2")
        self.assertEqual(follow_up["only_limit_changed"], {"name": "max_source_bytes", "from": 1_073_741_824, "to": 2_147_483_648})
        self.assertEqual(
            follow_up["preserved_limits"],
            {
                "max_models": 65_536,
                "max_model_bytes": 536_870_912,
                "max_staged_model_bytes": 536_870_912,
                "max_reads": 1_000_000,
                "max_requests": 16_000_000,
                "max_dependency_requests": 16_000_000,
                "max_raw_bytes": 268_435_456,
            },
        )
        self.assertFalse(follow_up["other_limits_changed"])
        self.assertEqual(follow_up["planned_coverage_layout_version"], 17)
        for name in (
            "source_capacity_sufficiency_proven",
            "model_output_sufficiency_proven",
            "wmo_462695_admission_proven",
            "full_world_coverage",
            "runtime_acceptance",
        ):
            self.assertFalse(follow_up[name], name)
        self.assertTrue(contract["boundary"]["real_target_attempted"])
        self.assertFalse(contract["boundary"]["real_extraction_accepted"])
        boundary = contract["boundary"]
        for name in (
            "real_extraction_accepted",
            "real_512_mib_boundary",
            "source_capacity_sufficiency",
            "model_output_inventory_sufficiency",
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
        self.assertFalse(self.contract["verification"]["full_extractor_complete"])
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
