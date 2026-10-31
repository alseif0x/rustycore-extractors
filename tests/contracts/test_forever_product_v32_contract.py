# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 RequestHeavyV1 geometry contract; synthetic v32 only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v32.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v32/all.tsv"
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
SOURCE_PROFILE = "linux70450-available-map-model-root-omissions-request-capacity-v1"
PLAN_PROFILE = "linux70450-available-map-model-root-omissions-request-capacity-vmap-product-v1"
COVERAGE_PROFILE = "linux70450-available-map-model-root-omissions-request-capacity-coverage-v5"
COMPLETION_PROFILE = "linux70450-available-map-model-root-omissions-request-capacity-collision-v1"
VERIFICATION_PROFILE = "linux70450-available-map-model-root-omissions-request-capacity-verification-v1"
COVERAGE_PATH = "vmaps/available-request-capacity-geometry-coverage-v5.bin"
RESOURCE_PROFILE = "linux-collision-request-heavy-resources-v1"
DECLARE_ROOT_ABSENT = "run::MissingWdtPolicy::DeclareRootAbsent"
DECLARE_M2_ABSENT = "run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2"


class ProductV32Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_plan_literals_and_exact_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v32", 32, "Build70450AvailableModelRootAbsentRequestCapacityGeometryV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual((contract["platform"], contract["backend"], contract["offline"]), ("linux", "rust-local", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], SOURCE_PROFILE)
        self.assertEqual(contract["completion"], COMPLETION_PROFILE)
        self.assertEqual(contract["plan_profile"], PLAN_PROFILE)
        self.assertEqual(contract["plan_coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(contract["domain_profile"], "available-map-model-root-omissions-request-capacity-v1")
        self.assertEqual(contract["codec_profile"], SOURCE_PROFILE)
        self.assertEqual(contract["verification_profile"], VERIFICATION_PROFILE)
        self.assertEqual(contract["verification_scope"], "artifact_integrity_available_map_model_root_omissions_request_capacity_and_independently_derived_map_vmap_domain_coverage")

        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(plan["trailing_db2_rows"], "exactly one final row: db2<TAB>Map<TAB>1349477")
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertIn("declared-skip", plan["partial_encrypted_sections"])
        self.assertIn("PLAN only", plan["partial_encrypted_sections"])
        self.assertIn("between decryption and collision-storage", plan["partial_encrypted_sections"])
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["collision_domain"], "all-available-maps")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertIn("no requested_map or substitute", plan["requested_map"])
        self.assertIn("zero is not an all-map sentinel", plan["requested_map_zero"])
        self.assertIn("MapAndAncestors(0) is not MapSelection::All", plan["map_and_ancestors_zero"])
        self.assertEqual(plan["collision_profile"], PLAN_PROFILE)
        self.assertEqual(plan["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(plan["domain_closure"], "available-map-model-root-omissions-request-capacity-v1")
        self.assertEqual((plan["terrain"], plan["navigation"], plan["offmesh"]), ("none", "none", "absent"))
        self.assertTrue(plan["pack"])
        self.assertFalse(plan["inspect_requires_client"])

        fixture = self.contract["fixtures"]
        self.assertEqual(fixture["classification"], "canonical synthetic PLAN fixtures only")
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertFalse(fixture["certified_real_storage_budgets"])
        self.assertEqual(len(fixture["cases"]), 1)
        self.assertEqual(fixture["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v32/all.tsv")
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual([row[0] for row in rows[:-1]], ORDERED_KEYS)
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        fields = {row[0]: row[1] for row in rows[:-1]}
        expected = {
            "forever-product": "32", "build": "70450", "locale": "esES",
            "index": "contiguous", "root": "header-50893-v2",
            "schema-catalog": "wowdbdefs-70450-v1", "decryption": "disabled",
            "partial_encrypted_sections": "declared-skip",
            "collision-storage": "linux-leased-append-disk-world-v1",
            "collision-profile": PLAN_PROFILE, "collision-domain": "all-available-maps",
            "collision-coverage": COVERAGE_PROFILE,
            "gameobject-scope": "excluded-by-profile-v1", "model-scope": "placement-only-v1",
            "components": "db2,vmaps", "terrain": "none", "pack": "true",
            "navigation": "none", "big-base-unit": "false", "skip-liquids": "false",
            "maximum-slope": "55", "non-steep-slope": "55",
        }
        expected.update({name: "1048576" for name in BUDGET_FIELDS})
        self.assertEqual(fields, expected)
        self.assertNotIn("requested_map", fields)
        self.assertNotIn("partial_encrypted_tables", fields)

    def test_request_heavy_changes_only_max_requests(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], RESOURCE_PROFILE)
        self.assertEqual(resource["collector_max_requests"], 16_000_000)
        self.assertEqual(resource["startup_collision_max_requests"], 16_000_000)
        self.assertEqual(resource["only_changed_capacity"], "max_requests")
        self.assertIn("reads, dependency_requests, source_bytes, raw_bytes, models", resource["all_other_limits"])
        self.assertIn("memory", resource["all_other_limits"])
        self.assertFalse(resource["disk_world_profile_implicit"])
        source = self.contract["source_policy"]
        self.assertIn("only collector.max_requests", source["resource_profile"])
        self.assertIn("dependency_requests", source["resource_profile"])
        self.assertIn("memory", source["resource_profile"])
        self.assertIn("never implicitly selected", source["resource_profile"])
        self.assertIn("request_calls=4096", source["request_calls"])
        self.assertIn("model handles", source["request_calls"])
        self.assertIn("memory limits remain enforced", source["request_calls"])
        limits = self.contract["limits"]
        self.assertEqual(limits["request_calls_max"], 16_000_000)
        self.assertIn("only max_requests", limits["request_calls_change"])
        for field in ("reads_max_raised", "dependency_requests_max_raised", "source_bytes_max_raised", "raw_bytes_max_raised", "models_max_raised", "memory_max_raised"):
            self.assertFalse(limits[field], field)
        self.assertEqual(limits["coverage_maps_max"], 4096)
        self.assertEqual(limits["models_max"], 4096)
        self.assertEqual(limits["missing_m2_max"], 4096)
        self.assertEqual(limits["staged_placement_bytes_max"], 128 * 1024**2)
        self.assertEqual(limits["staged_vmo_bytes_max"], 384 * 1024**2)
        self.assertEqual(limits["assembled_domain_bytes_max"], 128 * 1024**2)
        self.assertEqual(limits["coverage_artifacts_max"], 65535)
        self.assertEqual(limits["package_files_max"], 65536)
        self.assertIn("Map.db2 and RCVGEO1 coverage", limits["package_files_scope"])
        self.assertEqual(limits["package_total_bytes_equation"], "checked_add(collision-output-bytes, 17179869184) where 17179869184 is the 16 GiB DB2 ceiling; overflow rejects")
        request_case = self.cases()["request-capacity"]["expect"]
        self.assertIn("beyond 4096", request_case)
        self.assertIn("without extra model/source reads", request_case)
        self.assertIn("N+1", request_case)
        self.assertIn("memory limits remain enforced", request_case)

    def test_v5_coverage_and_v31_v32_crossings_are_closed(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], COVERAGE_PATH)
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 5))
        self.assertEqual(codec["profile"], SOURCE_PROFILE)
        self.assertEqual(codec["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(codec["resource_profile"], RESOURCE_PROFILE)
        self.assertEqual(codec["resource_profile_serialized_after"], "missing_m2")
        self.assertEqual(codec["selection"], "MapSelection::All")
        self.assertEqual(codec["missing_wdt_policy"], DECLARE_ROOT_ABSENT)
        self.assertEqual(codec["missing_m2_policy"], DECLARE_M2_ABSENT)
        self.assertEqual(codec["zero_omissions_still_layout_version"], 5)
        self.assertIn("after Coverage.maps", codec["missing_m2"])
        self.assertIn("before resource_profile and artifacts", codec["missing_m2"])
        self.assertTrue(codec["v1_v4_layouts_preserved"])
        wrappers = codec["strict_wrappers"]
        self.assertEqual(wrappers["v31"]["layout_version"], 4)
        self.assertEqual(wrappers["v31"]["resource_profile"], "linux-collision-bounded-resources-v1")
        self.assertEqual(wrappers["v32"]["layout_version"], 5)
        self.assertEqual(wrappers["v32"]["resource_profile"], RESOURCE_PROFILE)
        self.assertIn("does not permit crossing", wrappers["shared_mask17"])
        self.assertIn("resource profile", wrappers["shared_mask17"])
        self.assertIn("publication profile", wrappers["shared_mask17"])
        self.assertIn("RCVGEO1 v5", codec["context"]["binding"])
        self.assertIn("v31 remains RCVGEO1 v4 with BoundedV1", codec["context"]["binding"])

    def test_fixture_retains_omissions_poison_resume_and_fatal_boundaries(self):
        source = self.contract["source_policy"]
        for value in (source["wdt_root_absence"], source["model_root_absence"]):
            self.assertIn("guarded", value)
        self.assertIn("dependency_requests, not top-level request_calls", source["model_root_absence"])
        self.assertIn("zero child model_reads", source["model_root_absence"])
        self.assertIn("direct ADT M2 requests are never eligible", source["model_root_absence"])
        self.assertIn("WMO root VMO and WMO placement are preserved", source["omitted_model_effects"])
        self.assertIn("present doodads retain their VMOs/bindings/spawns", source["omitted_model_effects"])
        for fatal in ("direct ADT M2", "WMO root or WMO group", "ineligible", "central CKey absent", "encoding absent", "malformed", "liquid", "resource-limit"):
            self.assertTrue(any(fatal in item for item in source["fatal_not_model_absence"]), fatal)
        package = self.contract["package"]
        self.assertTrue(package["present_wmo_and_doodads_retained"])
        for field in ("vmos_for_omitted_doodads", "name_bindings_for_omitted_doodads", "valid_names_for_omitted_doodads", "spawns_for_omitted_doodads", "placements_for_omitted_doodads", "artifacts_for_root_absent_maps"):
            self.assertFalse(package[field], field)
        cases = self.cases()
        for name in ("per-table-manifest-and-resume", "coverage-codec-v5-and-strict-wrappers", "present-and-omitted-artifacts", "map-and-wdt-boundaries", "excluded-source-poisons", "parent-replay", "resource-boundaries", "cli-help-and-capabilities"):
            self.assertIn(name, cases)
        self.assertIn("fresh scratch", cases["per-table-manifest-and-resume"]["expect"])
        self.assertIn("GODI", cases["excluded-source-poisons"]["expect"])
        self.assertIn("LiquidObject", cases["excluded-source-poisons"]["expect"])
        self.assertIn("WDT==0", cases["map-and-wdt-boundaries"]["expect"])
        self.assertIn("present empty", cases["map-and-wdt-boundaries"]["expect"])

    def test_real_v32_failed_and_v31_failure_is_preserved(self):
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v32")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        evidence = real["evidence"]
        self.assertEqual(
            evidence["sha256"],
            "6dd3118c6df2878b5b1b03c18c199fdc1c979e008bc266d6046ffd88c294eccf",
        )
        self.assertEqual(
            evidence["diagnostic_sha256"],
            "304ed0879e6a5553e5e2a1461d518c14bf986681ef330ea913b9d657522d0c3f",
        )
        self.assertEqual((evidence["inspect_exit"], evidence["extract_exit"]), (0, 9))
        self.assertEqual(evidence["failure_code"], "collision_no_valid_collision")
        self.assertEqual(evidence["request_calls_at_diagnostic"], 6217)
        self.assertEqual(evidence["wmo_file_data_id"], 462695)
        self.assertFalse(evidence["accepted_prefix"])
        for stage in ("missing_m2_probe", "verify", "verify_links"):
            self.assertEqual(evidence[stage], "not-run")
        self.assertFalse(real["v31_evidence_transferred"])
        existing = self.contract["existing_evidence"]
        self.assertEqual(existing["v31_acceptance_status"], "failed")
        self.assertEqual(existing["v31_request_calls"], 4096)
        self.assertEqual(existing["v31_report_sha256"], "6dd0daaaf3f601b91859789266e851a38f8367b1d37b47aa3fd1778b30e79444")
        self.assertEqual((existing["v31_inspect_exit"], existing["v31_extract_exit"]), (0, 9))
        self.assertEqual(existing["v31_probe_verify_verify_links"], "not-run")
        self.assertFalse(existing["v31_accepted_prefix"])
        self.assertTrue(existing["v31_failure_is_not_v32_acceptance"])
        self.assertFalse(self.contract["full_extractor_complete"])
        for field in ("full_world_coverage", "complete_vmap_component", "runtime_acceptance", "format_parity", "distribution", "release_publication"):
            self.assertFalse(self.contract["boundary"][field], field)
        self.assertFalse(self.contract["verification"]["full_world_coverage"])
        self.assertFalse(self.contract["verification"]["complete_vmap_component"])


if __name__ == "__main__":
    unittest.main()
