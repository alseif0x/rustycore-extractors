"""GPL-3.0-or-later. Authored 2026-10-11: local Rust distribution boundaries."""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]

class DistributionContract(unittest.TestCase):
    def setUp(self):
        self.c = json.loads((ROOT / 'contracts/forever-distribution-contract-v1.json').read_text())

    def test_local_linux_preview_does_not_claim_goal_or_publication(self):
        self.assertEqual(self.c['contract'], 'forever-distribution-v1')
        self.assertEqual(self.c['version'], 1)
        self.assertEqual((self.c['platform'], self.c['target']), ('linux', 'x86_64-unknown-linux-gnu'))
        self.assertFalse(self.c['full_extractor_complete'])
        self.assertFalse(self.c['publication_authorized'])
        self.assertIn('Windows functional extraction or distribution', self.c['not_claimed'])

    def test_exact_existing_registry_closure_retains_notices(self):
        expected = {
            'adler2': ('2.0.1', '320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa'),
            'miniz_oxide': ('0.8.9', '1fa76a2c86f704bdb222d66965fb3d63269ce38518b83cb0575fca855ebb6316'),
        }
        self.assertEqual({p['name']: (p['version'], p['sha256']) for p in self.c['registry_dependencies']}, expected)
        self.assertTrue(all(p['selected_license'] == 'MIT' for p in self.c['registry_dependencies']))
        self.assertIn('all original notices', self.c['dependency_sources'])
        self.assertFalse(self.c['network'])
        self.assertIn('THIRD_PARTY_NOTICES.md', self.c['source']['required'])
        self.assertEqual(len(self.c['toolchain_notices']['sha256']), 13)
        self.assertIn('COPYRIGHT-library.html', self.c['toolchain_notices']['sha256'])
        self.assertIn('bundled copies for moved rebuild', self.c['toolchain_notices']['source'])

    def test_dirty_current_source_is_shipped_and_built(self):
        self.assertIn('including uncommitted', self.c['source']['identity'])
        self.assertIn('frozen shipped', self.c['build']['source'])
        self.assertEqual(self.c['source']['symlinks_and_special_files'], 'reject')
        self.assertIn('mutation', self.c['source'])
        self.assertIn('keys', self.c['source']['forbidden'])
        self.assertIn('generated extraction output', self.c['source']['forbidden'])

    def test_v12_profile_requires_v23_closure_without_packaging_claims(self):
        self.assertEqual(
            self.c['source']['profile'],
            'forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-source-v12',
        )
        required_v23 = {
            'crates/forever-extractor/src/driver/navigation_v23.rs',
            'crates/forever-extractor/src/driver/navigation_v23/coverage_tests.rs',
            'crates/forever-extractor/src/driver/navigation_v23/input_plan_tests.rs',
            'crates/forever-extractor/src/driver/v23_api_tests.rs',
            'crates/forever-extractor/src/plan/v23_tests.rs',
            'crates/forever-extractor/tests/v23_selected_navigation_cli.rs',
            'crates/forever-extractor/tests/v23_selected_navigation_cli/fixture.rs',
            'contracts/forever-product-contract-v23.json',
            'tests/contracts/test_forever_product_v23_contract.py',
            'tests/contracts/fixtures/forever-product-v23/map48.tsv',
            'docs/forever-product-v23.md',
            'docs/acceptance/forever-v23-selected-navigation-70450-20261020.json',
            'docs/forever-navigation.md',
        }
        self.assertTrue(required_v23 <= set(self.c['source']['required']))
        acceptance = self.c['source']['product_v23_selected_navigation_acceptance']
        self.assertFalse(acceptance['bundle_built'])
        self.assertFalse(acceptance['two_fresh_candidates_match'])
        self.assertFalse(acceptance['moved_source_rebuild_matches'])
        self.assertFalse(acceptance['packaged_v23_cli_accepted'])

    def test_offline_recipe_and_rebuild_are_explicit(self):
        self.assertEqual(self.c['toolchain'], '1.98.0')
        self.assertEqual(self.c['build']['recipe'], 'cargo build --offline --locked --release -p forever-extractor --no-default-features')
        self.assertIn('--epoch UNIX', self.c['commands']['build'])
        self.assertIn('--bundle UNPACKED_ROOT', self.c['commands']['rebuild'])
        self.assertFalse(self.c['build']['native_backend'])
        self.assertIn('moved-source rebuild', self.c['build']['reproducibility'])

    def test_archive_verifier_is_bounded_integrity_not_authentication(self):
        self.assertEqual(self.c['archive']['format'], 'sorted deterministic tar.gz')
        self.assertIn('duplicate rejection', self.c['archive']['verification'])
        self.assertIn('no execution', self.c['archive']['verification'])
        self.assertFalse(self.c['archive']['self_authentication'])
        self.assertIn('distribution-manifest.json', self.c['archive']['contains'])
        self.assertEqual(self.c['limits']['members'], 8192)
        self.assertEqual(self.c['limits']['total_payload_bytes'], 256 * 1024 * 1024)
        self.assertEqual(self.c['limits']['manifest_bytes'], 4 * 1024 * 1024)
        self.assertIn('not process RSS', self.c['limits']['scope'])

    def test_real_runtime_and_client_acceptance_remain_separate(self):
        for item in ['full real Forever extraction', 'new C++ output parity', 'new RustyCore runtime acceptance', 'release publication']:
            self.assertIn(item, self.c['not_claimed'])
        self.assertIn('no static or universal Linux ABI compatibility claim', self.c['build']['runtime_evidence'])

if __name__ == '__main__':
    unittest.main()
