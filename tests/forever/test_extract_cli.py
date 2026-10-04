"""Synthetic CLI checks for forever-db2-extract; no real client or key fixtures.

The binary requires the `native` feature, so these checks are skipped unless the
operator selects `FOREVER_DB2_EXTRACT`. Cases that need an installed client are
additionally gated on `FOREVER_CASC_CLIENT`.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

BINARY = os.environ.get("FOREVER_DB2_EXTRACT")
CLIENT = os.environ.get("FOREVER_CASC_CLIENT")

BUILD_INFO = (
    "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|"
    "CDN Key!HEX:16|Tags!STRING:0\n"
)


@unittest.skipUnless(BINARY, "extract binary not selected")
class ExtractCliSynthetic(unittest.TestCase):
    def invoke(self, args):
        result = subprocess.run([BINARY, *args], capture_output=True, text=True, check=False)
        self.assertEqual(result.stderr, "")
        return result.returncode, json.loads(result.stdout), result.stdout

    def test_invalid_arguments_exit_two_and_are_redacted(self):
        for args in (
            [],
            ["--client"],
            ["--client", "/private/secret", "--locale"],
            ["--client", "/private/secret", "--locale", "enUS", "--client", "/private/second"],
            ["--private-option", "secret"],
        ):
            with self.subTest(args=args):
                code, report, raw = self.invoke(args)
                self.assertEqual(code, 2)
                self.assertEqual(report["report_version"], 1)
                self.assertEqual(report["code"], "invalid_invocation")
                self.assertNotIn("/private/secret", raw)

    def test_missing_build_info_is_io_not_output(self):
        with tempfile.TemporaryDirectory(prefix="forever-extract-") as directory:
            out = Path(directory) / "out"
            code, report, raw = self.invoke(
                ["--client", directory, "--locale", "enUS", "--output", str(out)]
            )
            self.assertEqual(code, 4)
            self.assertEqual(report["code"], "input_io")
            self.assertNotIn(directory, raw)
            self.assertFalse(out.exists())

    def test_unsupported_build_exit_three_creates_nothing(self):
        with tempfile.TemporaryDirectory(prefix="forever-extract-") as directory:
            (Path(directory) / ".build.info").write_text(
                BUILD_INFO + "wow_classic_beta|1|1.60.1.70171|"
                "00000000000000000000000000000000|11111111111111111111111111111111|esES\n"
            )
            out = Path(directory) / "out"
            code, report, _ = self.invoke(
                ["--client", directory, "--locale", "enUS", "--output", str(out)]
            )
            self.assertEqual(code, 3)
            self.assertEqual(report["code"], "unsupported_client")
            self.assertFalse(out.exists())

    def test_help_is_standalone(self):
        result = subprocess.run([BINARY, "--help"], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stderr, "")
        self.assertNotIn("--file", result.stdout)


@unittest.skipUnless(BINARY and CLIENT, "real client not selected")
class ExtractCliLive(unittest.TestCase):
    def test_bounded_two_table_extraction_into_external_temp(self):
        with tempfile.TemporaryDirectory(prefix="forever-extract-live-") as directory:
            out = Path(directory) / "out"
            result = subprocess.run(
                [BINARY, "--client", CLIENT, "--locale", "esES", "--output", str(out)],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertIn(result.returncode, (0, 8))
            report = json.loads(result.stdout)
            self.assertEqual(report["report_version"], 1)
            self.assertFalse(report["full_client_extraction"])
            self.assertNotIn(CLIENT, result.stdout)


if __name__ == "__main__":
    unittest.main()
