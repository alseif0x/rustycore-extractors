# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original purely synthetic PE-layout fixtures, not loader evidence.
import hashlib
import struct
import unittest
import importlib.util
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("forever_windows_seal", ROOT / "tools/seal_forever_windows.py")
p = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = p
SPEC.loader.exec_module(p)

PE = 128
OPT = PE + 24
TABLE = OPT + 240
BASE = 0x140000000
RAW = [1024, 1536, 2048, 2560]
RVA = [0x1000, 0x2000, 0x3000, 0x4000]


def put(data, offset, value, width=4):
    data[offset:offset + width] = value.to_bytes(width, 'little')


def directory(data, index, rva, size):
    put(data, OPT + 112 + index * 8, rva)
    put(data, OPT + 116 + index * 8, size)


def fixture():
    data = bytearray(3072)
    data[:2] = b'MZ'
    put(data, 0x3c, PE)
    data[PE:PE + 4] = b'PE\0\0'
    put(data, PE + 4, 0x8664, 2)
    put(data, PE + 6, 4, 2)
    put(data, PE + 20, 240, 2)
    put(data, PE + 22, 0x22, 2)
    put(data, OPT, 0x20b, 2)
    put(data, OPT + 24, BASE, 8)
    put(data, OPT + 32, 4096)
    put(data, OPT + 36, 512)
    put(data, OPT + 56, 0x5000)
    put(data, OPT + 60, 1024)
    put(data, OPT + 68, 3, 2)
    put(data, OPT + 108, 16)
    for i, (name, virtual, flags) in enumerate([
            (b'.text', 128, 0x60000020), (b'.rceid', 64, 0x40000040),
            (b'.rdata', 512, 0xc0000040), (b'.reloc', 64, 0x42000040)]):
        off = TABLE + i * 40
        data[off:off + 8] = name.ljust(8, b'\0')
        for delta, value in [(8, virtual), (12, RVA[i]), (16, 512), (20, RAW[i]), (36, flags)]:
            put(data, off + delta, value)
    data[RAW[0]:RAW[0] + 4] = b'CODE'
    data[RAW[1]:RAW[1] + 32] = p.MARKER
    return data


def with_relocation(target=0x1020, kind=10):
    data = fixture()
    directory(data, 5, RVA[3], 12)
    put(data, RAW[3], target & ~4095)
    put(data, RAW[3] + 4, 12)
    put(data, RAW[3] + 8, (kind << 12) | (target & 4095), 2)
    return data


def with_tls(index_rva=0x3080, callback_rva=0x1000):
    data = fixture()
    directory(data, 9, RVA[2], 40)
    put(data, RAW[2] + 16, BASE + index_rva, 8)
    put(data, RAW[2] + 24, BASE + 0x30a0, 8)
    put(data, RAW[2] + 0xa0, BASE + callback_rva, 8)
    return data


def with_import(target=0x30a0):
    data = fixture()
    directory(data, 1, RVA[2], 40)
    put(data, RAW[2], 0x3080)
    put(data, RAW[2] + 12, 0x30c0)
    put(data, RAW[2] + 16, target)
    put(data, RAW[2] + 0x80, 0x8000000000000005, 8)
    put(data, RAW[2] + 0xa0, 0x8000000000000005, 8)
    data[RAW[2] + 0xc0:RAW[2] + 0xc8] = b'toy.dll\0'
    return data


