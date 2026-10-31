# SPDX-License-Identifier: GPL-3.0-or-later
"""Synthetic admission and scope gates for the v47 GameObjectExpanded product."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v47.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v47/all.tsv"
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


class ProductV47Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_and_canonical_plan(self):
        contract = self.contract
        self.assertEqual((contract["contract"], contract["version"]), ("forever-product-v47", 47))
        self.assertEqual(
            contract["status"],
            "synthetic-validation-passed; real-target-attempt-failed-unaccepted",
        )
        self.assertEqual(contract["synthetic_validation_status"], "passed")
        self.assertTrue(contract["real_target_attempted"])
        self.assertFalse(contract["real_extraction_accepted"])
        self.assertFalse(contract["full_real_target_acceptance"])
        attempt = contract["real_target_attempt"]
        self.assertEqual(attempt["run_id"], "v47-gameobject-expanded-wmo-20261028T032635Z")
        self.assertEqual(attempt["report_sha256"], "d8d1f10bc62d38363d4e277c83c06b4de037d3d08111ff22b1fe50c093b6fc95")
        self.assertEqual(attempt["extract_exit"], 9)
        self.assertEqual(attempt["failure"], "file_unavailable")
        self.assertEqual(attempt["component"], "vmaps")
        self.assertEqual(attempt["source_diagnostic"], {"phase": "db2", "file_data_id": 2198569})
        self.assertFalse(attempt["accepted_prefix"])
        self.assertFalse(contract["full_extractor_complete"])
        self.assertEqual(
            contract["synthetic_validation"]["contract_tests"],
            {
                "status": "passed",
                "command": "python3 -m unittest tests.contracts.test_forever_product_v47_contract -v",
                "tests": 3,
            },
        )
        qualification = contract["software_qualification"]
        self.assertEqual(qualification["status"], "passed")
        self.assertEqual(
            qualification["workspace"],
            {"suites": 91, "passed": 4712, "failed": 0, "ignored": 26},
        )
        self.assertEqual(
            qualification["test_gates"],
            {
                "forever_extractor_library": {"passed": 824, "total": 824},
                "v47_cli": {"passed": 7, "total": 7},
                "plan_admission": {"passed": 4, "total": 4},
                "casc_available": {"passed": 43, "total": 43},
                "compatible_vmaps_replay": {"passed": 81, "total": 81},
                "repository_contracts": {"passed": 582, "total": 582},
            },
        )
        self.assertEqual(
            qualification["quality_gates"],
            {
                "formatting": "passed",
                "clippy": "passed",
                "diff_check": "passed",
                "windows_gnu": "passed",
                "release_build": "passed",
            },
        )
        self.assertEqual(
            qualification["release_executable"],
            {
                "path": "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v47-qualified-software-20261028T025339Z/forever-extractor",
                "sha256": "fe8b4245ae05e647a36933f0f0b4cf2ff2334a36e0971441c186d5faaa41d415",
            },
        )
        self.assertEqual(
            qualification["release_inspect"],
            {
                "status": "passed",
                "numeric_limits_checked": 10,
                "identity_fields_checked": ["resource", "source", "coverage", "scopes"],
                "reported_full_extractor_complete": False,
            },
        )
        self.assertIn("pre-attempt qualification snapshot", qualification["evidence_basis"])
        self.assertEqual(
            qualification["log_sha256"],
            {
                "rust": "892e7083fb86d65e0cb6210d065df966634633a1703468b32b1b1e89f7ad63fd",
                "build": "cf982c7d6aa5994e8a9bec2c026e12bfca6d65dc5ab3a8efd66608e5483c03ab",
                "fmt": "ae150d010a40f281c9ad0ed012c6bf6c0fa9dad1d10179a818276a4453f579b8",
                "clippy": "2e4e99e48434c98fea288a837ab386beb649ab0204b42bffe8502b72341f60ad",
                "contracts": "0fad36361b34e52b1bf975b7747fb39b9b3e9bb704ffe4f7af5382c0b2f343b2",
                "windows": "e71b864ba1bf9436d828f089e3c27d971dc6a7db554807d3de5971ba5ff1eeab",
                "diff": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            },
        )
        self.assertFalse(qualification["qualification_changed_product_identities"])
        self.assertFalse(qualification["qualification_changed_product_limits"])
        self.assertEqual(qualification["target_status"], "target-not-run")
        self.assertFalse(qualification["real_target_attempted"])
        self.assertFalse(qualification["real_extraction_accepted"])
        self.assertFalse(qualification["full_real_target_acceptance"])
        self.assertIn("before the later real-target attempt", qualification["snapshot_timing"])
        self.assertFalse(qualification["full_extractor_complete"])
        self.assertEqual(
            contract["synthetic_validation"]["plan_admission_tests"]["status"], "passed"
        )
        self.assertEqual(
            contract["synthetic_validation"]["plan_admission_tests"]["tests"], 4
        )
        self.assertEqual(
            contract["synthetic_validation"]["cli_build_and_execution"]["status"], "passed"
        )
        self.assertEqual(
            contract["synthetic_validation"]["cli_build_and_execution"]["tests"], 7
        )
        self.assertEqual(contract["implemented_commands"], contract["commands"])
        self.assertIsNone(contract["unavailable_command_error"])
        self.assertFalse(contract["full_real_target_acceptance"])
        self.assertFalse(contract["full_extractor_complete"])
        self.assertEqual(
            contract["publication_profile"],
            "Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1",
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["implemented_commands"], contract["commands"])
        self.assertEqual(
            contract["implemented_command_scope"],
            "synthetic and admitted offline build-70450 profile; not complete real-target support",
        )
        self.assertIsNone(contract["unavailable_command_error"])
        self.assertEqual(contract["component_mask"], 17)
        self.assertEqual(
            contract["limits"],
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
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 31)
        self.assertTrue(fixture.startswith("forever-product\t47\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertEqual(
            fixture.splitlines()[29:],
            ["db2\tGameObjectDisplayInfo\t1266277", "db2\tMap\t1349477"],
        )
        plan = contract["plan"]
        self.assertEqual(plan["required_db2_entries"], EXPECTED_DB2)
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertEqual(plan["line_count"], 31)
        self.assertEqual(contract["plan_coverage_profile"], fixture.splitlines()[18].split("\t", 1)[1])
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])

    def test_v47_identities_and_scope_are_separate_from_v46(self):
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
                contract["verification_scope"],
            ),
            (
                "linux70450-gameobject-expanded-material-filtered-wmo-v1",
                "linux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20",
                "linux70450-gameobject-expanded-material-filtered-wmo-v1",
                "gameobject-expanded-material-filtered-wmo-v1",
                "linux-collision-gameobject-expanded-resources-v1",
                "linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1",
                "linux70450-gameobject-expanded-material-filtered-wmo-verification-v1",
                "artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage",
            ),
        )
        godi = contract["gameobject_geometry"]
        self.assertEqual(godi["source_table"], "GameObjectDisplayInfo")
        self.assertEqual(godi["scope"], "available-fdid-direct-v1")
        self.assertEqual(godi["model_scope"], "placement-and-gameobject-v1")
        self.assertEqual(godi["tree_path"], "vmaps/GameObjectModels.dtree")
        self.assertEqual(godi["tree_encoding"], "VMAP_4.E")
        self.assertFalse(godi["temporary_tree_source_published"])
        self.assertTrue(godi["tree_reconstruction_required_for_temp_sha256"])
        self.assertFalse(godi["copy_rows_reexecuted"])
        resource = contract["resource_profile"]
        self.assertEqual(resource["basis"], "AssemblyExpandedV1")
        self.assertFalse(resource["resource_limits_changed_vs_v46"])
        self.assertFalse(resource["disk_world_profile_implicit"])
        self.assertEqual(
            contract["coverage_codec"]["layout_version"], 20
        )
        self.assertEqual(contract["coverage_codec"]["placement_omission_list_max"], 4096)
        self.assertEqual(
            contract["coverage_codec"]["placement_omission_list_order"],
            ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"],
        )

    def test_authentication_and_exclusions_are_not_overclaimed(self):
        verification = self.contract["verification"]
        self.assertEqual(
            verification["map_and_godi_authentication"],
            "physical_rows_order_display_ids_and_fdids_independently_derived_from_sealed_db2",
        )
        self.assertEqual(
            verification["gameobject_disposition_authentication"],
            "producer_reported_consistency_checked_against_sealed_db2_physical_rows",
        )
        self.assertEqual(verification["map_domain_authentication"], "independently_derived")
        self.assertEqual(verification["placement_and_assembly_authentication"], "independently_derived")
        self.assertEqual(verification["bounds_and_transformed_raw_geometry_authentication"], "producer_reported")
        self.assertFalse(verification["full_world_coverage"])
        self.assertFalse(verification["complete_vmap_component"])
        self.assertFalse(verification["runtime_claim"])
        self.assertFalse(verification["full_extractor_complete"])
        boundaries = self.contract["boundaries"]
        self.assertEqual(boundaries["target_status"], "attempted-failed-unaccepted")
        self.assertTrue(boundaries["real_target_attempted"])
        self.assertEqual(
            boundaries["real_target_attempt"],
            {
                "run_id": "v47-gameobject-expanded-wmo-20261028T032635Z",
                "report_sha256": "d8d1f10bc62d38363d4e277c83c06b4de037d3d08111ff22b1fe50c093b6fc95",
                "extract_exit": 9,
                "failure": "file_unavailable",
                "component": "vmaps",
                "source_diagnostic": {"phase": "db2", "file_data_id": 2198569},
                "accepted_prefix": False,
            },
        )
        self.assertFalse(boundaries["real_extraction_accepted"])
        self.assertEqual(boundaries["synthetic_validation_status"], "passed")
        self.assertEqual(boundaries["synthetic_contract_tests"], 3)
        self.assertTrue(boundaries["rust_plan_admission_tests_passed"])
        self.assertTrue(boundaries["cli_build_and_execution_verified"])
        self.assertEqual(boundaries["synthetic_v47_cli_tests"], 7)
        self.assertEqual(boundaries["synthetic_extractor_library_tests"], 824)
        self.assertEqual(boundaries["synthetic_v46_cli_regression_tests"], 16)
        self.assertFalse(boundaries["real_target_acceptance"])
        self.assertFalse(boundaries["full_extractor_complete"])
        self.assertFalse(self.contract["full_real_target_acceptance"])
        for key in (
            "full_world_coverage",
            "complete_vmap_component",
            "liquidobject",
            "mmaps",
            "runtime_acceptance",
            "windows",
            "distribution",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(boundaries[key], key)
        self.assertFalse(boundaries["client_bytes_in_reports_or_repository"])


if __name__ == "__main__":
    unittest.main()
