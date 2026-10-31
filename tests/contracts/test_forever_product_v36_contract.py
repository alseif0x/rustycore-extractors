# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 material-filtered-WMO contract; structural and synthetic only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v36.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v36/all.tsv"
EXPECTED_PLAN_ROWS = [
    ["forever-product", "36"],
    ["build", "70450"],
    ["locale", "esES"],
    ["index", "contiguous"],
    ["root", "header-50893-v2"],
    ["schema-catalog", "wowdbdefs-70450-v1"],
    ["decryption", "disabled"],
    ["partial_encrypted_sections", "declared-skip"],
    ["collision-storage", "linux-leased-append-disk-world-v1"],
    ["collision-raw-bytes", "1048576"],
    ["collision-wdt-bytes", "1048576"],
    ["collision-cache-bytes", "1048576"],
    ["collision-placement-bytes", "1048576"],
    ["collision-vmo-bytes", "1048576"],
    ["collision-map-bytes", "1048576"],
    ["collision-output-bytes", "1048576"],
    ["collision-profile", "linux70450-material-filtered-wmo-vmap-product-v1"],
    ["collision-domain", "all-available-maps"],
    ["collision-coverage", "linux70450-material-filtered-wmo-coverage-v9"],
    ["gameobject-scope", "excluded-by-profile-v1"],
    ["model-scope", "placement-only-v1"],
    ["components", "db2,vmaps"],
    ["terrain", "none"],
    ["pack", "true"],
    ["navigation", "none"],
    ["big-base-unit", "false"],
    ["skip-liquids", "false"],
    ["maximum-slope", "55"],
    ["non-steep-slope", "55"],
    ["db2", "Map", "1349477"],
]
ORDERED_KEYS = [row[0] for row in EXPECTED_PLAN_ROWS[:-1]]
EXPECTED_LISTS = {
    "missing_m2": [970793],
    "missing_wmo": [],
    "empty_wmo": [],
    "unselected_wmo": [],
    "material_filtered_wmo": [462695],
}
MIXED_LISTS = {
    "missing_m2": [970793],
    "missing_wmo": [462694],
    "empty_wmo": [462697],
    "unselected_wmo": [462695],
    "material_filtered_wmo": [462701],
}
BUDGET_FIELDS = [
    "collision-raw-bytes", "collision-wdt-bytes", "collision-cache-bytes",
    "collision-placement-bytes", "collision-vmo-bytes", "collision-map-bytes",
    "collision-output-bytes",
]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def assert_canonical_plan_rows(rows):
    expected_widths = [2] * (len(EXPECTED_PLAN_ROWS) - 1) + [3]
    if [len(row) for row in rows] != expected_widths:
        raise AssertionError("PLAN row widths differ from the canonical schema")
    if rows != EXPECTED_PLAN_ROWS:
        raise AssertionError("PLAN rows differ from the canonical v36 fixture")


