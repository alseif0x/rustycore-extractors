# SPDX-License-Identifier: GPL-3.0-or-later
"""Build-70450 map-domain navigation contract and synthetic plans only."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "contracts/forever-product-contract-v21.json"
FIXTURES = ROOT / "tests/contracts/fixtures/forever-product-v21"


class ProductV21Contract(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads(CONTRACT.read_bytes())

    def test_exact_identity_mask_keys_storage_and_platform_boundary(self):
        c = self.contract
        self.assertEqual(
            (c["contract"], c["version"], c["publication_profile"], c["builds"]),
            ("forever-product-v21", 21, "Build70450NavigationV1", [70450]),
        )
        self.assertEqual((c["platform"], c["backend"], c["offline"]), ("linux", "rust-local", True))
        self.assertEqual(c["commands"], ["inspect", "extract", "resume", "verify", "verify-links"])
        p = c["plan"]
        self.assertEqual(p["schema_catalog"], "wowdbdefs-70450-v1")
        self.assertEqual(p["components"], ["db2", "maps", "vmaps", "mmaps"])
        self.assertEqual(p["component_mask"], 57)
        self.assertEqual(p["admitted_70450_component_masks"], [1, 7, 8, 16, 57])
        self.assertEqual(p["decryption"], "disabled")
        self.assertEqual(p["keys"], "refused")
        self.assertEqual(p["storage"], "linux-leased-append-disk-world-v1")
        self.assertEqual(
            p["collision_budget_fields"],
            ["raw-bytes", "wdt-bytes", "cache-bytes", "placement-bytes", "vmo-bytes", "map-bytes", "output-bytes"],
        )
        self.assertEqual(p["required_db2_entries"], {"Map": 1_349_477, "LiquidType": 1_371_380})
        self.assertFalse(c["full_extractor_complete"])
        self.assertFalse(c["verification"]["navigation_runtime_claim"])

    def test_canonical_map_and_offmesh_plans_are_exact_and_map_bound(self):
        names = ["navigation-map2.tsv", "navigation-map2-offmesh.tsv"]
        self.assertEqual(self.contract["plan"]["fixtures"], names)
        for name in names:
            raw = (FIXTURES / name).read_bytes()
            self.assertTrue(raw.endswith(b"\n"))
            self.assertNotIn(b"\r", raw)
            text = raw.decode("ascii")
            pairs = [line.split("\t") for line in text.splitlines()]
            self.assertTrue(all(len(pair) >= 2 for pair in pairs))
            fields = {
                pair[0]: pair[1]
                for pair in pairs
                if pair[0] not in ("offmesh-connection", "db2")
            }
            self.assertEqual(fields["forever-product"], "21")
            self.assertEqual(fields["build"], "70450")
            self.assertEqual(fields["schema-catalog"], "wowdbdefs-70450-v1")
            self.assertEqual(fields["decryption"], "disabled")
            self.assertEqual(fields["collision-storage"], "linux-leased-append-disk-world-v1")
            self.assertEqual(fields["map-domain"], "map-and-ancestors-v1")
            self.assertEqual(fields["map-id"], "2")
            self.assertEqual(fields["gameobject-scope"], "global-startup-v1")
            self.assertEqual(fields["placement-ids"], "selected-closure-serial-ids-v1")
            self.assertEqual(fields["components"], "db2,maps,vmaps,mmaps")
            self.assertEqual(fields["terrain"], "map:2")
            self.assertEqual(fields["navigation"], "map:2")
            self.assertEqual(fields["offmesh"], "inline-f32-bits-v1")
            db2 = [(pair[1], pair[2]) for pair in pairs if pair[0] == "db2"]
            self.assertEqual(db2, [("LiquidType", "1371380"), ("Map", "1349477")])
        without_connections = (FIXTURES / names[0]).read_text("ascii")
        with_connections = (FIXTURES / names[1]).read_text("ascii")
        self.assertNotIn("offmesh-connection\t", without_connections)
        self.assertEqual(
            [line for line in with_connections.splitlines() if line.startswith("offmesh-connection\t")],
            ["offmesh-connection\t2\t0\t0\t00000000\t00000000\t00000000\t3f800000\t3f800000\t3f800000\t3f000000\t0\t1\t1"],
        )

    def test_domain_and_runtime_gates_are_bound_to_complete_70450_readers(self):
        root = ROOT
        driver = (root / "crates/forever-extractor/src/driver.rs").read_text()
        domain = (root / "crates/forever-extractor/src/map_domain.rs").read_text()
        domain_plan = (root / "crates/forever-extractor/src/plan/map_domain.rs").read_text()
        verify = (root / "crates/forever-extractor/src/verify.rs").read_text()
        inventory = (root / "crates/forever-navigation/src/inventory.rs").read_text()
        load = (root / "crates/forever-navigation/src/terrain/load.rs").read_text()
        terrain_run = (root / "crates/forever-casc/src/terrain/run.rs").read_text()
        terrain_tables = (root / "crates/forever-casc/src/terrain/run/tables.rs").read_text()
        model = (root / "crates/forever-casc/src/package/model.rs").read_text()
        for reader in [
            "read_vmap_maps_70450",
            "read_map_70450",
            "read_navigation_maps_70450",
        ]:
            self.assertIn(reader, domain)
        self.assertIn("Domain::derive_70450", driver)
        self.assertIn("read_navigation_maps_70450", driver)
        self.assertIn("read_navigation_maps_70450", verify)
        self.assertIn("plan.terrain != Selection::Map(id)", domain_plan)
        self.assertIn("plan.navigation != Selection::Map(id)", domain_plan)
        self.assertIn("connection.map_id != id", domain_plan)
        self.assertIn("cfg!(target_os = \"linux\") && build == 70450", inventory)
        self.assertIn("maps::verify_70450(bytes, build, max_bytes)", load)
        self.assertIn("tilelist::TileList::new_70450(build)", terrain_run)
        self.assertIn("conversion::convert_adt_70450", terrain_run)
        self.assertIn("UnsupportedBuild", terrain_run)
        self.assertIn("build_specific_tilelists_preserve_older_headers_and_reject_unknowns", terrain_run)
        for reader in [
            "read_map_70450",
            "read_liquid_materials_70450",
            "read_liquid_objects_70450",
            "read_liquid_types_70450",
        ]:
            self.assertIn(reader, terrain_tables)
        self.assertIn("context70450_admits_only_the_five_linux_implemented_product_masks", model)
        self.assertIn("Component::Navigation.bit(),", model)
        self.assertIn("for components in 0..=u8::MAX", model)

        plan_tests = (root / "crates/forever-extractor/src/plan/v21_tests.rs").read_text()
        navigation_tests = driver.split("mod v21_navigation_tests {", 1)[1]
        for name in [
            "v21_fails_closed_on_foreign_build_catalog_and_incomplete_navigation_inputs",
            "v21_domain_path_rejects_a_hash_the_pinned_path_accepts",
            "v21_navigation_producer_uses_the_70450_map_reader",
            "v21_unavailable_or_encrypted_map_fails_before_creating_package_output",
            "v21_full_extract_emits_and_verifies_all_components_and_mmap_containers",
        ]:
            self.assertIn(name, plan_tests + navigation_tests)
        self.assertIn("TileView::parse", navigation_tests)
        self.assertIn("crate::verify::verify(&plan, &output", navigation_tests)
        self.assertIn("local_store_no_verified_encoding", navigation_tests)
        self.assertIn("file_unavailable", navigation_tests)
        self.assertIn("verify_70450", load)
        readme = (root / "README.md").read_text()
        guide = (root / "docs/forever-product-v21.md").read_text()
        workflow = (root / ".github/workflows/forever-rust.yml").read_text()
        self.assertIn("[Product v21 scope](docs/forever-product-v21.md)", readme)
        self.assertIn("no real-client navigation evidence", guide)
        self.assertIn("Check product v21 contract and synthetic CLI", workflow)

    def test_real_input_blockers_and_absence_of_real_evidence_are_explicit(self):
        policy = self.contract["source_policy"]
        self.assertIn("encrypted", policy["real_run_requirement"])
        self.assertIn("VMap", policy["real_run_requirement"])
        self.assertIn("No keys are accepted", policy["real_run_requirement"])
        self.assertIn("build-70450 catalogue readers", policy["terrain_stage_status"])
        self.assertIn("no real client run is claimed", policy["terrain_stage_status"])
        self.assertEqual(policy["real_navigation_evidence"], "none")
        self.assertIn("not an independent client snapshot", self.contract["verification"]["source_authentication"])


if __name__ == "__main__":
    unittest.main()
