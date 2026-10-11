"""Synthetic and gated parity checks for the independent WDC5 dump oracle.

Case (a) builds a minimal WDC5 file in memory and compares the oracle's dump
against an inline golden. Case (b) is gated on the environment variables below
and only reports equality or the first differing line index, never values:

* ``FOREVER_DB2_FILE``      -- local ``.db2`` path
* ``FOREVER_DB2_TABLE``     -- any reviewed table name in ``db2_oracle.SCHEMAS``
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
from unittest import mock

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
        copies=(),
        parents=None,
    ):
        self.schema = schema
        self.rows = rows
        self.min_id = min_id
        self.max_id = max_id
        self.ids = list(ids) if ids is not None else [0] * len(rows)
        self.modes = dict(modes or {})
        self.common = dict(common or {})
        self.pallet = dict(pallet or {})
        self.copies = list(copies)
        self.parents = list(parents) if parents is not None else None
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
                texts = values.get(field_index, "" if field.count == 1 else [""] * field.count)
                texts = [texts] if field.count == 1 else texts
                assert len(texts) == field.count
                for array, text in enumerate(texts):
                    if text:
                        string_positions[(row_index, field_index, array)] = len(string_table)
                        string_table += text.encode("utf-8") + b"\x00"
        records = [bytearray(self.record_size) for _ in self.rows]
        for row_index, values in enumerate(self.rows):
            record = records[row_index]
            for field_index, field in enumerate(self.schema.fields):
                offset, mode, _, a, b, _ = self.cols[field_index]
                if field.kind == "string":
                    for array in range(field.count):
                        position = string_positions.get((row_index, field_index, array))
                        if position is not None:
                            at = offset // 8 + 4 * array
                            stored = (
                                len(self.rows) * self.record_size
                                - row_index * self.record_size
                                - at
                                + position
                            )
                            record[at : at + 4] = (stored & 0xFFFFFFFF).to_bytes(4, "little")
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
        _put32(out, 184, int(self.parents is not None))
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
        for new, source in self.copies:
            out += (new & 0xFFFFFFFF).to_bytes(4, "little")
            out += (source & 0xFFFFFFFF).to_bytes(4, "little")
        parent_bytes = 0
        if self.parents is not None:
            parent_bytes = 12 + len(self.parents) * 8
            out += len(self.parents).to_bytes(4, "little")
            out += self.min_id.to_bytes(4, "little")
            out += self.max_id.to_bytes(4, "little")
            for parent_id, ordinal in self.parents:
                out += (parent_id & 0xFFFFFFFF).to_bytes(4, "little")
                out += (ordinal & 0xFFFFFFFF).to_bytes(4, "little")
        _put32(out, section_at + 8, metadata_end)
        _put32(out, section_at + 12, len(self.rows))
        _put32(out, section_at + 16, len(string_table))
        _put32(out, section_at + 20, 0)
        _put32(
            out,
            section_at + 24,
            len(self.ids) * 4 if self.schema.index < 0 else 0,
        )
        _put32(out, section_at + 28, parent_bytes)
        _put32(out, section_at + 32, 0)
        _put32(out, section_at + 36, len(self.copies))
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

# Independent fixture metadata from the pinned C++ DB2Metadata.h, commit
# 02245dcd245e7433e524577656177723d3e4992e. Keep these literals independent of
# db2_oracle.SCHEMAS so a mistaken transcription cannot change both the input
# shape and the decoder's expectation. Native float IsSigned flags do not
# convert their IEEE-754 bits to signed integers in the diagnostic format.
FLOAT_FIXTURE_SCHEMAS = (
    # ArmorLocationMeta:1128, FileDataId 1284818.
    Schema(
        name="ArmorLocation", layout_hash=0xFB67352F, index=-1, parent=-1,
        fields=(
            Field("float", 32, 1), Field("float", 32, 1), Field("float", 32, 1),
            Field("float", 32, 1), Field("float", 32, 1),
        ),
    ),
    # BoneWindModifiersMeta:2379, FileDataId 1416756.
    Schema(
        name="BoneWindModifiers", layout_hash=0x2CD1770E, index=-1, parent=-1,
        fields=(Field("float", 32, 3), Field("float", 32, 1)),
    ),
    # AreaTriggerSphereMeta:1088, FileDataId 1315630.
    Schema(
        name="AreaTriggerSphere", layout_hash=0xB851EBF6, index=-1, parent=-1,
        fields=(Field("float", 32, 1),),
    ),
    # DurabilityQualityMeta:7193, FileDataId 1130268.
    Schema(
        name="DurabilityQuality", layout_hash=0x08763C54, index=-1, parent=-1,
        fields=(Field("float", 32, 1),),
    ),
)

# Likewise transcribed independently from the same pinned metadata: name
# fields use FT_STRING_NOT_LOCALIZED; Cfg_Categories uses FT_STRING. Both
# are represented by decoded text here, with no locale substitution.
NAME_FIXTURE_SCHEMAS = (
    Schema(
        name="NamesProfanity", layout_hash=0xF227E638, index=-1, parent=-1,
        fields=(Field("string"), Field("signed", 8, signed=True)),
    ),
    Schema(
        name="NamesReserved", layout_hash=0x2B2D5D97, index=-1, parent=-1,
        fields=(Field("string"),),
    ),
    Schema(
        name="NamesReservedLocale", layout_hash=0x7B9823D4, index=-1, parent=-1,
        fields=(Field("string"), Field("unsigned", 8)),
    ),
    Schema(
        name="Cfg_Categories", layout_hash=0x8710BE94, index=-1, parent=-1,
        fields=(
            Field("string"), Field("unsigned", 16), Field("unsigned", 8),
            Field("unsigned", 8), Field("signed", 32, signed=True),
            Field("signed", 8, signed=True),
        ),
    ),
)

# The five remaining AppearanceRecords tables, independently transcribed
# from pinned DB2Metadata.h:4049,4160,3909,3715,3967 respectively.
APPEARANCE_FIXTURE_SCHEMAS = (
    Schema(
        name="ChrModel", layout_hash=0x03FAB755, index=2, parent=4,
        fields=(
            Field("float", 32, 3), Field("float", 32, 3), Field("unsigned", 32),
            Field("signed", 8), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("float"), Field("float"),
            Field("float"), Field("float"), Field("float"), Field("float"),
        ),
    ),
    Schema(
        name="ChrRaceXChrModel", layout_hash=0xA203BC29, index=-1, parent=0,
        fields=(Field("signed", 8), Field("signed", 32), Field("signed", 8), Field("signed", 32)),
    ),
    Schema(
        name="ChrCustomizationOption", layout_hash=0xDCC2A86E, index=1, parent=4,
        fields=(
            Field("string"), Field("unsigned", 32), Field("unsigned", 16),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("float"),
            Field("signed", 32), Field("signed", 32), Field("signed", 32), Field("signed", 32),
        ),
    ),
    Schema(
        name="ChrCustomizationChoice", layout_hash=0x9559C358, index=1, parent=2,
        fields=(
            Field("string"), Field("unsigned", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("unsigned", 16),
            Field("unsigned", 16), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32, 2),
        ),
    ),
    Schema(
        name="ChrCustomizationReqChoice", layout_hash=0xF925BC6F, index=-1, parent=1,
        fields=(Field("signed", 32),),
    ),
)

# Pinned DB2Metadata.h:12929,12953,9802; independent of the oracle constants.
ITEM_FIXTURE_SCHEMAS = (
    Schema(
        name="ItemSpec", layout_hash=0x83F3D113, index=-1, parent=2,
        fields=(
            Field("unsigned", 8), Field("unsigned", 8), Field("unsigned", 8),
            Field("unsigned", 8), Field("unsigned", 8), Field("unsigned", 16),
        ),
    ),
    Schema(
        name="ItemSpecOverride", layout_hash=0xB292998C, index=-1, parent=1,
        fields=(Field("unsigned", 16),),
    ),
    Schema(
        name="GemProperties", layout_hash=0x86487AD2, index=-1, parent=-1,
        fields=(Field("unsigned", 16), Field("signed", 32)),
    ),
)

# Independent fixture metadata from pinned DB2Metadata.h:19625,19715,19658,
# 3206,3229. SkillLineAbility's acceptance here is synthetic; it does not
# establish the availability of a complete local client file.
BIRTH_FIXTURE_SCHEMAS = (
    Schema(
        name="SkillLine", layout_hash=0x6763217C, index=5, parent=-1,
        fields=(
            Field("string"), Field("string"), Field("string"), Field("string"),
            Field("string"), Field("unsigned", 32), Field("signed", 8),
            Field("signed", 32), Field("signed", 8), Field("unsigned", 32),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32),
        ),
    ),
    Schema(
        name="SkillRaceClassInfo", layout_hash=0x24277B48, index=-1, parent=0,
        fields=(
            Field("signed", 16), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 8), Field("signed", 16),
            Field("signed", 32, 2),
        ),
    ),
    Schema(
        name="SkillLineAbility", layout_hash=0x224F7EA0, index=2, parent=3,
        fields=(
            Field("string"), Field("string"), Field("unsigned", 32),
            Field("signed", 16), Field("signed", 32), Field("signed", 16),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 16), Field("signed", 16), Field("signed", 32),
            Field("signed", 8), Field("signed", 16), Field("signed", 16),
            Field("signed", 16), Field("signed", 32, 2), Field("signed", 32, 2),
        ),
    ),
    Schema(
        name="CharacterLoadout", layout_hash=0x713CE8BB, index=-1, parent=-1,
        fields=(
            Field("signed", 8), Field("signed", 32), Field("unsigned", 8),
            Field("signed", 32), Field("signed", 32, 2),
        ),
    ),
    Schema(
        name="CharacterLoadoutItem", layout_hash=0x0C7A1862, index=-1, parent=0,
        fields=(Field("unsigned", 16), Field("unsigned", 32)),
    ),
)

# Exported for synthetic probes, in initialization order: Map, PowerType,
# Movie. Shapes are independent literals from pinned DB2Metadata.h:14538,
# 17452,15329; no real-client file is embedded or read by these fixtures.
INITIALIZATION_FIXTURE_SCHEMAS = (
    Schema(
        name="Map", layout_hash=0xD43AFAC3, index=-1, parent=-1,
        fields=(
            Field("string"), Field("string"), Field("string"), Field("string"),
            Field("string"), Field("string"), Field("float", 32, 2),
            Field("unsigned", 8), Field("signed", 8), Field("unsigned", 8),
            Field("unsigned", 16), Field("signed", 16), Field("signed", 16),
            Field("signed", 16), Field("signed", 16), Field("unsigned", 8),
            Field("float"), Field("signed", 16), Field("unsigned", 8),
            Field("signed", 16), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 32, 3),
        ),
    ),
    Schema(
        name="PowerType", layout_hash=0x14BBEEA1, index=2, parent=-1,
        fields=(
            Field("string"), Field("string"), Field("unsigned", 32),
            Field("signed", 8), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("signed", 32), Field("signed", 32),
            Field("signed", 32), Field("float"), Field("float"), Field("signed", 32),
        ),
    ),
    Schema(
        name="Movie", layout_hash=0xF53888FA, index=-1, parent=-1,
        fields=(
            Field("string"), Field("unsigned", 8), Field("unsigned", 8),
            Field("unsigned", 32), Field("unsigned", 32), Field("unsigned", 32),
        ),
    ),
)

# Exported for offline synthetic probes. Independent literals from pinned
# DB2Metadata.h:20706,21120,21807,21764, in this exact dependency order.
SPELL_DEPENDENCY_FIXTURE_SCHEMAS = (
    Schema(
        name="SpellCastTimes", layout_hash=0x75B6BD3A, index=-1, parent=-1,
        fields=(Field("signed", 32), Field("signed", 32)),
    ),
    Schema(
        name="SpellDuration", layout_hash=0xA931BD2B, index=-1, parent=-1,
        fields=(Field("signed", 32), Field("signed", 32), Field("signed", 32)),
    ),
    Schema(
        name="SpellRadius", layout_hash=0xF9A913EE, index=-1, parent=-1,
        fields=(Field("float"), Field("float"), Field("float"), Field("float")),
    ),
    Schema(
        name="SpellProcsPerMinute", layout_hash=0x2227014B, index=-1, parent=-1,
        fields=(Field("float"), Field("signed", 32)),
    ),
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


def _xpower_file(ids, parents):
    """Minimal synthetic WDC5 file for the reviewed ``ChrClassesXPowerTypes``.

    Schema: one signed 8-bit decoded column, ``IndexField -1`` so the record id
    comes from the separate u32 id table, and ``ParentIndexField 1`` as the
    extra field at the end. Header carries ``field_count = total_field_count =
    1`` and ``parent_lookup_count = 1``; ``parents`` is a list of
    ``(ParentId, RecordIndex)`` entries in the trailing parent-lookup block.
    """
    schema = db2_oracle.CHR_CLASSES_X_POWER_TYPES
    record_count = len(ids)
    record_size = 1
    out = bytearray(HEADER_SIZE)
    out[0:4] = b"WDC5"
    _put32(out, 4, 5)
    _put32(out, 136, record_count)
    _put32(out, 140, 1)  # FileFieldCount: the single decoded column
    _put32(out, 144, record_size)
    _put32(out, 148, 0)  # string table size
    _put32(out, 152, 0)  # table hash is not gated for this table
    _put32(out, 156, schema.layout_hash)
    _put32(out, 160, 0)
    _put32(out, 164, max(ids) if ids else 0)
    _put32(out, 176, 1)  # total_field_count
    _put32(out, 180, 0)  # packed data offset
    _put32(out, 184, 1)  # parent lookup count
    _put32(out, 188, 24)  # column meta size = fields * 24
    _put_i16(out, 174, schema.index)
    _put32(out, 200, 1)  # one section
    section_at = len(out)
    out += bytearray(SECTION_SIZE)
    out += (0).to_bytes(2, "little") + (0).to_bytes(2, "little")  # field offset 0
    column = bytearray(24)
    _put16(column, 0, 0)  # field offset
    _put32(column, 8, 0)  # compression mode 0 (direct value)
    out += column
    metadata_end = len(out)
    out += bytes(record_count)  # zeroed records (single signed byte each)
    for record_id in ids:
        out += (record_id & 0xFFFFFFFF).to_bytes(4, "little")
    parent_bytes = 12 + len(parents) * 8
    # Parent-lookup block: 12-byte {NumEntries, MinId, MaxId} header then
    # NumEntries 8-byte {ParentId, RecordIndex} entries.
    out += (len(parents) & 0xFFFFFFFF).to_bytes(4, "little")  # NumEntries
    out += (0).to_bytes(4, "little")  # MinId
    out += (max(ids) if ids else 0).to_bytes(4, "little")  # MaxId
    for parent_id, record_index in parents:
        out += (parent_id & 0xFFFFFFFF).to_bytes(4, "little")
        out += (record_index & 0xFFFFFFFF).to_bytes(4, "little")
    _put32(out, section_at + 8, metadata_end)
    _put32(out, section_at + 12, record_count)
    _put32(out, section_at + 16, 0)
    _put32(out, section_at + 20, 0)
    _put32(out, section_at + 24, record_count * 4)
    _put32(out, section_at + 28, parent_bytes)
    return bytes(out)


EXPECTED_CHR_CLASSES_X_POWER_TYPES = (
    "forever-db2-dump v1 ChrClassesXPowerTypes\n"
    "11\ti:0\ti:7\n"
    "22\ti:0\ti:9\n"
)


def _chr_specialization_file(ids, parents, rows=None):
    """Minimal synthetic WDC5 file for the reviewed ``ChrSpecialization``.

    Schema: three strings, the inline unsigned 32-bit id at field 3, a signed
    8-bit parent column at field 4, then further signed fields. ``IndexField 3``
    means the record id comes from the inline field 3; ``ParentIndexField 4`` is
    INSIDE the file field count, so the parent lookup overwrites the existing
    column 4 with the field's own width while the header keeps
    ``field_count == total_field_count == 13`` and ``parent_lookup_count = 1``.
    ``parents`` is a list of ``(ParentId, RecordIndex)`` entries.

    ``rows`` optionally supplies per-record inline field values keyed by field
    index (mode 0 direct values). Strings are omitted (empty) for simplicity.
    """
    schema = db2_oracle.CHR_SPECIALIZATION
    record_count = len(ids)
    field_count = len(schema.fields)
    # Mode-0 direct layout: sum of field widths in bytes, 8-byte aligned per
    # field only where the oracle's mode-0 read is byte-aligned; field 12 is an
    # array of two signed 32-bit values read as two consecutive bytes groups.
    offsets = []
    bit = 0
    for field in schema.fields:
        offsets.append(bit)
        bit += field.bits * field.count
    packed_data_offset = (bit + 7) // 8
    record_size = packed_data_offset
    out = bytearray(HEADER_SIZE)
    out[0:4] = b"WDC5"
    _put32(out, 4, 5)
    _put32(out, 136, record_count)
    _put32(out, 140, field_count)
    _put32(out, 144, record_size)
    _put32(out, 148, 0)  # string table size (no strings)
    _put32(out, 152, 0)  # table hash is not gated
    _put32(out, 156, schema.layout_hash)
    _put32(out, 160, 0)
    _put32(out, 164, max(ids) if ids else 0)
    _put32(out, 176, field_count)  # total_field_count == field_count
    _put32(out, 180, packed_data_offset)
    _put32(out, 184, 1)  # parent lookup count
    _put32(out, 188, field_count * 24)
    _put_i16(out, 174, schema.index)
    _put32(out, 200, 1)  # one section
    section_at = len(out)
    out += bytearray(SECTION_SIZE)
    for offset in offsets:
        out += (0).to_bytes(2, "little") + (offset).to_bytes(2, "little")
    for offset in offsets:
        entry = bytearray(24)
        _put16(entry, 0, offset)
        _put32(entry, 8, 0)  # compression mode 0 (direct value)
        out += entry
    metadata_end = len(out)
    for row_index in range(record_count):
        record = bytearray(record_size)
        values = (rows or [{}] * record_count)[row_index]
        for field_index, field in enumerate(schema.fields):
            value = values.get(field_index)
            if value is None or field.kind == "string":
                continue
            at = offsets[field_index] // 8
            if field.count > 1:
                for array, item in enumerate(value):
                    byte_width = field.bits // 8
                    record[at + array * byte_width : at + (array + 1) * byte_width] = (
                        item & ((1 << field.bits) - 1)
                    ).to_bytes(byte_width, "little")
            else:
                byte_width = field.bits // 8
                record[at : at + byte_width] = (
                    value & ((1 << field.bits) - 1)
                ).to_bytes(byte_width, "little")
        out += record
    # No separate id table: IndexField 3 is inline.
    parent_bytes = 12 + len(parents) * 8
    out += (len(parents) & 0xFFFFFFFF).to_bytes(4, "little")  # NumEntries
    out += (0).to_bytes(4, "little")  # MinId
    out += (max(ids) if ids else 0).to_bytes(4, "little")  # MaxId
    for parent_id, record_index in parents:
        out += (parent_id & 0xFFFFFFFF).to_bytes(4, "little")
        out += (record_index & 0xFFFFFFFF).to_bytes(4, "little")
    _put32(out, section_at + 8, metadata_end)
    _put32(out, section_at + 12, record_count)
    _put32(out, section_at + 16, 0)
    _put32(out, section_at + 20, 0)
    _put32(out, section_at + 24, 0)
    _put32(out, section_at + 28, parent_bytes)
    return bytes(out)


def _chr_customization_category_file():
    """One synthetic row of the reviewed ``ChrCustomizationCategory``.

    Schema: String, Signed(32,1) x6, Float(1), with ``IndexField -1`` so the
    record id comes from the separate u32 id table (DB2Metadata.h, layout
    0x1605B68A).
    """
    rows = [
        {
            0: "Category",
            1: 0xFFFFFFFF,  # -1
            2: 0x00000007,  # 7
            3: 0x80000000,  # -2147483648
            4: 0x7FFFFFFF,  # 2147483647
            5: 0xFFFFFFFE,  # -2
            6: 0x00000000,  # 0
            7: 0x3F800000,  # float 1.0
        }
    ]
    builder = Wdc5Builder(
        db2_oracle.CHR_CUSTOMIZATION_CATEGORY,
        rows,
        min_id=0,
        max_id=100,
        ids=[100],
    )
    return builder.build()


EXPECTED_CHR_CUSTOMIZATION_CATEGORY = (
    "forever-db2-dump v1 ChrCustomizationCategory\n"
    "100\tt:Category\ti:-1\ti:7\ti:-2147483648\ti:2147483647"
    "\ti:-2\ti:0\tf:0x3f800000\n"
)


def _chr_customization_material_file():
    """One synthetic row of the reviewed ``ChrCustomizationMaterial``.

    Schema: Signed(32,1) x2 with ``IndexField -1`` (separate u32 id table),
    layout 0xBE9767E9.
    """
    rows = [{0: 0xFFFFFFF9, 1: 0x000003E7}]  # -7, 999
    builder = Wdc5Builder(
        db2_oracle.CHR_CUSTOMIZATION_MATERIAL,
        rows,
        min_id=0,
        max_id=100,
        ids=[42],
    )
    return builder.build()


EXPECTED_CHR_CUSTOMIZATION_MATERIAL = (
    "forever-db2-dump v1 ChrCustomizationMaterial\n"
    "42\ti:-7\ti:999\n"
)


def _zero_section_file(schema, record_count):
    """A complete minimal WDC5 file with ``section_count == 0``.

    DB2FileLoader.cpp:1782-1784 still reads four primitive metadata bytes per
    file field; :1903-1912 requires the file size to equal the first section
    offset (the current metadata end when no sections exist). No column,
    pallet/common, string or record payload is present in this bounded form.
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
    _put32(out, 188, 0)
    _put32(out, 200, 0)
    out += bytes(len(schema.fields) * 4)
    return bytes(out)


