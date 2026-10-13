# SPDX-License-Identifier: GPL-3.0-or-later
"""Private orchestration source guards; byte/runtime evidence stays separate."""
import importlib.util
import json
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[2]


def read(path):
    return (ROOT / path).read_text(encoding='utf-8')


def require_common_boundary(source):
    if re.search(r'\b(target_os|target_family|windows|unix)\b|std::fs|LocalStore::open|Run::(?:create|resume)', source):
        raise ValueError('platform_setup_in_execution')


class CommonExecutionContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-common-execution-contract-v1.json'))
        cls.s = {name: read(path) for name, path in cls.c['sources'].items()}

    def test_scope_preserves_public_contract_and_open_gates(self):
        self.assertEqual(self.c['contract'], 'forever-common-execution-v1')
        self.assertEqual(self.c['components'], ['db2', 'gt', 'cameras', 'maps', 'vmaps', 'mmaps'])
        self.assertEqual(self.c['public_plan_versions_unchanged'], list(range(1, 7)))
        self.assertTrue(self.c['public_api_unchanged'])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        for key in ('new_cli_required', 'automatic_plan_added', 'full_extractor_complete',
                    'new_real_client_acceptance', 'new_native_windows_acceptance_recorded',
                    'new_distribution_acceptance'):
            self.assertFalse(self.c[key])

    def test_common_boundary_has_no_os_setup(self):
        require_common_boundary(self.s['execution'])
        self.assertIn('pub(super) trait Adapter', self.s['execution'])
        self.assertIn('adapter: impl Adapter', self.s['execution'])
        self.assertIn('mod execution;', self.s['linux'])

    def test_boundary_rejects_platform_coupling(self):
        for marker in ('#[cfg(windows)]', '#[cfg(target_os = "linux")]', 'std::fs::read(path)',
                       'LocalStore::open(options)', 'Run::create(output)', 'Run::resume(output)'):
            with self.subTest(marker=marker), self.assertRaisesRegex(ValueError, 'platform_setup_in_execution'):
                require_common_boundary(marker)

    def test_six_concrete_producers_keep_the_existing_implementations(self):
        s = self.s['execution']
        for call in ('super::db2(self.plan, self.store, &mut self.run)',
                     'super::game_tables(self.store, &mut self.run)',
                     'super::cameras(self.store, &mut self.run)',
                     'super::maps(self.plan, self.store, &mut self.run)',
                     'self.adapter.extract_collision(self.store, &mut self.run)',
                     'super::mmaps(self.plan, self.store, self.output, &mut self.run)'):
            self.assertIn(call, s)
        components = re.findall(r'Component::(\w+) => work\.', s)
        self.assertEqual(components, ['Db2', 'GameTables', 'Cameras', 'Terrain', 'Collision', 'Navigation'])

    def test_lifecycle_keeps_skips_verification_error_tags_and_seals(self):
        s = self.s['execution'].split('fn complete<W: Work>', 1)[1].split('#[cfg(test)]', 1)[0]
        markers = ['let completed = work.completed_components()?;', 'for component in Component::ALL',
                   'if selected & component.bit() == 0 || completed & component.bit() != 0',
                   'continue;', 'work.verify_before_producer()?;', 'let result = match component',
                   'result.map_err', 'error.component = Some(component);',
                   'work.verify_inputs()?;', 'work.finish_component(component)?;']
        positions = [s.index(marker) for marker in markers]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(s.count('work.verify_inputs()?;'), 2)
        self.assertEqual(s.count('error.component ='), 1)
        self.assertLess(s.rindex('work.verify_inputs()?;'), s.index('work.finish()'))
        production = self.s['execution'].split('impl<A: Adapter> Work', 1)[1].split('\nfn complete<', 1)[0]
        self.assertLess(production.index('.verify_unchanged()'), production.index('self.adapter.verify_inputs()'))

    def test_linux_owns_scratch_without_moving_platform_io_into_core(self):
        s = self.s['linux']
        self.assertIn('execution::execute(plan, &mut store, output, run, LinuxExecution { preflight })', s)
        adapter = s.split('struct LinuxExecution', 1)[1].split('fn read(', 1)[0]
        for marker in ('preflight: Option<disk::Preflight>', 'impl execution::Adapter for LinuxExecution',
                       'disk.verify_unchanged()?', 'Some(disk) => disk.run(store, run)', 'None => vmaps(store, run)'):
            self.assertIn(marker, adapter)
        self.assertNotIn('fn verify_before_producer', adapter)
        self.assertTrue(self.c['linux']['scratch_owned_through_finish'])

    def test_windows_retains_key_lease_and_every_return_guard(self):
        s = self.s['windows']
        self.assertIn('key_source: key_source.as_ref()', s)
        self.assertIn('verify_keys(key_source.as_ref())?;\n    result', s)
        adapter = s.split("struct WindowsExecution<'a>", 1)[1]
        self.assertIn("key_source: Option<&'a ReadFile>", adapter)
        self.assertIn("impl execution::Adapter for WindowsExecution<'_>", adapter)
        self.assertEqual(adapter.count('verify_keys(self.key_source)'), 2)
        self.assertIn('vmaps(store, run)', adapter)
        self.assertTrue(self.c['windows']['key_lease_owned_through_finish'])

    def test_exact_lifecycle_unit_inventory(self):
        names = re.findall(r'#\[test\]\s*fn (\w+)\(', self.s['execution'])
        self.assertEqual(len(names), 11)
        self.assertEqual(names, self.c['unit_test_names'])
        self.assertNotIn('#[ignore', self.s['execution'])

    def test_all_existing_ci_roles_select_this_contract(self):
        workflow = json.loads(read('.github/workflows/forever-rust.yml'))
        self.assertEqual(set(workflow['jobs']), {'linux', 'windows', 'windows-product'})
        command = 'python -m unittest discover -s tests/contracts -p test_forever_common_execution_contract.py -v'
        for name, job in workflow['jobs'].items():
            runs = '\n'.join(step.get('run', '') for step in job['steps'])
            self.assertEqual(runs.count(command), 1)
            if name != 'linux':
                self.assertIn(command + '\nif ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }', runs)

    def test_corresponding_source_selects_and_admits_new_files(self):
        spec = importlib.util.spec_from_file_location('common_execution_packager', ROOT / 'tools/package_forever.py')
        packager = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(packager)
        selected = set(packager.selected_paths(ROOT))
        for path in ('crates/forever-extractor/src/driver/execution.rs',
                     'contracts/forever-common-execution-contract-v1.json',
                     'tests/contracts/test_forever_common_execution_contract.py'):
            self.assertIn(path, selected)
            self.assertTrue(packager.bundle_path_allowed('source/' + path))


if __name__ == '__main__':
    unittest.main()
