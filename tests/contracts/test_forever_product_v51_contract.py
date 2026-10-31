# SPDX-License-Identifier: GPL-3.0-or-later
"""Scoped real acceptance and stable contract for the combined v51 product."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def unique_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate key: {key}")
        result[key] = value
    return result


class ProductV51Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v51.json").read_text(),
            object_pairs_hook=unique_keys,
        )

    def test_plan_identity_and_stable_mask(self):
        c = self.contract
        self.assertEqual((c["contract"], c["version"]), ("forever-product-v51", 51))
        self.assertEqual(c["builds"], [70450])
        self.assertEqual(c["component_mask"], 25)
        self.assertEqual(c["component_bits"], {"db2": 1, "maps": 8, "vmaps": 16})
        self.assertFalse(c["component_mask_translation"])
        plan = (ROOT / c["plan"]["source_fixture"]).read_text()
        lines = plan.splitlines()
        self.assertEqual(len(lines), 34)
        self.assertEqual(lines[0], "forever-product\t51")
        self.assertEqual(lines[21:23], ["components\tdb2,maps,vmaps", "terrain\tall-available-maps"])
        self.assertEqual(lines[29:], [
            "db2\tGameObjectDisplayInfo\t1266277", "db2\tMap\t1349477",
            "db2\tLiquidObject\t1308058", "db2\tLiquidMaterial\t1132538",
            "db2\tLiquidType\t1371380",
        ])
        self.assertTrue(plan.endswith("\n"))
        for version in [25, 50]:
            self.assertNotEqual(plan, (ROOT / f"tests/contracts/fixtures/forever-product-v{version}/all.tsv").read_text())
        self.assertEqual(lines[16], "collision-profile\t" + c["source_profile"])
        self.assertEqual(lines[18], "collision-coverage\t" + c["plan_coverage_profile"])
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])

    def test_payloads_limits_and_liquid_policy(self):
        c = self.contract
        v50 = json.loads((ROOT / "contracts/forever-product-contract-v50.json").read_text())
        limits = c["resource_profile"]["limits"]
        self.assertEqual(limits, v50["resource_profile"]["limits"])
        self.assertEqual(len(limits), 10)
        self.assertTrue(all(type(value) is int for value in limits.values()))
        self.assertFalse(c["resource_profile"]["geometry_limits_changed_vs_v50"])
        self.assertTrue(c["package_limits"]["terrain_and_liquid_evidence_included"])
        self.assertEqual(c["package_limits"]["files_max"], 262144)
        self.assertEqual(c["terrain"]["producer_basis"], "forever-product-v25")
        self.assertEqual(c["terrain"]["coverage_version"], 3)
        self.assertEqual(c["geometry"]["producer_basis"], "forever-product-v50")
        self.assertEqual(c["geometry"]["coverage_layout_version"], 22)
        self.assertEqual(c["geometry"]["coverage_magic"], "RCVGEO1\0")
        self.assertEqual(c["geometry"]["coverage_identity"], c["plan_coverage_profile"])
        liquid = c["liquid_sources"]
        self.assertEqual(liquid["complete_tables"], ["LiquidObject", "LiquidMaterial", "LiquidType"])
        self.assertEqual(liquid["partial_source"], "fatal")
        self.assertEqual(liquid["partial_allowed_tables"], ["GameObjectDisplayInfo", "Map"])
        self.assertEqual(liquid["evidence_bytes"], 112)
        self.assertEqual(liquid["evidence_magic"], "RCLIQ01\0")
        self.assertEqual(liquid["evidence_path"], v50["liquid_object_source"]["evidence_path"])
        self.assertTrue(liquid["terrain_output_is_separate"])

    def test_scoped_real_acceptance_boundaries_and_deliverables(self):
        c = self.contract
        self.assertEqual(c["status"], "scoped-real-extraction-accepted; software-closeout-passed")
        for key in ["real_target_attempted", "real_extraction_accepted"]:
            self.assertIs(c[key], True)
        for key in ["full_real_target_acceptance", "full_extractor_complete"]:
            self.assertIs(c[key], False)
        for key, value in c["boundaries"].items():
            self.assertIs(value, key in {"real_target_attempted", "real_extraction_accepted", "closeout_run"}, key)
        real = c["real_acceptance"]
        self.assertEqual(real["run_id"], "v51-db2-terrain-liquid-vmap-20261029T220401Z")
        self.assertEqual((real["status"], real["target_status"]), ("passed", "accepted"))
        self.assertEqual(real["elapsed_seconds"], 1190)
        self.assertEqual(real["exit_codes"], {"inspect": 0, "extract": 0, "probe": 0, "verify": 0, "verify_links": 0})
        for key in ["real_gate", "real_target_attempted", "real_extraction_accepted", "all_stage_exit_codes_zero", "client_metadata_unchanged", "inputs_unchanged", "fresh_extraction_matches_promoted_audit", "v49_godi_outcomes_and_omissions_retained"]:
            self.assertIs(real[key], True, key)
        for key in ["accepted_prefix", "red", "full_real_target_acceptance", "full_extractor_complete"]:
            self.assertIs(real[key], False, key)
        self.assertEqual(real["report_sha256"], "058a607138d0f12bb1115f55045acbf3aeae58b9dc286b12ef61e55da4f13e1e")
        self.assertEqual(real["audit_sha256"], "16e71e43c46c47928596bddbbc817ec4f46f68176d4b9fbed6e7cf3576c6f35b")
        self.assertEqual(real["binary_sha256"], "3ac80914a56388d74d2d90f717c277b8a446f1142b513843d0d31af48bb3d700")
        self.assertEqual(real["package"], {
            "published_files": 15448, "published_bytes": 708743828,
            "component_mask": 25, "db2_files": 5,
            "terrain": {"available_maps": 71, "loaded_wdts": 69, "map_tiles": 3322, "tilelists": 71, "unavailable_wdts": 2},
            "liquid_object": {"direct_rows": 57, "copy_records": 1213, "materialized_rows": 1270},
            "complete_liquid_sources": True,
        })
        self.assertEqual(real["shared_payload_parity"], {"v25": "matched", "v50": "matched"})
        closeout = c["software_closeout"]
        self.assertEqual(closeout["log_path"], "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/kimi/checks/closeout-20261029T223522Z-2741011.log")
        self.assertEqual(closeout["sha256"], "11322eae6c14fdb1df1d87e55b776103a64a9bb914e9cd9b15dadafe6d65c916")
        for key, expected in {"exit_code": 0, "workspace_suites": 95, "tests_passed": 4788, "tests_failed": 0, "tests_ignored": 26, "contracts_passed": 595}.items():
            self.assertEqual(closeout[key], expected, key)
        for key in ["status", "formatting", "clippy", "release_build"]:
            self.assertEqual(closeout[key], "passed", key)
        self.assertEqual(closeout["final_independent_review"], "Astra: passed")
        self.assertIn("omitted mask 25", closeout["first_closeout"])
        self.assertIn("test-only", closeout["correction"])
        self.assertTrue(closeout["binary_identical_after_test_correction"])
        self.assertTrue(closeout["separate_from_real_acceptance"])
        self.assertTrue(c["offline"])
        self.assertEqual(c["platform"], "linux")
        for path in [
            "docs/forever-product-v51.md",
            "crates/forever-extractor/tests/v51_db2_terrain_liquid_vmap_cli.rs",
            "crates/forever-extractor/src/plan/v51_tests.rs",
        ]:
            self.assertTrue((ROOT / path).is_file(), path)
        self.assertEqual(c["synthetic_validation"]["cli_harness_tests"], 8)
        self.assertEqual(c["synthetic_validation"]["focused_unit_tests"], 6)


if __name__ == "__main__":
    unittest.main()