class SyntheticOracle(unittest.TestCase):
    def test_existing_self_copy_is_rejected(self):
        schema = db2_oracle.ANIM_KIT_PRIORITY
        data = Wdc5Builder(schema, [{0: 7}], ids=[17], copies=[(17, 17)]).build()
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, "unsupported_record_variant")

    def test_absent_and_zero_self_copies_are_skipped(self):
        schema = db2_oracle.ANIM_KIT_PRIORITY
        data = Wdc5Builder(
            schema, [{0: 5}, {0: 7}], ids=[0, 17], copies=[(99, 99), (0, 0)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 AnimKitPriority\n0\tu:5\n17\tu:7\n",
        )

    def test_existing_copy_with_narrow_inline_id_is_rejected(self):
        schema = db2_oracle.CHR_CLASSES
        data = Wdc5Builder(schema, [{29: 17}], copies=[(81, 17)]).build()
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, "unsupported_record_variant")

    def test_narrow_inline_id_copies_with_absent_and_zero_sources_are_skipped(self):
        schema = db2_oracle.CHR_CLASSES
        rows = [{29: 0}, {29: 17}]
        baseline = Wdc5Builder(schema, rows).build()
        data = Wdc5Builder(schema, rows, copies=[(80, 0), (81, 77)]).build()
        self.assertEqual(
            db2_oracle.decode(data, schema), db2_oracle.decode(baseline, schema)
        )

    def test_reviewed_curve_copy_preserves_values_and_rewrites_u32_inline_id(self):
        schema = db2_oracle.CURVE
        data = Wdc5Builder(
            schema, [{0: 17, 1: 255, 2: -2}], max_id=900, copies=[(900, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 Curve\n"
            "17\tu:17\tu:255\ti:-2\n900\tu:900\tu:255\ti:-2\n",
        )

    def test_high_bit_appended_parent_is_signed_and_inherited_by_copy(self):
        schema = db2_oracle.CHR_CLASSES_X_POWER_TYPES
        data = Wdc5Builder(
            schema, [{0: 1}, {0: 2}], ids=[11, 22], max_id=900,
            parents=[(0x80000000, 0), (0xFFFFFFFF, 1)],
            copies=[(800, 11), (900, 22)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 ChrClassesXPowerTypes\n"
            "11\ti:1\ti:-2147483648\n22\ti:2\ti:-1\n"
            "800\ti:1\ti:-2147483648\n900\ti:2\ti:-1\n",
        )

    def test_high_bit_in_data_signed_parent_is_inherited_by_copy(self):
        schema = Schema(
            name="SyntheticSignedParentCopy", layout_hash=0xABCDEF0B, index=0, parent=1,
            fields=(Field("unsigned", 32), Field("signed", 32)),
        )
        data = Wdc5Builder(
            schema, [{0: 17, 1: 2}], parents=[(0x80000000, 0)], copies=[(99, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticSignedParentCopy\n"
            "17\tu:17\ti:-2147483648\n99\tu:99\ti:-2147483648\n",
        )

    def test_scalar_limit_includes_arrays_and_appended_parent_in_direct_rows(self):
        schema = Schema(
            name="SyntheticValueLimit", layout_hash=0xABCDEF08, index=-1, parent=1,
            fields=(Field("unsigned", 32, 3),),
        )
        data = Wdc5Builder(
            schema, [{0: [1, 2, 3]}, {0: [4, 5, 6]}], ids=[17, 99], parents=[],
        ).build()
        # Each row has three array scalars plus one appended parent, not one
        # value for its single file column.
        with mock.patch.object(db2_oracle, "VALUE_POLICY_LIMIT", 7):
            with self.assertRaises(db2_oracle.OracleError) as caught:
                db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, "record_policy_limit")

    def test_scalar_limit_applies_before_new_copy_row_is_materialized(self):
        schema = Schema(
            name="SyntheticCopyValueLimit", layout_hash=0xABCDEF09, index=-1, parent=1,
            fields=(Field("unsigned", 32, 3),),
        )
        data = Wdc5Builder(
            schema, [{0: [1, 2, 3]}], ids=[17], parents=[(7, 0)],
            copies=[(99, 17)],
        ).build()
        with mock.patch.object(db2_oracle, "VALUE_POLICY_LIMIT", 7):
            with self.assertRaises(db2_oracle.OracleError) as caught:
                db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, "record_policy_limit")

    def test_scalar_limit_does_not_charge_skipped_or_overwritten_copy_rows(self):
        schema = Schema(
            name="SyntheticCopyAtLimit", layout_hash=0xABCDEF0A, index=-1,
            fields=(Field("signed", 8),),
        )
        data = Wdc5Builder(
            schema, [{0: -1}, {0: 7}], ids=[17, 99],
            copies=[(80, 0), (81, 77), (82, 1000), (17, 99)],
        ).build()
        with mock.patch.object(db2_oracle, "VALUE_POLICY_LIMIT", 2):
            rows = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(rows, schema.name),
            "forever-db2-dump v1 SyntheticCopyAtLimit\n17\ti:7\n99\ti:7\n",
        )

    def test_sparse_high_id_copies_rewrite_inline_id_and_preserve_values(self):
        schema = Schema(
            name="SyntheticCopies",
            layout_hash=0xABCDEF01,
            index=1,
            fields=(Field("string"), Field("unsigned", 32), Field("signed", 16, 2)),
        )
        data = Wdc5Builder(
            schema,
            [{0: "alpha", 1: 41, 2: [-2, 7]}, {0: "beta", 1: 10000, 2: [8, 9]}],
            max_id=300000,
            copies=[(250000, 41)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticCopies\n"
            "41\tt:alpha\tu:41\ti:-2,i:7\n"
            "10000\tt:beta\tu:10000\ti:8,i:9\n"
            "250000\tt:alpha\tu:250000\ti:-2,i:7\n",
        )

    def test_copy_order_chains_missing_sources_and_overwrites(self):
        schema = Schema(
            name="SyntheticCopyOrder", layout_hash=0xABCDEF02, index=-1,
            fields=(Field("string"), Field("signed", 16)),
        )
        data = Wdc5Builder(
            schema,
            [{0: "first", 1: -2}, {0: "second", 1: 7}],
            ids=[17, 99],
            max_id=10000,
            copies=[
                (400, 300),  # Forward references are skipped, not retried.
                (300, 17),
                (500, 300),  # A previous copy is a valid source.
                (17, 99),  # Existing destination is replaced.
                (600, 17),  # Reads the replaced value.
                (99, 500),
                (800, 777),  # Absent source inside the index-table extent.
                (900, 10000),  # Source outside the observed extent.
            ],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticCopyOrder\n"
            "17\tt:second\ti:7\n"
            "99\tt:first\ti:-2\n"
            "300\tt:first\ti:-2\n"
            "500\tt:first\ti:-2\n"
            "600\tt:second\ti:7\n",
        )

    def test_zero_copy_source_is_skipped_but_zero_destination_is_valid(self):
        schema = Schema(
            name="SyntheticZeroCopy", layout_hash=0xABCDEF03, index=-1,
            fields=(Field("signed", 8),),
        )
        data = Wdc5Builder(
            schema, [{0: 5}, {0: 7}], ids=[0, 17],
            copies=[(80, 0), (81, 17), (0, 17), (82, 0)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticZeroCopy\n"
            "0\ti:7\n17\ti:7\n81\ti:7\n",
        )

    def test_copy_inherits_appended_parent_before_replacing_source(self):
        schema = db2_oracle.CHR_CLASSES_X_POWER_TYPES
        data = Wdc5Builder(
            schema, [{0: -1}, {0: 2}], ids=[11, 22], max_id=900,
            parents=[(7, 0), (9, 1)],
            copies=[(800, 11), (11, 22), (900, 800)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 ChrClassesXPowerTypes\n"
            "11\ti:2\ti:9\n22\ti:2\ti:9\n"
            "800\ti:-1\ti:7\n900\ti:-1\ti:7\n",
        )

    def test_copy_inherits_in_data_parent_and_rewrites_inline_index(self):
        schema = Schema(
            name="SyntheticParentCopy", layout_hash=0xABCDEF04, index=0, parent=1,
            fields=(Field("unsigned", 32), Field("signed", 8)),
        )
        data = Wdc5Builder(
            schema, [{0: 17, 1: 2}], max_id=800,
            parents=[(255, 0)], copies=[(800, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticParentCopy\n"
            "17\tu:17\ti:-1\n800\tu:800\ti:-1\n",
        )

    def test_copy_destinations_above_header_max_are_rejected_even_if_skipped(self):
        schema = Schema(
            name="SyntheticCopyBounds", layout_hash=0xABCDEF05, index=-1,
            fields=(Field("signed", 8),),
        )
        for source in (17, 0, 99):
            with self.subTest(source=source):
                data = Wdc5Builder(
                    schema, [{0: 7}], ids=[17], max_id=100,
                    copies=[(101, source)],
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, schema)
                self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_effective_u32_max_id_is_rejected_before_index_size_overflows(self):
        schema = Schema(
            name="SyntheticCopyOverflow", layout_hash=0xABCDEF06, index=-1,
            fields=(Field("signed", 8),),
        )
        for direct_id, copies in (
            (0xFFFFFFFF, []),
            (17, [(0xFFFFFFFF, 17)]),
            (17, [(0xFFFFFFFF, 0)]),
        ):
            with self.subTest(direct_id=direct_id, copies=copies):
                data = Wdc5Builder(
                    schema, [{0: 7}], ids=[direct_id], max_id=0xFFFFFFFF,
                    copies=copies,
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, schema)
                self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_max_header_with_smaller_effective_ids_and_near_u32_max_copy(self):
        schema = Schema(
            name="SyntheticCopyNearMax", layout_hash=0xABCDEF07, index=-1,
            fields=(Field("signed", 8),),
        )
        data = Wdc5Builder(
            schema, [{0: -1}], ids=[17], max_id=0xFFFFFFFF,
            copies=[(0xFFFFFFFE, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, schema), schema.name),
            "forever-db2-dump v1 SyntheticCopyNearMax\n"
            "17\ti:-1\n4294967294\ti:-1\n",
        )

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

    def test_reviewed_chr_classes_x_power_types_dump_matches_golden(self):
        # Extra-field-at-the-end parent variant: 1 file field (signed 8),
        # IndexField -1 (separate u32 id table) and ParentIndexField 1 filled
        # from the trailing parent-lookup block (DB2FileLoader.cpp:1877).
        data = _xpower_file([11, 22], [(7, 0), (9, 1)])
        rows = db2_oracle.decode(data, db2_oracle.CHR_CLASSES_X_POWER_TYPES)
        self.assertEqual(
            db2_oracle.render(rows, "ChrClassesXPowerTypes"),
            EXPECTED_CHR_CLASSES_X_POWER_TYPES,
        )

    def test_reviewed_chr_classes_x_power_types_parent_index_out_of_range(self):
        # A parent entry whose RecordIndex addresses no decoded ordinal is an
        # invalid_record_ids error, not a silent skip.
        data = _xpower_file([11, 22], [(7, 2)])
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.CHR_CLASSES_X_POWER_TYPES)
        self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_reviewed_chr_specialization_transcription(self):
        # ChrSpecialization: FileDataId 1343390, IndexField 3, ParentIndexField 4
        # (INSIDE the file field count), FieldCount == FileFieldCount == 13,
        # layout 0xDAB4CA4B. Field 4 is a signed 8-bit FT_BYTE.
        schema = db2_oracle.SCHEMAS["ChrSpecialization"]
        self.assertEqual(schema.layout_hash, 0xDAB4CA4B)
        self.assertEqual(schema.index, 3)
        self.assertEqual(schema.parent, 4)
        self.assertEqual(len(schema.fields), 13)
        self.assertLess(schema.parent, len(schema.fields), "in-data-block parent")
        for index in (0, 1, 2):
            self.assertEqual(
                (schema.fields[index].kind, schema.fields[index].bits),
                ("string", 32),
                "spec %d" % index,
            )
        self.assertEqual(
            (schema.fields[3].kind, schema.fields[3].bits), ("unsigned", 32)
        )
        for index in (4, 5, 6, 7, 10):
            self.assertEqual(
                (schema.fields[index].kind, schema.fields[index].bits),
                ("signed", 8),
                "spec %d" % index,
            )
        for index in (8, 9, 11):
            self.assertEqual(
                (schema.fields[index].kind, schema.fields[index].bits),
                ("signed", 32),
                "spec %d" % index,
            )
        self.assertEqual(
            (schema.fields[12].kind, schema.fields[12].bits, schema.fields[12].count),
            ("signed", 32, 2),
        )

    def test_reviewed_chr_specialization_in_data_block_dump_matches_golden(self):
        # The parent lookup overwrites the EXISTING signed 8-bit column 4, with
        # its own width and sign extension, and no appended field is added. Two
        # rows keyed by the inline field 3.
        data = _chr_specialization_file(
            [11, 22],
            [(0, 0), (200, 1)],
            rows=[
                {3: 11, 4: 0x7F, 8: 0xFFFFFFFF, 10: 0xFF, 12: [0, 0xFFFFFFFF]},
                {3: 22, 4: 0x05, 8: 0x0000002A, 10: 0x02, 12: [7, 8]},
            ],
        )
        rows = db2_oracle.decode(data, db2_oracle.CHR_SPECIALIZATION)
        self.assertEqual(
            db2_oracle.render(rows, db2_oracle.CHR_SPECIALIZATION.name),
            "forever-db2-dump v1 ChrSpecialization\n"
            # ParentId 0 -> i:0, overwriting the inline 0x7F.
            "11\tt:\tt:\tt:\tu:11\ti:0\ti:0\ti:0\ti:0\ti:-1\ti:0\ti:-1\ti:0\ti:0,i:-1\n"
            # ParentId 200 sign-extends at 8 bits to -56, overwriting 0x05.
            "22\tt:\tt:\tt:\tu:22\ti:-56\ti:0\ti:0\ti:0\ti:42\ti:0\ti:2\ti:0\ti:7,i:8\n",
        )

    def test_reviewed_chr_specialization_parent_over_byte_is_rejected(self):
        # A ParentId above 255 cannot fit the signed 8-bit FT_BYTE parent column,
        # so it is an invalid_record_ids error (the reference ASSERT).
        data = _chr_specialization_file([11], [(256, 0)], rows=[{3: 11}])
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.CHR_SPECIALIZATION)
        self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_reviewed_chr_customization_category_dump_matches_golden(self):
        # String + six signed 32-bit fields + float, IndexField -1 (separate u32
        # id table), no parent. The string offset is per-record relative and the
        # signed words sign-extend.
        data = _chr_customization_category_file()
        rows = db2_oracle.decode(data, db2_oracle.CHR_CUSTOMIZATION_CATEGORY)
        self.assertEqual(
            db2_oracle.render(rows, db2_oracle.CHR_CUSTOMIZATION_CATEGORY.name),
            EXPECTED_CHR_CUSTOMIZATION_CATEGORY,
        )

    def test_reviewed_chr_customization_material_dump_matches_golden(self):
        # A signed-only reviewed table (Signed(32,1) x2), no parent and a
        # separate u32 id table.
        data = _chr_customization_material_file()
        rows = db2_oracle.decode(data, db2_oracle.CHR_CUSTOMIZATION_MATERIAL)
        self.assertEqual(
            db2_oracle.render(rows, db2_oracle.CHR_CUSTOMIZATION_MATERIAL.name),
            EXPECTED_CHR_CUSTOMIZATION_MATERIAL,
        )

    def test_reviewed_curve_inline_index_dump_matches_golden(self):
        # `Curve` is [Unsigned(32,1), Unsigned(8,1), Signed(32,1)] with
        # IndexField 0: the record id comes from the inline field 0, not a
        # separate u32 id table. The unsigned byte must not sign-extend.
        schema = db2_oracle.CURVE
        rows = [{0: 5, 1: 0xFF, 2: 0xFFFFFFF9}]  # id 5, byte 255, -7
        data = Wdc5Builder(schema, rows, min_id=5, max_id=100).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 Curve\n"
            "5\tu:5\tu:255\ti:-7\n",
        )

    def test_reviewed_content_restriction_rule_set_inline_index_dump_matches_golden(self):
        # `ContentRestrictionRuleSet` is [Unsigned(32,1), Signed(32,1),
        # Signed(32,1)] with IndexField 0: the inline field 0 (unsigned) is also
        # the record id.
        schema = db2_oracle.CONTENT_RESTRICTION_RULE_SET
        rows = [{0: 0x0000000C, 1: 0xFFFFFFFF, 2: 0x7FFFFFFF}]  # id 12
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 ContentRestrictionRuleSet\n"
            "12\tu:12\ti:-1\ti:2147483647\n",
        )

    def test_reviewed_data_tag_info_inline_index_dump_matches_golden(self):
        # `DataTagInfo` is [String, Unsigned(32,1), Signed(32,1)] with
        # IndexField 1: the record id comes from the inline field 1 while field
        # 0 is a per-record relative string offset.
        schema = db2_oracle.DATA_TAG_INFO
        rows = [{0: "Tag", 1: 0x0000002A, 2: 0xFFFFFFF5}]  # id 42
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 DataTagInfo\n"
            "42\tt:Tag\tu:42\ti:-11\n",
        )

    def test_reviewed_curve_and_restriction_transcriptions_are_present(self):
        curve = db2_oracle.SCHEMAS["Curve"]
        self.assertEqual(curve.layout_hash, 0xD362535A)
        self.assertEqual(curve.index, 0)
        self.assertEqual(curve.parent, -1)
        self.assertEqual(len(curve.fields), 3)
        self.assertEqual(
            (curve.fields[0].kind, curve.fields[0].bits, curve.fields[0].signed),
            ("unsigned", 32, False),
        )
        self.assertEqual(
            (curve.fields[1].kind, curve.fields[1].bits, curve.fields[1].signed),
            ("unsigned", 8, False),
        )
        self.assertEqual(
            (curve.fields[2].kind, curve.fields[2].bits, curve.fields[2].signed),
            ("signed", 32, True),
        )

        rule_set = db2_oracle.SCHEMAS["ContentRestrictionRuleSet"]
        self.assertEqual(rule_set.layout_hash, 0x5774D75A)
        self.assertEqual(rule_set.index, 0)
        self.assertEqual(rule_set.parent, -1)
        self.assertEqual(len(rule_set.fields), 3)
        self.assertEqual(
            (
                rule_set.fields[0].kind,
                rule_set.fields[0].bits,
                rule_set.fields[0].signed,
            ),
            ("unsigned", 32, False),
        )
        for index, field in enumerate(rule_set.fields[1:], start=1):
            self.assertEqual(
                (field.kind, field.bits, field.signed),
                ("signed", 32, True),
                "rule_set %d" % index,
            )

        tag = db2_oracle.SCHEMAS["DataTagInfo"]
        self.assertEqual(tag.layout_hash, 0xDB46F772)
        self.assertEqual(tag.index, 1)
        self.assertEqual(tag.parent, -1)
        self.assertEqual(len(tag.fields), 3)
        self.assertEqual((tag.fields[0].kind, tag.fields[0].bits), ("string", 32))
        self.assertEqual(
            (tag.fields[1].kind, tag.fields[1].bits, tag.fields[1].signed),
            ("unsigned", 32, False),
        )
        self.assertEqual(
            (tag.fields[2].kind, tag.fields[2].bits, tag.fields[2].signed),
            ("signed", 32, True),
        )

    def test_reviewed_chr_customization_element_dump_matches_golden(self):
        # Thirteen signed 32-bit fields, IndexField -1 (separate u32 id table).
        schema = db2_oracle.CHR_CUSTOMIZATION_ELEMENT
        rows = [
            {
                0: 0xFFFFFFFF,
                1: 0x00000007,
                2: 0x80000000,
                3: 0x7FFFFFFF,
                4: 0xFFFFFFFE,
                5: 0x00000000,
                6: 0xFFFFFFF9,
                7: 123456,
                8: 0x0000000A,
                9: 0xFFFFFFF6,
                10: 0x00000001,
                11: 0x00000002,
                12: 0x00000003,
            }
        ]
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[77]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 ChrCustomizationElement\n"
            "77\ti:-1\ti:7\ti:-2147483648\ti:2147483647\ti:-2\ti:0\ti:-7"
            "\ti:123456\ti:10\ti:-10\ti:1\ti:2\ti:3\n",
        )

    def test_reviewed_chr_customization_display_info_dump_matches_golden(self):
        # Two signed 32-bit fields then three floats, IndexField -1.
        schema = db2_oracle.CHR_CUSTOMIZATION_DISPLAY_INFO
        rows = [
            {
                0: 0xFFFFFFF9,  # -7
                1: 0x000003E8,  # 1000
                2: 0x3F800000,  # float 1.0
                3: 0x40000000,  # float 2.0
                4: 0xC0000000,  # float -2.0
            }
        ]
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[12]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 ChrCustomizationDisplayInfo\n"
            "12\ti:-7\ti:1000\tf:0x3f800000\tf:0x40000000\tf:0xc0000000\n",
        )

    def test_reviewed_chr_customization_req_dump_matches_golden(self):
        # String + seven signed 32-bit fields + one Signed(32,2) array;
        # IndexField -1. The array prints both values comma-joined.
        schema = db2_oracle.CHR_CUSTOMIZATION_REQ
        rows = [
            {
                0: "Req",
                1: 0xFFFFFFFF,  # -1
                2: 0x00000000,  # 0
                3: 0x00000007,  # 7
                4: 0x80000000,  # -2147483648
                5: 0x7FFFFFFF,  # 2147483647
                6: 0xFFFFFFF8,  # -8
                7: 0x0000002A,  # 42
                8: [0xFFFFFFF5, 0x77359401],  # [-11, 2000000001]
            }
        ]
        data = Wdc5Builder(
            schema, rows, min_id=0, max_id=1000, ids=[345]
        ).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 ChrCustomizationReq\n"
            "345\tt:Req\ti:-1\ti:0\ti:7\ti:-2147483648\ti:2147483647"
            "\ti:-8\ti:42\ti:-11,i:2000000001\n",
        )

    def test_reviewed_forever_transcriptions_are_present(self):
        category = db2_oracle.SCHEMAS["ChrCustomizationCategory"]
        self.assertEqual(category.layout_hash, 0x1605B68A)
        self.assertEqual(category.index, -1)
        self.assertEqual(category.parent, -1)
        self.assertEqual(len(category.fields), 8)
        self.assertEqual((category.fields[0].kind, category.fields[0].bits), ("string", 32))
        for index in range(1, 7):
            self.assertEqual(
                (category.fields[index].kind, category.fields[index].bits, category.fields[index].signed),
                ("signed", 32, True),
                "field %d" % index,
            )
        self.assertEqual((category.fields[7].kind, category.fields[7].count), ("float", 1))
        for name, layout, fields in (
            ("ChrCustItemGeoModify", 0x2986E1F3, 3),
            ("ChrCustomizationGeoset", 0xBF55F2FB, 3),
            ("ChrCustomizationMaterial", 0xBE9767E9, 2),
            ("ChrCustomizationBoneSet", 0xCC2CFF6E, 2),
        ):
            schema = db2_oracle.SCHEMAS[name]
            self.assertEqual(schema.layout_hash, layout, name)
            self.assertEqual(schema.index, -1, name)
            self.assertEqual(schema.parent, -1, name)
            self.assertEqual(len(schema.fields), fields, name)
            self.assertEqual(schema.fields[0].kind, "signed", name)
            self.assertTrue(schema.fields[0].signed, name)

        element = db2_oracle.SCHEMAS["ChrCustomizationElement"]
        self.assertEqual(element.layout_hash, 0x6483C37E)
        self.assertEqual(element.index, -1)
        self.assertEqual(element.parent, -1)
        self.assertEqual(len(element.fields), 13)
        for index, field in enumerate(element.fields):
            self.assertEqual(
                (field.kind, field.bits, field.signed), ("signed", 32, True), "element %d" % index
            )

        display = db2_oracle.SCHEMAS["ChrCustomizationDisplayInfo"]
        self.assertEqual(display.layout_hash, 0xB24CE54F)
        self.assertEqual(display.index, -1)
        self.assertEqual(display.parent, -1)
        self.assertEqual(len(display.fields), 5)
        self.assertEqual((display.fields[0].kind, display.fields[0].bits, display.fields[0].signed), ("signed", 32, True))
        self.assertEqual((display.fields[1].kind, display.fields[1].bits, display.fields[1].signed), ("signed", 32, True))
        for index in range(2, 5):
            self.assertEqual((display.fields[index].kind, display.fields[index].count), ("float", 1), "display %d" % index)

        req = db2_oracle.SCHEMAS["ChrCustomizationReq"]
        self.assertEqual(req.layout_hash, 0xCA154412)
        self.assertEqual(req.index, -1)
        self.assertEqual(req.parent, -1)
        self.assertEqual(len(req.fields), 9)
        self.assertEqual((req.fields[0].kind, req.fields[0].bits), ("string", 32))
        for index in range(1, 8):
            self.assertEqual(
                (req.fields[index].kind, req.fields[index].bits, req.fields[index].signed),
                ("signed", 32, True),
                "req %d" % index,
            )
        self.assertEqual((req.fields[8].kind, req.fields[8].bits, req.fields[8].count), ("signed", 32, 2))

    def test_reviewed_anim_byte_table_transcriptions_are_present(self):
        # The three reviewed Forever `Anim*` byte tables: each declares
        # IndexField -1 (separate u32 id table), ParentIndexField -1 and
        # FieldCount == FileFieldCount. `AnimKitBoneSetAlias` is two unsigned
        # 8-bit fields; the other two are each a single unsigned 8-bit field.
        alias = db2_oracle.SCHEMAS["AnimKitBoneSetAlias"]
        self.assertEqual(alias.layout_hash, 0x19A6986C)
        self.assertEqual(alias.index, -1)
        self.assertEqual(alias.parent, -1)
        self.assertEqual(len(alias.fields), 2)
        for index in range(2):
            self.assertEqual(
                (alias.fields[index].kind, alias.fields[index].bits, alias.fields[index].signed),
                ("unsigned", 8, False),
                "alias %d" % index,
            )

        for name, layout in (
            ("AnimKitPriority", 0xCCF889D8),
            ("AnimReplacementSet", 0x7C047695),
        ):
            schema = db2_oracle.SCHEMAS[name]
            self.assertEqual(schema.layout_hash, layout, name)
            self.assertEqual(schema.index, -1, name)
            self.assertEqual(schema.parent, -1, name)
            self.assertEqual(len(schema.fields), 1, name)
            self.assertEqual(
                (
                    schema.fields[0].kind,
                    schema.fields[0].bits,
                    schema.fields[0].signed,
                ),
                ("unsigned", 8, False),
                name,
            )

    def test_reviewed_anim_kit_bone_set_alias_dump_matches_golden(self):
        # `AnimKitBoneSetAlias` is [Unsigned(8,1), Unsigned(8,1)] with
        # IndexField -1: the record id comes from the separate u32 id table and
        # neither byte is signed.
        schema = db2_oracle.ANIM_KIT_BONE_SET_ALIAS
        rows = [{0: 0xFF, 1: 0x01}]  # 255, 1
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[3]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 AnimKitBoneSetAlias\n"
            "3\tu:255\tu:1\n",
        )

    def test_reviewed_anim_tables_transcriptions_are_present(self):
        # The three reviewed Forever animation tables: each declares IndexField
        # -1 (separate u32 id table), ParentIndexField -1 and FieldCount ==
        # FileFieldCount. `AnimKitBoneSet` is a non-localized string plus signed
        # scalars; `AnimationData` adds an unsigned 16-bit field and a signed
        # 32-bit array of two; `AnimKitConfig` is a single signed 32-bit field.
        bone_set = db2_oracle.SCHEMAS["AnimKitBoneSet"]
        self.assertEqual(bone_set.layout_hash, 0x43E7736F)
        self.assertEqual(bone_set.index, -1)
        self.assertEqual(bone_set.parent, -1)
        self.assertEqual(len(bone_set.fields), 5)
        self.assertEqual((bone_set.fields[0].kind, bone_set.fields[0].bits), ("string", 32))
        for index in (1, 4):
            self.assertEqual(
                (bone_set.fields[index].kind, bone_set.fields[index].bits, bone_set.fields[index].signed),
                ("signed", 32, True),
                "bone_set %d" % index,
            )
        for index in (2, 3):
            self.assertEqual(
                (bone_set.fields[index].kind, bone_set.fields[index].bits, bone_set.fields[index].signed),
                ("signed", 8, True),
                "bone_set %d" % index,
            )

        animation = db2_oracle.SCHEMAS["AnimationData"]
        self.assertEqual(animation.layout_hash, 0xBBF66A3C)
        self.assertEqual(animation.index, -1)
        self.assertEqual(animation.parent, -1)
        self.assertEqual(len(animation.fields), 4)
        self.assertEqual(
            (animation.fields[0].kind, animation.fields[0].bits, animation.fields[0].signed),
            ("unsigned", 16, False),
        )
        self.assertEqual(
            (animation.fields[1].kind, animation.fields[1].bits, animation.fields[1].signed),
            ("signed", 8, True),
        )
        self.assertEqual(
            (animation.fields[2].kind, animation.fields[2].bits, animation.fields[2].signed),
            ("signed", 16, True),
        )
        self.assertEqual(
            (animation.fields[3].kind, animation.fields[3].bits, animation.fields[3].count),
            ("signed", 32, 2),
        )

        config = db2_oracle.SCHEMAS["AnimKitConfig"]
        self.assertEqual(config.layout_hash, 0x140718EF)
        self.assertEqual(config.index, -1)
        self.assertEqual(config.parent, -1)
        self.assertEqual(len(config.fields), 1)
        self.assertEqual(
            (config.fields[0].kind, config.fields[0].bits, config.fields[0].signed),
            ("signed", 32, True),
        )

    def test_reviewed_anim_kit_bone_set_dump_matches_golden(self):
        # A non-localized string plus short signed fields: 0x80 sign-extends at 8
        # bits to -128 while the string decodes as ordinary text. The signed
        # 32-bit fields keep full-width values.
        schema = db2_oracle.ANIM_KIT_BONE_SET
        rows = [{0: "BoneSet", 1: 0xFFFFFFF9, 2: 0x80, 3: 0x7F, 4: 0x0000002A}]  # -7, -128, 127, 42
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[100]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 AnimKitBoneSet\n"
            "100\tt:BoneSet\ti:-7\ti:-128\ti:127\ti:42\n",
        )

    def test_reviewed_animation_data_dump_matches_golden_with_array_of_two(self):
        # `AnimationData` is [Unsigned(16,1), Signed(8,1), Signed(16,1),
        # Signed(32,2)]: the unsigned 16-bit 0xFFFF stays 65535, the signed
        # 16-bit 0x8000 sign-extends to -32768 and the array prints both values
        # comma-joined.
        schema = db2_oracle.ANIMATION_DATA
        rows = [
            {
                0: 0xFFFF,  # 65535 (unsigned)
                1: 0x80,  # -128
                2: 0x8000,  # -32768
                3: [0xFFFFFFF5, 0x77359401],  # [-11, 2000000001]
            }
        ]
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[12]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 AnimationData\n"
            "12\tu:65535\ti:-128\ti:-32768\ti:-11,i:2000000001\n",
        )

    def test_reviewed_anim_kit_config_dump_matches_golden(self):
        # A single signed 32-bit field with a separate u32 id table.
        schema = db2_oracle.ANIM_KIT_CONFIG
        rows = [{0: 0xFFFFFFD6}]  # -42
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[34]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 AnimKitConfig\n"
            "34\ti:-42\n",
        )

    def test_reviewed_anim_kit_priority_dump_matches_golden(self):
        # A one-field file field count: `AnimKitPriority` has a single unsigned
        # 8-bit column and a separate u32 id table. 0x80 stays 128, not -128.
        schema = db2_oracle.ANIM_KIT_PRIORITY
        rows = [{0: 0x80}]
        data = Wdc5Builder(schema, rows, min_id=0, max_id=100, ids=[9]).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(
            db2_oracle.render(decoded, schema.name),
            "forever-db2-dump v1 AnimKitPriority\n"
            "9\tu:128\n",
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

    def _assert_reviewed_fixture_dump(
        self, fixture, rows, ids, expected, copies=(), parents=None,
    ):
        data = Wdc5Builder(
            fixture, rows, ids=ids, max_id=250000, copies=copies, parents=parents,
        ).build()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-reviewed.db2"
            path.write_bytes(data)
            self.assertEqual(db2_oracle.dump_file(path, fixture.name), expected)

    def test_reviewed_float_table_transcriptions_match_pinned_cpp(self):
        # Literal identities and scalar/array shapes from the pinned metadata,
        # independent of the oracle constants and the synthetic writer.
        expected = (
            ("ArmorLocation", 0xFB67352F, 5,
             (("float", 32, 1), ("float", 32, 1), ("float", 32, 1),
              ("float", 32, 1), ("float", 32, 1))),
            ("BoneWindModifiers", 0x2CD1770E, 2,
             (("float", 32, 3), ("float", 32, 1))),
            ("AreaTriggerSphere", 0xB851EBF6, 1, (("float", 32, 1),)),
            ("DurabilityQuality", 0x08763C54, 1, (("float", 32, 1),)),
        )
        for name, layout, field_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, -1)
                self.assertEqual(schema.parent, -1)
                self.assertEqual(len(schema.fields), field_count)
                self.assertEqual(
                    tuple((field.kind, field.bits, field.count) for field in schema.fields),
                    fields,
                )

    def test_armor_location_float_bits_and_high_id_copy_match_golden(self):
        # Integer bit patterns avoid a host float conversion, especially for
        # signed zero, subnormals and distinct quiet-NaN payloads.
        self._assert_reviewed_fixture_dump(
            FLOAT_FIXTURE_SCHEMAS[0],
            [
                {0: 0x00000000, 1: 0x80000000, 2: 0x00000001,
                 3: 0x007FFFFF, 4: 0x3F800000},
                {0: 0xBF800000, 1: 0x7F800000, 2: 0xFF800000,
                 3: 0x7FC12345, 4: 0xFFC54321},
            ],
            [17, 900],
            "forever-db2-dump v1 ArmorLocation\n"
            "17\tf:0x00000000\tf:0x80000000\tf:0x00000001\tf:0x007fffff\tf:0x3f800000\n"
            "900\tf:0xbf800000\tf:0x7f800000\tf:0xff800000\tf:0x7fc12345\tf:0xffc54321\n"
            "250000\tf:0xbf800000\tf:0x7f800000\tf:0xff800000\tf:0x7fc12345\tf:0xffc54321\n",
            copies=[(250000, 900)],
        )

    def test_bone_wind_modifiers_float_array_bits_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            FLOAT_FIXTURE_SCHEMAS[1],
            [
                {0: [0x00000000, 0x80000000, 0x00000001], 1: 0x3F800000},
                {0: [0x007FFFFF, 0x7F7FFFFF, 0xFF7FFFFF], 1: 0xBF000000},
                {0: [0x7F800000, 0xFF800000, 0x7FC12345], 1: 0xFFC54321},
            ],
            [41, 901, 9001],
            "forever-db2-dump v1 BoneWindModifiers\n"
            "41\tf:0x00000000,f:0x80000000,f:0x00000001\tf:0x3f800000\n"
            "901\tf:0x007fffff,f:0x7f7fffff,f:0xff7fffff\tf:0xbf000000\n"
            "9001\tf:0x7f800000,f:0xff800000,f:0x7fc12345\tf:0xffc54321\n"
            "250000\tf:0x7f800000,f:0xff800000,f:0x7fc12345\tf:0xffc54321\n",
            copies=[(250000, 9001)],
        )

    def test_area_trigger_sphere_float_bits_match_golden(self):
        self._assert_reviewed_fixture_dump(
            FLOAT_FIXTURE_SCHEMAS[2],
            [{0: bits} for bits in (
                0x00000000, 0x80000000, 0x00000001, 0x007FFFFF, 0x3F800000,
                0xBF800000, 0x7F800000, 0xFF800000, 0x7FC12345, 0xFFC54321,
            )],
            [11, 12, 13, 14, 15, 16, 17, 18, 19, 90000],
            "forever-db2-dump v1 AreaTriggerSphere\n"
            "11\tf:0x00000000\n12\tf:0x80000000\n13\tf:0x00000001\n"
            "14\tf:0x007fffff\n15\tf:0x3f800000\n16\tf:0xbf800000\n"
            "17\tf:0x7f800000\n18\tf:0xff800000\n19\tf:0x7fc12345\n"
            "90000\tf:0xffc54321\n",
        )

    def test_durability_quality_float_bits_match_golden(self):
        self._assert_reviewed_fixture_dump(
            FLOAT_FIXTURE_SCHEMAS[3],
            [{0: bits} for bits in (
                0x00000000, 0x80000000, 0x00000001, 0x007FFFFF, 0x3F800000,
                0xBF800000, 0x7F800000, 0xFF800000, 0x7FC12345, 0xFFC54321,
            )],
            [21, 22, 23, 24, 25, 26, 27, 28, 29, 91000],
            "forever-db2-dump v1 DurabilityQuality\n"
            "21\tf:0x00000000\n22\tf:0x80000000\n23\tf:0x00000001\n"
            "24\tf:0x007fffff\n25\tf:0x3f800000\n26\tf:0xbf800000\n"
            "27\tf:0x7f800000\n28\tf:0xff800000\n29\tf:0x7fc12345\n"
            "91000\tf:0xffc54321\n",
        )

    def test_reviewed_float_tables_reject_layout_count_and_parent_mismatches(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-float-mismatch.db2"
            for fixture in FLOAT_FIXTURE_SCHEMAS:
                base = Wdc5Builder(fixture, [{}], ids=[17]).build()
                mutations = (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file field count", 140, len(fixture.fields) + 1),
                    ("total field count", 176, len(fixture.fields) + 1),
                    ("parent", 184, 1),
                )
                for label, offset, value in mutations:
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_reviewed_float_tables_reject_truncated_metadata_records_and_ids(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-float-truncated.db2"
            for fixture in FLOAT_FIXTURE_SCHEMAS:
                builder = Wdc5Builder(fixture, [{}], ids=[17])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                cuts = (
                    ("header", HEADER_SIZE - 1, "metadata_truncated"),
                    ("section", HEADER_SIZE + SECTION_SIZE - 1, "metadata_truncated"),
                    ("column metadata", records_start - 1, "record_data_bounds"),
                    ("record", records_start + builder.record_size - 1, "record_data_bounds"),
                    ("id table", len(base) - 1, "record_data_bounds"),
                )
                for label, cut, code in cuts:
                    with self.subTest(table=fixture.name, truncation=label):
                        path.write_bytes(base[:cut])
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, code)

    def test_bone_wind_modifiers_rejects_record_too_short_for_array_or_last_scalar(self):
        fixture = FLOAT_FIXTURE_SCHEMAS[1]
        base = Wdc5Builder(
            fixture, [{0: [0x3F800000, 0x40000000, 0x40400000], 1: 0x40800000}],
            ids=[17],
        ).build()
        records_start = HEADER_SIZE + SECTION_SIZE + 2 * 28
        for record_size in (8, 12):
            with self.subTest(record_size=record_size):
                # Preserve the complete id table and the declared file extent;
                # the field read itself must reject the missing third array
                # element (8 bytes) or trailing scalar (12 bytes).
                data = bytearray(base[:records_start + record_size] + base[-4:])
                _put32(data, 144, record_size)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.BONE_WIND_MODIFIERS)
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_reviewed_name_table_transcriptions_match_pinned_cpp(self):
        expected = (
            ("NamesProfanity", 0xF227E638, 2,
             (("string", 32, 1, False), ("signed", 8, 1, True))),
            ("NamesReserved", 0x2B2D5D97, 1, (("string", 32, 1, False),)),
            ("NamesReservedLocale", 0x7B9823D4, 2,
             (("string", 32, 1, False), ("unsigned", 8, 1, False))),
            ("Cfg_Categories", 0x8710BE94, 6,
             (("string", 32, 1, False), ("unsigned", 16, 1, False),
              ("unsigned", 8, 1, False), ("unsigned", 8, 1, False),
              ("signed", 32, 1, True), ("signed", 8, 1, True))),
        )
        for name, layout, field_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, -1)
                self.assertEqual(schema.parent, -1)
                self.assertEqual(len(schema.fields), field_count)
                self.assertEqual(
                    tuple((f.kind, f.bits, f.count, f.signed) for f in schema.fields),
                    fields,
                )

    def test_names_profanity_utf8_escaping_signed_byte_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            NAME_FIXTURE_SCHEMAS[0],
            [{0: "ñ\tA\nB\\C\rß", 1: 0x80}, {0: "", 1: 0x7F}],
            [17, 41],
            "forever-db2-dump v1 NamesProfanity\n"
            "17\tt:ñ\\tA\\nB\\\\C\\rß\ti:-128\n41\tt:\ti:127\n"
            "900\tt:ñ\\tA\\nB\\\\C\\rß\ti:-128\n",
            copies=[(900, 17)],
        )

    def test_names_reserved_nonlocalized_utf8_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            NAME_FIXTURE_SCHEMAS[1],
            [{0: "保留🙂\\名"}, {0: ""}],
            [22, 700],
            "forever-db2-dump v1 NamesReserved\n"
            "22\tt:保留🙂\\\\名\n700\tt:\n900\tt:保留🙂\\\\名\n",
            copies=[(900, 22)],
        )

    def test_names_reserved_locale_utf8_unsigned_byte_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            NAME_FIXTURE_SCHEMAS[2],
            [{0: "Réservé\t名", 1: 0xFF}, {0: "", 1: 0x80}],
            [17, 99],
            "forever-db2-dump v1 NamesReservedLocale\n"
            "17\tt:Réservé\\t名\tu:255\n99\tt:\tu:128\n"
            "900\tt:Réservé\\t名\tu:255\n",
            copies=[(900, 17)],
        )

    def test_cfg_categories_utf8_signed_and_unsigned_widths_match_golden(self):
        self._assert_reviewed_fixture_dump(
            NAME_FIXTURE_SCHEMAS[3],
            [
                {0: "Español\tEuropa\n\\x\r終", 1: 0xFFFF, 2: 0xFF,
                 3: 0x80, 4: 0x80000000, 5: 0xFF},
                {0: "", 1: 0x8000, 2: 0, 3: 0x7F, 4: 0x7FFFFFFF, 5: 0x80},
            ],
            [17, 99],
            "forever-db2-dump v1 Cfg_Categories\n"
            "17\tt:Español\\tEuropa\\n\\\\x\\r終\tu:65535\tu:255\tu:128\ti:-2147483648\ti:-1\n"
            "99\tt:\tu:32768\tu:0\tu:127\ti:2147483647\ti:-128\n"
            "900\tt:Español\\tEuropa\\n\\\\x\\r終\tu:65535\tu:255\tu:128\ti:-2147483648\ti:-1\n",
            copies=[(900, 17)],
        )

    def test_reviewed_name_tables_reject_schema_and_parent_mismatches(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-name-mismatch.db2"
            for fixture in NAME_FIXTURE_SCHEMAS:
                base = Wdc5Builder(fixture, [{0: "text"}], ids=[17]).build()
                for label, offset, value in (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file field count", 140, len(fixture.fields) + 1),
                    ("total field count", 176, len(fixture.fields) + 1),
                    ("parent", 184, 1),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_reviewed_name_tables_reject_invalid_string_offsets_terminators_and_utf8(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-name-string.db2"
            for fixture in NAME_FIXTURE_SCHEMAS:
                builder = Wdc5Builder(fixture, [{0: "ok"}], ids=[17])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                strings_start = records_start + builder.record_size
                for label in ("offset", "terminator", "utf8"):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        if label == "offset":
                            _put32(data, records_start, builder.record_size + 3)
                        elif label == "terminator":
                            data[strings_start + 2] = ord("x")
                        else:
                            data[strings_start] = 0xFF
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "invalid_record_string")

    def test_reviewed_name_tables_reject_truncated_string_and_id_tables(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-name-truncated.db2"
            for fixture in NAME_FIXTURE_SCHEMAS:
                builder = Wdc5Builder(fixture, [{0: "ok"}], ids=[17])
                base = builder.build()
                strings_start = (
                    HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                    + builder.record_size
                )
                for label, cut in (("string table", strings_start + 2),
                                   ("id table", len(base) - 1)):
                    with self.subTest(table=fixture.name, truncation=label):
                        path.write_bytes(base[:cut])
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_appearance_table_transcriptions_match_pinned_cpp(self):
        expected = (
            ("ChrModel", 0x03FAB755, 2, 4, 17, 17,
             (("float", 32, 3), ("float", 32, 3), ("unsigned", 32, 1),
              ("signed", 8, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("float", 32, 1),
              ("float", 32, 1), ("float", 32, 1), ("float", 32, 1),
              ("float", 32, 1), ("float", 32, 1))),
            ("ChrRaceXChrModel", 0xA203BC29, -1, 0, 4, 4,
             (("signed", 8, 1), ("signed", 32, 1), ("signed", 8, 1), ("signed", 32, 1))),
            ("ChrCustomizationOption", 0xDCC2A86E, 1, 4, 13, 13,
             (("string", 32, 1), ("unsigned", 32, 1), ("unsigned", 16, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("float", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1))),
            ("ChrCustomizationChoice", 0x9559C358, 1, 2, 11, 11,
             (("string", 32, 1), ("unsigned", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("unsigned", 16, 1),
              ("unsigned", 16, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 2))),
            ("ChrCustomizationReqChoice", 0xF925BC6F, -1, 1, 1, 2,
             (("signed", 32, 1),)),
        )
        for name, layout, index, parent, file_count, total_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, index)
                self.assertEqual(schema.parent, parent)
                self.assertEqual(len(schema.fields), file_count)
                self.assertEqual(max(len(schema.fields), schema.parent + 1), total_count)
                self.assertEqual(
                    tuple((f.kind, f.bits, f.count) for f in schema.fields), fields
                )
                for field in schema.fields:
                    if field.kind == "signed":
                        self.assertTrue(field.signed)
                    elif field.kind == "unsigned":
                        self.assertFalse(field.signed)

    def test_chr_model_float_arrays_parent_and_inline_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            APPEARANCE_FIXTURE_SCHEMAS[0],
            [{
                0: [0x80000000, 0x00000001, 0x7FC12345],
                1: [0x3F800000, 0x7F800000, 0xFF800000],
                2: 17, 3: 0x80, 4: 123, 5: -2, 6: 0x80000000,
                7: 0x7FFFFFFF, 8: 0, 9: 42, 10: -1, 11: 0,
                12: 0x80000000, 13: 1, 14: 0x3F800000,
                15: 0x7FC54321, 16: 0x7F7FFFFF,
            }],
            [17],
            "forever-db2-dump v1 ChrModel\n"
            "17\tf:0x80000000,f:0x00000001,f:0x7fc12345"
            "\tf:0x3f800000,f:0x7f800000,f:0xff800000\tu:17\ti:-128"
            "\ti:-1\ti:-2\ti:-2147483648\ti:2147483647\ti:0\ti:42\ti:-1"
            "\tf:0x00000000\tf:0x80000000\tf:0x00000001\tf:0x3f800000\tf:0x7fc54321\tf:0x7f7fffff\n"
            "900\tf:0x80000000,f:0x00000001,f:0x7fc12345"
            "\tf:0x3f800000,f:0x7f800000,f:0xff800000\tu:900\ti:-128"
            "\ti:-1\ti:-2\ti:-2147483648\ti:2147483647\ti:0\ti:42\ti:-1"
            "\tf:0x00000000\tf:0x80000000\tf:0x00000001\tf:0x3f800000\tf:0x7fc54321\tf:0x7f7fffff\n",
            copies=[(900, 17)], parents=[(7, 0), (0xFFFFFFFF, 0)],
        )

    def test_chr_race_x_chr_model_parent_overwrite_and_default_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            APPEARANCE_FIXTURE_SCHEMAS[1],
            [{0: 3, 1: 0x80000000, 2: 0xFF, 3: 0x7FFFFFFF},
             {0: 4, 1: -2, 2: 0x80, 3: -1}],
            [17, 99],
            "forever-db2-dump v1 ChrRaceXChrModel\n"
            "17\ti:-128\ti:-2147483648\ti:-1\ti:2147483647\n"
            "99\ti:4\ti:-2\ti:-128\ti:-1\n"
            "900\ti:-128\ti:-2147483648\ti:-1\ti:2147483647\n"
            "901\ti:4\ti:-2\ti:-128\ti:-1\n",
            copies=[(900, 17), (901, 99)], parents=[(0x80, 0)],
        )

    def test_chr_customization_option_utf8_float_parent_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            APPEARANCE_FIXTURE_SCHEMAS[2],
            [{0: "Opción\t名\n\\x", 1: 17, 2: 0xFFFF, 3: -2, 4: 123,
              5: 0x80000000, 6: 0x7FFFFFFF, 7: -3, 8: 0xFFC01234,
              9: 0, 10: 1, 11: 42, 12: -4}],
            [17],
            "forever-db2-dump v1 ChrCustomizationOption\n"
            "17\tt:Opción\\t名\\n\\\\x\tu:17\tu:65535\ti:-2\ti:-1"
            "\ti:-2147483648\ti:2147483647\ti:-3\tf:0xffc01234\ti:0\ti:1\ti:42\ti:-4\n"
            "900\tt:Opción\\t名\\n\\\\x\tu:900\tu:65535\ti:-2\ti:-1"
            "\ti:-2147483648\ti:2147483647\ti:-3\tf:0xffc01234\ti:0\ti:1\ti:42\ti:-4\n",
            copies=[(900, 17)], parents=[(0xFFFFFFFF, 0)],
        )

    def test_chr_customization_choice_signed_array_parent_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            APPEARANCE_FIXTURE_SCHEMAS[3],
            [{0: "Elección\t名", 1: 41, 2: 99, 3: -2, 4: 0x7FFFFFFF,
              5: 0xFFFF, 6: 0x8000, 7: -3, 8: -1, 9: 42,
              10: [0x80000000, 0x7FFFFFFF]}],
            [41],
            "forever-db2-dump v1 ChrCustomizationChoice\n"
            "41\tt:Elección\\t名\tu:41\ti:-2147483648\ti:-2\ti:2147483647"
            "\tu:65535\tu:32768\ti:-3\ti:-1\ti:42\ti:-2147483648,i:2147483647\n"
            "900\tt:Elección\\t名\tu:900\ti:-2147483648\ti:-2\ti:2147483647"
            "\tu:65535\tu:32768\ti:-3\ti:-1\ti:42\ti:-2147483648,i:2147483647\n",
            copies=[(900, 41)], parents=[(0x80000000, 0)],
        )

    def test_chr_customization_req_choice_appended_parent_defaults_and_copies(self):
        self._assert_reviewed_fixture_dump(
            APPEARANCE_FIXTURE_SCHEMAS[4],
            [{0: -1}, {0: 0x80000000}],
            [11, 22],
            "forever-db2-dump v1 ChrCustomizationReqChoice\n"
            "11\ti:-1\ti:-1\n22\ti:-2147483648\ti:0\n"
            "800\ti:-1\ti:-1\n900\ti:-2147483648\ti:0\n",
            copies=[(800, 11), (900, 22)], parents=[(7, 0), (0xFFFFFFFF, 0)],
        )

    def test_appearance_tables_preserve_inline_parent_or_default_appended_parent(self):
        for fixture in APPEARANCE_FIXTURE_SCHEMAS:
            # Native C++ accepts both no lookup and an empty lookup; fixtures
            # with parents=None contain no trailing parent bytes at all.
            for parents in (None, []):
                with self.subTest(table=fixture.name, parents=parents):
                    values = {}
                    if fixture.index >= 0:
                        values[fixture.index] = 17
                    expected_parent = 0
                    if fixture.parent < len(fixture.fields):
                        values[fixture.parent] = 23
                        expected_parent = 23
                    data = Wdc5Builder(
                        fixture, [values], ids=[17], parents=parents, copies=[(99, 17)],
                    ).build()
                    decoded = db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(list(decoded), [17, 99])
                    for record_id in (17, 99):
                        parent = decoded[record_id][fixture.parent][0]
                        self.assertEqual((parent.kind, parent.number), ("signed", expected_parent))
                    if fixture.index >= 0:
                        self.assertEqual(decoded[99][fixture.index][0].number, 99)

    def test_appearance_tables_reject_layout_counts_and_unannounced_parent_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-appearance-mismatch.db2"
            for fixture in APPEARANCE_FIXTURE_SCHEMAS:
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                base = Wdc5Builder(fixture, [values], ids=[17], parents=[]).build()
                for label, offset, value, code in (
                    ("layout", 156, fixture.layout_hash ^ 1, "unsupported_table_schema"),
                    ("file count", 140, len(fixture.fields) + 1, "unsupported_table_schema"),
                    ("total count", 176, len(fixture.fields) + 1, "unsupported_table_schema"),
                    ("unannounced parent bytes", 184, 0, "record_data_bounds"),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, code)

    def test_appearance_tables_reject_parent_ordinal_outside_direct_rows(self):
        for fixture in APPEARANCE_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                data = Wdc5Builder(
                    fixture, [values], ids=[17], parents=[(7, 1)], copies=[(99, 17)],
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_appearance_tables_reject_truncated_parent_lookup(self):
        for fixture in APPEARANCE_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                data = Wdc5Builder(
                    fixture, [values], ids=[17], parents=[(7, 0)],
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data[:-1], db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_chr_race_x_chr_model_rejects_parent_over_byte_width(self):
        fixture = APPEARANCE_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(fixture, [{}], ids=[17], parents=[(256, 0)]).build()
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.CHR_RACE_X_CHR_MODEL)
        self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_appearance_localized_strings_reject_invalid_offsets_and_utf8(self):
        for fixture in APPEARANCE_FIXTURE_SCHEMAS[2:4]:
            builder = Wdc5Builder(fixture, [{0: "ok", 1: 17}], parents=[])
            base = builder.build()
            records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
            strings_start = records_start + builder.record_size
            for mutation in ("offset", "terminator", "utf8"):
                with self.subTest(table=fixture.name, mutation=mutation):
                    data = bytearray(base)
                    if mutation == "offset":
                        _put32(data, records_start, builder.record_size + 3)
                    elif mutation == "terminator":
                        data[strings_start + 2] = ord("x")
                    else:
                        data[strings_start] = 0xFF
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, "invalid_record_string")

    def test_item_table_transcriptions_match_pinned_cpp(self):
        expected = (
            ("ItemSpec", 0x83F3D113, 2, 6, 6,
             (("unsigned", 8, 1), ("unsigned", 8, 1), ("unsigned", 8, 1),
              ("unsigned", 8, 1), ("unsigned", 8, 1), ("unsigned", 16, 1))),
            ("ItemSpecOverride", 0xB292998C, 1, 1, 2, (("unsigned", 16, 1),)),
            ("GemProperties", 0x86487AD2, -1, 2, 2,
             (("unsigned", 16, 1), ("signed", 32, 1))),
        )
        for name, layout, parent, file_count, total_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, -1)
                self.assertEqual(schema.parent, parent)
                self.assertEqual(len(schema.fields), file_count)
                self.assertEqual(max(len(schema.fields), schema.parent + 1), total_count)
                self.assertEqual(tuple((f.kind, f.bits, f.count) for f in schema.fields), fields)
                for field in schema.fields:
                    self.assertEqual(field.signed, field.kind == "signed")

    def test_item_spec_unsigned_limits_lookup_order_and_copies_match_golden(self):
        self._assert_reviewed_fixture_dump(
            ITEM_FIXTURE_SCHEMAS[0],
            [{0: 0xFF, 1: 0x80, 2: 99, 3: 0, 4: 0x7F, 5: 0xFFFF},
             {0: 1, 1: 2, 2: 0xFF, 3: 4, 4: 5, 5: 0x8000}],
            [17, 99],
            "forever-db2-dump v1 ItemSpec\n"
            "17\tu:255\tu:128\tu:128\tu:0\tu:127\tu:65535\n"
            "99\tu:1\tu:2\tu:255\tu:4\tu:5\tu:32768\n"
            "900\tu:255\tu:128\tu:128\tu:0\tu:127\tu:65535\n"
            "901\tu:1\tu:2\tu:255\tu:4\tu:5\tu:32768\n",
            copies=[(900, 17), (901, 99)], parents=[(7, 0), (0x80, 0)],
        )

    def test_item_spec_without_parent_lookup_preserves_unsigned_parent_and_copy(self):
        self._assert_reviewed_fixture_dump(
            ITEM_FIXTURE_SCHEMAS[0],
            [{0: 0xFF, 1: 0x80, 2: 0xFF, 3: 0, 4: 0x7F, 5: 0xFFFF}],
            [17],
            "forever-db2-dump v1 ItemSpec\n"
            "17\tu:255\tu:128\tu:255\tu:0\tu:127\tu:65535\n"
            "900\tu:255\tu:128\tu:255\tu:0\tu:127\tu:65535\n",
            copies=[(900, 17)], parents=None,
        )

    def test_item_spec_override_signed_appended_parents_defaults_and_copies(self):
        self._assert_reviewed_fixture_dump(
            ITEM_FIXTURE_SCHEMAS[1],
            [{0: 0xFFFF}, {0: 0x8000}, {0: 0}],
            [17, 99, 101],
            "forever-db2-dump v1 ItemSpecOverride\n"
            "17\tu:65535\ti:-2147483648\n99\tu:32768\ti:-1\n"
            "101\tu:0\ti:0\n900\tu:65535\ti:-2147483648\n",
            copies=[(900, 17)], parents=[(0x80000000, 0), (0xFFFFFFFF, 1)],
        )

    def test_item_spec_override_without_lookup_initializes_appended_parent_to_zero(self):
        self._assert_reviewed_fixture_dump(
            ITEM_FIXTURE_SCHEMAS[1], [{0: 0xFFFF}], [17],
            "forever-db2-dump v1 ItemSpecOverride\n"
            "17\tu:65535\ti:0\n900\tu:65535\ti:0\n",
            copies=[(900, 17)], parents=None,
        )

    def test_xpower_without_lookup_initializes_appended_parent_and_copy_to_zero(self):
        schema = Schema(
            name="ChrClassesXPowerTypes", layout_hash=0x70DA1F8C,
            index=-1, parent=1, fields=(Field("signed", 8),),
        )
        self._assert_reviewed_fixture_dump(
            schema, [{0: 0x80}], [17],
            "forever-db2-dump v1 ChrClassesXPowerTypes\n"
            "17\ti:-128\ti:0\n900\ti:-128\ti:0\n",
            copies=[(900, 17)], parents=None,
        )

    def test_specialization_without_lookup_preserves_in_record_parent_and_copy(self):
        schema = db2_oracle.CHR_SPECIALIZATION
        data = Wdc5Builder(
            schema, [{3: 17, 4: 0x80}], copies=[(99, 17)], parents=None,
        ).build()
        decoded = db2_oracle.decode(data, schema)
        self.assertEqual(list(decoded), [17, 99])
        for record_id in (17, 99):
            self.assertEqual(decoded[record_id][3][0].number, record_id)
            self.assertEqual(decoded[record_id][4][0].number, -128)

    def test_parent_lookup_zero_rejects_leftover_lookup_blocks(self):
        for fixture in (
            db2_oracle.CHR_CLASSES_X_POWER_TYPES, db2_oracle.CHR_SPECIALIZATION,
            ITEM_FIXTURE_SCHEMAS[0], ITEM_FIXTURE_SCHEMAS[1],
        ):
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                data = bytearray(Wdc5Builder(fixture, [values], ids=[17], parents=[]).build())
                _put32(data, 184, 0)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_gem_properties_unsigned_short_signed_int_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            ITEM_FIXTURE_SCHEMAS[2],
            [{0: 0xFFFF, 1: 0x80000000}, {0: 0x8000, 1: 0x7FFFFFFF}],
            [17, 99],
            "forever-db2-dump v1 GemProperties\n"
            "17\tu:65535\ti:-2147483648\n99\tu:32768\ti:2147483647\n"
            "900\tu:65535\ti:-2147483648\n",
            copies=[(900, 17)],
        )

    def test_item_spec_rejects_parent_over_byte_or_invalid_direct_ordinal(self):
        fixture = ITEM_FIXTURE_SCHEMAS[0]
        for parent_id, ordinal in ((256, 0), (0xFFFFFFFF, 0), (7, 1)):
            with self.subTest(parent_id=parent_id, ordinal=ordinal):
                data = Wdc5Builder(
                    fixture, [{}], ids=[17], parents=[(parent_id, ordinal)],
                    copies=[(99, 17)],
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, db2_oracle.ITEM_SPEC)
                self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_item_spec_override_rejects_invalid_parent_ordinal(self):
        fixture = ITEM_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(fixture, [{}], ids=[17], parents=[(7, 1)]).build()
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.ITEM_SPEC_OVERRIDE)
        self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_gem_properties_rejects_unexpected_parent_lookup(self):
        data = Wdc5Builder(ITEM_FIXTURE_SCHEMAS[2], [{}], ids=[17], parents=[]).build()
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.GEM_PROPERTIES)
        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_item_tables_reject_layout_and_field_count_mismatches(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-item-mismatch.db2"
            for fixture in ITEM_FIXTURE_SCHEMAS:
                parents = [] if fixture.parent >= 0 else None
                base = Wdc5Builder(fixture, [{}], ids=[17], parents=parents).build()
                for label, offset, value in (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file count", 140, len(fixture.fields) + 1),
                    ("total count", 176, len(fixture.fields) + 1),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_item_tables_reject_truncated_records_ids_or_parent_data(self):
        for fixture in ITEM_FIXTURE_SCHEMAS:
            parents = [(7, 0)] if fixture.parent >= 0 else None
            builder = Wdc5Builder(fixture, [{}], ids=[17], parents=parents)
            base = builder.build()
            records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
            for cut in (records_start + builder.record_size - 1,
                        records_start + builder.record_size + 3, len(base) - 1):
                with self.subTest(table=fixture.name, cut=cut):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(base[:cut], db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_item_tables_accept_empty_single_section_with_reviewed_parent_metadata(self):
        for fixture in ITEM_FIXTURE_SCHEMAS:
            for parents in ((None, []) if fixture.parent >= 0 else (None,)):
                with self.subTest(table=fixture.name, parents=parents):
                    data = Wdc5Builder(fixture, [], ids=[], parents=parents).build()
                    self.assertEqual(db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name]), {})

    def test_birth_table_transcriptions_match_pinned_cpp(self):
        expected = (
            ("SkillLine", 0x6763217C, 5, -1, 15,
             (("string", 32, 1), ("string", 32, 1), ("string", 32, 1),
              ("string", 32, 1), ("string", 32, 1), ("unsigned", 32, 1),
              ("signed", 8, 1), ("signed", 32, 1), ("signed", 8, 1),
              ("unsigned", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1))),
            ("SkillRaceClassInfo", 0x24277B48, -1, 0, 7,
             (("signed", 16, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 8, 1), ("signed", 16, 1), ("signed", 32, 2))),
            ("SkillLineAbility", 0x224F7EA0, 2, 3, 18,
             (("string", 32, 1), ("string", 32, 1), ("unsigned", 32, 1),
              ("signed", 16, 1), ("signed", 32, 1), ("signed", 16, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 16, 1), ("signed", 16, 1), ("signed", 32, 1),
              ("signed", 8, 1), ("signed", 16, 1), ("signed", 16, 1),
              ("signed", 16, 1), ("signed", 32, 2), ("signed", 32, 2))),
            ("CharacterLoadout", 0x713CE8BB, -1, -1, 5,
             (("signed", 8, 1), ("signed", 32, 1), ("unsigned", 8, 1),
              ("signed", 32, 1), ("signed", 32, 2))),
            ("CharacterLoadoutItem", 0x0C7A1862, -1, 0, 2,
             (("unsigned", 16, 1), ("unsigned", 32, 1))),
        )
        for name, layout, index, parent, field_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, index)
                self.assertEqual(schema.parent, parent)
                self.assertEqual(len(schema.fields), field_count)
                self.assertEqual(tuple((f.kind, f.bits, f.count) for f in schema.fields), fields)
                for field in schema.fields:
                    if field.kind in ("signed", "unsigned"):
                        self.assertEqual(field.signed, field.kind == "signed")

    def test_skill_line_strings_unsigned_int_and_inline_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            BIRTH_FIXTURE_SCHEMAS[0],
            [{0: "Oficio\tñ", 1: "描述", 2: "", 3: "línea\nfin", 4: "token\\r\r",
              5: 17, 6: 0x80, 7: 0x80000000, 8: 0xFF, 9: 0xFFFFFFFF,
              10: 0x7FFFFFFF, 11: -2, 12: 0, 13: 42, 14: -1}],
            [17],
            "forever-db2-dump v1 SkillLine\n"
            "17\tt:Oficio\\tñ\tt:描述\tt:\tt:línea\\nfin\tt:token\\\\r\\r"
            "\tu:17\ti:-128\ti:-2147483648\ti:-1\tu:4294967295"
            "\ti:2147483647\ti:-2\ti:0\ti:42\ti:-1\n"
            "900\tt:Oficio\\tñ\tt:描述\tt:\tt:línea\\nfin\tt:token\\\\r\\r"
            "\tu:900\ti:-128\ti:-2147483648\ti:-1\tu:4294967295"
            "\ti:2147483647\ti:-2\ti:0\ti:42\ti:-1\n",
            copies=[(900, 17)],
        )

    def test_skill_race_class_info_signed_short_parent_mask_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            BIRTH_FIXTURE_SCHEMAS[1],
            [{0: 7, 1: 0x80000000, 2: 0x7FFFFFFF, 3: -1, 4: 0x80,
              5: 0x8000, 6: [0x80000000, 0x7FFFFFFF]}],
            [17],
            "forever-db2-dump v1 SkillRaceClassInfo\n"
            "17\ti:-1\ti:-2147483648\ti:2147483647\ti:-1\ti:-128\ti:-32768"
            "\ti:-2147483648,i:2147483647\n"
            "900\ti:-1\ti:-2147483648\ti:2147483647\ti:-1\ti:-128\ti:-32768"
            "\ti:-2147483648,i:2147483647\n",
            copies=[(900, 17)], parents=[(0x8000, 0), (0xFFFF, 0)],
        )

    def test_skill_line_ability_synthetic_strings_masks_parent_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            BIRTH_FIXTURE_SCHEMAS[2],
            [{0: "Habilidad\t名", 1: "Desc\n\\fin", 2: 41, 3: 99,
              4: 0x80000000, 5: 0x8000, 6: 0x7FFFFFFF, 7: -1, 8: 0,
              9: 0xFFFF, 10: 0x7FFF, 11: -2, 12: 0x80, 13: 0x8000,
              14: 0xFFFF, 15: 0x7FFF, 16: [0x80000000, 0x7FFFFFFF], 17: [-1, 0]}],
            [41],
            "forever-db2-dump v1 SkillLineAbility\n"
            "41\tt:Habilidad\\t名\tt:Desc\\n\\\\fin\tu:41\ti:-1"
            "\ti:-2147483648\ti:-32768\ti:2147483647\ti:-1\ti:0\ti:-1"
            "\ti:32767\ti:-2\ti:-128\ti:-32768\ti:-1\ti:32767"
            "\ti:-2147483648,i:2147483647\ti:-1,i:0\n"
            "900\tt:Habilidad\\t名\tt:Desc\\n\\\\fin\tu:900\ti:-1"
            "\ti:-2147483648\ti:-32768\ti:2147483647\ti:-1\ti:0\ti:-1"
            "\ti:32767\ti:-2\ti:-128\ti:-32768\ti:-1\ti:32767"
            "\ti:-2147483648,i:2147483647\ti:-1,i:0\n",
            copies=[(900, 41)], parents=[(0xFFFF, 0)],
        )

    def test_character_loadout_signed_mask_unsigned_byte_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            BIRTH_FIXTURE_SCHEMAS[3],
            [{0: 0x80, 1: 0x80000000, 2: 0xFF, 3: 0x7FFFFFFF, 4: [-1, 0x80000000]},
             {0: 0x7F, 1: -1, 2: 0x80, 3: 0, 4: [0x7FFFFFFF, 42]}],
            [17, 99],
            "forever-db2-dump v1 CharacterLoadout\n"
            "17\ti:-128\ti:-2147483648\tu:255\ti:2147483647\ti:-1,i:-2147483648\n"
            "99\ti:127\ti:-1\tu:128\ti:0\ti:2147483647,i:42\n"
            "900\ti:-128\ti:-2147483648\tu:255\ti:2147483647\ti:-1,i:-2147483648\n",
            copies=[(900, 17)],
        )

    def test_character_loadout_item_unsigned_parent_and_int_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            BIRTH_FIXTURE_SCHEMAS[4], [{0: 0x8000, 1: 0xFFFFFFFF}], [17],
            "forever-db2-dump v1 CharacterLoadoutItem\n"
            "17\tu:65535\tu:4294967295\n900\tu:65535\tu:4294967295\n",
            copies=[(900, 17)], parents=[(0xFFFF, 0)],
        )

    def test_birth_in_record_parents_without_lookup_preserve_signedness_and_copy(self):
        for fixture, kind, expected in (
            (BIRTH_FIXTURE_SCHEMAS[1], "signed", -32768),
            (BIRTH_FIXTURE_SCHEMAS[2], "signed", -32768),
            (BIRTH_FIXTURE_SCHEMAS[4], "unsigned", 32768),
        ):
            for parents in (None, []):
                with self.subTest(table=fixture.name, parents=parents):
                    values = {fixture.parent: 0x8000}
                    if fixture.index >= 0:
                        values[fixture.index] = 17
                    data = Wdc5Builder(
                        fixture, [values], ids=[17], parents=parents, copies=[(99, 17)],
                    ).build()
                    decoded = db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(list(decoded), [17, 99])
                    for record_id in (17, 99):
                        value = decoded[record_id][fixture.parent][0]
                        self.assertEqual((value.kind, value.number), (kind, expected))
                    if fixture.index >= 0:
                        self.assertEqual(decoded[99][fixture.index][0].number, 99)

    def test_birth_tables_reject_parent_above_short_width_or_invalid_ordinal(self):
        for fixture in (BIRTH_FIXTURE_SCHEMAS[1], BIRTH_FIXTURE_SCHEMAS[2], BIRTH_FIXTURE_SCHEMAS[4]):
            values = {fixture.index: 17} if fixture.index >= 0 else {}
            for parent_id, ordinal in ((0x10000, 0), (0xFFFFFFFF, 0), (7, 1)):
                with self.subTest(table=fixture.name, parent_id=parent_id, ordinal=ordinal):
                    data = Wdc5Builder(
                        fixture, [values], ids=[17], parents=[(parent_id, ordinal)],
                    ).build()
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_birth_tables_without_parent_reject_unexpected_lookup(self):
        for fixture in (BIRTH_FIXTURE_SCHEMAS[0], BIRTH_FIXTURE_SCHEMAS[3]):
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                data = Wdc5Builder(fixture, [values], ids=[17], parents=[]).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_birth_tables_reject_layout_and_field_count_mismatches(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-birth-mismatch.db2"
            for fixture in BIRTH_FIXTURE_SCHEMAS:
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                base = Wdc5Builder(fixture, [values], ids=[17]).build()
                for label, offset, value in (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file count", 140, len(fixture.fields) + 1),
                    ("total count", 176, len(fixture.fields) + 1),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_birth_strings_reject_bad_offset_unterminated_text_and_invalid_utf8(self):
        for fixture, string_fields in ((BIRTH_FIXTURE_SCHEMAS[0], range(5)),
                                       (BIRTH_FIXTURE_SCHEMAS[2], range(2))):
            for field in string_fields:
                values = {field: "ok", fixture.index: 17}
                builder = Wdc5Builder(fixture, [values])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                strings_start = records_start + builder.record_size
                for mutation in ("offset", "terminator", "utf8"):
                    with self.subTest(table=fixture.name, field=field, mutation=mutation):
                        data = bytearray(base)
                        if mutation == "offset":
                            at = records_start + builder.offsets[field] // 8
                            _put32(data, at, builder.record_size + 3 - builder.offsets[field] // 8)
                        elif mutation == "terminator":
                            data[strings_start + 2] = ord("x")
                        else:
                            data[strings_start] = 0xFF
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                        self.assertEqual(caught.exception.code, "invalid_record_string")

    def test_birth_masks_reject_a_record_missing_the_last_array_element(self):
        for fixture in (BIRTH_FIXTURE_SCHEMAS[1], BIRTH_FIXTURE_SCHEMAS[2], BIRTH_FIXTURE_SCHEMAS[3]):
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                builder = Wdc5Builder(fixture, [values], ids=[17])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                record_end = records_start + builder.record_size
                data = bytearray(base[:record_end - 4] + base[record_end:])
                _put32(data, 144, builder.record_size - 4)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_birth_tables_accept_complete_empty_files_with_valid_parent_flags(self):
        for fixture, expected_size in zip(BIRTH_FIXTURE_SCHEMAS, (264, 232, 276, 224, 212)):
            for parent_count in ((0, 1) if fixture.parent >= 0 else (0,)):
                with self.subTest(table=fixture.name, parent_count=parent_count):
                    parents = [] if parent_count else None
                    single = Wdc5Builder(fixture, [], ids=[], parents=parents).build()
                    self.assertEqual(db2_oracle.decode(single, db2_oracle.SCHEMAS[fixture.name]), {})
                    zero = bytearray(_zero_section_file(fixture, 0))
                    _put32(zero, 184, parent_count)
                    self.assertEqual(len(zero), expected_size)
                    self.assertEqual(db2_oracle.decode(bytes(zero), db2_oracle.SCHEMAS[fixture.name]), {})

    def test_birth_empty_files_reject_missing_metadata_trailing_bytes_and_payloads(self):
        for fixture in BIRTH_FIXTURE_SCHEMAS:
            base = _zero_section_file(fixture, 0)
            payload = bytearray(base)
            _put32(payload, 188, len(fixture.fields) * 24)
            for label, data, code in (
                ("header only", base[:HEADER_SIZE], "record_data_bounds"),
                ("trailing byte", base + b"\x00", "record_data_bounds"),
                ("column payload", bytes(payload), "unsupported_record_variant"),
            ):
                with self.subTest(table=fixture.name, mutation=label):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, code)

    def test_initialization_table_transcriptions_match_pinned_cpp(self):
        expected = (
            ("Map", 0xD43AFAC3, -1, 26,
             (("string", 32, 1), ("string", 32, 1), ("string", 32, 1),
              ("string", 32, 1), ("string", 32, 1), ("string", 32, 1),
              ("float", 32, 2), ("unsigned", 8, 1), ("signed", 8, 1),
              ("unsigned", 8, 1), ("unsigned", 16, 1), ("signed", 16, 1),
              ("signed", 16, 1), ("signed", 16, 1), ("signed", 16, 1),
              ("unsigned", 8, 1), ("float", 32, 1), ("signed", 16, 1),
              ("unsigned", 8, 1), ("signed", 16, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 3))),
            ("PowerType", 0x14BBEEA1, 2, 13,
             (("string", 32, 1), ("string", 32, 1), ("unsigned", 32, 1),
              ("signed", 8, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1),
              ("signed", 32, 1), ("float", 32, 1), ("float", 32, 1), ("signed", 32, 1))),
            ("Movie", 0xF53888FA, -1, 6,
             (("string", 32, 1), ("unsigned", 8, 1), ("unsigned", 8, 1),
              ("unsigned", 32, 1), ("unsigned", 32, 1), ("unsigned", 32, 1))),
        )
        for name, layout, index, field_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual(schema.index, index)
                self.assertEqual(schema.parent, -1)
                self.assertEqual(len(schema.fields), field_count)
                self.assertEqual(tuple((f.kind, f.bits, f.count) for f in schema.fields), fields)
                for field in schema.fields:
                    if field.kind in ("signed", "unsigned"):
                        self.assertEqual(field.signed, field.kind == "signed")

    def test_map_synthetic_strings_float_pair_signed_array_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            INITIALIZATION_FIXTURE_SCHEMAS[0],
            [{0: "Mapa_ñ", 1: "Zona\t名", 2: "", 3: "Sub\nzona",
              4: "Descripción\rX", 5: "終\\fin", 6: [0x80000000, 0x7FC12345],
              7: 0xFF, 8: 0x80, 9: 0x80, 10: 0xFFFF, 11: 0x8000,
              12: 0xFFFF, 13: 0x7FFF, 14: -2, 15: 0xFF, 16: 1,
              17: 0x8000, 18: 0xFF, 19: 0x7FFF, 20: 0x80000000,
              21: 0x7FFFFFFF, 22: -1, 23: 0, 24: 42,
              25: [0x80000000, 0x7FFFFFFF, -1]}],
            [17],
            "forever-db2-dump v1 Map\n"
            "17\tt:Mapa_ñ\tt:Zona\\t名\tt:\tt:Sub\\nzona\tt:Descripción\\rX\tt:終\\\\fin"
            "\tf:0x80000000,f:0x7fc12345\tu:255\ti:-128\tu:128\tu:65535"
            "\ti:-32768\ti:-1\ti:32767\ti:-2\tu:255\tf:0x00000001\ti:-32768"
            "\tu:255\ti:32767\ti:-2147483648\ti:2147483647\ti:-1\ti:0\ti:42"
            "\ti:-2147483648,i:2147483647,i:-1\n"
            "900\tt:Mapa_ñ\tt:Zona\\t名\tt:\tt:Sub\\nzona\tt:Descripción\\rX\tt:終\\\\fin"
            "\tf:0x80000000,f:0x7fc12345\tu:255\ti:-128\tu:128\tu:65535"
            "\ti:-32768\ti:-1\ti:32767\ti:-2\tu:255\tf:0x00000001\ti:-32768"
            "\tu:255\ti:32767\ti:-2147483648\ti:2147483647\ti:-1\ti:0\ti:42"
            "\ti:-2147483648,i:2147483647,i:-1\n",
            copies=[(900, 17)],
        )

    def test_power_type_nonlocalized_strings_float_bits_and_inline_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            INITIALIZATION_FIXTURE_SCHEMAS[1],
            [{0: "ENERGÍA\t名", 1: "tag\\x\n", 2: 17, 3: 0x80,
              4: 0x80000000, 5: 0x7FFFFFFF, 6: -1, 7: 0, 8: 42, 9: -2,
              10: 0x80000000, 11: 0xFFC54321, 12: -3}],
            [17],
            "forever-db2-dump v1 PowerType\n"
            "17\tt:ENERGÍA\\t名\tt:tag\\\\x\\n\tu:17\ti:-128\ti:-2147483648"
            "\ti:2147483647\ti:-1\ti:0\ti:42\ti:-2\tf:0x80000000\tf:0xffc54321\ti:-3\n"
            "900\tt:ENERGÍA\\t名\tt:tag\\\\x\\n\tu:900\ti:-128\ti:-2147483648"
            "\ti:2147483647\ti:-1\ti:0\ti:42\ti:-2\tf:0x80000000\tf:0xffc54321\ti:-3\n",
            copies=[(900, 17)],
        )

    def test_movie_utf8_unsigned_limits_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            INITIALIZATION_FIXTURE_SCHEMAS[2],
            [{0: "Película\t名\n\\fin", 1: 0xFF, 2: 0x80,
              3: 0xFFFFFFFF, 4: 0x80000000, 5: 0x7FFFFFFF}],
            [17],
            "forever-db2-dump v1 Movie\n"
            "17\tt:Película\\t名\\n\\\\fin\tu:255\tu:128\tu:4294967295\tu:2147483648\tu:2147483647\n"
            "900\tt:Película\\t名\\n\\\\fin\tu:255\tu:128\tu:4294967295\tu:2147483648\tu:2147483647\n",
            copies=[(900, 17)],
        )

    def test_initialization_float_bits_preserve_zero_subnormal_infinities_and_nan_payloads(self):
        pairs = ((0x00000000, 0x80000000), (0x00000001, 0x7F7FFFFF),
                 (0x7F800000, 0xFF800000), (0x7FC12345, 0xFFC54321))
        tokens = (("f:0x00000000", "f:0x80000000"), ("f:0x00000001", "f:0x7f7fffff"),
                  ("f:0x7f800000", "f:0xff800000"), ("f:0x7fc12345", "f:0xffc54321"))
        map_fixture, power_fixture = INITIALIZATION_FIXTURE_SCHEMAS[:2]
        map_data = Wdc5Builder(
            map_fixture, [{6: list(pair), 16: 0x3F800000} for pair in pairs],
            ids=[11, 12, 13, 14],
        ).build()
        power_data = Wdc5Builder(
            power_fixture, [{2: index + 11, 10: pair[0], 11: pair[1]}
                            for index, pair in enumerate(pairs)],
        ).build()
        map_lines = db2_oracle.render(db2_oracle.decode(map_data, db2_oracle.MAP), "Map").splitlines()[1:]
        power_lines = db2_oracle.render(db2_oracle.decode(power_data, db2_oracle.POWER_TYPE), "PowerType").splitlines()[1:]
        for row, expected in enumerate(tokens):
            with self.subTest(row=row):
                self.assertEqual(map_lines[row].split("\t")[7], ",".join(expected))
                self.assertEqual(map_lines[row].split("\t")[17], "f:0x3f800000")
                self.assertEqual(power_lines[row].split("\t")[11:13], list(expected))

    def test_initialization_tables_reject_layout_counts_and_unexpected_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-initialization-mismatch.db2"
            for fixture in INITIALIZATION_FIXTURE_SCHEMAS:
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                base = Wdc5Builder(fixture, [values], ids=[17]).build()
                for label, offset, value in (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file count", 140, len(fixture.fields) + 1),
                    ("total count", 176, len(fixture.fields) + 1),
                    ("unexpected parent", 184, 1),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_initialization_strings_reject_bad_offsets_terminators_and_utf8(self):
        for fixture, string_fields in (
            (INITIALIZATION_FIXTURE_SCHEMAS[0], range(6)),
            (INITIALIZATION_FIXTURE_SCHEMAS[1], range(2)),
            (INITIALIZATION_FIXTURE_SCHEMAS[2], range(1)),
        ):
            for field in string_fields:
                values = {field: "ok"}
                if fixture.index >= 0:
                    values[fixture.index] = 17
                builder = Wdc5Builder(fixture, [values], ids=[17])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                strings_start = records_start + builder.record_size
                for mutation in ("offset", "terminator", "utf8"):
                    with self.subTest(table=fixture.name, field=field, mutation=mutation):
                        data = bytearray(base)
                        if mutation == "offset":
                            field_offset = builder.offsets[field] // 8
                            _put32(data, records_start + field_offset,
                                   builder.record_size + 3 - field_offset)
                        elif mutation == "terminator":
                            data[strings_start + 2] = ord("x")
                        else:
                            data[strings_start] = 0xFF
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                        self.assertEqual(caught.exception.code, "invalid_record_string")

    def test_map_rejects_record_missing_float_pair_or_signed_array_element(self):
        fixture = INITIALIZATION_FIXTURE_SCHEMAS[0]
        builder = Wdc5Builder(fixture, [{}], ids=[17])
        base = builder.build()
        records_start = HEADER_SIZE + SECTION_SIZE + 26 * 28
        record_end = records_start + builder.record_size
        for record_size in (builder.offsets[6] // 8 + 4, builder.record_size - 4):
            with self.subTest(record_size=record_size):
                data = bytearray(base[:records_start + record_size] + base[record_end:])
                _put32(data, 144, record_size)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.MAP)
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_map_rejects_unreviewed_sparse_encrypted_and_multisection_variants(self):
        base = Wdc5Builder(INITIALIZATION_FIXTURE_SCHEMAS[0], [{}], ids=[17]).build()
        for variant in ("sparse", "encrypted", "multiple sections"):
            with self.subTest(variant=variant):
                data = bytearray(base)
                if variant == "sparse":
                    _put16(data, 172, 1)
                elif variant == "encrypted":
                    data[HEADER_SIZE] = 1
                else:
                    data[HEADER_SIZE + SECTION_SIZE:HEADER_SIZE + SECTION_SIZE] = bytes(SECTION_SIZE)
                    _put32(data, 200, 2)
                    _put32(data, HEADER_SIZE + 8, HEADER_SIZE + 2 * SECTION_SIZE + 26 * 28)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.MAP)
                self.assertEqual(caught.exception.code, "unsupported_record_variant")

    def test_initialization_tables_reject_duplicate_record_ids(self):
        for fixture in INITIALIZATION_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                values = {fixture.index: 17} if fixture.index >= 0 else {}
                data = Wdc5Builder(fixture, [values, values], ids=[17, 17]).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "invalid_record_ids")

    def test_initialization_tables_accept_complete_empty_files(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-initialization-empty.db2"
            for fixture, expected_size in zip(INITIALIZATION_FIXTURE_SCHEMAS, (308, 256, 228)):
                with self.subTest(table=fixture.name):
                    single = Wdc5Builder(fixture, [], ids=[]).build()
                    self.assertEqual(db2_oracle.decode(single, db2_oracle.SCHEMAS[fixture.name]), {})
                    zero = _zero_section_file(fixture, 0)
                    self.assertEqual(len(zero), expected_size)
                    path.write_bytes(zero)
                    self.assertEqual(db2_oracle.dump_file(path, fixture.name),
                                     "forever-db2-dump v1 " + fixture.name + "\n")

    def test_initialization_empty_files_reject_missing_metadata_extra_bytes_and_payloads(self):
        for fixture in INITIALIZATION_FIXTURE_SCHEMAS:
            base = _zero_section_file(fixture, 0)
            payload = bytearray(base)
            _put32(payload, 188, len(fixture.fields) * 24)
            for label, data, code in (
                ("header only", base[:HEADER_SIZE], "record_data_bounds"),
                ("trailing byte", base + b"\x00", "record_data_bounds"),
                ("column payload", bytes(payload), "unsupported_record_variant"),
            ):
                with self.subTest(table=fixture.name, mutation=label):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, code)

    def test_spell_dependency_transcriptions_match_pinned_cpp(self):
        expected = (
            ("SpellCastTimes", 0x75B6BD3A, 2, (("signed", 32, 1), ("signed", 32, 1))),
            ("SpellDuration", 0xA931BD2B, 3,
             (("signed", 32, 1), ("signed", 32, 1), ("signed", 32, 1))),
            ("SpellRadius", 0xF9A913EE, 4,
             (("float", 32, 1), ("float", 32, 1), ("float", 32, 1), ("float", 32, 1))),
            ("SpellProcsPerMinute", 0x2227014B, 2, (("float", 32, 1), ("signed", 32, 1))),
        )
        for name, layout, field_count, fields in expected:
            with self.subTest(table=name):
                schema = db2_oracle.SCHEMAS[name]
                self.assertEqual(schema.name, name)
                self.assertEqual(schema.layout_hash, layout)
                self.assertEqual((schema.index, schema.parent), (-1, -1))
                self.assertEqual(len(schema.fields), field_count)
                self.assertEqual(tuple((f.kind, f.bits, f.count) for f in schema.fields), fields)
                for field in schema.fields:
                    if field.kind == "signed":
                        self.assertTrue(field.signed)

    def test_spell_cast_times_signed_extrema_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            SPELL_DEPENDENCY_FIXTURE_SCHEMAS[0],
            [{0: 0x80000000, 1: 0x7FFFFFFF}, {0: -1, 1: 0}], [17, 99],
            "forever-db2-dump v1 SpellCastTimes\n"
            "17\ti:-2147483648\ti:2147483647\n99\ti:-1\ti:0\n"
            "900\ti:-2147483648\ti:2147483647\n",
            copies=[(900, 17)],
        )

    def test_spell_duration_signed_extrema_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            SPELL_DEPENDENCY_FIXTURE_SCHEMAS[1],
            [{0: 0x80000000, 1: -1, 2: 0x7FFFFFFF}, {0: 0, 1: 1, 2: -2}], [17, 99],
            "forever-db2-dump v1 SpellDuration\n"
            "17\ti:-2147483648\ti:-1\ti:2147483647\n99\ti:0\ti:1\ti:-2\n"
            "900\ti:-2147483648\ti:-1\ti:2147483647\n",
            copies=[(900, 17)],
        )

    def test_spell_radius_four_scalar_float_bits_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            SPELL_DEPENDENCY_FIXTURE_SCHEMAS[2],
            [{0: 0x00000000, 1: 0x80000000, 2: 0x00000001, 3: 0x7F7FFFFF},
             {0: 0x7F800000, 1: 0xFF800000, 2: 0x7FC12345, 3: 0xFFC54321}],
            [17, 99],
            "forever-db2-dump v1 SpellRadius\n"
            "17\tf:0x00000000\tf:0x80000000\tf:0x00000001\tf:0x7f7fffff\n"
            "99\tf:0x7f800000\tf:0xff800000\tf:0x7fc12345\tf:0xffc54321\n"
            "900\tf:0x7f800000\tf:0xff800000\tf:0x7fc12345\tf:0xffc54321\n",
            copies=[(900, 99)],
        )

    def test_spell_procs_per_minute_float_bits_signed_extrema_and_copy_match_golden(self):
        self._assert_reviewed_fixture_dump(
            SPELL_DEPENDENCY_FIXTURE_SCHEMAS[3],
            [{0: 0x80000000, 1: 0x80000000}, {0: 0x7FC12345, 1: 0x7FFFFFFF},
             {0: 0x7F800000, 1: -1}, {0: 0xFF800000, 1: 0}],
            [17, 99, 100, 101],
            "forever-db2-dump v1 SpellProcsPerMinute\n"
            "17\tf:0x80000000\ti:-2147483648\n99\tf:0x7fc12345\ti:2147483647\n"
            "100\tf:0x7f800000\ti:-1\n101\tf:0xff800000\ti:0\n"
            "900\tf:0x7fc12345\ti:2147483647\n",
            copies=[(900, 99)],
        )

    def test_spell_cast_times_common_and_pallet_values_survive_copy(self):
        fixture = SPELL_DEPENDENCY_FIXTURE_SCHEMAS[0]
        data = Wdc5Builder(
            fixture, [{1: 0}, {1: 1}], ids=[17, 99], max_id=900,
            modes={0: (2, 0xFFFFFFFF, 0, 0), 1: (3, 0, 1, 0)},
            common={0: (0xFFFFFFFF, [(17, 0x80000000)])},
            pallet={1: [0x7FFFFFFF, 0xFFFFFFFF]}, copies=[(900, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, db2_oracle.SPELL_CAST_TIMES), fixture.name),
            "forever-db2-dump v1 SpellCastTimes\n"
            "17\ti:-2147483648\ti:2147483647\n99\ti:-1\ti:-1\n"
            "900\ti:-2147483648\ti:2147483647\n",
        )

    def test_spell_duration_signed_immediate_common_and_pallet_match_golden(self):
        fixture = SPELL_DEPENDENCY_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(
            fixture, [{0: 0x80, 2: 0}, {0: 0x7F, 2: 1}], ids=[17, 99], max_id=900,
            modes={0: (5, 0, 8, 0), 1: (2, 0xFFFFFFFF, 0, 0), 2: (3, 8, 1, 0)},
            common={1: (0xFFFFFFFF, [(17, 0x80000000)])},
            pallet={2: [0x7FFFFFFF, 0x80000000]}, copies=[(900, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, db2_oracle.SPELL_DURATION), fixture.name),
            "forever-db2-dump v1 SpellDuration\n"
            "17\ti:-128\ti:-2147483648\ti:2147483647\n"
            "99\ti:127\ti:-1\ti:-2147483648\n"
            "900\ti:-128\ti:-2147483648\ti:2147483647\n",
        )

    def test_spell_radius_common_and_pallet_preserve_float_bits(self):
        fixture = SPELL_DEPENDENCY_FIXTURE_SCHEMAS[2]
        data = Wdc5Builder(
            fixture, [{1: 0, 2: 1, 3: 0x3F800000}, {1: 1, 2: 0x7F7FFFFF, 3: 0xBF800000}],
            ids=[17, 99], max_id=900,
            modes={0: (2, 0x80000000, 0, 0), 1: (3, 0, 1, 0)},
            common={0: (0x80000000, [(17, 0x7FC12345)])},
            pallet={1: [0xFF800000, 0x7F800000]}, copies=[(900, 17)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, db2_oracle.SPELL_RADIUS), fixture.name),
            "forever-db2-dump v1 SpellRadius\n"
            "17\tf:0x7fc12345\tf:0xff800000\tf:0x00000001\tf:0x3f800000\n"
            "99\tf:0x80000000\tf:0x7f800000\tf:0x7f7fffff\tf:0xbf800000\n"
            "900\tf:0x7fc12345\tf:0xff800000\tf:0x00000001\tf:0x3f800000\n",
        )

    def test_spell_procs_per_minute_pallet_float_common_int_match_golden(self):
        fixture = SPELL_DEPENDENCY_FIXTURE_SCHEMAS[3]
        data = Wdc5Builder(
            fixture, [{0: 0}, {0: 1}], ids=[17, 99], max_id=900,
            modes={0: (3, 0, 1, 0), 1: (2, 0xFFFFFFFF, 0, 0)},
            pallet={0: [0x80000000, 0xFFC54321]},
            common={1: (0xFFFFFFFF, [(17, 0x80000000)])}, copies=[(900, 99)],
        ).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, db2_oracle.SPELL_PROCS_PER_MINUTE), fixture.name),
            "forever-db2-dump v1 SpellProcsPerMinute\n"
            "17\tf:0x80000000\ti:-2147483648\n99\tf:0xffc54321\ti:-1\n"
            "900\tf:0xffc54321\ti:-1\n",
        )

    def test_spell_dependencies_apply_ordered_copies_and_skip_absent_or_zero_sources(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                data = Wdc5Builder(
                    fixture, [{0: 1}, {0: 2}], ids=[0, 17], max_id=1000,
                    copies=[(500, 400), (400, 17), (600, 400), (700, 0), (800, 777)],
                ).build()
                decoded = db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(list(decoded), [0, 17, 400, 600])
                self.assertEqual(decoded[0][0][0].number, 1)
                for record_id in (17, 400, 600):
                    self.assertEqual(decoded[record_id][0][0].number, 2)

    def test_spell_dependencies_reject_bad_ids_and_undefined_self_copy(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            for label, ids, max_id, copies, code in (
                ("duplicate", [17, 17], 100, [], "invalid_record_ids"),
                ("direct above header", [17, 101], 100, [], "invalid_record_ids"),
                ("copy above header", [17, 99], 100, [(101, 17)], "invalid_record_ids"),
                ("self copy", [17, 99], 100, [(17, 17)], "unsupported_record_variant"),
            ):
                with self.subTest(table=fixture.name, mutation=label):
                    data = Wdc5Builder(fixture, [{}, {}], ids=ids, max_id=max_id, copies=copies).build()
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, code)

    def test_spell_dependencies_reject_pallet_index_outside_declared_values(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                data = Wdc5Builder(
                    fixture, [{0: 1}], ids=[17], modes={0: (3, 0, 1, 0)}, pallet={0: [0]},
                ).build()
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "invalid_column_data")

    def test_spell_dependencies_reject_layout_counts_and_unexpected_parent(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-spell-dependency-mismatch.db2"
            for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
                base = Wdc5Builder(fixture, [{}], ids=[17]).build()
                for label, offset, value in (
                    ("layout", 156, fixture.layout_hash ^ 1),
                    ("file count", 140, len(fixture.fields) + 1),
                    ("total count", 176, len(fixture.fields) + 1),
                    ("unexpected parent", 184, 1),
                ):
                    with self.subTest(table=fixture.name, mutation=label):
                        data = bytearray(base)
                        _put32(data, offset, value)
                        path.write_bytes(data)
                        with self.assertRaises(db2_oracle.OracleError) as caught:
                            db2_oracle.dump_file(path, fixture.name)
                        self.assertEqual(caught.exception.code, "unsupported_table_schema")

    def test_spell_dependencies_reject_truncated_metadata_records_and_ids(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            builder = Wdc5Builder(fixture, [{}], ids=[17])
            base = builder.build()
            records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
            for label, cut, code in (
                ("header", HEADER_SIZE - 1, "metadata_truncated"),
                ("columns", records_start - 1, "record_data_bounds"),
                ("record", records_start + builder.record_size - 1, "record_data_bounds"),
                ("id table", len(base) - 1, "record_data_bounds"),
            ):
                with self.subTest(table=fixture.name, truncation=label):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(base[:cut], db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, code)

    def test_spell_dependencies_reject_record_missing_its_last_scalar(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            with self.subTest(table=fixture.name):
                builder = Wdc5Builder(fixture, [{}], ids=[17])
                base = builder.build()
                records_start = HEADER_SIZE + SECTION_SIZE + len(fixture.fields) * 28
                record_end = records_start + builder.record_size
                data = bytearray(base[:record_end - 4] + base[record_end:])
                _put32(data, 144, builder.record_size - 4)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_spell_dependencies_accept_complete_empty_files(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-spell-dependency-empty.db2"
            for fixture, expected_size in zip(SPELL_DEPENDENCY_FIXTURE_SCHEMAS, (212, 216, 220, 212)):
                with self.subTest(table=fixture.name):
                    single = Wdc5Builder(fixture, [], ids=[]).build()
                    self.assertEqual(db2_oracle.decode(single, db2_oracle.SCHEMAS[fixture.name]), {})
                    zero = _zero_section_file(fixture, 0)
                    self.assertEqual(len(zero), expected_size)
                    path.write_bytes(zero)
                    self.assertEqual(db2_oracle.dump_file(path, fixture.name),
                                     "forever-db2-dump v1 " + fixture.name + "\n")

    def test_spell_dependency_empty_files_reject_truncation_trailing_bytes_and_payloads(self):
        for fixture in SPELL_DEPENDENCY_FIXTURE_SCHEMAS:
            base = _zero_section_file(fixture, 0)
            payload = bytearray(base)
            _put32(payload, 188, len(fixture.fields) * 24)
            for label, data, code in (
                ("header only", base[:HEADER_SIZE], "record_data_bounds"),
                ("trailing byte", base + b"\x00", "record_data_bounds"),
                ("column payload", bytes(payload), "unsupported_record_variant"),
            ):
                with self.subTest(table=fixture.name, mutation=label):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, code)

    def test_reviewed_long_tables_decode_u64_and_five_count_arrays(self):
        # The three reviewed FT_LONG tables: a >32-bit field 0 must survive as a
        # full unsigned 64-bit value, and the 5-count arrays must print all five
        # comma-joined values. A u32 truncation or a dropped array element would
        # change the golden text below.
        item = db2_oracle.ITEM_EXTENDED_COST
        item_rows = [
            {
                0: 0x1_0000_0001,
                1: 0xFFFF,
                2: 0x80,  # -128
                3: 0xFFFFFFFF,  # -1
                4: 0xFF,  # 255
                5: 0x7FFFFFFF,  # 2147483647
                6: 0x00,
                7: [0xFFFFFFFF, 0x00000000, 0x00000007, 0x80000000, 0x7FFFFFFF],
                8: [1, 2, 3, 4, 5],
                9: [6, 7, 8, 9, 10],
                10: [11, 12, 13, 14, 15],
            }
        ]
        data = Wdc5Builder(item, item_rows, min_id=0, max_id=100, ids=[12]).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, item), item.name),
            "forever-db2-dump v1 ItemExtendedCost\n"
            "12\tu:4294967297\tu:65535\ti:-128\ti:-1\tu:255\ti:2147483647"
            "\tu:0\ti:-1,i:0,i:7,i:-2147483648,i:2147483647"
            "\tu:1,u:2,u:3,u:4,u:5\tu:6,u:7,u:8,u:9,u:10"
            "\tu:11,u:12,u:13,u:14,u:15\n",
        )

        transmog = db2_oracle.TRANSMOG_OUTFIT_ENTRY
        transmog_rows = [
            {
                0: 0x1_0000_0001,
                1: "Outfit",
                2: 42,  # inline id
                3: 0xFFFFFFFF,  # -1
                4: 0xFF,  # 255
                5: 0x00000007,
                6: 0x80,  # 128 (unsigned)
                7: 0x3F800000,  # float 1.0
                8: 0x7FFFFFFF,
            }
        ]
        data = Wdc5Builder(transmog, transmog_rows, min_id=0, max_id=100).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, transmog), transmog.name),
            "forever-db2-dump v1 TransmogOutfitEntry\n"
            "42\tu:4294967297\tt:Outfit\tu:42\ti:-1\tu:255\ti:7\tu:128"
            "\tf:0x3f800000\ti:2147483647\n",
        )

        bank = db2_oracle.BANK_TAB
        bank_rows = [
            {
                0: 0x1_0000_0001,
                1: 0xFF,  # 255 (unsigned)
                2: 0x80,  # -128
                3: 0xFFFFFFFF,  # -1
                4: 0x00000000,
                5: 0x00000007,
                6: 0x80000000,  # -2147483648
                7: 0x7FFFFFFF,  # 2147483647
                8: 0x0000002A,  # 42
            }
        ]
        data = Wdc5Builder(bank, bank_rows, min_id=0, max_id=100, ids=[3]).build()
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, bank), bank.name),
            "forever-db2-dump v1 BankTab\n"
            "3\tu:4294967297\tu:255\ti:-128\ti:-1\ti:0\ti:7\ti:-2147483648"
            "\ti:2147483647\ti:42\n",
        )

    def test_reviewed_long_table_transcriptions_are_present(self):
        # Layout/index/parent plus the exact field list for the three reviewed
        # FT_LONG tables, transcribed from DB2Metadata.h and mirrored in the Rust
        # `schema.rs` transcription.
        item = db2_oracle.SCHEMAS["ItemExtendedCost"]
        self.assertEqual(item.layout_hash, 0x22331DAB)
        self.assertEqual(item.index, -1)
        self.assertEqual(item.parent, -1)
        self.assertEqual(len(item.fields), 11)
        self.assertEqual(
            (item.fields[0].kind, item.fields[0].bits, item.fields[0].count),
            ("unsigned", 64, 1),
        )
        self.assertEqual((item.fields[1].kind, item.fields[1].bits), ("unsigned", 16))
        self.assertEqual(
            (item.fields[2].kind, item.fields[2].bits, item.fields[2].signed),
            ("signed", 8, True),
        )
        self.assertEqual(
            (item.fields[3].kind, item.fields[3].bits, item.fields[3].signed),
            ("signed", 32, True),
        )
        self.assertEqual((item.fields[4].kind, item.fields[4].bits), ("unsigned", 8))
        self.assertEqual(
            (item.fields[5].kind, item.fields[5].bits, item.fields[5].signed),
            ("signed", 32, True),
        )
        self.assertEqual((item.fields[6].kind, item.fields[6].bits), ("unsigned", 8))
        self.assertEqual(
            (item.fields[7].kind, item.fields[7].bits, item.fields[7].count),
            ("signed", 32, 5),
        )
        self.assertEqual(
            (item.fields[8].kind, item.fields[8].bits, item.fields[8].count),
            ("unsigned", 16, 5),
        )
        self.assertEqual(
            (item.fields[9].kind, item.fields[9].bits, item.fields[9].count),
            ("unsigned", 16, 5),
        )
        self.assertEqual(
            (item.fields[10].kind, item.fields[10].bits, item.fields[10].count),
            ("unsigned", 32, 5),
        )

        transmog = db2_oracle.SCHEMAS["TransmogOutfitEntry"]
        self.assertEqual(transmog.layout_hash, 0x91D51304)
        self.assertEqual(transmog.index, 2)
        self.assertEqual(transmog.parent, -1)
        self.assertEqual(len(transmog.fields), 9)
        self.assertEqual(
            (transmog.fields[0].kind, transmog.fields[0].bits),
            ("unsigned", 64),
        )
        self.assertEqual(
            (transmog.fields[1].kind, transmog.fields[1].bits), ("string", 32)
        )
        self.assertEqual(
            (transmog.fields[2].kind, transmog.fields[2].bits),
            ("unsigned", 32),
        )
        for index in (3, 5, 8):
            self.assertEqual(
                (
                    transmog.fields[index].kind,
                    transmog.fields[index].bits,
                    transmog.fields[index].signed,
                ),
                ("signed", 32, True),
                "transmog %d" % index,
            )
        self.assertEqual(
            (transmog.fields[4].kind, transmog.fields[4].bits), ("unsigned", 8)
        )
        self.assertEqual(
            (transmog.fields[6].kind, transmog.fields[6].bits), ("unsigned", 8)
        )
        self.assertEqual(
            (transmog.fields[7].kind, transmog.fields[7].bits),
            ("float", 32),
        )

        bank = db2_oracle.SCHEMAS["BankTab"]
        self.assertEqual(bank.layout_hash, 0x5B2F9B36)
        self.assertEqual(bank.index, -1)
        self.assertEqual(bank.parent, -1)
        self.assertEqual(len(bank.fields), 9)
        self.assertEqual((bank.fields[0].kind, bank.fields[0].bits), ("unsigned", 64))
        self.assertEqual((bank.fields[1].kind, bank.fields[1].bits), ("unsigned", 8))
        self.assertEqual(
            (bank.fields[2].kind, bank.fields[2].bits, bank.fields[2].signed),
            ("signed", 8, True),
        )
        for index in range(3, 9):
            self.assertEqual(
                (bank.fields[index].kind, bank.fields[index].bits, bank.fields[index].signed),
                ("signed", 32, True),
                "bank %d" % index,
            )

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

    def test_item_zero_section_files_include_complete_primitive_metadata(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic-zero-section.db2"
            for fixture, expected_size in zip(ITEM_FIXTURE_SCHEMAS, (228, 208, 212)):
                for parent_count in ((0, 1) if fixture.parent >= 0 else (0,)):
                    with self.subTest(table=fixture.name, parent_count=parent_count):
                        data = bytearray(_zero_section_file(fixture, 0))
                        _put32(data, 184, parent_count)
                        self.assertEqual(len(data), expected_size)
                        path.write_bytes(data)
                        self.assertEqual(
                            db2_oracle.dump_file(path, fixture.name),
                            "forever-db2-dump v1 " + fixture.name + "\n",
                        )

    def test_zero_section_rejects_missing_primitive_metadata_and_trailing_bytes(self):
        for fixture in ITEM_FIXTURE_SCHEMAS:
            base = _zero_section_file(fixture, 0)
            for label, data in (
                ("header only", base[:HEADER_SIZE]),
                ("partial primitive field", base[:-1]),
                ("trailing byte", base + b"\x00"),
            ):
                with self.subTest(table=fixture.name, mutation=label):
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(data, db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_zero_section_rejects_declared_string_and_column_storage_payloads(self):
        for fixture in ITEM_FIXTURE_SCHEMAS:
            for label, offset in (
                ("strings", 148), ("columns", 188), ("common", 192), ("pallet", 196),
            ):
                with self.subTest(table=fixture.name, payload=label):
                    data = bytearray(_zero_section_file(fixture, 0))
                    _put32(data, offset, 4)
                    with self.assertRaises(db2_oracle.OracleError) as caught:
                        db2_oracle.decode(bytes(data), db2_oracle.SCHEMAS[fixture.name])
                    self.assertEqual(caught.exception.code, "unsupported_record_variant")

    def test_zero_section_does_not_require_a_specific_record_size_or_id_flag(self):
        for record_size in (0, 8):
            for flags in (0, 4):
                with self.subTest(record_size=record_size, flags=flags):
                    data = bytearray(_zero_section_file(ITEM_FIXTURE_SCHEMAS[0], 0))
                    _put32(data, 144, record_size)
                    _put16(data, 172, flags)
                    self.assertEqual(db2_oracle.decode(bytes(data), db2_oracle.ITEM_SPEC), {})

    def test_metadata_only_parse_does_not_claim_zero_section_body_completeness(self):
        data = _zero_section_file(ITEM_FIXTURE_SCHEMAS[0], 0)[:HEADER_SIZE]
        header, sections = db2_oracle.parse(data)
        self.assertEqual(header.record_count, 0)
        self.assertEqual(sections, [])
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, db2_oracle.ITEM_SPEC)
        self.assertEqual(caught.exception.code, "record_data_bounds")

    def test_out_of_scope_variants_are_rejected(self):
        base = _inline_file()
        mutations = [
            ("sparse flag", 172, 2, 1, "unsupported_record_variant"),
            # The reviewed SYNTHETIC_INLINE schema has no parent index field, so
            # declaring a parent-lookup block is a schema mismatch, not an
            # unsupported record variant (DB2FileLoader.cpp:1775).
            ("parent lookup", 184, 4, 1, "unsupported_table_schema"),
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


# Literal source transcription: ConversationLineMeta, pin 02245dcd, layout
# AA1E7988, index/parent -1, nine scalar fields. No generated shape consulted.
SPARSE_CONVERSATION_SCHEMA = Schema(
    "ConversationLine", 0xAA1E7988, -1,
    (Field("unsigned"), Field("unsigned"), Field("unsigned"),
     Field("signed", signed=True), Field("unsigned", 16), Field("unsigned", 16),
     Field("unsigned", 8), Field("unsigned", 8), Field("unsigned", 8)),
)
SPARSE_UNUSED = (0, 0, 0, 0, 16, 16, 24, 24, 24)
SPARSE_RAW = (0xFFFFFFFF, 0x80000000, 7, 0x80000000, 65535, 32768, 255, 128, 0)


def _sparse_numeric_file(ids=(7,), rows=(SPARSE_RAW,), copies=(), *, unused=SPARSE_UNUSED, columns=True):
    """Separate fixture writer for the sparse catalog wire layout.

    Fixed literal identity/field count, not the oracle's schema mapping.
    Primitive and storage offsets are deliberately irrelevant to this layout.
    """
    if len(ids) != len(rows):
        raise ValueError("row count")
    data = bytearray(244)
    data[:4] = b"WDC5"
    for offset, value in ((4, 5), (136, len(rows)), (140, 9), (144, 24),
                          (156, 0xAA1E7988), (160, 50), (164, 100000),
                          (176, 9), (188, 216 if columns else 0), (200, 1)):
        _put32(data, offset, value)
    _put16(data, 172, 5)
    _put16(data, 174, 0xFFFF)
    for ordinal, bits in enumerate(unused):
        data.extend(bits.to_bytes(2, "little", signed=True))
        data.extend((64000 - ordinal).to_bytes(2, "little"))
    if columns:
        for _ in range(9):
            entry = bytearray(24)
            _put16(entry, 0, 60000)
            data.extend(entry)
    payload = len(data)
    entries = []
    for row in rows:
        start = len(data)
        for value, bits in zip(row, unused):
            size = 4 - bits // 8
            data.extend(value.to_bytes(4, "little")[:size])
        data.append(0xA5)
        entries.append((start, len(data) - start))
    catalog = len(data)
    for record_id in ids:
        data.extend(record_id.to_bytes(4, "little"))
    for new, source in copies:
        data.extend(new.to_bytes(4, "little"))
        data.extend(source.to_bytes(4, "little"))
    for at, size in entries:
        data.extend(at.to_bytes(4, "little"))
        data.extend(size.to_bytes(2, "little"))
    for _ in ids:
        data.extend(b"\xff\xff\xff\xff")
    for offset, value in ((212, payload), (216, len(rows)), (224, catalog),
                          (228, len(ids) * 4), (236, len(rows)), (240, len(copies))):
        _put32(data, offset, value)
    return bytes(data)


class SparseNumericOracleTests(unittest.TestCase):
    def assert_code(self, data, code, schema=SPARSE_CONVERSATION_SCHEMA):
        with self.assertRaises(db2_oracle.OracleError) as caught:
            db2_oracle.decode(data, schema)
        self.assertEqual(caught.exception.code, code)

    def test_literal_catalog_transcription(self):
        self.assertEqual(db2_oracle.load_catalog()["ConversationLine"], SPARSE_CONVERSATION_SCHEMA)

    def test_literal_golden_with_and_without_storage_columns(self):
        golden = (
            "forever-db2-dump v1 ConversationLine\n"
            "7\tu:4294967295\tu:2147483648\tu:7\ti:-2147483648\tu:65535\tu:32768\tu:255\tu:128\tu:0\n"
            "90000\tu:4294967295\tu:2147483648\tu:7\ti:-2147483648\tu:65535\tu:32768\tu:255\tu:128\tu:0\n"
            "100000\tu:4294967295\tu:2147483648\tu:7\ti:-2147483648\tu:65535\tu:32768\tu:255\tu:128\tu:0\n"
        )
        for columns in (False, True):
            data = _sparse_numeric_file(copies=((90000, 7), (100000, 90000)), columns=columns)
            rows = db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)
            self.assertEqual(db2_oracle.render(rows, "ConversationLine"), golden)

    def test_compressed_primitive_int_masks_and_signed_extremes(self):
        unused = (20, 31, 0, 20, 16, 16, 24, 24, 24)
        raw = (0xFABC, 0xFF, 7, 0xF800, 65535, 32768, 255, 128, 0)
        data = _sparse_numeric_file(rows=(raw,), unused=unused)
        self.assertEqual(
            db2_oracle.render(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA), "ConversationLine"),
            "forever-db2-dump v1 ConversationLine\n"
            "7\tu:2748\tu:1\tu:7\ti:-2048\tu:65535\tu:32768\tu:255\tu:128\tu:0\n",
        )

    def test_catalog_zero_and_ids_below_minimum_are_not_repaired(self):
        data = _sparse_numeric_file(ids=(0, 7), rows=(SPARSE_RAW, SPARSE_RAW), copies=((9, 0), (0, 7)))
        rows = db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)
        self.assertEqual(list(rows), [0, 7])
        self.assertEqual(rows[0], rows[7])

    def test_copy_order_overwrite_missing_forward_and_out_of_range(self):
        second = (22,) + SPARSE_RAW[1:]
        data = _sparse_numeric_file(
            ids=(7, 8), rows=(SPARSE_RAW, second),
            copies=((30, 31), (31, 7), (8, 31), (40, 0), (41, 77),
                    (100001, 7), (42, 100001), (43, 0xFFFFFFFF)),
        )
        rows = db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)
        self.assertEqual(list(rows), [7, 8, 31])
        self.assertEqual(rows[7], rows[8])
        self.assertEqual(rows[8], rows[31])
        self.assertEqual(rows[8][0][0].number, 0xFFFFFFFF)

    def test_applicable_self_copy_rejected_missing_and_zero_omitted(self):
        self.assert_code(_sparse_numeric_file(copies=((8, 7), (8, 8))), "unsupported_record_variant")
        data = _sparse_numeric_file(ids=(0, 7), rows=(SPARSE_RAW, SPARSE_RAW), copies=((0, 0), (8, 8)))
        self.assertEqual(list(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)), [0, 7])

    def test_extent_overflow_duplicate_and_large_catalog_ids(self):
        self.assert_code(_sparse_numeric_file(ids=(7, 7), rows=(SPARSE_RAW, SPARSE_RAW)), "invalid_record_ids")
        self.assert_code(_sparse_numeric_file(ids=(100001,)), "invalid_record_ids")
        data = bytearray(_sparse_numeric_file(ids=(0xFFFFFFFE,)))
        _put32(data, 164, 0xFFFFFFFE)
        self.assertEqual(list(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)), [0xFFFFFFFE])
        _put32(data, 164, 0xFFFFFFFF)
        self.assert_code(data, "invalid_record_ids")

    def test_every_truncation_and_extra_tail_fails(self):
        data = _sparse_numeric_file()
        for end in range(244, len(data)):
            with self.subTest(end=end):
                self.assert_code(data[:end], "record_data_bounds")
        self.assert_code(data + b"\0", "record_data_bounds")

    def test_record_range_and_catalog_table_bounds(self):
        data = _sparse_numeric_file()
        for offset, value, code in (
            (212, 495, "record_data_bounds"), (224, 495, "record_data_bounds"),
            (216, 2, "record_data_bounds"), (236, 2, "record_data_bounds"),
            (228, 0, "invalid_record_ids"), (160, 100001, "record_data_bounds"),
        ):
            with self.subTest(offset=offset):
                changed = bytearray(data)
                _put32(changed, offset, value)
                self.assert_code(changed, code)
        entry = int.from_bytes(data[224:228], "little") + 4
        for at, size in ((495, 24), (0xFFFFFFFF, 24), (496, 0), (496, 1), (496, 65535)):
            changed = bytearray(data)
            _put32(changed, entry, at)
            _put16(changed, entry + 4, size)
            self.assert_code(changed, "record_data_bounds")

    def test_irrelevant_header_record_size_and_primitive_offsets_are_ignored(self):
        data = bytearray(_sparse_numeric_file())
        _put32(data, 144, 0)
        _put32(data, 180, 0xFFFFFFFF)
        rows = db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)
        self.assertEqual(rows[7][3][0].number, -2147483648)

    def test_catalog_records_may_alias_the_same_bounded_payload(self):
        data = bytearray(_sparse_numeric_file(ids=(7, 8), rows=(SPARSE_RAW, (0,) * 9)))
        entries = int.from_bytes(data[224:228], "little") + 8
        _put32(data, entries + 6, 496)
        rows = db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)
        self.assertEqual(rows[7], rows[8])

    def test_unsupported_payload_variants_and_primitive_widths(self):
        data = _sparse_numeric_file()
        for offset in (148, 192, 196, 204, 220, 232, 284, 288):
            changed = bytearray(data)
            _put32(changed, offset, 1)
            self.assert_code(changed, "unsupported_record_variant")
        for field, unused in ((0, 0xFFFF), (0, 32), (4, 0), (6, 16)):
            changed = bytearray(data)
            _put16(changed, 244 + field * 4, unused)
            self.assert_code(changed, "invalid_column_data")
        for offset, value, code in ((188, 215, "invalid_column_data"), (184, 1, "unsupported_table_schema"),
                                     (200, 2, "unsupported_record_variant")):
            changed = bytearray(data)
            _put32(changed, offset, value)
            self.assert_code(changed, code)

    def test_dump_identity_gate_and_catalog_opt_in_still_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "synthetic.db2"
            for offset, value in ((156, 0), (140, 8), (176, 8)):
                data = bytearray(_sparse_numeric_file())
                _put32(data, offset, value)
                path.write_bytes(data)
                with self.assertRaises(db2_oracle.OracleError) as caught:
                    db2_oracle.dump_file(path, "ConversationLine", db2_oracle.CATALOG_ID)
                self.assertEqual(caught.exception.code, "unsupported_table_schema")
            path.write_bytes(_sparse_numeric_file())
            with self.assertRaises(db2_oracle.OracleError) as caught:
                db2_oracle.dump_file(path, "ConversationLine")
            self.assertEqual(caught.exception.code, "unsupported_table_schema")
            self.assertIn("i:-2147483648", db2_oracle.dump_file(path, "ConversationLine", db2_oracle.CATALOG_ID))

    def test_all_unreviewed_shapes_stay_excluded(self):
        base = SPARSE_CONVERSATION_SCHEMA
        for field in (Field("string"), Field("unsigned", 64), Field("signed", 64, signed=True),
                      Field("float"), Field("unsigned", 32, 2), Field("signed", 16, signed=True),
                      Field("signed", 8, signed=True)):
            schema = dataclasses.replace(base, fields=(field,) + base.fields[1:])
            self.assert_code(_sparse_numeric_file(), "unsupported_record_variant", schema)
        for schema in (dataclasses.replace(base, index=0), dataclasses.replace(base, parent=0),
                       dataclasses.replace(base, parent=9)):
            self.assert_code(_sparse_numeric_file(), "unsupported_record_variant", schema)

    def test_appended_parent_sparse_rejected_even_when_no_rows(self):
        schemas = db2_oracle.load_catalog()
        for name in ("CollectableSourceEncounterSparse", "CollectableSourceQuestSparse", "CollectableSourceVendorSparse"):
            schema = schemas[name]
            data = bytearray(_zero_section_file(schema, 0))
            _put16(data, 172, 7)
            self.assert_code(data, "unsupported_record_variant", schema)

    def test_empty_section_and_complete_zero_section_metadata(self):
        data = _sparse_numeric_file(ids=(), rows=())
        self.assertEqual(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA), {})
        data = bytearray(data[:204])
        _put32(data, 188, 0)
        _put32(data, 200, 0)
        for unused in SPARSE_UNUSED:
            data.extend(unused.to_bytes(2, "little"))
            data.extend(b"\0\0")
        self.assertEqual(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA), {})
        self.assert_code(data[:204], "record_data_bounds")
        self.assert_code(data + b"\0", "record_data_bounds")
        _put32(data, 188, 216)
        self.assert_code(data, "unsupported_record_variant")

    def test_scalar_and_record_policies_use_small_synthetic_limits(self):
        data = _sparse_numeric_file(copies=((8, 7), (8, 7), (9, 0), (10, 99)))
        with mock.patch.object(db2_oracle, "VALUE_POLICY_LIMIT", 18):
            self.assertEqual(list(db2_oracle.decode(data, SPARSE_CONVERSATION_SCHEMA)), [7, 8])
        for limit in (17, 8):
            with mock.patch.object(db2_oracle, "VALUE_POLICY_LIMIT", limit):
                self.assert_code(data, "record_policy_limit")
        for offset in (136, 240):
            changed = bytearray(_sparse_numeric_file())
            _put32(changed, offset, 100001)
            self.assert_code(changed, "record_policy_limit")


