"""Synthetic CLI checks for forever-db2-extract; no real client or key fixtures.

Select `FOREVER_DB2_EXTRACT` for native compatibility and explicit Rust-local
CLI checks. Optional `FOREVER_DB2_EXTRACT_NO_NATIVE` covers capability rejection.
Cases that need an installed client are
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

    def test_catalog_requires_explicit_manifest_and_exact_selector(self):
        base = ["--client", "/private/catalog-client", "--locale", "enUS", "--output", "/private/catalog-output"]
        for extra in (
            ["--schema-catalog", "pinned-metadata-v1"],
            ["--manifest", "/private/plan", "--schema-catalog", "unknown"],
            ["--manifest", "/private/plan", "--schema-catalog", "pinned-metadata-v1", "--schema-catalog", "pinned-metadata-v1"],
        ):
            with self.subTest(extra=extra):
                code, report, raw = self.invoke(base + extra)
                self.assertEqual((code, report['code']), (2, 'invalid_invocation'))
                self.assertNotIn('/private/', raw)

    def test_catalog_unknown_case_and_wrong_id_fail_before_storage_or_output(self):
        with tempfile.TemporaryDirectory(prefix="forever-catalog-cli-") as directory:
            root = Path(directory)
            (root / '.build.info').write_text(BUILD_INFO + 'wow_classic_beta|1|1.60.1.70205|'
                '00000000000000000000000000000000|11111111111111111111111111111111|enUS\n')
            plan = root / 'plan.tsv'
            output = root / 'output'
            for text in ['LiquidType\t1\n', 'liquidtype\t1371380\n', 'UnknownTable\t123\n']:
                with self.subTest(plan=text):
                    plan.write_text(text)
                    code, report, raw = self.invoke(['--client', directory, '--locale', 'enUS', '--output', str(output), '--manifest', str(plan), '--schema-catalog', 'pinned-metadata-v1'])
                    self.assertEqual((code, report['code']), (2, 'invalid_manifest'))
                    self.assertFalse(output.exists())
                    self.assertNotIn(directory, raw)


@unittest.skipUnless(BINARY, "extract binary not selected")
class ExtractRustLocalCliSynthetic(unittest.TestCase):
    def invoke(self, args):
        result = subprocess.run([BINARY, *args], capture_output=True, text=True, check=False)
        self.assertEqual(result.stderr, "")
        return result.returncode, json.loads(result.stdout), result.stdout

    def test_rust_bootstrap_failure_has_v2_profile_and_creates_no_output(self):
        with tempfile.TemporaryDirectory(prefix="forever-rust-db2-") as directory:
            root = Path(directory)
            output = root / 'output'
            code, report, raw = self.invoke(['--client', str(root), '--locale', 'enUS', '--output', str(output),
                '--backend', 'rust-local', '--index-layout', 'contiguous', '--root-layout', 'header-50893-v2'])
            self.assertEqual((code, report['code']), (4, 'local_store_input_io'))
            self.assertEqual(report['report_version'], 2)
            self.assertEqual(report['acquisition']['backend'], 'rust-local')
            self.assertEqual(report['acquisition']['limits']['file_bytes'], 16777216)
            self.assertFalse(output.exists())
            self.assertNotIn(directory, raw)

    def test_catalog_manifest_rejects_before_rust_store_bootstrap(self):
        with tempfile.TemporaryDirectory(prefix="forever-rust-db2-plan-") as directory:
            root = Path(directory)
            plan = root / 'plan.tsv'
            plan.write_text('LiquidType\t1\n')
            output = root / 'output'
            code, report, raw = self.invoke(['--client', str(root), '--locale', 'enUS', '--output', str(output),
                '--manifest', str(plan), '--schema-catalog', 'pinned-metadata-v1', '--backend', 'rust-local',
                '--index-layout', 'paged', '--root-layout', 'header-30080', '--max-file-bytes', '1024'])
            self.assertEqual((code, report['code']), (2, 'invalid_manifest'))
            self.assertEqual(report['report_version'], 2)
            self.assertEqual(report['acquisition']['limits']['file_bytes'], 1024)
            self.assertFalse(output.exists())
            self.assertNotIn(directory, raw)

    def test_backend_options_are_explicit_and_cannot_enlarge_the_file_cap(self):
        base = ['--client', '/private/client', '--locale', 'enUS', '--output', '/private/output']
        for extra in (['--backend', 'rust-local'], ['--index-layout', 'paged'], ['--max-file-bytes', '1'],
                      ['--backend', 'native', '--root-layout', 'header-30080'],
                      ['--backend', 'rust-local', '--index-layout', 'paged', '--root-layout', 'header-30080', '--max-file-bytes', '16777217']):
            code, report, raw = self.invoke(base + extra)
            self.assertEqual((code, report['code']), (2, 'invalid_invocation'))
            self.assertNotIn('/private/', raw)


@unittest.skipUnless(os.environ.get('FOREVER_DB2_EXTRACT_NO_NATIVE'), 'non-native extract binary not selected')
class ExtractNoNativeCliSynthetic(unittest.TestCase):
    def test_default_and_explicit_native_reject_before_client_manifest_or_output_io(self):
        binary = os.environ['FOREVER_DB2_EXTRACT_NO_NATIVE']
        with tempfile.TemporaryDirectory(prefix='forever-no-native-') as directory:
            output = Path(directory) / 'output'
            base = ['--client', '/private/missing-client', '--locale', 'enUS', '--output', str(output), '--manifest', '/private/missing-plan']
            for extra in ([], ['--backend', 'native']):
                result = subprocess.run([binary, *base, *extra], capture_output=True, text=True, check=False)
                self.assertEqual(result.stderr, '')
                report = json.loads(result.stdout)
                self.assertEqual((result.returncode, report['code']), (2, 'backend_unavailable'))
                self.assertEqual(report['report_version'], 1)
                self.assertFalse(output.exists())
                self.assertNotIn('/private/', result.stdout)


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
