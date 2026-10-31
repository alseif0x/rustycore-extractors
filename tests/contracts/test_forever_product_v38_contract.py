# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 source-heavy material-filtered-WMO contract gates."""
import hashlib
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v38.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v38/all.tsv"
EXPECTED_PLAN_ROWS = [
    ["forever-product", "38"],
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
    ["collision-profile", "linux70450-source-heavy-material-filtered-wmo-vmap-product-v1"],
    ["collision-domain", "all-available-maps"],
    ["collision-coverage", "linux70450-source-heavy-material-filtered-wmo-coverage-v11"],
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
TARGET_LISTS = {
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
KEEP = "flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)"
REJECT = "flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)"


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
        raise AssertionError("PLAN rows differ from the canonical v38 fixture")


class ProductV38Contract(unittest.TestCase):
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

    def test_v38_identity_and_unchanged_canonical_synthetic_fixture(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v38", 38, "Build70450SourceHeavyMaterialFilteredWmoGeometryV1"),
        )
        self.assertEqual(
            (contract["builds"], contract["platform"], contract["backend"], contract["offline"]),
            ([70450], "linux", "rust-local", True),
        )
        self.assertEqual(contract["component_mask"], 17)
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], "linux70450-source-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["plan_profile"], "linux70450-source-heavy-material-filtered-wmo-vmap-product-v1")
        self.assertEqual(contract["plan_coverage_profile"], "linux70450-source-heavy-material-filtered-wmo-coverage-v11")
        self.assertEqual(contract["domain_profile"], "source-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["codec_profile"], contract["source_profile"])
        self.assertEqual(contract["completion"], "linux70450-available-source-heavy-material-filtered-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-source-heavy-material-filtered-wmo-verification-v1")
        self.assertEqual(
            contract["verification_scope"],
            "artifact_integrity_source_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
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
        self.assertEqual((plan["index"], plan["root"], plan["schema_catalog"]), ("contiguous", "header-50893-v2", "wowdbdefs-70450-v1"))
        self.assertEqual((plan["decryption"], plan["keys"]), ("disabled", "refused"))
        self.assertEqual(plan["partial_encrypted_sections"].split(";", 1)[0], "declared-skip")
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1048576)
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual((plan["terrain"], plan["navigation"], plan["offmesh"]), ("none", "none", "absent"))
        self.assertEqual(
            (plan["pack"], plan["big_base_unit"], plan["skip_liquids"], plan["maximum_slope"], plan["non_steep_slope"]),
            (True, False, False, 55, 55),
        )
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])
        self.assertEqual(contract["fixtures"]["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v38/all.tsv")

        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual(len(rows), 30)
        self.assertEqual(rows, EXPECTED_PLAN_ROWS)
        assert_canonical_plan_rows(rows)
        values = {row[0]: row[1] for row in rows[:-1]}
        self.assertEqual(list(values), ORDERED_KEYS)
        self.assertEqual(values["forever-product"], "38")
        self.assertEqual(values["partial_encrypted_sections"], "declared-skip")
        self.assertEqual([values[field] for field in BUDGET_FIELDS], ["1048576"] * 7)
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        invalid = [list(row) for row in rows]
        invalid[0][1] = "37"
        with self.assertRaises(AssertionError):
            assert_canonical_plan_rows(invalid)

    def test_v11_codec_identity_order_resource_and_strict_cross_version_separation(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], "vmaps/available-source-heavy-material-filtered-wmo-geometry-coverage-v11.bin")
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 11))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["resource_profile"], "linux-collision-source-heavy-resources-v1")
        self.assertEqual(codec["selection"], "MapSelection::All")
        self.assertEqual(codec["resource_profile_serialized_after"], "missing_m2")
        self.assertEqual(codec["wmo_policy_serialized_after"], "resource_profile")
        self.assertEqual(codec["missing_wmo_serialized_after"], "wmo_policy")
        self.assertEqual(codec["empty_wmo_serialized_after"], "missing_wmo")
        self.assertEqual(codec["unselected_wmo_serialized_after"], "empty_wmo")
        self.assertEqual(codec["material_filtered_wmo_serialized_after"], "unselected_wmo")
        self.assertEqual(self.contract["wmo_omissions"]["list_order"], list(TARGET_LISTS))
        self.assertTrue(self.contract["wmo_omissions"]["pairwise_disjoint"])
        self.assertEqual(codec["list_max"], 4096)
        for name in TARGET_LISTS:
            self.assertEqual(self.contract["limits"][f"{name}_max"], 4096)
        self.assertEqual(codec["strict_wrappers"]["v36"]["layout_version"], 9)
        self.assertEqual(codec["strict_wrappers"]["v36"]["contract"], "forever-product-v36")
        self.assertEqual(codec["strict_wrappers"]["v37"]["layout_version"], 10)
        self.assertEqual(codec["strict_wrappers"]["v37"]["coverage_path"], "vmaps/available-dependency-heavy-material-filtered-wmo-geometry-coverage-v10.bin")
        self.assertEqual(codec["strict_wrappers"]["v38"]["layout_version"], 11)
        self.assertEqual(codec["strict_wrappers"]["v38"]["coverage_path"], codec["path"])
        self.assertIn("v1-v10", codec["cross_profile_rejection"])
        self.assertEqual(codec["zero_omissions_still_layout_version"], 11)

    def test_source_heavy_raises_only_source_capacity_and_preserves_raw_limit(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-source-heavy-resources-v1")
        self.assertEqual(resource["collector_max_requests"], 16_000_000)
        self.assertEqual(resource["collector_max_dependency_requests"], 16_000_000)
        self.assertEqual(resource["collector_max_source_bytes"], 512 * 1024 * 1024)
        self.assertEqual(resource["collector_max_raw_bytes"], 256 * 1024 * 1024)
        self.assertEqual(resource["startup_collision_max_requests"], 16_000_000)
        self.assertEqual(resource["startup_collision_max_dependency_requests"], 16_000_000)
        self.assertEqual(resource["startup_collision_max_source_bytes"], 512 * 1024 * 1024)
        self.assertEqual(resource["startup_collision_max_raw_bytes"], 256 * 1024 * 1024)
        self.assertEqual(resource["only_capacity_raised_vs_v37"], "max_source_bytes")
        self.assertEqual(resource["max_raw_bytes_change"], "none; 256 MiB retained from DependencyHeavyV1")
        self.assertEqual(resource["v37_dependency_heavy_source_bytes"], 256 * 1024 * 1024)
        self.assertEqual(resource["v37_dependency_heavy_raw_bytes"], 256 * 1024 * 1024)
        self.assertFalse(resource["disk_world_profile_implicit"])
        self.assertEqual(resource["other_limits"], "same as v37 DependencyHeavyV1, including reads, per-file, models, spool, inventory, stage, package, list_profile and memory")
        limits = self.contract["limits"]
        self.assertFalse(limits["request_calls_max_raised_vs_v37"])
        self.assertFalse(limits["dependency_requests_max_raised_vs_v37"])
        self.assertEqual(limits["request_calls_max"], 16_000_000)
        self.assertEqual(limits["dependency_requests_max"], 16_000_000)
        self.assertEqual(limits["reads_max"], 4096)
        self.assertEqual(limits["source_bytes_max"], 512 * 1024 * 1024)
        self.assertEqual(limits["raw_bytes_max"], 256 * 1024 * 1024)
        self.assertTrue(limits["source_bytes_max_raised"])
        self.assertFalse(limits["raw_bytes_max_raised"])
        self.assertFalse(limits["reads_max_raised"])
        for field in ("models_max_raised", "memory_max_raised"):
            self.assertFalse(limits[field], field)
        self.assertIn("max_source_bytes only", limits["request_calls_change"])
        self.assertFalse(resource["disk_world_profile_implicit"])

    def test_v38_real_acceptance_failed_at_read_limit(self):
        status = self.contract["real_acceptance"]
        self.assertEqual(status["status"], "failed")
        self.assertEqual(status["target"], "linux build-70450 forever-product-v38")
        self.assertEqual(status["run_id"], "v38-source-heavy-wmo-20261026T163808Z")
        self.assertEqual(status["report_sha256"], "67a07b1b6fcbe87eb89eba504541252b26ada1295d899dd4ef5c9a3e93b4d54d")
        self.assertEqual(status["diagnostic_report_sha256"], "caf57ba9f6687544fd16d237517dae87de9fd04cb24946d57dbb2c8a86acb2ee")
        diagnostic_path = Path(status["diagnostic_report"])
        self.assertTrue(diagnostic_path.is_file(), str(diagnostic_path))
        self.assertEqual(
            hashlib.sha256(diagnostic_path.read_bytes()).hexdigest(),
            status["diagnostic_report_sha256"],
        )
        self.assertTrue(status["performed"])
        self.assertFalse(status["accepted"])
        self.assertEqual(status["inspect_exit"], 0)
        self.assertEqual(status["extract_exit"], 9)
        self.assertEqual(status["extract_error"], "collision_policy_limit")
        self.assertEqual((status["probe_exit"], status["verify_exit"], status["verify_links_exit"]), (-1, -1, -1))
        self.assertEqual(status["elapsed_seconds"], 25)
        self.assertFalse(status["network_attempted"])
        self.assertTrue(status["client_metadata_unchanged"])
        self.assertTrue(status["inputs_unchanged"])
        self.assertFalse(status["accepted_prefix"])
        self.assertIsNone(status["audit"])
        self.assertIsNone(status["geometry"])
        diagnostic = status["diagnostic"]
        self.assertEqual(diagnostic["kind"], "resource_limit")
        self.assertFalse(diagnostic["guards_triggered"])
        self.assertEqual(diagnostic["cause"], "max_reads")
        self.assertEqual(
            {name: diagnostic[name] for name in ("request_calls", "reads", "dependency_requests", "source_bytes", "raw_bytes", "models", "resident_bytes", "external_bytes", "external_source_bytes", "store_resident_bytes", "memory_limit_bytes", "tolerated_wdt", "tolerated_child")},
            {
                "request_calls": 34486,
                "reads": 4096,
                "dependency_requests": 11497,
                "source_bytes": 305392091,
                "raw_bytes": 43744216,
                "models": 2424,
                "resident_bytes": 330762107,
                "external_bytes": 270738115,
                "external_source_bytes": 11875616,
                "store_resident_bytes": 306605630,
                "memory_limit_bytes": 1073741824,
                "tolerated_wdt": 2,
                "tolerated_child": 1,
            },
        )
        self.assertEqual(status["target_lists"], TARGET_LISTS)
        self.assertFalse(status["target_lists_are_observed_output"])
        self.assertFalse(status["wmo_462695_admission_proven"])
        self.assertFalse(status["full_world_coverage"])
        self.assertFalse(status["runtime_claim"])
        self.assertIn("max_reads=4096", status["note"])
        self.assertIn("were not the cause", status["note"])
        software = status["software_evidence"]
        self.assertEqual((software["suites"], software["tests_passed"], software["ignored"], software["contracts_passed"]), (82, 4479, 26, 535))
        self.assertEqual(software["software_sha256"], "e82599a49f1a393023eb782956946deb4af117bef33cd82cb5cf44a4728b0314")
        self.assertEqual(software["binary_sha256"], "1e49c7cba7c725fb0c666c5da7a19199213dec4bfec0f2e2351db19f825195c1")

    def test_prior_setup_attempt_is_not_product_failure_or_capacity_evidence(self):
        attempt = self.contract["prior_setup_attempt"]
        self.assertEqual(attempt["status"], "invalid setup attempt; not product failure or capacity evidence")
        self.assertEqual(attempt["run_id"], "v38-source-heavy-wmo-20261026T150518Z")
        self.assertEqual(attempt["report_sha256"], "d180720a5800c2531bb76a45dc43ee86c005dad97749d95dd8e627e769f4df31")
        binary_sha256 = attempt["binary_sha256"]
        self.assertRegex(binary_sha256, r"^[0-9a-f]{64}$")
        setup_report_path = Path(
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/"
            "v38-source-heavy-wmo-20261026T150518Z-acceptance.json"
        )
        self.assertTrue(setup_report_path.is_file(), str(setup_report_path))
        setup_report_bytes = setup_report_path.read_bytes()
        self.assertEqual(
            hashlib.sha256(setup_report_bytes).hexdigest(), attempt["report_sha256"]
        )
        setup_report = json.loads(setup_report_bytes)
        self.assertEqual(setup_report["software"]["binary_sha256"], binary_sha256)
        self.assertEqual(attempt["inspect_exit"], 2)
        self.assertEqual(attempt["inspect_error"], "product_plan_version")
        self.assertEqual(attempt["extract_exit"], -1)
        self.assertFalse(attempt["valid_acceptance_run"])
        self.assertIn("not a v38 product failure", attempt["note"])

    def test_v39_read_heavy_follow_up_is_design_only(self):
        design = self.contract["follow_up_design"]
        self.assertEqual(design["status"], "design-only; target-not-run")
        self.assertEqual(design["profile_name"], "ReadHeavyV1")
        self.assertEqual(design["only_capacity_raised_vs_v38"], "max_reads")
        self.assertEqual((design["v38_max_reads"], design["planned_max_reads"]), (4096, 1_000_000))
        self.assertEqual(
            design["preserved_limits"],
            {
                "max_requests": 16_000_000,
                "max_dependency_requests": 16_000_000,
                "max_source_bytes": 536_870_912,
                "max_raw_bytes": 268_435_456,
            },
        )
        self.assertEqual(design["all_other_limits"], "unchanged from SourceHeavyV1")
        self.assertEqual(design["planned_coverage_layout_version"], 12)
        self.assertEqual(design["acceptance_status"], "target-not-run")
        self.assertFalse(design["sufficiency_proven"])
        self.assertFalse(design["wmo_462695_admission_proven"])
        self.assertIn("no v39 implementation", design["note"])
        profile = self.contract["profile_change"]
        self.assertEqual(profile["status"], "implemented; real target failed at max_reads")
        self.assertEqual(profile["profile_name"], "SourceHeavyV1")
        self.assertEqual(profile["only_capacity_raised_vs_v37"], "max_source_bytes")
        self.assertEqual(profile["max_source_bytes"], 512 * 1024 * 1024)
        self.assertEqual(profile["max_raw_bytes"], 256 * 1024 * 1024)
        self.assertEqual(profile["preserved_request_limits"], {"max_requests": 16_000_000, "max_dependency_requests": 16_000_000})
        self.assertEqual(profile["coverage_layout_version"], 11)
        self.assertEqual(profile["synthetic_plan_budgets_bytes"], {name: 1048576 for name in ("raw", "wdt", "cache", "placement", "vmo", "map", "output")})
        self.assertIn("not target capacity evidence", profile["fixture_budget_effect"])
        self.assertIn("max_raw_bytes by the PLAN raw_bytes budget", profile["geometry_budget_effect"])
        self.assertFalse(profile["sufficiency_proven"])
        self.assertFalse(profile["wmo_462695_admission_proven"])


    def test_material_filtered_predicates_lists_and_synthetic_acceptance_matrix(self):
        omission = self.contract["wmo_omissions"]
        self.assertEqual(omission["canonical"], "each list is ordered, unique, nonzero, and bounded to 4096 IDs")
        predicate = omission["material_filtered_wmo"]
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
        self.assertEqual(material_filter["keep_predicate"], KEEP)
        self.assertEqual(material_filter["reject_predicate"], REJECT)
        self.assertEqual(material_filter["examples"], {"rejected": ["0x26", "0x66"], "kept": ["0x20", "0x08"]})
        keep = lambda flags: bool(flags & 0x08) or bool(flags & 0x20) and not bool(flags & 0x04)
        reject = lambda flags: not bool(flags & 0x08) and (not bool(flags & 0x20) or bool(flags & 0x04))
        for flags in range(256):
            self.assertNotEqual(keep(flags), reject(flags), f"predicates differ at {flags:#04x}")
        self.assertEqual(predicate["liquid"], "no liquid groups, vertices, or tiles")
        self.assertEqual(TARGET_LISTS, {"missing_m2": [970793], "missing_wmo": [], "empty_wmo": [], "unselected_wmo": [], "material_filtered_wmo": [462695]})
        self.assertEqual(MIXED_LISTS["unselected_wmo"], [462695])
        self.assertEqual(MIXED_LISTS["material_filtered_wmo"], [462701])
        self.assertTrue(set(MIXED_LISTS["unselected_wmo"]).isdisjoint(MIXED_LISTS["material_filtered_wmo"]))
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        for name in (
            "canonical-profile-and-plan", "synthetic-plan-fixture", "coverage-codec-v11",
            "five-way-mixed-omissions", "material-filtered-only", "normal-results-preserved",
            "predicate-rejections", "production-verifier-attacks", "list-limits",
            "lifecycle-and-cross-version", "real-acceptance",
        ):
            self.assertIn(name, cases)
        for item in ("missing_m2=[970793]", "missing_wmo=[462694]", "empty_wmo=[462697]", "unselected_wmo=[462695]", "material_filtered_wmo=[462701]"):
            self.assertIn(item, cases["five-way-mixed-omissions"]["expect"])
        for item in ("missing_m2=[970793]", "missing_wmo=[]", "empty_wmo=[]", "unselected_wmo=[]", "material_filtered_wmo=[462695]", "0x26/0x66", "0x20/0x08"):
            self.assertIn(item, cases["material-filtered-only"]["expect"])
        self.assertIn("all ten pairwise", cases["production-verifier-attacks"]["expect"])
        self.assertIn("accepts 4096 and rejects 4097", cases["list-limits"]["expect"])
        self.assertEqual(cases["real-acceptance"]["status"], "failed")
        self.assertEqual(self.contract["limits"]["reads_max"], 4096)
        self.assertFalse(self.contract["real_acceptance"]["target_lists_are_observed_output"])
        self.assertFalse(self.contract["real_acceptance"]["wmo_462695_admission_proven"])
        self.assertFalse(self.contract["package"]["omitted_wmo_artifacts"])
        self.assertFalse(self.contract["package"]["omitted_wmo_vmos_bindings_names_spawns_placements_and_ids"])
        self.assertTrue(self.contract["package"]["present_collision_and_doodad_outputs_preserved"])
        self.assertEqual(self.contract["verification"]["coverage_authentication"], "independently_derived")
        self.assertFalse(self.contract["verification"]["full_world_coverage"])
        self.assertFalse(self.contract["verification"]["runtime_claim"])
        self.assertFalse(self.contract["verification"]["full_extractor_complete"])
        self.assertEqual(self.contract["verification"]["material_filtered_wmo_authentication"], "producer_reported")

    def test_v37_failure_evidence_is_prior_context_not_v38_acceptance(self):
        prior = self.contract["prior_v37_failure_evidence"]
        self.assertEqual(prior["status"], "failed; prior evidence only, not v38 acceptance")
        self.assertEqual(prior["target"], "linux build-70450 forever-product-v37")
        self.assertEqual(prior["run_id"], "v37-dependency-heavy-wmo-20261026T120824Z")
        self.assertEqual(prior["report_sha256"], "e86bf2370e84897b74b6df7ac3c6fb42fa508fccd67d763b6c72181804a1e552")
        self.assertEqual(prior["extract_error"], "local_store_policy_limit")
        self.assertEqual(prior["diagnostic_report_sha256"], "a5ae7cf86a3613359bdf67c956377b26d5b0367a1707397e032cb2b4600c55a5")
        diagnostic = prior["diagnostic"]
        self.assertEqual(diagnostic["cause"], "max_source_bytes")
        self.assertEqual(diagnostic["source_bytes"], 268259239)
        self.assertEqual(diagnostic["max_source_bytes"], 268435456)
        self.assertEqual(diagnostic["source_bytes_remaining"], 176217)
        self.assertEqual(diagnostic["declared_bytes"], 407338)
        self.assertEqual(diagnostic["isolated_group_parse"], "parsed correctly")
        self.assertTrue(diagnostic["memory_ruled_out_for_this_failure"])
        self.assertTrue(diagnostic["parser_ruled_out_for_this_failure"])
        self.assertIn("does not demonstrate admission of WMO 462695", prior["scope_note"])
        self.assertIn("does not demonstrate", prior["scope_note"])
        self.assertTrue(self.contract["real_acceptance"]["performed"])
        setup = self.contract["prior_setup_attempt"]
        self.assertFalse(setup["valid_acceptance_run"])
        design = self.contract["follow_up_design"]
        self.assertEqual(design["profile_name"], "ReadHeavyV1")
        self.assertEqual(design["planned_max_reads"], 1_000_000)

    def test_v36_failure_is_prior_evidence_not_v38_acceptance(self):
        prior = self.contract["prior_evidence"]
        self.assertEqual(prior["v36_acceptance_status"], "failed")
        self.assertEqual(prior["v36_run_id"], "v36-material-filtered-wmo-20261026T085714Z")
        self.assertEqual(prior["v36_report_sha256"], "dfcca8f3c29b655d5d3a0a2e9794cad7d2484bbf2edb25e29bbe9eefff039e03")
        self.assertEqual(prior["v36_extract_error"], "collision_policy_limit")
        self.assertEqual(
            (prior["v36_dependency_requests"], prior["v36_request_calls"], prior["v36_models"]),
            (4096, 19243, 2118),
        )
        self.assertFalse(prior["v36_target_admission_proven"])
        self.assertFalse(prior["acceptance_transferred_to_v37"])
        self.assertFalse(prior["acceptance_transferred_to_v38"])
        for name in ("real_extraction", "full_world_coverage", "complete_vmap_component", "runtime_acceptance", "distribution", "release_publication", "full_extractor_complete"):
            self.assertFalse(self.contract["boundary"][name], name)
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
