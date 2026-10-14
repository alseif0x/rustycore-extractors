# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original static framing/routes source contract checks.
"""Static consistency only. No client access, route execution or runtime proof."""
import ast
import json
import hashlib
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def literal(name):
    for node in ast.parse((ROOT / 'tools/package_forever.py').read_text()).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class InputDiagnosticsContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads((ROOT / 'contracts/forever-input-diagnostics-contract-v1.json').read_text())
        cls.f = (ROOT / 'crates/forever-casc/src/blte/framing.rs').read_text()
        cls.routes = (ROOT / 'crates/forever-casc/src/local_store/routes.rs').read_text()

    def test_closed_diagnostic_scope(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-input-diagnostics-source-v1', 1))
        for key in ['network', 'keys_consulted', 'cli_enabled', 'complete_payload_accepted', 'windows_product_enabled', 'actual_windows_ntfs_accepted', 'full_extractor_complete', 'publication_authorized']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertFalse(self.c['composition']['implemented'])

    def test_mandatory_exact_source_closure(self):
        d = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())
        self.assertEqual(self.c['distribution_source_profile'], 'forever-seven-crate-windows-vmap-diagnostics-source-v5')
        self.assertIn(self.c['distribution_source_profile'], d['source']['prior_profiles'])
        self.assertEqual(literal('SOURCE_PROFILE'), 'forever-seven-crate-linux-product-db2-gt-cameras-source-v11')
        required = set(literal('INPUT_DIAGNOSTICS_SOURCE'))
        for path in self.c['sources']:
            self.assertTrue((ROOT / path).is_file(), path)
            self.assertIn(path, required)
            self.assertIn(path, d['source']['required'])
        self.assertEqual(d['source']['profile'], literal('SOURCE_PROFILE'))

    def test_route_doc_relocation_is_narrow(self):
        relocation = self.c['routes']['doc_relocation']
        self.assertEqual(relocation['to'], 'docs/forever-required-routes.md')
        self.assertNotIn(relocation['from'], literal('INPUT_DIAGNOSTICS_SOURCE'))
        self.assertFalse((ROOT / relocation['from']).exists())
        self.assertTrue(relocation['byte_identical'])
        self.assertIn('Authored 2026-10-11', (ROOT / relocation['to']).read_text())
        self.assertEqual(hashlib.sha256((ROOT / relocation['to']).read_bytes()).hexdigest(), relocation['sha256'])

    def test_fixed_metadata_route_api_and_no_raw_payload(self):
        self.assertEqual(self.c['routes']['fixed_fdids'], [1349477, 1266278, 1266277])
        self.assertEqual((self.c['routes']['max_root_matches_per_input'], self.c['routes']['max_candidates_per_input'], self.c['routes']['max_total_candidates']), (64, 64, 192))
        self.assertIn('forever-local-required-routes-v1', self.routes)
        self.assertIn('pub fn diagnose_required_routes(', self.routes)
        self.assertEqual(self.c['routes']['payload'], 'NotInspected')
        self.assertEqual(self.c['routes']['ciphertext'], 'NotInspected')
        self.assertFalse(self.c['routes']['raw_route_accessor'])
        self.assertIn('NotInspected', self.routes)
        self.assertIn('decimal', self.c['routes']['archive_filename'])

    def test_diagnostic_and_reader_share_decimal_archive_constructor(self):
        io = (ROOT / 'crates/forever-casc/src/local_store/io.rs').read_text()
        route = (ROOT / 'crates/forever-casc/src/local_store/io/route.rs').read_text()
        self.assertIn('pub(super) fn archive_path(', io)
        self.assertIn('archive_path(dir, entry.archive_index, limits.max_path_bytes)', io)
        self.assertIn('super::archive_path(directory, entry.archive_index, limits.max_path_bytes)', route)
        self.assertNotIn(':03x', route)
        self.assertIn('number % 10', io)
        self.assertIn('decimal', (ROOT / 'crates/forever-casc/src/local_store/route_tests.rs').read_text().lower())

    def test_framing_checks_complete_table_extents_eof_and_all_md5(self):
        for text in ['pub fn inspect(', 'header_size != table_size', 'offset != encoded.len()', 'declared_size != expected_content_size', 'md5::digest(frame).as_slice() != &descriptor[8..24]', 'frame_count > limits.max_frames']:
            self.assertIn(text, self.f)
        self.assertTrue(self.c['framing']['all_or_error'])
        self.assertEqual(self.c['framing']['allocations'], 0)
        self.assertIn('pub mod framing;', (ROOT / 'crates/forever-casc/src/blte.rs').read_text())
        self.assertIn('including zero', self.c['framing']['frame_md5'])

    def test_outer_modes_do_not_claim_plaintext_or_inner_validity(self):
        for text in ['name_size != 0 && name_size != 8', 'nonce_size != 4 && nonce_size != 8', "cipher != b'S'", 'ciphertext.is_empty()', "b'Z' => evidence.zlib_frames += 1", 'HeaderlessUnhashed']:
            self.assertIn(text, self.f)
        self.assertIn('no zlib interior', self.c['framing']['Z'])
        self.assertIn('no frame hash', self.c['framing']['headerless'])
        self.assertIn('no key ID/nonce/bytes', self.c['framing']['E'])
        for claim in ['key-only sufficiency', 'complete EKey or CKey authentication', 'plaintext integrity', 'encrypted inner validity']:
            self.assertIn(claim, self.c['not_claimed'])

    def test_corrected_linux_acceptance_has_explicit_scope(self):
        linux = self.c['validation']['routes_corrected_linux']
        self.assertTrue(linux['accepted'])
        self.assertEqual(linux['tests_each_profile'], 88)
        self.assertEqual(len(linux['handoff_sha256']), 64)
        self.assertEqual(len(linux['source_handoff_sha256']), 64)
        self.assertIn('initial hexadecimal handoff explicitly superseded', linux['scope'])
        self.assertFalse(self.c['validation']['windows_route_runtime_accepted'])

    def test_real_metadata_observation_does_not_accept_payload_or_keys(self):
        observation = self.c['validation']['real_metadata_observation']
        self.assertTrue(observation['accepted'])
        self.assertTrue(observation['terminal'])
        self.assertEqual(len(observation['sha256']), 64)
        self.assertIn('NotInspected', observation['scope'])
        for key in ['keys_read', 'requested_archive_payload_read', 'framing_inspected', 'content_accepted']:
            self.assertFalse(observation[key])
        self.assertEqual(observation['encoded_record_bytes'], [5042, 94617, 74592])
        self.assertEqual(observation['declared_decoded_bytes'], [10181, 347118, 116054])
        self.assertFalse(self.c['complete_payload_accepted'])

    def test_future_exact_span_adapter_remains_unimplemented(self):
        plan = self.c['composition']['future']
        for text in ['exact full record span', '30-byte local prelude', 'entire remaining BLTE span', 'success and error', 'no prefix']:
            self.assertIn(text, plan)
        self.assertIn('static consistency only', self.c['validation']['contract_tests'])
        self.assertEqual((self.c['validation']['framing_tests_each_profile'], self.c['validation']['framing_new_tests']), (80, 37))
        self.assertFalse(self.c['complete_payload_accepted'])


if __name__ == '__main__':
    unittest.main()
