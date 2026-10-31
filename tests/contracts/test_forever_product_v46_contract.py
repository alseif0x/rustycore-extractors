# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 AssemblyExpanded material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v46.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v46/all.tsv"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV46Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_plan_and_fixture_boundary(self):
        contract = self.contract
        self.assertEqual((contract["contract"], contract["version"]), ("forever-product-v46", 46))
        self.assertEqual(contract["status"], "synthetic-validation-passed; scoped-real-target-accepted")
        self.assertEqual(contract["publication_profile"], "Build70450AssemblyExpandedMaterialFilteredWmoGeometryV1")
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
            ),
            (
                "linux70450-assembly-expanded-material-filtered-wmo-v1",
                "linux70450-assembly-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-assembly-expanded-material-filtered-wmo-coverage-v19",
                "linux70450-assembly-expanded-material-filtered-wmo-v1",
                "assembly-expanded-material-filtered-wmo-v1",
                "linux-collision-assembly-expanded-resources-v1",
                "linux70450-available-assembly-expanded-material-filtered-wmo-collision-v1",
                "linux70450-assembly-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_assembly_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
            ),
        )
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 30)
        self.assertTrue(fixture.startswith("forever-product\t46\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertIn(contract["plan_coverage_profile"], fixture)
        plan = contract["plan"]
        self.assertEqual((plan["forever-product"], plan["product_version"], plan["build"]), ("46", 46, 70450))
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertEqual(plan["assembled_domain_bytes_max"], 268_435_456)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])
        self.assertTrue(contract["fixtures"]["used_by_plan_admission"])
        self.assertFalse(contract["fixtures"]["used_as_plan_file_by_cli_test"])

    def test_assembly_domain_expansion_preserves_v45_limits(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["basis"], "PlacementExpandedV1")
        self.assertEqual(resource["only_limit_changed_vs_v45"], "max_assembled_domain_bytes")
        self.assertEqual(resource["max_assembled_domain_bytes"], 268_435_456)
        self.assertEqual(resource["max_staged_placement_bytes"], 268_435_456)
        self.assertEqual(resource["per_map_placement_bytes_max"], 67_108_864)
        self.assertTrue(resource["assembly_domain_limit_is_independent"])
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
        limits = self.contract["limits"]
        self.assertEqual(limits["assembled_domain_bytes_max"], 268_435_456)
        self.assertEqual(limits["placement_bytes_max"], 268_435_456)
        self.assertEqual(limits["per_map_placement_bytes_max"], 67_108_864)

    def test_v19_codec_and_omission_boundaries(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 19))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertIn("v18 field layout", codec["preserved_layout"])
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["domain_profile"], self.contract["domain_profile"])
        self.assertEqual(codec["resource_profile"], self.contract["resource_profile"]["id"])
        self.assertEqual(codec["list_order"], LIST_ORDER)
        self.assertEqual(codec["list_max"], 4096)
        self.assertTrue(codec["pairwise_disjoint"])
        self.assertIn("v45 and v46", codec["cross_profile_rejection"])
        self.assertEqual(self.contract["wmo_omissions"]["list_order"], LIST_ORDER)
        self.assertTrue(self.contract["wmo_omissions"]["pairwise_disjoint"])

    def test_v45_failure_and_v46_scoped_acceptance_keep_distinct_boundaries(self):
        prior = self.contract["prior_v45_attempt"]
        self.assertEqual(prior["result"], "failed")
        self.assertEqual(prior["extract_error"], "product_selected_geometry_limit")
        self.assertEqual(prior["failed_component"], "vmaps")
        self.assertFalse(prior["root_cause_confirmed"])
        self.assertEqual(prior["prior_internal_assembled_domain_limit_bytes"], 134_217_728)
        self.assertIn("only a hypothesis", prior["interpretation"])
        validation = self.contract["validation_plan"]
        self.assertEqual(validation["target_status"], "accepted")
        self.assertEqual(validation["real_target_status"], "accepted")
        self.assertTrue(validation["real_target_attempted"])
        self.assertTrue(validation["real_extraction_accepted"])
        self.assertFalse(validation["real_256_mib_assembled_domain_boundary"])

        acceptance = self.contract["real_acceptance"]
        self.assertEqual(acceptance["run_id"], "v46-assembly-expanded-wmo-20261027T180034Z")
        self.assertEqual(acceptance["status"], "passed")
        self.assertEqual(acceptance["target_status"], "accepted")
        self.assertEqual(
            acceptance["scope"],
            "available_geometry_with_assembly_expanded_material_filtered_wmo_not_full_world_or_runtime",
        )
        self.assertTrue(acceptance["available_geometry_with_assembly_expanded_material_filtered_wmo_accepted"])
        self.assertTrue(acceptance["real_target_attempted"])
        self.assertTrue(acceptance["real_extraction_accepted"])
        self.assertFalse(acceptance["accepted_prefix"])
        self.assertFalse(acceptance["full_extractor_complete"])
        self.assertEqual(acceptance["elapsed_seconds"], 1107)
        self.assertEqual(
            acceptance["exit_codes"],
            {"inspect": 0, "extract": 0, "missing_m2_probe": 0, "verify": 0, "verify_links": 0},
        )
        self.assertFalse(acceptance["network_attempted"])
        self.assertTrue(acceptance["client_metadata_unchanged"])
        self.assertTrue(acceptance["inputs_unchanged"])
        self.assertTrue(acceptance["fresh_extraction_matches_promoted_audit"])
        self.assertEqual(
            acceptance["report"]["sha256"],
            "a7732a62448a321399994de309d723683cda7a21d0d3df696af303fe4380b4a6",
        )
        self.assertEqual(
            [attempt["report_sha256"] for attempt in acceptance["retained_unaccepted_attempts"]],
            [
                "a50a131485257d617bd22b8f543cca0e48caa2439de2d7f7af7308358b43d023",
                "0a948db13a4feaa7415025ecd928ff3550e3713e0055eec1a664b5c141956b0f",
                "faee5cec0e3de6c303d10ab31bd36c4c82d5c02f0d069f66f7f045996fc22b7d",
                "1e191c4837474f917b31ea77cc96b57b99e01d6b8f4b1ed78cc9e9264e25f2fd",
            ],
        )
        self.assertEqual(
            (
                acceptance["audit_sha256"],
                acceptance["source_sha256"],
                acceptance["software_sha256"],
                acceptance["binary_sha256"],
            ),
            (
                "3fcbc9b8bb82f3b08dcc3e5945ba2a3f41b8b6dd3e7afacc6d71311b9fc542fb",
                "a19619ed2e89db61934f01d7ef050441b945f4a87167be184a5f17b4d52b0645",
                "c8f0d893ed59f9b7165ffe7b365e2bc584d8176140a434c529b37128102655a3",
                "a55a03c71510ce83d5de80b744ec36d07b54a4b31bd556f00da036f87e3e42ec",
            ),
        )
        package = acceptance["package"]
        self.assertEqual(
            (package["published_files"], package["published_bytes"]), (11_237, 473_186_807)
        )
        self.assertEqual(
            (package["active_maps"], package["present_maps"], package["root_absent_maps"]),
            (71, 69, 2),
        )
        self.assertEqual(
            (package["placement_records"], package["placement_bytes"], package["placement_model_artifacts"]),
            (1_158_879, 90_392_562, 4_990),
        )
        self.assertEqual(
            (package["declared_direct_rows"], package["direct_rows"], package["materialized_rows"]),
            (81, 73, 73),
        )
        self.assertEqual(
            (
                package["inventory_sha256"],
                package["manifest_sha256"],
                package["coverage_sha256"],
                package["plan_sha256"],
            ),
            (
                "8681af27b2507656b2b66689cdadebb12649be341bb4ed4c2951121e8b478776",
                "08e948f47ebec6022467cc799fa90bac3883905b7a16381c63f2f96e2f7c897e",
                "9f5ec4e7039e4fea1ec6ffa5ace3d927dc71f38c845c5e5dcf51d959b47986ec",
                "0de284f0aec642e1b803177b093b5ae60cdb440cfbcab4bdc2cfe578bf3d5329",
            ),
        )
        self.assertEqual(
            package["omissions"],
            {
                "missing_m2": [192262, 194853, 243074, 243154, 243158, 243166, 243169, 243172, 603672, 970793],
                "missing_wmo": [],
                "empty_wmo": [],
                "unselected_wmo": [],
                "material_filtered_wmo": [462695, 7679500],
            },
        )
        for name, expected in (
            ("inventory_sha256", "8681af27b2507656b2b66689cdadebb12649be341bb4ed4c2951121e8b478776"),
            ("manifest_sha256", "08e948f47ebec6022467cc799fa90bac3883905b7a16381c63f2f96e2f7c897e"),
            ("coverage_sha256", "9f5ec4e7039e4fea1ec6ffa5ace3d927dc71f38c845c5e5dcf51d959b47986ec"),
            ("plan_sha256", "0de284f0aec642e1b803177b093b5ae60cdb440cfbcab4bdc2cfe578bf3d5329"),
        ):
            self.assertEqual(package[name], expected)
        boundary = self.contract["boundary"]
        real_boundary = acceptance["boundary"]
        self.assertEqual(
            real_boundary,
            {name: boundary[name] for name in real_boundary},
        )
        self.assertTrue(boundary["real_target_attempted"])
        self.assertEqual(boundary["real_target_status"], "accepted")
        self.assertTrue(boundary["real_extraction_accepted"])
        for name in (
            "real_256_mib_assembled_domain_boundary",
            "source_capacity_sufficiency",
            "model_output_sufficiency",
            "placement_capacity_sufficiency",
            "assembled_domain_capacity_sufficiency",
            "wmo_462695_admission",
            "full_world_coverage",
            "complete_vmap_component",
            "godi_global_gameobjects",
            "liquidobject",
            "mmaps",
            "runtime_acceptance",
            "rustycore_runtime_acceptance",
            "format_parity",
            "windows",
            "distribution",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(boundary[name], name)
        self.assertFalse(self.contract["full_extractor_complete"])

    def test_software_qualification_and_release_evidence_are_recorded_separately(self):
        evidence = self.contract["software_qualification"]
        self.assertEqual(evidence["status"], "passed")
        self.assertEqual(
            evidence["workspace"],
            {"test_suites": 97, "passed": 4689, "failed": 0, "ignored": 26},
        )
        self.assertEqual(
            self.contract["real_acceptance"]["software_qualification"]["workspace_tests_passed"],
            4690,
        )
        self.assertEqual(self.contract["real_acceptance"]["software_qualification"]["repository_contracts_passed"], 579)
        self.assertEqual(
            evidence["forever_extractor_library"], {"passed": 811, "failed": 0}
        )
        self.assertEqual(evidence["v46_cli"], {"passed": 16, "failed": 0})
        self.assertEqual(evidence["v46_plan"], {"passed": 4, "failed": 0})
        self.assertEqual(evidence["casc_resource"], {"passed": 1, "failed": 0})
        self.assertEqual(evidence["repository_contracts"], {"passed": 579, "failed": 0})
        self.assertEqual(
            self.contract["validation_plan"]["synthetic_test_results"]["repository_contracts"],
            {
                "status": "passed",
                "command": "tools/checks contracts",
                "tests": 579,
            },
        )
        self.assertEqual(
            evidence["checks"],
            {
                "fmt": "passed",
                "clippy": "passed",
                "diff_check": "passed",
                "windows_gnu": "passed",
                "release_build": "passed",
            },
        )
        release = evidence["release"]
        self.assertEqual(
            release["binary_path"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v46-software-20261027T140834Z/forever-extractor",
        )
        self.assertEqual(
            release["sha256"],
            "54fb39d2452dde9fd2637e731fa8df2c794bebb477ae34e1d2c392e4bd1c17dc",
        )
        inspect = release["inspect"]
        self.assertEqual(inspect["status"], "passed")
        self.assertEqual(inspect["contract"], "forever-product-v46")
        self.assertEqual(
            inspect["product_status"], "synthetic-validation-passed; real-target-not-run"
        )
        self.assertEqual(
            inspect["numeric_limits"],
            {
                "source_bytes_max": 2_147_483_648,
                "models_max": 65_536,
                "reads_max": 1_000_000,
                "request_calls_max": 16_000_000,
                "dependency_requests_max": 16_000_000,
                "raw_bytes_max": 268_435_456,
                "model_bytes_max": 536_870_912,
                "placement_bytes_max": 268_435_456,
                "placement_bytes_max_per_map": 67_108_864,
                "assembled_domain_bytes_max": 268_435_456,
            },
        )
        self.assertEqual(
            (
                inspect["resource_profile"],
                inspect["source_profile"],
                inspect["coverage_profile"],
            ),
            (
                self.contract["resource_profile"]["id"],
                self.contract["source_profile"],
                self.contract["plan_coverage_profile"],
            ),
        )
        self.assertFalse(inspect["full_extractor_complete"])
        self.assertEqual(inspect["product_status"], "synthetic-validation-passed; real-target-not-run")
        self.assertTrue(self.contract["validation_plan"]["real_target_attempted"])
        self.assertTrue(self.contract["validation_plan"]["real_extraction_accepted"])
        self.assertFalse(self.contract["boundary"]["real_256_mib_assembled_domain_boundary"])
        self.assertFalse(self.contract["boundary"]["assembled_domain_capacity_sufficiency"])
        self.assertFalse(self.contract["full_extractor_complete"])
        retained = evidence["evidence"]
        self.assertEqual(
            retained["log_directory"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v46-software-20261027T140834Z",
        )
        self.assertEqual(retained["final_contracts_log"], "contracts-final.log")
        self.assertEqual(
            retained["final_contracts_log_sha256"],
            "a856274a96c13fbac9d884c925e2d2a33938e76b539407e93486fad592f7c412",
        )
        self.assertNotIn("contracts.log", retained)
        self.assertEqual(
            retained["log_sha256"],
            {
                "rust": "c6e669a689cd15cab0e3b0d340b6d514e5174a58806a94228bc2c1d8e2299698",
                "build": "d5e2d07619fc36f80df46292d41c97a38b62d805b4cc0be466da60e19b023e0d",
                "fmt": "8f18988f1d190e4b29d80a8268eee7d986cf4dff58e70c4981836f87517963b0",
                "clippy": "7734622024907a6c7f988013f959315daee381b9e82c69edda83d90c7e53d8a1",
                "contracts_final": "a856274a96c13fbac9d884c925e2d2a33938e76b539407e93486fad592f7c412",
                "windows_gnu": "c919f93e53f1301891517fc24c84be831ad84544aa659782da653e6b2f7de065",
                "diff_check": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            },
        )


if __name__ == "__main__":
    unittest.main()
