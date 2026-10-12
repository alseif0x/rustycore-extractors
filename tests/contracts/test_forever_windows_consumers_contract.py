# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: source contract for guarded Windows read consumers.
"""Static contracts only; actual Windows and Wine execution are separate evidence."""
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(relative):
    return (ROOT / relative).read_text(encoding='utf-8')


class WindowsConsumersContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-consumers-contract-v1.json'))
        cls.sources = {key: read(path) for key, path in cls.c['sources'].items() if isinstance(path, str)}

    def test_profile_identity_and_incomplete_product_are_explicit(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-consumers-v1', 1))
        foundation = json.loads(read('contracts/forever-windows-read-contract-v1.json'))
        self.assertEqual(self.c['foundation_contract'], foundation['contract'])
        self.assertEqual(self.c['foundation_profile'], foundation['profile'])
        self.assertEqual(self.c['api_profile'], foundation['api_profile'])
        self.assertFalse(self.c['full_extractor_complete'])
        self.assertFalse(self.c['publication_authorized'])
        self.assertFalse(self.c['network'])
        self.assertFalse(self.c['client_data_in_git'])
        self.assertIn('read consumers integrated', self.c['status'])

    def test_windows_routes_actual_store_io_without_changing_nonwindows_branch(self):
        source = self.sources['store_io']
        self.assertIn('#[cfg(windows)]\nmod windows;', source)
        self.assertIn('#[cfg(windows)]\npub(super) use windows::{Guard, whole};', source)
        self.assertIn('#[cfg(not(windows))]\npub(super) fn whole(', source)
        self.assertIn('windows::read_range(&file, plan.offset(), &mut encoded)?;', source)
        self.assertIn('file.verify_unchanged().map_err(windows::map_error)?;', source)
        self.assertIn('archive_record::plan(entry, archive_len, limits.max_archive_span_bytes)', source)

    def test_archive_allows_explicit_hardlinks_but_input_rejects(self):
        self.assertIn('ReadFile::open(path, Limits::default(), LinkPolicy::Allow)', self.sources['store_windows'])
        self.assertIn('ReadFile::open(path, Limits::default(), LinkPolicy::Reject)', self.sources['input_windows'])
        self.assertEqual(self.c['input']['links'], 'LinkPolicy::Reject for both explicit input APIs')
        self.assertIn('no reparse', self.c['casc']['links'])

    def test_guard_captures_type_and_full_snapshot_without_retained_handles(self):
        source = self.sources['store_windows']
        body = re.search(r'pub\(in crate::local_store\) struct Guard \{(.*?)\n\}', source, re.S).group(1)
        self.assertIn('path: PathBuf', body)
        self.assertIn('identity: Identity', body)
        self.assertIn('kind: Kind', body)
        self.assertIn('expected: Option<Vec<u8>>', body)
        self.assertNotIn('ReadFile', body)
        self.assertNotIn('ReadDirectory', body)
        self.assertIn('drop(file);', source)
        self.assertIn('drop(directory);', source)
        self.assertIn('if directory.identity() != self.identity', source)
        self.assertIn('if file.identity() != self.identity', source)
        self.assertNotIn('verify_same_object', source)
        self.assertIn('expected.chunks(chunk.len())', source)
        self.assertIn('let mut chunk = [0; 4096]', source)
        self.assertIn('no persistent file handles', self.c['casc']['guard']['retained'])

    def test_enumeration_lease_is_dropped_before_file_acquisition(self):
        source = self.sources['store_io']
        lease = source.index('let directory = windows::directory(dir)?;')
        enumeration = source.index('for entry in fs::read_dir(dir)', lease)
        verify = source.index('directory.verify_unchanged()', enumeration)
        drop = source.index('drop(directory)', verify)
        acquisition = source.index('let guard = Guard::capture(path)?;', drop)
        self.assertLess(lease, enumeration)
        self.assertLess(enumeration, verify)
        self.assertLess(verify, drop)
        self.assertLess(drop, acquisition)

    def test_complete_acquisition_keeps_existing_limits_with_chunking(self):
        source = self.sources['store_windows']
        self.assertIn('if len > max_bytes', source)
        self.assertIn('budget.phase(size)?;', source)
        self.assertIn('reserve(&mut bytes, size)?;', source)
        self.assertIn('bytes.chunks_mut(MAX_READ_BYTES)', source)
        self.assertIn('.checked_add((n * MAX_READ_BYTES) as u64)', source)
        self.assertIn('file.read_exact_at(at, chunk)', source)
        range_body = source[source.index('pub(in crate::local_store) fn read_range'):]
        self.assertGreaterEqual(range_body.count('file.verify_unchanged()'), 2)
        limits = self.c['casc']['limits']
        self.assertEqual(limits['primitive_chunk_bytes'], 64 * 1024 * 1024)
        for key in ['encoding_bytes', 'root_bytes', 'archive_span_bytes']:
            self.assertEqual(limits[key], 256 * 1024 * 1024)
        self.assertFalse(limits['existing_limits_enlarged'])
        self.assertIn('max_archive_span_bytes: 256 * 1024 * 1024', self.sources['store'])
        self.assertIn('max_decoded_file_bytes: 64 * 1024 * 1024', self.sources['store'])

    def test_windows_lease_allowance_is_charged_once(self):
        source = self.sources['store_io']
        reservation = re.search(r'const fn windows_lease_reservation\(\) -> usize \{(.*?)\n\}', source, re.S).group(1)
        self.assertIn('256 * 1024', reservation)
        self.assertEqual(self.c['casc']['limits']['windows_resident_allowance_bytes'], 256 * 1024)
        self.assertEqual(self.sources['store'].count('budget.retain(io::windows_lease_reservation())?;'), 1)
        self.assertIn('not measured heap, RSS', self.c['casc']['limits']['allowance_scope'])

    def test_missing_captured_input_is_change_and_session_poison_is_permanent(self):
        source = self.sources['store_windows']
        self.assertIn('directory(&self.path).map_err(|_| Error::InputChanged)', source)
        self.assertIn('archive(&self.path).map_err(|_| Error::InputChanged)', source)
        self.assertIn('Err(forever_platform::Error::Missing) => return Ok(Err(Rejection::Missing))', self.sources['store_io'])
        self.assertIn('Guard and LocalStore do not themselves promise permanent poison', self.c['casc']['guard']['poison'])
        session = self.sources['session']
        self.assertIn('if self.changed {', session)
        self.assertIn('error.is_input_changed()', session)
        self.assertIn('self.changed = true;', session)
        self.assertNotIn('self.changed = false;', session)

    def test_admission_reads_complete_build_info_through_guarded_windows_api(self):
        source = self.sources['admission']
        self.assertIn('#[cfg(windows)]', source)
        self.assertIn('ReadFile::open(', source)
        self.assertIn('LinkPolicy::Allow', source)
        self.assertIn('file.read_all(MAX_BUILD_INFO as usize)', source)
        self.assertIn('const MAX_BUILD_INFO: u64 = 256 * 1024;', source)
        self.assertEqual(self.c['casc']['build_info_bytes'], 256 * 1024)
        self.assertEqual(self.c['casc']['builds'], [70170, 70205])
        self.assertIn('native_shape(info)?;', source)

    def test_input_keys_keep_opaque_retained_lease_and_existing_key_limit(self):
        source = self.sources['input_windows']
        self.assertIn('.read_all(forever_casc::keys::MAX_BYTES)', source)
        self.assertIn('forever_casc::keys::Keyring::parse(&bytes)', source)
        self.assertIn('_source_lease: lease', source)
        self.assertIn('_source_lease: forever_platform::windows::ReadFile', self.sources['input'])
        self.assertIn('pub fn into_keyring(self)', self.sources['input'])
        self.assertEqual(self.c['input']['key_bytes'], 256 * 1024)
        self.assertIn('pub const MAX_BYTES: usize = 256 * 1024;', read('crates/forever-casc/src/keys.rs'))
        self.assertTrue(self.c['input']['keys_not_discovered'])
        self.assertFalse(self.c['input']['secret_output'])
        self.assertFalse(self.c['input']['secure_erasure_guaranteed'])

    def test_complete_input_bytes_chunk_and_recheck_without_canonicalization(self):
        source = self.sources['input_windows']
        self.assertIn('if length > limit', source)
        self.assertIn('bytes.chunks_mut(MAX_READ_BYTES)', source)
        self.assertIn('.checked_mul(MAX_READ_BYTES)', source)
        self.assertIn('.read_exact_at(offset as u64, chunk)', source)
        self.assertIn('lease.verify_unchanged()', source)
        self.assertNotIn('std::fs::canonicalize(', source)
        self.assertIn('total may exceed', self.c['input']['bytes'])
        self.assertFalse(self.c['input']['loaded_executable_identity'])

    def test_unsupported_windows_outputs_refuse_before_io(self):
        for relative in self.c['sources']['output_guards']:
            source = read(relative)
            functions = re.findall(r'#\[cfg\(not\(unix\)\)\]\s*pub\(super\) fn (\w+)\([^\{]+\{([^}]+)\}', source)
            expected = {'create', 'publish'}
            if '/gametables/' in relative:
                expected |= {'create_catalog', 'complete'}
            if '/cameras/' in relative:
                expected |= {'complete'}
            self.assertEqual({name for name, _ in functions}, expected, relative)
            for name, body in functions:
                self.assertEqual(body.strip(), 'Err("output_platform")', (relative, name))
        self.assertFalse(self.c['outputs']['product_driver_enabled'])
        self.assertFalse(self.c['outputs']['private_dacl_backend'])
        self.assertFalse(self.c['outputs']['durable_publication_backend'])
        product = read('crates/forever-extractor/src/main.rs')
        self.assertIn('#[cfg(not(target_os = "linux"))]', product)
        self.assertIn('Err("product_platform".into())', product)

    def test_scope_excludes_real_windows_and_publication_acceptance(self):
        self.assertFalse(self.c['validation']['actual_windows_ntfs_accepted'])
        self.assertFalse(self.c['validation']['hosted_ci_executed'])
        self.assertIn('static source/contract consistency', self.c['validation']['contract_tests'])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertFalse(self.c['upstream_implementation_imported'])
        for crate in ['forever-casc', 'forever-extractor']:
            manifest = read(f'crates/{crate}/Cargo.toml')
            scope = 'cfg(windows)' if crate == 'forever-casc' else 'cfg(any(windows, target_os = "linux"))'
            self.assertIn("[target.'" + scope + "'.dependencies]\nforever-platform = { path = \"../forever-platform\" }", manifest)
        for item in ['complete Windows extraction', 'actual Windows/NTFS acceptance', 'loaded executable identity',
                     'alias-safe output/key-source containment', 'full extractor completion']:
            self.assertIn(item, self.c['not_claimed'])


if __name__ == '__main__':
    unittest.main()
