"""Synthetic and gated parity checks for the independent WDC5 dump oracle.

Case (a) builds a minimal WDC5 file in memory and compares the oracle's dump
against an inline golden. Case (b) is gated on the environment variables below
and only reports equality or the first differing line index, never values:

* ``FOREVER_DB2_FILE``      -- local ``.db2`` path
* ``FOREVER_DB2_TABLE``     -- ``ChrClasses`` or ``ChrRaces``
* ``FOREVER_DB2_RUST_DUMP`` -- path to the Rust ``db2_dump`` output for it
"""
import dataclasses
from contextlib import redirect_stderr
import io
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
import db2_oracle
from db2_oracle import Field, Schema

HEADER_SIZE = db2_oracle.HEADER_SIZE
SECTION_SIZE = db2_oracle.SECTION_SIZE


def _put16(buf, at, value):
    buf[at : at + 2] = (value & 0xFFFF).to_bytes(2, "little")


def _put_i16(buf, at, value):
    buf[at : at + 2] = (value & 0xFFFF).to_bytes(2, "little")


def _put32(buf, at, value):
    buf[at : at + 4] = (value & 0xFFFFFFFF).to_bytes(4, "little")


def _write_bits(buf, bit, width, value):
    for index in range(width):
        pos = bit + index
        if (value >> index) & 1:
            buf[pos // 8] |= 1 << (pos % 8)


class Wdc5Builder:
    """Minimal synthetic WDC5 writer used only by these tests."""

    def __init__(
        self,
        schema,
        rows,
        *,
        min_id=0,
        max_id=100,
        ids=None,
        modes=None,
        common=None,
        pallet=None,
    ):
        self.schema = schema
        self.rows = rows
        self.min_id = min_id
        self.max_id = max_id
        self.ids = list(ids) if ids is not None else [0] * len(rows)
        self.modes = dict(modes or {})
        self.common = dict(common or {})
        self.pallet = dict(pallet or {})
        self.offsets = []
        bit = 0
        for field in schema.fields:
            self.offsets.append(bit)
            bit += field.bits * field.count
        self.packed_data_offset = (bit + 7) // 8
        self.cols = []
        for index, field in enumerate(schema.fields):
            mode, a, b, c = self.modes.get(index, (0, 0, 0, 0))
            if mode == 2:
                extra = 8 * len(self.common.get(index, (0, {}))[1])
            elif mode in (3, 4):
                extra = 4 * len(self.pallet[index])
            else:
                extra = 0
            self.cols.append((self.offsets[index], mode, extra, a, b, c))
        packed_bits = 0
        for _, mode, _, a, b, _ in self.cols:
            if mode in (1, 3, 4, 5):
                packed_bits = max(packed_bits, a + b)
        self.record_size = self.packed_data_offset + (packed_bits + 7) // 8

    def _records(self):
        string_positions = {}
        string_table = bytearray()
        for row_index, values in enumerate(self.rows):
            for field_index, field in enumerate(self.schema.fields):
                if field.kind != "string":
                    continue
                text = values.get(field_index, "")
                if text:
                    string_positions[(row_index, field_index)] = len(string_table)
                    string_table += text.encode("utf-8") + b"\x00"
        records = [bytearray(self.record_size) for _ in self.rows]
        for row_index, values in enumerate(self.rows):
            record = records[row_index]
            for field_index, field in enumerate(self.schema.fields):
                offset, mode, _, a, b, _ = self.cols[field_index]
                if field.kind == "string":
                    position = string_positions.get((row_index, field_index))
                    if position is not None:
                        stored = (
                            len(self.rows) * self.record_size
                            - row_index * self.record_size
                            - offset // 8
                            + position
                        )
                        record[offset // 8 : offset // 8 + 4] = (
                            stored & 0xFFFFFFFF
                        ).to_bytes(4, "little")
                    continue
                value = values.get(field_index)
                if mode == 0:
                    byte_width = field.bits // 8
                    mask = (1 << field.bits) - 1
                    if field.count > 1:
                        for array, item in enumerate(value or [0] * field.count):
                            at = offset // 8 + array * byte_width
                            record[at : at + byte_width] = (item & mask).to_bytes(
                                byte_width, "little"
                            )
                    elif value is not None:
                        at = offset // 8
                        record[at : at + byte_width] = (value & mask).to_bytes(
                            byte_width, "little"
                        )
                elif mode in (1, 3, 4, 5):
                    _write_bits(
                        record,
                        self.packed_data_offset * 8 + a,
                        b,
                        value if value is not None else 0,
                    )
        return records, string_table

    def build(self):
        records, string_table = self._records()
        field_count = len(self.schema.fields)
        out = bytearray(HEADER_SIZE)
        out[0:4] = b"WDC5"
        _put32(out, 4, 5)
        _put32(out, 136, len(self.rows))
        _put32(out, 140, field_count)
        _put32(out, 144, self.record_size)
        _put32(out, 148, len(string_table))
        _put32(out, 152, 0)
        _put32(out, 156, self.schema.layout_hash)
        _put32(out, 160, self.min_id)
        _put32(out, 164, self.max_id)
        _put32(out, 168, 0)
        _put32(out, 176, field_count)
        _put32(out, 180, self.packed_data_offset)
        _put32(out, 184, 0)
        _put32(out, 188, field_count * 24)
        _put32(
            out,
            192,
            sum(
                extra
                for _, mode, extra, _, _, _ in self.cols
                if mode == 2
            ),
        )
        _put32(
            out,
            196,
            sum(
                extra
                for _, mode, extra, _, _, _ in self.cols
                if mode in (3, 4)
            ),
        )
        _put32(out, 200, 1)
        _put_i16(out, 174, self.schema.index)
        section_at = len(out)
        out += bytearray(SECTION_SIZE)
        for offset, _, _, _, _, _ in self.cols:
            out += (0).to_bytes(2, "little") + (offset).to_bytes(2, "little")
        for offset, mode, extra, a, b, c in self.cols:
            entry = bytearray(24)
            _put16(entry, 0, offset)
            _put32(entry, 4, extra)
            _put32(entry, 8, mode)
            _put32(entry, 12, a)
            _put32(entry, 16, b)
            _put32(entry, 20, c)
            out += entry
        for mode in (3, 4):
            for field_index, (_, col_mode, _, _, _, _) in enumerate(self.cols):
                if col_mode != mode:
                    continue
                for value in self.pallet[field_index]:
                    out += (value & 0xFFFFFFFF).to_bytes(4, "little")
        for field_index, (_, col_mode, _, _, _, _) in enumerate(self.cols):
            if col_mode != 2:
                continue
            for record_id, value in self.common[field_index][1]:
                out += (record_id & 0xFFFFFFFF).to_bytes(4, "little")
                out += (value & 0xFFFFFFFF).to_bytes(4, "little")
        metadata_end = len(out)
        for record in records:
            out += record
        out += string_table
        if self.schema.index < 0:
            for record_id in self.ids:
                out += (record_id & 0xFFFFFFFF).to_bytes(4, "little")
        _put32(out, section_at + 8, metadata_end)
        _put32(out, section_at + 12, len(self.rows))
        _put32(out, section_at + 16, len(string_table))
        _put32(out, section_at + 20, 0)
        _put32(
            out,
            section_at + 24,
            len(self.ids) * 4 if self.schema.index < 0 else 0,
        )
        _put32(out, section_at + 28, 0)
        _put32(out, section_at + 32, 0)
        _put32(out, section_at + 36, 0)
        return bytes(out)


SYNTHETIC_FIELDS = (
    Field("string"),  # 0
    Field("unsigned", 32),  # 1 (inline index)
    Field("signed", 8),  # 2
    Field("signed", 16),  # 3
    Field("unsigned", 32, 3),  # 4 array of three
    Field("unsigned", 32),  # 5 common
    Field("unsigned", 32),  # 6 pallet
    Field("float", 32),  # 7
)

SYNTHETIC_INLINE = Schema(
    name="Synthetic", layout_hash=0x12345678, index=1, fields=SYNTHETIC_FIELDS
)
SYNTHETIC_IDTABLE = dataclasses.replace(
    SYNTHETIC_INLINE, name="SyntheticIdTable", index=-1
)

EXPECTED_INLINE = (
    "forever-db2-dump v1 Synthetic\n"
    "7\tt:a\\tb\\nc\\\\d\\re\tu:7\ti:-128\ti:-32768"
    "\tu:0,u:4294967295,u:42\tu:287454020\tu:2864434397\tf:0x3f800000\n"
)

EXPECTED_IDTABLE = (
    "forever-db2-dump v1 SyntheticIdTable\n"
    "5\tt:x\tu:100\ti:-1\ti:-2\tu:1,u:2,u:3\tu:11\tu:2864434397\tf:0x3f800000\n"
    "40\tt:y\tu:200\ti:1\ti:2\tu:4,u:5,u:6\tu:22\tu:287454020\tf:0x40000000\n"
)


def _inline_file():
    rows = [
        {
            0: "a\tb\nc\\d\re",
            1: 7,
            2: 0x80,
            3: 0x8000,
            4: [0, 0xFFFFFFFF, 42],
            6: 0,
            7: 0x3F800000,
        }
    ]
    builder = Wdc5Builder(
        SYNTHETIC_INLINE,
        rows,
        modes={5: (2, 0, 0, 0), 6: (3, 0, 8, 0)},
        common={5: (0, [(7, 0x11223344)])},
        pallet={6: [0xAABBCCDD]},
    )
    return builder.build()


def _idtable_file():
    rows = [
        {0: "x", 1: 100, 2: 0xFF, 3: 0xFFFE, 4: [1, 2, 3], 6: 0, 7: 0x3F800000},
        {0: "y", 1: 200, 2: 0x01, 3: 0x0002, 4: [4, 5, 6], 6: 1, 7: 0x40000000},
    ]
    builder = Wdc5Builder(
        SYNTHETIC_IDTABLE,
        rows,
        min_id=5,
        max_id=100,
        ids=[3, 40],
        modes={5: (2, 0, 0, 0), 6: (3, 0, 8, 0)},
        common={5: (0, [(5, 11), (40, 22)])},
        pallet={6: [0xAABBCCDD, 0x11223344]},
    )
    return builder.build()


def _chr_class_ui_chr_model_info_file():
    """One synthetic row of the reviewed id-table ``ChrClassUIChrModelInfo``.

    Schema: Float(3), Signed(32,1), Signed(8,1), Float(1), Signed(32,1), with
    IndexField -1 so the record id comes from the separate u32 id table
    (DB2Metadata.h:3390, layout 0x26E5E322).
    """
    rows = [
        {
            0: [0x3F800000, 0x40000000, 0x40400000],
            1: 0xFFFFFFFE,
            2: 0x7F,
            3: 0x3F000000,
            4: 0x00000005,
        }
    ]
    builder = Wdc5Builder(
        db2_oracle.CHR_CLASS_UI_CHR_MODEL_INFO,
        rows,
        min_id=0,
        max_id=100,
        ids=[9],
    )
    return builder.build()


EXPECTED_CHR_CLASS_UI_CHR_MODEL_INFO = (
    "forever-db2-dump v1 ChrClassUIChrModelInfo\n"
    "9\tf:0x3f800000,f:0x40000000,f:0x40400000"
    "\ti:-2\ti:127\tf:0x3f000000\ti:5\n"
)


def _zero_section_file(schema, record_count):
    """A header-only WDC5 file with ``section_count == 0`` (204 bytes).

    DB2FileLoader.cpp:1778-1779 admits a zero-section header and
    DB2FileLoader.cpp:1877 assigns an empty record map, so only
    ``record_count == 0`` is a consistent body.
    """
    out = bytearray(HEADER_SIZE)
    out[0:4] = b"WDC5"
    _put32(out, 4, 5)
    _put32(out, 136, record_count)
    _put32(out, 140, len(schema.fields))
    _put32(out, 144, 8)
    _put32(out, 152, 0)
    _put32(out, 156, schema.layout_hash)
    _put32(out, 160, 0)
    _put32(out, 164, 100)
    _put_i16(out, 174, schema.index)
    _put32(out, 176, len(schema.fields))
    _put32(out, 188, len(schema.fields) * 24)
    _put32(out, 200, 0)
    return bytes(out)


class SyntheticOracle(unittest.TestCase):
    def test_inline_index_dump_matches_golden(self):
        data = _inline_file()
        rows = db2_oracle.decode(data, SYNTHETIC_INLINE)
        self.assertEqual(db2_oracle.render(rows, SYNTHETIC_INLINE.name), EXPECTED_INLINE)

    def test_id_table_dump_matches_golden(self):
        data = _idtable_file()
        rows = db2_oracle.decode(data, SYNTHETIC_IDTABLE)
        self.assertEqual(
            db2_oracle.render(rows, SYNTHETIC_IDTABLE.name), EXPECTED_IDTABLE
        )

    def test_reviewed_chr_class_ui_chr_model_info_dump_matches_golden(self):
        # An id-table (IndexField -1) reviewed `ChrClass*` schema: the record id
        # comes from the separate u32 id table, not an inline index field.
        data = _chr_class_ui_chr_model_info_file()
        rows = db2_oracle.decode(data, db2_oracle.CHR_CLASS_UI_CHR_MODEL_INFO)
        self.assertEqual(
            db2_oracle.render(rows, db2_oracle.CHR_CLASS_UI_CHR_MODEL_INFO.name),
            EXPECTED_CHR_CLASS_UI_CHR_MODEL_INFO,
        )

    def test_reviewed_chr_class_transcriptions_are_present(self):
        title = db2_oracle.SCHEMAS["ChrClassTitle"]
        model = db2_oracle.SCHEMAS["ChrClassUIChrModelInfo"]
        self.assertEqual(title.layout_hash, 0xC086F95D)
        self.assertEqual(title.index, -1)
        self.assertEqual(len(title.fields), 3)
        self.assertEqual((title.fields[0].kind, title.fields[0].bits), ("string", 32))
        self.assertEqual((title.fields[1].kind, title.fields[1].bits), ("string", 32))
        self.assertEqual((title.fields[2].kind, title.fields[2].bits, title.fields[2].signed), ("signed", 8, True))
        self.assertEqual(model.layout_hash, 0x26E5E322)
        self.assertEqual(model.index, -1)
        self.assertEqual(len(model.fields), 5)
        self.assertEqual((model.fields[0].kind, model.fields[0].count), ("float", 3))
        self.assertEqual((model.fields[1].kind, model.fields[1].bits, model.fields[1].signed), ("signed", 32, True))
        self.assertEqual((model.fields[2].kind, model.fields[2].bits, model.fields[2].signed), ("signed", 8, True))
        self.assertEqual((model.fields[3].kind, model.fields[3].count), ("float", 1))
        self.assertEqual((model.fields[4].kind, model.fields[4].bits, model.fields[4].signed), ("signed", 32, True))

    def test_reviewed_schema_transcription(self):
        classes = db2_oracle.SCHEMAS["ChrClasses"]
        races = db2_oracle.SCHEMAS["ChrRaces"]
        self.assertEqual(classes.layout_hash, 0xAFC9B0C2)
        self.assertEqual(classes.index, 29)
        self.assertEqual(len(classes.fields), 43)
        self.assertEqual(classes.fields[27].bits, 16)
        self.assertEqual(classes.fields[28].bits, 16)
        self.assertEqual((classes.fields[29].kind, classes.fields[29].bits), ("unsigned", 8))
        self.assertEqual(classes.fields[31].kind, "signed")
        self.assertEqual(races.layout_hash, 0x4F44C796)
        self.assertEqual(races.index, -1)
        self.assertEqual(len(races.fields), 51)
        self.assertEqual(races.fields[23].count, 3)
        self.assertEqual(races.fields[24].count, 3)
        self.assertEqual(races.fields[31].kind, "float")
        self.assertEqual(races.fields[31].count, 1)

    def test_zero_sections_with_zero_records_dumps_header_only(self):
        schema = db2_oracle.CHR_CLASS_UI_CHR_MODEL_INFO
        data = _zero_section_file(schema, 0)
        rows = db2_oracle.decode(data, schema)
        self.assertEqual(rows, {})
        self.assertEqual(
            db2_oracle.render(rows, schema.name),
            "forever-db2-dump v1 ChrClassUIChrModelInfo\n",
        )

    def test_zero_sections_with_declared_records_is_rejected(self):
        schema = db2_oracle.CHR_CLASS_UI_CHR_MODEL_INFO
        data = _zero_section_file(schema, 4)
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, "unsupported_record_variant")

    def test_out_of_scope_variants_are_rejected(self):
        base = _inline_file()
        mutations = [
            ("sparse flag", 172, 2, 1, "unsupported_record_variant"),
            ("parent lookup", 184, 4, 1, "unsupported_record_variant"),
            ("nonzero tact", 204, 1, 1, "unsupported_record_variant"),
            ("catalog count", 204 + 32, 4, 1, "unsupported_record_variant"),
        ]
        for name, at, size, value, code in mutations:
            with self.subTest(name=name):
                data = bytearray(base)
                if size == 1:
                    data[at] = value
                elif size == 2:
                    _put16(data, at, value)
                else:
                    _put32(data, at, value)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), SYNTHETIC_INLINE)
                self.assertEqual(caught.exception.code, code)

    def test_main_reports_stable_codes(self):
        with tempfile.TemporaryDirectory() as directory:
            bad_signature = Path(directory) / "bad-signature.db2"
            bad_signature.write_bytes(b"WDC4" + bytes(300))
            error = io.StringIO()
            with redirect_stderr(error):
                code = db2_oracle.main(
                    ["--file", str(bad_signature), "--table", "ChrClasses"]
                )
            self.assertEqual(code, 3)
            self.assertEqual(error.getvalue().strip(), "unsupported_signature")

            mismatched = Path(directory) / "synthetic.db2"
            mismatched.write_bytes(_inline_file())
            error = io.StringIO()
            with redirect_stderr(error):
                code = db2_oracle.main(
                    ["--file", str(mismatched), "--table", "ChrClasses"]
                )
            self.assertEqual(code, 4)
            self.assertEqual(error.getvalue().strip(), "unsupported_table_schema")

            missing = Path(directory) / "missing.db2"
            error = io.StringIO()
            with redirect_stderr(error):
                code = db2_oracle.main(["--file", str(missing), "--table", "ChrClasses"])
            self.assertEqual(code, 4)
            self.assertEqual(error.getvalue().strip(), "input_io")

        for args in ([], ["--file", "x"], ["--file", "x", "--file", "y", "--table", "ChrClasses"]):
            with self.subTest(args=args):
                error = io.StringIO()
                with redirect_stderr(error):
                    code = db2_oracle.main(args)
                self.assertEqual(code, 2)
                self.assertEqual(error.getvalue().strip(), "invalid_invocation")


class RustDumpParity(unittest.TestCase):
    def test_oracle_matches_rust_dump(self):
        file_path = os.environ.get("FOREVER_DB2_FILE")
        table = os.environ.get("FOREVER_DB2_TABLE")
        dump_path = os.environ.get("FOREVER_DB2_RUST_DUMP")
        if not (file_path and table and dump_path):
            self.skipTest("FOREVER_DB2_FILE/TABLE/RUST_DUMP not configured")
        try:
            expected = Path(dump_path).read_text(encoding="utf-8")
        except OSError:
            self.skipTest("Rust dump path is not readable")
        actual = db2_oracle.dump_file(file_path, table)
        if actual == expected:
            return
        actual_lines = actual.split("\n")
        expected_lines = expected.split("\n")
        for index in range(max(len(actual_lines), len(expected_lines))):
            actual_line = actual_lines[index] if index < len(actual_lines) else None
            expected_line = expected_lines[index] if index < len(expected_lines) else None
            if actual_line != expected_line:
                self.fail("oracle differs from Rust dump at line index %d" % index)
        self.fail("oracle differs from Rust dump")


if __name__ == "__main__":
    unittest.main()
