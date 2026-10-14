# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-12: original explicit membership source boundary checks.
"""Static source/contract/CI consistency; no native filesystem acceptance."""
import ast
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


def compact(source):
    return re.sub(r',(?=[)}])', '', re.sub(r'\s+', '', source))


def rust_integer(source, name):
    match = re.search(r'\bconst\s+' + name + r'\s*:\s*(?:usize|u64|u32)\s*=\s*([^;]+);', source)
    if not match:
        raise AssertionError('missing integer constant ' + name)

    def value(node):
        if isinstance(node, ast.Constant) and type(node.value) is int:
            return node.value
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Mult):
            return value(node.left) * value(node.right)
        raise AssertionError('unsupported integer constant expression ' + name)

    return value(ast.parse(match.group(1).strip(), mode='eval').body)


class WindowsMembershipContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-membership-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}
        cls.output = read(cls.c['integration_source'])
        cls.workflow = json.loads(read('.github/workflows/forever-rust.yml'))

    def focused_steps(self):
        return [step for step in self.workflow['jobs']['windows']['steps']
                if step.get('name', '').startswith('Test Windows private membership (')]

    def test_separate_versioned_foundation_and_disabled_product_gates(self):
        self.assertEqual((self.c['contract'], self.c['version']),
                         ('forever-windows-private-membership-v1', 1))
        self.assertEqual(self.c['api_profile'], self.c['contract'])
        foundation = json.loads(read('contracts/forever-windows-output-contract-v1.json'))
        self.assertEqual(self.c['foundation_contract'], foundation['contract'])
        self.assertEqual(self.c['foundation_profile'], foundation['api_profile'])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertFalse(self.c['validation']['native_windows_ntfs_accepted'])
        self.assertFalse(self.c['validation']['hosted_ci_executed'])
        self.assertIn('native execution acceptance pending', self.c['status'])
        for field in ('upstream_implementation_imported', 'network_api',
                      'client_data_in_git', 'windows_product_enabled',
                      'package_protocol_enabled', 'recovery_enabled',
                      'publication_authorized', 'new_bundle_built',
                      'full_extractor_complete'):
            self.assertFalse(self.c[field])

    def test_windows_only_explicit_safe_public_api(self):
        self.assertIn('#[cfg(windows)]\npub mod windows;',
                      read('crates/forever-platform/src/lib.rs'))
        self.assertIn('pub mod membership;', self.output)
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', self.s['module'])
        self.assertNotRegex(self.s['module'], r'\bunsafe\s*\{')
        self.assertIn('"' + self.c['api_profile'] + '"', self.s['module'])
        self.assertRegex(self.s['module'], r'pub fn\s+verify_membership\s*\(')
        for name in ('Limits', 'Summary'):
            self.assertRegex(self.s['module'], r'pub struct\s+' + name + r'\b')

    def test_existing_identity_api_and_mutations_do_not_auto_enumerate(self):
        source = compact(self.output)
        self.assertNotIn('membership::verify_membership(', source)
        self.assertNotIn('NtQueryDirectoryFile', self.output)
        self.assertFalse(self.c['explicit']['default_verify_unchanged_changed'])
        self.assertFalse(self.c['explicit']['automatic_mutation_scan'])
        self.assertFalse(self.c['explicit']['scan_parent_outside_private_root'])
        old_tests = read('crates/forever-platform/src/windows/output/tests.rs')
        self.assertIn('fn unknown_child_creation_is_observation_not_inventory_closure()', old_tests)

    def test_native_inventory_names_every_ordinary_source_case(self):
        source = self.s['native_tests']
        names = re.findall(r'#\[test\]\s+fn\s+([a-z0-9_]+)\s*\(', source)
        expected = self.c['validation']['native_test_names']
        self.assertGreater(len(expected), 0)
        self.assertEqual(len(expected), self.c['validation']['native_test_count'])
        self.assertEqual(len(expected), len(set(expected)))
        self.assertEqual(sorted(expected), sorted(names))
        self.assertNotIn('#[ignore', source)
        self.assertEqual(self.c['validation']['focused_filter'],
                         'windows::output::membership::tests::')

    def test_focused_ci_covers_both_locked_offline_profiles(self):
        job = self.workflow['jobs']['windows']
        self.assertEqual(job['runs-on'], 'windows-2022')
        self.assertEqual(job['defaults']['run']['shell'], 'pwsh')
        steps = self.focused_steps()
        self.assertEqual([step['name'] for step in steps], [
            'Test Windows private membership (debug; fixed NTFS)',
            'Test Windows private membership (release; fixed NTFS)',
        ])
        for step, flag in zip(steps, ('', ' --release')):
            command = ('cargo test --offline --locked -p forever-platform --lib '
                       '--no-default-features' + flag + ' windows::output::membership::tests::')
            runs = step['run'].splitlines()
            cargo = [line for line in runs if line.startswith('cargo ')]
            self.assertEqual(len(cargo), 2)
            self.assertTrue(cargo[0].startswith(command + ' -- --list --color never'))
            self.assertTrue(cargo[1].startswith(command + ' -- --test-threads=1 --color never'))
            for line in cargo:
                self.assertEqual(runs[runs.index(line) + 1],
                                 'if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }')
            self.assertEqual(step['env']['CARGO_TARGET_DIR'],
                             '${{ runner.temp }}/forever-rust-target')

    def test_focused_ci_requires_exact_inventory_and_zero_failures_ignored(self):
        for step in self.focused_steps():
            run = step['run']
            for marker in ('contracts/forever-windows-membership-contract-v1.json',
                           'validation.native_test_names', '$expected.Count -eq 0',
                           'Tee-Object -Variable listing',
                           '[string]::Join("`n", $listed) -cne [string]::Join("`n", $expected)',
                           'Tee-Object -Variable results',
                           ' passed; 0 failed; 0 ignored;',
                           "throw 'native membership result count mismatch'"):
                self.assertIn(marker, run)

    def test_focused_ci_records_fixed_ntfs_and_os_without_private_paths(self):
        for step in self.focused_steps():
            for marker in ('$env:RUNNER_TEMP', 'Get-Volume -DriveLetter',
                           "$volume.FileSystem -ne 'NTFS'", "$volume.DriveType -ne 'Fixed'",
                           '$env:TEMP = $temporaryRoot', '$env:TMP = $temporaryRoot',
                           'Native private membership fixture volume: fixed NTFS; OS=',
                           '[Environment]::OSVersion.VersionString'):
                self.assertIn(marker, step['run'])

    def test_previous_fourteen_output_cases_and_steps_remain_separate(self):
        previous = json.loads(read('contracts/forever-windows-output-contract-v1.json'))
        self.assertEqual(previous['validation']['native_test_count'], 14)
        self.assertEqual(previous['validation']['focused_filter'], 'windows::output::tests::')
        steps = [step for step in self.workflow['jobs']['windows']['steps']
                 if step.get('name', '').startswith('Test Windows private output (')]
        self.assertEqual(len(steps), 2)
        for step in steps:
            self.assertIn('contracts/forever-windows-output-contract-v1.json', step['run'])
            self.assertNotIn('windows::output::membership::tests::', step['run'])
        self.assertIn('a220596', self.c['validation']['historical_output'])
        self.assertIn('5ce8b83', self.c['validation']['historical_output'])

    def test_static_checks_scheduled_on_both_hosts_without_relabeling_distribution(self):
        for job in self.workflow['jobs'].values():
            runs = '\n'.join(step.get('run', '') for step in job['steps'])
            self.assertIn('python -m unittest discover -s tests/contracts '
                          '-p test_forever_windows_membership_contract.py -v', runs)
            self.assertIn('test_forever_windows_output_contract.py', runs)
        source = read('tools/package_forever.py')
        self.assertIn('forever-seven-crate-linux-product-db2-gt-cameras-source-v11', source)
        main = read('crates/forever-extractor/src/main.rs')
        self.assertIn('#[cfg(not(any(target_os = "linux", windows)))]', main)
        self.assertIn('product_platform', main)
        self.assertNotIn('membership::verify_membership', main)

    def test_documentation_distinguishes_observation_execution_and_product(self):
        doc = ' '.join(read(self.c['documentation']).split())
        for marker in ('Native tests require an actual Windows x86_64',
                       'not a filesystem', 'may or may not',
                       'permanently poisons', 'admitted/decoded-record cap',
                       'without decoding', 'not a claim',
                       'Linux hardlink/recovery', 'fourteen'):
            self.assertIn(marker, doc)
        for url in self.c['primary_sources'].values():
            self.assertIn(url, doc)
        self.assertFalse(self.c['guards']['atomic_snapshot'])
        self.assertFalse(self.c['guards']['same_sid_isolation'])
        self.assertFalse(self.c['guards']['directory_write_exclusion'])


    def test_downward_limits_fixed_inventory_and_invalid_admission_order(self):
        source = compact(self.s['module'])
        for field, constant in (('directories', 'MAX_DIRECTORIES'),
                                ('records', 'MAX_RECORDS'),
                                ('query_calls', 'MAX_QUERY_CALLS')):
            self.assertEqual(rust_integer(self.s['module'], constant), self.c['limits'][field])
            self.assertIn('self.max_' + field + '==0', source)
            self.assertIn('self.max_' + field + '>' + constant, source)
        verify = source.split('pubfnverify_membership(', 1)[1].split('#[derive', 1)[0]
        self.assertLess(verify.index('limits.validate()?'), verify.index('root.verify_unchanged()'))
        self.assertLess(verify.index('limits.validate()?'), verify.index('root.invalid.set(true)'))
        for marker in ('[Expected::EMPTY;MAX_OBJECTS]', '[None;MAX_OBJECTS]',
                       '[false;MAX_OBJECTS]', 'directory_count>limits.max_directories'):
            self.assertIn(marker, source)
        self.assertIn('without poisoning', self.c['limits']['invalid_arguments'])

    def test_native_abi_fixed_buffer_and_single_entry_query_parameters(self):
        module, ffi = compact(self.s['module']), compact(self.s['ffi'])
        self.assertEqual(rust_integer(self.s['module'], 'BUFFER_BYTES'), 640)
        self.assertEqual(rust_integer(self.s['module'], 'NAME_OFFSET'), 80)
        self.assertEqual(rust_integer(self.s['module'], 'MAX_NATIVE_NAME_UNITS'), 255)
        for marker in ('#[repr(C,align(8))]', 'bytes:[u8;BUFFER_BYTES]',
                       'offset_of!(FileIdFull,file_id)==72', 'offset_of!(FileIdFull,name)==80',
                       'size_of::<Io>()==16', 'offset_of!(Io,information)==8',
                       'buffer.bytes.fill(0)',
                       'BUFFER_BYTESasu32,38,1,null_mut(),u8::from(restart)'):
            self.assertIn(marker, ffi)
        self.assertIn('file.as_raw_handle(),null_mut(),None,null_mut(),&mutio', ffi)
        self.assertNotRegex(self.s['ffi'], r'\b(?:CreateFile|NtCreateFile|CloseHandle)\s*\(')
        self.assertIn('letmutrestart=true;', module)
        self.assertIn('restart=false;', module)

    def test_checked_record_extent_names_identity_and_reparse_policy(self):
        source = compact(self.s['module']).split('fndecode(', 1)[1].split('fncompletion(', 1)[0]
        for marker in ('u32_at(0)?!=0', 'u32_at(56)?', 'attributes&0x400!=0',
                       'u32_at(60)?', 'length%2!=0', 'length>MAX_NATIVE_NAME_UNITS*2',
                       'NAME_OFFSET.checked_add(length)', 'end.checked_add(7)',
                       'bytes.len()!=end&&bytes.len()!=padded',
                       'bytes.get(NAME_OFFSET..end)', 'char::decode_utf16(units())',
                       'count>MAX_NAME', 'Name::parse(text,MAX_NAME)',
                       'bytes.get(72..80)', 'file_id==0', 'attributes&0x10!=0'):
            self.assertIn(marker, source)
        self.assertNotIn('u32_at(4)', source)
        self.assertIn('opaque ignored alignment padding', self.c['enumeration']['framing'])
        self.assertFalse(self.c['enumeration']['file_index_used_as_identity'])

    def test_eof_requires_matching_status_and_zero_information(self):
        source = compact(self.s['module']).split('fncompletion(', 1)[1]
        for marker in ('status==0x8000_0006', 'io_status==status&&information==0',
                       'Ok(None)', 'status==0&&io_status==0&&information>0&&information<=BUFFER_BYTES',
                       'Ok(Some(information))', 'Err(Error::Query)'):
            self.assertIn(marker, source)
        ffi = compact(self.s['ffi'])
        self.assertIn('status:usize::MAX,information:usize::MAX', ffi)
        self.assertIn('completion(statusasu32,io.statusasu32,io.information)', ffi)
        self.assertIn('STATUS_NO_MORE_FILES only', self.c['enumeration']['eof'])

    def test_exact_owned_child_dot_identity_and_missing_duplicate_checks(self):
        source = compact(self.s['module'])
        accept = source.split('fnaccept(', 1)[1].split('fndecode(', 1)[0]
        for marker in ('child.parent==Some(directory)&&child.name==name',
                       '.ok_or(Error::Extra)?', 'ifseen[index]',
                       'child.file_id!=record.file_id', 'child.directory!=record.directory',
                       'ifdots[dot]', 'owned.file_id', 'None=>external_parent',
                       'if!record.directory', 'record.file_id!=file_id',
                       'child.parent==Some(directory)&&!seen[index]', 'Error::Missing'):
            self.assertIn(marker, accept)
        self.assertFalse(self.c['explicit']['unknown_entry_adoption'])
        self.assertFalse(self.c['explicit']['scan_parent_outside_private_root'])
        self.assertNotIn('std::fs::read_dir', self.s['module'])

    def test_full_scan_guards_poison_and_record_cap_eof_lookahead(self):
        source = compact(self.s['module'])
        verify = source.split('pubfnverify_membership(', 1)[1].split('#[derive', 1)[0]
        self.assertEqual(verify.count('root.verify_unchanged()'), 2)
        self.assertLess(verify.index('letscanned=scan('), verify.index('letguarded=root.verify_unchanged()'))
        self.assertLess(verify.index('letguarded='), verify.index('scanned.and_then('))
        self.assertRegex(verify, r'ifresult.is_err\(\)\{root.invalid.set\(true\);?\}')
        scan = source.split('fnscan(', 1)[1].split('fnfull_directory(', 1)[0]
        query = scan.index('ffi::query(')
        self.assertLess(scan.index('summary.query_calls>=limits.max_query_calls'), query)
        self.assertLess(scan.index('summary.query_calls+=1'), query)
        self.assertLess(query, scan.index('letSome(length)=lengthelse'))
        self.assertLess(scan.index('letSome(length)=lengthelse'), scan.index('summary.records>=limits.max_records'))
        self.assertLess(scan.index('summary.records>=limits.max_records'), scan.index('decode('))
        self.assertLess(scan.index('before[index]=Some(full_directory(entry)?)'), query)
        self.assertGreater(scan.index('directories_unchanged(root,&before)?'), scan.index('complete(inventory,'))
        self.assertGreater(scan.index('full_directory(entry)?!=captured'), scan.index('complete(inventory,'))
        self.assertIn('without decoding', self.c['limits']['record_cap_lookahead'])
        self.assertFalse(self.c['guards']['atomic_snapshot'])


if __name__ == '__main__':
    unittest.main()
