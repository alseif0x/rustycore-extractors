# SPDX-License-Identifier: GPL-3.0-or-later
"""Strict source and schema checks for the redacted inspect-db2 diagnostic."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class Db2InspectionContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-db2-inspection-contract-v1.json").read_bytes()
        )

    def test_exact_linux_build_catalogue_keyless_report_scope(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"], c["command"]),
            ("forever-db2-inspection-contract-v1", 1, "inspect-db2"),
        )
        self.assertEqual((c["platform"], c["offline"], c["build"], c["catalogue"]),
                         ("linux", True, 70450, "wowdbdefs-70450-v1"))
        self.assertEqual(c["keys"], "refused")
        self.assertTrue(c["extracts_nothing"])
        self.assertFalse(c["full_extractor_complete"])
        self.assertEqual(
            c["limits"],
            {
                "file_data_ids_per_report": 64,
                "decoded_file_bytes": 16 * 1024 * 1024,
                "report_bytes": 4 * 1024 * 1024,
            },
        )
        self.assertEqual(
            c["reader_policy"],
            {
                "high_volume_regular_profile": "wowdbdefs-70450-high-volume-regular-v1",
                "catalogue": "wowdbdefs-70450-v1",
                "declarations": [
                    "ManifestInterfaceData", "ModelFileData", "WMOMinimapTexture"
                ],
                "limits": {
                    "direct_records": 160000,
                    "copy_pairs": 100000,
                    "parent_entries": 160000,
                    "retained_values": 1250000,
                },
                "other_declarations_and_builds": "retain the 100000 direct-record, 100000 copy-pair, 100000 parent-entry, and 1000000 retained-value defaults unless another build-specific profile independently admits them",
                "default_limits_changed": False,
            },
        )

    def test_redacted_report_field_sets_are_closed_and_documented(self):
        report = self.contract["report"]
        self.assertEqual(report["schema"], self.contract["contract"])
        self.assertEqual(
            report["top_level_fields"],
            ["schema", "build", "locale", "keyless", "extracts_nothing", "files"],
        )
        self.assertEqual(
            report["declaration_fields"],
            [
                "file_data_id", "table_name", "total_decoded_size_bytes",
                "header_magic", "header_version", "header_flags", "section_count",
                "record_count", "field_count", "total_field_count", "min_id", "max_id",
                "packed_data_offset", "parent_lookup_count", "record_size",
                "string_block_size", "column_meta_size", "common_data_size",
                "pallet_data_size", "compression_kinds", "sections", "classification",
            ],
        )
        self.assertEqual(
            report["section_fields"],
            ["index", "numeric_type", "flags", "record_count", "field_count", "record_size", "string_block_size"],
        )
        self.assertEqual(report["classification_fields"], ["outcome", "reason_code", "reason"])
        self.assertEqual(report["numeric_section_type_codes"]["0"],
                         "regular WDC5 record variant; Header.Flags sparse bit clear")
        self.assertEqual(report["numeric_section_type_codes"]["1"],
                         "sparse WDC5 record variant; Header.Flags bit 0 set")
        self.assertIn("TACT value is not emitted", report["section_flags"])
        self.assertIn("CompressionType", report["compression_kinds"])
        self.assertIn("byte offset +8", report["compression_kinds"])
        self.assertIn("range 0..5", report["compression_kinds"])
        forbidden = " ".join(report["forbidden_fields"])
        for phrase in ["payload bytes", "CKey/EKey", "index prefixes", "archive offsets", "key material"]:
            self.assertIn(phrase, forbidden)

    def test_cli_refusals_and_synthetic_behavior_gates_are_pinned(self):
        main = (ROOT / "crates/forever-extractor/src/main.rs").read_text()
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        inspector = (ROOT / "crates/forever-extractor/src/driver/inspect_db2.rs").read_text()
        cli = (ROOT / "crates/forever-extractor/tests/inspect_db2_cli.rs").read_text()
        self.assertIn('"inspect-db2"', main)
        self.assertIn('"product_keys_forbidden"', main)
        self.assertIn('driver::inspect_db2_command', main)
        self.assertIn("pub(crate) mod inspect_db2", driver)
        for guard in [
            "plan.build != 70450",
            "WowDbDefs70450V1",
            "keys_supplied",
            "output_path(client, output)",
            "open_keyless_store_70450",
            "validate_db2(plan, table_name, bytes)",
            "atomic_write(client, &output",
        ]:
            self.assertIn(guard, inspector)
        for name in [
            "inspector_refuses_wrong_profile_keys_and_unbounded_or_unknown_ids",
            "output_path_refuses_client_and_git_checkout_descendants",
            "synthetic_sparse_parent_matches_real_shape_and_is_redacted_deterministic",
            "synthetic_unsupported_sparse_common_data_is_reported_as_rejected",
            "compression_kinds_are_fixed_offset_numeric_codes_filtered_to_reader_range",
        ]:
            self.assertIn(name, inspector)
        for name in [
            "inspect_db2_refuses_keys_and_non_70450_plans_before_client_access",
            "inspect_db2_refuses_client_and_git_checkout_output_paths",
        ]:
            self.assertIn(name, cli)

    def test_sparse_parent_failure_site_and_bounded_reader_are_explicit(self):
        records = (ROOT / "crates/forever-db2/src/records.rs").read_text()
        sparse = (ROOT / "crates/forever-db2/src/records/sparse.rs").read_text()
        inspector = (ROOT / "crates/forever-extractor/src/driver/inspect_db2.rs").read_text()
        self.assertIn("catalog::WOWDBDEFS_70450_CATALOG_ID", records)
        self.assertIn("StringMode::OpaqueBytes => Err(RecordError::Unsupported)", records)
        self.assertIn("schema.parent >= 0 && !appended_parent", sparse)
        self.assertIn("sparse parent field is outside the build-70450 raw-source allowlist", inspector)
        self.assertIn("unsupported_record_variant", inspector)
        sparse_tests = (ROOT / "crates/forever-db2/src/records/sparse/tests.rs").read_text()
        shared_fixture = (ROOT / "crates/forever-db2/src/records/sparse/synthetic_fixture.rs").read_text()
        self.assertIn("build70450_sparse_raw_source_allowlist_rejects_nonmembers_exhaustively", sparse_tests)
        self.assertIn("synthetic_fixture.rs", sparse_tests)
        self.assertIn("synthetic_fixture.rs", inspector)
        self.assertNotIn("fn synthetic_parent_sparse", inspector)
        self.assertIn("empty_parented_sparse_70450", shared_fixture)

    def test_build70450_high_volume_profile_allowlist_and_default_policy_are_pinned(self):
        records = (ROOT / "crates/forever-db2/src/records.rs").read_text()
        docs = (ROOT / "docs/forever-product.md").read_text()
        self.assertIn(
            '"wowdbdefs-70450-high-volume-regular-v1"', records
        )
        self.assertIn("TARGET_HIGH_VOLUME_70450_PROFILE", records)
        self.assertIn("target_high_volume_limits(catalog_id, table_name)", records)
        self.assertIn("high_volume_profile_tests", records)
        self.assertIn("Build-70450 high-volume DB2 profile", docs)


if __name__ == "__main__":
    unittest.main()
