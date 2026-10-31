# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 intrinsically empty-WMO geometry contract; synthetic v34 only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v34.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v34/all.tsv"
ORDERED_KEYS = [
    "forever-product", "build", "locale", "index", "root", "schema-catalog",
    "decryption", "partial_encrypted_sections", "collision-storage",
    "collision-raw-bytes", "collision-wdt-bytes", "collision-cache-bytes",
    "collision-placement-bytes", "collision-vmo-bytes", "collision-map-bytes",
    "collision-output-bytes", "collision-profile", "collision-domain",
    "collision-coverage", "gameobject-scope", "model-scope", "components",
    "terrain", "pack", "navigation", "big-base-unit", "skip-liquids",
    "maximum-slope", "non-steep-slope",
]
BUDGET_FIELDS = [
    "collision-raw-bytes", "collision-wdt-bytes", "collision-cache-bytes",
    "collision-placement-bytes", "collision-vmo-bytes", "collision-map-bytes",
    "collision-output-bytes",
]
PLAN_PROFILE = "linux70450-available-empty-wmo-vmap-product-v1"
COVERAGE_PROFILE = "linux70450-available-empty-wmo-coverage-v7"
SOURCE_PROFILE = "linux70450-available-empty-wmo-v1"
RESOURCE_PROFILE = "linux-collision-request-heavy-resources-v1"
COVERAGE_PATH = "vmaps/available-empty-wmo-geometry-coverage-v7.bin"


