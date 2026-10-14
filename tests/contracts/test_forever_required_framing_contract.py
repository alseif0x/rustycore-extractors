# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original static complete-span framing/source scope checks.
"""Static consistency only; no execution/client access or runtime proof."""
import ast
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'forever-seven-crate-linux-product-db2-gt-cameras-source-v11'
HISTORICAL_PROFILE = 'forever-seven-crate-windows-navigation-readers-source-v7'


def literal(name):
    for node in ast.parse((ROOT / 'tools/package_forever.py').read_text()).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class RequiredFramingContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads((ROOT / 'contracts/forever-required-framing-contract-v1.json').read_text())
        cls.f = (ROOT / 'crates/forever-casc/src/local_store/framing.rs').read_text()
        cls.io = (ROOT / 'crates/forever-casc/src/local_store/io/framing.rs').read_text()

    def test_profile_and_disabled_product_scope(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-required-framing-v1', 1))
        self.assertEqual(self.c['api_profile'], 'forever-local-required-framing-v1')
        self.assertTrue(self.c['original_implementation'])
        for key in ['network', 'keys_consulted', 'cli_enabled', 'windows_product_enabled', 'actual_windows_ntfs_accepted', 'full_extractor_complete', 'publication_authorized', 'complete_payload_accepted']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])

    def test_v6_mandatory_closure_and_historical_profiles(self):
        d = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())
        self.assertEqual(self.c['distribution_source_profile'], HISTORICAL_PROFILE)
        self.assertEqual(literal('SOURCE_PROFILE'), PROFILE)
        self.assertIn(HISTORICAL_PROFILE, d['source']['prior_profiles'])
        self.assertEqual(d['source']['profile'], PROFILE)
        self.assertEqual(literal('REQUIRED_FRAMING_SOURCE'), tuple(self.c['sources']))
        for path in self.c['sources']:
            self.assertTrue((ROOT / path).is_file(), path)
            self.assertIn(path, d['source']['required'])
        self.assertIn('forever-seven-crate-windows-vmap-diagnostics-source-v5', d['source']['prior_profiles'])
        self.assertIn('forever-seven-crate-windows-producer-source-v4', d['source']['prior_profiles'])
        self.assertEqual(d['contract'], 'forever-distribution-v1')

    def test_unchanged_exact_dependency_graph(self):
        packages = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
        self.assertEqual(len(packages), 9)
        self.assertEqual(len(literal('CRATES')), 7)
        self.assertEqual(next(p for p in packages if p['name'] == 'forever-vmap')['dependencies'], ['forever-platform'])
        self.assertEqual(next(p for p in packages if p['name'] == 'forever-navigation')['dependencies'], ['forever-db2', 'forever-platform', 'forever-terrain', 'forever-vmap'])
        registry = {p['name'] + '-' + p['version']: p['checksum'] for p in packages if 'source' in p}
        self.assertEqual(registry, literal('DEPENDENCIES'))

    def test_closed_three_api_and_key_free_gate(self):
        self.assertEqual(self.c['api']['fixed_fdids'], [1349477, 1266278, 1266277])
        self.assertEqual(self.c['api']['order'], ['Map', 'SkillLineAbility', 'GameObjectDisplayInfo'])
        for text in ['pub fn diagnose_required_framing(', 'routes::guarded(self,', 'if self.keys.is_some()', 'return Err(Error::KeysPresent)', 'pub type Error = routes::Error;']:
            self.assertIn(text, self.f)
        self.assertFalse(self.c['api']['raw_accessor'])
        self.assertFalse(self.c['api']['debug_route_reports'])
        self.assertIn('pub mod framing;', (ROOT / 'crates/forever-casc/src/local_store.rs').read_text())
        self.assertIn('pub(super) mod framing;', (ROOT / 'crates/forever-casc/src/local_store/io.rs').read_text())

    def test_downward_caps_are_per_candidate_for_frames(self):
        limits = self.c['limits']
        self.assertEqual([limits[k] for k in ['root_rows_per_input', 'candidates_per_input', 'candidates_total', 'record_bytes_including_prelude', 'total_indexed_record_bytes', 'declared_content_bytes_per_input', 'frames_per_candidate_blte_object', 'report_bytes']], [64, 64, 192, 131072, 262144, 524288, 4096, 131072])
        self.assertTrue(limits['only_downward'])
        self.assertIn('not aggregate per FDID', limits['frame_scope'])
        self.assertIn('max_frames: 4096', self.f)
        self.assertIn('limits.max_frames > caps.max_frames', self.f)
        self.assertIn('request.store_limits.max_blte_frames', self.io)
        doc = (ROOT / 'docs/forever-required-framing.md').read_text()
        self.assertIn('Frames per candidate BLTE object', doc)
        self.assertIn('not an aggregate per-input limit', doc)

    def test_all_alternatives_prefix_collisions_and_no_root_fallback(self):
        for text in ['selected.is_none() && self.central.contains(row.content_key)', 'self.central.encoded_keys(key).enumerate()', 'owned::equal_range', 'range.len().max(1)', 'total_bytes.checked_add(size)']:
            self.assertIn(text, self.f)
        self.assertIn('every nine-byte index-prefix collision', self.c['selection']['candidates'])
        self.assertIn('archive availability never', self.c['selection']['root'])
        self.assertIn('metadata-only', self.c['selection']['unknown_decoded_size'])

    def test_plan_and_capacity_admission_precedes_payload_read(self):
        self.assertLess(self.f.index('// No file payload is touched'), self.f.index('io::framing::inspect_span'))
        for text in ['plan.capacity()', 'diagnostic.candidates.capacity()', 'report_bytes', 'budget.retain(', 'budget.phase(scratch)']:
            self.assertIn(text, self.f)
        self.assertLess(self.io.index('let plan = match classify('), self.io.index('super::reserve(&mut record, len)'))
        self.assertLess(self.io.index('record.capacity()'), self.io.index('read(plan.offset(), &mut record)'))
        self.assertIn('caller serialization separate', self.c['limits']['memory'])
        self.assertIn('not RSS/all CPU', self.c['limits']['memory'])

    def test_complete_range_and_exact_prelude_prefix_policy(self):
        self.assertEqual(self.c['read']['prelude_bytes_ignored'], 30)
        self.assertEqual(self.c['read']['index_ekey_prefix_bytes'], 9)
        self.assertTrue(self.c['read']['complete_span'])
        self.assertFalse(self.c['read']['archive_eof_required'])
        self.assertTrue(self.c['read']['blte_eof_required'])
        self.assertFalse(self.c['read']['prelude_key_size_checksums_authenticated'])
        self.assertIn('&record[30..]', self.io)
        self.assertIn('request.ekey[..9] != request.entry.key_prefix', self.io)
        self.assertIn('file.read_exact(bytes)', self.io)
        self.assertIn('super::archive_path(', self.io)

    def test_final_guards_on_success_and_all_errors(self):
        self.assertIn('let result = read_and_inspect', self.io)
        self.assertIn('file.verify_unchanged().map_err(super::windows::map_error)?;', self.io)
        self.assertIn('let held = file.metadata()', self.io)
        self.assertIn('identity != super::Identity::from(&named)', self.io)
        for text in ['allocation/budget/read failure', 'outer store guards on success and error', 'reject whole report']:
            self.assertIn(text, self.c['read']['guards'])
        self.assertIn('no anchored ancestor or atomic snapshot', self.c['read']['linux'])

    def test_redacted_inspection_not_partial_plaintext_evidence(self):
        for text in ['NotRead', "Rejected(&'static str)", 'Complete(blte::framing::Evidence)']:
            self.assertIn(text, self.f)
        self.assertIn('No Debug or raw identity accessors', self.f)
        self.assertIn('no digest,key,path,offset,nonce,payload', self.c['inspection']['public_evidence'])
        self.assertIn('including zero hashes and later frames after E', self.c['inspection']['frame_md5'])
        self.assertIn('no inflate', self.c['inspection']['Z'])
        self.assertIn('no decrypt/interior', self.c['inspection']['E'])
        self.assertIn('not equivalent to framed integrity', self.c['inspection']['headerless'])
        self.assertFalse(self.c['read']['full_ekey_authenticated'])

    def test_imported_design_document_is_byte_exact_prelive_snapshot(self):
        self.assertEqual(hashlib.sha256((ROOT / 'docs/forever-required-framing.md').read_bytes()).hexdigest(), 'f5e3b865ea8267271a1187c7803477352ad7a4ece80f24c336fa3f41599df1f4')
        self.assertIn('pre-live', self.c['validation']['imported_design_doc'])
        companion = (ROOT / 'docs/forever-required-framing-acceptance.md').read_text()
        self.assertIn('pre-live implementation snapshot', companion)
        self.assertIn('local_file_incomplete', companion)

    def test_scoped_synthetic_source_acceptance_is_not_runtime_proof(self):
        v = self.c['validation']
        self.assertIn('static consistency only', v['static_tests'])
        self.assertEqual((v['linux_synthetic']['local_store_tests_each_profile'], v['linux_synthetic']['blte_tests_each_profile'], v['linux_synthetic']['adapter_new_tests_each_profile']), (111, 80, 23))
        self.assertEqual((ROOT / 'crates/forever-casc/src/local_store/framing_tests.rs').read_text().count('#[test]'), 23)
        self.assertTrue(v['windows_gnu_cross_clippy']['accepted'])
        self.assertFalse(v['windows_gnu_cross_clippy']['runtime_accepted'])
        self.assertEqual(len(v['source_handoff']['sha256']), 64)

    def test_real_acquisition_current_category_retains_failure(self):
        a = self.c['validation']['real_acquisition']
        self.assertTrue(a['accepted'] and a['terminal'] and a['key_free'])
        self.assertEqual((a['build'], a['locale'], a['inputs_acquired']), (70205, 'esES', 0))
        self.assertEqual(a['candidate_failures_each'], {'missing': 0, 'incomplete': 0, 'encrypted': 1, 'corrupt': 0, 'unsupported': 0})
        self.assertEqual(a['error_each'], 'local_store_no_verified_encoding')
        self.assertIn('not full framing', a['scope'])
        self.assertEqual(len(a['sha256']), 64)

    def test_real_framing_counts_are_qualified_and_sum_exactly(self):
        f = self.c['validation']['real_framing']
        self.assertTrue(f['accepted'] and f['terminal'] and f['key_free'])
        self.assertEqual((f['build'], f['locale'], f['candidates_each'], f['form_each']), (70205, 'esES', 1, 'FramedMd5'))
        self.assertEqual((f['frames_each'], f['zlib_frames_each'], f['encrypted_frames_each']), ([5, 7, 3], [2, 2, 2], [3, 5, 1]))
        for field, total in [('frames_each', 'frames'), ('zlib_frames_each', 'zlib_frames'), ('encrypted_frames_each', 'encrypted_frames'), ('verified_frame_md5_each', 'verified_frame_md5')]:
            self.assertEqual(sum(f[field]), f['totals'][total])
        self.assertEqual(f['totals'], {'frames': 15, 'zlib_frames': 6, 'encrypted_frames': 9, 'verified_frame_md5': 15})
        for field in ['full_ekey_authenticated', 'ckey_authenticated', 'plaintext_validated', 'zlib_interior_validated', 'encrypted_interior_validated']:
            self.assertFalse(f[field])
        for field in ['sha256', 'root_rehash_sha256', 'agent_closeout_sha256']:
            self.assertEqual(len(f[field]), 64)

    def test_history_preserves_failures_and_pending_typed_runtime(self):
        h = self.c['history']
        self.assertIn('earlier named native/batch/locale attempts', h['old_local_file_incomplete'])
        for key in ['typed_loaders_reexecuted', 'full_runtime_accepted', 'v6_bundle_built']:
            self.assertFalse(h[key])
        self.assertIn('remains failed at256MiB', self.c['validation']['historical_failure'])
        for item in ['full EKey or CKey authentication', 'Z interior validity', 'encrypted inner validity', 'plaintext integrity', 'key-only sufficiency', 'typed loader success', 'Windows/NTFS runtime acceptance', 'full extraction', 'publication']:
            self.assertIn(item, self.c['not_claimed'])


if __name__ == '__main__':
    unittest.main()
