# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: source-only synthetic CI boundary checks.
"""The workflow uses the JSON subset of YAML; no YAML dependency is needed.

These checks inspect the planned workflow. They do not prove a hosted run,
Windows product support, client extraction or semantic/runtime compatibility.
"""
import copy
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / '.github/workflows/forever-rust.yml'
PINS = {
    'actions/checkout': '11d5960a326750d5838078e36cf38b85af677262',
    'actions/setup-python': 'a26af69be951a213d495a4c3e4e4022e16d87065',
}


def validate_boundary(workflow):
    if workflow['permissions'] != {'contents': 'read'}:
        raise ValueError('permissions')
    if set(workflow['on']) != {'pull_request', 'push', 'workflow_dispatch'}:
        raise ValueError('events')
    for event in ('pull_request', 'push'):
        if workflow['on'][event]['branches'] != ['forever']:
            raise ValueError('version_line')
    if workflow['env']['RUSTUP_TOOLCHAIN'] != '1.98.0':
        raise ValueError('toolchain')
    if 'runner.' in json.dumps(workflow['env']):
        raise ValueError('context_availability')
    for job in workflow['jobs'].values():
        if not 1 <= job['timeout-minutes'] <= 20:
            raise ValueError('timeout')
        steps = job['steps']
        if steps[0].get('uses') != 'actions/checkout@' + PINS['actions/checkout']:
            raise ValueError('checkout')
        if steps[0]['with'] != {'persist-credentials': False}:
            raise ValueError('credentials')
        for step in steps:
            if 'uses' in step:
                name, _, commit = step['uses'].partition('@')
                if PINS.get(name) != commit:
                    raise ValueError('action_pin')
            run = step.get('run', '')
            if run and step.get('env', {}).get('CARGO_TARGET_DIR') != '${{ runner.temp }}/forever-rust-target':
                raise ValueError('external_outputs')
            for line in run.splitlines():
                if line.startswith('cargo '):
                    if '--locked' not in line:
                        raise ValueError('cargo_lock')
                    if not line.startswith('cargo fetch ') and '--offline' not in line:
                        raise ValueError('offline_execution')
                    if '--features native' in line or '--all-features' in line or '--ignored' in line:
                        raise ValueError('external_acceptance')
                if job['defaults']['run']['shell'] == 'pwsh' and line.startswith(('cargo ', 'python ', 'rustup ')):
                    suffix = run.split(line, 1)[1]
                    if not suffix.startswith('\nif ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }'):
                        raise ValueError('windows_exit')
        text = json.dumps(job)
        for forbidden in ('secrets.', 'upload-artifact', 'actions/cache', 'self-hosted',
                          'RCE_OPENCODE_CLIENT', 'RCE_FOREVER_CASC_REFERENCE',
                          'RCE_PRODUCT_BINARY', 'checks live', 'cargo publish'):
            if forbidden in text:
                raise ValueError('data_boundary')