class SealTests(unittest.TestCase):
    def reject(self, data, code=None):
        with self.assertRaises(p.Reject) as caught:
            p.locate(bytes(data))
        if code:
            self.assertEqual(str(caught.exception), code)

    def test_seal_full_hash_and_canonical_digest_are_distinct(self):
        source = bytes(fixture())
        sealed = p.seal(source)
        layout = p.locate(sealed)
        loaded = sealed[layout.slot - 32:layout.slot + 32]
        self.assertEqual(p.verify(sealed, loaded, RVA[1]), hashlib.sha256(sealed).digest())
        self.assertEqual(loaded[32:], hashlib.sha256(source).digest())
        self.assertNotEqual(p.verify(sealed, loaded, RVA[1]), loaded[32:])
        self.assertEqual(source[:layout.slot], sealed[:layout.slot])
        self.assertEqual(source[layout.slot + 32:], sealed[layout.slot + 32:])

    def test_disk_replacement_with_valid_different_seal_fails_loaded_binding(self):
        original = p.seal(bytes(fixture()))
        layout = p.locate(original)
        loaded = original[layout.slot - 32:layout.slot + 32]
        other = fixture()
        other[RAW[0]] ^= 1
        replacement = p.seal(bytes(other))
        with self.assertRaisesRegex(p.Reject, '^pe_loaded_mismatch$'):
            p.verify(replacement, loaded, RVA[1])
        self.assertEqual(p.verify(bytes(original), loaded, RVA[1]), hashlib.sha256(original).digest())

    def test_tampered_body_same_disk_and_loaded_slot_rejects(self):
        data = p.seal(bytes(fixture()))
        layout = p.locate(data)
        loaded = data[layout.slot - 32:layout.slot + 32]
        altered = bytearray(data)
        altered[RAW[0]] ^= 1
        with self.assertRaisesRegex(p.Reject, '^pe_digest_mismatch$'):
            p.verify(bytes(altered), loaded, RVA[1])

    def test_unsealed_loaded_literal_cannot_fall_back_to_disk(self):
        with self.assertRaisesRegex(p.Reject, '^pe_loaded_seal$'):
            p.verify(p.seal(bytes(fixture())), p.MARKER + bytes(32), RVA[1])

    def test_no_silent_resealing(self):
        with self.assertRaisesRegex(p.Reject, '^pe_already_sealed$'):
            p.seal(p.seal(bytes(fixture())))

    def test_overlay_is_hashed_without_authenticode_exclusions(self):
        data = p.seal(bytes(fixture()) + b'authored overlay')
        layout = p.locate(data)
        with self.assertRaisesRegex(p.Reject, '^pe_digest_mismatch$'):
            p.verify(data + b'x', data[layout.slot - 32:layout.slot + 32], RVA[1])

    def test_raw_offset_is_not_loaded_rva(self):
        layout = p.locate(bytes(fixture()))
        self.assertEqual(layout.slot, RAW[1] + 32)
        self.assertEqual(layout.slot_rva, RVA[1] + 32)
        self.assertNotEqual(layout.slot, layout.slot_rva)

    def test_marker_elsewhere_does_not_create_ambiguous_slot(self):
        data = fixture()
        data[RAW[0]:RAW[0] + 32] = p.MARKER
        p.seal(bytes(data))  # Selection is exact section/header, not global search.

    def test_duplicate_named_sections_reject(self):
        data = fixture()
        data[TABLE:TABLE + 8] = p.SECTION
        self.reject(data, 'pe_seal_count')

    def test_virtual_only_seal_rejects(self):
        data = fixture()
        put(data, TABLE + 40 + 16, 0)
        put(data, TABLE + 40 + 20, 0)
        self.reject(data, 'pe_seal_initialized')

    def test_zero_or_wrong_payload_size_rejects(self):
        for size in [0, 31, 63, 65, 512]:
            data = fixture()
            put(data, TABLE + 40 + 8, size)
            self.reject(data)

    def test_nonzero_padding_or_second_marker_rejects(self):
        data = fixture()
        data[RAW[1] + 64:RAW[1] + 96] = p.MARKER
        self.reject(data, 'pe_seal_padding')

    def test_writable_executable_discardable_and_shared_seals_reject(self):
        for flag in [0x80000000, 0x20000000, 0x02000000, 0x10000000, 0x80]:
            data = fixture()
            put(data, TABLE + 40 + 36, 0x40000040 | flag)
            self.reject(data, 'pe_seal_flags')

    def test_raw_and_virtual_overlaps_reject(self):
        data = fixture()
        put(data, TABLE + 40 + 20, RAW[0])
        self.reject(data, 'pe_raw_overlap')
        data = fixture()
        put(data, TABLE + 40 + 12, RVA[0])
        self.reject(data, 'pe_virtual_overlap')

    def test_truncated_raw_data_rejects(self):
        self.reject(fixture()[:-1], 'pe_raw_bounds')

    def test_header_alias_rejects(self):
        data = fixture()
        put(data, TABLE + 40 + 20, 512)
        self.reject(data, 'pe_raw_overlap')

    def test_checksum_must_be_explicitly_zeroed_before_seal(self):
        data = fixture()
        put(data, OPT + 64, 123)
        self.reject(data, 'pe_checksum_nonzero')
        prepared = p.prepare_unsigned(bytes(data))
        self.assertEqual(prepared[OPT + 64:OPT + 68], bytes(4))
        p.seal(prepared)

    def test_certificate_and_unsupported_loader_directories_reject(self):
        for index in [4, 7, 8, 10, 11, 13, 14, 15]:
            data = fixture()
            directory(data, index, RVA[2], 16)
            self.reject(data, 'pe_unsupported_directory')

    def test_valid_nonseal_dir64_relocation_and_padding(self):
        p.seal(bytes(with_relocation()))

    def test_relocation_into_marker_or_digest_rejects(self):
        for target in [RVA[1], RVA[1] + 32, RVA[1] + 56]:
            self.reject(with_relocation(target), 'pe_loader_writes_seal')

    def test_unknown_relocation_kind_rejects(self):
        self.reject(with_relocation(kind=3), 'pe_relocation_kind')

    def test_malformed_relocation_block_rejects(self):
        data = with_relocation()
        put(data, RAW[3] + 4, 11)
        self.reject(data, 'pe_relocation_block')

    def test_tls_index_and_executable_callback_positive(self):
        p.seal(bytes(with_tls()))

    def test_tls_index_into_seal_rejects(self):
        self.reject(with_tls(index_rva=RVA[1] + 32), 'pe_loader_writes_seal')

    def test_tls_callback_into_seal_rejects(self):
        self.reject(with_tls(callback_rva=RVA[1]), 'pe_tls_callback_code')

    def test_ordinal_import_positive(self):
        p.seal(bytes(with_import()))

    def test_import_thunk_write_into_seal_rejects(self):
        self.reject(with_import(target=RVA[1]), 'pe_loader_writes_seal')

    def test_data_directory_alias_seal_rejects(self):
        data = fixture()
        directory(data, 3, RVA[1], 8)
        self.reject(data, 'pe_loader_writes_seal')

    def test_machine_optional_header_and_section_count_reject(self):
        for off, value, width in [(PE + 4, 0x14c, 2), (OPT, 0x10b, 2), (PE + 6, 0, 2),
                                  (PE + 6, 97, 2), (PE + 20, 224, 2)]:
            data = fixture()
            put(data, off, value, width)
            self.reject(data)


