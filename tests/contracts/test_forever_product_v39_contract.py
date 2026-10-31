# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 read-heavy material-filtered-WMO contract gates."""
import hashlib
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v39.json"
FIXTURE = ROOT / "tests/contracts/fixtures/forever-product-v39/all.tsv"
EXPECTED_ROWS = [
    ["forever-product", "39"],
    ["build", "70450"],
    ["locale", "esES"],
    ["index", "contiguous"],
    ["root", "header-50893-v2"],
    ["schema-catalog", "wowdbdefs-70450-v1"],
    ["decryption", "disabled"],
    ["partial_encrypted_sections", "declared-skip"],
    ["collision-storage", "linux-leased-append-disk-world-v1"],
    ["collision-raw-bytes", "1048576"],
    ["collision-wdt-bytes", "1048576"],
    ["collision-cache-bytes", "1048576"],
    ["collision-placement-bytes", "1048576"],
    ["collision-vmo-bytes", "1048576"],
    ["collision-map-bytes", "1048576"],
    ["collision-output-bytes", "1048576"],
    ["collision-profile", "linux70450-read-heavy-material-filtered-wmo-vmap-product-v1"],
    ["collision-domain", "all-available-maps"],
    ["collision-coverage", "linux70450-read-heavy-material-filtered-wmo-coverage-v12"],
    ["gameobject-scope", "excluded-by-profile-v1"],
    ["model-scope", "placement-only-v1"],
    ["components", "db2,vmaps"],
    ["terrain", "none"],
    ["pack", "true"],
    ["navigation", "none"],
    ["big-base-unit", "false"],
    ["skip-liquids", "false"],
    ["maximum-slope", "55"],
    ["non-steep-slope", "55"],
    ["db2", "Map", "1349477"],
]
LIST_ORDER = ["missing_m2", "missing_wmo", "empty_wmo", "unselected_wmo", "material_filtered_wmo"]
MIXED_LISTS = {
    "missing_m2": [970793],
    "missing_wmo": [462694],
    "empty_wmo": [462697],
    "unselected_wmo": [462695],
    "material_filtered_wmo": [462701],
}


def reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


