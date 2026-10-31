# SPDX-License-Identifier: GPL-3.0-or-later
"""Synthetic scope and boundary gates for the v49 encrypted-unavailable product."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v49.json"
PLAN_FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v49/all.tsv"
EXPECTED_DB2 = [
    {"name": "GameObjectDisplayInfo", "file_data_id": 1266277, "count": 1},
    {"name": "Map", "file_data_id": 1349477, "count": 1},
]
EXPECTED_PROFILES = {
    "source_profile": "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1",
    "plan_profile": "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-vmap-product-v1",
    "plan_coverage_profile": "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22",
    "codec_profile": "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-v1",
    "domain_profile": "gameobject-root-encrypted-unavailable-material-filtered-wmo-v1",
    "completion": "linux70450-available-gameobject-root-encrypted-unavailable-material-filtered-wmo-collision-v1",
    "verification_profile": "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-verification-v1",
}


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV49Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_identity_and_canonical_plan(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"]), ("forever-product-v49", 49)
        )
        self.assertEqual(
            contract["status"], "synthetic-validation-passed; scoped-real-target-accepted"
        )
        self.assertTrue(contract["real_target_attempted"])
        self.assertTrue(contract["real_extraction_accepted"])
        self.assertEqual(
            contract["publication_profile"],
            "Build70450GameObjectRootAndEncryptedUnavailableMaterialFilteredWmoGeometryV1",
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["component_mask"], 17)
        self.assertTrue(PLAN_FIXTURE.is_file())
        fixture = PLAN_FIXTURE.read_text()
        self.assertEqual(len(fixture.splitlines()), 31)
        self.assertTrue(fixture.startswith("forever-product\t49\n"))
        self.assertTrue(fixture.endswith("\n"))
        self.assertEqual(
            fixture.splitlines()[29:],
            ["db2\tGameObjectDisplayInfo\t1266277", "db2\tMap\t1349477"],
        )
        plan = contract["plan"]
        self.assertEqual(plan["required_db2_entries"], EXPECTED_DB2)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(
            plan["gameobject_scope"],
            "available-fdid-direct-root-absent-or-encrypted-unavailable-v1",
        )
        self.assertEqual(
            plan["model_scope"],
            "placement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1",
        )
        self.assertEqual(plan["line_count"], 31)

    def test_profiles_codec_and_limits_are_explicit(self):
        self.assertEqual(
            {key: self.contract[key] for key in EXPECTED_PROFILES}, EXPECTED_PROFILES
        )
        resource = self.contract["resource_profile"]
        self.assertEqual(
            resource["id"],
            "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1",
        )
        self.assertEqual(resource["basis"], "GameObjectRootAndEncryptedUnavailableExpandedV1")
        self.assertFalse(resource["resource_limits_changed_vs_v48"])
        self.assertEqual(resource["maximum_direct_nonzero_godi_candidate_rows"], 4096)
        self.assertEqual(resource["maximum_missing_root_unique_fdids"], 4096)
        self.assertEqual(resource["maximum_encrypted_unavailable_unique_fdids"], 4096)
        outcomes = self.contract["gameobject_outcomes"]
        self.assertTrue(outcomes["outcomes_are_disjoint"])
        self.assertTrue(outcomes["encrypted_unavailable_is_not_root_absence"])
        self.assertEqual(outcomes["root_absence_and_encrypted_unavailable_authentication"], "producer_reported")
        self.assertEqual(outcomes["root_present_without_body_for_non_encrypted_reason"], "fatal; not declared as encrypted-unavailable")
        self.assertEqual(
            outcomes["encrypted_framing_validation"],
            "full selected EKey (descriptor table for framed BLTE; complete object for headerless BLTE), exact extents and frame MD5s, encrypted outer wrapper, and decodable observable N/Z frames; encrypted plaintext and complete CKey are not authenticated",
        )
        codec = self.contract["coverage_codec"]
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 22))
        self.assertEqual(codec["missing_root_list_max"], 4096)
        self.assertEqual(codec["encrypted_unavailable_list_max"], 4096)
        self.assertTrue(codec["lists_pairwise_disjoint"])
        self.assertFalse(codec["row_count_must_match_unique_list_count"])
        self.assertIn("v47 and v48", codec["cross_profile_rejection"])

    def test_verification_boundary_and_scoped_real_acceptance(self):
        verification = self.contract["verification"]
        self.assertEqual(
            verification["map_and_godi_authentication"],
            "physical_rows_order_display_ids_and_fdids_independently_derived_from_sealed_db2",
        )
        self.assertEqual(
            verification["raw_root_and_encrypted_unavailable_authentication"],
            "producer_reported; not independently re-derived by offline package verification",
        )
        self.assertTrue(verification["rejects_crossed_missing_root_and_encrypted_unavailable_lists"])
        self.assertTrue(verification["rejects_missing_or_encrypted_models_in_artifacts_and_placements"])
        self.assertFalse(verification["full_world_coverage"])
        self.assertFalse(verification["runtime_claim"])
        self.assertIn("no accepted prefix", verification["real_acceptance_scope"])
        boundaries = self.contract["boundaries"]
        self.assertEqual(boundaries["target_status"], "accepted")
        self.assertTrue(boundaries["real_target_attempted"])
        self.assertEqual(
            boundaries["real_target_attempt"],
            "v49-gameobject-root-encrypted-wmo-20261029T033410Z",
        )
        self.assertEqual(boundaries["synthetic_v49_cli_tests"], 10)
        framing = self.contract["synthetic_validation"]["framing_and_startup_regressions"]
        self.assertEqual(framing["tests"], 1891)
        self.assertIn("1892 forever-casc tests", framing["later_closeout_observation"])
        self.assertTrue(self.contract["real_extraction_accepted"])
        attempt = self.contract["real_acceptance"]
        self.assertEqual(attempt["report"]["sha256"], "04e63482a957c5361c3fa966d4b4efa3d296ed53b4d490bf48b9f9ea30449045")
        self.assertEqual(attempt["audit_sha256"], "c3c9e2a81fe5048e369f5c66726cf27b73f46a8a59a1d27e61f16194844bb4a2")
        self.assertEqual(attempt["source_snapshot_sha256"], "40a9fa7805986a8e49553bf3b74826a83529b56e8ffc6e790e6a3729f2c6591f")
        self.assertEqual(attempt["exit_codes"], {"inspect": 0, "extract": 0, "missing_m2_probe": 0, "verify": 0, "verify_links": 0})
        self.assertEqual(attempt["package"]["published_files"], 12050)
        self.assertEqual(attempt["package"]["published_bytes"], 493057703)
        self.assertEqual(attempt["package"]["db2_files"], 2)
        self.assertEqual(attempt["package"]["vmap_files"], 12048)
        package = attempt["package"]
        self.assertEqual(package["materialized_rows"], 73)
        self.assertEqual(package["gameobject_partial_frames"], 2)
        self.assertEqual(package["map_available_bytes"], 10006)
        self.assertFalse(package["map_full_ckey_verified"])
        self.assertEqual(package["placement_records"], 1158879)
        self.assertEqual(package["placement_model_artifacts"], 5801)
        self.assertEqual(package["encrypted_unavailable_fdids"], [7385605])
        self.assertFalse(attempt["accepted_prefix"])
        self.assertFalse(attempt["full_world_coverage"])
        self.assertFalse(attempt["complete_vmap_component"])
        self.assertFalse(attempt["runtime_acceptance"])
        self.assertFalse(attempt["publication_authorized"])
        self.assertFalse(self.contract["full_real_target_acceptance"])
        self.assertFalse(self.contract["full_extractor_complete"])
        self.assertFalse(self.contract["fixtures"]["contains_client_data"])
        self.assertTrue(self.contract["fixtures"]["real_extraction_evidence"])
        self.assertFalse(self.contract["fixtures"]["real_extraction_report"]["contains_client_data"])
        self.assertEqual(self.contract["synthetic_validation"]["plan_admission_tests"]["tests"], 5)
        self.assertEqual(
            self.contract["synthetic_validation"]["forever_extractor_all_targets"]["tests"],
            1310,
        )
        self.assertEqual(
            self.contract["synthetic_validation"]["forever_extractor_all_targets"]["targets"],
            49,
        )


if __name__ == "__main__":
    unittest.main()
