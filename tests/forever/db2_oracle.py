"""Independent offline WDC5 DB2 oracle for the reviewed character tables.

This is a from-scratch, read-only decoder used to cross-check the Rust
``crates/forever-db2/examples/db2_dump.rs`` diagnostic byte-for-byte. It never
imports or links the Rust crate, never touches CASC, the network or a client
directory, and only consumes a local file named on the command line.

Wire layout and semantics follow the pinned C++ reference (TrinityCore
``02245dcd245e7433e524577656177723d3e4992e``):

* ``src/common/DataStores/DB2FileLoader.h`` -- ``DB2Header`` (204 bytes) and
  ``DB2SectionHeader`` (40 bytes) field order.
* ``src/common/DataStores/DB2FileLoader.cpp`` -- 24-byte ``DB2ColumnMeta``
  (``BitOffset``, ``BitSize``, ``AdditionalDataSize``, ``CompressionType`` and
  the immediate/commonData/pallet union); ``RecordGetVarInt`` modes 0..5,
  ``RecordGetString`` (``src/...:800-806``), ``RecordGetId`` (:747-760) and the
  non-inline id repair ``id <= MinId -> MinId + i`` (:1958-1967);
  ``AutoProduceRecordCopies`` (:613-634); ``GetFieldOffset`` (:889-905).
* ``src/server/game/DataStores/DB2Metadata.h`` -- ``ChrClassesMeta`` (:3455,
  43 fields, index 29, layout ``0xAFC9B0C2``), ``ChrRacesMeta`` (:4182,
  51 fields, index -1, layout ``0x4F44C796``) and the four ``ChrClass*``
  tables: ``ChrClassTitleMeta`` (:3369, 3 fields, layout ``0xC086F95D``),
  ``ChrClassUIChrModelInfoMeta`` (:3390, 5 fields, layout ``0x26E5E322``),
  ``ChrClassUIDisplayMeta`` (:3413, 3 fields, layout ``0x50F5AD66``) and
  ``ChrClassVillainMeta`` (:3434, 3 fields, layout ``0x46041630``); each
  ``ChrClass*`` declares ``IndexField = -1``. The reviewed Forever
  ``ChrCustomization*`` tables add ``ChrCustomizationElementMeta`` (3512765,
  13 fields, layout ``0x6483C37E``), ``ChrCustomizationDisplayInfoMeta``
  (3593032, 5 fields, layout ``0xB24CE54F``) and ``ChrCustomizationReqMeta``
  (3450453, 9 fields, layout ``0xCA154412``); ``ChrCustomizationReq`` carries a
  signed 32-bit array of two.
* ``src/common/DataStores/DB2Meta.h`` (:32-49) has no table-hash member, so the
  loader's identity gate is LayoutHash (:1763) plus the field-count check
  (:1771); ``_header.TableHash`` is never validated against the metadata.

Canonical dump format, identical to ``db2_dump.rs``::

    forever-db2-dump v1 <table>
    <id>\t<field0>\t<field1>...

Every field prints its array values comma-joined; a value is ``u:<dec>``
(unsigned), ``i:<dec>`` (signed), ``f:0x<8 lowercase hex>`` (raw float bits) or
``t:<text>`` where ``\\`` -> ``\\\\``, tab -> ``\\t``, CR -> ``\\r`` and
LF -> ``\\n``. Output is UTF-8 with LF, terminated by a single trailing newline.
"""
from __future__ import annotations

import dataclasses
import sys

HEADER_SIZE = 204
SECTION_SIZE = 40
MAX_SECTIONS = 1024
MAX_BYTES = 16 * 1024 * 1024
RECORD_POLICY_LIMIT = 100_000
COPY_POLICY_LIMIT = 100_000
VALUE_POLICY_LIMIT = 1_000_000
SPARSE_FLAG = 0x1

# Stable error code -> process exit status. Exit 2 is reserved for argument
# errors, 3 for an unsupported signature/version, 6 for the inspection limit.
_CODES = {
    "invalid_invocation": 2,
    "input_io": 4,
    "metadata_truncated": 4,
    "unsupported_signature": 3,
    "unsupported_version": 3,
    "invalid_parent_lookup_count": 4,
    "section_policy_limit": 6,
    "invalid_record_metadata": 4,
    "unsupported_table_schema": 4,
    "unsupported_record_variant": 4,
    "record_data_bounds": 4,
    "invalid_column_data": 4,
    "invalid_record_ids": 4,
    "invalid_record_string": 4,
    "record_policy_limit": 4,
}


