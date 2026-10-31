# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 vacated-WMO geometry contract; synthetic v33 only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v33.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v33/all.tsv"
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
PLAN_PROFILE = "linux70450-available-vacated-wmo-vmap-product-v1"
COVERAGE_PROFILE = "linux70450-available-vacated-wmo-coverage-v6"
SOURCE_PROFILE = "linux70450-available-vacated-wmo-v1"
RESOURCE_PROFILE = "linux-collision-request-heavy-resources-v1"
COVERAGE_PATH = "vmaps/available-vacated-wmo-geometry-coverage-v6.bin"


class ProductV33Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_plan_literals_and_exact_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v33", 33, "Build70450AvailableVacatedWmoGeometryV1"),
        )
        self.assertEqual((contract["builds"], contract["platform"], contract["offline"]), ([70450], "linux", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["plan_profile"], PLAN_PROFILE)
        self.assertEqual(contract["plan_coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(contract["domain_profile"], "available-vacated-wmo-v1")
        self.assertEqual(contract["codec_profile"], SOURCE_PROFILE)
        self.assertEqual(contract["completion"], "linux70450-available-vacated-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-available-vacated-wmo-verification-v1")
        self.assertEqual(contract["verification_scope"], "artifact_integrity_available_vacated_wmo_and_independently_derived_map_vmap_domain_coverage")

        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(plan["component_mask"], 17)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["collision_profile"], PLAN_PROFILE)
        self.assertEqual(plan["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(plan["domain_closure"], "available-vacated-wmo-v1")
        self.assertEqual(plan["partial_encrypted_sections"], "declared-skip; mandatory in the PLAN only, immediately between decryption and collision-storage")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual((plan["terrain"], plan["navigation"], plan["offmesh"]), ("none", "none", "absent"))

        fixture = contract["fixtures"]
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertFalse(fixture["certified_real_storage_budgets"])
        self.assertEqual(fixture["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v33/all.tsv")
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual([row[0] for row in rows[:-1]], ORDERED_KEYS)
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        fields = {row[0]: row[1] for row in rows[:-1]}
        self.assertEqual(fields["forever-product"], "33")
        self.assertEqual(fields["collision-profile"], PLAN_PROFILE)
        self.assertEqual(fields["collision-coverage"], COVERAGE_PROFILE)
        self.assertEqual(fields["components"], "db2,vmaps")
        self.assertEqual({fields[name] for name in BUDGET_FIELDS}, {"1048576"})
        self.assertNotIn("requested_map", fields)
        self.assertNotIn("partial_encrypted_tables", fields)

    def test_codec_v6_v32_crossing_and_request_heavy_boundaries(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], COVERAGE_PATH)
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 6))
        self.assertEqual(codec["profile"], SOURCE_PROFILE)
        self.assertEqual(codec["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(codec["resource_profile"], RESOURCE_PROFILE)
        self.assertEqual(codec["resource_profile_serialized_after"], "missing_m2")
        self.assertEqual(codec["missing_wmo_policy"], "run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2")
        self.assertEqual(codec["missing_wmo_policy_serialized_after"], "resource_profile")
        self.assertEqual(codec["zero_omissions_still_layout_version"], 6)
        self.assertEqual(codec["missing_wmo_max"], 4096)
        self.assertIn("selection references empty after omission", codec["vacated_wmo_admission"])
        self.assertTrue(codec["v1_v5_layouts_preserved"])
        wrappers = codec["strict_wrappers"]
        self.assertEqual(wrappers["v32"]["layout_version"], 5)
        self.assertEqual(wrappers["v33"]["layout_version"], 6)
        self.assertIn("WDT/M2/WMO policy", wrappers["shared_mask17"])
        self.assertEqual(self.contract["resource_profile"]["only_changed_capacity"], "max_requests")
        self.assertEqual(self.contract["resource_profile"]["relation_to_v32"], "no resource ceilings change from v32")
        self.assertEqual(self.contract["limits"]["request_calls_max"], 16_000_000)
        self.assertIn("no limits relative to v32", self.contract["limits"]["request_calls_change"])
        self.assertFalse(self.contract["limits"]["memory_max_raised"])
        self.assertFalse(self.contract["limits"]["dependency_requests_max_raised"])

    def test_wmo_omission_predicate_positive_and_negative_boundaries(self):
        source = self.contract["source_policy"]
        self.assertIn("locally declared ROOT-absent M2 doodad", source["vacated_wmo"])
        self.assertIn("MODR participated in selection", source["vacated_wmo"])
        self.assertIn("selection.references is empty", source["vacated_wmo"])
        self.assertIn("originally empty WMO", source["vacated_wmo"])
        self.assertIn("MODR participated in selection", source["vacated_wmo"])
        self.assertIn("collision_no_valid_collision", source["vacated_wmo"])
        self.assertIn("WMO FDID 462695", self.cases()["vacated-wmo-positive"]["expect"])
        self.assertIn("FDID 970793", self.cases()["vacated-wmo-positive"]["expect"])
        self.assertIn("present doodad", self.cases()["present-parent-and-child-preserved"]["expect"])
        package = self.contract["package"]
        for field in ("vmos_for_vacated_wmos", "bindings_for_vacated_wmos", "valid_names_for_vacated_wmos", "spawns_for_vacated_wmos", "placements_for_vacated_wmos"):
            self.assertFalse(package[field], field)
        for field in ("vmos_for_omitted_doodads", "name_bindings_for_omitted_doodads", "valid_names_for_omitted_doodads", "spawns_for_omitted_doodads", "placements_for_omitted_doodads"):
            self.assertFalse(package[field], field)
        self.assertIn("GODI", self.cases()["poisoned-excluded-sources"]["expect"])
        self.assertIn("LiquidObject", self.cases()["poisoned-excluded-sources"]["expect"])
        self.assertIn("new scratch", self.cases()["fresh-scratch-resume"]["expect"])
        self.assertIn("collision_no_valid_collision", self.cases()["vacated-wmo-fatal-negatives"]["expect"])
        self.assertIn("not used", self.cases()["vacated-wmo-fatal-negatives"]["expect"])
        self.assertNotIn(">4096", json.dumps(self.cases()["vacated-wmo-positive"]))

    def test_authentication_and_product_claim_boundaries(self):
        verification = self.contract["verification"]
        self.assertEqual(verification["root_absence_authentication"], "producer_reported")
        self.assertEqual(verification["model_absence_authentication"], "producer_reported")
        self.assertEqual(verification["wmo_absence_authentication"], "producer_reported")
        for key in ("root_absence_independently_proven", "model_absence_independently_proven", "wmo_absence_independently_proven"):
            self.assertFalse(verification[key])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        evidence = real["evidence"]
        self.assertEqual(evidence["sha256"], "7ca5b12e7f78899a384617f79d6350e19d320415022f741471c38951bf11e1d4")
        self.assertEqual(evidence["diagnostic_sha256"], "9e5c6e05ff039e652dda5cf77f713146375066b6378fdfeffac6d8cb02b7e9b7")
        self.assertEqual((evidence["inspect_exit"], evidence["extract_exit"]), (0, 9))
        self.assertEqual(evidence["wmo_file_data_id"], 462695)
        self.assertEqual(evidence["local_missing_m2"], 0)
        self.assertEqual(evidence["omitted_usable"], 0)
        self.assertFalse(evidence["accepted_prefix"])
        for stage in ("missing_m2_probe", "verify", "verify_links"):
            self.assertEqual(evidence[stage], "not-run")
        previous = self.contract["existing_evidence"]
        self.assertEqual(previous["v32_acceptance_status"], "failed")
        self.assertEqual(previous["v32_failure_code"], "collision_no_valid_collision")
        self.assertEqual(previous["v32_report_sha256"], "6dd3118c6df2878b5b1b03c18c199fdc1c979e008bc266d6046ffd88c294eccf")
        self.assertEqual(previous["v32_diagnostic_sha256"], "304ed0879e6a5553e5e2a1461d518c14bf986681ef330ea913b9d657522d0c3f")
        self.assertTrue(previous["v32_evidence_is_not_v33_acceptance"])
        for field in ("full_world_coverage", "complete_vmap_component", "runtime_acceptance", "distribution", "full_extractor_complete"):
            self.assertFalse(self.contract["boundary"][field], field)
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
