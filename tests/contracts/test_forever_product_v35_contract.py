# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 unselected-WMO product contract; synthetic v35 only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v35.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v35/all.tsv"
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
EXPECTED_LISTS = {
    "missing_m2": [970793],
    "missing_wmo": [],
    "empty_wmo": [],
    "unselected_wmo": [462695],
}
MIXED_LISTS = {
    "missing_m2": [970793],
    "missing_wmo": [462694],
    "empty_wmo": [462697],
    "unselected_wmo": [462695],
}
BUDGET_FIELDS = [
    "collision-raw-bytes",
    "collision-wdt-bytes",
    "collision-cache-bytes",
    "collision-placement-bytes",
    "collision-vmo-bytes",
    "collision-map-bytes",
    "collision-output-bytes",
]
FIXTURE_FIELD_VALUES = {
    "forever-product": lambda c: str(c["version"]),
    "build": lambda c: str(c["builds"][0]),
    "locale": lambda c: c["fixtures"]["cases"][0]["locale"],
    "index": lambda c: c["plan"]["index"],
    "root": lambda c: c["plan"]["root"],
    "schema-catalog": lambda c: c["plan"]["schema_catalog"],
    "decryption": lambda c: c["plan"]["decryption"],
    "partial_encrypted_sections": lambda c: c["plan"]["partial_encrypted_sections"].split(";", 1)[0],
    "collision-storage": lambda c: c["plan"]["storage"],
    **{field: lambda c: str(c["fixtures"]["synthetic_budget_bytes"]) for field in BUDGET_FIELDS},
    "collision-profile": lambda c: c["plan"]["collision_profile"],
    "collision-domain": lambda c: c["plan"]["collision_domain"],
    "collision-coverage": lambda c: c["plan"]["coverage_profile"],
    "gameobject-scope": lambda c: c["plan"]["gameobject_scope"],
    "model-scope": lambda c: c["plan"]["model_scope"],
    "components": lambda c: ",".join(c["plan"]["components"]),
    "terrain": lambda c: c["plan"]["terrain"],
    "pack": lambda c: str(c["plan"]["pack"]).lower(),
    "navigation": lambda c: c["plan"]["navigation"],
    "big-base-unit": lambda c: str(c["plan"]["big_base_unit"]).lower(),
    "skip-liquids": lambda c: str(c["plan"]["skip_liquids"]).lower(),
    "maximum-slope": lambda c: str(c["plan"]["maximum_slope"]),
    "non-steep-slope": lambda c: str(c["plan"]["non_steep_slope"]),
}


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV35Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_json_loader_rejects_duplicate_keys_recursively(self):
        for text in (
            '{"field":1,"field":2}',
            '{"outer":{"field":1,"field":2}}',
        ):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError, "duplicate JSON object key"):
                json.loads(text, object_pairs_hook=reject_duplicate_keys)

    def test_identity_and_existing_plan_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v35", 35, "Build70450AvailableUnselectedWmoGeometryV1"),
        )
        self.assertEqual((contract["builds"], contract["platform"], contract["backend"], contract["offline"]), ([70450], "linux", "rust-local", True))
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], "linux70450-available-unselected-wmo-v1")
        self.assertEqual(contract["plan_profile"], "linux70450-available-unselected-wmo-vmap-product-v1")
        self.assertEqual(contract["plan_coverage_profile"], "linux70450-available-unselected-wmo-coverage-v8")
        self.assertEqual(contract["domain_profile"], "available-unselected-wmo-v1")
        self.assertEqual(contract["codec_profile"], "linux70450-available-unselected-wmo-v1")
        self.assertEqual(contract["completion"], "linux70450-available-unselected-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-available-unselected-wmo-verification-v1")
        self.assertEqual(contract["verification_scope"], "artifact_integrity_available_unselected_wmo_and_independently_derived_map_vmap_domain_coverage")
        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(plan["component_mask"], 17)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["index"], "contiguous")
        self.assertEqual(plan["root"], "header-50893-v2")
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["partial_encrypted_sections"].split(";", 1)[0], "declared-skip")
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["domain_closure"], "available-unselected-wmo-v1")
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["collision_domain"], "all-available-maps")
        self.assertEqual(plan["terrain"], "none")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["offmesh"], "absent")
        self.assertEqual(plan["pack"], True)
        self.assertEqual(plan["big_base_unit"], False)
        self.assertEqual(plan["skip_liquids"], False)
        self.assertEqual((plan["maximum_slope"], plan["non_steep_slope"]), (55, 55))
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["requested_map"], "absent; no requested_map or substitute field is admitted")
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["collision_profile"], "linux70450-available-unselected-wmo-vmap-product-v1")
        self.assertEqual(plan["coverage_profile"], "linux70450-available-unselected-wmo-coverage-v8")
        self.assertEqual(plan["domain_closure"], "available-unselected-wmo-v1")
        self.assertEqual(contract["fixtures"]["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v35/all.tsv")
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])
        self.assertFalse(contract["fixtures"]["certified_real_storage_budgets"])

        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertTrue(all(len(row) == 2 for row in rows[:-1]))
        self.assertEqual(len(rows[-1]), 3)
        self.assertEqual([row[0] for row in rows[:-1]], ORDERED_KEYS)
        self.assertEqual(rows[-1], ["db2", "Map", str(plan["required_db2_entry"]["file_data_id"])])
        fields = {row[0]: row[1] for row in rows[:-1]}
        self.assertEqual(set(fields), set(FIXTURE_FIELD_VALUES))
        for key, expected in FIXTURE_FIELD_VALUES.items():
            self.assertEqual(fields[key], expected(contract), key)
        self.assertEqual(set(plan["storage_budget_fields"]), set(BUDGET_FIELDS))
        self.assertEqual({fields[name] for name in BUDGET_FIELDS}, {"1048576"})
        self.assertEqual(plan["storage_budgets_downward_only"], True)
        self.assertNotIn("requested_map", fields)

    def test_v8_codec_and_strict_version_boundaries(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], "vmaps/available-unselected-wmo-geometry-coverage-v8.bin")
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 8))
        self.assertEqual(codec["profile"], "linux70450-available-unselected-wmo-v1")
        self.assertEqual(codec["coverage_profile"], "linux70450-available-unselected-wmo-coverage-v8")
        self.assertEqual(codec["resource_profile"], "linux-collision-request-heavy-resources-v1")
        self.assertEqual(codec["missing_wmo_policy"], "run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected")
        self.assertEqual(codec["resource_profile_serialized_after"], "missing_m2")
        self.assertEqual(codec["missing_wmo_policy_serialized_after"], "resource_profile")
        self.assertEqual(codec["empty_wmo_serialized_after"], "missing_wmo")
        self.assertEqual(codec["unselected_wmo_serialized_after"], "empty_wmo")
        self.assertEqual(codec["unselected_wmo_max"], 4096)
        self.assertTrue(codec["v1_v7_layouts_preserved"])
        self.assertEqual(codec["strict_wrappers"]["v34"]["layout_version"], 7)
        v35 = codec["strict_wrappers"]["v35"]
        self.assertEqual(v35["contract"], "forever-product-v35")
        self.assertEqual(v35["layout_version"], 8)
        self.assertEqual(v35["component_mask"], 17)
        self.assertEqual(v35["coverage_path"], codec["path"])
        self.assertEqual(v35["profile"], codec["profile"])
        self.assertEqual(v35["resource_profile"], codec["resource_profile"])
        self.assertEqual(v35["selection"], "MapSelection::All")
        self.assertEqual(v35["coverage_authentication"], "independently_derived")
        self.assertEqual(v35["wmo_absence_authentication"], "producer_reported")
        self.assertEqual(v35["verification_profile"], "linux70450-available-unselected-wmo-verification-v1")
        self.assertEqual(v35["wmo_absence_authentication"], "producer_reported")
        self.assertEqual(v35["verification_profile"], self.contract["verification_profile"])
        self.assertIn("V34 remains coverage v7", codec["binding"])
        self.assertIn("cannot be relabeled as v35", codec["binding"])

        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-request-heavy-resources-v1")
        self.assertEqual(resource["collector_max_requests"], 16_000_000)
        self.assertEqual(resource["startup_collision_max_requests"], 16_000_000)
        self.assertEqual(resource["only_changed_capacity"], "max_requests")
        self.assertEqual(resource["relation_to_v32"], "no resource ceilings change from v32")
        self.assertIn("dependency_requests", resource["all_other_limits"])
        self.assertIn("memory", resource["all_other_limits"])
        self.assertFalse(resource["disk_world_profile_implicit"])
        limits = self.contract["limits"]
        self.assertEqual(limits["request_calls_max"], 16_000_000)
        self.assertEqual(limits["request_calls_change"], "only max_requests changes relative to BoundedV1; v35 changes no limits relative to v32 through v34")
        for field in ("reads_max_raised", "dependency_requests_max_raised", "source_bytes_max_raised", "raw_bytes_max_raised", "models_max_raised", "memory_max_raised"):
            self.assertFalse(limits[field], field)
        self.assertEqual(limits["coverage_maps_max"], 4096)
        self.assertEqual(limits["coverage_artifacts_max"], 65535)
        self.assertEqual(limits["package_files_max"], 65536)
        self.assertEqual(limits["staged_vmo_bytes_max"], 402653184)
        self.assertEqual(limits["staged_placement_bytes_max"], 134217728)
        self.assertEqual(limits["models_max"], 4096)
        self.assertEqual(limits["missing_m2_max"], 4096)
        self.assertEqual(limits["missing_wmo_max"], 4096)
        self.assertEqual(limits["empty_wmo_max"], 4096)
        self.assertEqual(limits["unselected_wmo_max"], 4096)

        self.assertEqual(self.contract["source_profile"], codec["profile"])
        self.assertEqual(self.contract["plan_profile"], self.contract["plan"]["collision_profile"])
        self.assertEqual(self.contract["plan_coverage_profile"], codec["coverage_profile"])
        self.assertEqual(self.contract["domain_profile"], codec["domain_closure"])
        self.assertEqual(self.contract["codec_profile"], codec["profile"])
        self.assertEqual(self.contract["completion"], "linux70450-available-unselected-wmo-collision-v1")
        self.assertEqual(self.contract["verification_profile"], v35["verification_profile"])
        self.assertEqual(self.contract["verification_scope"], self.contract["verification"]["report_scope"])
        verification = self.contract["verification"]
        self.assertEqual(verification["profile"], v35["verification_profile"])
        self.assertEqual(verification["report_scope"], self.contract["verification_scope"])
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertEqual(verification["wmo_absence_authentication"], "producer_reported")
        self.assertEqual(verification["unselected_wmo_authentication"], "producer_reported")
        self.assertIn("unselected-WMO", verification["verify_scope"])
        self.assertIn("unselected-WMO", verification["verify_links_scope"])

    def test_separate_omissions_and_exact_requested_scenario(self):
        self.assertEqual(self.contract["real_acceptance"]["required_lists"], EXPECTED_LISTS)
        predicate = " ".join((self.contract["source_policy"]["unselected_wmo"], self.contract["coverage_codec"]["unselected_wmo_admission"]))
        for condition in (
            "FileDataId-origin",
            "local_missing_m2=0",
            "omitted_usable=false",
            "output=None",
            "empty bindings",
            "selection.references",
            "present nonempty root selection sets",
            "skip_reason",
            "source triangle",
            "index",
            "vertex counts",
            "no unskipped group",
            "material filtering",
        ):
            self.assertIn(condition, predicate)
        source_policy = self.contract["source_policy"]
        cases = self.cases()
        self.assertIn("not proof of intrinsic emptiness", source_policy["unselected_wmo"])
        self.assertIn("normal Collision or DoodadOnly", source_policy["unselected_wmo"])
        self.assertIn("RAW output=None without a normal DoodadOnly result", source_policy["unselected_wmo"])
        self.assertIn("without a complete admitted omission predicate remains fatal", source_policy["unselected_wmo"])
        self.assertIn("Required-source acquisition/parser errors are always fatal", source_policy["unselected_wmo"])
        self.assertIn("even if a WMO omission declaration otherwise applies", source_policy["unselected_wmo"])
        producer_reported = " ".join(self.contract["verification"]["producer_reported_facts"])
        for condition in ("unselected_wmo producer classification", "local_missing_m2=0", "omitted_usable=false", "skipped group"):
            self.assertIn(condition, producer_reported)
        offline_limit = self.contract["verification"]["offline_limit"]
        self.assertIn("unselected_wmo producer classification", offline_limit)
        self.assertIn("nonempty sets", offline_limit)
        self.assertIn("skipped source geometry counts", offline_limit)
        self.assertIn("all-nonempty unskipped group", offline_limit)
        fatal = " ".join(self.contract["source_policy"]["fatal_not_model_or_wmo_absence"])
        self.assertIn("present-but-unselected-WMO predicate", fatal)
        self.assertIn("conversion/no-collision", fatal)
        self.assertIn("RAW output=None without a normal DoodadOnly result and without a complete admitted WMO omission predicate", fatal)
        self.assertIn("any required-source acquisition or parser error, regardless of WMO omission declarations", fatal)
        self.assertIn("useful output/selection/references instead follows normal Collision/DoodadOnly", cases["unselected-predicate-and-normal-results"]["expect"])
        failure_policy = source_policy["failure_policy"]
        self.assertIn("Required-source acquisition and parser errors are always fatal", failure_policy)
        self.assertIn("no WMO omission declaration compensates for them", failure_policy)
        self.assertIn("RAW output=None alone is not fatal when normal DoodadOnly output exists", failure_policy)
        self.assertIn("there is no normal DoodadOnly result", failure_policy)
        self.assertIn("no complete admitted WMO omission predicate applies", failure_policy)
        self.assertIn("intrinsic-empty predicate", failure_policy)
        self.assertIn("present-but-unselected predicate", failure_policy)
        codec_admission = self.contract["coverage_codec"]["unselected_wmo_admission"]
        self.assertIn("RAW output=None alone is not fatal when a normal DoodadOnly result exists", codec_admission)
        self.assertIn("there is no normal DoodadOnly result", codec_admission)
        self.assertIn("no complete admitted omission predicate applies", codec_admission)
        self.assertIn("Required-source acquisition or parser errors are always fatal", codec_admission)
        self.assertIn("cannot be compensated by a WMO omission declaration", codec_admission)
        predicate_matrix = cases["unselected-predicate-and-normal-results"]
        self.assertIn("RAW output=None alone is not fatal when a normal DoodadOnly result exists", predicate_matrix["expect"])
        self.assertIn("no normal DoodadOnly result", predicate_matrix["expect"])
        self.assertIn("no complete admitted omission predicate", predicate_matrix["expect"])
        self.assertIn("required-source acquisition/parser errors are always fatal", predicate_matrix["expect"])
        self.assertIn("cannot be compensated by any WMO omission declaration", predicate_matrix["expect"])
        self.assertIn("required-source acquisition or parser errors are always fatal regardless of WMO omission declarations", predicate_matrix["fatal-cases"])
        self.assertIn("no normal DoodadOnly result", predicate_matrix["fatal-cases"])
        self.assertIn("no complete admitted WMO omission predicate", predicate_matrix["fatal-cases"])
        self.assertEqual(cases["unselected-predicate-and-normal-results"]["case"], "unselected-predicate-and-normal-results")
        predicate_expectation = cases["unselected-predicate-and-normal-results"]["expect"]
        for condition in ("FileDataId origin", "local_missing_m2=0", "omitted_usable=false", "output=None", "empty bindings/references", "present nonempty sets", "at least one skipped group", "no unskipped group", "normal Collision/DoodadOnly", "material-filtered"):
            self.assertIn(condition, predicate_expectation)
        self.assertIn("exact missing_m2=[970793]", cases["mixed-omissions"]["expect"])
        for name, ids in MIXED_LISTS.items():
            self.assertIn(f"{name}={ids}", cases["mixed-omissions"]["expect"])
        self.assertIn("unselected_wmo=[462695]", cases["unselected-wmo-only"]["expect"])
        self.assertIn("never intrinsically empty", cases["unselected-wmo-only"]["expect"])
        for name in ("missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo"):
            self.assertEqual(self.contract["limits"][f"{name}_max"], 4096)
        package = self.contract["package"]
        for category, fields in {
            "omitted_doodads": (
                "vmos_for_omitted_doodads",
                "name_bindings_for_omitted_doodads",
                "valid_names_for_omitted_doodads",
                "spawns_for_omitted_doodads",
                "placements_for_omitted_doodads",
            ),
            "vacated_wmos": tuple(f"{field}_for_vacated_wmos" for field in ("vmos", "bindings", "valid_names", "spawns", "placements")),
            "empty_wmos": tuple(f"{field}_for_empty_wmos" for field in ("vmos", "bindings", "valid_names", "spawns", "placements")),
            "unselected_wmos": tuple(f"{field}_for_unselected_wmos" for field in ("vmos", "bindings", "valid_names", "spawns", "placements")),
        }.items():
            for field in fields:
                self.assertFalse(package[field], f"{category}: {field}")
        self.assertIn("GODI", cases["poisoned-excluded-sources"]["expect"])
        self.assertIn("LiquidObject", cases["poisoned-excluded-sources"]["expect"])

    def test_real_v35_failure_and_v34_prior_evidence_are_not_promoted(self):
        prior = self.contract["existing_evidence"]
        self.assertEqual(prior["v34_acceptance_status"], "failed")
        self.assertEqual(prior["v34_report_sha256"], "3a00385706ce85359bd5032fb47853d0a4fed9335436ddf3df1ffd81fdaf019c")
        self.assertEqual(prior["v34_diagnostic_sha256"], "8bbaa2e588292980ac2894d30e6c3bac446571439f233e051376d2ab43d3f68f")
        self.assertTrue(prior["v34_failure_is_not_v35_acceptance"])
        self.assertFalse(prior["older_acceptance_transferred_to_v35"])

        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v35")
        self.assertEqual(real["run_id"], "v35-unselected-wmo-20261026T040320Z")
        self.assertEqual(real["report"], "reports/acceptance/v35-unselected-wmo-20261026T040320Z-acceptance.json")
        self.assertEqual(real["report_sha256"], "dc088ff8159a276b2004586b86437bd86cb2c3c23abf37f3ec102c214b6a237a")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        self.assertEqual(real["required_lists"], EXPECTED_LISTS)
        self.assertEqual(real["commands"], {
            "inspect": 0,
            "extract": 9,
            "extract_code": "collision_no_valid_collision",
            "extract_component": "vmaps",
            "probe": -1,
            "verify": -1,
            "verify_links": -1,
        })
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["client_metadata_unchanged"])
        self.assertTrue(real["inputs_unchanged"])
        self.assertEqual(real["elapsed_seconds"], 12)
        self.assertFalse(real["accepted_prefix"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        self.assertIn("no skip_reason", real["cause"])
        self.assertEqual(real["software_qualification"], {
            "rust_suites": 79,
            "rust_passed": 4376,
            "rust_ignored": 26,
            "rust_failed": 0,
            "contract_tests": 513,
            "binary_sha256": "82c6f97ea009bfebfae92a03b9549a5cd5609493cb078b4876eca9f72f8195a6",
            "source_sha256": "66375ae9280743b196e73b5a5b81bd2965dc776f0d2eae19825815a0776c0013",
            "software_sha256": "3f90d258d30578b073fa956355948e8cb590ada6e0aad9fdc72054087cfc4b1d",
        })
        cases = self.cases()
        real_case = cases["real-acceptance"]
        self.assertEqual(real_case["status"], "failed")
        self.assertEqual(real_case["run_id"], real["run_id"])
        self.assertIn("collision_no_valid_collision", real_case["expect"])
        self.assertIn("do not infer a skip_reason", real_case["expect"])
        self.assertEqual(cases["v34-retained-failure"]["status"], "failed")
        self.assertFalse(self.contract["full_extractor_complete"])
        for claim in ("full_world_coverage", "complete_vmap_component", "runtime_acceptance", "distribution"):
            self.assertFalse(self.contract["boundary"][claim])

    def test_selection_probe_diagnostics_and_unexecuted_future_design(self):
        real = self.contract["real_acceptance"]
        self.assertEqual((real["status"], real["accepted"]), ("failed", False))
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        probe = real["selection_probe"]
        self.assertEqual(probe["schema"], "v35-selection-probe-v1")
        self.assertEqual(probe["status"], "ok")
        self.assertEqual(probe["report"], "reports/acceptance/v35-selection-probe-20261026T041841Z-evidence.json")
        self.assertEqual(probe["report_sha256"], "9779537a4dccccfeac45f4b30eab90a4f03c7536b0ed0806d5685bdbbb013425")
        self.assertEqual(probe["evidence_sha256"], "1d0d798af16e711e10c5ce1f6b16bfdc12f1cc3f33d801b0df01a5cce0ebcf3d")
        self.assertEqual(probe["tool_sha256"], "df3d88bc4b8809f5f1a20321d49408ea2b41e0770d026691487b6fec547f1212")
        self.assertEqual((probe["build"], probe["locale"]), (70450, "esES"))
        self.assertEqual((probe["root_file_data_id"], probe["group_file_data_id"]), (462695, 463228))
        self.assertIsNone(probe["skip_reason"])
        self.assertEqual(probe["source_geometry"], {"triangles": 368, "indices": 1104, "vertices": 591, "batches": 1})
        self.assertFalse(probe["skipped_source_geometry"])
        self.assertTrue(probe["unskipped_source_geometry"])
        self.assertEqual(probe["mopy_flag_histogram"], {"38": 60, "102": 308})
        self.assertEqual(probe["material_filter"], "flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)")
        self.assertEqual(probe["projection"], {"triangles": 0, "indices": 0, "vertices": 0, "batches": 1})
        self.assertEqual(probe["liquid"], {"groups": 0, "vertices": 0, "tiles": 0})
        self.assertFalse(probe["unselected_predicate"])
        self.assertEqual(probe["diagnostic"], "unselected_predicate_missing_skipped_source_geometry")
        self.assertTrue(probe["explains_v35_failure_branch"])
        self.assertFalse(probe["complete_raw_or_doodad_audit"])
        self.assertFalse(probe["complete_category_proof"])
        self.assertFalse(probe["acceptance"])

        design = self.contract["material_filtered_wmo_design"]
        self.assertEqual(design["status"], "target-not-run")
        self.assertFalse(design["contract_created"])
        self.assertFalse(design["implemented"])
        self.assertEqual(design["category"], "material_filtered_wmo")
        self.assertEqual(design["coverage_layout_version"], 9)
        self.assertFalse(design["limits_changed"])
        for condition in (
            "WMO FileDataId origin", "local_missing_m2=0", "omitted_usable=false",
            "RAW output=None", "bindings empty", "selection references empty",
            "selection sets present and nonempty", "at least one unskipped source group",
            "no skipped source groups", "every source triangle", "no liquid",
        ):
            self.assertIn(condition, design["admission_predicate"])
        self.assertFalse(design["acceptance"])
        cases = self.cases()
        self.assertEqual(cases["selection-probe-diagnostic-only"]["status"], "diagnostic-only")
        self.assertEqual(cases["material-filtered-wmo-design"]["status"], "target-not-run")
        self.assertIn("No v36 contract", cases["material-filtered-wmo-design"]["expect"])


if __name__ == "__main__":
    unittest.main()
