# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-19: source boundary for rooted available selected navigation.
"""Static source contract only; no filesystem acceptance or runtime proof."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-selected-navigation-filesystem-contract-v1.json"


class SelectedNavigationFilesystemContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())
        self.available = (ROOT / "crates/forever-navigation/src/inventory/fs/available.rs").read_text()
        self.runner = (ROOT / "crates/forever-navigation/src/inventory/fs/run.rs").read_text()

    def test_identity_and_bounds_are_explicit(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"]),
            ("forever-selected-navigation-filesystem-contract-v1", 1),
        )
        self.assertEqual(
            (c["scope"]["product_line"], c["scope"]["client_build"], c["scope"]["platform"]),
            ("forever", 70450, "linux"),
        )
        self.assertTrue(c["scope"]["api_only"])
        bounds = c["bounded_execution"]
        self.assertEqual((bounds["output_maps"], bounds["selected_output_tiles"]), (1, 4096))
        self.assertIn("not represented as a universal measured work sum", bounds["other_pipeline_work"])
        self.assertIn("no total RSS claim", bounds["heap"])

    def test_typed_owner_and_runner_keep_partial_coverage_separate(self):
        self.assertIn("pub struct AvailableSelectedPackage", self.available)
        self.assertIn("pub fn open_70450", self.available)
        self.assertIn("pub fn plan(&self) -> &AvailableSelectedPlan", self.available)
        self.assertNotIn("Plan::from_rows", self.available)
        self.assertNotIn("pub fn into_complete", self.available)
        self.assertIn("pipeline::run_available_selected", self.runner)
        self.assertIn("pub fn execute_available_selected", self.runner)
        self.assertIn("Selection::All", self.runner)
        self.assertIn("tile_limits.builder.work = tile_limits.builder.work.min(builder_work)", self.runner)
        self.assertIn("tile_limits.save.work = tile_limits.save.work.min(save_work)", self.runner)
        self.assertIn("package.latch_failure(reason)", self.runner)

    def test_global_buffer_is_released_before_tile_processing(self):
        start = self.runner.index("pub fn execute_available_selected<")
        end = self.runner.index("\nfn bound_available_tile(", start)
        body = self.runner[start:end]
        ordered_markers = (
            "sink.emit(Event::Global {",
            "progress.output_bytes = size;",
            "drop(global);",
            "for &tile in &selected_tiles[..selected_tile_count] {",
        )
        positions = [body.index(marker) for marker in ordered_markers]
        self.assertEqual(positions, sorted(positions))

    def test_rooted_discovery_and_validation_scope_are_declared(self):
        self.assertIn("guard::Root::new", self.available)
        self.assertIn("guard.verify(&mut budget)", self.available)
        self.assertIn("scan_maps(", self.available)
        self.assertIn("scan_vmaps(", self.available)
        filesystem = (ROOT / "crates/forever-navigation/src/inventory/fs.rs").read_text()
        for code in self.contract["error_codes"].values():
            self.assertIn(code, filesystem)
        self.assertIn("pub mod available;", (ROOT / "crates/forever-navigation/src/inventory/fs.rs").read_text())
        self.assertEqual(self.contract["scope"]["new_registry_dependencies"], [])
        self.assertIn("no Windows filesystem support", self.contract["non_claims"][-2])


if __name__ == "__main__":
    unittest.main()