class CliTests(unittest.TestCase):
    def invoke(self, *args):
        import subprocess
        return subprocess.run([sys.executable, str(ROOT / 'tools/seal_forever_windows.py'), *map(str, args)],
                              capture_output=True, text=True, timeout=20)

    def test_explicit_new_output_and_supplied_seal_verification(self):
        import json
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source with spaces.exe'
            output = pathlib.Path(root) / 'sealed.exe'
            original = bytes(fixture())
            source.write_bytes(original)
            result = self.invoke('seal', '--input', source, '--output', output)
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual(source.read_bytes(), original)
            data = output.read_bytes()
            layout = p.locate(data)
            loaded = data[layout.slot - 32:layout.slot + 32]
            result = self.invoke('verify', '--input', output, '--loaded-payload', loaded.hex(), '--payload-rva', hex(RVA[1]))
            self.assertEqual(result.returncode, 0, result.stdout)
            report = json.loads(result.stdout)
            self.assertEqual(report['file_sha256'], hashlib.sha256(data).hexdigest())
            self.assertEqual(report['scope'], 'artifact_integrity_with_supplied_seal')
            self.assertFalse(report['publisher_authenticated'])
            self.assertNotIn(str(output), result.stdout)

    def test_existing_output_and_inplace_are_refused(self):
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source.exe'
            source.write_bytes(bytes(fixture()))
            result = self.invoke('seal', '--input', source, '--output', source)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(source.read_bytes(), bytes(fixture()))
            output = pathlib.Path(root) / 'existing.exe'
            output.write_bytes(b'protected existing output')
            result = self.invoke('seal', '--input', source, '--output', output)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(output.read_bytes(), b'protected existing output')

    def test_checksum_preparation_requires_explicit_flag(self):
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source.exe'
            output = pathlib.Path(root) / 'sealed.exe'
            data = fixture(); put(data, OPT + 64, 123)
            source.write_bytes(data)
            result = self.invoke('seal', '--input', source, '--output', output)
            self.assertEqual(result.returncode, 2)
            self.assertIn('pe_checksum_nonzero', result.stdout)
            self.assertFalse(output.exists())
            result = self.invoke('seal', '--input', source, '--output', output, '--zero-checksum')
            self.assertEqual(result.returncode, 0, result.stdout)
            self.assertEqual(source.read_bytes(), data)
            self.assertEqual(output.read_bytes()[OPT + 64:OPT + 68], bytes(4))

    def test_loaded_rva_and_unsealed_supplied_value_reject(self):
        data = p.seal(bytes(fixture()))
        layout = p.locate(data)
        loaded = data[layout.slot - 32:layout.slot + 32]
        for rva in (0, RAW[1], RVA[1] + 32, 2**32 - 1):
            with self.assertRaisesRegex(p.Reject, '^pe_loaded_rva$'):
                p.verify(data, loaded, rva)
        for supplied in (p.MARKER + bytes(32), bytes(64)):
            with self.assertRaisesRegex(p.Reject, '^pe_loaded_seal$'):
                p.verify(data, supplied, RVA[1])

    def test_cli_verify_rejects_bad_payload_rva_and_body(self):
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'sealed.exe'
            data = p.seal(bytes(fixture()))
            layout = p.locate(data)
            loaded = data[layout.slot - 32:layout.slot + 32].hex()
            source.write_bytes(data)
            for value, rva, code in [('00', RVA[1], 'pe_loaded_payload'), ('z' * 128, RVA[1], 'pe_loaded_payload'), (loaded, RVA[1] + 32, 'pe_loaded_rva')]:
                result = self.invoke('verify', '--input', source, '--loaded-payload', value, '--payload-rva', rva)
                self.assertEqual(result.returncode, 2)
                self.assertIn(code, result.stdout)
            changed = bytearray(data); changed[RAW[0]] ^= 1; source.write_bytes(changed)
            result = self.invoke('verify', '--input', source, '--loaded-payload', loaded, '--payload-rva', RVA[1])
            self.assertEqual(result.returncode, 2)
            self.assertIn('pe_digest_mismatch', result.stdout)

    def test_cli_input_directory_and_symlink_reject(self):
        import os
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            output = pathlib.Path(root) / 'out.exe'
            result = self.invoke('seal', '--input', root, '--output', output)
            self.assertEqual(result.returncode, 2)
            self.assertFalse(output.exists())
            if os.name == 'posix':
                source = pathlib.Path(root) / 'input.exe'
                source.write_bytes(bytes(fixture()))
                link = pathlib.Path(root) / 'link.exe'; link.symlink_to(source)
                result = self.invoke('seal', '--input', link, '--output', output)
                self.assertEqual(result.returncode, 2)
                self.assertFalse(output.exists())

    def test_failed_output_is_retained_without_path_cleanup(self):
        import contextlib
        import io
        import json
        import tempfile
        from unittest import mock
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'input.exe'
            output = pathlib.Path(root) / 'partial.exe'
            source.write_bytes(bytes(fixture()))
            log = io.StringIO()
            with mock.patch('os.fsync', side_effect=OSError('private failing path')), contextlib.redirect_stdout(log):
                self.assertEqual(p.main(['seal', '--input', str(source), '--output', str(output)]), 2)
            self.assertTrue(output.exists())
            self.assertEqual(output.read_bytes(), p.seal(bytes(fixture())))
            self.assertEqual(json.loads(log.getvalue())['code'], 'pe_file_io')
            self.assertNotIn('private failing path', log.getvalue())

    @unittest.skipUnless(sys.platform != 'win32', 'replacing an open test output requires Unix semantics')
    def test_failure_never_unlinks_another_actors_replacement(self):
        import contextlib
        import io
        import tempfile
        from unittest import mock
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'input.exe'
            output = pathlib.Path(root) / 'partial.exe'
            retained = pathlib.Path(root) / 'retained-created-file.exe'
            source.write_bytes(bytes(fixture()))
            def replace_then_fail(_descriptor):
                output.rename(retained)
                output.write_bytes(b'protected concurrent replacement')
                raise OSError('private failure after replacement')
            log = io.StringIO()
            with mock.patch('os.fsync', side_effect=replace_then_fail), contextlib.redirect_stdout(log):
                self.assertEqual(p.main(['seal', '--input', str(source), '--output', str(output)]), 2)
            self.assertEqual(output.read_bytes(), b'protected concurrent replacement')
            self.assertEqual(retained.read_bytes(), p.seal(bytes(fixture())))
            self.assertNotIn('private failure', log.getvalue())