class OracleError(Exception):
    """Decode failure carrying a stable code and exit status."""

    def __init__(self, code):
        super().__init__(code)
        self.code = code
        self.exit_code = _CODES[code]


@dataclasses.dataclass(frozen=True)
class Field:
    """One logical field: ``kind`` in string/unsigned/signed/float."""

    kind: str
    bits: int = 32
    count: int = 1
    signed: bool = False

    def __post_init__(self):
        if self.count < 1:
            raise ValueError("field count must be positive")

    @property
    def width_bits(self):
        # Strings store a 32-bit offset (DB2FileLoader.cpp:800-806), so their
        # decode width is 32 like the Rust ``Kind::String`` transcription.
        return self.bits


@dataclasses.dataclass(frozen=True)
class Schema:
    """Reviewed schema identity plus the transcribed field list.

    ``fields`` is the decoded column list (one column per ``FileFieldCount``);
    the header's ``field_count`` equals ``len(fields)``. ``parent`` is the
    reviewed ``ParentIndexField``, an index into the TOTAL field array (columns
    plus any appended parent) and ``-1`` when the table has no parent. When
    ``parent >= len(fields)`` the parent is the "extra field at the end"
    variant, filled from the separate parent-lookup block.
    """

    name: str
    layout_hash: int
    index: int
    fields: tuple
    parent: int = -1


def _signed_field(bits, count=1):
    return Field("signed", bits, count, signed=True)


# Transcribed from DB2Metadata.h:3455 (ChrClassesMeta Fields[43]).
CHR_CLASSES = Schema(
    name="ChrClasses",
    layout_hash=0xAFC9B0C2,
    index=29,
    fields=(
        Field("string"),  # 0
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),  # 9
        Field("unsigned", 32),  # 10
        Field("unsigned", 32),
        Field("unsigned", 32),
        Field("unsigned", 32),  # 13
        _signed_field(32),  # 14
        _signed_field(32),  # 15
        Field("unsigned", 32),  # 16
        Field("unsigned", 32),  # 17
        _signed_field(32),  # 18
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),  # 26
        Field("unsigned", 16),  # 27
        Field("unsigned", 16),  # 28
        Field("unsigned", 8),  # 29 (index)
        Field("unsigned", 8),  # 30
        _signed_field(8),  # 31
        _signed_field(8),  # 32
        Field("unsigned", 8),  # 33
        Field("unsigned", 8),  # 34
        Field("unsigned", 8),  # 35
        Field("unsigned", 8),  # 36
        Field("unsigned", 8),  # 37
        Field("unsigned", 8),  # 38
        Field("unsigned", 8),  # 39
        Field("unsigned", 8),  # 40
        Field("unsigned", 8),  # 41
        Field("unsigned", 8),  # 42
    ),
)

# Transcribed from DB2Metadata.h:4182 (ChrRacesMeta Fields[51]).
CHR_RACES = Schema(
    name="ChrRaces",
    layout_hash=0x4F44C796,
    index=-1,
    fields=(
        Field("string"),  # 0
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),
        Field("string"),  # 14
        _signed_field(32),  # 15
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),  # 22
        _signed_field(32, 3),  # 23
        _signed_field(32, 3),  # 24
        _signed_field(32),  # 25
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),
        _signed_field(32),  # 29
        Field("float", 32, 3),  # 30
        Field("float", 32, 1),  # 31
        Field("float", 32, 3),  # 32
        Field("float", 32, 3),  # 33
        _signed_field(8),  # 34
        Field("unsigned", 8),  # 35
        _signed_field(8),  # 36
        _signed_field(8),  # 37
        _signed_field(8),  # 38
        _signed_field(8),  # 39
        _signed_field(8),  # 40
        _signed_field(8),  # 41
        _signed_field(8),  # 42
        _signed_field(8),  # 43
        _signed_field(8),  # 44
        _signed_field(8),  # 45
        _signed_field(8),  # 46
        _signed_field(8),  # 47
        _signed_field(8),  # 48
        _signed_field(8),  # 49
        _signed_field(8),  # 50
    ),
)

# Transcribed from DB2Metadata.h:3369 (ChrClassTitleMeta Fields[3],
# layout 0xC086F95D, IndexField -1).
CHR_CLASS_TITLE = Schema(
    name="ChrClassTitle",
    layout_hash=0xC086F95D,
    index=-1,
    fields=(
        Field("string"),  # 0
        Field("string"),  # 1
        _signed_field(8),  # 2
    ),
)

