# SPDX-License-Identifier: GPL-3.0-or-later
"""V2 source/profile consistency only; native acceptance is separate evidence."""
import ast
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


class WindowsMsvcProducerContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-producer-msvc-contract-v2.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}

    def test_versioned_opt_in_preserves_v1_and_scope(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-producer-seal-msvc-v2', 2))
        self.assertTrue(self.c['prior_profile_unchanged'])
        prior = json.loads(read('contracts/forever-windows-producer-contract-v1.json'))
        self.assertEqual(self.c['prior_contract'], prior['contract'])
        self.assertIn(10, prior['pe']['unsupported_directory_indexes'])
        self.assertEqual(prior['seal']['marker_ascii'], 'RCE-FOREVER-PE-SEAL-V1')
        for flag in ('upstream_implementation_imported', 'network_api', 'client_data_in_git',
                     'publication_authorized', 'full_extractor_complete'):
            self.assertFalse(self.c[flag])
        self.assertEqual(self.c['new_registry_dependencies'], [])

    def test_features_are_explicit_and_default_off(self):
        product = tomllib.loads(read('crates/forever-extractor/Cargo.toml'))
        platform = tomllib.loads(read('crates/forever-platform/Cargo.toml'))
        self.assertEqual(product['features']['default'], [])
        self.assertEqual(product['features'][self.c['extractor_feature']],
                         ['forever-platform/' + self.c['platform_feature']])
        self.assertEqual(platform['features'][self.c['platform_feature']], [])
        self.assertEqual(platform['features'].get('default', []), [])
        self.assertEqual(self.c['target_environment'], 'msvc')
        self.assertIn('target_env = "msvc"', self.s['platform'])
        self.assertIn('feature = "producer-msvc-v2"', self.s['platform'])

    def test_runtime_machine_gate_is_independent_of_file_metadata(self):
        gate = self.c['runtime']
        self.assertEqual((gate['api'], gate['process_machine'], gate['native_machine']),
                         ('IsWow64Process2', 0, 0x8664))
        for key in ('success_required', 'checked_before_path_or_file_access', 'rechecked_with_loaded_snapshot'):
            self.assertTrue(gate[key])
        self.assertFalse(gate['arm64_emulation_accepted'])
        ffi = self.s['platform_ffi']
        for name in ('IsWow64Process2', 'GetCurrentProcess', 'read_volatile'):
            self.assertIn(name, ffi)
        self.assertNotIn('std::env::current_exe', ffi)

    def test_profile_and_marker_are_distinct_and_exact(self):
        seal = self.c['seal']
        self.assertEqual(seal['marker_ascii'], 'RCE-FOREVER-PE-SEAL-V2')
        self.assertEqual(len(seal['marker_ascii'].encode()) + seal['marker_nul_padding_bytes'], 32)
        self.assertEqual((seal['payload_bytes'], seal['digest_bytes']), (64, 32))
        for key in ('platform', 'consumer', 'sealer'):
            self.assertIn(seal['marker_ascii'], self.s[key])
        for name in ('locate_with_profile', 'verify_pe_with_profile', 'MsvcV2'):
            self.assertIn(name, self.s['consumer'])
        self.assertIn('--profile', self.s['sealer'])
        self.assertIn(self.c['contract'], self.s['sealer'])
        self.assertFalse(seal['profile_fallback'])

    def test_closed_config_and_empirical_volatile_shape(self):
        self.assertEqual(self.c['pe']['dll_characteristics'], 0x8160)
        self.assertEqual(self.c['pe']['closed_section_roles']['cookie'],
                         {'name': '.data', 'characteristics': 0xc0000040})
        config = self.c['load_config']
        self.assertEqual((config['directory_bytes'], config['declared_size'], config['guard_flags']), (320, 320, 0x100))
        self.assertEqual(config['va_field_offsets'], [88, 112, 120, 256, 280, 288, 296, 304, 312])
        self.assertEqual(len(config['guard_slot_va_field_offsets']), 7)
        self.assertEqual((config['nonzero_guard_slots'], config['castguard_initial_value']), (6, 0))
        self.assertEqual(config['conservative_cookie_write_bytes'], 8)
        self.assertEqual(config['conservative_guard_slot_write_bytes'], 8)
        volatile = self.c['volatile_metadata']
        self.assertEqual((volatile['header_bytes'], volatile['access_bytes'], volatile['range_bytes']), (24, 40, 24))
        self.assertEqual(volatile['complete_contiguous_object_bytes'], 88)
        self.assertEqual(volatile['signature_word_hex'], '80038003')
        self.assertFalse(volatile['signature_is_interpreted_as_version'])
        self.assertFalse(volatile['arm64_consumer_or_emulation_acceptance'])
        self.assertIn('not an official general ABI', volatile['scope'])

    def test_parser_resource_ceilings_include_tls_terminator(self):
        limits = self.c['limits']
        self.assertEqual(limits['candidate_and_image_bytes'], 256 * 1024 * 1024)
        self.assertEqual(limits['relocation_entries'], 65536)
        self.assertEqual(limits['import_thunks'], 65536)
        self.assertEqual(limits['tls_callback_slots_including_terminator'], 1024)
        self.assertEqual(limits['tls_nonzero_callbacks'], 1023)
        self.assertEqual(limits['protected_control_spans'], 65568)
        self.assertEqual(limits['typed_fixup_entries'], 1043)
        self.assertEqual(limits['rust_vector_reservation_chunk_entries'], 256)
        self.assertFalse(limits['recursive_parser_stack'])
        self.assertFalse(limits['rss_or_all_cpu_work_bound'])

    def test_control_guards_and_hash_scope_do_not_claim_loader_safety(self):
        controls = self.c['loader_controls']
        for key in ('partial_or_duplicate_control_fixups_rejected',
                    'iat_and_tls_index_cannot_overwrite_controls',
                    'known_cookie_and_slot_writes_cannot_overlap_controls'):
            self.assertTrue(controls[key])
        self.assertIn('TLS', controls['typed_aslr_exceptions'])
        self.assertFalse(controls['complete_windows_loader_or_crt_write_enumeration'])
        self.assertFalse(controls['arbitrary_startup_behavior_validation'])
        seal = self.c['seal']
        self.assertIn('entire file', seal['canonical_hash'])
        self.assertIn('only the 32-byte digest slot', seal['canonical_hash'])
        self.assertTrue(seal['actual_loaded_payload_and_start_rva_required'])
        self.assertFalse(seal['publisher_authentication'])
        self.assertFalse(seal['loaded_code_page_measurement'])

    def test_native_build_and_sealer_profile_are_explicit(self):
        script = self.s['native_script']
        builds = [line for line in script.splitlines() if line.startswith('cargo build ')]
        self.assertEqual(len(builds), 4)
        for build in builds:
            self.assertIn('--features ' + self.c['extractor_feature'], build)
            self.assertIn('--offline --locked', build)
        sealing = [line for line in script.splitlines() if 'python tools/seal_forever_windows.py seal ' in line]
        self.assertEqual(len(sealing), 2)
        for command in sealing:
            self.assertIn('--profile ' + self.c['contract'], command)
            self.assertIn('--zero-checksum', command)
        product = json.loads(read('contracts/forever-windows-product-contract-v1.json'))
        self.assertEqual(product['producer_profile'], self.c['prior_contract'])
        self.assertEqual(product['msvc_feature_producer_profile'], self.c['contract'])
        self.assertEqual(product['msvc_feature'], self.c['extractor_feature'])
        self.assertEqual(product['validation']['msvc_producer_native_test_names'],
                         ['msvc_v2_native_machine_query_admits_actual_amd64_test_process'])
        self.assertIn('foreach ($release in @($false, $true))', script)
        self.assertIn("@('') $release 'producer-msvc-v2'", script)
        workflow = json.loads(read('.github/workflows/forever-rust.yml'))
        windows = next(job for job in workflow['jobs'].values()
                       if job['runs-on'] == 'windows-2022')
        steps = windows['steps']
        lint = next(step for step in steps
                    if step.get('name') == 'Check explicit MSVC v2 producer lint policy')
        self.assertIn('--all-targets --no-default-features', lint['run'])
        self.assertIn('--features forever-extractor/windows-msvc-seal-v2', lint['run'])
        self.assertIn('-- -D warnings', lint['run'])
        preflight = next(step for step in steps
                         if step.get('name') == 'Preflight native producer PE directory profile')
        workspace = next(step for step in steps
                         if step.get('name') == 'Test Rust workspace on synthetic inputs')
        self.assertLess(steps.index(preflight), steps.index(lint))
        self.assertLess(steps.index(lint), steps.index(workspace))

    def test_full_native_run_log_is_bound_to_ci_and_actual_checkout(self):
        script = self.s['native_script']
        fields = {
            'github_run_id': 'GITHUB_RUN_ID', 'github_run_attempt': 'GITHUB_RUN_ATTEMPT',
            'github_job': 'GITHUB_JOB', 'github_repository': 'GITHUB_REPOSITORY',
            'github_sha': 'GITHUB_SHA', 'runner_os': 'RUNNER_OS', 'runner_arch': 'RUNNER_ARCH'}
        for field, environment in fields.items():
            self.assertIn("'" + environment + "'", script)
            self.assertIn(field + ' = $env:' + environment, script)
        for marker in ('[Environment]::GetEnvironmentVariable($name)',
                       '[string]::IsNullOrWhiteSpace($value)',
                       "$env:RUNNER_OS -cne 'Windows'", "$env:RUNNER_ARCH -cne 'X64'",
                       "$env:GITHUB_REPOSITORY -cne 'alseif0x/rustycore-extractors'",
                       "$env:GITHUB_SHA -cnotmatch '^[0-9a-f]{40}$'",
                       '$checkoutSha = @(& git rev-parse HEAD)',
                       '$LASTEXITCODE -ne 0 -or $checkoutSha.Count -ne 1',
                       "$checkoutSha[0] -cnotmatch '^[0-9a-f]{40}$'",
                       "type = 'synthetic'", 'checkout_sha = $checkoutSha[0]',
                       '$runIdentity | ConvertTo-Json -Compress'):
            self.assertIn(marker, script)
        self.assertEqual(script.count("'WINDOWS_PRODUCT_RUN '"), 1)
        emitted = script.index("Write-Output ('WINDOWS_PRODUCT_RUN '")
        self.assertGreater(emitted, script.index('\nreturn\n}\n'))
        self.assertLess(emitted, script.index(
            'python -m unittest discover -s tests/forever -p test_windows_product_cli.py -v'))

    def test_current_source_bundle_includes_candidate_without_relabeling_v10(self):
        packager = read('tools/package_forever.py')
        self.assertIn(self.c['sources']['documentation'], packager)
        profile = next(ast.literal_eval(n.value) for n in ast.parse(packager).body
                       if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'SOURCE_PROFILE' for t in n.targets))
        self.assertEqual(profile, 'forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-source-v12')

    def test_failed_compiler_observation_is_not_native_v2_acceptance(self):
        observation = self.c['validation']['compiler_observation']
        self.assertEqual(observation['overall_result'], 'failed_pe_unsupported_directory')
        self.assertEqual(observation['run'], 38345728238)
        self.assertEqual(len(observation['report_sha256']), 64)
        self.assertIn('before any product CLI execution', observation['scope'])
        self.assertEqual(self.c['validation']['actual_windows_product_acceptance'], 'pending')
        doc = self.s['documentation']
        for phrase in ('not general volatile-metadata ABI support', 'No profile fallback',
                       'not a successful v2 execution', 'No release is authorized'):
            self.assertIn(phrase, doc)


if __name__ == '__main__':
    unittest.main()