class InputIdentityTests(unittest.TestCase):
    """Authored 2026-10-13: portable fault injection, plus a native-only read.

    Mock snapshots do not establish Windows execution; the native CLI suite
    executes the real CreateFile/GetFileInformationByHandle path separately.
    """
    def identity(self):
        return p._WindowsInputIdentity(1, 0x20, 7, 11, len(fixture()), 123, 456, 1)

    def read_snapshots(self, named=None, held=None):
        import tempfile
        from unittest import mock
        original = bytes(fixture())
        identity = self.identity()
        api = mock.Mock()
        api.named.side_effect = [identity, identity] if named is None else named
        api.held.side_effect = [identity, identity] if held is None else held
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source with spaces.exe'
            source.write_bytes(original)
            with mock.patch.object(p, '_WindowsInputAPI', return_value=api):
                result = p._read_file_windows(source)
        self.assertEqual(result, original)
        self.assertEqual(api.named.call_count, 2)
        self.assertEqual(api.held.call_count, 2)

    def test_windows_path_and_held_native_snapshots_read_complete_exe(self):
        self.read_snapshots()

    def test_windows_initial_binding_retains_every_native_identity_field(self):
        from dataclasses import fields, replace
        identity = self.identity()
        for field in fields(identity):
            with self.subTest(field=field.name):
                changed = replace(identity, **{field.name: getattr(identity, field.name) + 1})
                with self.assertRaisesRegex(p.Reject, '^pe_input_changed$'):
                    self.read_snapshots(held=[changed])

    def test_windows_held_postflight_retains_every_native_identity_field(self):
        from dataclasses import fields, replace
        identity = self.identity()
        for field in fields(identity):
            with self.subTest(field=field.name):
                changed = replace(identity, **{field.name: getattr(identity, field.name) + 1})
                with self.assertRaisesRegex(p.Reject, '^pe_input_changed$'):
                    self.read_snapshots(held=[identity, changed])

    def test_windows_named_postflight_retains_every_native_identity_field(self):
        from dataclasses import fields, replace
        identity = self.identity()
        for field in fields(identity):
            with self.subTest(field=field.name):
                changed = replace(identity, **{field.name: getattr(identity, field.name) + 1})
                with self.assertRaisesRegex(p.Reject, '^pe_input_changed$'):
                    self.read_snapshots(named=[identity, changed])

    def test_windows_reparse_directory_and_nondisk_reject_before_data_open(self):
        from dataclasses import replace
        from unittest import mock
        identity = self.identity()
        for changed in (replace(identity, attributes=0x400),
                        replace(identity, attributes=0x10),
                        replace(identity, file_type=3)):
            with self.subTest(identity=changed):
                api = mock.Mock()
                api.named.return_value = changed
                with mock.patch.object(p, '_WindowsInputAPI', return_value=api), \
                     mock.patch('os.open', side_effect=AssertionError('data open forbidden')):
                    with self.assertRaisesRegex(p.Reject, '^pe_input_type$'):
                        p._read_file_windows('authored-missing.exe')
                api.held.assert_not_called()

    def test_windows_size_limit_rejects_before_data_open(self):
        from dataclasses import replace
        from unittest import mock
        api = mock.Mock()
        api.named.return_value = replace(self.identity(), size=p.MAX_FILE + 1)
        with mock.patch.object(p, '_WindowsInputAPI', return_value=api), \
             mock.patch('os.open', side_effect=AssertionError('data open forbidden')):
            with self.assertRaisesRegex(p.Reject, '^pe_limit$'):
                p._read_file_windows('authored-missing.exe')
        api.held.assert_not_called()

    def test_windows_short_read_and_extra_byte_reject(self):
        from dataclasses import replace
        for adjustment in (-1, 1):
            identity = replace(self.identity(), size=len(fixture()) + adjustment)
            with self.subTest(adjustment=adjustment):
                with self.assertRaisesRegex(p.Reject, '^pe_input_changed$'):
                    self.read_snapshots(named=[identity, identity], held=[identity, identity])

    def test_posix_execute_bits_still_participate_in_initial_identity(self):
        import tempfile
        from types import SimpleNamespace
        from unittest import mock
        before = SimpleNamespace(st_dev=7, st_ino=11, st_mode=0o100700,
                                 st_size=len(fixture()), st_mtime_ns=123, st_ctime_ns=456)
        held = SimpleNamespace(**dict(vars(before), st_mode=0o100600))
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source.exe'
            source.write_bytes(bytes(fixture()))
            with mock.patch('os.name', 'posix'), mock.patch('os.lstat', return_value=before), \
                 mock.patch('os.fstat', return_value=held):
                with self.assertRaisesRegex(p.Reject, '^pe_input_changed$'):
                    p._read_file(source)

    @unittest.skipUnless(sys.platform == 'win32', 'real Win32 input snapshots require Windows')
    def test_native_windows_exe_snapshots_agree_while_data_reader_is_open(self):
        import os
        import tempfile
        with tempfile.TemporaryDirectory() as root:
            source = pathlib.Path(root) / 'source with spaces.exe'
            original = bytes(fixture())
            source.write_bytes(original)
            native = p._WindowsInputAPI()
            before = native.named(source)
            with source.open('rb') as stream:
                self.assertEqual(native.held(stream.fileno()), before)
                self.assertEqual(native.named(source), before)
                self.assertEqual(stream.read(), original)
                self.assertEqual(native.held(stream.fileno()), before)
                self.assertEqual(native.named(source), before)
            self.assertEqual(p._read_file(source), original)
            self.assertEqual(source.read_bytes(), original)
            self.assertEqual(os.lstat(source).st_size, len(original))


