"""Opt-in read-only client checks; no client bytes or paths become fixtures.

Set FOREVER_CASC_CLIENT, FOREVER_CASC_EXPECTED_BUILD and FOREVER_CASC_PROBE.
strace blocks every network syscall and the checks also reject attempted use.
These are live smoke checks, not an independent byte/schema conformance oracle.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


@unittest.skipUnless(os.environ.get("FOREVER_CASC_CLIENT"), "private live client not selected")
class LiveCascCli(unittest.TestCase):
    def invoke(self, file_id, *extra):
        with tempfile.TemporaryDirectory(prefix="forever-casc-network-") as directory:
            trace = Path(directory) / "network.trace"
            result = subprocess.run([
                "strace", "-qq", "-f", "-e", "trace=network", "-e",
                "inject=network:error=EPERM", "-o", str(trace),
                os.environ["FOREVER_CASC_PROBE"], "--client",
                os.environ["FOREVER_CASC_CLIENT"], "--locale", "esES",
                "--file-id", str(file_id), "--json", *extra,
            ], capture_output=True, text=True, check=False, timeout=60)
            # Never include private command/path/trace contents in assertions.
            self.assertEqual(result.stderr, "")
            network = trace.read_text()
            self.assertFalse(any(token in network for token in
                                 ("socket(", "connect(", "send", "recv", "bind(", "listen(", "accept(")))
            report = json.loads(result.stdout)
            self.assertNotIn(os.environ["FOREVER_CASC_CLIENT"], result.stdout)
            return result.returncode, report

    def test_complete_reads_report_selected_build_without_schema_claims(self):
        for file_id in (1361031, 1305311):
            with self.subTest(file_id=file_id):
                code, report = self.invoke(file_id)
                self.assertEqual(code, 0)
                self.assertEqual(report["build"], int(os.environ["FOREVER_CASC_EXPECTED_BUILD"]))
                self.assertEqual(report["file_id"], file_id)
                self.assertGreater(report["bytes"], 204)
                self.assertEqual(report["magic"], "WDC5")
                self.assertFalse(report["table_schema_validated"])
                self.assertFalse(report["record_data_decoded"])
                self.assertIsNone(report["reference_bytes_equal"])

    def test_budget_never_exposes_partial_success(self):
        code, report = self.invoke(1361031, "--max-bytes", "1")
        self.assertEqual(code, 6)
        self.assertEqual(report["code"], "read_policy_limit")
        self.assertNotIn("bytes", report)

    def test_missing_file_is_explicit(self):
        code, report = self.invoke(4294967294)
        self.assertEqual(code, 4)
        self.assertEqual(report["code"], "file_unavailable")
        self.assertNotIn("bytes", report)


if __name__ == "__main__":
    unittest.main()