class TextBudgetTests(unittest.TestCase):
    def decode_with_limit(self, data, schema, limit):
        # Production entry points retain the fixed 64 MiB policy; only these
        # small synthetic tests replace the module-private policy constant.
        with mock.patch.object(db2_oracle, "TEXT_POLICY_LIMIT", limit):
            return db2_oracle.decode(data, schema)

    def assert_error(self, data, schema, limit, code="record_policy_limit"):
        with self.assertRaises(db2_oracle.OracleError) as caught:
            self.decode_with_limit(data, schema, limit)
        self.assertEqual(caught.exception.code, code)

    def test_text_budget_fixed_default_exact_limit_and_no_mutation_on_error(self):
        self.assertEqual(db2_oracle.TEXT_POLICY_LIMIT, 64 * 1024 * 1024)
        budget = db2_oracle._TextBudget(6)
        budget.charge(2)
        budget.charge(4)
        budget.charge(0)
        with self.assertRaises(db2_oracle.OracleError) as caught:
            budget.charge(1)
        self.assertEqual(caught.exception.code, "record_policy_limit")
        self.assertEqual(budget.used, 6)

    def test_text_budget_counts_utf8_bytes_and_preserves_under_limit_golden(self):
        fixture = NAME_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(fixture, [{0: "é🙂"}, {}], ids=[11, 22]).build()
        rows = self.decode_with_limit(data, db2_oracle.NAMES_RESERVED, 6)
        self.assertEqual(
            db2_oracle.render(rows, "NamesReserved"),
            "forever-db2-dump v1 NamesReserved\n11\tt:é🙂\n22\tt:\n",
        )
        self.assertEqual(rows, db2_oracle.decode(data, db2_oracle.NAMES_RESERVED))
        self.assert_error(data, db2_oracle.NAMES_RESERVED, 5)

    def test_text_budget_charges_shared_pointers_and_suffixes_individually(self):
        fixture = NAME_FIXTURE_SCHEMAS[1]
        builder = Wdc5Builder(fixture, [{0: "abcdef"}, {}, {}], ids=[11, 22, 33])
        data = bytearray(builder.build())
        start = int.from_bytes(data[212:216], "little")
        _put32(data, start + builder.record_size, 2 * builder.record_size)
        _put32(data, start + 2 * builder.record_size, builder.record_size + 3)
        rows = self.decode_with_limit(bytes(data), db2_oracle.NAMES_RESERVED, 15)
        self.assertEqual([rows[rid][0][0].text for rid in (11, 22, 33)], ["abcdef", "abcdef", "def"])
        self.assert_error(bytes(data), db2_oracle.NAMES_RESERVED, 14)

    def test_text_budget_charges_all_aliased_string_array_elements(self):
        # Literal synthetic shape; its six relative pointers share one payload.
        fixture = Schema(
            name="SyntheticTextArray", layout_hash=0xAABBCCDD, index=-1,
            parent=-1, fields=(Field("string", 32, 6),),
        )
        builder = Wdc5Builder(fixture, [{0: ["é", "", "", "", "", ""]}], ids=[11])
        data = bytearray(builder.build())
        start = int.from_bytes(data[212:216], "little")
        for array in range(6):
            _put32(data, start + array * 4, builder.record_size - array * 4)
        rows = self.decode_with_limit(bytes(data), fixture, 12)
        self.assertEqual([value.text for value in rows[11][0]], ["é"] * 6)
        self.assert_error(bytes(data), fixture, 11)

    def test_text_budget_charges_copy_chains_and_overwrites_without_refunds(self):
        fixture = NAME_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(
            fixture, [{0: "é"}], ids=[11], copies=[(22, 11), (33, 22)],
        ).build()
        self.assertEqual(len(self.decode_with_limit(data, db2_oracle.NAMES_RESERVED, 6)), 3)
        self.assert_error(data, db2_oracle.NAMES_RESERVED, 5)
        data = Wdc5Builder(
            fixture, [{0: "é"}, {0: "🙂"}], ids=[11, 22],
            copies=[(22, 11), (22, 11)],
        ).build()
        rows = self.decode_with_limit(data, db2_oracle.NAMES_RESERVED, 10)
        self.assertEqual([rows[rid][0][0].text for rid in (11, 22)], ["é", "é"])
        # Four bytes remain, but six direct plus four cloned bytes were charged.
        self.assert_error(data, db2_oracle.NAMES_RESERVED, 9)

    def test_text_budget_skips_absent_zero_sources_and_empty_text_costs_zero(self):
        fixture = NAME_FIXTURE_SCHEMAS[1]
        data = Wdc5Builder(
            fixture, [{0: "é"}], ids=[11],
            copies=[(22, 0), (33, 99), (44, 33)],
        ).build()
        self.assertEqual(len(self.decode_with_limit(data, db2_oracle.NAMES_RESERVED, 2)), 1)
        for nonzero_pointer in (False, True):
            builder = Wdc5Builder(
                fixture, [{0: "a" if nonzero_pointer else ""}], ids=[11],
                copies=[(22, 11), (33, 22)],
            )
            data = bytearray(builder.build())
            if nonzero_pointer:
                start = int.from_bytes(data[212:216], "little")
                data[start + builder.record_size] = 0
            self.assertEqual(len(self.decode_with_limit(bytes(data), db2_oracle.NAMES_RESERVED, 0)), 3)

    def test_text_budget_preserves_string_errors_below_limit_and_checks_before_decode(self):
        fixture = NAME_FIXTURE_SCHEMAS[1]
        builder = Wdc5Builder(fixture, [{0: "a"}], ids=[11])
        data = bytearray(builder.build())
        start = int.from_bytes(data[212:216], "little") + builder.record_size
        data[start] = 0xFF
        self.assert_error(bytes(data), db2_oracle.NAMES_RESERVED, 1, "invalid_record_string")
        self.assert_error(bytes(data), db2_oracle.NAMES_RESERVED, 0)
        data[start] = ord("a")
        data[start + 1] = ord("b")
        self.assert_error(bytes(data), db2_oracle.NAMES_RESERVED, 0, "invalid_record_string")

    def test_empty_string_lookup_does_not_copy_remaining_string_block(self):
        class SliceTrackingBytes(bytes):
            def __getitem__(self, key):
                if isinstance(key, slice):
                    slices.append((key.start, key.stop))
                return super().__getitem__(key)

        slices = []
        data = SliceTrackingBytes(b"x\x00" + b"unused" * 8 + b"\x00")
        column = db2_oracle._Column(offset=0, compression=0, extra=0, a=0, b=32, c=1)
        budget = db2_oracle._TextBudget(0)
        self.assertEqual(db2_oracle._string_value(data, column, 1, 0, 1, len(data), budget), "")
        self.assertEqual(slices, [(1, 1)])
        self.assertEqual(budget.used, 0)


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