class ProductV36Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_json_loader_rejects_duplicate_keys_recursively(self):
        for text in (
            '{"field":1,"field":2}',
            '{"outer":{"field":1,"field":2}}',
            '{"outer":[{"field":1,"field":2}]}',
        ):
            with self.subTest(text=text), self.assertRaisesRegex(
                ValueError, "duplicate JSON object key"
            ):
                json.loads(text, object_pairs_hook=reject_duplicate_keys)

    def test_v36_identity_and_existing_synthetic_plan_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v36", 36, "Build70450MaterialFilteredWmoGeometryV1"),
        )
        self.assertEqual(
            (contract["builds"], contract["platform"], contract["backend"], contract["offline"]),
            ([70450], "linux", "rust-local", True),
        )
        self.assertEqual(contract["component_mask"], 17)
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], "linux70450-material-filtered-wmo-v1")
        self.assertEqual(contract["plan_profile"], "linux70450-material-filtered-wmo-vmap-product-v1")
        self.assertEqual(contract["plan_coverage_profile"], "linux70450-material-filtered-wmo-coverage-v9")
        self.assertEqual(contract["domain_profile"], "material-filtered-wmo-v1")
        self.assertEqual(contract["codec_profile"], contract["source_profile"])
        self.assertEqual(contract["completion"], "linux70450-available-material-filtered-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-material-filtered-wmo-verification-v1")
        self.assertEqual(
            contract["verification_scope"],
            "artifact_integrity_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
        )
        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(plan["component_mask"], 17)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["collision_profile"], contract["plan_profile"])
        self.assertEqual(plan["coverage_profile"], contract["plan_coverage_profile"])
        self.assertEqual(plan["collision_domain"], "all-available-maps")
        self.assertEqual(plan["domain_closure"], contract["domain_profile"])
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["requested_map"], "absent; requested_map and substitute fields are forbidden")
        self.assertEqual(plan["requested_map_zero"], "rejected; zero is not an all-map sentinel")
        self.assertEqual(plan["index"], "contiguous")
        self.assertEqual(plan["root"], "header-50893-v2")
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertEqual(plan["partial_encrypted_sections"].split(";", 1)[0], "declared-skip")
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1048576)
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual(plan["terrain"], "none")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["offmesh"], "absent")
        self.assertTrue(plan["pack"])
        self.assertFalse(plan["big_base_unit"])
        self.assertFalse(plan["skip_liquids"])
        self.assertEqual(plan["maximum_slope"], 55)
        self.assertEqual(plan["non_steep_slope"], 55)
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])
        fixture_path = contract["fixtures"]["cases"][0]["path"]
        self.assertEqual(fixture_path, "tests/contracts/fixtures/forever-product-v36/all.tsv")
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual(rows, EXPECTED_PLAN_ROWS)
        assert_canonical_plan_rows(rows)
        plan_values = {row[0]: row[1] for row in rows[:-1]}
        self.assertEqual(list(plan_values), ORDERED_KEYS)
        self.assertEqual(plan_values["forever-product"], "36")
        self.assertEqual(plan_values["build"], "70450")
        self.assertEqual(plan_values["schema-catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(plan_values["decryption"], "disabled")
        self.assertEqual(plan_values["partial_encrypted_sections"], "declared-skip")
        self.assertEqual(plan_values["collision-storage"], plan["storage"])
        self.assertEqual(
            [plan_values[field] for field in BUDGET_FIELDS],
            [str(plan["synthetic_budget_bytes"])] * 7,
        )
        for key, contract_key in (
            ("collision-profile", "collision_profile"),
            ("collision-coverage", "coverage_profile"),
            ("collision-domain", "collision_domain"),
        ):
            self.assertEqual(plan_values[key], plan[contract_key])
        self.assertEqual(plan_values["components"], ",".join(plan["components"]))
        for row_key, plan_key in (
            ("terrain", "terrain"),
            ("pack", "pack"),
            ("navigation", "navigation"),
            ("big-base-unit", "big_base_unit"),
            ("skip-liquids", "skip_liquids"),
            ("maximum-slope", "maximum_slope"),
            ("non-steep-slope", "non_steep_slope"),
        ):
            value = plan[plan_key]
            self.assertEqual(plan_values[row_key], str(value).lower() if isinstance(value, bool) else str(value))
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        self.assertEqual(rows[-1][1:], [plan["required_db2_entry"]["name"], str(plan["required_db2_entry"]["file_data_id"])])

        invalid_rows = [list(row) for row in rows]
        invalid_rows[0].append("unexpected")
        invalid_build = [list(row) for row in rows]
        invalid_build[1][1] = "70451"
        invalid_decryption = [list(row) for row in rows]
        invalid_decryption[6][1] = "enabled"
        for invalid in (invalid_rows, invalid_build, invalid_decryption):
            with self.subTest(plan=invalid), self.assertRaises(AssertionError):
                assert_canonical_plan_rows(invalid)
        self.assertEqual(len(rows[-1]), 3)
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])

    def test_v9_identity_order_limits_and_resource_profile(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], "vmaps/available-material-filtered-wmo-geometry-coverage-v9.bin")
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 9))
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["material_filtered_wmo_serialized_after"], "unselected_wmo")
        self.assertEqual(self.contract["wmo_omissions"]["list_order"], list(EXPECTED_LISTS))
        self.assertTrue(self.contract["wmo_omissions"]["pairwise_disjoint"])
        for name in EXPECTED_LISTS:
            self.assertEqual(codec["list_max"], 4096)
            self.assertEqual(self.contract["limits"][f"{name}_max"], 4096)
        self.assertEqual(codec["strict_wrappers"]["v36"]["layout_version"], 9)
        self.assertEqual(codec["strict_wrappers"]["v35"]["layout_version"], 8)
        self.assertEqual(codec["strict_wrappers"]["v35"]["contract"], "forever-product-v35")
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-request-heavy-resources-v1")
        self.assertEqual(resource["collector_max_requests"], 16_000_000)
        self.assertEqual(resource["startup_collision_max_requests"], 16_000_000)
        self.assertEqual(resource["only_changed_capacity"], "max_requests")
        self.assertFalse(resource["disk_world_profile_implicit"])
        limits = self.contract["limits"]
        for field in (
            "reads_max_raised", "dependency_requests_max_raised", "source_bytes_max_raised",
            "raw_bytes_max_raised", "models_max_raised", "memory_max_raised",
        ):
            self.assertFalse(limits[field], field)
        self.assertIn("v36 raises no limit from v35", limits["request_calls_change"])

    def test_material_filtered_admission_and_negative_cases_are_structured(self):
        predicate = self.contract["wmo_omissions"]["material_filtered_wmo"]
        self.assertEqual(predicate["origin"], "WMO FileDataId only; logical-name origin is fatal")
        self.assertEqual(predicate["local_missing_m2"], 0)
        self.assertFalse(predicate["omitted_usable"])
        self.assertIsNone(predicate["raw_output"])
        self.assertTrue(predicate["bindings_empty"])
        self.assertTrue(predicate["selection_references_empty"])
        self.assertEqual(predicate["selection_sets"], "present and nonempty")
        self.assertIn("at least one unskipped group", predicate["source_groups"])
        self.assertIn("no skipped source groups", predicate["source_groups"])
        self.assertEqual(predicate["source_triangles"], "every source triangle matches the REJECT predicate")
        material_filter = predicate["material_filter"]
        keep_predicate = "flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)"
        reject_predicate = "flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)"
        self.assertEqual(material_filter["keep_predicate"], keep_predicate)
        self.assertEqual(material_filter["reject_predicate"], reject_predicate)
        self.assertEqual(
            material_filter["examples"],
            {"rejected": ["0x26", "0x66"], "kept": ["0x20", "0x08"]},
        )
        keep = lambda flags: bool(flags & 0x08) or bool(flags & 0x20) and not bool(flags & 0x04)
        reject = lambda flags: not bool(flags & 0x08) and (not bool(flags & 0x20) or bool(flags & 0x04))
        for flag in range(256):
            self.assertNotEqual(keep(flag), reject(flag), f"predicates are not complements at {flag:#04x}")
        for value in (0x26, 0x66):
            self.assertFalse(keep(value))
            self.assertTrue(reject(value))
        for value in (0x20, 0x08):
            self.assertTrue(keep(value))
            self.assertFalse(reject(value))
        self.assertEqual(predicate["liquid"], "no liquid groups, vertices, or tiles")
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        for name in (
            "five-way-mixed-omissions", "material-filtered-only", "normal-results-preserved",
            "predicate-rejections", "production-verifier-attacks", "list-limits", "lifecycle",
        ):
            self.assertIn(name, cases)
        material_expect = cases["material-filtered-only"]["expect"]
        self.assertIn("missing_m2=[970793]", material_expect)
        self.assertIn("missing_wmo=[]", material_expect)
        self.assertIn("empty_wmo=[]", material_expect)
        self.assertIn("unselected_wmo=[]", material_expect)
        self.assertIn("material_filtered_wmo=[462695]", material_expect)
        self.assertIn("0x26/0x66", material_expect)
        self.assertIn("0x20/0x08", material_expect)
        self.assertIn("VMO and placement", material_expect)
        self.assertEqual(MIXED_LISTS["unselected_wmo"], [462695])
        self.assertEqual(MIXED_LISTS["material_filtered_wmo"], [462701])
        self.assertTrue(set(MIXED_LISTS["unselected_wmo"]).isdisjoint(MIXED_LISTS["material_filtered_wmo"]))
        self.assertIn("skip_reason=None", material_expect)
        mixed_expect = cases["five-way-mixed-omissions"]["expect"]
        for expected in (
            "missing_m2=[970793]",
            "missing_wmo=[462694]",
            "empty_wmo=[462697]",
            "unselected_wmo=[462695]",
            "material_filtered_wmo=[462701]",
        ):
            self.assertIn(expected, mixed_expect)
        self.assertIn("distinct", mixed_expect)
        for rejected in (
            "mixed skipped and material-filtered groups", "useful selection", "local missing M2",
            "logical origin", "absent/empty sets", "liquid", "acquisition/parser",
            "conversion", "resource bounds",
        ):
            self.assertIn(rejected, cases["predicate-rejections"]["expect"])
        self.assertIn("accepts 4096 and rejects 4097", cases["list-limits"]["expect"])
        self.assertIn("both production verifiers", cases["list-limits"]["expect"])
        self.assertFalse(self.contract["package"]["omitted_wmo_artifacts"])
        self.assertFalse(self.contract["package"]["omitted_wmo_vmos_bindings_names_spawns_placements_and_ids"])
        self.assertTrue(self.contract["package"]["present_collision_and_doodad_outputs_preserved"])

    def test_v36_real_failure_and_v37_future_design_are_not_acceptance(self):
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v36")
        self.assertEqual(real["run_id"], "v36-material-filtered-wmo-20261026T085714Z")
        self.assertEqual(
            real["report"],
            "reports/acceptance/v36-material-filtered-wmo-20261026T085714Z-acceptance.json",
        )
        self.assertEqual(real["report_sha256"], "dfcca8f3c29b655d5d3a0a2e9794cad7d2484bbf2edb25e29bbe9eefff039e03")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        self.assertEqual(real["inspect_exit"], 0)
        self.assertEqual(real["extract_exit"], 9)
        self.assertEqual(real["extract_error"], "collision_policy_limit")
        self.assertEqual((real["probe_exit"], real["verify_exit"]), (-1, -1))
        self.assertEqual(real["elapsed_seconds"], 23)
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["client_unchanged"])
        self.assertTrue(real["input_unchanged"])
        self.assertFalse(real["accepted_prefix"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        self.assertEqual(real["required_lists"], EXPECTED_LISTS)
        self.assertNotIn("admitted", real)
        self.assertIn("do not establish that WMO 462695 was admitted", real["note"])
        self.assertEqual(
            real["diagnostic_counters"],
            {
                "request_calls": 19243,
                "reads": 3362,
                "dependency_requests": 4096,
                "models": 2118,
                "source_bytes": 221090913,
                "raw_bytes": 31135205,
                "external_bytes": 269362435,
                "external_source_bytes": 11872896,
                "resident_bytes": 314993930,
                "store_bytes": 306605630,
                "memory_limit_bytes": 1073741824,
            },
        )
        software = real["software_evidence"]
        self.assertEqual(
            (software["suites"], software["tests_passed"], software["ignored"], software["contracts_passed"]),
            (80, 4414, 26, 519),
        )
        self.assertEqual(software["binary_sha256"], "1a6fc2bf5de707b82ebb3e2c3e72d77a1eb2fcfd9dad4a8bee04e9ceb22f624c")
        self.assertEqual(software["source_sha256"], "26ab8317baead71a3434b4af7906418057344211075c65bf661169b4d4fa6625")
        self.assertEqual(software["software_sha256"], "a289aeffc2aa216a731f1bce1ff24a2126fb9c5fcb0c997d4c9f820af397ff85")

        v37 = self.contract["future_v37_design"]
        self.assertEqual(v37["status"], "design-only; not implemented or accepted")
        self.assertEqual(v37["profile_name"], "DependencyHeavyV1")
        self.assertEqual(v37["planned_capacity_change"], "max_dependency_requests only")
        self.assertEqual(v37["planned_max_dependency_requests"], 16_000_000)
        self.assertEqual(v37["preserve_request_heavy_max_requests"], 16_000_000)
        self.assertEqual(v37["all_other_limits"], "unchanged from v36")
        self.assertEqual(v37["coverage_layout_version"], 10)
        self.assertEqual(v37["target_lists"], EXPECTED_LISTS)
        self.assertEqual(v37["acceptance_status"], "target-not-run")
        self.assertFalse(v37["implementation_claim"])
        self.assertFalse(v37["acceptance_claim"])
        prior = self.contract["prior_evidence"]
        self.assertEqual(prior["v35_acceptance_status"], "failed")
        self.assertEqual(prior["v35_report_sha256"], "dc088ff8159a276b2004586b86437bd86cb2c3c23abf37f3ec102c214b6a237a")
        self.assertEqual(prior["v35_selection_probe_report_sha256"], "9779537a4dccccfeac45f4b30eab90a4f03c7536b0ed0806d5685bdbbb013425")
        self.assertEqual(prior["v35_selection_probe_evidence_sha256"], "1d0d798af16e711e10c5ce1f6b16bfdc12f1cc3f33d801b0df01a5cce0ebcf3d")
        self.assertEqual(prior["v35_selection_probe_tool_sha256"], "df3d88bc4b8809f5f1a20321d49408ea2b41e0770d026691487b6fec547f1212")
        self.assertFalse(prior["acceptance_transferred_to_v36"])
        self.assertFalse(self.contract["full_extractor_complete"])
        for claim in ("full_world_coverage", "complete_vmap_component", "runtime_acceptance", "distribution", "release_publication"):
            self.assertFalse(self.contract["boundary"][claim])


if __name__ == "__main__":
    unittest.main()
