# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 partial-catalog DB2 plus terrain contract; synthetic inputs only."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v27.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v27"
ORDERED_KEYS = [
    "forever-product",
    "build",
    "locale",
    "index",
    "root",
    "schema-catalog",
    "decryption",
    "partial_encrypted_sections",
    "components",
    "terrain",
    "pack",
    "navigation",
    "big-base-unit",
    "skip-liquids",
    "maximum-slope",
    "non-steep-slope",
]


class ProductV27Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(CONTRACT.read_bytes())

    def cases(self):
        return {case["case"]: case for case in self.contract["acceptance_matrix"]}

    def test_identity_and_exact_plan_scope(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v27", 27, "Build70450PartialCatalogTerrainV1"),
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
        self.assertEqual(plan["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual((plan["components"], plan["component_mask"]), (["db2", "maps"], 9))
        self.assertEqual(plan["component_bits"], {"db2": 1, "gt": 2, "cameras": 4, "maps": 8})
        self.assertEqual(plan["decryption"], "disabled")
        self.assertEqual(plan["keys"], "refused")
        self.assertIn("declared-skip", plan["partial_encrypted_sections"])
        self.assertIn("PLAN only", plan["partial_encrypted_sections"])
        self.assertEqual(plan["storage"], "resident")
        self.assertEqual(plan["terrain"], ["all-available-maps", "map:<canonical-u32>"])
        self.assertEqual(plan["terrain_coverage"], "maps/compatible-terrain-coverage-v3.tsv")
        self.assertEqual(plan["navigation"], "none")
        self.assertEqual(plan["map_domain"], "absent")
        self.assertEqual(plan["offmesh"], "absent")
        self.assertEqual(plan["scratch"], "forbidden")
        self.assertTrue(plan["pack"])
        self.assertFalse(plan["big_base_unit"])
        self.assertFalse(plan["skip_liquids"])
        self.assertEqual((plan["maximum_slope"], plan["non_steep_slope"]), (55, 55))
        self.assertEqual(
            plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1}
        )
        self.assertIn("1..=1161", plan["db2_entries"])
        self.assertIn("exactly one Map", plan["db2_entries"])
        self.assertIn("every other canonical table is optional", plan["db2_entries"])

    def test_fixtures_are_canonical_eses_with_map_before_spell_effect(self):
        for filename, terrain in (("partial.tsv", "map:48"), ("all.tsv", "all-available-maps")):
            with self.subTest(filename=filename):
                raw = (FIXTURES / filename).read_bytes()
                self.assertTrue(raw.endswith(b"\n"))
                self.assertNotIn(b"\r", raw)
                self.assertTrue(raw.isascii())
                rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
                self.assertEqual([row[0] for row in rows[:-2]], ORDERED_KEYS)
                self.assertTrue(all(len(row) == 2 for row in rows[:-2]))
                fields = {row[0]: row[1] for row in rows[:-2]}
                self.assertEqual(fields["forever-product"], "27")
                self.assertEqual(fields["build"], "70450")
                self.assertEqual(fields["locale"], "esES")
                self.assertEqual(fields["index"], "contiguous")
                self.assertEqual(fields["root"], "header-50893-v2")
                self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
                self.assertEqual(fields["decryption"], "disabled")
                self.assertEqual(fields["partial_encrypted_sections"], "declared-skip")
                self.assertEqual(fields["components"], "db2,maps")
                self.assertEqual(fields["terrain"], terrain)
                self.assertEqual(fields["pack"], "true")
                self.assertEqual(fields["navigation"], "none")
                self.assertEqual(fields["big-base-unit"], "false")
                self.assertEqual(fields["skip-liquids"], "false")
                self.assertEqual(fields["maximum-slope"], "55")
                self.assertEqual(fields["non-steep-slope"], "55")
                self.assertEqual(
                    rows[-2:],
                    [["db2", "Map", "1349477"], ["db2", "SpellEffect", "1140088"]],
                )
                self.assertNotIn("partial_encrypted_tables", fields)

    def test_manifest_is_per_table_only_and_multi_frame_covers_map(self):
        package = self.contract["package"]
        self.assertIn("per-table", package["partial_encrypted_tables"])
        self.assertIn("including Map when partial", package["partial_encrypted_tables"])
        self.assertIn("lookup, frame_ordinal, decoded_size", package["partial_encrypted_tables"])
        self.assertIn("forbidden", package["partial_encrypted_sections"])
        self.assertIn("PLAN gate remains mandatory", package["partial_encrypted_sections"])
        self.assertIn("forbidden", package["global_partial_encrypted_sections"])
        self.assertIn("v25 global manifest declarations are not admitted", package["global_partial_encrypted_sections"])
        self.assertIn("no global or package-wide", package["declaration_boundary"])
        self.assertIn("every selected table, including Map", package["multi_frame_coverage"])
        self.assertIn("exactly tile each ordered TACT-declared extent", package["multi_frame_coverage"])
        self.assertIn("Multiple frames per extent", package["multi_frame_coverage"])
        manifest_case = self.cases()["per-table-manifest"]["expect"]
        self.assertIn("only per-table partial_encrypted_tables", manifest_case)
        self.assertIn("including Map when partial", manifest_case)
        self.assertIn("global partial_encrypted_sections is forbidden", manifest_case)
        multi_frame = self.cases()["multi-frame-tact-coverage"]["expect"]
        self.assertIn("including Map", multi_frame)
        self.assertIn("multiple E frames per TACT extent", multi_frame)
        self.assertIn("crossing, missing, extra", multi_frame)

    def test_numeric_limits_and_rejection_boundaries(self):
        limits = self.contract["limits"]
        self.assertEqual((limits["min_db2_entries"], limits["max_db2_entries"]), (1, 1161))
        self.assertEqual(limits["required_map_entries"], 1)
        self.assertEqual(limits["db2_file_bytes"], 16 * 1024 * 1024)
        self.assertEqual(limits["db2_total_bytes"], 16 * 1024 * 1024 * 1024)
        self.assertEqual(limits["partial_frames_per_table"], 65_536)
        self.assertEqual(limits["partial_frames_total"], 18_576)
        self.assertIn("including Map", limits["partial_frames_scope"])
        self.assertIn("CASC max_blte_frames", limits["partial_frames_scope"])
        self.assertIn("every selected table, including Map", limits["partial_frame_coverage"])
        self.assertEqual(limits["terrain_output_bytes"], 64 * 1024**3)
        self.assertEqual(limits["package_total_bytes"], 80 * 1024**3)
        self.assertEqual(limits["package_total_bytes"], 85_899_345_920)
        self.assertEqual(
            limits["package_total_bytes"],
            limits["terrain_output_bytes"] + limits["db2_total_bytes"],
        )
        self.assertEqual(limits["package_total_bytes_equation"], "64 GiB terrain + 16 GiB DB2 = 80 GiB total")
        catalog_case = self.cases()["catalog-limit"]["expect"]
        self.assertIn("1 through 1161", catalog_case)
        self.assertIn("entry 1162", catalog_case)
        boundaries = self.cases()["resource-boundaries"]["expect"]
        for text in (
            "16 MiB per DB2 table",
            "16 GiB DB2 total",
            "65,536 frames per table",
            "18,576 frames total including Map",
            "table 1162",
            "frame 65,537 in one table",
            "frame 18,577 overall",
            "64 GiB",
            "80 GiB / 85,899,345,920 bytes",
        ):
            self.assertIn(text, boundaries)

    def test_map_authentication_and_offline_verification_scope(self):
        source = self.contract["source_policy"]["map_terrain"]
        self.assertIn("terrain_v20::authenticate_map/read_map_70450/materialize", source)
        self.assertIn("terrain coverage v3", source)
        verification = self.contract["verification"]
        self.assertEqual(verification["profile"], "linux70450-partial-catalog-authenticated-map-terrain-product-v1")
        self.assertEqual(verification["report_scope"], "artifact_integrity_selected_db2_and_authenticated_terrain_with_per_table_partial_sections")
        self.assertEqual(verification["coverage_authentication"], "independently_derived")
        self.assertFalse(verification["client_reads"])
        self.assertFalse(verification["partial_decryption_claim"])
        self.assertIn("including Map", verification["verify_scope"])
        self.assertIn("terrain coverage v3", verification["verify_scope"])
        self.assertIn("no navigation", verification["verify_links_scope"])
        authenticated = self.cases()["map-terrain-authentication"]["expect"]
        self.assertIn("v20 authenticate/read/materialize", authenticated)
        self.assertIn("WDT FileDataIDs", authenticated)
        self.assertIn("compatible terrain coverage v3", authenticated)

    def test_real_acceptance_is_bounded_and_not_transferred(self):
        target = self.cases()["combined-real-acceptance"]
        self.assertEqual(target["status"], "passed")
        self.assertIn("1,163 artifacts / 55,737,931 bytes", target["expect"])
        self.assertIn("4,555 / 271,392,871 bytes", target["expect"])
        self.assertIn("no full-world or runtime claim", target["expect"])
        evidence = self.contract["existing_evidence"]
        self.assertIn("1,055 complete and 106 declared partial", evidence["v26"])
        self.assertIn("map:48 and all-available-maps", evidence["v25"])
        self.assertTrue(evidence["v27_real_acceptance"])
        self.assertEqual(evidence["v27_acceptance_status"], "bounded-real-passed")
        self.assertEqual(evidence["map48"]["files"], 1163)
        self.assertEqual(evidence["map48"]["bytes"], 55737931)
        self.assertEqual(evidence["map48"]["terrain_map_tiles"], 0)
        self.assertEqual(evidence["all_available"]["files"], 4555)
        self.assertEqual(evidence["all_available"]["bytes"], 271392871)
        self.assertEqual(evidence["all_available"]["terrain_map_tiles"], 3322)
        self.assertEqual(evidence["all_available"]["terrain_tile_lists"], 71)
        self.assertEqual(evidence["all_available"]["inventory_sha256"], "8ba5d6866ce779a97b410c82283e1c89d213a5e414d75b78d508b3ee4ffe6157")
        self.assertEqual(evidence["all_available"]["redacted_report_sha256"], "de15dce370d7b9526f66e7a11c3f84dcff077726763a816a81480c66499eb3b7")
        self.assertEqual(evidence["payload_comparison"]["db2_payloads_matching_v26"], 1161)
        self.assertEqual(evidence["payload_comparison"]["terrain_payloads_matching_v25_excluding_coverage"], 3393)
        self.assertTrue(evidence["payload_comparison"]["partial_declarations_matching_v26"])
        self.assertFalse(evidence["prior_acceptance_transferred_to_v27"])

    def test_legacy_preservation_and_all_non_claim_boundaries(self):
        legacy = self.contract["legacy_preservation"]
        self.assertTrue(legacy["v25_plan_grammar_unchanged"])
        self.assertTrue(legacy["v26_plan_grammar_unchanged"])
        self.assertIn("Build70450AuthenticatedTerrainPartialMapV1", legacy["v25_contract"])
        self.assertIn("Build70450Db2PartialV1", legacy["v26_contract"])
        self.assertFalse(legacy["v25_and_v26_acceptance_transferred_to_v27"])
        for name in (
            "decryption",
            "key_discovery_or_use",
            "complete_db2_content",
            "full_world_coverage",
            "runtime_acceptance",
            "format_parity",
            "windows",
            "release_publication",
            "v27_full_world_or_runtime_acceptance",
            "full_extractor_complete",
        ):
            self.assertFalse(self.contract["boundary"][name])
        for name in (
            "complete_db2_content_claim",
            "whole_client_map_coverage",
            "full_world_coverage",
            "runtime_claim",
            "format_parity_claim",
            "full_extractor_complete",
        ):
            self.assertFalse(self.contract["verification"][name])
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
