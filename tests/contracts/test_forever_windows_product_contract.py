# SPDX-License-Identifier: GPL-3.0-or-later
"""Source/corpus/workflow consistency; native execution is separate evidence."""
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


def python311_tree(source, filename):
    tree = ast.parse(source, filename=filename, feature_version=(3, 11))
    # On newer CPython, feature_version does not restore pre-PEP701 f-string
    # tokenization. Explicitly retain the legacy restriction that caused the
    # native 3.11 import failure; actual 3.11 compile/import remains authoritative.
    for node in ast.walk(tree):
        if isinstance(node, ast.FormattedValue):
            expression = ast.get_source_segment(source, node.value)
            if expression is None or '\\' in expression:
                raise SyntaxError('Python 3.11 f-string expression contains a backslash: ' + filename)
    return tree


def native_import_closure():
    roots = (ROOT / 'tests/forever', ROOT / 'tools')
    pending = [roots[0] / 'test_windows_product_cli.py']
    visited = {}
    while pending:
        path = pending.pop()
        if path in visited:
            continue
        tree = python311_tree(path.read_text(encoding='utf-8'), str(path.relative_to(ROOT)))
        visited[path] = tree
        for node in ast.walk(tree):
            names = [a.name for a in node.names] if isinstance(node, ast.Import) else (
                [node.module] if isinstance(node, ast.ImportFrom) and node.level == 0 and node.module else [])
            for name in names:
                for root in roots:
                    dependency = root.joinpath(*name.split('.')).with_suffix('.py')
                    if dependency.is_file():
                        pending.append(dependency)
    return visited


class WindowsProductContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-windows-product-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}

    def test_separate_profiles_preserve_historical_and_target_gates(self):
        c = self.c
        self.assertEqual(c['contract'], 'forever-windows-product-ntfs-v1')
        self.assertEqual(c['namespace_profile'], 'forever-windows-package-namespace-v1')
        self.assertEqual(c['producer_profile'], 'forever-windows-producer-seal-v1')
        self.assertEqual(c['msvc_feature_producer_profile'], 'forever-windows-producer-seal-msvc-v2')
        self.assertEqual(c['msvc_feature'], 'windows-msvc-seal-v2')
        self.assertEqual(c['components'], ['db2', 'gt', 'cameras', 'maps', 'vmaps', 'mmaps'])
        self.assertTrue(c['windows_resident_product_enabled'])
        self.assertFalse(c['linux_leased_disk_enabled_on_windows'])
        self.assertEqual(c['targets']['70338'], {'plan_version': 5, 'components': ['db2']})
        self.assertEqual(c['new_registry_dependencies'], [])
        for field in ('network_api', 'client_data_in_git', 'publication_authorized', 'full_extractor_complete'):
            self.assertFalse(c[field])
        for name in ('output', 'membership', 'producer'):
            historical = json.loads(read(f'contracts/forever-windows-{name}-contract-v1.json'))
            self.assertFalse(historical['windows_product_enabled'])

    def test_windows_state_is_distinct_from_unchanged_public_and_linux_formats(self):
        c = self.c['package']
        self.assertEqual((c['manifest_version'], c['windows_state_version'], c['linux_state_version']), (1, 3, 2))
        self.assertFalse(c['state_migration'])
        self.assertFalse(c['power_loss_durability_claimed'])
        self.assertIn('"state_version\\\":3', self.s['package'])
        self.assertIn('"transaction_version\\\":3', self.s['package'])
        self.assertIn(self.c['contract'], self.s['package'])
        disk = read('crates/forever-casc/src/package/disk.rs')
        self.assertIn('hard_link(', disk)
        self.assertNotIn('hard_link(', self.s['package'] + self.s['recovery'])

    def test_producer_identity_and_disk_admission_are_explicit(self):
        main = read('crates/forever-extractor/src/main.rs')
        self.assertIn('producer::loaded_identity()', main)
        self.assertLess(main.index('driver::admit(&plan)?'), main.index('producer::loaded_identity()'))
        self.assertLess(main.index('driver::admit(&plan)?'), main.index('input::keys(p)'))
        self.assertIn('product_disk_unsupported', self.s['driver'])
        self.assertIn('inventory::windows as navigation_input', self.s['driver'])
        self.assertIn('verify_keys(key_source.as_ref())', read('crates/forever-extractor/src/driver/windows.rs'))
        self.assertIn('input::SuppliedKeys', read('crates/forever-extractor/src/driver/windows.rs'))
        for field in ('loaded_producer_required_for_client_commands',
                      'same_lease_hash_and_inspection', 'replay_unsealed_facts',
                      'complete_writer_resume_rejected', 'recovery_preflight_before_mutation'):
            self.assertTrue(self.c['guards'][field])
        self.assertFalse(self.c['guards']['plan_only_inspect_requires_seal'])
        self.assertFalse(self.c['guards']['verify_requires_seal'])

    def test_scalable_namespace_has_independent_budgets_and_no_raw_file_api(self):
        source = self.s['namespace']
        for marker in ('pub struct Root', 'pub struct Limits', 'pub struct Entry',
                       'pub fn observe', 'pub fn verify_membership', 'pub fn with_new_file',
                       'pub fn with_read_file', 'pub fn rename_no_replace', 'pub fn remove_same',
                       'pub fn lock', 'pub fn protect_output', 'pub fn file_identity'):
            self.assertIn(marker, source)
        self.assertIn(self.c['namespace_profile'], source)
        self.assertNotIn('PrivateRoot', source)
        self.assertNotRegex(source, r'pub fn[^\n]*->[^\n]*\bFile\b')
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', source)
        self.assertNotRegex(source, r'\bunsafe\s*\{')
        self.assertEqual(self.c['limits']['namespace_entries'], 786_464)
        self.assertEqual(self.c['limits']['namespace_handles'], 96)
        for field in ('atomic_snapshot', 'same_sid_isolation'):
            self.assertFalse(self.c['guards'][field])

    def test_exact_native_test_inventories_match_authored_functions(self):
        for key, path in [('namespace', 'crates/forever-platform/src/windows/package/tests.rs'),
                          ('package', 'crates/forever-casc/src/package/windows/tests.rs')]:
            names = re.findall(r'#\[test\]\s*fn (\w+)\(', read(path))
            self.assertTrue(names)
            self.assertEqual(len(names), len(set(names)))
            self.assertEqual(names, self.c['validation']['native_test_names'][key])
            self.assertNotIn('#[ignore', read(path))

    def test_exact_native_cli_inventory_and_boundary_are_explicit(self):
        source = self.s['native_cli']
        tree = ast.parse(source)
        names = sorted(n.name for cls in tree.body if isinstance(cls, ast.ClassDef)
                       for n in cls.body if isinstance(n, ast.FunctionDef) and n.name.startswith('test_'))
        self.assertEqual(names, self.c['validation']['cli_test_names'])
        self.assertEqual(len(names), 15)
        for marker in ('process.kill()', 'observed_db2_seal_before_final_manifest',
                       'pe_digest_mismatch', 'producer_unsealed', 'product_disk_unsupported',
                       'self.assertEqual(snapshot(self.client), self.inputs)',
                       'self.plan.write_bytes(text.encode())',
                       "'WINDOWS_PRODUCT_CASE_REPORT '",
                       "{'case': self._testMethodName, 'report': report}"):
            self.assertIn(marker, source)
        self.assertFalse(self.c['validation']['network_syscall_tracing'])
        self.assertNotIn('strace', source)

    def test_python311_grammar_guard_catches_original_fstring_failure(self):
        old = 'value = f"{(b\'\\x55\' * 16).hex()}"'
        with self.assertRaises(SyntaxError):
            python311_tree(old, 'original-synthetic-regression.py')
        python311_tree('cdn_key = (b"\\x55" * 16).hex()\nvalue = f"{cdn_key}"', 'fixed.py')

    def test_native_cli_entire_local_import_closure_uses_python311_grammar(self):
        closure = native_import_closure()
        self.assertEqual({path.stem for path in closure}, {
            'test_windows_product_cli', 'product_fixture', 'product_disk_fixture',
            'product_keys_fixture', 'product_offmesh_fixture', 'db2_oracle',
            'test_db2_oracle', 'seal_forever_windows'})

    def test_native_cli_fixture_closure_imports_without_running_native_cases(self):
        script = '''import importlib, json, sys, unittest
sys.path[:0] = sys.argv[1:]
module = importlib.import_module('test_windows_product_cli')
names = ('product_fixture', 'product_disk_fixture', 'product_keys_fixture',
         'product_offmesh_fixture', 'db2_oracle', 'test_db2_oracle', 'seal_forever_windows')
print(json.dumps({'modules': sorted(name for name in names if name in sys.modules),
                  'cases': unittest.defaultTestLoader.loadTestsFromModule(module).countTestCases()}))
'''
        env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
        for name in ('FOREVER_WINDOWS_PRODUCT_BINARY', 'FOREVER_WINDOWS_PRODUCT_UNSEALED',
                     'FOREVER_WINDOWS_PRODUCT_CASES'):
            env.pop(name, None)
        result = subprocess.run([sys.executable, '-c', script, str(ROOT / 'tests/forever'),
                                 str(ROOT / 'tools')], cwd=ROOT, env=env, capture_output=True,
                                text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {
            'modules': sorted(['product_fixture', 'product_disk_fixture', 'product_keys_fixture',
                               'product_offmesh_fixture', 'db2_oracle', 'test_db2_oracle',
                               'seal_forever_windows']), 'cases': 15})

    def test_synthetic_golden_is_complete_and_internally_bound(self):
        for key in ('golden', 'golden_70170'):
            golden = json.loads(self.s[key])
            facts = golden['artifacts']
            self.assertEqual(len(facts), self.c['validation']['golden_artifacts'])
            self.assertEqual(sum(a['bytes'] for a in facts), self.c['validation']['golden_bytes'])
            encoded = ''.join(json.dumps(a, separators=(',', ':')) + '\n' for a in facts).encode()
            self.assertEqual(hashlib.sha256(encoded).hexdigest(), golden['inventory_sha256'])
            self.assertFalse(golden['native_cpp_execution'])
            self.assertFalse(golden['real_client_inputs'])
            self.assertEqual(golden['linux_source_commit'], '6f1a6fa2f17fb174ec74cbd7b9440b026b66e973')
            self.assertEqual({a['component'] for a in facts}, set(self.c['components']))
            for a in facts:
                self.assertRegex(a['sha256'], '^[0-9a-f]{64}$')
                self.assertFalse(a['path'].startswith(('/', '..')))

    def test_native_workflow_runs_both_profiles_with_exact_counts(self):
        workflow = json.loads(read('.github/workflows/forever-rust.yml'))
        job = workflow['jobs']['windows']
        self.assertEqual(job['runs-on'], 'windows-2022')
        steps = [s for s in job['steps'] if s.get('run') == '. ./tools/check_forever_windows_product.ps1']
        self.assertEqual(len(steps), 1)
        self.assertEqual(steps[0]['env']['CARGO_TARGET_DIR'], '${{ runner.temp }}/forever-rust-target')
        script = self.s['native_script']
        for marker in ("$volume.FileSystem -ne 'NTFS'", "$volume.DriveType -ne 'Fixed'",
                       "foreach ($release in @($false, $true))", "foreach ($profile in @('debug', 'release'))",
                       '--offline', '--locked', '--test-threads=1', '-- --list',
                       'native package test inventory mismatch', '0 failed; 0 ignored;',
                       'seal --input $unsealed --output $sealed --zero-checksum',
                       "'WINDOWS_PRODUCT_BINARY '",
                       'python -m unittest discover -s tests/distribution -p test_windows_seal.py -v',
                       '--features windows-msvc-seal-v2',
                       '--profile forever-windows-producer-seal-msvc-v2',
                       "$release 'producer-msvc-v2'",
                       'if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'):
            self.assertIn(marker, script)
        for j in workflow['jobs'].values():
            runs = '\n'.join(s.get('run', '') for s in j['steps'])
            self.assertIn('test_forever_windows_product_contract.py', runs)
            self.assertIn('test_forever_windows_producer_msvc_contract.py', runs)

    def test_documented_scope_does_not_turn_a_source_candidate_into_acceptance(self):
        doc = read(self.c['documentation'])
        for marker in ('Cross-compilation is not native Windows/NTFS acceptance',
                       'No release or client-data publication is authorized',
                       'same SID', 'without an atomic', 'not RSS',
                       'cross-platform Rust artifact regression',
                       'not an injected power-loss', 'Native execution and CI report identities are pending'):
            self.assertIn(marker, doc)


if __name__ == '__main__':
    unittest.main()