class ForeverCiContract(unittest.TestCase):
    def setUp(self):
        self.workflow = json.loads(WORKFLOW.read_text(encoding='utf-8'))

    def test_workflow_has_closed_synthetic_boundary(self):
        validate_boundary(self.workflow)

    def test_linux_exercises_workspace_both_profiles(self):
        job = self.workflow['jobs']['linux']
        self.assertEqual(job['runs-on'], 'ubuntu-24.04')
        self.assertEqual(job['strategy']['matrix']['include'], [
            {'profile': 'debug', 'profile-flag': ''},
            {'profile': 'release', 'profile-flag': '--release'},
        ])
        runs = '\n'.join(s.get('run', '') for s in job['steps'])
        self.assertIn('cargo test --offline --locked --workspace --all-targets --no-default-features ${{ matrix.profile-flag }}', runs)
        self.assertIn('cargo clippy --offline --locked --workspace --all-targets --no-default-features -- -D warnings', runs)

    def test_windows_workspace_covers_read_consumers_and_unsupported_output_guards(self):
        job = self.workflow['jobs']['windows']
        self.assertEqual(job['runs-on'], 'windows-2022')
        tests = [s['run'] for s in job['steps'] if s.get('run', '').startswith('cargo test ')]
        self.assertEqual(len(tests), 1)
        self.assertEqual(tests[0].splitlines()[0], 'cargo test --offline --locked --workspace --all-targets --no-default-features')
        builds = [s['run'] for s in job['steps'] if s.get('run', '').startswith('cargo build ')]
        self.assertEqual(builds[0].splitlines()[0], 'cargo build --offline --locked --workspace --bins --no-default-features')

    def test_python_uses_stdlib_checks_without_pip(self):
        for job in self.workflow['jobs'].values():
            step = next(s for s in job['steps'] if s.get('uses', '').startswith('actions/setup-python@'))
            self.assertEqual(step['with'], {'python-version': '3.11'})
            text = '\n'.join(s.get('run', '') for s in job['steps'])
            self.assertNotIn('pip ', text)
            for filename in ('test_forever_ci_contract.py', 'test_forever_product_contract.py',
                             'test_forever_product_keys_contract.py', 'test_forever_product_offmesh_contract.py',
                             'test_forever_distribution_contract.py',
                             'test_db2_oracle.py', 'test_metadata_catalog.py', 'test_metadata_cli.py',
                             'test_forever_windows_read_contract.py', 'test_forever_windows_consumers_contract.py'):
                self.assertIn(filename, text)

    def test_metadata_cli_is_explicitly_built_and_addressed(self):
        for name, job in self.workflow['jobs'].items():
            step = next(s for s in job['steps'] if s.get('name') == 'Exercise synthetic metadata CLI')
            binary = step['env']['FOREVER_DB2_INSPECT']
            self.assertTrue(binary.startswith('${{ runner.temp }}/forever-rust-target/'))
            self.assertEqual(binary.endswith('.exe'), name == 'windows')
            build = '\n'.join(s.get('run', '') for s in job['steps'])
            self.assertIn('cargo build --offline --locked', build)

    def test_download_plan_precedes_fetch_and_execution_is_offline(self):
        for job in self.workflow['jobs'].values():
            names = [s.get('name') for s in job['steps']]
            self.assertLess(names.index('Display explicit build-only download plan'), names.index('Install exact Rust version'))
            self.assertLess(names.index('Install exact Rust version'), names.index('Fetch locked registry closure'))
            self.assertLess(names.index('Fetch locked registry closure'), next(i for i, name in enumerate(names) if name and name.startswith('Test ')))

    def rejects_mutation(self, change, code):
        workflow = copy.deepcopy(self.workflow)
        change(workflow)
        with self.assertRaisesRegex(ValueError, '^' + code + '$'):
            validate_boundary(workflow)

    def test_rejects_runner_context_at_workflow_env_scope(self):
        self.rejects_mutation(lambda w: w['env'].update(CARGO_TARGET_DIR='${{ runner.temp }}/target'), 'context_availability')

    def test_rejects_write_permission(self):
        self.rejects_mutation(lambda w: w['permissions'].update(contents='write'), 'permissions')

    def test_rejects_untrusted_privileged_event(self):
        self.rejects_mutation(lambda w: w['on'].update(pull_request_target={}), 'events')

    def test_rejects_moving_action_tag(self):
        self.rejects_mutation(lambda w: w['jobs']['linux']['steps'][1].update(uses='actions/setup-python@v5'), 'action_pin')

    def test_rejects_unlocked_fetch(self):
        self.rejects_mutation(lambda w: next(s for s in w['jobs']['linux']['steps'] if s.get('name') == 'Fetch locked registry closure').update(run='cargo fetch'), 'cargo_lock')

    def test_rejects_network_enabled_test(self):
        self.rejects_mutation(lambda w: next(s for s in w['jobs']['linux']['steps'] if s.get('name', '').startswith('Test ')).update(run='cargo test --locked --workspace'), 'offline_execution')

    def test_rejects_ignored_external_corpus_execution(self):
        self.rejects_mutation(lambda w: next(s for s in w['jobs']['linux']['steps'] if s.get('name', '').startswith('Test ')).update(run='cargo test --offline --locked --workspace -- --ignored'), 'external_acceptance')

    def test_rejects_windows_failure_masking(self):
        self.rejects_mutation(lambda w: next(s for s in w['jobs']['windows']['steps'] if s.get('name') == 'Fetch locked registry closure').update(run='cargo fetch --locked'), 'windows_exit')

    def test_rejects_artifact_upload(self):
        self.rejects_mutation(lambda w: w['jobs']['linux']['steps'].append({'uses': 'actions/upload-artifact@' + '0' * 40}), 'action_pin')


if __name__ == '__main__':
    unittest.main()