TERRAIN_SOURCE_FIXTURE_SCHEMAS = (
    INITIALIZATION_FIXTURE_SCHEMAS[0],  # independent Map transcription above
    Schema("LiquidType", 0xD1ECEEC9, -1, (
        Field("string"), Field("string", 32, 6), Field("signed", 32),
        Field("unsigned", 8), Field("unsigned", 32), Field("unsigned", 32),
        Field("float"), Field("float"), Field("float"), Field("float"),
        Field("unsigned", 16), Field("float"), Field("unsigned", 8),
        Field("unsigned", 8), Field("unsigned", 8), Field("signed", 32),
        Field("unsigned", 8, 6), Field("signed", 32, 3), Field("float", 32, 38),
        Field("unsigned", 32, 4), Field("float", 32, 4)), parent=-1),
    Schema("LiquidMaterial", 0x98E5D7AA, -1,
           (Field("signed", 32), Field("signed", 8)), parent=-1),
)


class TerrainSourceOracleTests(unittest.TestCase):
    def read(self, name, rows, **kwargs):
        fixture = next(s for s in TERRAIN_SOURCE_FIXTURE_SCHEMAS if s.name == name)
        if name == "Map":
            # A truthy NUL gives a nonzero pointer to empty text in this writer.
            # Explicit overrides, including "" (null pointer), remain intact.
            rows = [{0: "\0", 1: "\0", **row} for row in rows]
        return db2_oracle.read_terrain_sources(Wdc5Builder(fixture, rows, **kwargs).build(), name)

    def error(self, code, operation):
        with self.assertRaises(db2_oracle.OracleError) as raised:
            operation()
        self.assertEqual(raised.exception.code, code)

    def test_literal_shapes_match_pinned_facts_without_expanding_reviewed_default(self):
        catalog = db2_oracle.load_catalog()
        for fixture in TERRAIN_SOURCE_FIXTURE_SCHEMAS:
            self.assertEqual((catalog[fixture.name].layout_hash, catalog[fixture.name].index, catalog[fixture.name].parent),
                             (fixture.layout_hash, fixture.index, fixture.parent))
            self.assertEqual([(f.kind, f.bits, f.count) for f in catalog[fixture.name].fields],
                             [(f.kind, f.bits, f.count) for f in fixture.fields])
        self.assertNotIn("LiquidType", db2_oracle.SCHEMAS)
        self.assertNotIn("LiquidMaterial", db2_oracle.SCHEMAS)

    def test_map_source_only_index_preserves_duplicates_order_zero_and_full_u32(self):
        source = self.read("Map", [{21:10},{21:20},{21:30},{21:40,0:"dir\té",1:"name\\\n"},{21:0}],
                           ids=[0,7,9,7,11], max_id=1,
                           copies=[(9,7),(100,9),(101,100),(0,0),(7,7),(7,0),(0xFFFFFFFF,7),(12,11)])
        self.assertEqual([rid for rid, _ in source.direct_records], [0,7,9,7,11])
        rows = db2_oracle.materialize_terrain(source)
        self.assertEqual([(rid, fields[0]) for rid, fields in rows],
                         [(0,10),(7,20),(9,30),(7,40),(9,40),(100,30),(0,10),(7,40),(7,10),(0xFFFFFFFF,40)])
        self.assertEqual(rows[-1][1], (40,"name\\\n","dir\té"))

    def test_map_wdt_filter_is_zero_only_and_happens_after_copies(self):
        source = self.read("Map", [{21:-1},{}], ids=[0,4], copies=[(5,4),(6,0),(4,0)])
        self.assertEqual(db2_oracle.materialize_terrain(source),
                         [(0,(-1,"","")),(6,(-1,"","")),(4,(-1,"",""))])

    def test_map_selected_null_strings_fail_even_for_filtered_rows_and_generic_is_unchanged(self):
        fixture = TERRAIN_SOURCE_FIXTURE_SCHEMAS[0]
        for field in (0, 1):
            for wdt in (0, 1):
                rows = [{0: "dir", 1: "name"}, {0: "dir", 1: "name", 21: wdt, field: ""}]
                raw = Wdc5Builder(fixture, rows, ids=[7,8]).build()
                self.error("invalid_record_string", lambda: db2_oracle.read_terrain_sources(raw,"Map"))
                self.assertEqual(db2_oracle.decode(raw, fixture)[8][field][0].text, "")

    def test_map_pointed_empty_strings_accept_without_changing_unselected_nulls(self):
        source = self.read("Map", [{21: 1}], ids=[7])
        self.assertEqual(source.decoded_utf8_bytes, 0)
        self.assertEqual(source.direct_records, ((7, (1,"","")),))
        self.assertEqual(db2_oracle.materialize_terrain(source), [(7, (1,"",""))])

    def test_liquid_types_missing_sources_create_keys_and_copies_chain(self):
        source = self.read("LiquidType", [{3:255,14:255},{3:2,14:255},{3:3,14:255}], ids=[0,7,7],
                           max_id=0, copies=[(9,7),(7,0),(10,9),(11,99),(100,99),(0,0),(88,88),(0xFFFFFFFF,10)])
        self.assertEqual(db2_oracle.materialize_terrain(source),
                         {0:(255,255),7:(255,255),9:(3,255),10:(3,255),11:(0,0),88:(0,0),99:(0,0),100:(0,0),0xFFFFFFFF:(3,255)})

    def test_liquid_material_flags_and_lvf_preserve_all_bits(self):
        source = self.read("LiquidMaterial", [{0:0x80000002,1:0x80},{0:-1,1:-1}], ids=[1,2],
                           copies=[(3,1),(4,2),(5,100)])
        self.assertEqual(db2_oracle.materialize_terrain(source),
                         {1:(0x80000002,-128),2:(0xFFFFFFFF,-1),3:(0x80000002,-128),4:(0xFFFFFFFF,-1),5:(0,0),100:(0,0)})

    def test_id_repair_is_checked_but_header_extent_and_duplicates_are_not_source_policy(self):
        source = self.read("LiquidMaterial", [{},{}], ids=[0,0], min_id=50, max_id=0)
        self.assertEqual(source.direct_records, ((50,(0,0)),(51,(0,0))))
        self.error("invalid_record_ids", lambda: self.read("LiquidMaterial", [{},{}],ids=[0,0],min_id=0xFFFFFFFF))
        source = self.read("LiquidMaterial", [{}], ids=[0xFFFFFFFF],max_id=0)
        self.assertEqual(source.direct_records[0][0],0xFFFFFFFF)

    def test_full_file_identity_variant_and_tail_checks(self):
        for fixture in TERRAIN_SOURCE_FIXTURE_SCHEMAS:
            row = {0: "\0", 1: "\0"} if fixture.name == "Map" else {}
            raw = Wdc5Builder(fixture,[row],ids=[7]).build()
            for offset, number, code in ((156,0,"unsupported_table_schema"),(140,1,"unsupported_table_schema"),
                 (176,1,"unsupported_table_schema"),(184,1,"unsupported_table_schema"),
                 (172,1,"unsupported_record_variant"),(204,1,"unsupported_record_variant"),
                 (200,2,"unsupported_record_variant"),(240,100001,"record_policy_limit")):
                altered = bytearray(raw); _put32(altered,offset,number)
                self.error(code,lambda: db2_oracle.read_terrain_sources(altered,fixture.name))
            for altered in (raw[:-1],raw+b"\0"):
                self.error("record_data_bounds",lambda: db2_oracle.read_terrain_sources(altered,fixture.name))
        self.error("unsupported_table_schema",lambda: db2_oracle.read_terrain_sources(b"", "MapOther"))

    def test_bad_unselected_text_and_final_copy_never_return_partial_sources(self):
        fixture = TERRAIN_SOURCE_FIXTURE_SCHEMAS[1]
        builder = Wdc5Builder(fixture,[{},{}],ids=[7,8],copies=[(9,7)])
        raw = bytearray(builder.build())
        start = int.from_bytes(raw[212:216],"little")
        at = start+builder.record_size+builder.offsets[1]//8+5*4
        _put32(raw,at,0xFFFFFFFF)
        self.error("invalid_record_string",lambda: db2_oracle.read_terrain_sources(raw,"LiquidType"))
        raw = builder.build()
        self.error("record_data_bounds",lambda: db2_oracle.read_terrain_sources(raw[:-1],"LiquidType"))

    def test_utf8_budget_carries_unused_fields_and_charges_before_wdt_filter(self):
        raw = Wdc5Builder(TERRAIN_SOURCE_FIXTURE_SCHEMAS[0],[{0:"é",1:"a",2:"ignored"}],ids=[7],copies=[(7,7),(8,7)]).build()
        exact = db2_oracle.read_terrain_sources(raw,"Map",_text_limit=16)
        self.assertEqual(exact.decoded_utf8_bytes,10)
        self.assertEqual(db2_oracle.materialize_terrain(exact),[])
        short = db2_oracle.read_terrain_sources(raw,"Map",_text_limit=15)
        self.error("record_policy_limit",lambda: db2_oracle.materialize_terrain(short))
        self.error("record_policy_limit",lambda: db2_oracle.read_terrain_sources(raw,"Map",_text_limit=9))

    def test_small_replay_limits_count_missing_source_and_destination_separately(self):
        source = self.read("LiquidMaterial",[],copies=[(7,8)])
        self.error("record_policy_limit",lambda: db2_oracle.materialize_terrain(source,_row_limit=1))
        self.assertEqual(db2_oracle.materialize_terrain(source,_row_limit=2),{7:(0,0),8:(0,0)})
        source = self.read("Map",[{21:1}],ids=[7],copies=[(7,7)])
        self.error("record_policy_limit",lambda: db2_oracle.materialize_terrain(source,_row_limit=1))

    def test_generic_zero_source_and_self_copy_policies_are_unchanged(self):
        fixture = TERRAIN_SOURCE_FIXTURE_SCHEMAS[2]
        raw = Wdc5Builder(fixture,[{}],ids=[7],copies=[(7,7)]).build()
        self.error("unsupported_record_variant",lambda: db2_oracle.decode(raw,fixture))
        self.assertEqual(db2_oracle.materialize_terrain(db2_oracle.read_terrain_sources(raw,"LiquidMaterial")),{7:(0,0)})
        raw = Wdc5Builder(fixture,[{0:2}],ids=[0],copies=[(7,0)]).build()
        self.assertEqual(list(db2_oracle.decode(raw,fixture)),[0])
        self.assertEqual(db2_oracle.materialize_terrain(db2_oracle.read_terrain_sources(raw,"LiquidMaterial")),{0:(2,0),7:(2,0)})


