# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 model-heavy material-filtered-WMO contract gates."""
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v40.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v40/all.tsv"
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV40Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_contract_identity_and_plan_profile(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v40", 40, "Build70450ModelHeavyMaterialFilteredWmoGeometryV1"),
        )
        self.assertEqual(
            (contract["builds"], contract["platform"], contract["backend"], contract["offline"]),
            ([70450], "linux", "rust-local", True),
        )
        self.assertEqual(contract["component_mask"], 17)
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], "linux70450-model-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["plan_profile"], "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1")
        self.assertEqual(contract["plan_coverage_profile"], "linux70450-model-heavy-material-filtered-wmo-coverage-v13")
        self.assertEqual(contract["codec_profile"], "linux70450-model-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["domain_profile"], "model-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["resource_profile"]["id"], "linux-collision-model-heavy-resources-v1")
        self.assertEqual(contract["completion"], "linux70450-available-model-heavy-material-filtered-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-model-heavy-material-filtered-wmo-verification-v1")
        self.assertEqual(
            contract["verification_scope"],
            "artifact_integrity_model_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
        )

        plan = contract["plan"]
        self.assertEqual(plan["forever-product"], "40")
        self.assertEqual(plan["product_version"], 40)
        self.assertEqual(plan["collision_profile"], contract["plan_profile"])
        self.assertEqual(plan["coverage_profile"], contract["plan_coverage_profile"])
        self.assertEqual(plan["domain_closure"], contract["domain_profile"])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["requested_map_zero"], "rejected; zero is not an all-map sentinel")
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertEqual(plan["component_mask"], 17)
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1048576)

        fixture = contract["fixtures"]
        self.assertFalse(fixture["contains_client_data"])
        self.assertFalse(fixture["real_extraction_evidence"])
        self.assertEqual(fixture["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v40/all.tsv")
        self.assertEqual(fixture["fixture_status"], "canonical synthetic plan fixture supplied; contains no client data or real extraction evidence")

    def test_model_heavy_changes_only_the_coupled_model_identity_capacity(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-model-heavy-resources-v1")
        for prefix in ("collector", "startup_collision"):
            self.assertEqual(resource[f"{prefix}_max_reads"], 1_000_000)
            self.assertEqual(resource[f"{prefix}_max_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_dependency_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_source_bytes"], 536_870_912)
            self.assertEqual(resource[f"{prefix}_max_raw_bytes"], 268_435_456)
        self.assertEqual(resource["collector_max_models"], 65_536)
        self.assertEqual(resource["startup_collision_max_models"], 65_536)
        for name in (
            "spool_max_model_identity_records",
            "placements_max_models",
            "model_staging_max_models",
            "package_model_inventory_max",
        ):
            self.assertEqual(resource[name], 65_536, name)
        read_heavy = json.loads((ROOT / "contracts/forever-product-contract-v39.json").read_bytes())[
            "resource_profile"
        ]
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], read_heavy["inventory_max_wdt_cache_entries"])
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], read_heavy["stage_max_callbacks_per_map_tile"])
        self.assertEqual(
            resource["capacity_units"]["inventory_max_wdt_cache_entries"],
            "WDT/cache inventory entries",
        )
        self.assertEqual(
            resource["capacity_units"]["stage_max_callbacks_per_map_tile"],
            "callbacks per map/tile",
        )
        self.assertEqual(
            resource["coherent_capacity_routes"],
            ["collector models", "startup collision models", "spool model identity records", "placement model identities", "model staging identities", "package model inventory"],
        )
        self.assertEqual(
            resource["preserved_non_identity_limits"],
            "inventory WDT/cache entries and callbacks per map/tile remain at 4096, equal to ReadHeavyV1",
        )
        self.assertEqual(resource["only_capacity_raised_vs_v39"], "model_identity_capacities_only")
        self.assertEqual(resource["all_other_limits"], "same as v39 ReadHeavyV1")
        self.assertFalse(resource["disk_world_profile_implicit"])

        limits = self.contract["limits"]
        self.assertEqual(
            (limits["reads_max"], limits["request_calls_max"], limits["dependency_requests_max"]),
            (1_000_000, 16_000_000, 16_000_000),
        )
        self.assertEqual((limits["source_bytes_max"], limits["raw_bytes_max"]), (536_870_912, 268_435_456))
        self.assertEqual(limits["memory_limit_bytes"], 1_073_741_824)
        self.assertEqual(limits["models_max"], 65_536)
        self.assertEqual(limits["model_identity_capacity"], 65_536)
        self.assertTrue(limits["models_max_raised_vs_v39"])
        for name in (
            "reads_max_raised_vs_v39",
            "request_calls_max_raised_vs_v39",
            "dependency_requests_max_raised_vs_v39",
            "source_bytes_max_raised_vs_v39",
            "raw_bytes_max_raised_vs_v39",
        ):
            self.assertFalse(limits[name], name)
        for name in LIST_ORDER:
            self.assertEqual(limits[f"{name}_max"], 4096)

    def test_v13_codec_preserves_v12_layout_and_all_five_lists(self):
        codec = self.contract["coverage_codec"]
        expected_path = "vmaps/available-model-heavy-material-filtered-wmo-geometry-coverage-v13.bin"
        self.assertEqual(codec["path"], expected_path)
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 13))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertEqual(codec["preserved_layout"], "v12 field layout with the v13 version and profile identities; no field additions or removals")
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["resource_profile"], "linux-collision-model-heavy-resources-v1")
        self.assertEqual(codec["list_max"], 4096)
        self.assertEqual(codec["zero_omissions_still_layout_version"], 13)
        self.assertTrue(codec["canonical_roundtrip_required"])
        self.assertEqual(codec["strict_wrappers"]["v39"]["layout_version"], 12)
        self.assertEqual(codec["strict_wrappers"]["v39"]["contract"], "forever-product-v39")
        self.assertEqual(codec["strict_wrappers"]["v40"]["layout_version"], 13)
        self.assertEqual(codec["strict_wrappers"]["v40"]["coverage_path"], expected_path)
        self.assertIn("v1-v12", codec["cross_profile_rejection"])
        omission = self.contract["wmo_omissions"]
        self.assertEqual(omission["list_order"], LIST_ORDER)
        self.assertTrue(omission["pairwise_disjoint"])
        self.assertEqual(omission["canonical"], "each list is ordered, unique, nonzero, and bounded to 4096 IDs")
        self.assertEqual(codec["material_filtered_wmo_serialized_after"], "unselected_wmo")
        predicate = omission["material_filtered_wmo"]["material_filter"]
        self.assertEqual(predicate["keep_predicate"], "flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)")
        self.assertEqual(predicate["reject_predicate"], "flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)")

    def test_acceptance_is_failed_and_v41_remains_design_only(self):
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v40")
        self.assertEqual(real["run_id"], "v40-model-heavy-wmo-20261027T000044Z")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        self.assertFalse(real["accepted_prefix"])
        self.assertEqual(real["elapsed_seconds"], 72)
        self.assertEqual((real["inspect_exit_code"], real["extract_exit_code"]), (0, 9))
        self.assertEqual(real["extract_error"], "local_store_policy_limit")
        self.assertEqual(
            (real["probe_exit_code"], real["verify_exit_code"], real["verify_links_exit_code"]),
            (-1, -1, -1),
        )
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["inputs_unchanged"])
        self.assertTrue(real["client_unchanged"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        self.assertEqual(
            real["report_sha256"],
            "f1851ba06cf85679f4f83d5c080352c81dff533c655843f78232f2b67b3cc0c9",
        )
        self.assertEqual(
            real["diagnostic_sha256"],
            "bcebb01353a9908a19fe2e9c94c4ee8482609fb438cccbc63f5ec08dbecba10b",
        )
        self.assertEqual(
            real["software_sha256"],
            "f7776c05e70428a3a0477ee5d84f11ab836b0f66f0f4da163f143dfe27fe5bac",
        )
        self.assertEqual(
            real["binary_sha256"],
            "ba3f30864d4066c3f7963fa38463452bb65511169df27c5df2ae951e867e80b6",
        )
        counters = real["diagnostic_counters"]
        self.assertEqual((counters["phase"], counters["stage"], counters["file_data_id"], counters["ordinal"]),
                         ("models", "wmo_group", 7957271, 174))
        self.assertEqual(counters["result"], "typed Limit")
        self.assertFalse(counters["guards_triggered"])
        self.assertEqual((counters["source_bytes"], counters["source_bytes_max"], counters["source_bytes_remaining"]),
                         (536853486, 536870912, 17426))
        self.assertEqual(counters["models"], 4450)
        self.assertEqual((counters["reads"], counters["reads_max"]), (7885, 1_000_000))
        self.assertEqual(counters["memory_limit_bytes"], 1_073_741_824)
        probe = real["source_limit_probe"]
        self.assertEqual(
            probe["report"],
            "/home/joe/.local/share/rustycore-extractor-work/runs/candidate/v40-source-limit-probe-20261027T000438Z/report.redacted.json",
        )
        self.assertEqual(
            probe["report_sha256"],
            "93cbe5e506bd21d0f8799001b148c83168001fb718ce5e676ce1f1a5b9d29051",
        )
        self.assertEqual(
            probe["original_evidence_sha256"],
            "9c68fc294b28bc0628b7ca7baf85e7b8ec6abfac1f109a66ad9155bd4b184784",
        )
        self.assertEqual((probe["decoded"], probe["excess"], probe["parse_status"]), (22224, 4798, "correct"))
        self.assertEqual((probe["triangles"], probe["indices"], probe["vertices"]), (313, 939, 502))
        self.assertEqual((probe["batches"], probe["outer_chunks"], probe["group_chunks"]), (13, 2, 14))
        self.assertFalse(probe["liquid_present"])
        self.assertIsNone(real["target_lists"])
        self.assertFalse(real["target_lists_are_observed_output"])
        self.assertEqual(
            real["report"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v40-model-heavy-wmo-20261027T000044Z-acceptance.json",
        )
        self.assertEqual(real["software_tests"], {"suites": 84, "passed": 4534, "ignored": 26, "contracts": 547})
        self.assertFalse(real["capacity_sufficiency_proven"])
        self.assertFalse(real["wmo_462695_admission_proven"])
        self.assertFalse(real["full_world_coverage"])
        self.assertFalse(real["runtime_claim"])

        followup = self.contract["v41_followup_design"]
        self.assertEqual(followup["status"], "design-only; target-not-run")
        self.assertEqual(followup["profile"], "SourceExpandedV1")
        self.assertEqual(followup["coverage_layout_planned"], 14)
        self.assertEqual(followup["only_resource_change"], "max_source_bytes")
        self.assertEqual((followup["max_source_bytes_from"], followup["max_source_bytes_to"]),
                         (536870912, 1073741824))
        self.assertEqual(followup["preserved_limits"]["models_max"], 65536)
        self.assertEqual(followup["preserved_limits"]["reads_max"], 1000000)
        self.assertEqual(followup["preserved_limits"]["request_calls_max"], 16000000)
        self.assertEqual(followup["preserved_limits"]["dependency_requests_max"], 16000000)
        self.assertEqual(followup["preserved_limits"]["raw_bytes_max"], 268435456)
        self.assertTrue(followup["preserved_limits"]["all_other_limits_unchanged"])
        self.assertFalse(followup["disk_world_implied"])
        self.assertFalse(followup["capacity_sufficiency_proven"])
        self.assertFalse(followup["target_acceptance"])

        prior = self.contract["prior_v39_failure_evidence"]
        self.assertEqual(prior["status"], "failed; prior v39 evidence only, not v40 acceptance")
        self.assertEqual(prior["run_id"], "v39-read-heavy-wmo-20261026T181306Z")
        self.assertEqual(prior["report_sha256"], "443644827f14008e4b04ae9bcea53b965114d92614010c38f6a767e4be99e128")
        self.assertEqual((prior["extract_error"], prior["cause"]), ("collision_policy_limit", "max_models"))
        self.assertEqual((prior["models"], prior["v39_models_max"]), (4096, 4096))
        self.assertEqual((prior["reads"], prior["v39_reads_max"]), (7218, 1_000_000))
        self.assertFalse(prior["acceptance_transferred"])
        self.assertFalse(prior["v40_capacity_sufficiency_proven"])
        self.assertFalse(prior["wmo_462695_admission_proven"])

        synthetic = self.contract["synthetic_validation_evidence"]
        self.assertEqual(synthetic["cli_cases_passed"], 12)
        self.assertEqual(synthetic["cli_cases_total"], 12)
        self.assertEqual(synthetic["library_tests_passed"], 757)
        self.assertEqual(synthetic["library_tests_total"], 757)
        self.assertEqual(synthetic["geometry_limits_effective"], {"model_heavy": 65536, "read_heavy": 4096})
        self.assertEqual(synthetic["auditor_cases"], 46)
        self.assertEqual(synthetic["auditor_reports"], 125)
        self.assertTrue(synthetic["synthetic_cli_auditor_passed"])
        self.assertFalse(synthetic["structural_65536_65537_boundary_proven"])
        self.assertFalse(synthetic["real_acceptance"])

        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        self.assertIn("model-identity-profile-value", cases)
        self.assertIn("no structural 65536/65537 model-identity boundary", cases["model-identity-profile-value"]["expect"])
        self.assertIn("reduced-placement-inventory-boundary", cases)
        self.assertIn("4100-model inventory and rejects 4101", cases["reduced-placement-inventory-boundary"]["expect"])
        self.assertIn("genuine-v39-package-under-v40-plan", cases)
        self.assertIn("genuine synthetic package", cases["genuine-v39-package-under-v40-plan"]["expect"])
        self.assertIn("production-verifier-attacks", cases)
        attack_expectation = cases["production-verifier-attacks"]["expect"]
        self.assertIn("all ten resealed pairwise omission-list overlaps", attack_expectation)
        self.assertIn("resealed v13-to-v12 codec mutation", attack_expectation)
        self.assertIn("received synthetic CLI evidence", attack_expectation)
        self.assertNotIn("remain unverified", attack_expectation)
        self.assertIn("CLI omission lists accepts 4096 and rejects 4097", cases["list-limits"]["expect"])
        self.assertEqual(cases["real-acceptance"]["status"], "failed")
        verification = self.contract["verification"]
        self.assertFalse(verification["full_world_coverage"])
        self.assertFalse(verification["runtime_claim"])
        self.assertFalse(verification["full_extractor_complete"])
        self.assertFalse(self.contract["boundary"]["real_extraction"])
        self.assertFalse(self.contract["boundary"]["real_extraction_accepted"])
        for name, value in self.contract["boundary"].items():
            self.assertFalse(value, name)
        self.assertFalse(self.contract["full_extractor_complete"])

    @unittest.skipUnless(FIXTURE.is_file(), "canonical v40 fixture not yet supplied by integration; intentionally not fabricated")
    def test_canonical_fixture_matches_declared_plan(self):
        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual(rows[0], ["forever-product", "40"])
        self.assertIn(["collision-profile", "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1"], rows)
        self.assertIn(["collision-coverage", "linux70450-model-heavy-material-filtered-wmo-coverage-v13"], rows)
        self.assertEqual([row[1] for row in rows if row[0].endswith("-bytes")], ["1048576"] * 7)


if __name__ == "__main__":
    unittest.main()
