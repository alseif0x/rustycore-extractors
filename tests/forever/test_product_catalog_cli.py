"""GPL-3.0-or-later. Retained synthetic catalog v4/v5 CLI acceptance.

Set RCE_PRODUCT_BINARY and a NEW RCE_PRODUCT_CATALOG_CASES outside every Git
checkout. All CLI calls deny network syscalls and retain stdout/stderr/traces.
Authored DB2 bytes, no real client data, keys, native backend or downloads.
"""
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import unittest

import product_catalog_fixture as fixture


BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_CATALOG_CASES')
REPO = Path(__file__).resolve().parents[2]


class CatalogFixture(unittest.TestCase):
    def test_literal_metadata_matches_both_catalogs(self):
        for _, build, catalog in fixture.PROFILES:
            with self.subTest(build=build):
                source = json.loads((REPO / f'contracts/{catalog}.json').read_text())
                declarations = {t['name']: t for t in source['tables']}
                for name, (fid, table_hash, layout_hash, widths) in fixture.TABLES.items():
                    meta = declarations[name]
                    self.assertEqual((meta['file_data_id'], meta['table_hash'], meta['layout_hash']),
                                     (fid, table_hash, layout_hash))
                    self.assertEqual(meta['index'], -1)
                    self.assertEqual(meta['parent_index'], -1)
                    self.assertEqual(meta['file_field_count'], len(widths))
                    self.assertEqual(meta['decode_kinds'],
                                     ['String', 'Signed(32, 1)', 'Signed(8, 1)', 'Signed(8, 1)', 'Signed(32, 1)']
                                     if name == 'AnimKitBoneSet' else ['Signed(32, 1)'] * 3)

    def test_authored_text_uses_logical_pool_offsets_and_full_eof(self):
        data = fixture.table('AnimKitBoneSet')
        self.assertEqual(struct.unpack_from('<I', data, 136)[0], 3)
        self.assertEqual(struct.unpack_from('<I', data, 200)[0], 2)
        first = struct.unpack_from('<I', data, fixture.HEADER + 8)[0]
        second = struct.unpack_from('<I', data, fixture.HEADER + fixture.SECTION + 8)[0]
        self.assertEqual(struct.unpack_from('<I', data, first)[0], 3 * 14)
        self.assertEqual(struct.unpack_from('<I', data, first + 14)[0], 0)
        self.assertEqual(struct.unpack_from('<I', data, second)[0], 3 * 14 + 6 - 2 * 14)
        self.assertEqual(data[first + 28:first + 34], b'alpha\0')
        self.assertEqual(data[second + 14:second + 20], 'café\0'.encode())
        self.assertEqual(data[second + 20:], struct.pack('<III', 30, 50, 40))


