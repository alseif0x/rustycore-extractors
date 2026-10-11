# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: synthetic package v1 schema and contract checks.
import copy
import hashlib
import json
from pathlib import Path
import unittest

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / 'schemas/forever-package-v1.schema.json').read_text())
CONTRACT = json.loads((ROOT / 'contracts/forever-package-contract-v1.json').read_text())
VALIDATOR = jsonschema.Draft202012Validator(SCHEMA)


def fixture():
    context = dict(build=70205, locale='enUS', components=63,
                   source_sha256='1' * 64, plan_sha256='2' * 64,
                   producer_sha256='3' * 64)
    facts = [dict(component=c, path=p, bytes=0,
                  sha256=hashlib.sha256(b'').hexdigest())
             for c, p in [('db2', 'dbc/enUS/X.db2'), ('gt', 'gt/xp.txt'),
                          ('cameras', 'cameras/1.m2'), ('maps', 'maps/0001.map'),
                          ('vmaps', "vmaps/a model (test)'s.vmo"),
                          ('mmaps', 'mmaps/0001.mmap')]]
    facts.sort(key=lambda a: a['path'].lower())
    digest = hashlib.sha256(''.join(json.dumps(a, separators=(',', ':')) + '\n'
                                    for a in facts).encode()).hexdigest()
    return dict(manifest_version=1, scope='artifact_integrity', status='complete',
                context=context, artifacts=facts, file_count=len(facts),
                total_bytes=0, inventory_sha256=digest)


class PackageContract(unittest.TestCase):
    def reject(self, value):
        self.assertTrue(list(VALIDATOR.iter_errors(value)))

    def test_schema_self_validates(self):
        jsonschema.Draft202012Validator.check_schema(SCHEMA)

    def test_all_six_component_manifest(self):
        VALIDATOR.validate(fixture())

    def test_complete_empty_is_structurally_valid(self):
        f = fixture()
        f.update(artifacts=[], file_count=0,
                 inventory_sha256=hashlib.sha256(b'').hexdigest())
        VALIDATOR.validate(f)

    def test_extra_properties_and_bad_version_or_scope_reject(self):
        for key, value in [('extra', 1), ('manifest_version', 2),
                           ('scope', 'runtime_compatible'), ('status', 'failed')]:
            f = fixture()
            f[key] = value
            self.reject(f)

    def test_context_legacy_unknown_locale_and_selection_reject(self):
        for key, value in [('build', 51943), ('locale', 'frFR'),
                           ('components', 0), ('components', 64)]:
            f = fixture()
            f['context'][key] = value
            self.reject(f)

    def test_digest_lexical_profile(self):
        for value in ['0' * 63, 'A' * 64, 'g' * 64, '1' * 65]:
            f = fixture()
            f['inventory_sha256'] = value
            self.reject(f)

    def test_unsafe_paths_reject(self):
        for path in ['mmaps/../x', 'mmaps/.x', 'mmaps/a.', 'mmaps/a ',
                     '/mmaps/x', 'mmaps//x', 'mmaps/x/', 'mmaps/é',
                     'mmaps/a\\b', 'mmaps/a"b', 'mmaps/a:b', 'mmaps/x\n']:
            f = fixture()
            f['artifacts'] = [dict(component='mmaps', path=path, bytes=0,
                                   sha256='1' * 64)]
            self.reject(f)

    def test_prefixes_and_file_limits_reject(self):
        for change in [dict(path='maps/a'), dict(component='unknown'),
                       dict(bytes=-1), dict(bytes=134217729)]:
            f = fixture()
            a = dict(component='mmaps', path='mmaps/a', bytes=0, sha256='1' * 64)
            a.update(change)
            f['artifacts'] = [a]
            self.reject(f)

    def test_schema_alone_cannot_prove_hashes_or_completion(self):
        f = copy.deepcopy(fixture())
        f['file_count'] = 0
        f['inventory_sha256'] = '0' * 64
        # Byte verifier additionally checks cardinality, hashes, receipts/seals,
        # context binding and exact canonical serialization against actual files.
        VALIDATOR.validate(f)
        self.assertFalse(CONTRACT['semantic_compatibility'])
        self.assertIn('Not admitted', CONTRACT['resume'])

    def test_component_bits_and_limits_are_explicit(self):
        self.assertEqual(CONTRACT['component_bits'],
                         dict(db2=1, gt=2, cameras=4, maps=8, vmaps=16, mmaps=32))
        self.assertEqual(sum(CONTRACT['component_bits'].values()), 63)
        self.assertEqual(CONTRACT['limits']['minimum_metadata_bytes'], 131072)
        self.assertEqual(CONTRACT['limits']['file_bytes'], 134217728)


if __name__ == '__main__':
    unittest.main()