# Transcribed from DB2Metadata.h:3413 (ChrClassUIDisplayMeta Fields[3],
# layout 0x50F5AD66, IndexField -1).
CHR_CLASS_UI_DISPLAY = Schema(
    name="ChrClassUIDisplay",
    layout_hash=0x50F5AD66,
    index=-1,
    fields=(
        _signed_field(8),  # 0
        Field("unsigned", 32),  # 1
        Field("unsigned", 32),  # 2
    ),
)

# Transcribed from DB2Metadata.h:3390 (ChrClassUIChrModelInfoMeta Fields[5],
# layout 0x26E5E322, IndexField -1).
CHR_CLASS_UI_CHR_MODEL_INFO = Schema(
    name="ChrClassUIChrModelInfo",
    layout_hash=0x26E5E322,
    index=-1,
    fields=(
        Field("float", 32, 3),  # 0
        _signed_field(32),  # 1
        _signed_field(8),  # 2
        Field("float", 32, 1),  # 3
        _signed_field(32),  # 4
    ),
)

# Transcribed from DB2Metadata.h:3434 (ChrClassVillainMeta Fields[3],
# layout 0x46041630, IndexField -1).
CHR_CLASS_VILLAIN = Schema(
    name="ChrClassVillain",
    layout_hash=0x46041630,
    index=-1,
    fields=(
        Field("string"),  # 0
        _signed_field(8),  # 1
        _signed_field(8),  # 2
    ),
)

# Transcribed from DB2Metadata.h::ChrClassesXPowerTypesMeta (FileDataId
# 1121420, IndexField -1, ParentIndexField 1, FieldCount 2, FileFieldCount 1,
# layout 0x70DA1F8C). `fields` is the single decoded column; `parent = 1` sits
# at the total-array index just past it (the "extra field at the end" variant),
# so the header carries field_count = 1 and total_field_count = 1.
CHR_CLASSES_X_POWER_TYPES = Schema(
    name="ChrClassesXPowerTypes",
    layout_hash=0x70DA1F8C,
    index=-1,
    parent=1,
    fields=(_signed_field(8),),
)

# Transcribed from DB2Metadata.h::ChrCustomizationCategoryMeta (FileDataId
# 3526439, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0x1605B68A): one string, six signed 32-bit fields and a float.
CHR_CUSTOMIZATION_CATEGORY = Schema(
    name="ChrCustomizationCategory",
    layout_hash=0x1605B68A,
    index=-1,
    fields=(
        Field("string"),  # 0
        _signed_field(32),  # 1
        _signed_field(32),  # 2
        _signed_field(32),  # 3
        _signed_field(32),  # 4
        _signed_field(32),  # 5
        _signed_field(32),  # 6
        Field("float", 32, 1),  # 7
    ),
)

# Transcribed from DB2Metadata.h::ChrCustItemGeoModifyMeta (FileDataId 3952714,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x2986E1F3): one signed byte plus two signed 32-bit fields.
CHR_CUST_ITEM_GEO_MODIFY = Schema(
    name="ChrCustItemGeoModify",
    layout_hash=0x2986E1F3,
    index=-1,
    fields=(
        _signed_field(8),  # 0
        _signed_field(32),  # 1
        _signed_field(32),  # 2
    ),
)

# Transcribed from DB2Metadata.h::ChrCustomizationGeosetMeta (FileDataId
# 3456171, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0xBF55F2FB): one signed byte plus two signed 32-bit fields.
CHR_CUSTOMIZATION_GEOSET = Schema(
    name="ChrCustomizationGeoset",
    layout_hash=0xBF55F2FB,
    index=-1,
    fields=(
        _signed_field(8),  # 0
        _signed_field(32),  # 1
        _signed_field(32),  # 2
    ),
)

# Transcribed from DB2Metadata.h::ChrCustomizationMaterialMeta (FileDataId
# 3459652, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0xBE9767E9): two signed 32-bit fields.
CHR_CUSTOMIZATION_MATERIAL = Schema(
    name="ChrCustomizationMaterial",
    layout_hash=0xBE9767E9,
    index=-1,
    fields=(
        _signed_field(32),  # 0
        _signed_field(32),  # 1
    ),
)

# Transcribed from DB2Metadata.h::ChrCustomizationBoneSetMeta (FileDataId
# 3460181, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0xCC2CFF6E): two signed 32-bit fields.
CHR_CUSTOMIZATION_BONE_SET = Schema(
    name="ChrCustomizationBoneSet",
    layout_hash=0xCC2CFF6E,
    index=-1,
    fields=(
        _signed_field(32),  # 0
        _signed_field(32),  # 1
    ),
)

