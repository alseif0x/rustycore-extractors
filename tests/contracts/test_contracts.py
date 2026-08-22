# Copyright (C) 2026 rustycore-extractors contributors
# SPDX-License-Identifier: GPL-3.0-or-later

import copy
import json
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "contracts" / "fixtures"


def load_json(path: Path):
    with path.open(encoding="utf-8") as source:
        return json.load(source)


class ContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = load_json(ROOT / "contracts" / "extractor-contract-v1.json")
        cls.network_lock = load_json(ROOT / "contracts" / "network-lock-v1.json")
        cls.manifest_schema = load_json(ROOT / "schemas" / "data-manifest-v1.schema.json")
        cls.result_schema = load_json(ROOT / "schemas" / "command-result-v1.schema.json")
        cls.network_schema = load_json(ROOT / "schemas" / "network-lock-v1.schema.json")
        cls.complete_manifest = load_json(FIXTURES / "data-manifest.complete.json")
        cls.incomplete_manifest = load_json(FIXTURES / "data-manifest.incomplete.json")

        for schema in (cls.manifest_schema, cls.result_schema, cls.network_schema):
            Draft202012Validator.check_schema(schema)

        cls.manifest_validator = Draft202012Validator(
            cls.manifest_schema, format_checker=FormatChecker()
        )
        cls.result_validator = Draft202012Validator(cls.result_schema)
        cls.network_validator = Draft202012Validator(cls.network_schema)

    def assertValid(self, validator, instance):
        errors = sorted(validator.iter_errors(instance), key=lambda error: list(error.path))
        self.assertEqual([], errors, "\n".join(error.message for error in errors))

    def assertInvalid(self, validator, instance):
        self.assertNotEqual([], list(validator.iter_errors(instance)))

    def test_manifest_fixtures_validate(self):
        self.assertValid(self.manifest_validator, self.complete_manifest)
        self.assertValid(self.manifest_validator, self.incomplete_manifest)

        for manifest in (self.complete_manifest, self.incomplete_manifest):
            artifacts = manifest["outputs"]["artifacts"]
            integrity = manifest["outputs"]["integrity"]
            self.assertEqual(sum(item["file_count"] for item in artifacts), integrity["file_count"])
            self.assertEqual(sum(item["total_bytes"] for item in artifacts), integrity["total_bytes"])

    def test_command_result_fixtures_validate(self):
        for path in sorted(FIXTURES.glob("command-result.*.json")):
            with self.subTest(path=path.name):
                self.assertValid(self.result_validator, load_json(path))

    def test_supported_manifest_rejects_wrong_build(self):
        manifest = copy.deepcopy(self.complete_manifest)
        manifest["client"]["build"] = 99999
        self.assertInvalid(self.manifest_validator, manifest)

    def test_experimental_manifest_requires_reason(self):
        manifest = copy.deepcopy(self.incomplete_manifest)
        del manifest["compatibility"]["reason"]
        self.assertInvalid(self.manifest_validator, manifest)

    def test_complete_manifest_rejects_incomplete_marker(self):
        manifest = copy.deepcopy(self.complete_manifest)
        manifest["outputs"]["incomplete_marker_present"] = True
        self.assertInvalid(self.manifest_validator, manifest)

    def test_remote_casc_requires_network_and_lock(self):
        manifest = copy.deepcopy(self.complete_manifest)
        manifest["network"]["mode"] = "remote-casc"
        manifest["network"]["remote_casc"] = True
        self.assertInvalid(self.manifest_validator, manifest)

    def test_contract_constants_match_manifest_fixture(self):
        formats = self.contract["formats"]
        manifest_formats = self.complete_manifest["formats"]
        self.assertEqual(formats["map"]["magic_ascii"], manifest_formats["map"]["magic"])
        self.assertEqual(formats["map"]["version"], manifest_formats["map"]["version"])
        self.assertEqual(formats["vmap"]["magic_ascii"], manifest_formats["vmap"]["magic"])
        self.assertEqual(formats["vmap"]["raw_magic_ascii"], manifest_formats["vmap"]["raw_magic"])
        self.assertEqual(formats["mmap"]["version"], manifest_formats["mmap"]["version"])
        self.assertEqual(
            formats["mmap"]["detour_navmesh_version"],
            manifest_formats["mmap"]["detour_navmesh_version"],
        )
        schema_exit_codes = self.result_schema["properties"]["exit_code"]["enum"]
        contract_exit_codes = [item["code"] for item in self.contract["exit_codes"]]
        self.assertEqual(contract_exit_codes, schema_exit_codes)

    def test_manifest_rejects_unknown_fields(self):
        manifest = copy.deepcopy(self.complete_manifest)
        manifest["client"]["absolute_path"] = "/synthetic/client"
        self.assertInvalid(self.manifest_validator, manifest)

    def test_default_commands_have_no_network_destination(self):
        self.assertEqual("offline", self.contract["network"]["default_mode"])
        self.assertFalse(self.contract["network"]["uploads_permitted"])
        for command in self.contract["commands"].values():
            self.assertEqual("offline", command["network_default"])

        self.assertValid(self.network_validator, self.network_lock)
        self.assertEqual([], self.network_lock["metadata_resources"])
        self.assertEqual([], self.network_lock["remote_casc_profiles"])
        self.assertEqual("reject-network", self.contract["network"]["empty_lock_behavior"])

    def test_synthetic_fixtures_do_not_disclose_local_paths(self):
        for path in sorted(FIXTURES.glob("*.json")):
            text = path.read_text(encoding="utf-8")
            self.assertNotIn("/home/", text)
            self.assertNotIn("World of Warcraft", text)


if __name__ == "__main__":
    unittest.main()
