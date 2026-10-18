# SPDX-License-Identifier: GPL-3.0-or-later
"""Explicit 70450 supplied-key DB2 profile facts; no client or key input."""
from pathlib import Path
import hashlib
import json
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ProductV17Contract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.contract = json.loads(
            (ROOT / "contracts/forever-product-contract-v17.json").read_bytes()
        )

    def test_identity_and_exact_supplied_key_db2_scope(self):
        contract = self.contract
        self.assertEqual(
            (
                contract["version"],
                contract["contract"],
                contract["publication_profile"],
            ),
            (17, "forever-product-v17", "Build70450Db2ProvidedKeysV1"),
        )
        self.assertEqual(contract["builds"], [70450])
        self.assertEqual(contract["platform"], "linux")
        self.assertEqual(contract["backend"], "rust-local")
        self.assertTrue(contract["offline"])
        self.assertEqual(
            contract["commands"],
            ["inspect", "extract", "resume", "verify", "verify-links"],
        )
        plan = contract["plan"]
        self.assertEqual(plan["schema_catalog"]["id"], "wowdbdefs-70450-v1")
        self.assertEqual(plan["schema_catalog"]["required_build"], 70450)
        self.assertEqual(plan["schema_catalog"]["table_count"], 1161)
        self.assertEqual(plan["components"], ["db2"])
        self.assertEqual(plan["component_mask"], 1)
        self.assertEqual(plan["collision_storage"], "resident")
        self.assertEqual(plan["map_domain"], "field absent")
        self.assertEqual(plan["terrain"], ["none"])
        self.assertEqual(plan["navigation"], ["none"])
        self.assertEqual(
            plan["decryption"],
            "required after schema-catalog; provided-salsa20-v1 only",
        )
        self.assertIn("Field must be absent", plan["offmesh_connections"])
        self.assertEqual(
            contract["provided_keys"]["required_for"],
            [
                "extract/resume when decryption=provided-salsa20-v1",
                "inspect --client when decryption=provided-salsa20-v1",
            ],
        )
        self.assertIn("product_keys_required", contract["failure_codes"]["missing_option"])
        self.assertIn("product_keys_forbidden", contract["failure_codes"]["disabled_option"])
        self.assertIn("verify", contract["provided_keys"]["forbidden_for"])
        self.assertIn("verify-links", contract["provided_keys"]["forbidden_for"])
        self.assertFalse(contract["full_extractor_complete"])

    def test_catalog_provenance_and_canonical_provided_plan(self):
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

        data = (ROOT / "tests/contracts/fixtures/forever-product-v17/provided.tsv").read_text()
        self.assertTrue(data.endswith("\n"))
        self.assertTrue(data.startswith("forever-product\t17\nbuild\t70450\n"))
        self.assertIn("schema-catalog\twowdbdefs-70450-v1\n", data)
        self.assertIn("decryption\tprovided-salsa20-v1\n", data)
        self.assertIn("components\tdb2\n", data)
        self.assertIn("terrain\tnone\n", data)
        self.assertIn("navigation\tnone\n", data)
        self.assertNotIn("collision-storage", data)
        self.assertNotIn("collision-domain", data)
        self.assertNotIn("offmesh\t", data)
        ordered = [
            line.split("\t", 1)[0]
            for line in data.splitlines()
            if not line.startswith("db2\t")
        ]
        self.assertEqual(ordered, self.contract["plan"]["ordered_keys"])
        self.assertEqual(
            [line.split("\t") for line in data.splitlines() if line.startswith("db2\t")],
            [["db2", "AccountStoreCategory", "6220124"]],
        )

        targets = (ROOT / "tests/contracts/fixtures/forever-product-v17/encrypted-targets.tsv").read_text()
        self.assertTrue(targets.startswith("forever-product\t17\nbuild\t70450\n"))
        self.assertEqual(
            [line.split("\t") for line in targets.splitlines() if line.startswith("db2\t")],
            [
                ["db2", "GameObjectDisplayInfo", "1266277"],
                ["db2", "Map", "1349477"],
            ],
        )

    def test_key_custody_and_partial_key_lists_are_not_completion_proof(self):
        keys = self.contract["provided_keys"]
        self.assertEqual(keys["option"], "--keys PATH")
        self.assertEqual(keys["max_file_bytes"], 262144)
        self.assertEqual(keys["max_entries"], 4096)
        self.assertIn("sorted unique lowercase hex16", keys["record_format"])
        self.assertFalse(keys["discovery"])
        self.assertFalse(keys["built_in_keys"])
        self.assertFalse(keys["network"])
        self.assertIn("explicit regular file", keys["input"])
        self.assertIn(
            "Key names, values, counts, hashes and input paths are excluded",
            self.contract["identities"]["keys"],
        )
        self.assertIn("supplied-key full-file decode route", self.contract["limits"]["full_source_decode"])
        self.assertIn("local_store_no_verified_encoding", self.contract["limits"]["full_source_decode"])
        self.assertIn("public key list", self.contract["verification"]["key_coverage"])
        self.assertIn("partial", self.contract["verification"]["key_coverage"])

    def test_v14_v15_v16_and_generic_admission_remain_distinct(self):
        v14 = json.loads((ROOT / "contracts/forever-product-contract-v14.json").read_bytes())
        v15 = json.loads((ROOT / "contracts/forever-product-contract-v15.json").read_bytes())
        v16 = json.loads((ROOT / "contracts/forever-product-contract-v16.json").read_bytes())
        self.assertEqual((v14["version"], v14["builds"]), (14, [70425]))
        self.assertEqual(v15["publication_profile"], "Build70450CompatibleCollisionV1")
        self.assertEqual(v15["plan"]["components"], ["vmaps"])
        self.assertEqual(v15["plan"]["component_mask"], 16)
        self.assertEqual(
            v15["plan"]["collision_profile"],
            "linux70450-compatible-gnu15-vmap-product-v1",
        )
        self.assertEqual(v16["publication_profile"], "Build70450Db2OnlyV1")
        self.assertEqual(v16["plan"]["decryption"], "required after schema-catalog; disabled only")
        self.assertTrue(self.contract["legacy_preservation"]["catalog_does_not_select_v17"])
        self.assertEqual(
            self.contract["legacy_preservation"]["canonical_versions"],
            [1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17],
        )
        admission = (ROOT / "crates/forever-casc/src/admission.rs").read_text()
        supported = admission.split("pub const SUPPORTED_CLIENTS:", 1)[1].split("];", 1)[0]
        self.assertNotIn("70450", supported)
        self.assertIn("admit_db2_70450", admission)

    def test_keyed_70450_bootstrap_and_existing_readers_are_reused(self):
        store = (ROOT / "crates/forever-casc/src/local_store.rs").read_text()
        driver = (ROOT / "crates/forever-extractor/src/driver.rs").read_text()
        self.assertIn("open_db2_70450_with_keys", store)
        self.assertIn("BootstrapTarget::Db2Build70450", store)
        self.assertIn("admit_db2_70450", driver)
        self.assertIn("open_db2_70450_with_keys", driver)
        self.assertIn("read_file_data_id_with_limit", driver)
        self.assertIn("run.put(", driver)

    def test_linux_ci_runs_the_v17_contract_and_synthetic_cli(self):
        workflow = json.loads((ROOT / ".github/workflows/forever-rust.yml").read_bytes())
        step = next(
            item
            for item in workflow["jobs"]["linux"]["steps"]
            if item.get("name") == "Check product v17 contract and synthetic CLI"
        )
        self.assertIn(
            "python -m unittest discover -s tests/contracts -p test_forever_product_v17_contract.py -v",
            step["run"].splitlines(),
        )
        self.assertIn(
            "cargo test --offline --locked -p forever-extractor --test product_v17_cli --no-default-features -- --test-threads=1",
            step["run"].splitlines(),
        )


if __name__ == "__main__":
    unittest.main()