# Transcribed from DB2Metadata.h::ChrCustomizationElementMeta (FileDataId
# 3512765, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0x6483C37E): thirteen signed 32-bit fields.
CHR_CUSTOMIZATION_ELEMENT = Schema(
    name="ChrCustomizationElement",
    layout_hash=0x6483C37E,
    index=-1,
    fields=tuple(_signed_field(32) for _ in range(13)),
)

# Transcribed from DB2Metadata.h::ChrCustomizationDisplayInfoMeta (FileDataId
# 3593032, IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0xB24CE54F): two signed 32-bit fields then three floats.
CHR_CUSTOMIZATION_DISPLAY_INFO = Schema(
    name="ChrCustomizationDisplayInfo",
    layout_hash=0xB24CE54F,
    index=-1,
    fields=(
        _signed_field(32),  # 0
        _signed_field(32),  # 1
        Field("float", 32, 1),  # 2
        Field("float", 32, 1),  # 3
        Field("float", 32, 1),  # 4
    ),
)

# Transcribed from DB2Metadata.h::ChrCustomizationReqMeta (FileDataId 3450453,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xCA154412): one string, seven signed 32-bit fields and one signed 32-bit
# array of two.
CHR_CUSTOMIZATION_REQ = Schema(
    name="ChrCustomizationReq",
    layout_hash=0xCA154412,
    index=-1,
    fields=(
        Field("string"),  # 0
        _signed_field(32),  # 1
        _signed_field(32),  # 2
        _signed_field(32),  # 3
        _signed_field(32),  # 4
        _signed_field(32),  # 5
        _signed_field(32),  # 6
        _signed_field(32),  # 7
        _signed_field(32, 2),  # 8 array of two
    ),
)

# Transcribed from DB2Metadata.h::CurveMeta (FileDataId 892585, IndexField 0,
# ParentIndexField -1, FieldCount == FileFieldCount, layout 0xD362535A): the
# record id is read from the inline field 0 (Unsigned(32,1)); field 1 is an
# UNSIGNED FT_BYTE and field 2 a signed 32-bit field.
CURVE = Schema(
    name="Curve",
    layout_hash=0xD362535A,
    index=0,
    fields=(
        Field("unsigned", 32),  # 0 (inline index)
        Field("unsigned", 8),  # 1
        _signed_field(32),  # 2
    ),
)

# Transcribed from DB2Metadata.h::ContentRestrictionRuleSetMeta (FileDataId
# 4281307, IndexField 0, ParentIndexField -1, FieldCount == FileFieldCount,
# layout 0x5774D75A): three signed 32-bit fields; the id is read from the inline
# field 0.
CONTENT_RESTRICTION_RULE_SET = Schema(
    name="ContentRestrictionRuleSet",
    layout_hash=0x5774D75A,
    index=0,
    fields=(
        _signed_field(32),  # 0 (inline index)
        _signed_field(32),  # 1
        _signed_field(32),  # 2
    ),
)

# Transcribed from DB2Metadata.h::DataTagInfoMeta (FileDataId 6652252,
# IndexField 1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xDB46F772): one string plus two signed 32-bit fields; the id is read from the
# inline field 1.
DATA_TAG_INFO = Schema(
    name="DataTagInfo",
    layout_hash=0xDB46F772,
    index=1,
    fields=(
        Field("string"),  # 0
        Field("unsigned", 32),  # 1 (inline index)
        _signed_field(32),  # 2
    ),
)

SCHEMAS = {
    "ChrClasses": CHR_CLASSES,
    "ChrRaces": CHR_RACES,
    "ChrClassTitle": CHR_CLASS_TITLE,
    "ChrClassUIDisplay": CHR_CLASS_UI_DISPLAY,
    "ChrClassUIChrModelInfo": CHR_CLASS_UI_CHR_MODEL_INFO,
    "ChrClassVillain": CHR_CLASS_VILLAIN,
    "ChrClassesXPowerTypes": CHR_CLASSES_X_POWER_TYPES,
    "ChrCustomizationCategory": CHR_CUSTOMIZATION_CATEGORY,
    "ChrCustItemGeoModify": CHR_CUST_ITEM_GEO_MODIFY,
    "ChrCustomizationGeoset": CHR_CUSTOMIZATION_GEOSET,
    "ChrCustomizationMaterial": CHR_CUSTOMIZATION_MATERIAL,
    "ChrCustomizationBoneSet": CHR_CUSTOMIZATION_BONE_SET,
    "ChrCustomizationElement": CHR_CUSTOMIZATION_ELEMENT,
    "ChrCustomizationDisplayInfo": CHR_CUSTOMIZATION_DISPLAY_INFO,
    "ChrCustomizationReq": CHR_CUSTOMIZATION_REQ,
    "Curve": CURVE,
    "ContentRestrictionRuleSet": CONTENT_RESTRICTION_RULE_SET,
    "DataTagInfo": DATA_TAG_INFO,
}