class ProductV34Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_plan_literals_and_exact_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v34", 34, "Build70450AvailableEmptyWmoGeometryV1"),
        )
        self.assertEqual((contract["builds"], contract["platform"], contract["offline"]), ([70450], "linux", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["plan_profile"], PLAN_PROFILE)
        self.assertEqual(contract["plan_coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(contract["domain_profile"], "available-empty-wmo-v1")
        self.assertEqual(contract["codec_profile"], SOURCE_PROFILE)
        self.assertEqual(contract["completion"], "linux70450-available-empty-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-available-empty-wmo-verification-v1")
        self.assertEqual(contract["verification_scope"], "artifact_integrity_available_empty_wmo_and_independently_derived_map_vmap_domain_coverage")

        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(plan["component_mask"], 17)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["collision_profile"], PLAN_PROFILE)
        self.assertEqual(plan["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(plan["domain_closure"], "available-empty-wmo-v1")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual((plan["terrain"], plan["navigation"], plan["offmesh"]), ("none", "none", "absent"))

        fixture = contract["fixtures"]
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertFalse(fixture["certified_real_storage_budgets"])
        self.assertEqual(fixture["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v34/all.tsv")
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual([row[0] for row in rows[:-1]], ORDERED_KEYS)
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        fields = {row[0]: row[1] for row in rows[:-1]}
        self.assertEqual(fields["forever-product"], "34")
        self.assertEqual(fields["collision-profile"], PLAN_PROFILE)
        self.assertEqual(fields["collision-coverage"], COVERAGE_PROFILE)
        self.assertEqual(fields["components"], "db2,vmaps")
        self.assertEqual({fields[name] for name in BUDGET_FIELDS}, {"1048576"})
        self.assertNotIn("requested_map", fields)
        self.assertNotIn("partial_encrypted_tables", fields)

    def test_codec_v7_v33_crossing_and_request_heavy_boundaries(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], COVERAGE_PATH)
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 7))
        self.assertEqual(codec["profile"], SOURCE_PROFILE)
        self.assertEqual(codec["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(codec["resource_profile"], RESOURCE_PROFILE)
        self.assertEqual(codec["resource_profile_serialized_after"], "missing_m2")
        self.assertEqual(codec["missing_wmo_policy"], "run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty")
        self.assertEqual(codec["missing_wmo_policy_serialized_after"], "resource_profile")
        self.assertEqual(codec["empty_wmo_serialized_after"], "missing_wmo")
        self.assertEqual(codec["zero_omissions_still_layout_version"], 7)
        self.assertEqual(codec["missing_wmo_max"], 4096)
        self.assertEqual(codec["empty_wmo_max"], 4096)
        self.assertIn("omitted_usable=false before any omission", codec["empty_wmo_admission"])
        self.assertIn("selection references empty", codec["empty_wmo_admission"])
        self.assertTrue(codec["v1_v6_layouts_preserved"])
        wrappers = codec["strict_wrappers"]
        self.assertEqual(wrappers["v32"]["layout_version"], 5)
        self.assertEqual(wrappers["v33"]["layout_version"], 6)
        self.assertEqual(wrappers["v33"]["contract"], "forever-product-v33")
        self.assertEqual(wrappers["v34"]["layout_version"], 7)
        self.assertIn("empty_wmo", wrappers["shared_mask17"])
        self.assertEqual(self.contract["resource_profile"]["only_changed_capacity"], "max_requests")
        self.assertEqual(self.contract["limits"]["request_calls_max"], 16_000_000)
        self.assertIn("no limits relative to v32 or v33", self.contract["limits"]["request_calls_change"])
        self.assertFalse(self.contract["limits"]["memory_max_raised"])
        self.assertFalse(self.contract["limits"]["dependency_requests_max_raised"])

    def test_separate_wmo_predicates_and_artifact_boundaries(self):
        source = self.contract["source_policy"]
        self.assertIn("locally declared ROOT-absent M2 doodad", source["vacated_wmo"])
        self.assertIn("MODR participated in selection", source["vacated_wmo"])
        self.assertIn("local_missing_m2=0", source["empty_wmo"])
        self.assertIn("omitted_usable=false before any omission", source["empty_wmo"])
        self.assertIn("nonempty selection sets", source["empty_wmo"])
        self.assertIn("disjoint", source["empty_wmo"])
        self.assertIn("collision_no_valid_collision", source["empty_wmo"])
        self.assertIn("FDID 462695", self.cases()["empty-wmo-positive"]["expect"])
        self.assertIn("missing_wmo remains empty", self.cases()["empty-wmo-positive"]["expect"])
        self.assertIn("FDID 970793", self.cases()["vacated-wmo-positive"]["expect"])
        self.assertIn("present doodad", self.cases()["present-parent-and-child-preserved"]["expect"])
        package = self.contract["package"]
        for category in ("vacated_wmos", "empty_wmos"):
            for field in ("vmos", "bindings", "valid_names", "spawns", "placements"):
                self.assertFalse(package[f"{field}_for_{category}"], f"{field}_for_{category}")
        self.assertIn("GODI", self.cases()["poisoned-excluded-sources"]["expect"])
        self.assertIn("LiquidObject", self.cases()["poisoned-excluded-sources"]["expect"])
        self.assertIn("new scratch", self.cases()["fresh-scratch-resume"]["expect"])
        self.assertIn("remaining geometry", self.cases()["empty-wmo-fatal-negatives"]["expect"])
        self.assertNotIn(">4096", json.dumps(self.cases()["empty-wmo-positive"]))

    def test_authentication_real_gate_and_product_claim_boundaries(self):
        verification = self.contract["verification"]
        self.assertEqual(verification["root_absence_authentication"], "producer_reported")
        self.assertEqual(verification["model_absence_authentication"], "producer_reported")
        self.assertEqual(verification["wmo_absence_authentication"], "producer_reported")
        self.assertEqual(verification["missing_wmo_policy"], "run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty")
        self.assertIn("disjoint from missing_m2 and missing_wmo", verification["empty_wmo_canonical"])
        for key in ("root_absence_independently_proven", "model_absence_independently_proven", "wmo_absence_independently_proven"):
            self.assertFalse(verification[key])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        evidence = real["evidence"]
        self.assertEqual(evidence["sha256"], "3a00385706ce85359bd5032fb47853d0a4fed9335436ddf3df1ffd81fdaf019c")
        self.assertEqual(evidence["source_json_sha256"], "adc4a0e68783dab6a70d4af999a96d9c2c44d3850d0536572cb18c4b847115e6")
        self.assertEqual(evidence["software_json_sha256"], "db6b50e96fff18ea06cfa79baa461a889701c631d0b37e2eca7fef0b58ba41eb")
        self.assertEqual((evidence["inspect_exit"], evidence["extract_exit"]), (0, 9))
        self.assertEqual(evidence["failure_code"], "collision_no_valid_collision")
        self.assertEqual(evidence["diagnostic_sha256"], "8bbaa2e588292980ac2894d30e6c3bac446571439f233e051376d2ab43d3f68f")
        self.assertEqual((evidence["wmo_file_data_id"], evidence["group_file_data_id"]), (462695, 463228))
        self.assertEqual(evidence["intrinsic_source"], 0)
        self.assertEqual((evidence["triangles"], evidence["indices"], evidence["vertices"], evidence["batches"]), (368, 1104, 591, 1))
        self.assertEqual(evidence["required_missing_m2"], [970793])
        self.assertEqual(evidence["required_missing_wmo"], [])
        self.assertEqual(evidence["required_empty_wmo"], [462695])
        self.assertFalse(evidence["accepted_prefix"])
        for stage in ("missing_m2_probe", "verify", "verify_links"):
            self.assertEqual(evidence[stage], "not-run")
        previous = self.contract["existing_evidence"]
        self.assertEqual(previous["v33_acceptance_status"], "failed")
        self.assertEqual(previous["v33_report_sha256"], "7ca5b12e7f78899a384617f79d6350e19d320415022f741471c38951bf11e1d4")
        self.assertEqual(previous["v33_diagnostic_sha256"], "9e5c6e05ff039e652dda5cf77f713146375066b6378fdfeffac6d8cb02b7e9b7")
        self.assertTrue(previous["v33_evidence_is_not_v34_acceptance"])
        self.assertFalse(previous["older_acceptance_transferred_to_v34"])
        for field in ("full_world_coverage", "complete_vmap_component", "runtime_acceptance", "distribution", "full_extractor_complete"):
            self.assertFalse(self.contract["boundary"][field], field)
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
