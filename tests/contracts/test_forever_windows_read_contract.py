# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original Windows read foundation source contract.
"""Static consistency checks. They do not execute Win32 or establish NTFS acceptance."""
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / 'crates/forever-platform'


def constant(source, name):
    value = re.search(r'const ' + name + r': (?:u32|usize) = ([^;]+);', source).group(1)
    # Only literal numeric factors are accepted. No source code is evaluated.
    factors = value.split('*')
    if not all(re.fullmatch(r'\s*(?:0x[0-9a-fA-F_]+|[0-9_]+)\s*', factor) for factor in factors):
        raise ValueError('nonliteral_constant')
    product = 1
    for factor in factors:
        value = factor.strip().replace('_', '')
        product *= int(value, 16 if value.startswith('0x') else 10)
    return product


class WindowsReadContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads((ROOT / 'contracts/forever-windows-read-contract-v1.json').read_text(encoding='utf-8'))
        cls.path = (CRATE / 'src/path.rs').read_text(encoding='utf-8')
        cls.windows = (CRATE / 'src/windows.rs').read_text(encoding='utf-8')
        cls.ffi = (CRATE / 'src/windows/ffi.rs').read_text(encoding='utf-8')
        cls.lib = (CRATE / 'src/lib.rs').read_text(encoding='utf-8')
        cls.readme = (CRATE / 'README.md').read_text(encoding='utf-8')

    def test_separate_versioned_api_does_not_enable_windows_product(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-windows-read-v1', 1))
        self.assertEqual(self.c['profile'], 'windows-x64-fixed-ntfs-read-lease-v1')
        self.assertIn('driver remains disabled', self.c['status'])
        self.assertEqual(self.c['api_profile'], 'forever-windows-read-ntfs-v1')
        self.assertIn(self.c['api_profile'], self.windows)
        self.assertFalse(self.c['full_extractor_complete'])
        self.assertFalse(self.c['publication_authorized'])
        self.assertFalse(self.c['network'])
        self.assertFalse(self.c['output_writes'])
        self.assertIn('complete Windows product extraction and driver integration', self.c['not_claimed'])
        self.assertIn('#[cfg(windows)]\npub mod windows;', self.lib)
        self.assertIn('not(target_arch = "x86_64")', self.lib)

    def test_no_new_upstream_source_or_registry_dependency(self):
        self.assertFalse(self.c['implementation']['upstream_implementation_imported'])
        self.assertEqual(self.c['implementation']['registry_dependencies'], [])
        self.assertNotIn('[dependencies]', (CRATE / 'Cargo.toml').read_text())
        self.assertIn('unsafe_code = "deny"', (CRATE / 'Cargo.toml').read_text())
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', self.windows)
        self.assertIn('unsafe extern "system"', self.ffi)
        self.assertNotIn('unsafe {', self.windows)
        self.assertIn('1.98.0', self.c['implementation']['ffi_rationale'])
        self.assertIn('Original, dependency-free', self.readme)

    def test_exact_path_and_read_limits_match_source(self):
        limits = self.c['limits']
        for key, name in [('ancestors_including_volume_root', 'MAX_ANCESTORS'),
                          ('path_utf16_units', 'MAX_PATH_UNITS'),
                          ('read_bytes_per_operation', 'MAX_READ_BYTES')]:
            self.assertEqual(limits[key], constant(self.path, name))
        self.assertEqual(limits['component_utf16_units'], 255)
        self.assertIn('value.encode_utf16().count() > 255', self.path)
        self.assertEqual(limits['steady_retained_handles'], limits['ancestors_including_volume_root'] + 1)
        self.assertEqual(limits['peak_handles_per_lease'], limits['steady_retained_handles'] + limits['temporary_verification_handles'])
        self.assertEqual(limits['peak_handles_per_lease'], 65)
        self.assertTrue(limits['downward_only'])
        self.assertIn('self.max_ancestors > MAX_ANCESTORS', self.path)
        self.assertIn('self.max_path_units > MAX_PATH_UNITS', self.path)
        self.assertIn('self.max_read_bytes > MAX_READ_BYTES', self.path)
        self.assertIn('self.ancestors.len() + 1', self.windows)

    def test_directory_read_sharing_requires_list_access(self):
        opening = self.c['open']
        flags = opening['directory_access']['flags']
        self.assertEqual(flags, ['FILE_LIST_DIRECTORY', 'FILE_READ_ATTRIBUTES'])
        self.assertEqual(opening['directory_access']['value'], 0x81)
        self.assertEqual(constant(self.windows, flags[0]) | constant(self.windows, flags[1]), 0x81)
        self.assertIn('options.access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES);', self.windows)
        self.assertTrue(opening['directory_access']['requires_list_directory_permission'])
        self.assertIn('metadata-only', opening['directory_access']['reason'])
        self.assertEqual(opening['share_mode']['value'], constant(self.windows, 'FILE_SHARE_READ'))
        self.assertEqual(opening['share_mode']['flags'], ['FILE_SHARE_READ'])
        self.assertFalse(opening['share_mode']['write_sharing'])
        self.assertFalse(opening['share_mode']['delete_sharing'])
        self.assertIn('.share_mode(FILE_SHARE_READ)', self.windows)
        self.assertIn('options.read(true);', self.windows)

    def test_no_reparse_following_or_filesystem_fallback(self):
        self.assertEqual(self.c['open']['custom_flags'], {
            name: constant(self.windows, name)
            for name in ['FILE_FLAG_BACKUP_SEMANTICS', 'FILE_FLAG_OPEN_REPARSE_POINT']
        })
        self.assertIn('.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)', self.windows)
        self.assertIn('identity.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0', self.windows)
        self.assertIn('GetDriveTypeW(wide.as_ptr()) } != 3', self.ffi)
        self.assertIn('filesystem[..5] != [78, 84, 70, 83, 0]', self.ffi)
        self.assertIn('GetFileType(handle) } != 1', self.ffi)
        self.assertIn('ffi::ntfs_volume(&file)?;', self.windows)
        for excluded in ['ReFS', 'network drives', 'removable drives', 'junctions', 'cloud reparses']:
            self.assertIn(excluded, self.c['admission']['unsupported'])

    def test_three_queries_capture_change_time_and_repeat_fields(self):
        identity = self.c['identity']
        self.assertEqual(identity['snapshot_query_order'], [
            'GetFileInformationByHandle(before)', 'GetFileInformationByHandleEx(FileBasicInfo)',
            'GetFileInformationByHandle(after)',
        ])
        first = self.ffi.index('GetFileInformationByHandle(handle, &mut before)')
        middle = self.ffi.index('GetFileInformationByHandleEx(', first)
        last = self.ffi.index('GetFileInformationByHandle(handle, &mut after)', middle)
        self.assertLess(first, middle)
        self.assertLess(middle, last)
        self.assertEqual(identity['basic_info_class'], 0)
        self.assertIn('change_time: basic.change', self.ffi)
        self.assertIn('result == convert(after)', self.ffi)
        self.assertIn('result.creation_time == basic.creation as u64', self.ffi)
        self.assertIn('result.write_time == basic.write as u64', self.ffi)
        self.assertIn('result.attributes != basic.attributes', self.ffi)
        for name, size in identity['ffi_layout_bytes'].items():
            self.assertIn(f'std::mem::size_of::<{name}>() == {size}', self.ffi)
        self.assertIn('std::mem::align_of::<BasicInfo>() == 8', self.ffi)

    def test_ancestor_and_leaf_identity_comparisons_are_distinct(self):
        identity = self.c['identity']
        self.assertEqual(identity['ancestor_comparison'], ['volume', 'file_index', 'attributes'])
        start = self.windows.index('fn same_object(')
        end = self.windows.index('\n    }', start)
        body = self.windows[start:end]
        for name in identity['ancestor_comparison']:
            self.assertIn(f'self.{name} == other.{name}', body)
        for name in ['size', 'links', 'creation_time', 'write_time', 'change_time']:
            self.assertNotIn(f'self.{name}', body)
        fields = re.search(r'pub struct Identity \{(.*?)\n\}', self.windows, re.S).group(1)
        self.assertEqual(re.findall(r'pub (\w+):', fields), identity['fields'])
        self.assertEqual(identity['excluded'], ['last_access_time'])
        self.assertNotIn('pub access', fields)
        self.assertIn('pub fn verify_unchanged(&self) -> Result<(), Error> {\n        self.verify(false)', self.windows)
        self.assertIn('if object_only {\n                self.leaf.identity.same_object(identity)\n            } else {\n                self.leaf.identity == identity', self.windows)
        self.assertIn('if !matches(current)', self.windows)
        self.assertIn('self.check_leaf(&self.leaf.file, object_only)?', self.windows)
        self.assertIn('self.check_leaf(&named, object_only)', self.windows)

    def test_object_acquisition_ignores_timestamps_before_comparison(self):
        identity = self.c['identity']
        self.assertEqual(identity['object_query_order'], identity['snapshot_query_order'])
        self.assertEqual(identity['object_query_ignored_consistency_fields'],
                         ['links', 'size', 'creation_time', 'write_time', 'change_time'])
        self.assertIn('full snapshots:', identity['repeated_query_checks'])
        self.assertIn('both FileInfo samples', identity['object_query_checks'])
        opening = self.windows[self.windows.index('for _ in 0..path.ancestor_count()'):self.windows.index('drop(prefixes);')]
        self.assertIn('let identity = ffi::object_identity(&file)?;', opening)
        self.assertIn('let identity = ffi::identity(&file)?;', opening)
        ancestors = self.windows[self.windows.index('for (prefix, held)'):self.windows.index('self.check_leaf(&self.leaf.file')]
        self.assertIn('ffi::object_identity(&held.file)?', ancestors)
        self.assertIn('ffi::object_identity(&named)?', ancestors)
        self.assertNotIn('ffi::identity(', ancestors)
        leaf = self.windows[self.windows.index('fn check_leaf('):self.windows.index('pub fn read_all(')]
        self.assertIn('if object_only {\n            ffi::object_identity(file)?\n        } else {\n            ffi::identity(file)?', leaf)
        self.assertIn('result.same_object(convert(after))', self.ffi)
        self.assertIn('before.links == 0 || after.links == 0', self.ffi)
        self.assertIn('object_query_accepts_within_query_metadata_churn_but_full_query_rejects', self.ffi)
        self.assertIn('object_and_full_queries_reject_identity_attribute_or_link_admission_changes', self.ffi)

    def test_observed_verification_failure_permanently_poisons_lease(self):
        self.assertIn('if self.invalid.get()', self.windows)
        self.assertIn('if result.is_err() {\n            self.invalid.set(true);', self.windows)
        self.assertIn('result.map_err(|_| Error::Changed)', self.windows)
        self.assertNotIn('self.invalid.set(false)', self.windows)
        self.assertIn('permanently invalidates', self.c['identity']['poison'])
        self.assertTrue(self.c['identity']['not_authentication'])
        self.assertTrue(self.c['identity']['not_atomic_snapshot'])

    def test_alias_admission_does_not_claim_output_containment(self):
        path = self.c['admission']['path']
        self.assertIn('8.3', path['short_names'])
        self.assertIn('held identity', path['case_normalization'])
        self.assertIn('not lexical output-path containment', path['output_protection'])
        self.assertIn('lexical output-path or key-source protection', self.c['not_claimed'])
        self.assertEqual(path['unicode_normalization'], 'none')
        for reserved in path['reserved_stems']:
            self.assertIn('"' + reserved + '"', self.path)
        self.assertIn('"¹" | "²" | "³"', self.path)

    def test_link_policy_is_explicit_and_range_reads_remain_bounded(self):
        self.assertEqual(set(self.c['admission']['hardlinks']), {'Reject', 'Allow'})
        self.assertIn('links == LinkPolicy::Reject && identity.links != 1', self.windows)
        self.assertIn('output.len() > self.limits.max_read_bytes', self.windows)
        self.assertIn('.checked_add(output.len() as u64)', self.windows)
        self.assertIn('end > self.leaf.identity.size', self.windows)
        self.assertIn('.seek_read(&mut extra, self.leaf.identity.size)', self.windows)
        self.assertIn('caller buffer may hold a prefix', self.c['reads']['failure'])
        for method in ['identity', 'retained_handles', 'verify_unchanged', 'read_all', 'read_exact_at']:
            self.assertIn('pub fn ' + method + '(', self.windows)

    def test_source_contract_does_not_claim_actual_windows_acceptance(self):
        validation = self.c['validation']
        self.assertFalse(validation['real_windows_ntfs_accepted'])
        self.assertIn('not execution of actual Windows calls', validation['contract_tests'])
        self.assertIn('preliminary only', validation['wine'])
        self.assertIn('0x81', validation['wine'])
        self.assertIn('ignored by default', validation['reparse_test'])
        for gate in ['actual Windows/NTFS acceptance', 'DACL creation or private output directory policy',
                     'crash durability or directory-entry publication protocol',
                     'loaded executable identity', 'full real Forever extraction',
                     'RustyCore runtime acceptance', 'goal completion']:
            self.assertIn(gate, self.c['not_claimed'])

    def test_directory_full_snapshot_and_explicit_object_mode_share_poison(self):
        self.assertIn('pub struct ReadDirectory {\n    inner: ReadFile,', self.windows)
        self.assertIn('ReadFile::open_kind(path, limits, LinkPolicy::Allow, true)?', self.windows)
        directory = self.windows[self.windows.index('impl ReadDirectory'):self.windows.index('pub fn operator_path')]
        self.assertIn('self.inner.verify_unchanged()', directory)
        self.assertIn('pub fn verify_same_object(&self) -> Result<(), Error> {\n        self.inner.verify(true)', directory)
        self.assertIn('object comparison cannot relax', self.c['identity']['directory_comparison']['after_failed_full_check'])
        self.assertEqual(self.c['directory']['final_access'], 0x81)
        self.assertIn('no enumeration', self.c['directory']['operation'])

    def test_operator_paths_do_no_io_and_missing_is_only_notfound(self):
        helper = self.windows[self.windows.index('pub fn operator_path'):self.windows.index('\nfn open(')]
        self.assertIn('AbsolutePath::parse(', helper)
        self.assertIn('result.push(admitted.as_str())', helper)
        for operation in ['canonicalize(', 'ReadFile::open(', 'File::open(', 'ReadDirectory::open(']:
            self.assertNotIn(operation, helper)
        self.assertIn('no', self.c['operator_path'])
        opening = self.windows[self.windows.index('\nfn open('):self.windows.index('\nfn admit(')]
        self.assertIn('error.kind() == std::io::ErrorKind::NotFound', opening)
        self.assertIn('Error::Missing', opening)
        self.assertIn('Error::Open', opening)
        self.assertIn('Self::Missing => "platform_missing"', self.lib)
        self.assertEqual(set(self.c['diagnostic_categories']), {'platform_missing', 'platform_open', 'platform_changed'})

    def test_constant_reader_rejects_nonliteral_source(self):
        with self.assertRaisesRegex(ValueError, 'nonliteral_constant'):
            constant('const TEST: usize = unsafe_function();', 'TEST')


if __name__ == '__main__':
    unittest.main()
