"""Synthetic native CLI admission checks. No real client or key fixtures."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


@unittest.skipUnless(os.environ.get("FOREVER_CASC_PROBE"), "native prototype not selected")
class CascCli(unittest.TestCase):
    def invoke(self, args):
        result = subprocess.run([os.environ["FOREVER_CASC_PROBE"], *args, "--json"],
                                capture_output=True, text=True, check=False)
        self.assertEqual(result.stderr, "")
        return result.returncode, json.loads(result.stdout)

    def test_unsupported_client_rejected_before_native_storage_access(self):
        with tempfile.TemporaryDirectory(prefix="private-casc-") as directory:
            path = Path(directory) / ".build.info"
            data = "Product|Active|Version\nwow_classic_beta|1|1.60.1.70171\n"
            path.write_text(data)
            code, report = self.invoke(["--client", directory, "--locale", "esES", "--file-id", "1361031"])
            self.assertEqual(code, 3)
            self.assertEqual(report["code"], "unsupported_client")
            self.assertNotIn(directory, json.dumps(report))
            self.assertEqual(path.read_text(), data)
            self.assertEqual([p.name for p in Path(directory).iterdir()], [".build.info"])

    def test_missing_build_info_redacted(self):
        with tempfile.TemporaryDirectory(prefix="private-casc-") as directory:
            code, report = self.invoke(["--client", directory, "--locale", "enUS", "--file-id", "1"])
            self.assertEqual(code, 4)
            self.assertEqual(report["code"], "input_io")
            self.assertNotIn(directory, json.dumps(report))

    def test_valid_metadata_but_missing_storage_fails_without_output(self):
        for build in (70170, 70205):
            with self.subTest(build=build):
                self.check_missing_storage(build)

    def check_missing_storage(self, build):
        with tempfile.TemporaryDirectory(prefix="private-casc-") as directory:
            (Path(directory) / ".build.info").write_text(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\n"
                f"wow_classic_beta|1|1.60.1.{build}|00000000000000000000000000000000|11111111111111111111111111111111|esES\n")
            code, report = self.invoke(["--client", directory, "--locale", "enUS", "--file-id", "1"])
            self.assertEqual(code, 4)
            self.assertEqual(report["status"], "error")
            self.assertNotIn(directory, json.dumps(report))
            self.assertEqual([p.name for p in Path(directory).iterdir()], [".build.info"])

    def test_incomplete_native_metadata_rejected_before_library(self):
        with tempfile.TemporaryDirectory(prefix="private-casc-") as directory:
            (Path(directory) / ".build.info").write_text(
                "Product|Active|Version\nwow_classic_beta|1|1.60.1.70170\n")
            code, report = self.invoke(["--client", directory, "--locale", "enUS", "--file-id", "1"])
            self.assertEqual(code, 4)
            self.assertEqual(report["code"], "invalid_build_metadata")

    def test_invalid_options_do_not_echo_values(self):
        for args in [[], ["--private-option", "secret"],
                     ["--client", "secret", "--locale", "zzZZ", "--file-id", "1"]]:
            code, report = self.invoke(args)
            self.assertEqual(code, 2)
            self.assertEqual(report["code"], "invalid_invocation")
            self.assertNotIn("secret", json.dumps(report))


if __name__ == "__main__":
    unittest.main()
