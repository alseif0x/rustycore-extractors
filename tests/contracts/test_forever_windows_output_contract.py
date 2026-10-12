# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-12: original Windows private-output source boundary checks.
"""Source/contract/workflow consistency only; native NTFS evidence is separate."""
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


class WindowsPrivateOutputContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-output-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}
        cls.workflow = json.loads(read('.github/workflows/forever-rust.yml'))

    def test_separate_versioned_foundation_preserves_product_gates(self):
        self.assertEqual((self.c['contract'], self.c['version']),
                         ('forever-windows-private-output-v1', 1))
        self.assertEqual(self.c['api_profile'], self.c['contract'])
        foundation = json.loads(read('contracts/forever-windows-read-contract-v1.json'))
        self.assertEqual(self.c['foundation_contract'], foundation['contract'])
        self.assertEqual(self.c['read_api_profile'], foundation['api_profile'])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        for field in ('upstream_implementation_imported', 'network_api',
                      'client_data_in_git', 'windows_product_enabled',
                      'package_protocol_enabled', 'recovery_enabled',
                      'publication_authorized', 'full_extractor_complete'):
            self.assertFalse(self.c[field])
        main = read('crates/forever-extractor/src/main.rs')
        self.assertIn('#[cfg(not(target_os = "linux"))]', main)
        self.assertIn('product_platform', main)
        self.assertNotIn('windows::output', main)
        self.assertNotIn('PrivateRoot', main)

    def test_windows_only_api_keeps_unsafe_inside_private_ffi(self):
        self.assertIn('#[cfg(windows)]\npub mod windows;',
                      read('crates/forever-platform/src/lib.rs'))
        self.assertIn('pub mod output;', read('crates/forever-platform/src/windows.rs'))
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', self.s['module'])
        self.assertNotRegex(self.s['module'], r'\bunsafe\s*\{')
        for name in ('PrivateRoot', 'Directory', 'OutputFile', 'Limits'):
            self.assertRegex(self.s['module'], r'pub struct ' + name + r'\b')
        self.assertIn('"' + self.c['api_profile'] + '"', self.s['module'])

    def test_public_operations_and_opaque_same_owner_tokens(self):
        source = self.s['module']
        for method in ('create', 'root', 'stats', 'verify_unchanged',
                       'create_directory', 'create_file', 'append', 'flush', 'rename'):
            self.assertRegex(source, r'pub fn\s+' + method + r'\s*\(')
        source = compact(source)
        for token in ('Directory', 'OutputFile'):
            self.assertIn('pubstruct' + token + '{owner:u64,index:usize}', source)
        self.assertIn('token.owner!=self.owner', source)
        self.assertIn("parent:&'parentReadDirectory", source)
        self.assertIn('invalid:Cell<bool>', source)
        self.assertFalse(self.c['tokens']['raw_handles_public'])

    def test_downward_limits_match_implementation_constants(self):
        fields = {
            'objects': ('MAX_OBJECTS', 'max_objects'),
            'depth': ('MAX_DEPTH', 'max_depth'),
            'name_ascii_bytes': ('MAX_NAME', 'max_name_bytes'),
            'file_bytes': ('MAX_FILE', 'max_file_bytes'),
            'total_bytes': ('MAX_TOTAL', 'max_total_bytes'),
            'append_bytes': ('MAX_WRITE', 'max_write_bytes'),
            'operations': ('MAX_OPERATIONS', 'max_operations'),
        }
        source = compact(self.s['module'])
        self.assertTrue(self.c['limits']['downward_only'])
        for field, (constant, member) in fields.items():
            self.assertEqual(self.c['limits'][field], rust_integer(self.s['module'], constant))
            self.assertIn('self.' + member + '>' + constant, source)
        self.assertIn('depth:0', source)
        self.assertIn('operations:1', source)
        self.assertEqual(self.c['limits']['path_utf16_units'], 32760)
        self.assertIn('units>32760', source)
        self.assertIn('path_units(', source)

    def test_names_are_bounded_relative_ascii_components(self):
        source = compact(self.s['module'])
        for marker in ("b'a'..=b'z'", "b'0'..=b'9'", "b'.'", "b'_'", "b'-'",
                       'b.is_empty()', 'b.len()>max', 'b.len()>MAX_NAME',
                       'name=="."', 'name==".."', 'name==".git"',
                       "name.ends_with('.')", '"con"|"prn"|"aux"|"nul"',
                       'stem.starts_with("com")', 'stem.starts_with("lpt")'):
            self.assertIn(marker, source)
        self.assertIn('DOS reserved stems', ' '.join(self.c['names']['reject']))

    def test_relative_create_never_opens_or_adopts_existing_output(self):
        ffi = compact(self.s['ffi'])
        self.assertEqual(self.c['creation']['api'], 'NtCreateFile')
        self.assertEqual(self.c['creation']['disposition'], 'FILE_CREATE')
        self.assertIn('root:parent.as_raw_handle()', ffi)
        self.assertIn('NtCreateFile(', ffi)
        self.assertIn('share,ifcreate{2}else{1},options', ffi)
        self.assertIn('io.information!=ifcreate{2}else{1}', ffi)
        self.assertIn('File::from_raw_handle(raw)', ffi)
        self.assertIn('implDropforOwned', ffi)

    def test_process_sid_protected_single_ace_and_security_validation(self):
        ffi = compact(self.s['ffi'])
        for api in ('OpenThreadToken', 'OpenProcessToken', 'GetTokenInformation',
                    'AddAccessAllowedAceEx', 'SetSecurityDescriptorOwner',
                    'SetSecurityDescriptorDacl', 'SetSecurityDescriptorControl',
                    'GetSecurityInfo', 'GetSecurityDescriptorOwner',
                    'GetSecurityDescriptorDacl', 'GetAclInformation', 'GetAce'):
            self.assertIn(api + '(', ffi)
        self.assertEqual(rust_integer(self.s['ffi'], 'FULL'), 0x001f01ff)
        for marker in ('Error::Impersonation', 'GetLastError()}!=1008',
                       'SetSecurityDescriptorControl(sd,0x1000,0x1000)',
                       'info.count!=1', 'header.flags!=0', 'header.mask!=FULL',
                       'sid_bytes(owner,raw,len)', 'sid_bytes(ace.cast::<u8>().add(8)'):
            self.assertIn(marker, ffi)
        native = self.s['native_tests']
        for marker in ('fn independent_powershell_dacl_oracle()',
                       'Get-Acl -LiteralPath $env:RCE_OUTPUT_ACL_PATH',
                       'System.Security.Principal.SecurityIdentifier',
                       'AreAccessRulesProtected', '$inherited.Count -ne 0',
                       '$rules.Count -ne 1', 'FileSystemRights]::FullControl',
                       'InheritanceFlags]::None', 'PropagationFlags]::None',
                       'fn thread_impersonation_rejects_before_create_and_poisons_existing_root()',
                       'ffi::test_impersonate()', 'Err(Error::Impersonation)',
                       'Err(Error::Changed)'):
            self.assertIn(marker, native)
        for marker in ('$acl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value',
                       '$rule.IdentityReference.Value -ne $sid.Value',
                       'AccessControlType]::Allow', '$rule.IsInherited'):
            self.assertIn(marker, native)
        oracle = compact(native[native.index('fn independent_powershell_dacl_oracle()'):
                                native.index('fn acl_oracle(')])
        verified = oracle.index('root.verify_unchanged().unwrap();')
        dropped_root = oracle.index('drop(root);')
        dropped_parent = oracle.index('drop(parent);')
        persisted = oracle.index('for(fixture,path)in[')
        asserted = oracle.index('assert!(status.success()')
        self.assertLess(verified, dropped_root)
        self.assertLess(dropped_root, dropped_parent)
        self.assertLess(dropped_parent, persisted)
        self.assertLess(persisted, asserted)
        self.assertIn('let(status,numeric)=acl_oracle(script,&path);', oracle)
        live = oracle[oracle.index('letlive=acl_oracle('):verified]
        self.assertIn('if!live.0.success()', live)
        self.assertNotRegex(live, r'\bassert(?:_eq|_ne)?!\(')
        helper = compact(native[native.index('fn acl_oracle('):
                                native.index('fn thread_impersonation_rejects_before_create_and_poisons_existing_root()')])
        for marker in ('.stdout(std::process::Stdio::piped())',
                       '.stderr(std::process::Stdio::null())',
                       '.env_remove("PSModulePath")',
                       'Option<[i64;2]>', 'ifoutput.stdout.len()<=128',
                       'std::str::from_utf8(&output.stdout).ok()',
                       'text.split_ascii_whitespace()',
                       'letfirst=words.next()?.parse::<i64>().ok()?;',
                       'letsecond=words.next()?.parse::<i64>().ok()?;',
                       'ifwords.next().is_some(){None}else{Some([first,second])}',
                       '(output.status,numeric)'):
            self.assertIn(marker, helper)
        self.assertEqual(helper.count('.parse::<i64>()'), 2)
        self.assertNotRegex(helper, r'\b(?:e?print(?:ln)?|dbg|format|panic)!\(')
        self.assertNotIn('String::from_utf8_lossy', helper)
        self.assertIn('ImpersonateSelf(2)', ffi)
        self.assertIn('RevertToSelf()', ffi)
        self.assertIn('implDropforImpersonation', ffi)
        self.assertFalse(self.c['security']['same_user_isolation'])
        self.assertFalse(self.c['security']['privileged_mutation_protection'])

    def test_sharing_preserves_distinct_directory_and_file_boundaries(self):
        ffi = compact(self.s['ffi'])
        for marker in ('(0x0012_0081,3)', '(0x0013_0083,1)',
                       '(0x0012_0080|ifdirectory{0x20}else{0},7)'):
            self.assertIn(marker, ffi)
        sharing = self.c['sharing']
        self.assertEqual(sharing['directory_keeper']['share'],
                         ['FILE_SHARE_READ', 'FILE_SHARE_WRITE'])
        self.assertFalse(sharing['directory_keeper']['delete_sharing'])
        self.assertEqual(sharing['file_writer']['share'], ['FILE_SHARE_READ'])
        self.assertEqual(sharing['directory_target_probe']['share'],
                         ['FILE_SHARE_READ', 'FILE_SHARE_WRITE', 'FILE_SHARE_DELETE'])
        self.assertIn('SYNCHRONIZE', sharing['file_probe']['access'])
        self.assertIn('flags!=0', ffi)
        self.assertIn('NtQueryInformationFile(', ffi)
        self.assertIn(',4,71)', ffi)

    def test_append_charges_before_io_and_observed_failures_stay_poisoned(self):
        source = compact(self.s['module'])
        append = source[source.index('pubfnappend('):source.index('fnsettle(')]
        self.assertLess(append.index('self.charged_bytes=total'), append.index('writer.write_all(bytes)'))
        self.assertLess(append.index('writer.write_all(bytes)'), append.index('ffi::flush(writer)'))
        self.assertIn('self.invalid.set(true)', source)
        self.assertRegex(source, r'ifself\.invalid\.get\(\)\{returnErr\(Error::Changed\);?\}')
        self.assertIn('self.parent.verify_same_object()', source)
        self.assertIn('ffi::private(file,&self.sid)', source)

    def test_rename_is_same_owner_file_only_without_replacement(self):
        source = compact(self.s['module'])
        ffi = compact(self.s['ffi'])
        self.assertIn('pubfnrename(&mutself,file:OutputFile,destination:Directory,name:&str)', source)
        self.assertIn('self.file_index(file)?', source)
        self.assertIn('self.directory_index(destination)?', source)
        self.assertIn('replace:0', ffi)
        self.assertIn('root:destination.as_raw_handle()', ffi)
        self.assertIn('NtSetInformationFile(', ffi)
        self.assertIn('size_of::<Rename>()asu32,10)', ffi)
        self.assertIn('self.entries[i].name=name', source)
        self.assertIn('self.entries[i].parent=Some(p)', source)
        self.assertFalse(self.c['rename']['replace_if_exists'])
        self.assertFalse(self.c['flush']['durable_directory_entries'])

    def test_native_inventory_names_every_ordinary_source_test(self):
        source = self.s['native_tests']
        names = re.findall(r'#\[test\]\s+fn\s+([a-z0-9_]+)\s*\(', source)
        expected = self.c['validation']['native_test_names']
        self.assertEqual(len(expected), self.c['validation']['native_test_count'])
        self.assertEqual(len(expected), 14)
        self.assertEqual(len(expected), len(set(expected)))
        self.assertEqual(sorted(expected), sorted(names))
        self.assertNotIn('#[ignore', source)
        self.assertIn('mod privileged_tests', self.s['module'])
        self.assertTrue(self.c['validation']['privileged_reparse_gate']['ignored_by_default'])
        self.assertEqual(self.c['validation']['focused_filter'], 'windows::output::tests::')
        self.assertIn('static source/contract/workflow consistency only',
                      self.c['validation']['contract_tests'])

    def focused_steps(self):
        return [step for step in self.workflow['jobs']['windows']['steps']
                if step.get('name', '').startswith('Test Windows private output (')]

    def test_native_ci_covers_both_locked_offline_profiles(self):
        job = self.workflow['jobs']['windows']
        self.assertEqual(job['runs-on'], 'windows-2022')
        self.assertEqual(job['defaults']['run']['shell'], 'pwsh')
        steps = self.focused_steps()
        self.assertEqual([step['name'] for step in steps], [
            'Test Windows private output (debug; fixed NTFS)',
            'Test Windows private output (release; fixed NTFS)',
        ])
        for step, flag in zip(steps, ('', ' --release')):
            command = ('cargo test --offline --locked -p forever-platform --lib '
                       '--no-default-features' + flag + ' windows::output::tests::')
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

    def test_native_ci_requires_exact_inventory_and_successful_counts(self):
        for step in self.focused_steps():
            run = step['run']
            for marker in ('validation.native_test_names', 'Sort-Object',
                           '$expected.Count -eq 0', 'Tee-Object -Variable listing',
                           '[string]::Join("`n", $listed) -cne [string]::Join("`n", $expected)',
                           'Tee-Object -Variable results', '$expected.Count',
                           ' passed; 0 failed; 0 ignored;',
                           "throw 'native output result count mismatch'"):
                self.assertIn(marker, run)

    def test_native_ci_requires_fixed_ntfs_and_explicit_temporary_root(self):
        for step in self.focused_steps():
            run = step['run']
            for marker in ('$ErrorActionPreference = \'Stop\'', '$env:RUNNER_TEMP',
                           'Get-Volume -DriveLetter', "$volume.FileSystem -ne 'NTFS'",
                           "$volume.DriveType -ne 'Fixed'", '$env:TEMP = $temporaryRoot',
                           '$env:TMP = $temporaryRoot',
                           'Native private output fixture volume: fixed NTFS; OS=',
                           '[Environment]::OSVersion.VersionString'):
                self.assertIn(marker, run)

    def test_static_check_is_scheduled_on_both_ci_hosts(self):
        for job in self.workflow['jobs'].values():
            runs = '\n'.join(step.get('run', '') for step in job['steps'])
            self.assertIn('python -m unittest discover -s tests/contracts '
                          '-p test_forever_windows_output_contract.py -v', runs)

    def test_documentation_keeps_execution_and_product_scope_separate(self):
        doc = read(self.c['documentation'])
        for marker in ('Native tests require an actual Windows x86_64',
                       'Source contract checks', 'Wine', 'not a claim',
                       'directory write exclusion', 'Linux hardlink-based',
                       'does not enable the Windows product driver'):
            self.assertIn(marker, doc)
        for field in ('Windows product driver integration',
                      'Windows package recovery or durable directory entries',
                      'migration of Linux hardlink/recovery protocol', 'crash durability',
                      'same-user or privileged-attacker isolation',
                      'full real Forever extraction', 'RustyCore runtime acceptance'):
            self.assertIn(field, self.c['not_claimed'])


if __name__ == '__main__':
    unittest.main()