class VMapSourceOracleTests(unittest.TestCase):
    def fixture(self, rows, **kwargs):
        return Wdc5Builder(TERRAIN_SOURCE_FIXTURE_SCHEMAS[0],
                          [{0: "Directory", 1: "Name", **row} for row in rows],
                          **kwargs).build()

    def test_parent_signed_bits_and_cosmetic_fallback(self):
        cases = [(0, 9, 0), (32767, 9, 32767), (0xFFFF, 9, 9),
                 (0x8000, 0xFFFF, -1), (0xFFFF, 0x8000, -32768)]
        data = self.fixture([{13: p, 14: c, 21: -1} for p, c, _ in cases],
                            ids=list(range(1, 6)))
        source = db2_oracle.read_vmap_sources(data)
        self.assertEqual([fields[1] for _, fields in source.direct_records],
                         [wanted for _, _, wanted in cases])
        self.assertTrue(all(fields[0] == -1 for _, fields in source.direct_records))

    def test_zero_wdt_and_direct_copy_snapshot_remain_before_graph(self):
        data = self.fixture([{13: -1, 14: -1, 21: 10},
                             {13: 17, 21: 0}], ids=[7, 7],
                            copies=[(8, 7), (9, 8), (7, 7), (10, 99)])
        source = db2_oracle.read_vmap_sources(data)
        materialized = db2_oracle.materialize_terrain(source)
        self.assertEqual([rid for rid, _ in materialized], [7, 7, 8, 7])
        self.assertEqual([fields[0] for _, fields in materialized], [10, 0, 0, 0])
        self.assertEqual([fields[1] for _, fields in materialized], [-1, 17, 17, 17])
        self.assertEqual(len(source.direct_records), 2)

    def test_selected_null_and_extra_bytes_reject(self):
        for field in (0, 1):
            with self.assertRaises(db2_oracle.OracleError):
                db2_oracle.read_vmap_sources(self.fixture([{field: "", 21: 0}], ids=[7]))
        good = self.fixture([{13: -1, 14: -1}], ids=[7])
        for data in (good[:-1], good + b"\0"):
            with self.assertRaises(db2_oracle.OracleError):
                db2_oracle.read_vmap_sources(data)

    def test_empty_and_text_copy_budget(self):
        empty = db2_oracle.read_vmap_sources(self.fixture([]))
        self.assertEqual(db2_oracle.materialize_terrain(empty), [])
        data = self.fixture([{0: "é", 1: "x", 2: "unused"}], ids=[7], copies=[(8, 7)])
        source = db2_oracle.read_vmap_sources(data, _text_limit=11)
        with self.assertRaises(db2_oracle.OracleError):
            db2_oracle.materialize_terrain(source)
        source = db2_oracle.read_vmap_sources(data, _text_limit=12)
        self.assertEqual(len(db2_oracle.materialize_terrain(source)), 2)


if __name__ == "__main__":
    unittest.main()
