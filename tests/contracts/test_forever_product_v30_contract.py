# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 available ROOT-omission geometry contract; synthetic only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v30.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v30/all.tsv"
ORDERED_KEYS = [
    "forever-product",
    "build",
    "locale",
    "index",
    "root",
    "schema-catalog",
    "decryption",
    "partial_encrypted_sections",
    "collision-storage",
    "collision-raw-bytes",
    "collision-wdt-bytes",
    "collision-cache-bytes",
    "collision-placement-bytes",
    "collision-vmo-bytes",
    "collision-map-bytes",
    "collision-output-bytes",
    "collision-profile",
    "collision-domain",
    "collision-coverage",
    "gameobject-scope",
    "model-scope",
    "components",
    "terrain",
    "pack",
    "navigation",
    "big-base-unit",
    "skip-liquids",
    "maximum-slope",
    "non-steep-slope",
]
BUDGET_FIELDS = [
    "collision-raw-bytes",
    "collision-wdt-bytes",
    "collision-cache-bytes",
    "collision-placement-bytes",
    "collision-vmo-bytes",
    "collision-map-bytes",
    "collision-output-bytes",
]
SOURCE_PROFILE = "linux70450-available-map-root-omissions-v1"
COVERAGE_PROFILE = "linux70450-available-map-root-omissions-coverage-v3"
COMPLETION_PROFILE = "linux70450-available-map-root-omissions-collision-v1"
VERIFICATION_PROFILE = "linux70450-available-map-root-omissions-verification-v1"
COVERAGE_PATH = "vmaps/available-root-absent-geometry-coverage-v3.bin"
DECLARE_ROOT_ABSENT = "run::MissingWdtPolicy::DeclareRootAbsent"


