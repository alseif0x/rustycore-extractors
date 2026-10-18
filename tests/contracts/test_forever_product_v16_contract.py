# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit 70450 DB2-only profile facts and source wiring; no client or keys."""
from pathlib import Path
import hashlib
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ProductV16Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v16.json").read_bytes()
        )

    def test_identity_and_exact_component_scope(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["version"],
                contract["contract"],
                contract["publication_profile"],
            ),
            (16, "forever-product-v16", "Build70450Db2OnlyV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["platform"], "linux")
        self.assertEqual(contract["backend"], "rust-local")
        self.assertEqual(contract["offline"], True)
        self.assertEqual(
            contract["commands"],
            ["inspect", "extract", "resume", "verify", "verify-links"],
        )
        plan = contract["plan"]
        self.assertEqual(plan["schema_catalog"], {
            "id": "wowdbdefs-70450-v1",
            "required_build": 70450,
            "table_count": 1161,
            "provenance": "contracts/wowdbdefs-70450-v1.json and crates/forever-db2/src/records/wowdbdefs_70450_generated.rs",
            "source_commit": "bfebcbc5181f47b954e7886862f92c5577dbbe4a",
            "source_blob": "f95528066d44e1b70ea6cd124a35a5a63685d384",
            "source_sha256": "f7b752a991d6a0321d07ecb7f06e62cd94694c1594d2fe5ce0cb7b243c71559a",
            "facts_sha256": "4589beb2e106f5bba4b8a1801b051f40915dca51dc0aafe85350b5b3c24b29c8",
        })
        self.assertEqual(plan["components"], ["db2"])
        self.assertEqual(plan["component_mask"], 1)
        self.assertEqual(plan["collision_storage"], "resident")
        self.assertEqual(plan["map_domain"], "field absent")
        self.assertEqual(plan["terrain"], ["none"])
        self.assertEqual(plan["navigation"], ["none"])
        self.assertEqual(plan["decryption"], "required after schema-catalog; disabled only")
        self.assertIn("Field must be absent", plan["offmesh_connections"])
        self.assertEqual(contract["provided_keys"]["required_for"], [])
        self.assertIn("product_keys_forbidden", contract["failure_codes"]["key_option"])
        self.assertIn("--keys is forbidden for verify and verify-links", contract["verification"]["metadata_preflight"])
        self.assertFalse(contract["full_extractor_complete"])

    def test_catalog_provenance_matches_the_pinned_70450_facts(self):
        raw = (ROOT / "contracts/wowdbdefs-70450-v1.json").read_bytes()
        catalog = json.loads(raw)
        schema = self.contract["plan"]["schema_catalog"]
        self.assertEqual(catalog["catalog_id"], schema["id"])
        self.assertEqual(catalog["target"]["build"], schema["required_build"])
        self.assertEqual(len(catalog["tables"]), schema["table_count"])
        self.assertEqual(catalog["source"]["commit"], schema["source_commit"])
        self.assertEqual(catalog["source"]["manifest_blob"], schema["source_blob"])
        self.assertEqual(catalog["source"]["manifest_sha256"], schema["source_sha256"])
        self.assertEqual(hashlib.sha256(raw).hexdigest(), schema["facts_sha256"])

    def test_disabled_plan_fixture_is_canonical_and_selects_only_catalog_db2(self):
        data = (ROOT / "tests/contracts/fixtures/forever-product-v16/disabled.tsv").read_text()
        self.assertTrue(data.endswith("\n"))
        self.assertTrue(data.startswith("forever-product\t16\nbuild\t70450\n"))
        self.assertIn("schema-catalog\twowdbdefs-70450-v1\n", data)
        self.assertIn("decryption\tdisabled\n", data)
        self.assertIn("components\tdb2\n", data)
        self.assertIn("terrain\tnone\n", data)
        self.assertIn("navigation\tnone\n", data)
        self.assertNotIn("collision-storage", data)
        self.assertNotIn("collision-domain", data)
        self.assertNotIn("offmesh\t", data)
        ordinary_keys = [
            line.split("\t", 1)[0]
            for line in data.splitlines()
            if not line.startswith("db2\t")
        ]
        self.assertEqual(ordinary_keys, self.contract["plan"]["ordered_keys"])
        table = [line.split("\t") for line in data.splitlines() if line.startswith("db2\t")]
        self.assertEqual(table, [["db2", "AccountStoreCategory", "6220124"]])

    def test_strict_db2_decode_and_existing_product_gates_are_retained(self):
        contract = self.contract
        self.assertIn("strict full-file decode", contract["limits"]["full_source_decode"])
        self.assertIn("local_store_no_verified_encoding", contract["limits"]["full_source_decode"])
        self.assertIn("never relaxed", contract["limits"]["full_source_decode"])
        self.assertIn("local_store_no_verified_encoding", contract["failure_codes"]["no_verified_encoding"])
        self.assertIn("run.put(", (ROOT / "crates/forever-extractor/src/driver.rs").read_text())
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        self.assertIn("admit_db2_70450", driver)
        self.assertIn("open_db2_70450", driver)
        self.assertIn("read_file_data_id_with_limit", driver)
        self.assertIn("WowDbDefs70450V1", driver)

    def test_global_admission_stays_closed_and_v15_collision_identity_is_untouched(self):
        admission = (ROOT / "crates/forever-casc/src/admission.rs").read_text()
        legacy = admission.split("pub const SUPPORTED_CLIENTS:", 1)[1].split("];", 1)[0]
        self.assertNotIn("70450", legacy)
        self.assertIn("admit_db2_70450", admission)

        v15 = json.loads((ROOT / "contracts/forever-product-contract-v15.json").read_bytes())
        self.assertEqual(v15["version"], 15)
        self.assertEqual(v15["builds"], [70450])
        self.assertEqual(v15["publication_profile"], "Build70450CompatibleCollisionV1")
        self.assertEqual(v15["plan"]["components"], ["vmaps"])
        self.assertEqual(v15["plan"]["component_mask"], 16)
        self.assertEqual(
            v15["plan"]["collision_profile"],
            "linux70450-compatible-gnu15-vmap-product-v1",
        )
        self.assertTrue(self.contract["legacy_preservation"]["v15_collision_profile_untouched"])
        self.assertEqual(
            self.contract["legacy_preservation"]["canonical_versions"],
            [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        )

    def test_linux_contract_step_runs_the_new_contract_test(self):
        workflow = json.loads((ROOT / ".github/workflows/forever-rust.yml").read_bytes())
        step = next(
            item
            for item in workflow["jobs"]["linux"]["steps"]
            if item.get("name") == "Check source contracts and synthetic Python oracle"
        )
        self.assertIn(
            "python -m unittest discover -s tests/contracts -p test_forever_product_v16_contract.py -v",
            step["run"].splitlines(),
        )


if __name__ == "__main__":
    unittest.main()
