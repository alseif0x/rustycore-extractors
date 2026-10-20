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
        for item in ("no full-world geometry or navigation acceptance",
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


if __name__ == "__main__":
    unittest.main()
