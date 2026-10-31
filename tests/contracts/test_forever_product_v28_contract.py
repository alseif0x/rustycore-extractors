# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 authenticated selected-geometry contract; synthetic plans only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v28.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v28"
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


class ProductV28Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_and_exact_plan_surface(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v28", 28, "Build70450AuthenticatedSelectedGeometryV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(
            (contract["platform"], contract["backend"], contract["offline"]),
            ("linux", "rust-local", True),
        )
        self.assertEqual(
            contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"]
        )

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
        self.assertEqual(plan["collision_domain"], "map:<canonical-u32>")
        self.assertEqual(plan["domain_closure"], "available-map-active-ancestors-v1")
        self.assertEqual(plan["gameobject_scope"], "excluded-by-profile-v1")
        self.assertEqual(plan["model_scope"], "placement-only-v1")
        self.assertEqual(plan["db2_entries"], "exactly one canonical Map DB2 entry; no other table")
        self.assertEqual(
            plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1}
        )
        self.assertEqual((plan["terrain"], plan["navigation"], plan["offmesh"]), ("none", "none", "absent"))
        self.assertTrue(plan["pack"])
        self.assertFalse(plan["big_base_unit"])
        self.assertFalse(plan["skip_liquids"])
        self.assertEqual((plan["maximum_slope"], plan["non_steep_slope"]), (55, 55))
        self.assertFalse(plan["all_available_maps_selection"])
        self.assertFalse(plan["inspect_requires_client"])

    def test_fixtures_are_canonical_synthetic_and_map_only(self):
        metadata = self.contract["fixtures"]
        self.assertEqual(metadata["classification"], "canonical synthetic PLAN fixtures only")
        self.assertFalse(metadata["contains_client_data"])
        self.assertFalse(metadata["real_extraction_evidence"])
        self.assertFalse(metadata["certified_real_storage_budgets"])
        self.assertEqual(metadata["synthetic_budget_bytes"], 1048576)
        self.assertIn("not real acceptance budgets", metadata["budget_note"])
        self.assertIn("not certified for real extraction", metadata["budget_note"])
        cases = {Path(case["path"]).name: case for case in metadata["cases"]}

        for filename, locale, domain in (("map2.tsv", "enUS", "map:2"), ("map48.tsv", "esES", "map:48")):
            with self.subTest(filename=filename):
                raw = (FIXTURES / filename).read_bytes()
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
                    "forever-product": "28",
                    "build": "70450",
                    "locale": locale,
                    "index": "contiguous",
                    "root": "header-50893-v2",
                    "schema-catalog": "wowdbdefs-70450-v1",
                    "decryption": "disabled",
                    "partial_encrypted_sections": "declared-skip",
                    "collision-storage": "linux-leased-append-disk-world-v1",
                    "collision-profile": "linux70450-selected-map-geometry-vmap-product-v1",
                    "collision-domain": domain,
                    "collision-coverage": "linux70450-selected-map-geometry-coverage-v1",
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

                case = cases[filename]
                self.assertEqual(case["path"], f"tests/contracts/fixtures/forever-product-v28/{filename}")
                self.assertEqual((case["locale"], case["collision_domain"]), (locale, domain))
                self.assertTrue(case["canonical_fixture"])
                self.assertFalse(case["real_evidence"])
                self.assertFalse(case["certified_real_budget"])

    def test_manifest_is_per_table_only_including_map(self):
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
        self.assertFalse(package["terrain_outputs"])
        self.assertFalse(package["navigation_outputs"])
        self.assertFalse(package["other_components"])
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
        self.assertEqual((limits["min_db2_entries"], limits["max_db2_entries"]), (1, 1))
        self.assertEqual(limits["required_map_entries"], 1)
        self.assertEqual(limits["db2_file_bytes"], 16 * 1024 * 1024)
        self.assertEqual(limits["db2_total_bytes"], 16 * 1024**3)
        self.assertEqual(limits["db2_total_bytes"], 17_179_869_184)
        self.assertEqual(limits["partial_frames_per_table"], 65_536)
        self.assertEqual(limits["partial_frames_total"], 18_576)
        self.assertIn("including Map", limits["partial_frames_scope"])
        self.assertIn("CASC max_blte_frames", limits["partial_frames_scope"])
        self.assertIn("exactly tile", limits["partial_frame_coverage"])
        self.assertEqual(limits["package_total_bytes"], "collision-output-bytes + 17179869184")
        self.assertIn("checked_add(collision-output-bytes, 17179869184)", limits["package_total_bytes_equation"])
        self.assertIn("16 GiB DB2", limits["package_total_bytes_equation"])
        self.assertIn("overflow rejects", limits["package_total_bytes_equation"])
        self.assertEqual(limits["package_total_bytes_max"], 85_899_345_920)
        self.assertEqual(
            limits["package_total_bytes_max"],
            limits["collision_output_bytes_max"] + limits["db2_total_bytes"],
        )
        self.assertFalse(limits["client_bytes_in_reports_or_repository"])
        boundaries = self.cases()["resource-boundaries"]["expect"]
        for text in (
            "retain every v22 numeric ceiling",
            "16 MiB",
            "16 GiB DB2 total",
            "65,536 frames per table",
            "18,576 total",
            "checked_add(collision-output-bytes, 17179869184)",
            "85,899,345,920 bytes",
        ):
            self.assertIn(text, boundaries)

    def test_rcvgeo1_wrappers_context_and_independent_map_authentication(self):
        codec = self.contract["coverage_codec"]
        self.assertEqual(codec["magic"], "RCVGEO1")
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertEqual(codec["layout_version"], 1)
        self.assertTrue(codec["v22_layout_preserved"])
        self.assertTrue(codec["canonical_roundtrip_required"])
        wrappers = codec["strict_wrappers"]
        self.assertEqual(wrappers["v22"]["component_mask"], 16)
        self.assertEqual(wrappers["v22"]["coverage_authentication"], "producer_reported")
        self.assertEqual(wrappers["v28"]["component_mask"], 17)
        self.assertEqual(wrappers["v28"]["coverage_authentication"], "independently_derived")
        self.assertIn("rejected", wrappers["foreign_or_mixed_masks"])
        self.assertIn("verify-links", wrappers["foreign_or_mixed_masks"])
        self.assertEqual(codec["context"]["build"], 70450)
        self.assertEqual(codec["context"]["component_mask"], 17)
        self.assertIn("mask 16", codec["context"]["binding"])
        self.assertIn("every foreign mask reject", codec["context"]["binding"])

        verification = self.contract["verification"]
        self.assertEqual(verification["profile"], "linux70450-authenticated-selected-map-geometry-verification-v1")
        self.assertEqual(verification["report_scope"], "artifact_integrity_selected_map_geometry_associations_and_independently_derived_map_vmap_domain_coverage")
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertEqual(verification["source_authentication_flags"], "producer_reported")
        self.assertEqual(verification["map_reader"], "read_vmap_maps_70450")
        self.assertIn("retaining WDT==0", verification["map_materialization"])
        self.assertEqual(verification["domain_builder"], "Maps::from_materialized_selection")
        self.assertIn("MapEntry::is_parent", verification["parent_derivation"])
        self.assertIn("complete graph", verification["parent_derivation"])
        self.assertTrue(verification["is_parent_uses_complete_graph"])
        self.assertTrue(verification["unselected_children_affect_is_parent"])
        self.assertTrue(verification["vmap_materialized_rows_retain_wdt_zero"])
        self.assertFalse(verification["terrain_wdt_zero_filter_reused"])
        derived = " ".join(verification["independently_derived_facts"])
        for text in ("section", "direct/copy/materialized", "active-ancestor domain", "children outside the selection"):
            self.assertIn(text, derived)
        self.assertIn("CKey/EKey", verification["source_authentication_flags_scope"])
        self.assertIn("not independent cryptographic proof", verification["source_authentication_flags_scope"])
        self.assertFalse(verification["independent_cryptographic_source_authentication"])
        self.assertFalse(verification["partial_decryption_claim"])
        self.assertFalse(verification["client_reads"])
        source = self.contract["source_policy"]
        self.assertIn("read_vmap_maps_70450", source["vmap_map_projection"])
        self.assertIn("WDT==0", source["vmap_map_projection"])
        self.assertIn("distinct from terrain", source["vmap_map_projection"])
        self.assertIn("Maps::from_materialized_selection", source["domain"])
        self.assertIn("children outside the selected domain", source["domain"])

        coverage = self.contract["coverage"]
        self.assertEqual(coverage["selection"], "selected-map-and-active-ancestors")
        self.assertEqual(coverage["authentication"], "independently_derived")
        self.assertEqual(coverage["source_authentication_flags"], "producer_reported")
        self.assertFalse(coverage["complete_global_map_coverage_claim"])
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
            "full-world coverage",
            "global or package-wide partial-section manifest declarations",
        ):
            self.assertTrue(any(excluded in value for value in contract["excluded"]), excluded)

        for name in (
            "decryption",
            "key_discovery_or_use",
            "complete_db2_content",
            "full_world_coverage",
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
            "runtime_claim",
            "format_parity_claim",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["verification"][name], name)
        for name in (
            "runtime_acceptance",
            "full_world_coverage",
            "format_parity",
            "mmaps",
            "windows",
            "full_extractor_complete",
        ):
            self.assertFalse(contract["unqualified_status"][name], name)
        self.assertNotIn("real_extraction", contract["unqualified_status"])
        self.assertFalse(contract["full_extractor_complete"])

    def test_real_acceptance_is_bounded_and_legacy_identities_stay_separate(self):
        target = self.cases()["real-acceptance"]
        self.assertEqual(target["status"], "passed")
        self.assertIn("67 artifacts / 2,090,335 bytes", target["expect"])
        self.assertIn("65 non-coverage geometry payloads matching v22", target["expect"])
        self.assertIn("no full-world/runtime claim", target["expect"])
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "passed")
        self.assertTrue(real["performed"])
        evidence = real["evidence"]
        self.assertEqual(evidence["run_id"], "v28-authenticated-geometry-map48-20261024T125226Z-2022337")
        self.assertEqual(evidence["files"], 67)
        self.assertEqual(evidence["bytes"], 2090335)
        self.assertEqual(evidence["component_files"], {"db2": 1, "vmaps": 66})
        self.assertEqual(evidence["inventory_sha256"], "e848ec9121147667461918c9d38edbb5f88f0c679b7bbf92fe9a79f9a7514833")
        self.assertEqual(evidence["redacted_report_sha256"], "0e9a83c3b56416fc31eeb4a93b89b062c37e5181dc5fda2561671cd138df8165")
        self.assertTrue(evidence["map_payload_matching_v26"])
        self.assertEqual(evidence["geometry_payloads_matching_v22_excluding_coverage"], 65)
        self.assertEqual(evidence["map_partial_frames"], 3)
        self.assertEqual(evidence["map_partial_frame_bytes"], 575)
        self.assertEqual(evidence["global_partial_declarations"], 0)
        self.assertFalse(evidence["network_attempted"])
        self.assertTrue(evidence["client_metadata_unchanged"])
        self.assertTrue(evidence["inputs_unchanged"])
        self.assertFalse(real["existing_v27_evidence_transferred"])
        evidence = self.contract["existing_evidence"]
        self.assertEqual(evidence["v27_acceptance_status"], "bounded-real-passed")
        self.assertTrue(evidence["v27_evidence_is_separate"])
        self.assertIn("does not execute or qualify the v28", evidence["v27_scope"])
        self.assertFalse(evidence["v27_acceptance_transferred_to_v28"])
        self.assertFalse(evidence["v22_acceptance_transferred_to_v28"])

        legacy = self.contract["legacy_preservation"]
        self.assertEqual(legacy["v22_component_mask"], 16)
        self.assertEqual(legacy["v22_coverage_authentication"], "producer_reported")
        self.assertTrue(legacy["v22_plan_grammar_unchanged"])
        self.assertTrue(legacy["v22_not_relabelled_v28"])
        self.assertIn("Build70450SelectedGeometryV1", legacy["v22_contract"])
        self.assertEqual(legacy["v27_component_mask"], 9)
        self.assertTrue(legacy["v27_plan_grammar_unchanged"])
        self.assertIn("Build70450PartialCatalogTerrainV1", legacy["v27_contract"])
        self.assertFalse(legacy["v22_and_v27_acceptance_transferred_to_v28"])

        v22 = json.loads((ROOT / "contracts/forever-product-contract-v22.json").read_bytes())
        v27 = json.loads((ROOT / "contracts/forever-product-contract-v27.json").read_bytes())
        self.assertEqual(v22["plan"]["component_mask"], 16)
        self.assertEqual(v22["verification"]["coverage_authentication"], "producer_reported")
        self.assertEqual(v27["plan"]["component_mask"], 9)
        self.assertEqual(v27["verification"]["coverage_authentication"], "independently_derived")
        self.assertEqual(v27["existing_evidence"]["v27_acceptance_status"], "bounded-real-passed")
        self.assertNotEqual(v22["publication_profile"], self.contract["publication_profile"])
        self.assertNotEqual(v27["publication_profile"], self.contract["publication_profile"])


if __name__ == "__main__":
    unittest.main()
