# SPDX-License-Identifier: GPL-3.0-or-later
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


class TrinityCompatibilityContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-trinity-compatibility-contract-v1.json").read_text()
        )

    def test_command_binds_existing_v51_and_v57_packages(self):
        command = self.contract["command"]
        self.assertIn("verify-trinity-compat", command)
        for option in ("--terrain-package", "--terrain-plan", "--navigation-package", "--navigation-plan"):
            self.assertIn(option, command)
        self.assertEqual(self.contract["source_profiles"]["terrain"], "forever-product-v51")
        self.assertEqual(self.contract["source_profiles"]["navigation"], "forever-product-v57")
        self.assertFalse(self.contract["verification"]["reextraction"])
        self.assertFalse(self.contract["verification"]["output_bytes_modified"])
        self.assertFalse(self.contract["verification"]["network"])

    def test_omissions_are_checked_not_manufactured(self):
        verification = self.contract["verification"]
        self.assertIn("nonempty", verification["omission_identity"])
        self.assertIn("4108-byte", verification["tilelist"])
        self.assertIn("zero present", verification["tilelist"])
        self.assertIn("no global", verification["navigation_artifacts"])
        self.assertIn("no tile", verification["navigation_artifacts"])
        self.assertEqual(self.contract["status"], "compatible-partial")

    def test_partial_compatibility_never_completes_the_extractor(self):
        claims = self.contract["claims"]
        self.assertTrue(claims["compatible_partial"])
        for key in ("full_world_complete", "runtime_accepted", "full_extractor_complete", "release_authorized", "publication_authorized"):
            self.assertFalse(claims[key])
        self.assertTrue((ROOT / "crates/forever-extractor/src/verify/trinity_compat.rs").is_file())
        self.assertIn("verify-trinity-compat", (ROOT / "crates/forever-extractor/src/main.rs").read_text())
        acceptance = self.contract["real_acceptance"]
        self.assertEqual(acceptance["status"], "accepted")
        self.assertTrue(acceptance["compatible_partial"])
        self.assertEqual((acceptance["maps"], acceptance["planned"], acceptance["root_absent"]), (71, 69, 2))
        self.assertFalse(acceptance["network_attempted"])
        self.assertTrue((ROOT / acceptance["report"]).is_file())


if __name__ == "__main__":
    unittest.main()
