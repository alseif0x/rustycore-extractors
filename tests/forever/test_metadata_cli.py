"""Synthetic CLI checks; no client data, sockets or Rust dependencies downloaded."""
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest


class MetadataCli(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.binary = os.environ["FOREVER_DB2_INSPECT"]

    def invoke(self, args):
        return subprocess.run([self.binary, *args], capture_output=True, text=True, check=False)

    def inspect_bytes(self, data):
        with tempfile.TemporaryDirectory(prefix="forever-synthetic-") as directory:
            path = Path(directory) / "private-name.db2"
            path.write_bytes(data)
            result = self.invoke(["--file", str(path), "--json"])
            self.assertEqual(path.read_bytes(), data)
            self.assertEqual(sorted(p.name for p in Path(directory).iterdir()), [path.name])
            self.assertNotIn(directory, result.stdout + result.stderr)
            self.assertNotIn(path.name, result.stdout + result.stderr)
            self.assertEqual(result.stderr, "")
            return result.returncode, json.loads(result.stdout)

    @staticmethod
    def header(sections=0):
        data = bytearray(204 + 40 * sections)
        data[:4] = b"WDC5"
        struct.pack_into("<I", data, 4, 5)
        struct.pack_into("<I", data, 200, sections)
        return data

    def test_structured_metadata_and_non_claims(self):
        data = self.header(1)
        struct.pack_into("<h", data, 174, -1)
        struct.pack_into("<I", data, 136, 0x80000020)
        struct.pack_into("<Q", data, 204, 0xFFFFFFFFFFFFFFFF)
        code, report = self.inspect_bytes(data)
        self.assertEqual(code, 0)
        self.assertEqual(report["scope"], "metadata_only")
        self.assertEqual(report["header"]["index_field"], -1)
        self.assertEqual(report["header"]["record_count"], 0x80000020)
        self.assertTrue(report["sections"][0]["tact_declared"])
        self.assertNotIn("tact_id", report["sections"][0])
        for field in ("client_build_verified", "table_schema_validated", "record_data_decoded"):
            self.assertFalse(report[field])
        self.assertTrue(report["offline"])
        self.assertTrue(report["read_only"])

    def test_bad_signature_version_truncation_parent_and_limit(self):
        cases = [(b"", 4, "metadata_truncated")]
        for at, value, code, diagnostic in [
            (0, int.from_bytes(b"WDC4", "little"), 3, "unsupported_signature"),
            (4, 4, 3, "unsupported_version"),
            (184, 2, 4, "invalid_parent_lookup_count"),
            (200, 1025, 6, "section_policy_limit"),
        ]:
            data = self.header()
            struct.pack_into("<I", data, at, value)
            cases.append((data, code, diagnostic))
        for data, code, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                actual, report = self.inspect_bytes(data)
                self.assertEqual(actual, code)
                self.assertEqual(report["status"], "error")
                self.assertEqual(report["code"], diagnostic)

    def test_argument_errors_are_json_and_redacted(self):
        for args in [
            ["--json"], ["--file", "--json"],
            ["--file", "x", "--file", "private-second", "--json"],
            ["--file", "x", "--json", "--json"],
            ["--file", "x", "--private-secret-option", "--json"],
        ]:
            result = self.invoke(args)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(json.loads(result.stdout)["code"], "invalid_invocation")
            self.assertEqual(result.stderr, "")
            self.assertNotIn("private", result.stdout)

    def test_missing_file_reports_safe_io_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            result = self.invoke(["--file", str(Path(directory) / "missing"), "--json"])
            self.assertEqual(result.returncode, 4)
            self.assertEqual(json.loads(result.stdout)["code"], "input_io")
            self.assertNotIn(directory, result.stdout)
            self.assertEqual(result.stderr, "")

    def test_help_is_standalone_and_no_implicit_current_directory_input(self):
        self.assertEqual(self.invoke(["--help"]).returncode, 0)
        self.assertEqual(self.invoke([]).returncode, 2)
        self.assertEqual(self.invoke(["--help", "--json"]).returncode, 2)


if __name__ == "__main__":
    unittest.main()
