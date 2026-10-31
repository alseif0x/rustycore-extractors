# SPDX-License-Identifier: GPL-3.0-or-later
"""Synthetic scope and boundary gates for the v50 LiquidObject source product."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v50.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v50/all.tsv"
EXPECTED_DB2 = [
    {"name": "GameObjectDisplayInfo", "file_data_id": 1266277, "count": 1},
    {"name": "Map", "file_data_id": 1349477, "count": 1},
    {"name": "LiquidObject", "file_data_id": 1308058, "count": 1},
]
EXPECTED_IDENTITIES = {
    "source_profile": "linux70450-liquid-object-complete-source-vmap-product-v1",
    "plan_profile": "linux70450-liquid-object-complete-source-vmap-product-v1",
    "plan_coverage_profile": "linux70450-liquid-object-complete-source-coverage-v23",
    "codec_profile": "linux70450-liquid-object-complete-source-v1",
    "domain_profile": "liquid-object-complete-source-v1",
    "verification_profile": "linux70450-liquid-object-complete-source-verification-v1",
}
LIMITS = {
    "source_bytes_max": 2147483648,
    "model_bytes_max": 536870912,
    "placement_bytes_max": 268435456,
    "placement_bytes_max_per_map": 67108864,
    "models_max": 65536,
    "reads_max": 1000000,
    "request_calls_max": 16000000,
    "dependency_requests_max": 16000000,
    "raw_bytes_max": 268435456,
    "assembled_domain_bytes_max": 268435456,
}


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV50Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_and_canonical_plan(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"]), ("forever-product-v50", 50)
        )
        self.assertEqual(
            contract["status"], "synthetic-validation-passed; scoped-real-target-accepted"
        )
        self.assertEqual(contract["synthetic_validation_status"], "passed")
        self.assertTrue(contract["real_target_attempted"])
        self.assertTrue(contract["real_extraction_accepted"])
        self.assertFalse(contract["full_real_target_acceptance"])
        self.assertFalse(contract["full_extractor_complete"])
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["platform"], "linux")
        self.assertTrue(contract["offline"])
        self.assertEqual(
            contract["commands"],
            ["inspect", "extract", "resume", "verify", "verify-links"],
        )
        self.assertEqual(contract["component_mask"], 17)
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 32)
        self.assertTrue(fixture.startswith("forever-product\t50\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertEqual(
            fixture.splitlines()[29:],
            [
                "db2\tGameObjectDisplayInfo\t1266277",
                "db2\tMap\t1349477",
                "db2\tLiquidObject\t1308058",
            ],
        )
        plan = contract["plan"]
        self.assertEqual(plan["required_db2_entries"], EXPECTED_DB2)
        self.assertEqual(plan["line_count"], 32)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["component_mask"], 17)
        self.assertTrue(plan["storage_budgets_downward_only"])

    def test_profiles_limits_and_complete_source_policy(self):
        contract = self.contract
        self.assertEqual(
            {key: contract[key] for key in EXPECTED_IDENTITIES}, EXPECTED_IDENTITIES
        )
        resource = contract["resource_profile"]
        self.assertEqual(
            resource["id"],
            "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1",
        )
        self.assertEqual(resource["basis"], "GameObjectRootAndEncryptedUnavailableExpandedV1")
        self.assertFalse(resource["geometry_limits_changed_vs_v49"])
        self.assertEqual(
            {key: resource["limits"][key] for key in LIMITS}, LIMITS
        )
        self.assertEqual(resource["liquid_object_source_bytes_max"], 16777216)
        self.assertEqual(resource["liquid_object_materialized_rows_max"], 300000)
        source = contract["liquid_object_source"]
        self.assertEqual(source["source_table"], "LiquidObject")
        self.assertEqual(source["file_data_id"], 1308058)
        self.assertEqual(source["source_path"], "dbc/esES/LiquidObject.db2")
        self.assertEqual(source["source_max_bytes"], 16777216)
        self.assertEqual(source["materialized_rows_max"], 300000)
        self.assertIn("not a terrain or liquid geometry output", source["source_scope"])
        self.assertIn("bit-preserving GetUInt16 semantics", source["field_policy"])
        self.assertIn("later assignments overwrite", source["copy_rows"])
        self.assertIn("absent source is inserted with default fields", source["copy_rows"])
        self.assertIn("self-copy inserts at most once", source["copy_rows"])
        self.assertEqual(source["partial_source"], "fatal; no partial LiquidObject declaration is admitted")
        self.assertEqual(source["malformed_or_trailing_source"], "fatal; no package is published")
        self.assertFalse(source["network_or_key_fetch"])
        self.assertFalse(source["consumer_outputs"])
        codec = contract["coverage_codec"]
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 22))
        self.assertEqual(codec["coverage_profile"], EXPECTED_IDENTITIES["plan_coverage_profile"])
        self.assertIn("v49 and v50", codec["cross_profile_rejection"])

    def test_verification_and_scoped_real_acceptance_boundaries(self):
        verification = self.contract["verification"]
        self.assertIn("independently recomputed complete-source facts", verification["liquid_object_db2_authentication"])
        self.assertIn("re-encoded and compared", verification["liquid_object_evidence_authentication"])
        self.assertFalse(verification["full_world_coverage"])
        self.assertFalse(verification["complete_vmap_component"])
        self.assertFalse(verification["runtime_claim"])
        self.assertFalse(verification["full_extractor_complete"])
        self.assertIn("no accepted prefix", verification["real_acceptance_scope"])
        fixtures = self.contract["fixtures"]
        self.assertIn("real output is retained externally only", fixtures["classification"])
        self.assertIn("synthetic PLAN, client and package fixtures", fixtures["classification"])
        self.assertFalse(fixtures["contains_client_data"])
        self.assertTrue(fixtures["real_extraction_evidence"])
        report = fixtures["real_extraction_report"]
        self.assertEqual(
            report["path"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v50-liquid-object-complete-source-20261029T135738Z-acceptance.json",
        )
        self.assertEqual(
            report["sha256"],
            "f368b1e715f86502c85d80675668cce3fbcf680336a752f1259990de13f38bde",
        )
        self.assertFalse(report["contains_client_data"])
        attempt = self.contract["real_acceptance"]
        self.assertEqual(attempt["run_id"], "v50-liquid-object-complete-source-20261029T135738Z")
        self.assertEqual((attempt["status"], attempt["target_status"]), ("passed", "accepted"))
        self.assertTrue(attempt["real_target_attempted"])
        self.assertTrue(attempt["real_extraction_accepted"])
        self.assertFalse(attempt["accepted_prefix"])
        self.assertEqual(attempt["elapsed_seconds"], 1088)
        self.assertEqual(
            attempt["exit_codes"],
            {"inspect": 0, "extract": 0, "probe": 0, "verify": 0, "verify_links": 0},
        )
        self.assertTrue(attempt["all_stage_exit_codes_zero"])
        self.assertFalse(attempt["network_attempted"])
        self.assertFalse(attempt["red"])
        self.assertTrue(attempt["real_gate"])
        self.assertTrue(attempt["client_metadata_unchanged"])
        self.assertTrue(attempt["inputs_unchanged"])
        self.assertTrue(attempt["fresh_extraction_matches_promoted_audit"])
        self.assertEqual(
            attempt["audit_sha256"],
            "9286430073704beef9a7b1c272f6dc79baf3d697b94e5fc94aced00ff822d8c6",
        )
        self.assertEqual(
            attempt["binary_sha256"],
            "295e780066351503f4321334cb17c3550d5979d254058367f8b9d3c6bade77d9",
        )
        package = attempt["package"]
        self.assertEqual(package["published_files"], 12052)
        self.assertEqual(package["published_bytes"], 493068435)
        self.assertEqual(package["active_maps"], 71)
        self.assertEqual(package["present_maps"], 69)
        self.assertEqual(package["root_absent_maps"], 2)
        self.assertEqual(package["direct_rows"], 57)
        self.assertEqual(package["copy_records"], 1213)
        self.assertEqual(package["materialized_rows"], 1270)
        outcomes = attempt["gameobject_outcomes"]
        self.assertEqual(outcomes["missing_root_rows"], 20)
        self.assertEqual(outcomes["encrypted_unavailable_rows"], 1)
        self.assertEqual(outcomes["encrypted_unavailable_fdids"], [7385605])
        self.assertIn("v49 contract", outcomes["full_missing_root_fdids"])
        self.assertTrue(package["liquid_object_source_complete"])
        self.assertFalse(package["liquid_object_partial_declarations"])
        self.assertFalse(attempt["full_world_coverage"])
        self.assertFalse(attempt["complete_vmap_component"])
        self.assertFalse(attempt["liquidobject_geometry"])
        self.assertFalse(attempt["liquidobject_terrain_output"])
        self.assertFalse(attempt["mmaps"])
        self.assertFalse(attempt["runtime_acceptance"])
        self.assertFalse(attempt["publication_authorized"])
        boundaries = self.contract["boundaries"]
        self.assertEqual(boundaries["target_status"], "accepted")
        self.assertTrue(boundaries["real_target_attempted"])
        self.assertEqual(
            boundaries["real_target_attempt"],
            "v50-liquid-object-complete-source-20261029T135738Z",
        )
        self.assertTrue(boundaries["real_extraction_accepted"])
        self.assertEqual(boundaries["synthetic_v50_cli_product_tests"], 3)
        self.assertEqual(boundaries["synthetic_v50_contract_tests"], 3)
        self.assertFalse(boundaries["full_real_target_acceptance"])
        self.assertFalse(boundaries["full_world_coverage"])
        self.assertFalse(boundaries["complete_vmap_component"])
        self.assertFalse(boundaries["liquidobject_geometry"])
        self.assertFalse(boundaries["liquidobject_terrain_output"])
        self.assertFalse(boundaries["runtime_acceptance"])
        self.assertFalse(boundaries["full_extractor_complete"])
        self.assertFalse(boundaries["publication_authorized"])
        closeout = boundaries["software_closeout"]
        self.assertEqual(closeout["status"], "passed")
        self.assertEqual(
            closeout["log_path"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/kimi/checks/closeout-20261029T143338Z-2456466.log",
        )
        self.assertEqual(
            closeout["sha256"],
            "c7b3518c16fefe0f230e12e811c72c996f787f727df57b8eefc6a89d23dcc884",
        )
        self.assertEqual(closeout["final_independent_review"], "passed")
        self.assertTrue(closeout["source_and_snapshot_hashes_verified"])
        self.assertEqual(closeout["exit_code"], 0)
        self.assertEqual(closeout["workspace_suites"], 94)
        self.assertEqual(closeout["tests_passed"], 4774)
        self.assertEqual(closeout["tests_failed"], 0)
        self.assertEqual(closeout["tests_ignored"], 26)
        self.assertEqual(closeout["contracts_passed"], 592)
        self.assertTrue(closeout["separate_from_real_acceptance"])
        self.assertEqual(
            self.contract["synthetic_validation"]["contract_test"]["status"], "passed"
        )


if __name__ == "__main__":
    unittest.main()