class ProductV30Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_and_exact_plan_surface(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v30", 30, "Build70450AvailableRootAbsentGeometryV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(
            (contract["platform"], contract["backend"], contract["offline"]),
            ("linux", "rust-local", True),
        )
        self.assertEqual(
            contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"]
        )
        self.assertEqual(contract["source_profile"], SOURCE_PROFILE)
        self.assertEqual(contract["completion"], COMPLETION_PROFILE)

        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], ORDERED_KEYS)
        self.assertEqual(
            plan["trailing_db2_rows"], "exactly one final row: db2<TAB>Map<TAB>1349477"
        )
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "vmaps"], 17))
        self.assertEqual(
            plan["component_bits"],
            {"db2": 1, "gt": 2, "cameras": 4, "maps": 8, "vmaps": 16, "mmaps": 32},
        )
        self.assertEqual((plan["decryption"], plan["keys"]), ("disabled", "refused"))
        self.assertIn("declared-skip", plan["partial_encrypted_sections"])
        self.assertIn("PLAN only", plan["partial_encrypted_sections"])
        self.assertIn("between decryption and collision-storage", plan["partial_encrypted_sections"])
        self.assertEqual(plan["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(plan["storage_budget_fields"], BUDGET_FIELDS)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertTrue(plan["extract_resume_require_scratch"])
        self.assertTrue(plan["inspect_verify_verify_links_forbid_scratch"])
        self.assertEqual(
            plan["collision_profile"],
            "linux70450-available-map-root-omissions-vmap-product-v1",
        )
        self.assertEqual(plan["collision_domain"], "all-available-maps")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertIn("no requested_map or substitute", plan["requested_map"])
        self.assertIn("zero is not an all-map sentinel", plan["requested_map_zero"])
        self.assertIn("MapAndAncestors(0) is not MapSelection::All", plan["map_and_ancestors_zero"])
        self.assertEqual(plan["domain_closure"], "available-map-root-omissions-v1")
        self.assertEqual(plan["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual(plan["db2_entries"], "exactly one canonical Map DB2 entry; no other table")
        self.assertEqual(
            plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1}
        )
        self.assertEqual(
            (plan["terrain"], plan["navigation"], plan["offmesh"]),
            ("none", "none", "absent"),
        )
        self.assertTrue(plan["pack"])
        self.assertFalse(plan["big_base_unit"])
        self.assertFalse(plan["skip_liquids"])
        self.assertEqual((plan["maximum_slope"], plan["non_steep_slope"]), (55, 55))
        self.assertTrue(plan["all_available_maps_selection"])
        self.assertFalse(plan["inspect_requires_client"])

    def test_fixture_is_canonical_synthetic_eses_with_only_map(self):
        metadata = self.contract["fixtures"]
        self.assertEqual(metadata["classification"], "canonical synthetic PLAN fixtures only")
        self.assertFalse(metadata["contains_client_data"])
        self.assertFalse(metadata["real_extraction_evidence"])
        self.assertFalse(metadata["certified_real_storage_budgets"])
        self.assertEqual(metadata["synthetic_budget_bytes"], 1048576)
        self.assertIn("synthetic parser fixtures only", metadata["budget_note"])
        self.assertIn("not real acceptance budgets", metadata["budget_note"])
        self.assertIn("certified for real extraction", metadata["budget_note"])
        self.assertEqual(len(metadata["cases"]), 1)
        case = metadata["cases"][0]
        self.assertEqual(case["path"], "tests/contracts/fixtures/forever-product-v30/all.tsv")
        self.assertEqual(
            (case["locale"], case["collision_domain"], case["map_selection"]),
            ("esES", "all-available-maps", "MapSelection::All"),
        )
        self.assertTrue(case["canonical_fixture"])
        self.assertFalse(case["real_evidence"])
        self.assertFalse(case["certified_real_budget"])

        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual(len(rows), len(ORDERED_KEYS) + 1)
        self.assertEqual([row[0] for row in rows[:-1]], ORDERED_KEYS)
        self.assertTrue(all(len(row) == 2 for row in rows[:-1]))
        self.assertEqual(rows[-1], ["db2", "Map", "1349477"])
        fields = {row[0]: row[1] for row in rows[:-1]}
        expected = {
            "forever-product": "30",
            "build": "70450",
            "locale": "esES",
            "index": "contiguous",
            "root": "header-50893-v2",
            "schema-catalog": "wowdbdefs-70450-v1",
            "decryption": "disabled",
            "partial_encrypted_sections": "declared-skip",
            "collision-storage": "linux-leased-append-disk-world-v1",
            "collision-profile": "linux70450-available-map-root-omissions-vmap-product-v1",
            "collision-domain": "all-available-maps",
            "collision-coverage": COVERAGE_PROFILE,
            "gameobject-scope": "excluded-by-profile-v1",
            "model-scope": "placement-only-v1",
            "components": "db2,vmaps",
            "terrain": "none",
            "pack": "true",
            "navigation": "none",
            "big-base-unit": "false",
            "skip-liquids": "false",
            "maximum-slope": "55",
            "non-steep-slope": "55",
        }
        expected.update({name: "1048576" for name in BUDGET_FIELDS})
        self.assertEqual(fields, expected)
        self.assertNotIn("partial_encrypted_tables", fields)
        self.assertNotIn("requested_map", fields)
        partial = ORDERED_KEYS.index("partial_encrypted_sections")
        self.assertEqual(ORDERED_KEYS[partial - 1], "decryption")
        self.assertEqual(ORDERED_KEYS[partial + 1], "collision-storage")

    def test_manifest_and_present_vs_root_absent_artifact_rules(self):
        package = self.contract["package"]
        self.assertEqual(package["component_order"], ["db2", "vmaps"])
        self.assertEqual(package["db2_map"], "dbc/<locale>/Map.db2")
        self.assertIn("v26/v27 per-table", package["partial_encrypted_tables"])
        self.assertIn("including Map when partial", package["partial_encrypted_tables"])
        self.assertIn("lookup, frame_ordinal, decoded_size", package["partial_encrypted_tables"])
        self.assertIn("forbidden", package["partial_encrypted_sections"])
        self.assertIn("PLAN gate remains mandatory", package["partial_encrypted_sections"])
        self.assertIn("forbidden", package["global_partial_encrypted_sections"])
        self.assertIn("no v25-style global", package["global_partial_encrypted_sections"])
        self.assertIn("Only per-table", package["declaration_boundary"])
        self.assertIn("Resume rejects recovered global declarations and foreign tables", package["declaration_boundary"])
        self.assertEqual(package["coverage"], COVERAGE_PATH)
        self.assertEqual(package["completion"], COMPLETION_PROFILE)
        self.assertTrue(package["tree_required_for_every_present_map"])
        self.assertTrue(package["placement_proof_required_for_every_present_map"])
        self.assertTrue(package["tiles_only_for_present_maps"])
        self.assertFalse(package["artifacts_for_root_absent"])
        self.assertTrue(package["maps_equals_wdt_files_plus_root_absent"])
        self.assertTrue(package["package_files_include_map_and_coverage"])
        self.assertIn("distinct from ROOT-absent", package["present_empty_maps"])
        self.assertIn("remains in Coverage.maps", package["root_absent_maps"])
        self.assertIn("no tree, placement proof, tile, model", package["root_absent_maps"])
        self.assertIn("not an empty present map", package["root_absent_maps"])
        self.assertFalse(package["terrain_outputs"])
        self.assertFalse(package["navigation_outputs"])
        self.assertFalse(package["other_components"])
        self.assertTrue(package["provisional_until_sealed"])
        resume = self.cases()["per-table-manifest-and-resume"]["expect"]
        self.assertIn("global or foreign-table declarations", resume)
        self.assertIn("new scratch", resume)
        self.assertIn("reproduces the baseline", resume)

    def test_limits_preserve_v22_and_include_map_and_coverage_in_package_cap(self):
        limits = self.contract["limits"]
        v22 = json.loads((ROOT / "contracts/forever-product-contract-v22.json").read_bytes())
        for name, value in v22["limits"].items():
            if name != "meaning":
                with self.subTest(v22_limit=name):
                    self.assertEqual(limits[name], value)
        self.assertFalse(limits["v22_ceilings_raised"])
        self.assertEqual(limits["assembled_domain_bytes_max"], 128 * 1024**2)
        self.assertEqual(limits["staged_placement_bytes_max"], 128 * 1024**2)
        self.assertEqual(limits["staged_vmo_bytes_max"], 384 * 1024**2)
        self.assertEqual(limits["coverage_maps_max"], 4096)
        self.assertEqual(limits["coverage_artifacts_max"], 65535)
        self.assertEqual(limits["package_files_max"], 65536)
        self.assertIn("includes Map.db2 and RCVGEO1 coverage", limits["package_files_scope"])
        self.assertIn("simultaneously", limits["package_files_scope"])
        self.assertEqual((limits["min_db2_entries"], limits["max_db2_entries"]), (1, 1))
        self.assertEqual(limits["required_map_entries"], 1)
        self.assertEqual(limits["db2_file_bytes"], 16 * 1024**2)
        self.assertEqual(limits["db2_total_bytes"], 17_179_869_184)
        self.assertEqual(limits["partial_frames_per_table"], 65_536)
        self.assertEqual(limits["partial_frames_total"], 18_576)
        self.assertIn("including Map", limits["partial_frames_scope"])
        self.assertIn("CASC max_blte_frames", limits["partial_frames_scope"])
        self.assertIn("exactly tile", limits["partial_frame_coverage"])
        self.assertEqual(limits["package_total_bytes"], "collision-output-bytes + 17179869184")
        self.assertIn(
            "checked_add(collision-output-bytes, 17179869184)",
            limits["package_total_bytes_equation"],
        )
        self.assertIn("overflow rejects", limits["package_total_bytes_equation"])
        self.assertEqual(limits["package_total_bytes_max"], 85_899_345_920)
        self.assertEqual(
            limits["package_total_bytes_max"],
            limits["collision_output_bytes_max"] + limits["db2_total_bytes"],
        )
        self.assertFalse(limits["client_bytes_in_reports_or_repository"])
        boundaries = self.cases()["resource-boundaries"]["expect"]
        for text in (
            "128 MiB aggregate",
            "128 MiB placements",
            "384 MiB models",
            "4096 maps",
            "65535 coverage artifacts",
            "65536 package files including Map+coverage",
            "checked_add(collision-output-bytes, 17179869184)",
        ):
            self.assertIn(text, boundaries)

    def test_rcvgeo1_v3_policy_root_states_and_strict_wrappers(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], COVERAGE_PATH)
        self.assertEqual((codec["magic"], codec["encoded_magic"]), ("RCVGEO1", "RCVGEO1\\0"))
        self.assertEqual(codec["layout_version"], 3)
        self.assertEqual(codec["profile"], SOURCE_PROFILE)
        self.assertEqual(codec["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(codec["requested_map"], "absent")
        self.assertEqual(codec["requested_map_substitute"], "forbidden")
        self.assertFalse(codec["requested_map_zero_allowed"])
        self.assertFalse(codec["map_and_ancestors_zero_allowed"])
        self.assertEqual(codec["selection"], "MapSelection::All")
        self.assertTrue(codec["selection_does_not_determine_version"])
        self.assertEqual(codec["zero_omissions_still_layout_version"], 3)
        self.assertEqual(codec["domain_closure"], "available-map-root-omissions-v1")
        self.assertEqual(codec["missing_wdt_policy"], DECLARE_ROOT_ABSENT)
        self.assertEqual(
            codec["missing_wdt_policy_variants"],
            ["run::MissingWdtPolicy::Reject", DECLARE_ROOT_ABSENT],
        )
        self.assertTrue(codec["policy_discriminant_required"])
        self.assertIn("0 means Present", codec["domain_map_fact_root_absent"])
        self.assertIn("1 means RootAbsent", codec["domain_map_fact_root_absent"])
        self.assertIn("complete active domain", codec["maps"])
        self.assertIn("maps=wdt_files+root_absent", codec["maps"])
        self.assertTrue(codec["canonical_roundtrip_required"])

        wrappers = codec["strict_wrappers"]
        expected = (
            ("v22", "forever-product-v22", 1, 16, "linux70450-selected-map-geometry-v1"),
            ("v28", "forever-product-v28", 1, 17, "linux70450-selected-map-geometry-v1"),
            ("v29", "forever-product-v29", 2, 17, "linux70450-available-map-geometry-v1"),
            ("v30", "forever-product-v30", 3, 17, SOURCE_PROFILE),
        )
        for name, contract, version, mask, profile in expected:
            with self.subTest(wrapper=name):
                wrapper = wrappers[name]
                self.assertEqual(wrapper["contract"], contract)
                self.assertEqual(wrapper["layout_version"], version)
                self.assertEqual(wrapper["component_mask"], mask)
                self.assertEqual(wrapper["profile"], profile)
                self.assertEqual(
                    wrapper["selection"],
                    "MapSelection::MapAndAncestors(requested_map)" if name in ("v22", "v28") else "MapSelection::All",
                )
        self.assertEqual(wrappers["v22"]["coverage_authentication"], "producer_reported")
        for name in ("v28", "v29", "v30"):
            self.assertEqual(wrappers[name]["coverage_authentication"], "independently_derived")
        self.assertEqual(wrappers["v30"]["missing_wdt_policy"], DECLARE_ROOT_ABSENT)
        self.assertEqual(wrappers["v30"]["root_absence_authentication"], "producer_reported")
        self.assertEqual(wrappers["v29"]["coverage_path"], "vmaps/available-geometry-coverage-v2.bin")
        self.assertEqual(wrappers["v30"]["coverage_path"], COVERAGE_PATH)
        self.assertIn("does not permit crossing", wrappers["shared_mask17"])
        self.assertIn("missing-WDT policy", wrappers["shared_mask17"])
        self.assertIn("publication profile", wrappers["shared_mask17"])
        self.assertIn("requested_map=0", wrappers["zero_selection_aliases"])
        self.assertIn("MapSelection::MapAndAncestors(0)", wrappers["zero_selection_aliases"])
        self.assertEqual(codec["context"]["build"], 70450)
        self.assertEqual(codec["context"]["component_mask"], 17)
        self.assertIn("RCVGEO1 version 3", codec["context"]["binding"])
        self.assertIn("v29/All remains v2", codec["context"]["binding"])
        self.assertIn("zero omissions", codec["context"]["binding"])

    def test_root_admission_offline_boundary_and_parent_replay(self):
        source = self.contract["source_policy"]
        self.assertIn("MissingFile followed by a guarded raw-ROOT query proving zero rows", source["root_absence_admission"])
        self.assertIn("MissingFile alone", source["root_absence_admission"])
        self.assertIn("Present ROOT rows remain present even when ineligible", source["root_absence_admission"])
        for text in (
            "ROOT present but ineligible",
            "CKey central absent",
            "encoding absent or unverified",
            "corruption",
            "missing or malformed ADT",
            "malformed WDT",
            "input guard changes",
        ):
            self.assertTrue(any(text in value for value in source["fatal_not_root_absence"]), text)
        self.assertIn("Skip WDT parser", source["root_absent_effects"])
        self.assertIn("do not create SourceFacts", source["root_absent_effects"])
        self.assertIn("report.maps includes the entire active domain", source["present_effects"])
        self.assertIn("report.wdt_files counts present WDT", source["present_effects"])
        self.assertIn("immediate parent is ROOT-absent", source["parent_replay"])
        self.assertIn("do not skip to a populated grandparent", source["parent_replay"])
        self.assertIn("child with its own WDT and payload continues", source["parent_replay"])
        self.assertIn("producer_reported", source["root_absence_authentication"])
        self.assertIn("offline verification cannot independently reproduce", source["root_absence_authentication"])

        verification = self.contract["verification"]
        self.assertEqual(verification["profile"], VERIFICATION_PROFILE)
        self.assertEqual(
            verification["report_scope"],
            "artifact_integrity_available_map_geometry_root_omissions_and_independently_derived_map_vmap_domain_coverage",
        )
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertEqual(verification["root_absence_authentication"], "producer_reported")
        self.assertEqual(verification["source_authentication_flags"], "producer_reported")
        self.assertFalse(verification["root_absence_independently_proven"])
        self.assertIn("cannot prove that a WDT had zero raw ROOT rows", verification["root_absence_offline_limit"])
        self.assertEqual(verification["map_reader"], "read_vmap_maps_70450")
        self.assertEqual(verification["domain_builder"], "Maps::from_materialized_selection")
        self.assertEqual(verification["map_selection"], "MapSelection::All")
        self.assertEqual(verification["missing_wdt_policy"], DECLARE_ROOT_ABSENT)
        self.assertTrue(verification["maps_contains_entire_active_domain"])
        self.assertTrue(verification["maps_equals_wdt_files_plus_root_absent"])
        self.assertTrue(verification["tree_required_for_every_present_map"])
        self.assertTrue(verification["placement_proof_required_for_every_present_map"])
        self.assertFalse(verification["artifacts_for_root_absent"])
        self.assertFalse(verification["parent_only_through_root_absent"])
        self.assertFalse(verification["grandparent_substitution_for_root_absent_parent"])
        self.assertEqual(verification["zero_omissions_still_layout_version"], 3)
        derived = " ".join(verification["independently_derived_facts"])
        for text in ("Map section", "direct/copy/materialized", "complete materialized graph", "present-map tree/proof/tile/model", "maps=wdt_files+root_absent"):
            self.assertIn(text, derived)
        reported = " ".join(verification["producer_reported_facts"])
        self.assertIn("raw ROOT absence", reported)
        self.assertIn("CKey/EKey", reported)
        self.assertFalse(verification["independent_cryptographic_source_authentication"])
        self.assertFalse(verification["client_reads"])

        coverage = self.contract["coverage"]
        self.assertEqual(coverage["profile"], COVERAGE_PROFILE)
        self.assertEqual(coverage["path"], COVERAGE_PATH)
        self.assertEqual(coverage["missing_wdt_policy"], DECLARE_ROOT_ABSENT)
        self.assertEqual(coverage["root_absence_authentication"], "producer_reported")
        self.assertTrue(coverage["maps_equals_wdt_files_plus_root_absent"])
        self.assertTrue(coverage["present_empty_maps_have_tree_and_proof"])
        self.assertFalse(coverage["artifacts_for_root_absent"])
        self.assertEqual(coverage["zero_omissions_still_layout_version"], 3)
        self.assertFalse(coverage["complete_global_map_coverage_claim"])
        self.assertFalse(coverage["complete_vmap_component_claim"])
        self.assertFalse(coverage["full_world_coverage"])

    def test_exclusions_and_all_non_claim_boundaries(self):
        contract = self.contract
        source = contract["source_policy"]
        self.assertIn("GODI and global GameObject startup are excluded", source["gameobjects"])
        self.assertIn("No GameObjectDisplayInfo request", source["gameobjects"])
        self.assertIn("present-map placement requests", source["models"])
        self.assertIn("exact used_models/VMO inventory", source["models"])
        self.assertIn("LiquidObject is out of scope", source["liquids"])
        self.assertIn("no decryption or key material", source["partial_sections"])
        for excluded in (
            "MissingFile without guarded raw ROOT zero-row proof",
            "ROOT present ineligible",
            "central/encoding absent",
            "missing ADT or model as ROOT absence",
            "WDT==0",
            "artifacts for ROOT-absent maps",
            "parent-only inheritance through ROOT-absent",
            "grandparent",
            "GODI/global GameObject",
            "LiquidObject",
            "terrain and MAPS output",
            "navigation and MMaps",
            "full-world",
            "complete VMap component",
            "global or foreign partial-section manifest declarations",
            "requested_map=0",
            "MapSelection::MapAndAncestors(0)",
            "release publication",
        ):
            self.assertTrue(any(excluded in value for value in contract["excluded"]), excluded)
        for name in (
            "decryption",
            "key_discovery_or_use",
            "complete_db2_content",
            "whole_client_map_coverage",
            "full_world_coverage",
            "complete_vmap_component",
            "root_absence_as_independent_offline_proof",
            "missing_file_as_sufficient_root_proof",
            "fatal_source_failures_as_root_absence",
            "artifacts_for_root_absent",
            "parent_only_through_root_absent",
            "grandparent_substitution_for_root_absent_parent",
            "requested_map_zero_as_all",
            "map_and_ancestors_zero_as_all",
            "global_gameobjects",
            "liquid_object",
            "terrain",
            "navigation_or_mmaps",
            "global_or_foreign_partial_manifest_declarations",
            "source_flags_as_independent_cryptographic_proof",
            "full_world_or_runtime_acceptance",
            "runtime_acceptance",
            "format_parity",
            "windows",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["boundary"][name], name)
        for name in (
            "complete_db2_content_claim",
            "whole_client_map_coverage",
            "full_world_coverage",
            "complete_vmap_component",
            "runtime_claim",
            "format_parity_claim",
            "release_publication_claim",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["verification"][name], name)
        for name in (
            "real_extraction",
            "runtime_acceptance",
            "full_world_coverage",
            "complete_vmap_component",
            "format_parity",
            "mmaps",
            "windows",
            "release_publication",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["unqualified_status"][name], name)
        self.assertFalse(contract["full_extractor_complete"])

    def test_real_acceptance_failed_at_model_and_previous_versions_remain_separate(self):
        target = self.cases()["real-acceptance"]
        self.assertEqual(target["status"], "failed")
        self.assertIn("no final package or acceptance", target["expect"])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v30")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        evidence = real["evidence"]
        self.assertEqual(evidence["run_id"], "v30-root-absent-geometry-20261024T193255Z")
        self.assertEqual(
            evidence["report_sha256"],
            "47b3057c3f6be62bdeca22af4da5a4807ad031ac5c43c32f126543e8621bd17e",
        )
        self.assertEqual((evidence["inspect_exit"], evidence["extract_exit"]), (0, 9))
        self.assertEqual((evidence["verify"], evidence["verify_links"]), ("not-run", "not-run"))
        self.assertEqual(evidence["tolerated_wdt_root_absences"], 2)
        self.assertEqual(evidence["failed_model_file_data_id"], 970793)
        self.assertEqual(evidence["failure_phase"], "models/child")
        self.assertIn("No final package or acceptance", real["note"])
        self.assertFalse(real["prior_v28_or_v29_evidence_transferred"])
        self.assertTrue(self.contract["existing_evidence"]["v29_attempts_are_separate"])
        self.assertTrue(self.contract["existing_evidence"]["v29_failure_motivates_policy_but_is_not_acceptance"])
        self.assertFalse(self.contract["existing_evidence"]["v28_acceptance_transferred_to_v30"])
        self.assertFalse(self.contract["existing_evidence"]["v29_attempts_transferred_to_v30"])

        legacy = self.contract["legacy_preservation"]
        self.assertEqual(legacy["v22_component_mask"], 16)
        self.assertEqual(legacy["v22_coverage_layout_version"], 1)
        self.assertEqual(legacy["v28_component_mask"], 17)
        self.assertEqual(legacy["v28_coverage_layout_version"], 1)
        self.assertEqual(legacy["v29_component_mask"], 17)
        self.assertEqual(legacy["v29_coverage_layout_version"], 2)
        self.assertTrue(legacy["v22_v28_v29_plan_grammar_unchanged"])
        self.assertEqual(legacy["v22_v28_v29_missing_wdt_policy"], "run::MissingWdtPolicy::Reject")
        self.assertFalse(legacy["shared_mask17_permits_profile_or_selection_crossing"])
        self.assertFalse(legacy["prior_acceptance_transferred_to_v30"])

        v22 = json.loads((ROOT / "contracts/forever-product-contract-v22.json").read_bytes())
        v28 = json.loads((ROOT / "contracts/forever-product-contract-v28.json").read_bytes())
        v29 = json.loads((ROOT / "contracts/forever-product-contract-v29.json").read_bytes())
        self.assertEqual(v22["plan"]["component_mask"], 16)
        self.assertEqual(v28["coverage_codec"]["layout_version"], 1)
        self.assertEqual(v29["coverage_codec"]["layout_version"], 2)
        self.assertNotEqual(v29["publication_profile"], self.contract["publication_profile"])
        self.assertNotEqual(v28["publication_profile"], self.contract["publication_profile"])
        self.assertNotEqual(v22["publication_profile"], self.contract["publication_profile"])


if __name__ == "__main__":
    unittest.main()
