# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: synthetic state-v2 schema and durability contract checks.
import hashlib
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator
ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT/'schemas/forever-package-state-v2.schema.json').read_text())
CONTRACT = json.loads((ROOT/'contracts/forever-package-state-v2.json').read_text())
VALIDATOR = Draft202012Validator(SCHEMA)
def protocol():
    return dict(state_version=2, context_sha256='1'*64)
def transaction():
    return dict(transaction_version=2, context_sha256='1'*64,
                artifact=dict(component='mmaps',path='mmaps/x',bytes=3,
                              sha256=hashlib.sha256(b'abc').hexdigest()))
class PackageState(unittest.TestCase):
    def reject(self,value):
        self.assertTrue(list(VALIDATOR.iter_errors(value)))
    def test_schema_self_validates(self):
        Draft202012Validator.check_schema(SCHEMA)
    def test_protocol_and_transaction_are_distinct(self):
        VALIDATOR.validate(protocol())
        VALIDATOR.validate(transaction())
        v=protocol()
        v.update(transaction())
        self.reject(v)
    def test_unknown_version_rejects(self):
        for maker,key in [(protocol,'state_version'),(transaction,'transaction_version')]:
            for version in [1,3,0,-1,'2']:
                v=maker();v[key]=version;self.reject(v)
    def test_context_digest_profile_is_required(self):
        for digest in ['0'*64,'A'*64,'1'*63,'1'*64+'\n']:
            v=protocol();v['context_sha256']=digest;self.reject(v)
    def test_missing_or_extra_properties_reject(self):
        for maker in [protocol,transaction]:
            v=maker();v['extra']=1;self.reject(v)
            for key in maker():
                v=maker();del v[key];self.reject(v)
    def test_transaction_cannot_rename_or_change_component_prefix(self):
        for path in ['maps/x','mmaps/../x','mmaps/.x','mmaps/a\n']:
            v=transaction();v['artifact']['path']=path;self.reject(v)
    def test_file_budgets_apply_to_declared_expected_bytes(self):
        for size in [-1,134217729]:
            v=transaction();v['artifact']['bytes']=size;self.reject(v)
    def test_legacy_final_manifest_contract_remains_v1(self):
        self.assertEqual(CONTRACT['manifest_version'],1)
        self.assertIn('not migrated',CONTRACT['legacy'])
        legacy=json.loads((ROOT/'contracts/forever-package-contract-v1.json').read_text())
        self.assertIn('Not admitted',legacy['resume'])
    def test_preflight_before_mutation_is_required(self):
        self.assertTrue(any('before complete preflight' in s for s in CONTRACT['required_preflight']))
        self.assertIn('same inode',CONTRACT['required_preflight'][4])
    def test_restart_does_not_claim_computational_checkpoint_or_runtime(self):
        self.assertIn('computation is replayed',CONTRACT['producer_replay'])
        self.assertIn('semantic or new runtime compatibility acceptance',CONTRACT['excluded'])
if __name__=='__main__':
    unittest.main()