@dataclasses.dataclass
class _Header:
    record_count: int
    field_count: int
    record_size: int
    string_table_size: int
    table_hash: int
    layout_hash: int
    min_id: int
    max_id: int
    flags: int
    index_field: int
    total_field_count: int
    packed_data_offset: int
    parent_lookup_count: int
    column_meta_size: int
    common_data_size: int
    pallet_data_size: int
    section_count: int


@dataclasses.dataclass
class _Section:
    tact: bool
    file_offset: int
    record_count: int
    string_table_size: int
    catalog_data_offset: int
    id_table_size: int
    parent_lookup_data_size: int
    catalog_data_count: int
    copy_table_count: int


def _slice(data, at, size):
    end = at + size
    if end > len(data):
        raise OracleError("record_data_bounds")
    return data[at:end]


def _u16(data, at):
    return int.from_bytes(_slice(data, at, 2), "little")


def _u32(data, at):
    return int.from_bytes(_slice(data, at, 4), "little")


def _i16(data, at):
    return int.from_bytes(_slice(data, at, 2), "little", signed=True)


def parse(data):
    """Parse header and section headers; raises metadata errors only."""
    if len(data) > MAX_BYTES:
        raise OracleError("record_policy_limit")
    if len(data) < HEADER_SIZE:
        raise OracleError("metadata_truncated")
    if data[:4] != b"WDC5":
        raise OracleError("unsupported_signature")
    if _u32(data, 4) != 5:
        raise OracleError("unsupported_version")
    if _u32(data, 184) > 1:
        raise OracleError("invalid_parent_lookup_count")
    section_count = _u32(data, 200)
    if section_count > MAX_SECTIONS:
        raise OracleError("section_policy_limit")
    needed = HEADER_SIZE + SECTION_SIZE * section_count
    if needed > len(data):
        raise OracleError("metadata_truncated")
    header = _Header(
        record_count=_u32(data, 136),
        field_count=_u32(data, 140),
        record_size=_u32(data, 144),
        string_table_size=_u32(data, 148),
        table_hash=_u32(data, 152),
        layout_hash=_u32(data, 156),
        min_id=_u32(data, 160),
        max_id=_u32(data, 164),
        flags=_u16(data, 172),
        index_field=_i16(data, 174),
        total_field_count=_u32(data, 176),
        packed_data_offset=_u32(data, 180),
        parent_lookup_count=_u32(data, 184),
        column_meta_size=_u32(data, 188),
        common_data_size=_u32(data, 192),
        pallet_data_size=_u32(data, 196),
        section_count=section_count,
    )
    sections = []
    for i in range(section_count):
        base = HEADER_SIZE + SECTION_SIZE * i
        sections.append(
            _Section(
                tact=any(data[base : base + 8]),
                file_offset=_u32(data, base + 8),
                record_count=_u32(data, base + 12),
                string_table_size=_u32(data, base + 16),
                catalog_data_offset=_u32(data, base + 20),
                id_table_size=_u32(data, base + 24),
                parent_lookup_data_size=_u32(data, base + 28),
                catalog_data_count=_u32(data, base + 32),
                copy_table_count=_u32(data, base + 36),
            )
        )
    return header, sections


