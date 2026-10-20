# SPDX-License-Identifier: GPL-3.0-or-later
"""Selected Linux navigation consumer ownership and admission boundaries."""
import hashlib
import json
from pathlib import Path
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROFILE = "forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-consumer-source-v13"


class NavigationConsumerContract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(
            (ROOT / "contracts/forever-rustycore-navigation-consumer-contract-v1.json").read_text()
        )

    def test_explicit_selected_profile_and_remaining_scope(self):
        contract = self.contract
        self.assertEqual(contract["contract"], "forever-rustycore-navigation-consumer-v1")
        self.assertEqual(contract["version"], 1)
        self.assertEqual(contract["issue"], 104)
        self.assertEqual(contract["scope"]["platform"], "linux")
        self.assertEqual(contract["scope"]["client_build"], 70450)
        self.assertEqual(contract["scope"]["mmap_version"], 16)
        self.assertEqual(contract["scope"]["detour_version"], 7)
        self.assertEqual(contract["scope"]["detour_reference_bits"], 64)
        self.assertEqual(contract["scope"]["selected_maps_per_package"], 1)
        self.assertEqual(contract["scope"]["offmesh_connections"], 0)
        self.assertFalse(contract["scope"]["server_activation"])
        for item in ("no new extractor product or archive qualification",
                     "no complete real geometry acceptance",
                     "no full-world geometry or navigation acceptance",
                     "no server gameplay, creation or world-entry acceptance",
                     "no recovered encryption keys",
                     "no Windows delivery or release authorization"):
            self.assertIn(item, contract["non_claims"])

    def test_one_component_codec_and_generic_envelope_owner(self):
        admission = self.contract["admission"]
        self.assertEqual(admission["component_owner"], "forever-navigation::package_v23")
        self.assertEqual(admission["envelope_owner"], "forever-casc::package")
        cli = (ROOT / "crates/forever-extractor/src/driver/navigation_v23.rs").read_text()
        schema = (ROOT / "crates/forever-navigation/src/package_v23/schema.rs").read_text()
        self.assertIsNone(re.search(r"^struct (?:Coverage|SourceIdentity)\b", cli, re.M))
        self.assertRegex(schema, r"pub struct Coverage\b")
        self.assertRegex(schema, r"pub struct SourceIdentity\b")
        self.assertIn("package_v23::verify(", cli)
        self.assertIn("into_parts()", cli)
        self.assertIn("native Detour topology safety from framing or hashes alone", admission["not_proven"])

    def test_owned_batch_scope_has_no_provisional_native_callback(self):
        batch = self.contract["owned_read_batch"]
        self.assertTrue(batch["same_verification_pass"])
        self.assertTrue(batch["global_container_included"])
        self.assertFalse(batch["caller_callbacks_before_final_verification"])
        self.assertEqual(batch["defaults"]["maximum_requested_tiles"], 64)
        self.assertEqual(batch["defaults"]["maximum_retained_bytes"], 128 * 1024 * 1024)
        self.assertIn("unavailable", batch["availability"])
        self.assertIn("initially admitted context", batch["later_loads"])
        source = (ROOT / "crates/forever-navigation/src/package_v23/verify.rs").read_text()
        self.assertIn("pub fn read_batch(", source)
        self.assertIn("package::verify_with_preflight(", source)
        self.assertIn("pub fn into_parts(", source)
        self.assertIn("pub fn bytes(", source)

    def test_only_reviewed_linux_path_edge_is_added(self):
        manifest = tomllib.loads((ROOT / "crates/forever-navigation/Cargo.toml").read_text())
        edge = manifest["target"]['cfg(target_os = "linux")']["dependencies"]["forever-casc"]
        self.assertEqual(edge["path"], "../forever-casc")
        self.assertFalse(edge["default-features"])
        lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
        packages = {item["name"]: item for item in lock["package"]}
        self.assertIn("forever-casc", packages["forever-navigation"]["dependencies"])
        self.assertNotIn("forever-navigation", packages["forever-casc"]["dependencies"])
        registry = {(item["name"], item["version"]) for item in lock["package"] if "source" in item}
        self.assertEqual(registry, {("adler2", "2.0.1"), ("miniz_oxide", "0.8.9")})

    def test_shared_edge_exact_inverse_preserves_historical_source_pins(self):
        historical = json.loads((ROOT / "contracts/forever-windows-navigation-contract-v1.json").read_text())
        deltas = self.contract["preserved_windows_source_deltas"]
        self.assertEqual({entry["path"] for entry in deltas},
                         {"Cargo.lock", "crates/forever-navigation/Cargo.toml"})
        for entry in deltas:
            with self.subTest(path=entry["path"]):
                current = (ROOT / entry["path"]).read_bytes()
                self.assertEqual(entry["baseline_sha256"], historical["validated_import_sha256"][entry["path"]])
                self.assertEqual(hashlib.sha256(current).hexdigest(), entry["candidate_sha256"])
                after = entry["after"].encode("utf-8")
                self.assertEqual(current.count(after), 1)
                restored = current.replace(after, entry["before"].encode("utf-8"), 1)
                self.assertEqual(hashlib.sha256(restored).hexdigest(), entry["baseline_sha256"])

    def test_native_owner_and_runtime_acceptance_remain_explicit(self):
        native = self.contract["native_ingress"]
        owner = self.contract["rustycore_owner"]
        self.assertEqual(native["owner"], "wow-recastdetour")
        self.assertEqual(owner["native_owner"], "existing MMapManager/MMapData")
        self.assertIn("dedicated native pathfinder owner thread", owner["execution"])
        self.assertIn("remain errors", owner["errors"])
        self.assertIn("partial outcomes remain visible", owner["status"])
        self.assertIn("Failed preparation", owner["rollback"])
        self.assertEqual(owner["consumer_defaults"]["instance_query_nodes"], 4096)
        self.assertEqual(owner["consumer_defaults"]["maximum_corridor_references"], 256)
        self.assertEqual(owner["consumer_defaults"]["maximum_instance_queries"], 64)
        self.assertEqual(owner["consumer_defaults"]["maximum_loaded_artifact_bytes"], 128 * 1024 * 1024)
        self.assertIn("strict native ingress malformed-index, size, numeric, portal, detail and BV rejection cases before FFI",
                      self.contract["acceptance"]["required"])
        self.assertEqual(self.contract["status"], "bounded_local_accepted_core_merged")
        self.assertEqual(owner["implementation_status"], self.contract["status"])
        publication = self.contract["rustycore_publication"]
        self.assertEqual(publication["state"], "bounded_consumer_squash_merged")
        self.assertEqual(publication["pull_request"], "https://github.com/alseif0x/rustycore/pull/1454")
        for field in ("tested_code_commit", "publication_commit", "base_commit"):
            self.assertRegex(publication[field], r"^[0-9a-f]{40}$")
        self.assertEqual(publication["component_tests_passed"], 5944)
        self.assertEqual(publication["component_tests_ignored"], 3)
        self.assertEqual(publication["tested_code_commit"], "1bf5ee4b48d8f729e806f7acddfdebad01025b58")
        self.assertEqual(publication["publication_commit"], "803449bd030ca68d48b906880b78ca8a31810751")
        self.assertEqual(publication["base_commit"], "b8330b1b5ba50c7ec870b8ca9f1f0969df8e667a")
        self.assertEqual(publication["merged_branch"], "forever")
        self.assertEqual(publication["merged_commit"], "984a0c6abc6ac0b00bacaaaf58224058d7297df3")
        self.assertEqual(publication["merge_strategy"], "squash")
        self.assertTrue(publication["merged"])
        self.assertEqual(
            publication["tree_identity"],
            {
                "published_qualified_tree": "7d87fdb8416dd871f97314719ecd1527a29f08bf",
                "merged_tree": "7d87fdb8416dd871f97314719ecd1527a29f08bf",
                "matches": True,
            },
        )
        guide = publication["readme_guide"]
        self.assertEqual(guide["pull_request"], "https://github.com/alseif0x/rustycore/pull/1456")
        self.assertEqual(guide["merged_commit"], "33820338567870954b9a4a8003aa417efefb12f1")
        self.assertEqual(guide["merge_strategy"], "squash")
        self.assertIn("README-only", guide["scope"])
        self.assertEqual(guide["extractor_source_commit"], "3baac307329bd0391cd7f083a55e03b982081b27")
        self.assertEqual(
            guide["extractor_cargo_lock_sha256"],
            "8afafe1f5f8b9c98f53ed55155160b2a6d21f6efb5edf1d1c7f0f6e60819af27",
        )
        self.assertFalse(guide["ci_bootstrap_included"])
        self.assertTrue(publication["documentation_only_delta_after_code_qualification"])
        self.assertTrue(publication["sdk_physical_failure_repaired_without_ceiling_change"])
        self.assertTrue(publication["existing_inherited_debt_experimental_publication_exception_used"])
        for field in ("general_final_accepted", "new_validation_failure_waived",
                      "server_runtime_accepted", "shipped_extractor_product_sources_changed",
                      "complete_geometry_accepted", "full_world_accepted",
                      "frozen_v13_qualification_scope_changed",
                      "extractor_product_acceptance_transferred",
                      "new_extractor_product_acceptance_established"):
            self.assertFalse(publication[field], field)

        amendment = self.contract["bootstrap_amendment"]
        self.assertEqual(amendment["candidate_commit"], "c89279417bc39a97c703c73cc485099a852640af")
        self.assertEqual(amendment["candidate_status"], "historical_qualified_local_unpublished")
        self.assertEqual(amendment["branch"], "forever-navigation-ci-bootstrap")
        self.assertTrue(amendment["current_rebase_requires_separate_qualification"])
        self.assertTrue(amendment["workflow_bytes_match_historical_candidate"])
        self.assertFalse(amendment["published"])
        self.assertFalse(amendment["workflow_amendment_published"])
        self.assertEqual(amendment["push_status"], "pending_missing_github_oauth_workflow_scope")
        self.assertEqual(amendment["push_attempt_result"], "rejected_missing_github_oauth_workflow_scope")
        self.assertFalse(amendment["push_retry_or_bypass_attempted"])
        self.assertEqual(amendment["remote_pr_publication_commit"], publication["publication_commit"])
        self.assertTrue(amendment["remote_pr_publication_commit_is_historical"])
        self.assertEqual(amendment["source_pin"]["commit"], "3baac307329bd0391cd7f083a55e03b982081b27")
        self.assertEqual(
            amendment["source_pin"]["cargo_lock_sha256"],
            "8afafe1f5f8b9c98f53ed55155160b2a6d21f6efb5edf1d1c7f0f6e60819af27",
        )
        focused = amendment["focused_checks"]
        self.assertEqual(focused["actionlint_version"], "1.7.12")
        self.assertTrue(focused["actionlint_passed"])
        self.assertTrue(focused["bash_n_passed"])
        self.assertEqual(focused["shellcheck"], "unavailable")
        self.assertEqual(focused["synthetic_cases_passed"], 16)
        self.assertEqual(
            focused["synthetic_case_scope"],
            [
                "fresh checkout success",
                "existing directory, file, symlink and dangling symlink rejection",
                "wrong HEAD and wrong lock rejection",
                "fetch failure",
            ],
        )
        self.assertTrue(focused["public_exact_sha_fetch_passed"])
        self.assertTrue(focused["extractor_lock_checksum_passed"])
        self.assertTrue(focused["cargo_metadata"]["passed"])
        self.assertTrue(focused["cargo_metadata"]["feature_disabled"])
        self.assertEqual(focused["cargo_metadata"]["workspace_packages"], 38)
        self.assertIn("not one green campaign", focused["retained_path_diagnostic"])
        self.assertEqual(
            focused["focused_receipt_sha256"],
            "b95dea4b5d570f49fdfbd768ed0ad0c38fbb521f3fcaacd1a01c3532288c490e",
        )
        self.assertEqual(
            focused["live_receipt_sha256"],
            "496c840cbc526adc0a58ff1a63091a6d7d7bffb000c963267db78084bd0707b2",
        )
        final = amendment["core_final"]
        self.assertEqual(final["elapsed_seconds"], 52.636)
        self.assertEqual(
            final["receipt_sha256"],
            "c1c363146e52b039d4a61786e533e8b9cc0c4c899c0578d4328c39a92fc64e71",
        )
        self.assertFalse(final["general_final_accepted"])
        self.assertTrue(final["same_inherited_rejection"])
        self.assertFalse(final["new_validation_failure"])
        self.assertFalse(final["new_validation_failure_waived"])
        self.assertTrue(amendment["core_product_rust_unchanged"])
        self.assertTrue(amendment["core_manifests_unchanged"])
        self.assertTrue(amendment["core_lock_unchanged"])
        self.assertTrue(amendment["core_dependency_policy_unchanged"])
        self.assertTrue(amendment["does_not_change_historical_publication_or_code_qualification"])
        self.assertFalse(amendment["server_runtime_accepted"])


if __name__ == "__main__":
    unittest.main()