class ProductV39Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            CONTRACT.read_bytes(), object_pairs_hook=reject_duplicate_keys
        )

    def test_contract_identity_and_canonical_synthetic_plan(self):
        contract = self.contract
        self.assertEqual(
            (contract["contract"], contract["version"], contract["publication_profile"]),
            ("forever-product-v39", 39, "Build70450ReadHeavyMaterialFilteredWmoGeometryV1"),
        )
        self.assertEqual(
            (contract["builds"], contract["platform"], contract["backend"], contract["offline"]),
            ([70450], "linux", "rust-local", True),
        )
        self.assertEqual(contract["component_mask"], 17)
        self.assertEqual(contract["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        self.assertEqual(contract["source_profile"], "linux70450-read-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["plan_profile"], "linux70450-read-heavy-material-filtered-wmo-vmap-product-v1")
        self.assertEqual(contract["plan_coverage_profile"], "linux70450-read-heavy-material-filtered-wmo-coverage-v12")
        self.assertEqual(contract["codec_profile"], "linux70450-read-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["domain_profile"], "read-heavy-material-filtered-wmo-v1")
        self.assertEqual(contract["resource_profile"]["id"], "linux-collision-read-heavy-resources-v1")
        self.assertEqual(contract["completion"], "linux70450-available-read-heavy-material-filtered-wmo-collision-v1")
        self.assertEqual(contract["verification_profile"], "linux70450-read-heavy-material-filtered-wmo-verification-v1")
        self.assertEqual(
            contract["verification_scope"],
            "artifact_integrity_read_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage",
        )

        plan = contract["plan"]
        self.assertEqual(plan["ordered_keys"], [row[0] for row in EXPECTED_ROWS[:-1]])
        self.assertEqual(plan["required_db2_entry"], {"name": "Map", "file_data_id": 1349477, "count": 1})
        self.assertEqual(plan["collision_profile"], contract["plan_profile"])
        self.assertEqual(plan["coverage_profile"], contract["plan_coverage_profile"])
        self.assertEqual(plan["domain_closure"], contract["domain_profile"])
        self.assertEqual(plan["map_selection"], "MapSelection::All")
        self.assertEqual(plan["map_domain"], "absent (map_domain=None)")
        self.assertEqual(plan["requested_map_zero"], "rejected; zero is not an all-map sentinel")
        self.assertEqual(plan["component_mask"], 17)
        self.assertEqual(plan["components"], ["db2", "vmaps"])
        self.assertTrue(plan["storage_budgets_downward_only"])
        self.assertEqual(plan["synthetic_budget_bytes"], 1048576)
        self.assertFalse(contract["fixtures"]["contains_client_data"])
        self.assertFalse(contract["fixtures"]["real_extraction_evidence"])
        self.assertEqual(contract["fixtures"]["cases"][0]["path"], "tests/contracts/fixtures/forever-product-v39/all.tsv")

        raw = FIXTURE.read_bytes()
        self.assertTrue(raw.endswith(b"\n"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.isascii())
        rows = [line.split("\t") for line in raw.decode("ascii").splitlines()]
        self.assertEqual(rows, EXPECTED_ROWS)
        self.assertEqual(len(rows[-1]), 3)
        self.assertEqual([row[1] for row in rows[1:-1] if row[0].endswith("-bytes")], ["1048576"] * 7)

    def test_read_heavy_raises_only_reads_and_preserves_source_heavy_limits(self):
        resource = self.contract["resource_profile"]
        self.assertEqual(resource["id"], "linux-collision-read-heavy-resources-v1")
        self.assertEqual(resource["collector_max_reads"], 1_000_000)
        self.assertEqual(resource["startup_collision_max_reads"], 1_000_000)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        for prefix in ("collector", "startup_collision"):
            self.assertEqual(resource[f"{prefix}_max_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_dependency_requests"], 16_000_000)
            self.assertEqual(resource[f"{prefix}_max_source_bytes"], 536_870_912)
            self.assertEqual(resource[f"{prefix}_max_raw_bytes"], 268_435_456)
        self.assertEqual(resource["inventory_max_wdt_cache_entries"], 4096)
        self.assertEqual(resource["stage_max_callbacks_per_map_tile"], 4096)
        self.assertEqual(
            resource["capacity_units"],
            {
                "inventory_max_wdt_cache_entries": "WDT/cache inventory entries",
                "stage_max_callbacks_per_map_tile": "callbacks per map/tile",
            },
        )
        self.assertEqual(resource["only_capacity_raised_vs_v38"], "max_reads")
        self.assertEqual(resource["all_other_limits"], "same as v38 SourceHeavyV1")
        self.assertFalse(resource["disk_world_profile_implicit"])

        limits = self.contract["limits"]
        self.assertEqual(limits["reads_max"], 1_000_000)
        self.assertEqual(limits["request_calls_max"], 16_000_000)
        self.assertEqual(limits["dependency_requests_max"], 16_000_000)
        self.assertEqual(limits["source_bytes_max"], 536_870_912)
        self.assertEqual(limits["raw_bytes_max"], 268_435_456)
        self.assertTrue(limits["reads_max_raised_vs_v38"])
        for name in (
            "request_calls_max_raised_vs_v38",
            "dependency_requests_max_raised_vs_v38",
            "source_bytes_max_raised_vs_v38",
            "raw_bytes_max_raised_vs_v38",
            "models_max_raised",
            "memory_max_raised",
        ):
            self.assertFalse(limits[name], name)
        for name in LIST_ORDER:
            self.assertEqual(limits[f"{name}_max"], 4096)

    def test_v12_codec_has_same_five_list_layout_and_strict_v38_boundary(self):
        codec = self.contract["coverage_codec"]
        expected_path = "vmaps/available-read-heavy-material-filtered-wmo-geometry-coverage-v12.bin"
        self.assertEqual(codec["path"], expected_path)
        self.assertEqual((codec["magic"], codec["layout_version"]), ("RCVGEO1", 12))
        self.assertEqual(codec["encoded_magic"], "RCVGEO1\\0")
        self.assertEqual(codec["profile"], self.contract["codec_profile"])
        self.assertEqual(codec["coverage_profile"], self.contract["plan_coverage_profile"])
        self.assertEqual(codec["resource_profile"], "linux-collision-read-heavy-resources-v1")
        self.assertEqual(codec["list_max"], 4096)
        self.assertEqual(codec["zero_omissions_still_layout_version"], 12)
        self.assertTrue(codec["canonical_roundtrip_required"])
        self.assertEqual(codec["strict_wrappers"]["v38"]["layout_version"], 11)
        self.assertEqual(codec["strict_wrappers"]["v38"]["contract"], "forever-product-v38")
        self.assertEqual(codec["strict_wrappers"]["v39"]["layout_version"], 12)
        self.assertEqual(codec["strict_wrappers"]["v39"]["coverage_path"], expected_path)
        self.assertIn("v1-v11", codec["cross_profile_rejection"])
        omissions = self.contract["wmo_omissions"]
        self.assertEqual(omissions["list_order"], LIST_ORDER)
        self.assertTrue(omissions["pairwise_disjoint"])
        self.assertEqual(omissions["canonical"], "each list is ordered, unique, nonzero, and bounded to 4096 IDs")
        self.assertEqual(codec["material_filtered_wmo_serialized_after"], "unselected_wmo")

    def test_five_lists_and_material_filtered_predicate_are_unchanged(self):
        omission = self.contract["wmo_omissions"]
        self.assertEqual(MIXED_LISTS, {
            "missing_m2": [970793],
            "missing_wmo": [462694],
            "empty_wmo": [462697],
            "unselected_wmo": [462695],
            "material_filtered_wmo": [462701],
        })
        self.assertTrue(omission["pairwise_disjoint"])
        predicate = omission["material_filtered_wmo"]
        self.assertEqual(predicate["origin"], "WMO FileDataId only; logical-name origin is fatal")
        self.assertEqual(predicate["local_missing_m2"], 0)
        self.assertFalse(predicate["omitted_usable"])
        self.assertIsNone(predicate["raw_output"])
        self.assertEqual(predicate["selection_sets"], "present and nonempty")
        self.assertEqual(predicate["material_filter"]["keep_predicate"], "flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)")
        self.assertEqual(predicate["material_filter"]["reject_predicate"], "flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)")
        self.assertEqual(predicate["material_filter"]["examples"], {"rejected": ["0x26", "0x66"], "kept": ["0x20", "0x08"]})
        cases = {case["case"]: case for case in self.contract["acceptance_matrix"]}
        for name in (
            "canonical-profile-and-plan", "synthetic-plan-fixture", "coverage-codec-v12",
            "read-heavy-resource-profile", "reduced-plan-budget", "five-way-mixed-omissions",
            "material-filtered-only", "production-verifier-attacks", "list-limits",
            "lifecycle-and-cross-version", "real-acceptance",
        ):
            self.assertIn(name, cases)
        self.assertEqual(cases["real-acceptance"]["status"], "failed")
        self.assertIn("4096 and rejects 4097", cases["list-limits"]["expect"])
        self.assertIn("cross-version", cases["lifecycle-and-cross-version"]["expect"])

    def test_v39_real_target_failure_and_v38_failure_are_not_acceptance(self):
        real = self.contract["real_acceptance"]
        self.assertEqual(real["status"], "failed")
        self.assertEqual(real["target"], "linux build-70450 forever-product-v39")
        self.assertTrue(real["performed"])
        self.assertFalse(real["accepted"])
        self.assertEqual(real["run_id"], "v39-read-heavy-wmo-20261026T181306Z")
        self.assertEqual(
            real["report"],
            "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v39-read-heavy-wmo-20261026T181306Z-acceptance.json",
        )
        report_path = Path(real["report"])
        self.assertTrue(report_path.is_file(), str(report_path))
        report_bytes = report_path.read_bytes()
        self.assertEqual(hashlib.sha256(report_bytes).hexdigest(), real["report_sha256"])
        self.assertEqual(
            real["diagnostic"],
            "/home/joe/.local/share/rustycore-extractor-work/runs/candidate/v39-read-heavy-wmo-20261026T181306Z/diagnostic.jsonl",
        )
        diagnostic_path = Path(real["diagnostic"])
        self.assertTrue(diagnostic_path.is_file(), str(diagnostic_path))
        diagnostic_bytes = diagnostic_path.read_bytes()
        self.assertEqual(hashlib.sha256(diagnostic_bytes).hexdigest(), real["diagnostic_sha256"])
        self.assertEqual(
            real["report_sha256"],
            "443644827f14008e4b04ae9bcea53b965114d92614010c38f6a767e4be99e128",
        )
        self.assertEqual(
            real["diagnostic_sha256"],
            "03819305bb2c919d5b36287cc81158e80ba560c1689ffafadb4a2aec361bd701",
        )
        self.assertEqual(
            real["software_sha256"],
            "aa2fa1e905b27c0c8766f28d275d84d178b8f6e228473592a7b3f2af80ee8c53",
        )
        self.assertEqual(
            real["binary_sha256"],
            "86cdfd8bee9a29e377e68bc407459313ef338246253508e10a3096010b046992",
        )
        self.assertEqual(
            (real["inspect_exit"], real["extract_exit"], real["extract_error"], real["cause"]),
            (0, 9, "collision_policy_limit", "max_models"),
        )
        self.assertEqual(
            (real["probe_exit"], real["verify_exit"], real["verify_links_exit"], real["elapsed_seconds"]),
            (-1, -1, -1, 64),
        )
        self.assertFalse(real["network_attempted"])
        self.assertTrue(real["client_metadata_unchanged"])
        self.assertTrue(real["inputs_unchanged"])
        self.assertFalse(real["accepted_prefix"])
        self.assertIsNone(real["audit"])
        self.assertIsNone(real["geometry"])
        counters = real["observed_counters"]
        self.assertEqual(
            tuple(counters[name] for name in (
                "request_calls", "reads", "dependency_requests", "source_bytes", "raw_bytes",
                "models", "resident_bytes", "external", "external_source", "store",
                "memory_limit_bytes", "tolerated_wdt", "tolerated_child", "guards_triggered",
            )),
            (128773, 7218, 29131, 507339175, 77203828, 4096, 376113281, 279511403,
             11870712, 306605630, 1073741824, 2, 2, False),
        )
        self.assertLess(counters["reads"], real["limits_at_failure"]["reads_max"])
        self.assertLess(counters["source_bytes"], real["limits_at_failure"]["source_bytes_max"])
        self.assertEqual(counters["models"], real["limits_at_failure"]["models_max"])
        self.assertEqual(real["limits_at_failure"], {
            "reads_max": 1000000,
            "request_calls_max": 16000000,
            "dependency_requests_max": 16000000,
            "source_bytes_max": 536870912,
            "raw_bytes_max": 268435456,
            "models_max": 4096,
        })
        self.assertEqual(real["software_validation"], {
            "suite_count": 83, "tests_passed": 4504, "tests_ignored": 26, "contracts": 542
        })
        self.assertFalse(real["target_lists_are_observed_output"])
        self.assertIsNone(real["observed_target_lists"])
        self.assertFalse(real["read_capacity_sufficiency_proven"])
        self.assertFalse(real["wmo_462695_admission_proven"])
        self.assertFalse(real["full_world_coverage"])
        self.assertFalse(real["runtime_claim"])

        follow_up = self.contract["follow_up_design"]
        self.assertEqual(follow_up["status"], "design-only")
        self.assertEqual(follow_up["product"], "forever-product-v40")
        self.assertEqual(follow_up["profile"], "ModelHeavyV1")
        self.assertEqual(follow_up["real_target_status"], "target-not-run")
        self.assertFalse(follow_up["implemented"])
        self.assertEqual(follow_up["proposed_model_identity_capacity"], 65536)
        self.assertEqual(follow_up["proposed_coverage_layout_version"], 13)
        self.assertEqual(
            follow_up["capacity_must_be_coherent_across"],
            ["collector/startup models", "spool model identity records", "placement model identities",
             "model staging identities", "package model inventory"],
        )
        self.assertEqual(
            follow_up["preserved_non_identity_limits"],
            {
                "inventory_wdt_cache_entries_max": 4096,
                "stage_callbacks_per_map_tile_max": 4096,
                "units": {
                    "inventory_wdt_cache_entries_max": "WDT/cache inventory entries",
                    "stage_callbacks_per_map_tile_max": "callbacks per map/tile",
                },
                "same_as_read_heavy_v1": True,
            },
        )
        self.assertEqual(follow_up["preserved_limits"]["reads_max"], 1000000)
        self.assertEqual(follow_up["preserved_limits"]["request_calls_max"], 16000000)
        self.assertEqual(follow_up["preserved_limits"]["dependency_requests_max"], 16000000)
        self.assertEqual(follow_up["preserved_limits"]["source_bytes_max"], 536870912)
        self.assertEqual(follow_up["preserved_limits"]["raw_bytes_max"], 268435456)
        self.assertEqual(follow_up["preserved_limits"]["memory_limit_bytes"], 1073741824)
        self.assertFalse(follow_up["disk_world_profile_implied"])
        self.assertFalse(follow_up["capacity_sufficiency_proven"])
        self.assertFalse(follow_up["wmo_462695_admission_proven"])
        self.assertFalse(follow_up["full_world_coverage"])
        self.assertFalse(follow_up["runtime_claim"])

        prior = self.contract["prior_v38_failure_evidence"]
        self.assertEqual(prior["status"], "failed; prior v38 evidence only, not v39 acceptance")
        self.assertEqual(prior["run_id"], "v38-source-heavy-wmo-20261026T163808Z")
        self.assertEqual(prior["report_sha256"], "67a07b1b6fcbe87eb89eba504541252b26ada1295d899dd4ef5c9a3e93b4d54d")
        self.assertEqual(prior["extract_error"], "collision_policy_limit")
        self.assertEqual(prior["cause"], "max_reads")
        self.assertEqual((prior["reads"], prior["v38_reads_max"], prior["v39_reads_max"]), (4096, 4096, 1_000_000))
        self.assertFalse(prior["acceptance_transferred"])
        self.assertFalse(prior["wmo_462695_admission_proven"])
        self.assertFalse(self.contract["verification"]["full_world_coverage"])
        self.assertFalse(self.contract["verification"]["runtime_claim"])
        self.assertFalse(self.contract["verification"]["full_extractor_complete"])
        for name in ("real_extraction", "full_world_coverage", "complete_vmap_component", "runtime_acceptance", "format_parity", "windows", "distribution", "release_publication", "full_extractor_complete"):
            self.assertFalse(self.contract["boundary"][name], name)
        self.assertFalse(self.contract["full_extractor_complete"])


if __name__ == "__main__":
    unittest.main()
