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
        with tempfile.TemporaryDirectory(prefix="private-casc-") as directory:
            (Path(directory) / ".build.info").write_text(
                "Product|Active|Version\nwow_classic_beta|1|1.60.1.70170\n")
            code, report = self.invoke(["--client", directory, "--locale", "enUS", "--file-id", "1"])
            self.assertEqual(code, 4)
            self.assertEqual(report["status"], "error")
            self.assertNotIn(directory, json.dumps(report))
            self.assertEqual([p.name for p in Path(directory).iterdir()], [".build.info"])

    def test_invalid_options_do_not_echo_values(self):
        for args in [[], ["--private-option", "secret"],
                     ["--client", "secret", "--locale", "zzZZ", "--file-id", "1"]]:
            code, report = self.invoke(args)
            self.assertEqual(code, 2)
            self.assertEqual(report["code"], "invalid_invocation")
            self.assertNotIn("secret", json.dumps(report))


if __name__ == "__main__":
    unittest.main()