@unittest.skipUnless(BINARY and CASES, 'explicit external synthetic catalog product run required')
class ProductCatalogCli(unittest.TestCase):
    def setUp(self):
        parent = Path(CASES)
        self.assertTrue(parent.is_absolute())
        self.root = parent / self._testMethodName
        self.assertFalse(any((p / '.git').exists() for p in (self.root.resolve(), *self.root.resolve().parents)))
        self.root.mkdir(parents=True, mode=0o700)
        self.calls = 0

    def prepare(self, profile, client_build=None, make_client=True, **kwargs):
        self.version, self.build, self.catalog = profile
        self.case = self.root / f'v{self.version}'
        self.case.mkdir(mode=0o700)
        self.client = self.case / 'client'
        self.output = self.case / 'output'
        self.plan = self.case / 'plan.tsv'
        self.plan.write_text(fixture.plan(*profile), encoding='ascii')
        self.payloads = fixture.files(**kwargs)
        if make_client:
            fixture.store(self.client, self.payloads, self.build if client_build is None else client_build)
        (self.case / 'authored-input-inventory.json').write_text(json.dumps({
            str(fid): {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
            for fid, data in sorted(self.payloads.items())}, sort_keys=True) + '\n')

    def invoke(self, command, code=0, error=None, parsed=True):
        self.calls += 1
        prefix = self.case / f'{self.calls:02}-{command}'
        options = ['--plan', str(self.plan)]
        if command == 'extract':
            options += ['--client', str(self.client), '--output', str(self.output)]
        elif command == 'verify':
            options += ['--output', str(self.output)]
        trace = prefix.with_suffix('.network')
        result = subprocess.run(['strace', '-f', '-qq', '-yy', '-e', 'trace=network',
                                 '-e', 'inject=network:error=ENETUNREACH', '-o', str(trace),
                                 BINARY, command, *options], capture_output=True, text=True, timeout=240)
        prefix.with_suffix('.stdout').write_text(result.stdout)
        prefix.with_suffix('.stderr').write_text(result.stderr)
        self.assertEqual(trace.read_text(), '')
        self.assertNotIn(str(self.root), result.stdout + result.stderr)
        self.assertEqual(result.returncode, code, result.stderr)
        self.assertEqual(result.stderr if code == 0 else result.stdout, '')
        report = json.loads(result.stdout if code == 0 else result.stderr)
        self.assertEqual(report['contract'], f'forever-product-v{self.version}' if parsed else 'forever-product-v1')
        self.assertIs(report['full_extractor_complete'], False)
        if error:
            self.assertEqual(report['code'], error)
            self.assertEqual(report['status'], 'failed')
        else:
            self.assertEqual(report['schema_catalog'], self.catalog)
        return report

    def roundtrip(self, profile, **kwargs):
        self.prepare(profile, **kwargs)
        inspected = self.invoke('inspect')
        self.assertEqual(inspected['status'], 'planned')
        self.assertEqual(inspected['build'], self.build)
        extracted = self.invoke('extract')
        verified = self.invoke('verify')
        self.assertEqual(extracted['status'], 'selected_producers_complete')
        self.assertEqual(verified['scope'], 'artifact_integrity_and_intrinsic_formats')
        self.assertEqual(extracted['package'], verified['package'])
        context = extracted['package']['context']
        self.assertEqual(context['build'], self.build)
        self.assertEqual(context['components'], 1)
        self.assertEqual(context['plan_sha256'], hashlib.sha256(self.plan.read_bytes()).hexdigest())
        self.assertEqual(context['producer_sha256'], hashlib.sha256(Path(BINARY).read_bytes()).hexdigest())
        inventory = json.loads((self.output / 'forever-package.json').read_text())['artifacts']
        expected = {}
        for name, metadata in fixture.TABLES.items():
            path = f'dbc/enUS/{name}.db2'
            data = self.payloads[metadata[0]]
            self.assertEqual((self.output / path).read_bytes(), data)
            expected[path] = (len(data), hashlib.sha256(data).hexdigest())
        self.assertEqual({a['path']: (a['bytes'], a['sha256']) for a in inventory}, expected)
        self.assertFalse((self.output / '.forever-package-incomplete').exists())

    def rejected_source(self, profile, error, **kwargs):
        self.prepare(profile, **kwargs)
        self.invoke('extract', code=9, error=error)
        self.assertTrue((self.output / '.forever-package-incomplete').is_file())
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.assertEqual(list(self.output.rglob('*.db2')), [])
        self.invoke('verify', code=8, error='package_incomplete')

    def test_v4_multisection_numeric_and_utf8_text_roundtrip(self):
        self.roundtrip(fixture.PROFILES[0])

    def test_v5_multisection_numeric_and_utf8_text_roundtrip(self):
        self.roundtrip(fixture.PROFILES[1])

    def test_empty_uncompressed_text_still_roundtrips_both_catalogs(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.roundtrip(profile, empty_text=True)

    def test_empty_compressed_text_cannot_publish_complete_package(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_source(profile, 'invalid_column_data', empty_text=True, text_compression=1)

    def test_nonempty_compressed_text_cannot_publish_complete_package(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_source(profile, 'invalid_column_data', text_compression=1)

    def test_table_hash_gate_rejects_both_catalogs(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_source(profile, 'unsupported_table_schema', bad_hash='table')

    def test_layout_hash_gate_rejects_both_catalogs(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_source(profile, 'unsupported_table_schema', bad_hash='layout')

    def rejected_plan(self, profile, before, after, error):
        self.prepare(profile, make_client=False)
        self.plan.write_text(self.plan.read_text().replace(before, after))
        self.invoke('extract', code=2, error=error, parsed=False)
        self.assertFalse(self.client.exists())
        self.assertFalse(self.output.exists())

    def test_catalog_identity_gate_before_source_or_output_io(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_plan(profile, profile[2], fixture.PROFILES[1 if profile[0] == 4 else 0][2],
                                   'product_plan_schema_catalog')

    def test_plan_build_gate_before_source_or_output_io(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_plan(profile, f'build\t{profile[1]}', 'build\t70170', 'product_plan_schema_catalog')

    def test_catalog_name_fdid_binding_before_source_or_output_io(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.rejected_plan(profile, 'AnimKitBoneSet\t1375433', 'AnimKitBoneSet\t1375434',
                                   'product_plan_schema')

    def test_client_build_mismatch_before_output_creation(self):
        for profile in fixture.PROFILES:
            with self.subTest(profile=profile):
                self.prepare(profile, client_build=70170)
                self.invoke('extract', code=9, error='product_build_mismatch')
                self.assertFalse(self.output.exists())

    def test_v5_non_db2_scope_before_source_or_output_io(self):
        self.rejected_plan(fixture.PROFILES[1], 'components\tdb2', 'components\tdb2,gt',
                           'product_plan_component_scope')


if __name__ == '__main__':
    unittest.main()
