# SPDX-License-Identifier: GPL-3.0-or-later
"""Linux link-profile source guards; runtime acceptance is separately retained."""
import ast
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


class LinuxOutputLinksContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-linux-output-links-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}

    def test_explicit_scope_preserves_previous_contracts(self):
        self.assertEqual(self.c['verification_profile'], 'forever-terrain-navigation-links-v1')
        self.assertEqual(self.c['platform'], 'linux')
        self.assertEqual(self.c['public_plan_versions_unchanged'], list(range(1, 7)))
        self.assertEqual(self.c['existing_verify_scope'], 'artifact_integrity_and_intrinsic_formats')
        self.assertTrue(self.c['package_manifest_unchanged'])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertTrue(all(value is False for value in self.c['claims'].values()))
        for key in ('requires_client', 'requires_keys', 'requires_scratch', 'windows_input_io'):
            self.assertFalse(self.c[key])

    def test_platform_gate_precedes_path_io(self):
        main = self.s['main']
        guard = '#[cfg(not(target_os = "linux"))]\n    if command == "verify-links"'
        self.assertIn(guard, main)
        self.assertLess(main.index(guard), main.index('let bytes = input::bytes'))
        self.assertIn('return Err("product_platform".into());', main[main.index(guard):])
        self.assertIn('#[cfg(target_os = "linux")]\npub mod links;', self.s['verify'])
        self.assertIn('#[cfg(target_os = "linux")]\npub fn verify_links(', self.s['verify'])

    def test_existing_verify_has_unchanged_scope_and_shared_traversal(self):
        verify = self.s['verify']
        self.assertIn('verify_inner(plan, output, expected, Scope::Intrinsic)', verify)
        self.assertIn('verify_inner(plan, output, expected, Scope::Links)', verify)
        self.assertEqual(verify.count('package::verify_with('), 1)
        self.assertIn('artifact_integrity_and_intrinsic_formats', self.s['main'])
        self.assertIn('verification_profile', self.s['main'])
        self.assertIn(self.c['verification_profile'], self.s['links'])
        self.assertIn(self.c['scope'], self.s['links'])

    def test_v8_auto_enables_links_without_changing_legacy_intrinsic_scope(self):
        verify = re.sub(r'\s+', '', self.s['verify'])
        self.assertIn('(matches!(scope,Scope::Links)||plan.map_domain.is_some()).then(links::Facts::default)', verify)
        # Both operands are false for the existing intrinsic invocation with
        # map_domain=None; only explicit links or the new v8 domain enable it.
        self.assertIn('verify_inner(plan,output,expected,Scope::Intrinsic)', verify)
        self.assertIn('ifplan.map_domain.is_none(){returnOk(());}', verify)
        self.assertLess(verify.index('facts.finish_domain(plan)?;'), verify.index('links.finish(plan)?;'))

    def test_link_facts_follow_intrinsic_checks_and_complete_context(self):
        verify = self.s['verify']
        self.assertLess(verify.index('facts.inspect(plan, artifact, &bytes)?;'),
                        verify.index('links.inspect(artifact, &bytes)?;'))
        self.assertLess(verify.index('product_plan_mismatch'), verify.index('links.finish(plan)?;'))
        self.assertLess(verify.index('product_inventory'), verify.index('links.finish(plan)?;'))
        self.assertLess(verify.index('links.finish(plan)?;'), verify.index('Ok(summary)'))

    def test_link_metadata_has_no_independent_file_access(self):
        source = self.s['links']
        for marker in ('std::fs', 'File::open', 'read_to_end', 'LocalStore', 'Run::', 'unsafe {'):
            self.assertNotIn(marker, source)
        self.assertIn('maps: [u64; WORDS]', source)
        self.assertIn('listed: [u64; WORDS]', source)
        self.assertIn('4095 - reverse', source)
        self.assertIn('(y * 64 + x) as usize', source)
        self.assertIn('Selection::All | Selection::Map(_)', source)
        self.assertIn('map.maps != map.listed', source)
        self.assertIn('map.navigation_tile && !map.navigation_global', source)

    def test_ceiling_precedes_allocation_and_has_explicit_accounting(self):
        limits = self.c['limits']
        self.assertEqual(limits['observed_map_ids'], 8192)
        self.assertEqual(limits['logical_metadata_bytes_per_map'], 4096)
        self.assertEqual(limits['logical_metadata_bytes'], 32 * 1024 * 1024)
        self.assertFalse(limits['exact_rss_claim'])
        self.assertFalse(limits['retained_artifact_bytes'])
        source = self.s['links']
        self.assertIn('MAX_LINK_MAPS: usize = 8192', source)
        self.assertIn('LOGICAL_BYTES_PER_MAP: usize = 4096', source)
        self.assertIn('MAX_LINK_MAPS * LOGICAL_BYTES_PER_MAP', source)
        self.assertIn('size_of::<MapFacts>() * 2 <= LOGICAL_BYTES_PER_MAP', source)
        self.assertLess(source.index('self.maps.len() >= MAX_LINK_MAPS'), source.index('.try_reserve_exact(1)'))
        for error in self.c['errors']:
            self.assertIn(error, source)

    def test_exact_unit_inventory(self):
        names = re.findall(r'#\[test\]\s*fn (\w+)\(', self.s['unit_tests'])
        self.assertEqual(names, self.c['unit_test_names'])
        self.assertEqual(len(names), 23)
        self.assertNotIn('#[ignore', self.s['unit_tests'])

    def test_exact_authored_cli_inventory(self):
        tree = ast.parse(self.s['cli_tests'])
        names = sorted(n.name for cls in tree.body if isinstance(cls, ast.ClassDef)
                       for n in cls.body if isinstance(n, ast.FunctionDef) and n.name.startswith('test_'))
        self.assertEqual(names, self.c['cli_test_names'])
        self.assertEqual(len(names), 20)
        for marker in ('inject=network:error=ENETUNREACH', 'RCE_PRODUCT_LINKS_CASES', 'verify-links'):
            self.assertIn(marker, self.s['cli_tests'])

    def test_linux_ci_selects_profile_source_checks(self):
        workflow = json.loads(read('.github/workflows/forever-rust.yml'))
        command = 'python -m unittest discover -s tests/contracts -p test_forever_linux_output_links_contract.py -v'
        for name, job in workflow['jobs'].items():
            runs = '\n'.join(step.get('run', '') for step in job['steps'])
            self.assertEqual(runs.count(command), 1 if name == 'linux' else 0)

    def test_corresponding_source_includes_all_new_paths(self):
        spec = importlib.util.spec_from_file_location('linux_links_packager', ROOT / 'tools/package_forever.py')
        packager = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(packager)
        selected = set(packager.selected_paths(ROOT))
        paths = list(self.c['sources'].values()) + [
            'contracts/forever-linux-output-links-contract-v1.json',
            'tests/contracts/test_forever_linux_output_links_contract.py']
        for path in paths:
            self.assertIn(path, selected)
            self.assertTrue(packager.bundle_path_allowed('source/' + path))


if __name__ == '__main__':
    unittest.main()