@dataclasses.dataclass
class _Column:
    offset: int
    compression: int
    extra: int
    a: int
    b: int
    c: int
    pallet: list = dataclasses.field(default_factory=list)
    common: dict = dataclasses.field(default_factory=dict)

    def bits(self, record, packed_data_offset, record_id, array, width, count):
        """Mode 0..5 value extraction (DB2FileLoader.cpp:808-905)."""
        if array >= count or width == 0 or width > 32:
            raise OracleError("invalid_column_data")
        mask = 0xFFFFFFFF if width == 32 else (1 << width) - 1
        mode = self.compression
        if mode == 0:
            # Direct-value storage requires whole-byte widths and offsets.
            if self.offset % 8 or width % 8:
                raise OracleError("invalid_column_data")
            byte_width = width // 8
            at = self.offset // 8 + array * byte_width
            value = int.from_bytes(_slice(record, at, byte_width), "little")
        elif mode == 2:
            if count != 1:
                raise OracleError("invalid_column_data")
            value = self.common.get(record_id, self.a)
        else:
            if self.b == 0 or self.b > 32 or (mode != 4 and count != 1):
                raise OracleError("invalid_column_data")
            bit = packed_data_offset * 8 + self.a
            value = 0
            for index in range(self.b):
                pos = bit + index
                byte = _slice(record, pos // 8, 1)[0]
                value |= ((byte >> (pos % 8)) & 1) << index
            if mode == 1:
                pass
            elif mode == 5:
                sign = 1 << (self.b - 1)
                value = (value ^ sign) - sign
            elif mode == 3:
                if value >= len(self.pallet):
                    raise OracleError("invalid_column_data")
                value = self.pallet[value]
            elif mode == 4:
                if self.c != count:
                    raise OracleError("invalid_column_data")
                index = value * count + array
                if index >= len(self.pallet):
                    raise OracleError("invalid_column_data")
                value = self.pallet[index]
            else:
                raise OracleError("invalid_column_data")
        return value & mask


def _columns(data, header):
    """Read the field offset table, every column entry and the pallet/common
    blocks; return the columns and the first record byte offset."""
    fields = header.field_count
    if header.column_meta_size != fields * 24:
        raise OracleError("invalid_column_data")
    # 204 header + one 40-byte section + one 4-byte field-offset word per field.
    start = HEADER_SIZE + SECTION_SIZE + fields * 4
    at = start + fields * 24
    if at > len(data):
        raise OracleError("record_data_bounds")
    columns = []
    for index in range(fields):
        base = start + index * 24
        entry = _slice(data, base, 24)
        compression = int.from_bytes(entry[8:12], "little")
        if compression > 5:
            raise OracleError("invalid_column_data")
        columns.append(
            _Column(
                offset=int.from_bytes(entry[0:2], "little"),
                compression=compression,
                extra=int.from_bytes(entry[4:8], "little"),
                a=int.from_bytes(entry[12:16], "little"),
                b=int.from_bytes(entry[16:20], "little"),
                c=int.from_bytes(entry[20:24], "little"),
            )
        )
    pallet_start = at
    # Source order: all scalar pallets (3), all array pallets (4), then common (2).
    for mode in (3, 4, 2):
        for column in columns:
            if column.compression != mode:
                continue
            stride = 8 if mode == 2 else 4
            if column.extra % stride:
                raise OracleError("invalid_column_data")
            end = at + column.extra
            if end > len(data):
                raise OracleError("record_data_bounds")
            pos = at
            while pos < end:
                key = _u32(data, pos)
                if mode == 2:
                    column.common[key] = _u32(data, pos + 4)
                else:
                    column.pallet.append(key)
                pos += stride
            at = end
        if mode == 4:
            if at - pallet_start != header.pallet_data_size:
                raise OracleError("invalid_column_data")
    consumed = at - pallet_start
    if consumed != header.pallet_data_size + header.common_data_size:
        raise OracleError("invalid_column_data")
    for column in columns:
        if column.compression in (0, 1, 5) and column.extra != 0:
            raise OracleError("invalid_column_data")
    return columns, at


def _string_value(data, column, bits, record_start, strings_start, strings_end):
    if column.compression != 0:
        raise OracleError("invalid_column_data")
    if bits == 0:
        return ""
    at = record_start + column.offset // 8 + bits
    if not strings_start <= at < strings_end:
        raise OracleError("invalid_record_string")
    tail = data[at:strings_end]
    nul = tail.find(b"\x00")
    if nul < 0:
        raise OracleError("invalid_record_string")
    try:
        return tail[:nul].decode("utf-8")
    except UnicodeDecodeError:
        raise OracleError("invalid_record_string")


def _signed_value(bits, width):
    mask = 0xFFFFFFFF if width == 32 else (1 << width) - 1
    value = bits & mask
    if width == 8:
        return value - 256 if value >= 128 else value
    if width == 16:
        return value - 65536 if value >= 32768 else value
    return value - 2**32 if value >= 2**31 else value


@dataclasses.dataclass
class _Value:
    kind: str
    number: int = 0
    text: str = ""


def _decode_parsed(data, header, sections, schema):
    if header.flags & SPARSE_FLAG:
        raise OracleError("unsupported_record_variant")
    # DB2FileLoader.cpp:1767 rejects ParentLookupCount > 1; :1773 then requires
    # the reviewed metadata to agree on whether the table has a parent field.
    if header.parent_lookup_count > 1:
        raise OracleError("unsupported_record_variant")
    if (header.parent_lookup_count == 1) != (schema.parent >= 0):
        raise OracleError("unsupported_table_schema")
    if len(sections) == 0:
        # DB2FileLoader.cpp:1778-1779 admits a zero-section header and
        # DB2FileLoader.cpp:1877 then assigns an empty record map: no sections
        # means no records. A header that still declares records has no offsets
        # to read, so it is rejected rather than silently dropping them.
        if header.record_count != 0:
            raise OracleError("unsupported_record_variant")
        return {}
    if len(sections) > 1:
        raise OracleError("unsupported_record_variant")
    section = sections[0]
    if section.tact or section.catalog_data_count != 0:
        raise OracleError("unsupported_record_variant")
    value_count = header.record_count * len(schema.fields)
    if (
        header.record_count > RECORD_POLICY_LIMIT
        or section.copy_table_count > COPY_POLICY_LIMIT
        or value_count > VALUE_POLICY_LIMIT
    ):
        raise OracleError("record_policy_limit")
    if (
        section.record_count != header.record_count
        or section.string_table_size != header.string_table_size
        or header.record_size == 0
        or header.min_id > header.max_id
    ):
        raise OracleError("record_data_bounds")
    columns, metadata_end = _columns(data, header)
    if section.file_offset != metadata_end:
        raise OracleError("record_data_bounds")
    record_size = header.record_size
    records_start = section.file_offset
    strings_start = records_start + header.record_count * record_size
    strings_end = strings_start + section.string_table_size
    id_bytes = header.record_count * 4 if schema.index < 0 else 0
    if section.id_table_size != id_bytes:
        raise OracleError("invalid_record_ids")
    copy_start = strings_end + id_bytes
    end = copy_start + section.copy_table_count * 8
    # Parent-lookup block: 12-byte header (NumEntries, MinId, MaxId) then
    # NumEntries 8-byte {ParentId, RecordIndex} entries, after the copy table.
    parent_start = 0
    parent_entries = 0
    if header.parent_lookup_count == 1:
        declared = _u32(data, end)
        if declared > RECORD_POLICY_LIMIT:
            raise OracleError("record_policy_limit")
        parent_bytes = 12 + declared * 8
        if section.parent_lookup_data_size != parent_bytes:
            raise OracleError("record_data_bounds")
        if end + parent_bytes != len(data):
            raise OracleError("record_data_bounds")
        parent_start = end
        parent_entries = declared
    elif section.parent_lookup_data_size != 0 or end != len(data):
        raise OracleError("record_data_bounds")
    if schema.index >= 0 and columns[schema.index].compression not in (0, 1, 5):
        raise OracleError("invalid_column_data")

    rows = {}
    # Record ordinal -> row id, so the parent block can address a row by its
    # ordinal (DB2FileLoader.cpp:1877 fills the appended field from the block).
    ordinal_ids = []
    for record in range(header.record_count):
        record_start = records_start + record * record_size
        raw = _slice(data, record_start, record_size)
        if schema.index >= 0:
            field = schema.index
            record_id = columns[field].bits(
                raw,
                header.packed_data_offset,
                0,
                0,
                schema.fields[field].bits,
                1,
            )
        else:
            raw_id = _u32(data, strings_end + record * 4)
            # DB2FileLoader.cpp:1958-1967 repair for a single section.
            record_id = (
                header.min_id + record if raw_id <= header.min_id else raw_id
            )
        if record_id > header.max_id or record_id in rows:
            raise OracleError("invalid_record_ids")
        fields = []
        for field, kind in enumerate(schema.fields):
            values = []
            for array in range(kind.count):
                bits = columns[field].bits(
                    raw,
                    header.packed_data_offset,
                    record_id,
                    array,
                    kind.width_bits,
                    kind.count,
                )
                if kind.kind == "string":
                    values.append(
                        _Value(
                            "text",
                            text=_string_value(
                                data,
                                columns[field],
                                bits,
                                record_start,
                                strings_start,
                                strings_end,
                            ),
                        )
                    )
                elif kind.kind == "float":
                    values.append(_Value("float", number=bits))
                elif kind.kind == "signed":
                    values.append(
                        _Value("signed", number=_signed_value(bits, kind.bits))
                    )
                else:
                    values.append(_Value("unsigned", number=bits))
            fields.append(values)
        # Extra field at the end: grow the row to `parent` and default the
        # appended slot to a signed zero; filled from the parent block below.
        if schema.parent >= 0:
            while len(fields) <= schema.parent:
                fields.append([])
            fields[schema.parent] = [_Value("signed", number=0)]
        ordinal_ids.append(record_id)
        rows[record_id] = fields

    # Apply the parent lookup before the copy table so a copied row carries its
    # source's parent value. A later duplicate for the same RecordIndex wins.
    if header.parent_lookup_count == 1:
        entries_at = parent_start + 12
        for entry in range(parent_entries):
            at = entries_at + entry * 8
            parent_id = _u32(data, at)
            record_index = _u32(data, at + 4)
            if record_index >= len(ordinal_ids):
                raise OracleError("invalid_record_ids")
            rid = ordinal_ids[record_index]
            if rid not in rows:
                raise OracleError("invalid_record_ids")
            rows[rid][schema.parent][0] = _Value("signed", number=parent_id)

    # AutoProduceRecordCopies (DB2FileLoader.cpp:613-634).
    for index in range(section.copy_table_count):
        entry = copy_start + index * 8
        new = _u32(data, entry)
        source = _u32(data, entry + 4)
        if (
            source == 0
            or source >= header.record_count
            or new >= header.record_count
            or source not in rows
        ):
            continue
        fields = [[dataclasses.replace(value) for value in group] for group in rows[source]]
        if schema.index >= 0:
            fields[schema.index][0] = _Value("unsigned", number=new)
        rows[new] = fields
    return dict(sorted(rows.items()))


def decode(data, schema):
    """Decode one regular, unencrypted, single-section WDC5 file."""
    header, sections = parse(data)
    return _decode_parsed(data, header, sections, schema)


def _escape_text(text):
    return (
        text.replace("\\", "\\\\")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
        .replace("\n", "\\n")
    )


def _render_value(value):
    if value.kind == "unsigned":
        return "u:%d" % value.number
    if value.kind == "signed":
        return "i:%d" % value.number
    if value.kind == "float":
        return "f:0x%08x" % value.number
    return "t:" + _escape_text(value.text)


def render(rows, table_name):
    """Canonical deterministic dump text; always ends with a single newline."""
    lines = ["forever-db2-dump v1 %s" % table_name]
    for record_id in rows:
        tokens = [str(record_id)]
        for group in rows[record_id]:
            tokens.append(",".join(_render_value(value) for value in group))
        lines.append("\t".join(tokens))
    return "\n".join(lines) + "\n"


def dump_file(path, table_name):
    """Read a local file and return its canonical dump for a reviewed table."""
    schema = SCHEMAS.get(table_name)
    if schema is None:
        raise OracleError("unsupported_table_schema")
    try:
        with open(path, "rb") as handle:
            data = handle.read(MAX_BYTES + 1)
    except OSError:
        raise OracleError("input_io")
    header, sections = parse(data)
    # DB2FileLoader.cpp:1763 validates LayoutHash; :1771 gates the field counts:
    #   TotalFieldCount + (ParentIndexField >= TotalFieldCount ? 1 : 0)
    #   == Meta->FieldCount
    # with Meta->FieldCount == len(fields) + 1 for the reviewed
    # extra-field-at-the-end parent variant (so total_field_count == fields),
    # and total_field_count == field_count otherwise.
    field_count_ok = header.field_count == len(schema.fields)
    if schema.parent >= 0 and schema.parent >= len(schema.fields):
        total_field_count_ok = header.total_field_count + 1 == len(schema.fields) + 1
    else:
        total_field_count_ok = header.total_field_count == header.field_count
    if (
        header.layout_hash != schema.layout_hash
        or not field_count_ok
        or not total_field_count_ok
    ):
        raise OracleError("unsupported_table_schema")
    rows = _decode_parsed(data, header, sections, schema)
    return render(rows, schema.name)


def main(argv=None):
    args = sys.argv[1:] if argv is None else argv
    path = None
    table = None
    index = 0
    while index < len(args):
        if args[index] == "--file" and path is None and index + 1 < len(args):
            path = args[index + 1]
            index += 2
        elif args[index] == "--table" and table is None and index + 1 < len(args):
            table = args[index + 1]
            index += 2
        else:
            sys.stderr.write("invalid_invocation\n")
            return 2
    if path is None or table is None:
        sys.stderr.write("invalid_invocation\n")
        return 2
    try:
        dump = dump_file(path, table)
    except OracleError as error:
        sys.stderr.write(error.code + "\n")
        return error.exit_code
    sys.stdout.write(dump)
    return 0


if __name__ == "__main__":
    sys.exit(main())
