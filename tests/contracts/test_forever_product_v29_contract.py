# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 authenticated all-active-map geometry contract; synthetic only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v29.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v29/all.tsv"
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
SOURCE_PROFILE = "linux70450-available-map-geometry-v1"
COVERAGE_PROFILE = "linux70450-available-map-geometry-coverage-v2"
COMPLETION_PROFILE = "linux70450-available-map-geometry-collision-v1"
AVAILABLE_COVERAGE_PATH = "vmaps/available-geometry-coverage-v2.bin"


class ProductV29Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_and_exact_plan_surface(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v29", 29, "Build70450AuthenticatedAvailableGeometryV1"),
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
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
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
            "linux70450-authenticated-available-map-geometry-vmap-product-v1",
        )
        self.assertEqual(plan["collision_domain"], "all-available-maps")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertIn("no requested_map or substitute", plan["requested_map"])
        self.assertIn("zero is not an all-map sentinel", plan["requested_map_zero"])
        self.assertIn("MapAndAncestors(0) is not MapSelection::All", plan["map_and_ancestors_zero"])
        self.assertEqual(plan["domain_closure"], "available-map-all-active-v1")
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

    def test_fixture_is_canonical_synthetic_eses_all_with_only_map(self):
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
        self.assertEqual(case["path"], "tests/contracts/fixtures/forever-product-v29/all.tsv")
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
            "forever-product": "29",
            "build": "70450",
            "locale": "esES",
            "index": "contiguous",
            "root": "header-50893-v2",
            "schema-catalog": "wowdbdefs-70450-v1",
            "decryption": "disabled",
            "partial_encrypted_sections": "declared-skip",
            "collision-storage": "linux-leased-append-disk-world-v1",
            "collision-profile": "linux70450-authenticated-available-map-geometry-vmap-product-v1",
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

    def test_manifest_is_per_table_only_and_active_map_artifacts_are_mandatory(self):
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
        self.assertIn("no global or package-wide", package["declaration_boundary"])
        self.assertIn("multi-frame", package["declaration_boundary"])
        self.assertEqual(package["coverage"], AVAILABLE_COVERAGE_PATH)
        self.assertEqual(package["completion"], COMPLETION_PROFILE)
        self.assertTrue(package["tree_required_for_every_active_map"])
        self.assertTrue(package["placement_proof_required_for_every_active_map"])
        self.assertIn("including an empty one", self.contract["source_policy"]["required_active_map_artifacts"])
        self.assertIn("never converted into an empty active map", package["active_empty_maps"])
        self.assertFalse(package["terrain_outputs"])
        self.assertFalse(package["navigation_outputs"])
        self.assertFalse(package["other_components"])
        self.assertTrue(package["provisional_until_sealed"])
        manifest_case = self.cases()["per-table-manifest"]["expect"]
        self.assertIn("only v26/v27 per-table", manifest_case)
        self.assertIn("including Map", manifest_case)
        self.assertIn("global partial_encrypted_sections is forbidden", manifest_case)

    def test_limits_preserve_v22_and_use_checked_combined_package_budget(self):
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
        self.assertEqual((limits["min_db2_entries"], limits["max_db2_entries"]), (1, 1))
        self.assertEqual(limits["required_map_entries"], 1)
        self.assertEqual(limits["db2_file_bytes"], 16 * 1024**2)
        self.assertEqual(limits["db2_total_bytes"], 16 * 1024**3)
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
        self.assertIn("16 GiB DB2", limits["package_total_bytes_equation"])
        self.assertIn("overflow rejects", limits["package_total_bytes_equation"])
        self.assertEqual(limits["package_total_bytes_max"], 85_899_345_920)
        self.assertEqual(
            limits["package_total_bytes_max"],
            limits["collision_output_bytes_max"] + limits["db2_total_bytes"],
        )
        self.assertFalse(limits["client_bytes_in_reports_or_repository"])
        for text in (
            "128 MiB aggregate",
            "128 MiB placement",
            "384 MiB model",
            "4096-map",
            "65535-artifact",
        ):
            self.assertIn(text, limits["meaning"])
        boundaries = self.cases()["resource-boundaries"]["expect"]
        for text in (
            "retain every v22 numeric ceiling",
            "128 MiB aggregate",
            "128 MiB placements",
            "384 MiB models",
            "4096 maps",
            "65535 artifacts",
            "16 MiB",
            "16 GiB DB2 total",
            "65,536 frames per table",
            "18,576 total",
            "checked_add(collision-output-bytes, 17179869184)",
            "85,899,345,920 bytes",
        ):
            self.assertIn(text, boundaries)

    def test_rcvgeo1_v2_has_no_requested_map_and_wrappers_never_cross(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["path"], AVAILABLE_COVERAGE_PATH)
        self.assertEqual(codec["magic"], "RCVGEO1")
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertEqual(codec["layout_version"], 2)
        self.assertEqual(codec["profile"], SOURCE_PROFILE)
        self.assertEqual(codec["coverage_profile"], COVERAGE_PROFILE)
        self.assertEqual(codec["requested_map"], "absent")
        self.assertEqual(codec["requested_map_substitute"], "forbidden")
        self.assertFalse(codec["requested_map_zero_allowed"])
        self.assertFalse(codec["map_and_ancestors_zero_allowed"])
        self.assertEqual(codec["selection"], "MapSelection::All")
        self.assertEqual(codec["domain_closure"], "available-map-all-active-v1")
        self.assertTrue(codec["v22_layout_preserved"])
        self.assertTrue(codec["v28_layout_preserved"])
        self.assertTrue(codec["canonical_roundtrip_required"])

        wrappers = codec["strict_wrappers"]
        expected = (
            ("v22", "forever-product-v22", 1, 16, "producer_reported"),
            ("v28", "forever-product-v28", 1, 17, "independently_derived"),
            ("v29", "forever-product-v29", 2, 17, "independently_derived"),
        )
        for name, contract, version, mask, authentication in expected:
            with self.subTest(wrapper=name):
                wrapper = wrappers[name]
                self.assertEqual(wrapper["contract"], contract)
                self.assertEqual(wrapper["layout_version"], version)
                self.assertEqual(wrapper["component_mask"], mask)
                self.assertEqual(wrapper["coverage_authentication"], authentication)
                self.assertEqual(wrapper["profile"], SOURCE_PROFILE if name == "v29" else "linux70450-selected-map-geometry-v1")
                self.assertEqual(wrapper["coverage_profile"], COVERAGE_PROFILE if name == "v29" else "linux70450-selected-map-geometry-coverage-v1")
                self.assertEqual(
                    wrapper["domain_closure"],
                    "available-map-all-active-v1" if name == "v29" else "available-map-active-ancestors-v1",
                )
                self.assertEqual(
                    wrapper["selection"],
                    "MapSelection::All" if name == "v29" else "MapSelection::MapAndAncestors(requested_map)",
                )
                self.assertEqual(
                    wrapper["coverage_path"],
                    AVAILABLE_COVERAGE_PATH if name == "v29" else "vmaps/selected-geometry-coverage-v1.bin",
                )
        self.assertEqual(wrappers["v29"]["requested_map"], "absent; no substitute")
        self.assertIn("rejected", wrappers["foreign_or_mixed_masks"])
        self.assertIn("verify-links", wrappers["foreign_or_mixed_masks"])
        self.assertIn("does not permit crossing", wrappers["shared_mask17"])
        self.assertIn("selection", wrappers["shared_mask17"])
        self.assertIn("publication profile", wrappers["shared_mask17"])
        self.assertIn("requested_map=0", wrappers["zero_selection_aliases"])
        self.assertIn("MapSelection::MapAndAncestors(0)", wrappers["zero_selection_aliases"])
        self.assertIn("rejected", wrappers["zero_selection_aliases"])
        self.assertIn("never treated as MapSelection::All", wrappers["zero_selection_aliases"])
        self.assertEqual(codec["context"]["build"], 70450)
        self.assertEqual(codec["context"]["component_mask"], 17)
        self.assertIn("RCVGEO1 version 2", codec["context"]["binding"])
        self.assertIn("MapSelection::All", codec["context"]["binding"])
        self.assertIn("v28 also uses mask 17", codec["context"]["binding"])

    def test_independent_graph_authentication_and_all_active_map_semantics(self):
        verification = self.contract["verification"]
        self.assertEqual(
            verification["profile"],
            "linux70450-authenticated-available-map-geometry-verification-v1",
        )
        self.assertEqual(
            verification["report_scope"],
            "artifact_integrity_available_map_geometry_associations_and_independently_derived_map_vmap_domain_coverage",
        )
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertEqual(verification["source_authentication_flags"], "producer_reported")
        self.assertEqual(verification["map_reader"], "read_vmap_maps_70450")
        self.assertIn("retaining WDT==0", verification["map_materialization"])
        self.assertEqual(verification["domain_builder"], "Maps::from_materialized_selection")
        self.assertEqual(verification["map_selection"], "MapSelection::All")
        self.assertEqual(verification["requested_map"], "absent; no substitute")
        self.assertFalse(verification["requested_map_zero_allowed"])
        self.assertFalse(verification["map_and_ancestors_zero_allowed"])
        self.assertEqual(verification["domain_closure"], "available-map-all-active-v1")
        self.assertIn("MapEntry::is_parent", verification["parent_derivation"])
        self.assertIn("complete materialized graph", verification["parent_derivation"])
        self.assertTrue(verification["is_parent_uses_complete_graph"])
        self.assertTrue(verification["vmap_materialized_rows_retain_wdt_zero"])
        self.assertFalse(verification["wdt_zero_rows_counted_in_active_maps"])
        self.assertFalse(verification["terrain_wdt_zero_filter_reused"])
        self.assertTrue(verification["active_empty_maps_included"])
        self.assertFalse(verification["missing_or_unknown_ids_synthesized_as_empty"])
        self.assertTrue(verification["tree_required_for_every_active_map"])
        self.assertTrue(verification["placement_proof_required_for_every_active_map"])
        derived = " ".join(verification["independently_derived_facts"])
        for text in (
            "section",
            "direct/copy/materialized",
            "complete materialized graph",
            "MapSelection::All",
            "active empty maps",
            "WDT==0",
            "active_maps",
            "tree and placement-proof",
        ):
            self.assertIn(text, derived)
        self.assertIn("CKey/EKey", verification["source_authentication_flags_scope"])
        self.assertIn("not independent cryptographic proof", verification["source_authentication_flags_scope"])
        self.assertFalse(verification["independent_cryptographic_source_authentication"])
        self.assertFalse(verification["partial_decryption_claim"])
        self.assertFalse(verification["client_reads"])

        source = self.contract["source_policy"]
        self.assertIn("read_vmap_maps_70450", source["vmap_map_projection"])
        self.assertIn("WDT==0", source["vmap_map_projection"])
        self.assertIn("distinct from active_maps", source["vmap_map_projection"])
        self.assertIn("Maps::from_materialized_selection", source["domain"])
        self.assertIn("MapSelection::All", source["domain"])
        self.assertIn("including genuinely empty active maps", source["domain"])
        self.assertIn("MapSelection::MapAndAncestors(0)", source["domain"])
        self.assertIn("All and only active admitted", source["active_maps"])
        self.assertIn("not a fallback", source["active_maps"])
        self.assertIn("excluded from active_maps", source["wdt_zero_rows"])
        self.assertIn("never invented", source["omitted_or_unknown_ids"])
        self.assertIn("not evidence of an empty map", source["omitted_or_unknown_ids"])
        self.assertIn("Every active admitted map", source["required_active_map_artifacts"])
        self.assertIn("including an empty one", source["required_active_map_artifacts"])
        self.assertIn("never silently omitted or converted into an empty map", source["failure_policy"])

        coverage = self.contract["coverage"]
        self.assertEqual(coverage["profile"], COVERAGE_PROFILE)
        self.assertEqual(coverage["path"], AVAILABLE_COVERAGE_PATH)
        self.assertEqual(coverage["selection"], "MapSelection::All")
        self.assertEqual(coverage["domain_closure"], "available-map-all-active-v1")
        self.assertEqual(coverage["authentication"], "independently_derived")
        self.assertEqual(coverage["source_authentication_flags"], "producer_reported")
        self.assertTrue(coverage["active_empty_maps_included"])
        self.assertFalse(coverage["wdt_zero_rows_counted_in_active_maps"])
        self.assertFalse(coverage["missing_or_unknown_ids_synthesized_as_empty"])
        self.assertTrue(coverage["tree_required_for_every_active_map"])
        self.assertTrue(coverage["placement_proof_required_for_every_active_map"])
        self.assertFalse(coverage["complete_global_map_coverage_claim"])
        self.assertFalse(coverage["whole_client_map_coverage_claim"])
        self.assertFalse(coverage["complete_vmap_component_claim"])
        self.assertFalse(coverage["full_world_coverage"])

    def test_exclusions_and_all_non_claim_boundaries(self):
        contract = self.contract
        source = contract["source_policy"]
        self.assertIn("GODI and global GameObject startup are excluded", source["gameobjects"])
        self.assertIn("No GameObjectDisplayInfo request", source["gameobjects"])
        self.assertIn("placement requests", source["models"])
        self.assertIn("exact used_models/VMO-artifact inventory", source["models"])
        self.assertIn("LiquidObject is out of scope", source["liquids"])
        self.assertIn("no decryption or key material", source["partial_sections"])
        for excluded in (
            "GODI/global GameObject",
            "LiquidObject",
            "terrain and MAPS output",
            "navigation and MMaps",
            "full-world",
            "complete VMap component",
            "global or package-wide partial-section manifest declarations",
            "requested_map=0",
            "MapSelection::MapAndAncestors(0)",
            "WDT==0",
            "unknown or omitted map IDs as empty active maps",
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
            "missing_or_unknown_maps_as_empty",
            "wdt_zero_rows_in_active_maps",
            "requested_map_zero_as_all",
            "map_and_ancestors_zero_as_all",
            "global_gameobjects",
            "liquid_object",
            "terrain",
            "navigation_or_mmaps",
            "global_partial_manifest_declarations",
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

    def test_failed_real_attempt_is_retained_and_legacy_identities_stay_separate(self):
        target = self.cases()["real-acceptance"]
        self.assertEqual(target["status"], "pending-after-failed-attempt")
        self.assertIn("no real v29 acceptance", target["expect"])
        self.assertIn("v28 selected-map evidence remains separate", target["expect"])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "pending-after-failed-attempt")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v29")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        evidence = real["evidence"]
        self.assertEqual(evidence["status"], "failed")
        self.assertEqual(evidence["failure_code"], "file_unavailable")
        self.assertEqual(evidence["component"], "vmaps")
        self.assertFalse(evidence["accepted_prefix"])
        self.assertEqual(
            evidence["report_sha256"],
            "20b0735f0fc85808b502451c7a642109bfa441f37ec57cba20aa6a6d294043a9",
        )
        follow_up = real["follow_up"]
        self.assertEqual(follow_up["status"], "failed")
        self.assertFalse(follow_up["accepted_prefix"])
        self.assertEqual(
            follow_up["report_sha256"],
            "597ad8e3e276f087d2bf8fdc273d0ca6f71c6c3338b10bb2fa59185ac4f867aa",
        )
        self.assertEqual(follow_up["diagnostic"]["phase"], "placements")
        self.assertEqual(follow_up["diagnostic"]["request_kind"], "wdt")
        self.assertEqual(follow_up["diagnostic"]["file_data_id"], 857697)
        census = follow_up["census"]
        self.assertEqual(census["active_maps"], 71)
        self.assertEqual(census["root_routed_maps"], 69)
        self.assertEqual(census["missing_root_maps"], 2)
        self.assertEqual(census["missing_wdt_fdids"], [857697, 7937200])
        self.assertEqual(census["root_without_encoding_maps"], 0)
        self.assertEqual(
            census["report_sha256"],
            "2a8483ddce57924f157be0c04d855a228df7665a869b2e900aba14e92144a48d",
        )
        self.assertIn("no accepted real v29 package", real["note"])
        self.assertFalse(real["existing_v28_evidence_transferred"])

        evidence = self.contract["existing_evidence"]
        self.assertEqual(evidence["v28_acceptance_status"], "passed")
        self.assertTrue(evidence["v28_evidence_is_separate"])
        self.assertIn("does not execute or qualify v29", evidence["v28_scope"])
        self.assertIn("MapSelection::All", evidence["v28_scope"])
        self.assertFalse(evidence["v28_acceptance_transferred_to_v29"])
        self.assertFalse(evidence["v22_acceptance_transferred_to_v29"])
        self.assertFalse(evidence["v28_all_active_selection_qualified"])

        legacy = self.contract["legacy_preservation"]
        self.assertEqual(legacy["v22_component_mask"], 16)
        self.assertEqual(legacy["v22_coverage_layout_version"], 1)
        self.assertEqual(legacy["v22_domain_closure"], "available-map-active-ancestors-v1")
        self.assertEqual(legacy["v22_coverage_authentication"], "producer_reported")
        self.assertTrue(legacy["v22_plan_grammar_unchanged"])
        self.assertTrue(legacy["v22_not_relabelled_v29"])
        self.assertIn("Build70450SelectedGeometryV1", legacy["v22_contract"])
        self.assertEqual(legacy["v28_component_mask"], 17)
        self.assertEqual(legacy["v28_coverage_layout_version"], 1)
        self.assertEqual(legacy["v28_domain_closure"], "available-map-active-ancestors-v1")
        self.assertEqual(legacy["v28_coverage_authentication"], "independently_derived")
        self.assertTrue(legacy["v28_plan_grammar_unchanged"])
        self.assertTrue(legacy["v28_not_relabelled_v29"])
        self.assertIn("Build70450AuthenticatedSelectedGeometryV1", legacy["v28_contract"])
        self.assertFalse(legacy["shared_mask17_permits_profile_or_selection_crossing"])
        self.assertFalse(legacy["v22_and_v28_acceptance_transferred_to_v29"])

        v22 = json.loads((ROOT / "contracts/forever-product-contract-v22.json").read_bytes())
        v28 = json.loads((ROOT / "contracts/forever-product-contract-v28.json").read_bytes())
        self.assertEqual(v22["plan"]["component_mask"], 16)
        self.assertEqual(v22["plan"]["domain_closure"], "available-map-active-ancestors-v1")
        self.assertEqual(v22["verification"]["coverage_authentication"], "producer_reported")
        self.assertEqual(v28["plan"]["component_mask"], 17)
        self.assertEqual(v28["plan"]["domain_closure"], "available-map-active-ancestors-v1")
        self.assertEqual(v28["coverage_codec"]["layout_version"], 1)
        self.assertEqual(v28["verification"]["coverage_authentication"], "independently_derived")
        self.assertNotEqual(v22["publication_profile"], self.contract["publication_profile"])
        self.assertNotEqual(v28["publication_profile"], self.contract["publication_profile"])


if __name__ == "__main__":
    unittest.main()
