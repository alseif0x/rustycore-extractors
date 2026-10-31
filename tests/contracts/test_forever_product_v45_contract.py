# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 PlacementExpanded material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v45.json"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV45Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_and_no_new_plan_fixture(self):
        contract = self.contract
        self.assertEqual((contract["contract"], contract["version"]), ("forever-product-v45", 45))
        self.assertEqual(
            contract["status"],
            "synthetic-cli-integration-passed; real-attempt-failed; acceptance-not-achieved",
        )
        self.assertEqual(contract["publication_profile"], "Build70450PlacementExpandedMaterialFilteredWmoGeometryV1")
        self.assertEqual((contract["builds"], contract["platform"], contract["offline"]), ([70450], "linux", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
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
                contract["verification_scope"],
                contract["coverage_codec"]["path"],
            ),
            (
                "linux70450-placement-expanded-material-filtered-wmo-v1",
                "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-placement-expanded-material-filtered-wmo-coverage-v18",
                "linux70450-placement-expanded-material-filtered-wmo-v1",
                "placement-expanded-material-filtered-wmo-v1",
                "linux-collision-placement-expanded-resources-v1",
                "linux70450-available-placement-expanded-material-filtered-wmo-collision-v1",
                "linux70450-placement-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_placement_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
                "vmaps/available-placement-expanded-material-filtered-wmo-geometry-coverage-v18.bin",
            ),
        )
        self.assertEqual(contract["source_profile"], "linux70450-placement-expanded-material-filtered-wmo-v1")
        self.assertEqual(contract["component_mask"], 17)
        fixture = contract["fixtures"]
        self.assertEqual(fixture["source_plan"], "tests/contracts/fixtures/forever-product-v45/all.tsv")
        self.assertEqual(fixture["cli_source_plan"], "tests/contracts/fixtures/forever-product-v44/all.tsv")
        self.assertTrue((ROOT / fixture["source_plan"]).is_file())
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertTrue(fixture["used_by_plan_admission"])
        self.assertFalse(fixture["used_as_plan_file_by_cli_test"])
        self.assertIn("PLAN admission", fixture["classification"])
        self.assertEqual(contract["plan"]["ordered_keys"][0:2], ["forever-product", "build"])
        self.assertEqual((contract["plan"]["forever-product"], contract["plan"]["product_version"]), ("45", 45))
        self.assertEqual(contract["plan"]["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual((contract["plan"]["components"], contract["plan"]["component_mask"]), (["db2", "vmaps"], 17))
        self.assertTrue(contract["plan"]["storage_budgets_downward_only"])
        self.assertTrue(contract["plan"]["synthetic_storage_budgets_are_reduced"])

    def test_placement_expansion_and_preserved_source_expanded_limits(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["basis"], "SourceExpandedV2")
        self.assertEqual(resource["only_limit_changed_vs_v44"], "max_staged_placement_bytes")
        self.assertEqual(resource["max_staged_placement_bytes"], 268_435_456)
        self.assertEqual(resource["per_map_placement_bytes_max"], 67_108_864)
        self.assertTrue(resource["placement_staging_limit_matches_plan_max"])
        self.assertFalse(resource["disk_world_profile_implicit"])
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
        self.assertTrue(resource["all_model_identity_routes_coherent"])

        limits = self.contract["limits"]
        self.assertEqual(
            (limits["source_bytes_max"], limits["models_max"], limits["reads_max"], limits["placement_bytes_max"]),
            (2_147_483_648, 65_536, 1_000_000, 268_435_456),
        )
        self.assertEqual(
            (limits["plan_collision_placement_bytes_max"], limits["per_map_placement_bytes_max"]),
            (268_435_456, 67_108_864),
        )
        self.assertEqual(
            (limits["request_calls_max"], limits["dependency_requests_max"], limits["raw_bytes_max"]),
            (16_000_000, 16_000_000, 268_435_456),
        )
        self.assertEqual(
            (limits["plan_collision_vmo_bytes_max"], limits["maximum_staged_model_bytes"], limits["model_output_bytes_max"]),
            (536_870_912, 536_870_912, 536_870_912),
        )
        self.assertEqual(
            (
                self.contract["validation_plan"]["source_bytes_max_inspect_value"],
                self.contract["validation_plan"]["models_max_inspect_value"],
                self.contract["validation_plan"]["reads_max_inspect_value"],
                self.contract["validation_plan"]["request_calls_max_inspect_value"],
                self.contract["validation_plan"]["dependency_requests_max_inspect_value"],
                self.contract["validation_plan"]["raw_bytes_max_inspect_value"],
                self.contract["validation_plan"]["model_bytes_max_inspect_value"],
                self.contract["validation_plan"]["placement_bytes_max_inspect_value"],
                self.contract["validation_plan"]["placement_bytes_max_per_map_inspect_value"],
            ),
            (
                2_147_483_648,
                65_536,
                1_000_000,
                16_000_000,
                16_000_000,
                268_435_456,
                536_870_912,
                268_435_456,
                67_108_864,
            ),
        )

    def test_v18_codec_and_omission_boundaries(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 18))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v17 field layout", codec["preserved_layout"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["domain_profile"], self.contract["domain_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        self.assertIn("all ten pairwise overlaps", codec["cross_profile_rejection"])
        self.assertEqual(self.contract["wmo_omissions"]["list_order"], LIST_ORDER)
        self.assertTrue(self.contract["wmo_omissions"]["pairwise_disjoint"])

    def test_placement_counts_are_independent_and_diagnostics_optional(self):
        diagnostics = self.contract["placement_diagnostics"]
        self.assertEqual(diagnostics["expected_names_if_emitted"], ["total_placements", "per_map_placements", "placement_domain"])
        self.assertIn("each field may be absent", diagnostics["check_policy"])
        self.assertIn("exact type, structure", diagnostics["check_policy"])
        self.assertEqual(
            diagnostics["expected_shapes_if_emitted"],
            {
                "total_placements": "nonnegative integer equal to the independently counted placement records in the synthetic package",
                "per_map_placements": "object with every active map ID as a decimal string key and its exact placement-record count, including zero",
                "placement_domain": "ascending array of active map IDs independently determined from the synthetic Map fixture",
            },
        )
        self.assertEqual(diagnostics["synthetic_expected_domain"], [1, 2, 4, 5, 6])
        self.assertTrue(diagnostics["counts_are_independent"])
        for name in ("infer_total_from_per_map", "infer_per_map_from_total", "infer_domain_from_counts"):
            self.assertFalse(diagnostics[name], name)

    def test_cli_scope_and_acceptance_boundaries_remain_unclaimed(self):
        validation = self.contract["validation_plan"]
        cli = validation["synthetic_test_results"]["cli"]
        extractor = validation["synthetic_test_results"]["extractor_unit"]
        self.assertEqual((cli["status"], cli["passed"], cli["failed"]), ("passed", 16, 0))
        self.assertEqual((cli["binary_tests"], cli["harness_tests"]), (16, 16))
        self.assertEqual((extractor["status"], extractor["passed"], extractor["failed"]), ("passed", 803, 0))
        contracts = validation["repository_contracts"]
        self.assertEqual((contracts["status"], contracts["passed"], contracts["failed"]), ("passed", 574, 0))
        self.assertEqual(contracts["command"], "tools/checks contracts")
        self.assertEqual(validation["target_status"], "target-not-run")
        self.assertEqual(validation["real_target_status"], "target-not-run")
        self.assertTrue(validation["real_target_attempted"])
        self.assertFalse(validation["real_extraction_accepted"])
        self.assertTrue(self.contract["boundary"]["real_target_attempted"])
        self.assertEqual(self.contract["boundary"]["real_target_status"], "target-not-run")
        self.assertFalse(self.contract["boundary"]["real_extraction_accepted"])
        self.assertFalse(self.contract["boundary"]["full_extractor_complete"])
        attempt = validation["real_attempt"]
        self.assertEqual(attempt["run_id"], "v45-placement-expanded-wmo-20261027T120702Z")
        self.assertEqual(
            attempt["report_sha256"],
            "482bca44af93d72cba8ea3b7a8f28b048fe63592296563070e98469b907c4f08",
        )
        self.assertEqual(attempt["elapsed_seconds"], 441)
        self.assertEqual(
            attempt["report_path"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v45-placement-expanded-wmo-20261027T120702Z-acceptance.json",
        )
        self.assertEqual(
            (attempt["software_sha256"], attempt["source_sha256"], attempt["binary_sha256"]),
            (
                "1a28e98bdf582f1f3bab5a18d16390e4ae5c5ab50e9a1057d05bf5541afe63d7",
                "8e126c04ec1716e9fd15d0236416fda00d168173c18b4cb2e66085e718b74c68",
                "aec16378970121c8d521cf4e58671e9bcd4cf4fc0ae9b6136401b02f2ab9f690",
            ),
        )
        self.assertEqual(attempt["result"], "failed")
        self.assertEqual(attempt["software_qualification"]["fmt"], "passed")
        self.assertEqual(attempt["software_qualification"]["clippy"], "passed")
        self.assertEqual(attempt["software_qualification"]["diff_check"], "passed")
        self.assertEqual(attempt["software_qualification"]["release"], "passed")
        self.assertEqual(attempt["software_qualification"]["windows_gnu_check"], "passed")
        self.assertEqual(
            (attempt["inspect_exit"], attempt["extract_exit"], attempt["extract_error"], attempt["failed_component"]),
            (0, 9, "product_selected_geometry_limit", "vmaps"),
        )
        self.assertEqual((attempt["probe_exit"], attempt["verify_exit"], attempt["verify_links_exit"]), (-1, -1, -1))
        self.assertFalse(attempt["network_attempted"])
        self.assertTrue(attempt["client_metadata_unchanged"])
        self.assertTrue(attempt["inputs_unchanged"])
        self.assertFalse(attempt["accepted_prefix"])
        self.assertIsNone(attempt["audit_sha256"])
        self.assertEqual(attempt["partial_package_bytes"], 125_548_426)
        self.assertTrue(attempt["geometry_published"])
        self.assertFalse(attempt["root_cause_confirmed"])
        assembly = attempt["assembly_limit_followup"]
        self.assertEqual(assembly["symbol"], "MAX_ASSEMBLED_DOMAIN_BYTES")
        self.assertEqual((assembly["limit_bytes"], assembly["plan_map_limit_bytes"]), (134_217_728, 268_435_456))
        self.assertIn("candidate", assembly["interpretation"])
        self.assertIn("not a confirmed cause", assembly["interpretation"])
        self.assertEqual(
            (
                attempt["software_qualification"]["workspace_test_suites"],
                attempt["software_qualification"]["workspace_tests_passed"],
                attempt["software_qualification"]["workspace_tests_failed"],
                attempt["software_qualification"]["workspace_tests_ignored"],
                attempt["software_qualification"]["contracts_passed"],
            ),
            (96, 4663, 0, 26, 574),
        )
        self.assertEqual(
            cli["command"],
            "cargo test --offline --locked -p forever-extractor --test v45_placement_expanded_material_filtered_wmo_geometry_cli",
        )
        self.assertEqual(cli["failures"], [])
        self.assertEqual(
            set(validation["v45_cli_test_cases"]),
            {"lifecycle-resume-determinism", "v44-v45-cross-version", "resealed-version-profile-attacks", "ten-pairwise-omission-overlaps", "five-omission-list-boundaries", "reduced-plan-resource-metadata"},
        )
        self.assertIn("no real 256 MiB placement-boundary test", validation["synthetic_test_scope"])
        self.assertFalse(self.contract["verification"]["full_world_coverage"])
        self.assertFalse(self.contract["verification"]["runtime_claim"])
        self.assertFalse(self.contract["verification"]["full_extractor_complete"])
        boundary = self.contract["boundary"]
        self.assertTrue(boundary["real_target_attempted"])
        self.assertEqual(boundary["real_target_status"], "target-not-run")
        self.assertFalse(boundary["real_extraction_accepted"])
        for name in (
            "real_extraction_accepted",
            "real_256_mib_placement_boundary",
            "source_capacity_sufficiency",
            "model_output_sufficiency",
            "placement_capacity_sufficiency",
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
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
