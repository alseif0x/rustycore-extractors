# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-12: original static GNU/Linux lease/archive source checks.
"""Source consistency only. No client, native oracle, runtime or bundle execution."""
import ast
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROFILE = 'forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-source-v12'
HISTORICAL_PROFILE = 'forever-seven-crate-linux-collision-leased-archive-source-v8'


def read(path):
    return (ROOT / path).read_text()


def literal(name):
    for node in ast.parse(read('tools/package_forever.py')).body:
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == name for t in node.targets):
            return ast.literal_eval(node.value)
    raise AssertionError(name)


class LinuxCollisionArchiveContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.c = json.loads(read('contracts/forever-linux-collision-archive-contract-v1.json'))
        cls.spool = read('crates/forever-casc/src/collision/spool.rs')
        cls.archive = read('crates/forever-casc/src/collision/spool/archive.rs')
        cls.lease = read('crates/forever-platform/src/linux/lease.rs')
        cls.ffi = read('crates/forever-platform/src/linux/lease/ffi.rs')

    def test_versioned_explicit_library_scope_keeps_product_gates_closed(self):
        self.assertEqual((self.c['contract'], self.c['version']), ('forever-linux-collision-leased-archive-v1', 1))
        self.assertTrue(self.c['original_implementation'])
        for key in ['network', 'product_cli_enabled', 'windows_product_enabled', 'full_runtime_accepted', 'real_client_accepted', 'full_extractor_complete', 'bundle_built', 'publication_authorized']:
            self.assertFalse(self.c[key])
        self.assertEqual(self.c['new_registry_dependencies'], [])
        self.assertIn('FileSetV1', self.spool)
        self.assertIn('LeasedAppendV1', self.spool)
        for name in ['default', 'explicit']:
            self.assertIn(self.c['profiles'][name], self.spool)
        self.assertIn('BackendProfile::FileSetV1', self.spool)

    def test_current_mandatory_closure_and_historical_v7_v8_are_distinct(self):
        d = json.loads(read('contracts/forever-distribution-contract-v1.json'))
        self.assertEqual(literal('SOURCE_PROFILE'), PROFILE)
        self.assertEqual(d['source']['profile'], PROFILE)
        self.assertEqual(self.c['distribution_source_profile'], HISTORICAL_PROFILE)
        self.assertIn(HISTORICAL_PROFILE, d['source']['prior_profiles'])
        self.assertIn('forever-seven-crate-windows-navigation-readers-source-v7', d['source']['prior_profiles'])
        self.assertEqual(tuple(self.c['sources']), literal('LINUX_COLLISION_ARCHIVE_SOURCE'))
        self.assertEqual(len(self.c['sources']), 28)
        for name in self.c['sources']:
            self.assertTrue((ROOT / name).is_file(), name)
            self.assertIn(name, d['source']['required'])
        self.assertIn('private append archive', d['source']['linux_collision_archive_scope'])

    def test_only_reviewed_linux_manifest_edge_no_registry_or_lock_change(self):
        m = tomllib.loads(read('crates/forever-casc/Cargo.toml'))
        edge = {'forever-platform': {'path': '../forever-platform'}}
        self.assertEqual(m['target']['cfg(target_os = "linux")']['dependencies'], edge)
        self.assertEqual(m['target']['cfg(windows)']['dependencies'], edge)
        p = tomllib.loads(read('crates/forever-platform/Cargo.toml'))
        self.assertNotIn('dependencies', p)
        packages = tomllib.loads(read('Cargo.lock'))['package']
        self.assertEqual(len(packages), 9)
        self.assertEqual(len(literal('CRATES')), 7)
        self.assertEqual({p['name'] + '-' + p['version']: p['checksum'] for p in packages if 'source' in p}, literal('DEPENDENCIES'))
        self.assertIn('forever-platform', next(p for p in packages if p['name'] == 'forever-casc')['dependencies'])

    def test_scoped_ffi_retains_exact_abi_and_safe_outer_calls(self):
        self.assertIn('#[cfg(target_os = "linux")]\npub mod linux;', read('crates/forever-platform/src/lib.rs'))
        self.assertIn('#[allow(unsafe_code)]\nmod ffi;', self.lease)
        self.assertIn('target_arch = "x86_64", target_env = "gnu"', self.lease)
        self.assertIn('size_of::<SigSet>() == 128', self.ffi)
        self.assertIn('size_of::<Owner>() == 8', self.ffi)
        self.assertIn('unsafe_code = "deny"', read('crates/forever-platform/Cargo.toml'))
        self.assertNotIn('unsafe {', self.archive)
        self.assertIn('Err(Error::Unsupported)', self.lease)
        self.assertIn('BackendUnavailable', self.spool)

    def test_helper_blocks_targeted_signal_before_lease_and_never_restores_mask(self):
        body = self.ffi[self.ffi.index('pub(super) fn establish_with_hook'):]
        self.assertLess(body.index('pthread_sigmask(SIG_BLOCK'), body.index('fcntl(fd, F_SETOWN_EX'))
        self.assertLess(body.index('fcntl(fd, F_SETOWN_EX'), body.index('fcntl(fd, F_SETLEASE'))
        self.assertLess(body.index('sigpending(&mut pending)'), body.index('setup.keep = true'))
        self.assertIn('fcntl(fd, F_SETOWN, 0 as c_int)', body)
        self.assertNotIn('SIG_UNBLOCK', self.ffi)
        self.assertNotIn('sigaction(', self.ffi)
        self.assertEqual((self.c['lease']['helper_stack_bytes'], self.c['lease']['helper_reservation_bytes']), (131072, 262144))
        self.assertIn('HELPER_STACK_BYTES: usize = 128 * 1024', self.lease)
        self.assertIn('HELPER_RESERVATION_BYTES: usize = 256 * 1024', self.lease)
        self.assertFalse(self.c['lease']['advisory_fallback'])
        self.assertEqual(read('crates/forever-platform/src/linux/lease/tests.rs').count('#[test]'), 4)

    def test_private_framing_keeps_complete_hash_and_independent_guards(self):
        self.assertEqual((self.c['framing']['container_bytes'], self.c['framing']['record_bytes']), (32, 64))
        for text in ['b"RCEARC1\\0"', 'b"RCERCD1\\0"', 'O_PATH', 'NOFOLLOW', 'current.links != 1', 'lease::verify', 'hash_body', 'sync_all()', 'in_progress']:
            self.assertIn(text, self.archive)
        self.assertIn('stored != header(ordinal, entry.bytes, entry.digest)', self.archive)
        self.assertIn('hash.finish() != entry.digest', self.archive)
        self.assertIn('complete_header_and_body_before_callback', self.c['framing'])
        tests = read('crates/forever-casc/src/collision/spool/archive/tests.rs')
        self.assertEqual(tests.count('#[test]'), 11)
        self.assertEqual(tests.count('#[ignore'), 1)
        for name in ['partial_writer', 'caught_writer_panic', 'lease_breaks', 'name_mode_and_link', 'reserved_header', 'source_and_parent', '4097_distinct_records']:
            self.assertIn(name, tests)

    def test_six_actual_backend_owners_reject_mixing_before_acquisition(self):
        for path,checks in {
            'run/inventory.rs':['wdts.backend() != collector.backend()', 'caches.backend() != collector.backend()'],
            'assemble/staged.rs':['repository.backend() != startup.collector_mut().backend()', 'wdts.backend() != repository.backend()', 'caches.backend() != repository.backend()'],
            'assemble/staged/models.rs':['repository.backend() != stage.archive().repository.backend()'],
            'assemble/staged/models/package.rs':['repository.backend() != models.view().repository.backend()'],
        }.items():
            source = read('crates/forever-casc/src/collision/' + path)
            for text in checks:
                self.assertIn(text, source)
        self.assertTrue(self.c['profiles']['six_actual_owner_backend_equality'])
        self.assertTrue(self.c['package_lock_release']['explicit_unlock_on_owner_drop'])
        self.assertIn('impl Drop for PackageLock', read('crates/forever-casc/src/package/disk.rs'))
        self.assertIn('let _ = self.0.unlock();', read('crates/forever-casc/src/package/disk.rs'))
        self.assertIn('retained_description', read('crates/forever-casc/src/package/disk/recover/tests.rs'))
        self.assertTrue(self.c['profiles']['volume_policy_independent'] and self.c['profiles']['resource_policy_independent'])

    def test_accounting_is_framed_bounded_capacity_work_not_rss(self):
        a = self.c['accounting']
        for key in ['disk_includes_container_and_framing', 'reload_includes_selected_record_framing', 'global_guard_reads_are_not_all_io_counters', 'actual_capacity_and_old_new_overlap', 'geometric_descriptor_growth', 'guard_work_constant_in_record_count']:
            self.assertTrue(a[key])
        self.assertFalse(a['rss_or_all_cpu_measurement'] or a['memory_and_active_body_ceilings_raised'])
        self.assertIn('projected_capacity_bytes', self.archive)
        self.assertIn('capacity_bytes()', self.archive)
        self.assertIn('HELPER_RESERVATION_BYTES', self.archive)
        self.assertTrue(a['helper_and_complete_root_before_archive_creation'])
        create = self.spool[self.spool.index('pub fn create_with_backend'):self.spool.index('pub const fn backend')]
        self.assertLess(create.index('.checked_add(lease_reservation)', create.index('let mut out')), create.index('archive::Archive::create'))
        self.assertIn('RECORD_BYTES', self.spool)

    def test_earlier_focused_source_does_not_accept_current_matrix_or_bundle(self):
        e = self.c['earlier_focused_acceptance']
        self.assertEqual((e['explicit_distinct_records_each'], e['exact_archive_guards_each']), (4097, 20485))
        self.assertEqual(len(e['source_freeze_sha256']), 64)
        self.assertEqual(len(e['report_sha256']), 64)
        self.assertIn('not acceptance of currentv8', e['scope'])
        doc = read('docs/forever-collision-leased-archive.md')
        self.assertIn('Complete current-v8 executable-source acceptance', doc)
        a = self.c['workspace_acceptance']
        self.assertEqual((a['ordinary_passed_each'], a['ordinary_ignored_each'], a['target_summaries_each']), (2599, 26, 41))
        self.assertEqual((a['contracts'], a['distribution_checks']), (180, 108))
        self.assertFalse(a['windows_runtime_executed'] or a['native_or_real_inputs_executed'])
        self.assertIn('not a product', doc)
        self.assertIn('old/new allocation overlap', doc)
        self.assertTrue(e['same_steady_fd_count'])


if __name__ == '__main__':
    unittest.main()
