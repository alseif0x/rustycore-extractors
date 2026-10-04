# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later
"""Contract checks for the Forever v2 manifest line.

This suite is deliberately separate from ``test_contracts.py``: Forever builds
70170/70205 use a NEW output contract. It must never re-label, merge with or
silently accept the legacy 3.4.3/51943 v1 data-manifest.
"""

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "contracts" / "fixtures"

EXPECTED_SCHEMA_ID = (
    "https://github.com/alseif0x/rustycore-extractors/schemas/"
    "forever-manifest-v2.schema.json"
)
ORACLE_COMMIT = "02245dcd245e7433e524577656177723d3e4992e"

# Reviewed real-client identities for the character tables. Format hashes (not
# client assets) pinned so future fixtures cannot invent plausible values.
# Key: (file_id, table_hash, layout_hash, meta_index_field, field_count).
REVIEWED_TABLE_IDENTITY = {
    "ChrClasses": (1361031, 0xF5889D8C, 0xAFC9B0C2, 29, 43),
    "ChrRaces": (1305311, 0x53F1783C, 0x4F44C796, -1, 51),
}


def load_json(path: Path):
    with path.open(encoding="utf-8") as source:
        return json.load(source)


class ForeverContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = load_json(ROOT / "contracts" / "forever-contract-v2.json")
        cls.schema = load_json(ROOT / "schemas" / "forever-manifest-v2.schema.json")
        # Legacy v1 line, loaded read-only to prove line separation.
        cls.legacy_schema = load_json(ROOT / "schemas" / "data-manifest-v1.schema.json")
        cls.complete = load_json(FIXTURES / "forever-manifest.complete.json")
        cls.incomplete = load_json(FIXTURES / "forever-manifest.incomplete.json")

        Draft202012Validator.check_schema(cls.schema)
        Draft202012Validator.check_schema(cls.legacy_schema)

        cls.validator = Draft202012Validator(cls.schema, format_checker=FormatChecker())
        cls.legacy_validator = Draft202012Validator(
            cls.legacy_schema, format_checker=FormatChecker()
        )

    def assertValid(self, validator, instance):
        errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.path))
        self.assertEqual([], errors, "\n".join(error.message for error in errors))

    def assertInvalid(self, validator, instance):
        self.assertNotEqual([], list(validator.iter_errors(instance)))

    def test_schema_identity_and_version(self):
        self.assertEqual(EXPECTED_SCHEMA_ID, self.schema["$id"])
        self.assertEqual(2, self.schema["properties"]["manifest_version"]["const"])
        self.assertFalse(self.schema["additionalProperties"])

    def test_fixtures_validate(self):
        self.assertValid(self.validator, self.complete)
        self.assertValid(self.validator, self.incomplete)
        self.assertEqual("complete", self.complete["status"])
        self.assertEqual("incomplete", self.incomplete["status"])

    def test_fixture_coherence_invariants(self):
        for manifest in (self.complete, self.incomplete):
            with self.subTest(run_id=manifest["run_id"]):
                artifacts = manifest["outputs"]["artifacts"]
                integrity = manifest["outputs"]["integrity"]
                complete_rows = [item for item in artifacts if item["status"] == "complete"]
                self.assertEqual(len(complete_rows), integrity["file_count"])
                self.assertEqual(
                    sum(item["bytes"] for item in complete_rows), integrity["total_bytes"]
                )
                # The incomplete fixture keeps an error row whose bytes are not
                # published, so only complete rows enter the inventory.

        self.assertFalse(self.complete["outputs"]["incomplete_marker_present"])
        self.assertNotIn("reason", self.complete)
        self.assertIsInstance(self.complete["timestamps"]["completed_at"], str)

        self.assertTrue(self.incomplete["outputs"]["incomplete_marker_present"])
        self.assertNotEqual("", self.incomplete["reason"])
        self.assertIsNone(self.incomplete["timestamps"]["completed_at"])

    def test_reviewed_table_identity(self):
        tables = {item["name"]: item for item in self.complete["formats"]["tables"]}
        for name, expected in REVIEWED_TABLE_IDENTITY.items():
            table = tables[name]
            actual = (
                table["file_id"],
                table["table_hash"],
                table["layout_hash"],
                table["meta_index_field"],
                table["field_count"],
            )
            self.assertEqual(expected, actual)

    def test_all_fixtures_match_reviewed_table_identity(self):
        # Any fixture carrying a reviewed table must publish its exact real
        # identity; this blocks fabricated hashes in future fixtures.
        for manifest in (self.complete, self.incomplete):
            tables = {item["name"]: item for item in manifest["formats"]["tables"]}
            for name, expected in REVIEWED_TABLE_IDENTITY.items():
                if name not in tables:
                    continue
                with self.subTest(run_id=manifest["run_id"], table=name):
                    table = tables[name]
                    actual = (
                        table["file_id"],
                        table["table_hash"],
                        table["layout_hash"],
                        table["meta_index_field"],
                        table["field_count"],
                    )
                    self.assertEqual(expected, actual)

    def test_build_version_pairing_rejects_crossed_pair(self):
        manifest = copy.deepcopy(self.incomplete)
        self.assertEqual(70170, manifest["client"]["build"])
        manifest["client"]["version"] = "1.60.1.70205"
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.complete)
        manifest["client"]["version"] = "1.60.1.70170"
        self.assertInvalid(self.validator, manifest)

    def test_wrong_product_rejected(self):
        manifest = copy.deepcopy(self.complete)
        manifest["client"]["product"] = "wow_classic"
        self.assertInvalid(self.validator, manifest)

    def test_network_must_stay_local(self):
        manifest = copy.deepcopy(self.complete)
        manifest["network"]["allow_network"] = True
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.complete)
        manifest["network"]["downloaded_bytes"] = 1
        self.assertInvalid(self.validator, manifest)

    def test_privacy_flags_are_const(self):
        manifest = copy.deepcopy(self.complete)
        manifest["privacy"]["paths_redacted"] = False
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.complete)
        manifest["privacy"]["client_data_embedded"] = True
        self.assertInvalid(self.validator, manifest)

    def test_error_artifact_requires_code(self):
        manifest = copy.deepcopy(self.incomplete)
        del manifest["outputs"]["artifacts"][0]["code"]
        self.assertInvalid(self.validator, manifest)

    def test_complete_artifact_rejects_code(self):
        manifest = copy.deepcopy(self.complete)
        manifest["outputs"]["artifacts"][0]["code"] = "row_decode_mismatch"
        self.assertInvalid(self.validator, manifest)

    def test_complete_rejects_marker_and_missing_completed_at(self):
        manifest = copy.deepcopy(self.complete)
        manifest["outputs"]["incomplete_marker_present"] = True
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.complete)
        manifest["timestamps"]["completed_at"] = None
        self.assertInvalid(self.validator, manifest)

    def test_non_complete_requires_marker_and_reason(self):
        manifest = copy.deepcopy(self.incomplete)
        manifest["outputs"]["incomplete_marker_present"] = False
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.incomplete)
        del manifest["reason"]
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.incomplete)
        manifest["reason"] = ""
        self.assertInvalid(self.validator, manifest)

    def test_manifest_version_one_rejected(self):
        manifest = copy.deepcopy(self.complete)
        manifest["manifest_version"] = 1
        self.assertInvalid(self.validator, manifest)

    def test_implementation_must_be_rust(self):
        manifest = copy.deepcopy(self.complete)
        manifest["extractor"]["implementation"] = "cpp"
        self.assertInvalid(self.validator, manifest)

    def test_relative_path_stays_local_simple_name(self):
        manifest = copy.deepcopy(self.complete)
        manifest["outputs"]["artifacts"][0]["relative_path"] = "../ChrClasses.db2"
        self.assertInvalid(self.validator, manifest)

        manifest = copy.deepcopy(self.complete)
        manifest["outputs"]["artifacts"][0]["relative_path"] = "out/ChrClasses.db2"
        self.assertInvalid(self.validator, manifest)

    def test_each_line_schema_rejects_the_other_line_fixtures(self):
        # The v1 legacy schema must not accept a Forever v2 manifest...
        self.assertInvalid(self.legacy_validator, self.complete)
        self.assertInvalid(self.legacy_validator, self.incomplete)
        # ...and the Forever v2 schema must not accept the legacy fixtures.
        legacy_complete = load_json(FIXTURES / "data-manifest.complete.json")
        legacy_incomplete = load_json(FIXTURES / "data-manifest.incomplete.json")
        self.assertInvalid(self.validator, legacy_complete)
        self.assertInvalid(self.validator, legacy_incomplete)

    def test_contract_document_fields(self):
        contract = self.contract
        self.assertEqual(2, contract["contract_version"])
        self.assertEqual("forever", contract["line"])
        self.assertEqual("forever-manifest.json", contract["manifest"]["published_name"])
        self.assertEqual("report.json", contract["manifest"]["report_name"])
        self.assertEqual(EXPECTED_SCHEMA_ID, contract["manifest"]["schema_id"])

        self.assertEqual("wow_classic_beta", contract["client"]["product"])
        self.assertEqual(
            [(70170, "1.60.1.70170"), (70205, "1.60.1.70205")],
            [(item["build"], item["version"]) for item in contract["client"]["admitted_builds"]],
        )
        self.assertEqual(["enUS", "esES"], contract["client"]["supported_locales"])
        self.assertEqual(ORACLE_COMMIT, self.complete["extractor"]["compatibility_oracle_commit"])

        self.assertEqual(256, contract["limits"]["max_entries"])
        self.assertEqual(16777216, contract["limits"]["max_file_bytes"])
        self.assertEqual(268435456, contract["limits"]["max_batch_bytes"])
        self.assertEqual("local", contract["network"]["mode"])
        self.assertFalse(contract["network"]["upload_permitted"])

    def test_contract_exit_codes_and_failure_codes(self):
        contract = self.contract
        self.assertEqual(
            [0, 2, 3, 4, 8, 9, 10],
            [item["code"] for item in contract["verification"]["exit_codes"]],
        )
        self.assertEqual(
            [
                "MANIFEST_SCHEMA_INVALID",
                "INVENTORY_MISMATCH",
                "TABLE_IDENTITY_MISMATCH",
                "UNSAFE_PATH",
                "INCOMPLETE_RUN",
                "ROW_DECODE_MISMATCH",
            ],
            contract["verification"]["failure_codes"],
        )
        bound = {item["code"]: item["exit"] for item in contract["verification"]["code_bindings"]}
        self.assertEqual(2, bound["MANIFEST_SCHEMA_INVALID"])
        self.assertEqual(8, bound["INCOMPLETE_RUN"])
        for code in ("INVENTORY_MISMATCH", "TABLE_IDENTITY_MISMATCH", "UNSAFE_PATH", "ROW_DECODE_MISMATCH"):
            self.assertEqual(9, bound[code])

    def test_contract_limits_match_schema_and_fixture(self):
        contract = self.contract
        options = self.schema["properties"]["options"]["properties"]
        self.assertEqual(contract["limits"]["max_file_bytes"], options["max_file_bytes"]["const"])
        self.assertEqual(contract["limits"]["max_batch_bytes"], options["max_batch_bytes"]["const"])
        self.assertEqual(
            contract["limits"]["max_entries"],
            options["entry_count"]["maximum"],
        )
        fixture_options = self.complete["options"]
        self.assertEqual(contract["limits"]["max_file_bytes"], fixture_options["max_file_bytes"])
        self.assertEqual(contract["limits"]["max_batch_bytes"], fixture_options["max_batch_bytes"])

    def test_synthetic_fixtures_do_not_disclose_local_paths(self):
        for path in sorted(FIXTURES.glob("forever-manifest.*.json")):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("/home/", text)
            self.assertNotIn("World of Warcraft", text)

    def test_optional_identity_fields_may_be_omitted(self):
        # run_id, the install fingerprint and the extractor git commit have no
        # source in the local batch runtime; they are optional, never invented.
        self.assertNotIn("run_id", self.schema["required"])
        self.assertNotIn(
            "install_fingerprint_sha256",
            self.schema["properties"]["client"]["required"],
        )
        self.assertNotIn("commit", self.schema["properties"]["extractor"]["required"])

        checks = [
            ("run_id", lambda m: m.pop("run_id")),
            ("install_fingerprint_sha256", lambda m: m["client"].pop("install_fingerprint_sha256")),
            ("commit", lambda m: m["extractor"].pop("commit")),
        ]
        for field, remove in checks:
            with self.subTest(field=field):
                manifest = copy.deepcopy(self.complete)
                remove(manifest)
                self.assertValid(self.validator, manifest)

        # A present value must still be well formed.
        manifest = copy.deepcopy(self.complete)
        manifest["client"]["install_fingerprint_sha256"] = "not-a-hash"
        self.assertInvalid(self.validator, manifest)

    def test_extractor_name_matches_emitted_document(self):
        self.assertEqual(
            "forever-db2-extract",
            self.schema["properties"]["extractor"]["properties"]["name"]["const"],
        )

    def test_forever_schema_has_no_v1_references(self):
        text = (ROOT / "schemas" / "forever-manifest-v2.schema.json").read_text(encoding="utf-8")
        self.assertNotIn("data-manifest-v1", text)
        self.assertNotIn('"supported"', text)


if __name__ == "__main__":
    unittest.main()
