# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original loaded-producer source contract checks.
"""Static consistency checks only. Runtime/NTFS/sealing evidence is separate."""
import ast
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


def literal_assignment(source, name):
    for node in ast.parse(source).body:
        if isinstance(node, ast.Assign) and any(isinstance(target, ast.Name) and target.id == name for target in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError('missing literal assignment ' + name)


class WindowsProducerContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-producer-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}

    def test_separate_profile_retains_disabled_product_and_original_dependency_scope(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-producer-seal-v1', 1))
        self.assertEqual(self.c['api_profile'], self.c['contract'])
        foundation = json.loads(read('contracts/forever-windows-read-contract-v1.json'))
        self.assertEqual(self.c['foundation_contract'], foundation['contract'])
        self.assertEqual(self.c['foundation_profile'], foundation['profile'])
        self.assertEqual(self.c['read_api_profile'], foundation['api_profile'])
        for key in ['network', 'client_data_in_git', 'windows_product_enabled', 'full_extractor_complete', 'publication_authorized', 'upstream_implementation_imported']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertFalse(self.c['apis']['loaded_helper_product_dispatch'])

    def test_versioned_distribution_requires_complete_producer_source(self):
        packager = read('tools/package_forever.py')
        distribution = json.loads(read('contracts/forever-distribution-contract-v1.json'))
        expected = 'forever-seven-crate-windows-producer-source-v4'
        self.assertEqual(self.c['distribution_source_profile'], expected)
        current = 'forever-seven-crate-linux-collision-leased-archive-source-v8'
        self.assertEqual(literal_assignment(packager, 'SOURCE_PROFILE'), current)
        self.assertEqual(distribution['source']['profile'], current)
        self.assertIn(expected, distribution['source']['prior_profiles'])
        self.assertIn('forever-seven-crate-windows-vmap-required-framing-source-v6', distribution['source']['prior_profiles'])
        required = set(literal_assignment(packager, 'WINDOWS_PRODUCER_SOURCE')) | set(literal_assignment(packager, 'DOCS'))
        for path in self.c['sources'].values():
            self.assertIn(path, required)
            self.assertIn(path, distribution['source']['required'])
        self.assertIn('contracts/forever-windows-producer-contract-v1.json', required)
        self.assertEqual(distribution['source']['workspace_crates'], 7)
        self.assertEqual(len(literal_assignment(packager, 'CRATES')), 7)
        self.assertEqual(len(distribution['registry_dependencies']), 2)
        self.assertIn('len(packages) == 9', packager)
        self.assertIn('forever-seven-crate-windows-input-source-v3', distribution['source']['prior_profiles'])

    def test_platform_exports_safe_acquisition_api_and_read_scope(self):
        source = self.s['platform']
        self.assertIn('pub mod producer;', read('crates/forever-platform/src/windows.rs'))
        self.assertIn('pub struct LoadedProducer', source)
        for api in ['pub fn capture(', 'pub fn payload(', 'pub fn payload_rva(', 'pub fn read_all(', 'pub fn verify_unchanged(']:
            self.assertIn(api, source)
        self.assertIn('pub const WINDOWS_PRODUCER_PROFILE: &str = "forever-windows-producer-seal-v1"', source)
        self.assertIn('ReadFile::open(&path, limits, LinkPolicy::Allow)', source)
        self.assertIn('self.source.verify_unchanged()', source)
        self.assertIn('self.invalid.set(true)', source)
        self.assertFalse(self.c['loaded']['raw_handles_exposed'])
        self.assertIn('LinkPolicy::Allow', self.c['loaded']['candidate'])

    def test_live_seal_is_volatile_from_actual_main_module_and_measured_rva(self):
        ffi = self.s['platform_ffi']
        self.assertIn('read_volatile', ffi)
        self.assertIn('GetModuleHandleW', ffi)
        self.assertIn('GetModuleHandleExW', ffi)
        self.assertIn('GetModuleFileNameW', ffi)
        self.assertIn('checked_sub', ffi)
        self.assertIn('payload_rva', ffi)
        self.assertIn('running module', self.c['loaded']['payload_source'])
        self.assertIn('payload START RVA', self.c['loaded']['measurement'])
        self.assertNotIn('std::env::current_exe', self.s['platform'])
        self.assertNotIn('std::env::current_exe', ffi)

    def test_unsealed_capture_precedes_path_and_file_access(self):
        source = self.s['platform']
        snapshot = source.index('let snapshot = ffi::snapshot()?;')
        path = source.index('let path = ffi::module_path(limits)?;')
        file = source.index('ReadFile::open(&path, limits, LinkPolicy::Allow)')
        self.assertLess(snapshot, path)
        self.assertLess(path, file)
        self.assertIn('Error::Unsealed', self.s['platform_ffi'])
        self.assertIn('before module pathname query or filesystem acquisition', self.c['loaded']['unsealed'])

    def test_limits_match_complete_guarded_chunked_acquisition(self):
        source = self.s['platform']
        limits = self.c['limits']
        self.assertEqual(limits['candidate_file_bytes'], 256 * 1024 * 1024)
        self.assertEqual(limits['primitive_read_bytes'], 64 * 1024 * 1024)
        self.assertEqual(limits['chunks'], 4096)
        self.assertIn('MAX_PRODUCER_BYTES: usize = 256 * 1024 * 1024', source)
        self.assertIn('MAX_PRODUCER_CHUNKS: usize = 4096', source)
        self.assertIn('chunks_mut(self.limits.max_read_bytes)', source)
        self.assertIn('len.div_ceil(limits.max_read_bytes)', source)
        self.assertIn('try_reserve_exact(len)', source)
        self.assertIn('MAX_FILE_BYTES: usize = 256 * 1024 * 1024', self.s['consumer'])
        self.assertTrue(limits['limits_only_reduce'])
        self.assertIn('not allocator, kernel memory or RSS', limits['scope'])

    def test_payload_marker_and_measured_payload_start_are_explicit(self):
        seal = self.c['seal']
        self.assertEqual((seal['payload_bytes'], seal['marker_bytes'], seal['digest_bytes']), (64, 32, 32))
        self.assertEqual(len(seal['marker_ascii'].encode()) + seal['marker_nul_padding_bytes'], 32)
        for name in ['platform', 'consumer']:
            self.assertIn(seal['marker_ascii'], self.s[name])
        consumer = self.s['consumer']
        self.assertIn('pub struct Layout', consumer)
        self.assertIn('pub digest_offset: usize', consumer)
        self.assertIn('pub payload_rva: u32', consumer)
        self.assertIn('measured_payload_rva == layout.payload_rva', consumer)
        self.assertIn('payload_rva: seal.rva as u32', consumer)
        self.assertIn('digest_offset: seal.raw + 32', consumer)

    def test_strict_unsigned_console_pe_profile_has_no_certificate_fallback(self):
        pe = self.c['pe']
        self.assertEqual((pe['machine'], pe['format'], pe['subsystem_number']), (0x8664, 'PE32+', 3))
        self.assertEqual((pe['optional_header_bytes'], pe['directory_count'], pe['sections_max']), (240, 16, 96))
        self.assertEqual(pe['checksum'], 0)
        self.assertEqual(pe['seal_section_flags'], 0x40000040)
        self.assertEqual(pe['unsupported_directory_indexes'], [4, 7, 8, 10, 11, 13, 14, 15])
        for code in ['pe_machine', 'pe_magic', 'pe_console_only', 'pe_checksum_nonzero', 'pe_seal_count', 'pe_seal_flags', 'pe_seal_padding', 'pe_unsupported_directory', 'pe_raw_overlap', 'pe_virtual_overlap']:
            self.assertIn('"' + code + '"', self.s['consumer'])
        self.assertFalse(pe['general_pe_compatibility'])
        self.assertIn('certificate/security directory is rejected', pe['unsigned'])

    def test_loader_write_destinations_are_excluded_with_explicit_work_caps(self):
        consumer = self.s['consumer']
        for marker in ['fn not_seal(', 'fn relocations(', 'fn imports(', 'fn tls(', 'pe_loader_writes_seal', 'pe_relocation_limit', 'pe_import_limit', 'pe_tls_callbacks_limit']:
            self.assertIn(marker, consumer)
        limits = self.c['limits']
        self.assertEqual(limits['relocation_directory_bytes'], 1024 * 1024)
        self.assertEqual(limits['import_directory_bytes'], 1024 * 1024)
        self.assertEqual(limits['relocation_entries'], 65_536)
        self.assertEqual(limits['import_thunks'], 65_536)
        self.assertEqual(limits['tls_callbacks'], 1024)
        self.assertIn('MAX_WORK: usize = 65_536', consumer)
        self.assertIn('0..1024_u64', consumer)
        self.assertIn('pe_loader_control_write', consumer)
        self.assertIn('declared relocation or import directory', self.c['pe']['loader_control_writes'])
        self.assertIn('declared static', self.c['pe']['loader_writes'])

    def test_canonical_hash_zeroes_only_digest_and_returns_complete_hash(self):
        consumer = self.s['consumer']
        self.assertIn('let zero = [0; 32]', consumer)
        self.assertIn('.chain(zero.as_slice())', consumer)
        self.assertIn('.chain(&data[layout.digest_offset + 32..])', consumer)
        self.assertIn('stored == &loaded_payload[32..]', consumer)
        self.assertIn('canonical_digest(data, layout)?.as_slice() == stored', consumer)
        self.assertIn('Ok(integrity::digest(data))', consumer)
        self.assertFalse(self.c['seal']['authentication'])
        self.assertIn('exactly the 32-byte digest slot', self.c['seal']['canonical_digest'])

    def test_extractor_stays_unsafe_forbidden_and_main_dispatch_disabled(self):
        self.assertIn('unsafe_code = "forbid"', read('Cargo.toml'))
        self.assertNotRegex(self.s['consumer'], r'\bunsafe\s*\{')
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', self.s['platform'])
        main = read('crates/forever-extractor/src/main.rs')
        self.assertIn('#[cfg(not(target_os = "linux"))]', main)
        self.assertIn('product_platform', main)
        self.assertNotIn('producer::loaded_identity(', main)
        self.assertIn('pub mod producer;', read('crates/forever-extractor/src/lib.rs'))

    def test_loaded_helper_retains_final_verification_even_after_pe_failure(self):
        consumer = self.s['consumer']
        helper = consumer[consumer.index('pub fn loaded_identity_evidence()'):]
        operations = ['LoadedProducer::capture(', '.read_all(MAX_FILE_BYTES)',
                      'let identity = verify_pe(', 'source.verify_unchanged()',
                      'identity.map(']
        positions = [helper.index(operation) for operation in operations]
        self.assertEqual(positions, sorted(positions))
        self.assertIn('pub struct VerifiedProducer', consumer)
        self.assertIn('pub const fn file_sha256(', consumer)
        self.assertIn('pub const fn payload_rva(', consumer)
        self.assertIn('loaded_identity_evidence().map(|identity| identity.file_sha256())', consumer)
        self.assertIn('#[cfg(windows)]\npub fn loaded_identity_evidence()', consumer)

    def test_offline_sealer_supplied_payload_is_separate_from_loaded_evidence(self):
        sealer = self.s['sealer']
        for command in ['--input', '--output', '--zero-checksum', '--loaded-payload', '--payload-rva']:
            self.assertIn(command, sealer)
        self.assertFalse(self.c['sealer']['network'])
        self.assertFalse(self.c['sealer']['downloads'])
        self.assertFalse(self.c['sealer']['publishing'])
        self.assertIn('artifact integrity only', self.c['sealer']['supplied_seal_scope'])
        self.assertIn('no in-place rewrite', self.c['sealer']['output'])
        self.assertIn('no pathname unlink cleanup', self.c['sealer']['failure'])
        self.assertIn('no parent-directory durability', self.c['sealer']['durability'])
        self.assertNotIn('os.unlink(', sealer)
        self.assertNotIn('.unlink(', sealer)
        self.assertIn('os.fsync(stream.fileno())', sealer)
        self.assertIn('memoryview(', sealer)
        self.assertIn('pe_loader_control_write', sealer)
        self.assertIn('seal', self.s['sealer_tests'])
        self.assertIn('windows_producer_identity', self.c['sources']['example'])

    def test_historical_prototype_does_not_accept_production_or_windows(self):
        validation = self.c['validation']
        self.assertIn('static source/contract consistency only', validation['contract_tests'])
        self.assertIn('29 external prototype cases', validation['historical_prototype']['scope'])
        wine = validation['preliminary_wine_execution']
        self.assertTrue(wine['accepted'])
        self.assertTrue(wine['terminal'])
        self.assertEqual((wine['rust_tests_each_profile'], wine['python_tests'], wine['helper_cases']), (37, 41, 6))
        self.assertEqual((wine['debug_measured_payload_rva'], wine['release_measured_payload_rva']), (5009408, 2412544))
        self.assertIn('Wine only', wine['scope'])
        self.assertIn('denied attempts may be absent', wine['filtered_trace_scope'])
        self.assertEqual(len(wine['closeout_sha256']), 64)
        self.assertEqual(len(wine['runtime_sha256']), 64)
        self.assertNotIn('production_execution_accepted', validation)
        self.assertFalse(validation['actual_windows_ntfs_accepted'])
        self.assertFalse(validation['hosted_ci_executed'])
        for item in ['publisher authentication', 'actual Windows/NTFS acceptance', 'enabled Windows product or distribution', 'full extractor completion']:
            self.assertIn(item, self.c['not_claimed'])


if __name__ == '__main__':
    unittest.main()
