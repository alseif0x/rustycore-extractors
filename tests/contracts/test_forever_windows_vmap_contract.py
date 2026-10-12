# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original static Windows VMap source consistency checks.
"""Static source contract checks only; not runtime or actual NTFS proof."""
import ast
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


def literal(name):
    for node in ast.parse((ROOT / 'tools/package_forever.py').read_text()).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class WindowsVMapContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads((ROOT / 'contracts/forever-windows-vmap-contract-v1.json').read_text())
        cls.source = (ROOT / 'crates/forever-vmap/src/store/windows.rs').read_text()

    def test_separate_source_scope_keeps_product_gates_closed(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-vmap-source-v1', 1))
        self.assertTrue(self.c['original_implementation'])
        for key in ['network', 'windows_product_enabled', 'actual_windows_ntfs_accepted', 'full_extractor_complete', 'publication_authorized']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])

    def test_v5_mandatory_closure_and_historical_v4(self):
        d = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())
        self.assertEqual(self.c['distribution_source_profile'], 'forever-seven-crate-windows-vmap-diagnostics-source-v5')
        self.assertIn(self.c['distribution_source_profile'], d['source']['prior_profiles'])
        self.assertEqual(literal('SOURCE_PROFILE'), 'forever-seven-crate-linux-product-collision-disk-source-v9')
        self.assertEqual(d['source']['profile'], literal('SOURCE_PROFILE'))
        self.assertIn('forever-seven-crate-windows-producer-source-v4', d['source']['prior_profiles'])
        required = set(literal('WINDOWS_VMAP_SOURCE')) | set(literal('WINDOWS_INPUT_SOURCE'))
        for path in self.c['sources']:
            self.assertTrue((ROOT / path).is_file(), path)
            self.assertIn(path, required)
            self.assertIn(path, d['source']['required'])

    def test_exact_local_edge_and_registry_closure(self):
        manifest = tomllib.loads((ROOT / 'crates/forever-vmap/Cargo.toml').read_text())
        self.assertEqual(manifest['target']['cfg(windows)']['dependencies'], {'forever-platform': {'path': '../forever-platform'}})
        packages = tomllib.loads((ROOT / 'Cargo.lock').read_text())['package']
        self.assertEqual(len(packages), 9)
        self.assertEqual(next(p for p in packages if p['name'] == 'forever-vmap')['dependencies'], ['forever-platform'])
        registry = {p['name']: p['version'] for p in packages if 'source' in p}
        self.assertEqual(registry, self.c['dependency_graph']['registry_packages'])
        self.assertEqual(len(literal('CRATES')), 7)

    def test_exports_safe_eof_and_windows_source(self):
        self.assertIn('#[cfg(windows)]\npub mod windows;', (ROOT / 'crates/forever-vmap/src/store.rs').read_text())
        for api in ['pub struct RootedSource', 'pub struct FsLimits', 'pub struct FsStats', 'pub fn new(']:
            self.assertIn(api, self.source)
        platform = (ROOT / 'crates/forever-platform/src/windows.rs').read_text()
        body = platform.split('pub fn verify_eof(&self)', 1)[1].split('fn check_eof_result', 1)[0]
        self.assertEqual(body.count('self.verify_unchanged()?'), 2)
        self.assertIn('seek_read(&mut extra, self.leaf.identity.size)', body)
        self.assertIn('self.check_eof_result(result)', body)
        self.assertIn('.verify_eof()', self.source)

    def test_bounds_and_counter_scope_match_source(self):
        limits = self.c['limits']
        self.assertEqual((limits['chunk_bytes'], limits['lease_allowance_bytes']), (2 * 1024 * 1024, 256 * 1024))
        for text in ['CHUNK_BYTES: usize = 2 * 1024 * 1024', 'LEASE_BYTES: usize = 256 * 1024', 'MAX_METADATA_BYTES: usize = 128 * 1024 * 1024', 'MAX_ENTRIES: usize = 65_536', 'MAX_ROOT_BYTES: usize = 4096', 'MAX_VERIFY_STEPS: usize = 262_144']:
            self.assertIn(text, self.source)
        self.assertEqual(limits['peak_handles'], 65)
        self.assertEqual(self.c['policy']['resident_handles_between_operations'], 0)
        self.assertIn('failed kernel partial reads', self.c['policy']['read_bytes'])
        self.assertIn('not exact allocator', limits['scope'])

    def test_metadata_snapshot_claim_stays_bounded(self):
        self.assertIn('not an atomic snapshot or authentication', self.c['policy']['metadata_verification'])
        self.assertIn('self.failed = true', self.source)
        self.assertIn('no handles survive an operation', self.source.lower())
        self.assertIn('exact_artifact_spelling', self.c['policy'])

    def test_preliminary_evidence_is_not_ntfs_acceptance(self):
        wine = self.c['validation']['preliminary_wine']
        self.assertTrue(wine['accepted'])
        self.assertTrue(wine['terminal'])
        self.assertEqual(wine['tests_each_profile'], wine['vmap_tests'] + wine['eof_tests'] + wine['existing_read_all_tests'])
        self.assertEqual(wine['tests_each_profile'], 28)
        self.assertEqual(self.c['validation']['linux_vmap_tests_each_profile'], 477)
        self.assertEqual(len(wine['closeout_sha256']), 64)
        self.assertIn('Wine only', wine['scope'])
        self.assertIn('denied attempts may be absent', self.c['validation']['trace_limits'])
        self.assertIn('static source consistency only', self.c['validation']['contract_tests'])


if __name__ == '__main__':
    unittest.main()