class LoaderControlTests(unittest.TestCase):
    def test_relocation_cannot_rewrite_a_later_relocation_block(self):
        data = with_relocation(target=0x400c)
        directory(data, 5, RVA[3], 24)
        put(data, RAW[3] + 12, 0x1000)
        put(data, RAW[3] + 16, 12)
        put(data, RAW[3] + 20, 0xa020, 2)
        with self.assertRaisesRegex(p.Reject, '^pe_loader_control_write$'):
            p.locate(bytes(data))

    def test_relocation_cannot_rewrite_import_descriptors(self):
        data = with_import()
        r = with_relocation(target=RVA[2])
        directory(data, 5, RVA[3], 12)
        data[RAW[3]:RAW[3] + 12] = r[RAW[3]:RAW[3] + 12]
        with self.assertRaisesRegex(p.Reject, '^pe_loader_control_write$'):
            p.locate(bytes(data))

    def test_iat_cannot_rewrite_import_descriptors_or_relocations(self):
        with self.assertRaisesRegex(p.Reject, '^pe_loader_control_write$'):
            p.locate(bytes(with_import(target=RVA[2])))
        data = with_import(target=RVA[3])
        r = with_relocation()
        directory(data, 5, RVA[3], 12)
        data[RAW[3]:RAW[3] + 12] = r[RAW[3]:RAW[3] + 12]
        with self.assertRaisesRegex(p.Reject, '^pe_loader_control_write$'):
            p.locate(bytes(data))

    def test_original_first_thunk_zero_admits_normal_in_place_resolution(self):
        data = with_import(); put(data, RAW[2], 0)
        data = p.seal(bytes(data)); layout = p.locate(data)
        self.assertEqual(p.verify(data, data[layout.slot - 32:layout.slot + 32], RVA[1]), hashlib.sha256(data).digest())


if __name__ == '__main__':
    unittest.main()
