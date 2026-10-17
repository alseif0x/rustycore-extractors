# SPDX-License-Identifier: GPL-3.0-or-later
"""Public build-70425 collision capability and authored plan examples."""
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ProductV14Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v14.json").read_bytes())

    def test_explicit_collision_scope(self):
        c = self.contract
        self.assertEqual(
            (c["version"], c["builds"], c["platform"], c["backend"]),
            (14, [70425], "linux", "rust-local"))
        self.assertEqual(c["plan"]["component_mask"], 16)
        self.assertEqual(c["plan"]["components"], ["vmaps"])
        self.assertEqual(c["commands"], [
            "inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(c["plan"]["collision_budget_keys"], [
            "collision-raw-bytes",
            "collision-wdt-bytes",
            "collision-cache-bytes",
            "collision-placement-bytes",
            "collision-vmo-bytes",
            "collision-map-bytes",
            "collision-output-bytes",
        ])
        self.assertEqual(
            c["plan"]["collision_profile"],
            "linux70425-compatible-gnu15-vmap-product-v1")
        self.assertEqual(c["plan"]["db2_max_entries"], 1161)
        expected_numeric_limits = {
            "coverage_bytes": 134217728,
            "coverage_maps": 4096,
            "coverage_files": 262144,
            "package_files": 262144,
            "collision_record_bytes_min": 32,
            "collision_record_bytes_max": 137438953472,
            "collision_output_bytes_min": 1,
            "collision_output_bytes_max": 68719476736,
            "disk_world_combined_bytes": 1860173824,
            "disk_world_run_allowance_bytes": 270532608,
            "disk_world_guard_allowance_bytes": 16777216,
            "coverage_heap_bytes": 134238208,
            "compatible_phase_logical_memory_ceiling_bytes": 1725935616,
            "disk_headroom_bytes": 67108864,
        }
        self.assertIn("meaning", c["limits"])
        self.assertEqual(
            {key: value for key, value in c["limits"].items()
             if key != "meaning"},
            expected_numeric_limits)
        self.assertEqual(c["plan"]["decryption"], "disabled")
        self.assertEqual(c["plan"]["navigation"], "none")
        self.assertEqual(
            c["plan"]["collision_storage"],
            "linux-leased-append-disk-world-v1")
        self.assertFalse(c["full_extractor_complete"])
        self.assertIn("native_failure_parity", c["source_policy"])
        self.assertFalse(c["source_policy"]["native_failure_parity"])
        self.assertEqual(c["source_profile"], "linux70425-compatible-gnu15-v2")
        self.assertFalse(c["coverage"]["complete_global_map_coverage_claim"])
        self.assertEqual(c["coverage"]["authentication"], "producer_reported")
        self.assertIn("native backend", c["excluded"])
        self.assertIn("Windows execution", c["excluded"])
        self.assertIn("real full-world acceptance", c["excluded"])
        self.assertIn("RustyCore runtime", c["excluded"])
        self.assertIn("release acceptance", c["excluded"])
        self.assertIn("no older-build acceptance transfers to 70425",
                      c["legacy_preservation"])

    def test_canonical_examples(self):
        plan_source = (ROOT / "crates/forever-extractor/src/plan/v14_tests.rs").read_text()
        literals = re.findall(
            r'pub\(crate\) const V14: &str = "((?:\\.|[^"\\])*)";',
            plan_source)
        self.assertEqual(len(literals), 1)
        literal = literals[0]
        self.assertNotRegex(literal, r"\\(?![tn])")
        canonical = literal.replace(r"\t", "\t").replace(r"\n", "\n")
        self.assertTrue(canonical.endswith("\n"))

        expected_selections = {
            "all-available-maps": "all-available-maps",
            "map0": "map:0",
        }
        for name, selection in (("all-available-maps", "all-available-maps"),
                                ("map0", "map:0")):
            expected_selection = expected_selections[name]
            raw = (ROOT / f"tests/contracts/fixtures/forever-product-v14/{name}.tsv").read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            pairs = [line.split("\t") for line in raw.decode("ascii").splitlines()]
            self.assertEqual(len(pairs), 26)
            self.assertTrue(all(len(pair) == 2 for pair in pairs))
            fields = dict(pairs)
            self.assertEqual(len(fields), 26)
            expected = {
                "forever-product": "14",
                "build": "70425",
                "locale": "esES",
                "index": "contiguous",
                "root": "header-50893-v2",
                "schema-catalog": "wowdbdefs-70425-v1",
                "decryption": "disabled",
                "collision-storage": "linux-leased-append-disk-world-v1",
                "collision-raw-bytes": "1048576",
                "collision-wdt-bytes": "1048576",
                "collision-cache-bytes": "1048576",
                "collision-placement-bytes": "1048576",
                "collision-vmo-bytes": "1048576",
                "collision-map-bytes": "1048576",
                "collision-output-bytes": "1048576",
                "collision-profile": "linux70425-compatible-gnu15-vmap-product-v1",
                "collision-domain": expected_selection,
                "collision-coverage": "compatible-collision-coverage-v1",
                "components": "vmaps",
                "terrain": "none",
                "pack": "true",
                "navigation": "none",
                "big-base-unit": "false",
                "skip-liquids": "false",
                "maximum-slope": "55",
                "non-steep-slope": "55",
            }
            self.assertEqual(selection, expected_selection)
            self.assertEqual(set(fields), set(expected))
            self.assertEqual(fields, expected)
            self.assertEqual(fields["collision-domain"], expected_selection)
            expected_source = canonical
            if name == "map0":
                self.assertEqual(expected_selection, "map:0")
                expected_source = canonical.replace(
                    "collision-domain\tall-available-maps\n",
                    "collision-domain\tmap:0\n")
            self.assertEqual(raw, expected_source.encode("ascii"))

    def test_verification_coverage_and_source_literals(self):
        c = self.contract
        plan = (ROOT / "crates/forever-extractor/src/plan.rs").read_text()
        self.assertEqual(c["verification"]["profile"],
                         "linux70425-compatible-gnu15-vmap-product-v1")
        self.assertEqual(
            c["verification"]["scope"],
            "artifact_integrity_complete_declared_vmap_associations_and_producer_reported_available_source_coverage")
        self.assertIn(f'"{c["verification"]["profile"]}"', plan)
        self.assertIn(f'"{c["verification"]["scope"]}"', plan)
        self.assertIn("pub const MAX_70425_DB2_ENTRIES: usize = 1161;", plan)
        self.assertIn("WowDbDefs70425V1", plan)
        disk_plan = (ROOT / "crates/forever-extractor/src/plan/disk.rs").read_text()
        self.assertIn(
            'field(lines, "collision-storage")? != "linux-leased-append-disk-world-v1"',
            disk_plan)
        self.assertIn("Self::LeasedDiskV1(budget)", disk_plan)
        coverage = (ROOT / "crates/forever-extractor/src/collision_coverage.rs").read_text()
        self.assertIn(c["package"]["coverage"], coverage)
        self.assertIn('const MAGIC: &[u8; 8] = b"RCVMCV1\\0";', coverage)
        self.assertEqual(c["coverage"]["authentication"], "producer_reported")
        self.assertEqual(c["coverage"]["binds"], [
            "source", "plan", "producer", "build", "locale", "selection", "pack"])
        self.assertFalse(c["coverage"]["complete_global_map_coverage_claim"])
        self.assertIn("producer-reported source coverage, not an independent client snapshot",
                      c["verification"]["source_authentication"])
        self.assertIn("malformed or otherwise fails", c["source_policy"]["dependencies"])
        self.assertIn("has unknown coverage and emits no tree or placement proof",
                      c["source_policy"]["failure"])
        self.assertIn("without falling back", c["source_policy"]["failure"])
        self.assertIn("genuine six-owner result", c["resume"])
        self.assertIn("complete output refuses writes", c["resume"])
        self.assertIn("downward-only API budgets", c["limits"]["meaning"])
        self.assertIn("never truncate output", c["limits"]["meaning"])

    def test_v14_help_capability_and_verify_dispatch(self):
        main = (ROOT / "crates/forever-extractor/src/main.rs").read_text()
        self.assertIn(
            "Linux v14 separately selects disabled-key70425 collision-only GNU15-v2 leased disk, genuine available-source coverage and complete declared VMap association verification; extract/resume require fresh private scratch and replay every final artifact. No runtime/full-client claim.",
            main)
        self.assertIn(
            r',\"collision_source_profile\":\"linux70425-compatible-gnu15-v2\",\"coverage_authentication\":\"producer_reported\",\"whole_client_map_coverage\":false',
            main)
        verify = (ROOT / "crates/forever-extractor/src/verify.rs").read_text()
        self.assertIn("if plan.is_v14()", verify)
        self.assertIn("return compatible_vmaps::verify(plan, output, expected);", verify)

    def test_packager_carries_v14_source_closure(self):
        spec = importlib.util.spec_from_file_location(
            "v14_packager", ROOT / "tools/package_forever.py")
        packager = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(packager)
        selected = set(packager.selected_paths(ROOT))
        for name in (
                "contracts/forever-product-contract-v14.json",
                "tests/contracts/test_forever_product_v14_contract.py",
                "tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv",
                "tests/contracts/fixtures/forever-product-v14/map0.tsv",
                "docs/forever-product-v14.md",
                "crates/forever-extractor/src/plan/v14_tests.rs",
                "crates/forever-extractor/src/collision_coverage.rs",
                "crates/forever-extractor/src/verify/compatible_vmaps.rs",
                "crates/forever-extractor/src/verify/compatible_vmaps/tests.rs",
                "crates/forever-extractor/src/driver/disk/compatible.rs",
                "crates/forever-extractor/src/driver/disk/compatible/tests.rs",
                "crates/forever-extractor/src/driver/disk/compatible/fixture.rs",
                "crates/forever-extractor/tests/product_v14_cli.rs",
                "crates/forever-casc/src/collision/run/compatible_inventory/coverage.rs"):
            self.assertIn(name, selected)
            self.assertTrue((ROOT / name).is_file(), name)
            self.assertTrue(packager.bundle_path_allowed("source/" + name))


if __name__ == "__main__":
    unittest.main()
