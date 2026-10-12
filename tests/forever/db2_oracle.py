"""Independent offline WDC5 DB2 oracle for reviewed and explicit catalogs.

Modified 2026-10-11: corrected copy-ID extent, scalar budgets and signed
parents; unsafe reference copy variants are explicitly rejected. A separate
numeric sparse path transcribes SparseImpl:1020-1570 for scalar U8/U16/U32/I32
with catalog IDs and no parent; other sparse shapes remain excluded.
Modified 2026-10-12: added an explicit WoWDBDefs build-70205 schema catalog
reader, independently normalizing its field declarations and checking target
table/layout hashes. This is not evidence that any client file was decoded.

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
  ``AutoProduceData`` (:394-398), ``GetMaxId`` (:955-977),
  ``AutoProduceRecordCopies`` (:613-634); ``GetFieldOffset`` (:889-905).
  The sparse path uses catalog IDs, primitive widths and Header.MaxId+1
  (:1020-1057, :1064, :1297-1320, :1487-1570).
* ``src/server/shared/DataStores/DB2Store.cpp`` (:101-107) passes the index-table size
  (maximum observed direct/copy-destination id plus one for regular files;
  Header.MaxId+1 for sparse files) to the corresponding copy routine.
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
  signed 32-bit array of two. The reviewed Forever ``Anim*`` tables add
  ``AnimKitBoneSetAliasMeta`` (1266937, 2 fields, layout ``0x19A6986C``),
  ``AnimKitPriorityMeta`` (1266540, 1 field, layout ``0xCCF889D8``) and
  ``AnimReplacementSetMeta`` (1284817, 1 field, layout ``0x7C047695``); all three
  declare ``IndexField = -1`` and a separate u32 id table. The reviewed Forever
  ``AnimKitBoneSetMeta`` (1375433, 5 fields, layout ``0x43E7736F``),
  ``AnimationDataMeta`` (1375431, 4 fields, layout ``0xBBF66A3C``) and
  ``AnimKitConfigMeta`` (1300872, 1 field, layout ``0x140718EF``) likewise declare
  ``IndexField = -1``, ``ParentIndexField = -1`` and a separate u32 id table;
  ``AnimKitBoneSet`` carries a ``FT_STRING_NOT_LOCALIZED`` leading field that
  decodes as ordinary text, and ``AnimationData`` a signed 32-bit array of two.
  The first reviewed float tables are ``ArmorLocationMeta`` (1284818, 5 fields,
  layout ``0xFB67352F``), ``BoneWindModifiersMeta`` (1416756, 2 fields, layout
  ``0x2CD1770E``, a ``Float(3)`` arm), ``AreaTriggerSphereMeta`` (1315630,
  1 field, ``0xB851EBF6``) and ``DurabilityQualityMeta`` (1130268, 1 field,
  ``0x08763C54``); all four declare ``IndexField = -1`` and
  ``ParentIndexField = -1``.
  The reviewed name tables add ``NamesProfanityMeta`` (:15780,
  ``0xF227E638``), ``NamesReservedMeta`` (:15800, ``0x2B2D5D97``),
  ``NamesReservedLocaleMeta`` (:15819, ``0x7B9823D4``), plus
  ``Cfg_CategoriesMeta`` (:2747, ``0x8710BE94``). Their leading string is
  followed respectively by a signed byte, no field, an unsigned byte, or
  unsigned short/byte/byte and signed int/byte fields. All have separate u32
  id tables and no parent; the three name tables use nonlocalized strings.
  The reviewed appearance tables include ``ChrModelMeta`` (:4049),
  ``ChrRaceXChrModelMeta`` (:4160), ``ChrCustomizationOptionMeta`` (:3909),
  ``ChrCustomizationChoiceMeta`` (:3715), and
  ``ChrCustomizationReqChoiceMeta`` (:3967). The first four carry their
  parent inside the file fields; the last appends a signed 32-bit parent.
  ``ItemSpecMeta`` (:12929) has five unsigned bytes and an unsigned short,
  with its parent in field 2; ``ItemSpecOverrideMeta`` (:12953) has an
  unsigned short and an appended signed parent; ``GemPropertiesMeta``
  (:9802) has an unsigned short and signed int without a parent.
  Birth-record schemas include ``SkillLineMeta`` (:19625),
  ``SkillRaceClassInfoMeta`` (:19715), ``SkillLineAbilityMeta`` (:19658),
  ``CharacterLoadoutMeta`` (:3206) and ``CharacterLoadoutItemMeta`` (:3229).
  SkillLine's fifth string is nonlocalized; its other four strings and
  SkillLineAbility's two strings are localized. Mask columns are signed
  int arrays of two; parent fields, where present, are in-record shorts.
  Initialization schemas add ``MapMeta`` (:14538), ``PowerTypeMeta``
  (:17452) and ``MovieMeta`` (:15329), all without parents. Map's first
  string and both PowerType strings are nonlocalized; Map's remaining
  strings and Movie's string are localized. Map carries a float pair and
  signed int array of three; PowerType has an inline unsigned id at field 2.
  Scalar spell dependencies add ``SpellCastTimesMeta`` (:20706),
  ``SpellDurationMeta`` (:21120), ``SpellRadiusMeta`` (:21807) and
  ``SpellProcsPerMinuteMeta`` (:21764). All use separate id tables with no
  parent. Radius has four separate float columns, not one array column.
* ``src/common/DataStores/DB2Meta.h`` (:32-49) has no table-hash member, so the
  loader's identity gate is LayoutHash (:1763) plus the field-count check
  (:1771); ``_header.TableHash`` is never validated against the metadata.

Canonical dump format, identical to ``db2_dump.rs``::

    forever-db2-dump v1 <table>
    <id>\t<field0>\t<field1>...

Every field prints its array values comma-joined; a value is ``u:<dec>``
(unsigned, including a full 64-bit ``FT_LONG``), ``i:<dec>`` (signed),
``f:0x<8 lowercase hex>`` (raw float bits) or ``t:<text>`` where ``\\`` ->
``\\\\``, tab -> ``\\t``, CR -> ``\\r`` and LF -> ``\\n``. Output is UTF-8 with
LF, terminated by a single trailing newline.
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
from pathlib import Path
import sys
from typing import Optional

HEADER_SIZE = 204
SECTION_SIZE = 40
MAX_SECTIONS = 1024
MAX_BYTES = 16 * 1024 * 1024
RECORD_POLICY_LIMIT = 100_000
COPY_POLICY_LIMIT = 100_000
VALUE_POLICY_LIMIT = 1_000_000
# Cumulative logical UTF-8 payload: direct values and every applicable copy,
# including overwrites. This is a decoder policy, not a WDC5 format limit.
TEXT_POLICY_LIMIT = 64 * 1024 * 1024
SPARSE_FLAG = 0x1
CATALOG_ID = "pinned-metadata-v1"
# Pin the complete factual artifact, not just its self-declared provenance.
# Reproduction against the immutable upstream Git object is tested separately.
CATALOG_SHA256 = "0f1ed0c419e851078d99f46ad3b37cadddb27b0556a2d1b52d3fd9ab62b76ec9"
WOWDBDEFS_70205_CATALOG_ID = "wowdbdefs-70205-v1"
WOWDBDEFS_70205_CATALOG_SHA256 = "fc8a2d07e18931be37a269e018e57ab00df6bd3b19794f2a025d06eaa280521b"

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
    """Schema identity plus a field list independently decoded by Python.

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
    appended_parent_unsigned: bool = False
    table_hash: Optional[int] = None
    other_layout_hashes: tuple = ()


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

# Transcribed from DB2Metadata.h::ChrSpecializationMeta (FileDataId 1343390,
# IndexField 3, ParentIndexField 4, FieldCount == FileFieldCount == 13, layout
# 0xDAB4CA4B): three strings, the inline unsigned 32-bit id at field 3 and field
# 4 is a signed 8-bit column that is ALSO the parent index. Because
# `parent = 4 < len(fields) = 13` the parent lives INSIDE the data block: the
# parent lookup overwrites the existing column 4 (with the field's own width)
# instead of appending a field, and the header keeps
# `total_field_count == field_count`.
CHR_SPECIALIZATION = Schema(
    name="ChrSpecialization",
    layout_hash=0xDAB4CA4B,
    index=3,
    parent=4,
    fields=(
        Field("string"),  # 0
        Field("string"),  # 1
        Field("string"),  # 2
        Field("unsigned", 32),  # 3 (inline index)
        _signed_field(8),  # 4 (parent index, FT_BYTE)
        _signed_field(8),  # 5
        _signed_field(8),  # 6
        _signed_field(8),  # 7
        _signed_field(32),  # 8
        _signed_field(32),  # 9
        _signed_field(8),  # 10
        _signed_field(32),  # 11
        _signed_field(32, 2),  # 12 array of two
    ),
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
# layout 0x5774D75A): one unsigned 32-bit field and two signed 32-bit fields;
# the id is read from the inline field 0 (FT_INT IsSigned = false).
CONTENT_RESTRICTION_RULE_SET = Schema(
    name="ContentRestrictionRuleSet",
    layout_hash=0x5774D75A,
    index=0,
    fields=(
        Field("unsigned", 32),  # 0 (inline index)
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

# Transcribed from DB2Metadata.h::AnimKitBoneSetAliasMeta (FileDataId 1266937,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x19A6986C): two unsigned 8-bit fields, separate u32 id table.
ANIM_KIT_BONE_SET_ALIAS = Schema(
    name="AnimKitBoneSetAlias",
    layout_hash=0x19A6986C,
    index=-1,
    fields=(
        Field("unsigned", 8),  # 0
        Field("unsigned", 8),  # 1
    ),
)

# Transcribed from DB2Metadata.h::AnimKitPriorityMeta (FileDataId 1266540,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xCCF889D8): a single unsigned 8-bit field, separate u32 id table.
ANIM_KIT_PRIORITY = Schema(
    name="AnimKitPriority",
    layout_hash=0xCCF889D8,
    index=-1,
    fields=(Field("unsigned", 8),),
)

# Transcribed from DB2Metadata.h::AnimReplacementSetMeta (FileDataId 1284817,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x7C047695): a single unsigned 8-bit field, separate u32 id table.
ANIM_REPLACEMENT_SET = Schema(
    name="AnimReplacementSet",
    layout_hash=0x7C047695,
    index=-1,
    fields=(Field("unsigned", 8),),
)

# Transcribed from DB2Metadata.h::AnimKitBoneSetMeta (FileDataId 1375433,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x43E7736F): one FT_STRING_NOT_LOCALIZED string (decoded as ordinary text),
# two signed 32-bit fields and two signed 8-bit fields.
ANIM_KIT_BONE_SET = Schema(
    name="AnimKitBoneSet",
    layout_hash=0x43E7736F,
    index=-1,
    fields=(
        Field("string"),  # 0
        _signed_field(32),  # 1
        _signed_field(8),  # 2
        _signed_field(8),  # 3
        _signed_field(32),  # 4
    ),
)

# Transcribed from DB2Metadata.h::AnimationDataMeta (FileDataId 1375431,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xBBF66A3C): an unsigned 16-bit field, a signed 8-bit field, a signed 16-bit
# field and a signed 32-bit array of two.
ANIMATION_DATA = Schema(
    name="AnimationData",
    layout_hash=0xBBF66A3C,
    index=-1,
    fields=(
        Field("unsigned", 16),  # 0
        _signed_field(8),  # 1
        _signed_field(16),  # 2
        _signed_field(32, 2),  # 3 array of two
    ),
)

# Transcribed from DB2Metadata.h::AnimKitConfigMeta (FileDataId 1300872,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x140718EF): a single signed 32-bit field, separate u32 id table.
ANIM_KIT_CONFIG = Schema(
    name="AnimKitConfig",
    layout_hash=0x140718EF,
    index=-1,
    fields=(_signed_field(32),),
)

# Transcribed from DB2Metadata.h::ItemExtendedCostMeta (FileDataId 801681,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x22331DAB): an unsigned 64-bit (FT_LONG) field then ten further scalar and
# array fields; separate u32 id table. The 64-bit field is read as 8
# little-endian bytes in compression 0.
ITEM_EXTENDED_COST = Schema(
    name="ItemExtendedCost",
    layout_hash=0x22331DAB,
    index=-1,
    fields=(
        Field("unsigned", 64),  # 0
        Field("unsigned", 16),  # 1
        _signed_field(8),  # 2
        _signed_field(32),  # 3
        Field("unsigned", 8),  # 4
        _signed_field(32),  # 5
        Field("unsigned", 8),  # 6
        _signed_field(32, 5),  # 7
        Field("unsigned", 16, 5),  # 8
        Field("unsigned", 16, 5),  # 9
        Field("unsigned", 32, 5),  # 10
    ),
)

# Transcribed from DB2Metadata.h::TransmogOutfitEntryMeta (FileDataId 7320164,
# IndexField 2, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x91D51304): an unsigned 64-bit (FT_LONG) field, a string and seven further
# fields; the record id comes from the inline field 2.
TRANSMOG_OUTFIT_ENTRY = Schema(
    name="TransmogOutfitEntry",
    layout_hash=0x91D51304,
    index=2,
    fields=(
        Field("unsigned", 64),  # 0
        Field("string"),  # 1
        Field("unsigned", 32),  # 2 (inline index)
        _signed_field(32),  # 3
        Field("unsigned", 8),  # 4
        _signed_field(32),  # 5
        Field("unsigned", 8),  # 6
        Field("float", 32, 1),  # 7
        _signed_field(32),  # 8
    ),
)

# Transcribed from DB2Metadata.h::BankTabMeta (FileDataId 5242411, IndexField -1,
# ParentIndexField -1, FieldCount == FileFieldCount, layout 0x5B2F9B36): an
# unsigned 64-bit (FT_LONG) field, an unsigned byte, a signed byte and six
# signed 32-bit fields; separate u32 id table.
BANK_TAB = Schema(
    name="BankTab",
    layout_hash=0x5B2F9B36,
    index=-1,
    fields=(
        Field("unsigned", 64),  # 0
        Field("unsigned", 8),  # 1
        _signed_field(8),  # 2
        _signed_field(32),  # 3
        _signed_field(32),  # 4
        _signed_field(32),  # 5
        _signed_field(32),  # 6
        _signed_field(32),  # 7
        _signed_field(32),  # 8
    ),
)

# Transcribed from DB2Metadata.h::ArmorLocationMeta (FileDataId 1284818,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xFB67352F): five floats, separate u32 id table. First reviewed all-float
# table.
ARMOR_LOCATION = Schema(
    name="ArmorLocation",
    layout_hash=0xFB67352F,
    index=-1,
    fields=(
        Field("float", 32, 1),  # 0
        Field("float", 32, 1),  # 1
        Field("float", 32, 1),  # 2
        Field("float", 32, 1),  # 3
        Field("float", 32, 1),  # 4
    ),
)

# Transcribed from DB2Metadata.h::BoneWindModifiersMeta (FileDataId 1416756,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x2CD1770E): a float array of three followed by one float, separate u32 id
# table. First reviewed float-array column.
BONE_WIND_MODIFIERS = Schema(
    name="BoneWindModifiers",
    layout_hash=0x2CD1770E,
    index=-1,
    fields=(
        Field("float", 32, 3),  # 0 array of three
        Field("float", 32, 1),  # 1
    ),
)

# Transcribed from DB2Metadata.h::AreaTriggerSphereMeta (FileDataId 1315630,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0xB851EBF6): a single float, separate u32 id table.
AREA_TRIGGER_SPHERE = Schema(
    name="AreaTriggerSphere",
    layout_hash=0xB851EBF6,
    index=-1,
    fields=(Field("float", 32, 1),),
)

# Transcribed from DB2Metadata.h::DurabilityQualityMeta (FileDataId 1130268,
# IndexField -1, ParentIndexField -1, FieldCount == FileFieldCount, layout
# 0x08763C54): a single float, separate u32 id table.
DURABILITY_QUALITY = Schema(
    name="DurabilityQuality",
    layout_hash=0x08763C54,
    index=-1,
    fields=(Field("float", 32, 1),),
)

# Transcribed from pinned DB2Metadata.h:15780 (FileDataId 1117086):
# FT_STRING_NOT_LOCALIZED followed by signed FT_BYTE; separate u32 id table.
NAMES_PROFANITY = Schema(
    name="NamesProfanity",
    layout_hash=0xF227E638,
    index=-1,
    fields=(Field("string"), _signed_field(8)),
)

# DB2Metadata.h:15800 (FileDataId 1117085): one nonlocalized string.
NAMES_RESERVED = Schema(
    name="NamesReserved",
    layout_hash=0x2B2D5D97,
    index=-1,
    fields=(Field("string"),),
)

# DB2Metadata.h:15819 (FileDataId 1117087): nonlocalized string plus UNSIGNED
# FT_BYTE. This byte deliberately differs from NamesProfanity's signed byte.
NAMES_RESERVED_LOCALE = Schema(
    name="NamesReservedLocale",
    layout_hash=0x7B9823D4,
    index=-1,
    fields=(Field("string"), Field("unsigned", 8)),
)

# DB2Metadata.h:2747 (FileDataId 1068162): FT_STRING, unsigned FT_SHORT,
# unsigned FT_BYTE twice, signed FT_INT, then signed FT_BYTE.
CFG_CATEGORIES = Schema(
    name="Cfg_Categories",
    layout_hash=0x8710BE94,
    index=-1,
    fields=(
        Field("string"), Field("unsigned", 16), Field("unsigned", 8),
        Field("unsigned", 8), _signed_field(32), _signed_field(8),
    ),
)

# DB2Metadata.h:4049 (FileDataId 3384313): two float arrays, an inline u32
# id, signed byte, seven signed ints (including parent field 4), six floats.
CHR_MODEL = Schema(
    name="ChrModel", layout_hash=0x03FAB755, index=2, parent=4,
    fields=(
        Field("float", 32, 3), Field("float", 32, 3), Field("unsigned", 32),
        _signed_field(8), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32), Field("float"), Field("float"),
        Field("float"), Field("float"), Field("float"), Field("float"),
    ),
)

# DB2Metadata.h:4160 (FileDataId 3490304): separate id table; signed byte
# parent at field 0, signed int, signed byte and signed int.
CHR_RACE_X_CHR_MODEL = Schema(
    name="ChrRaceXChrModel", layout_hash=0xA203BC29, index=-1, parent=0,
    fields=(_signed_field(8), _signed_field(32), _signed_field(8), _signed_field(32)),
)

# DB2Metadata.h:3909 (FileDataId 3384247): localized string, inline u32 id,
# unsigned short, signed ints (parent at 4), float and further signed ints.
CHR_CUSTOMIZATION_OPTION = Schema(
    name="ChrCustomizationOption", layout_hash=0xDCC2A86E, index=1, parent=4,
    fields=(
        Field("string"), Field("unsigned", 32), Field("unsigned", 16),
        _signed_field(32), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32), Field("float"),
        _signed_field(32), _signed_field(32), _signed_field(32), _signed_field(32),
    ),
)

# DB2Metadata.h:3715 (FileDataId 3450554): localized string, inline u32 id,
# signed parent at 2, unsigned shorts at 5/6, and a signed int array at 10.
CHR_CUSTOMIZATION_CHOICE = Schema(
    name="ChrCustomizationChoice", layout_hash=0x9559C358, index=1, parent=2,
    fields=(
        Field("string"), Field("unsigned", 32), _signed_field(32),
        _signed_field(32), _signed_field(32), Field("unsigned", 16),
        Field("unsigned", 16), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32, 2),
    ),
)

# DB2Metadata.h:3967 (FileDataId 3580359): one signed int file field, then
# a signed int appended parent; Meta FieldCount 2, FileFieldCount 1.
CHR_CUSTOMIZATION_REQ_CHOICE = Schema(
    name="ChrCustomizationReqChoice", layout_hash=0xF925BC6F, index=-1, parent=1,
    fields=(_signed_field(32),),
)

# DB2Metadata.h:12929 (FileDataId 1135120): five unsigned bytes and an
# unsigned short; byte field 2 is an in-record parent. Separate u32 id table.
ITEM_SPEC = Schema(
    name="ItemSpec", layout_hash=0x83F3D113, index=-1, parent=2,
    fields=(
        Field("unsigned", 8), Field("unsigned", 8), Field("unsigned", 8),
        Field("unsigned", 8), Field("unsigned", 8), Field("unsigned", 16),
    ),
)

# DB2Metadata.h:12953 (FileDataId 1134576): one unsigned short file field,
# then an appended signed int parent; Meta FieldCount 2, FileFieldCount 1.
ITEM_SPEC_OVERRIDE = Schema(
    name="ItemSpecOverride", layout_hash=0xB292998C, index=-1, parent=1,
    fields=(Field("unsigned", 16),),
)

# DB2Metadata.h:9802 (FileDataId 1343604): unsigned short and signed int.
GEM_PROPERTIES = Schema(
    name="GemProperties", layout_hash=0x86487AD2, index=-1,
    fields=(Field("unsigned", 16), _signed_field(32)),
)

# DB2Metadata.h:19625 (FileDataId 1240935): first four strings FT_STRING,
# fifth FT_STRING_NOT_LOCALIZED; inline unsigned id at 5, no parent.
SKILL_LINE = Schema(
    name="SkillLine", layout_hash=0x6763217C, index=5,
    fields=(
        Field("string"), Field("string"), Field("string"), Field("string"),
        Field("string"), Field("unsigned", 32), _signed_field(8),
        _signed_field(32), _signed_field(8), Field("unsigned", 32),
        _signed_field(32), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32),
    ),
)

# DB2Metadata.h:19715 (FileDataId 1240406): signed short parent at 0,
# separate id table, and signed int array-of-two at field 6.
SKILL_RACE_CLASS_INFO = Schema(
    name="SkillRaceClassInfo", layout_hash=0x24277B48, index=-1, parent=0,
    fields=(
        _signed_field(16), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(8), _signed_field(16), _signed_field(32, 2),
    ),
)

# DB2Metadata.h:19658 (FileDataId 1266278): two FT_STRING fields, inline
# unsigned id at 2, signed short parent at 3, two signed int arrays at 16/17.
SKILL_LINE_ABILITY = Schema(
    name="SkillLineAbility", layout_hash=0x224F7EA0, index=2, parent=3,
    fields=(
        Field("string"), Field("string"), Field("unsigned", 32),
        _signed_field(16), _signed_field(32), _signed_field(16),
        _signed_field(32), _signed_field(32), _signed_field(32),
        _signed_field(16), _signed_field(16), _signed_field(32),
        _signed_field(8), _signed_field(16), _signed_field(16),
        _signed_field(16), _signed_field(32, 2), _signed_field(32, 2),
    ),
)

# DB2Metadata.h:3206 (FileDataId 1344281): signed byte/int, unsigned byte,
# signed int and signed int array-of-two. Separate id table, no parent.
CHARACTER_LOADOUT = Schema(
    name="CharacterLoadout", layout_hash=0x713CE8BB, index=-1,
    fields=(
        _signed_field(8), _signed_field(32), Field("unsigned", 8),
        _signed_field(32), _signed_field(32, 2),
    ),
)

# DB2Metadata.h:3229 (FileDataId 1302846): unsigned short parent at 0,
# unsigned int at 1 and a separate id table.
CHARACTER_LOADOUT_ITEM = Schema(
    name="CharacterLoadoutItem", layout_hash=0x0C7A1862, index=-1, parent=0,
    fields=(Field("unsigned", 16), Field("unsigned", 32)),
)

# DB2Metadata.h:14538 (FileDataId 1349477): first string nonlocalized,
# remaining five localized; float pair at 6 and signed int array at 25.
MAP = Schema(
    name="Map", layout_hash=0xD43AFAC3, index=-1,
    fields=(
        Field("string"), Field("string"), Field("string"), Field("string"),
        Field("string"), Field("string"), Field("float", 32, 2),
        Field("unsigned", 8), _signed_field(8), Field("unsigned", 8),
        Field("unsigned", 16), _signed_field(16), _signed_field(16),
        _signed_field(16), _signed_field(16), Field("unsigned", 8),
        Field("float"), _signed_field(16), Field("unsigned", 8),
        _signed_field(16), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32), _signed_field(32), _signed_field(32, 3),
    ),
)

# DB2Metadata.h:17452 (FileDataId 1266022): two nonlocalized strings,
# inline unsigned id at 2, signed byte/ints and two scalar floats.
POWER_TYPE = Schema(
    name="PowerType", layout_hash=0x14BBEEA1, index=2,
    fields=(
        Field("string"), Field("string"), Field("unsigned", 32),
        _signed_field(8), _signed_field(32), _signed_field(32),
        _signed_field(32), _signed_field(32), _signed_field(32),
        _signed_field(32), Field("float"), Field("float"), _signed_field(32),
    ),
)

# DB2Metadata.h:15329 (FileDataId 1332556): localized string, two
# unsigned bytes and three unsigned ints. Separate u32 id table.
MOVIE = Schema(
    name="Movie", layout_hash=0xF53888FA, index=-1,
    fields=(
        Field("string"), Field("unsigned", 8), Field("unsigned", 8),
        Field("unsigned", 32), Field("unsigned", 32), Field("unsigned", 32),
    ),
)

# DB2Metadata.h:20706 (FileDataId 1134089): two signed int scalars.
SPELL_CAST_TIMES = Schema(
    name="SpellCastTimes", layout_hash=0x75B6BD3A, index=-1,
    fields=(_signed_field(32), _signed_field(32)),
)

# DB2Metadata.h:21120 (FileDataId 1137828): three signed int scalars.
SPELL_DURATION = Schema(
    name="SpellDuration", layout_hash=0xA931BD2B, index=-1,
    fields=(_signed_field(32), _signed_field(32), _signed_field(32)),
)

# DB2Metadata.h:21807 (FileDataId 1134584): four distinct float scalars.
SPELL_RADIUS = Schema(
    name="SpellRadius", layout_hash=0xF9A913EE, index=-1,
    fields=(Field("float"), Field("float"), Field("float"), Field("float")),
)

# DB2Metadata.h:21764 (FileDataId 1133526): float then signed int.
SPELL_PROCS_PER_MINUTE = Schema(
    name="SpellProcsPerMinute", layout_hash=0x2227014B, index=-1,
    fields=(Field("float"), _signed_field(32)),
)

SCHEMAS = {
    "ChrClasses": CHR_CLASSES,
    "ChrRaces": CHR_RACES,
    "ChrClassTitle": CHR_CLASS_TITLE,
    "ChrClassUIDisplay": CHR_CLASS_UI_DISPLAY,
    "ChrClassUIChrModelInfo": CHR_CLASS_UI_CHR_MODEL_INFO,
    "ChrClassVillain": CHR_CLASS_VILLAIN,
    "ChrClassesXPowerTypes": CHR_CLASSES_X_POWER_TYPES,
    "ChrSpecialization": CHR_SPECIALIZATION,
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
    "AnimKitBoneSetAlias": ANIM_KIT_BONE_SET_ALIAS,
    "AnimKitPriority": ANIM_KIT_PRIORITY,
    "AnimReplacementSet": ANIM_REPLACEMENT_SET,
    "AnimKitBoneSet": ANIM_KIT_BONE_SET,
    "AnimationData": ANIMATION_DATA,
    "AnimKitConfig": ANIM_KIT_CONFIG,
    "ItemExtendedCost": ITEM_EXTENDED_COST,
    "TransmogOutfitEntry": TRANSMOG_OUTFIT_ENTRY,
    "BankTab": BANK_TAB,
    "ArmorLocation": ARMOR_LOCATION,
    "BoneWindModifiers": BONE_WIND_MODIFIERS,
    "AreaTriggerSphere": AREA_TRIGGER_SPHERE,
    "DurabilityQuality": DURABILITY_QUALITY,
    "NamesProfanity": NAMES_PROFANITY,
    "NamesReserved": NAMES_RESERVED,
    "NamesReservedLocale": NAMES_RESERVED_LOCALE,
    "Cfg_Categories": CFG_CATEGORIES,
    "ChrModel": CHR_MODEL,
    "ChrRaceXChrModel": CHR_RACE_X_CHR_MODEL,
    "ChrCustomizationOption": CHR_CUSTOMIZATION_OPTION,
    "ChrCustomizationChoice": CHR_CUSTOMIZATION_CHOICE,
    "ChrCustomizationReqChoice": CHR_CUSTOMIZATION_REQ_CHOICE,
    "ItemSpec": ITEM_SPEC,
    "ItemSpecOverride": ITEM_SPEC_OVERRIDE,
    "GemProperties": GEM_PROPERTIES,
    "SkillLine": SKILL_LINE,
    "SkillRaceClassInfo": SKILL_RACE_CLASS_INFO,
    "SkillLineAbility": SKILL_LINE_ABILITY,
    "CharacterLoadout": CHARACTER_LOADOUT,
    "CharacterLoadoutItem": CHARACTER_LOADOUT_ITEM,
    "Map": MAP,
    "PowerType": POWER_TYPE,
    "Movie": MOVIE,
    "SpellCastTimes": SPELL_CAST_TIMES,
    "SpellDuration": SPELL_DURATION,
    "SpellRadius": SPELL_RADIUS,
    "SpellProcsPerMinute": SPELL_PROCS_PER_MINUTE,
}


def load_catalog(catalog_id=CATALOG_ID, path=None):
    """Construct schemas from verified raw facts, independently of Rust Kinds.

    This is explicit opt-in. It never modifies the reviewed SCHEMAS mapping,
    fetches metadata, reads a client or infers record/runtime acceptance.
    """
    if catalog_id == CATALOG_ID:
        default_path = Path(__file__).resolve().parents[2] / "contracts/forever-metadata-v1.json"
        expected_digest = CATALOG_SHA256
    elif catalog_id == WOWDBDEFS_70205_CATALOG_ID:
        default_path = Path(__file__).resolve().parents[2] / "contracts/wowdbdefs-70205-v1.json"
        expected_digest = WOWDBDEFS_70205_CATALOG_SHA256
    else:
        raise OracleError("invalid_invocation")
    path = Path(path) if path is not None else default_path
    try:
        with path.open("rb") as handle:
            raw = handle.read(4 * 1024 * 1024 + 1)
        if hashlib.sha256(raw).hexdigest() != expected_digest:
            raise OracleError("unsupported_table_schema")
        document = json.loads(raw)
        result = {}
        if catalog_id == CATALOG_ID:
            if (
                document['catalog_id'] != CATALOG_ID
                or document['source']['commit'] != '02245dcd245e7433e524577656177723d3e4992e'
            ):
                raise OracleError("unsupported_table_schema")
            widths = {'FT_BYTE': 8, 'FT_SHORT': 16, 'FT_INT': 32, 'FT_LONG': 64}
            for table in document['tables']:
                fields = []
                for field in table['fields'][:table['file_field_count']]:
                    kind = field['type']
                    if kind in ('FT_STRING', 'FT_STRING_NOT_LOCALIZED'):
                        fields.append(Field('string', 32, field['array_size']))
                    elif kind == 'FT_FLOAT':
                        fields.append(Field('float', 32, field['array_size']))
                    else:
                        fields.append(Field('signed' if field['signed'] else 'unsigned', widths[kind], field['array_size'], signed=field['signed']))
                parent_unsigned = table['parent_index'] >= table['file_field_count'] and not table['fields'][table['parent_index']]['signed']
                result[table['name']] = Schema(table['name'], table['layout_hash'], table['index'], tuple(fields), table['parent_index'], parent_unsigned)
            if len(result) != 1126:
                raise OracleError("unsupported_table_schema")
            return result

        source = document['source']
        if (
            document['catalog_id'] != WOWDBDEFS_70205_CATALOG_ID
            or document['target'] != {
                'product': 'wow_classic_beta', 'version': '1.60.1.70205', 'build': 70205
            }
            or source['commit'] != '674636522d51c010f250ad1b5fe549ef6bc8eb0a'
            or source['manifest_blob'] != '8a5d8122f312ae6a9564e8286fdc491831fecc22'
            or source['manifest_sha256'] != '4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265'
        ):
            raise OracleError("unsupported_table_schema")
        widths = {'Byte': 8, 'Short': 16, 'Int': 32, 'Long': 64}
        for table in document['tables']:
            if table['blockers']:
                raise OracleError("unsupported_table_schema")
            file_field_count = table['file_field_count']
            declared_fields = table['fields']
            if not 1 <= file_field_count <= len(declared_fields) or len(declared_fields) != table['field_count']:
                raise OracleError("unsupported_table_schema")
            fields = []
            for field in declared_fields[:file_field_count]:
                kind = field['kind']
                if kind in ('String', 'NonLocalizedString'):
                    fields.append(Field('string', 32, field['array_size']))
                elif kind == 'Float':
                    fields.append(Field('float', 32, field['array_size']))
                elif kind in widths:
                    fields.append(Field('signed' if field['signed'] else 'unsigned', widths[kind], field['array_size'], signed=field['signed']))
                else:
                    raise OracleError("unsupported_table_schema")
            parent = table['parent_index']
            if parent < -1 or parent >= len(declared_fields):
                raise OracleError("unsupported_table_schema")
            parent_unsigned = parent >= file_field_count and not declared_fields[parent]['signed']
            schema = Schema(
                table['name'], table['layout_hash'], table['index'], tuple(fields), parent,
                parent_unsigned, table['table_hash'], tuple(table['other_layout_hashes']),
            )
            if schema.name in result:
                raise OracleError("unsupported_table_schema")
            result[schema.name] = schema
        if len(result) != 1161:
            raise OracleError("unsupported_table_schema")
        return result
    except (OSError, KeyError, TypeError, ValueError):
        raise OracleError("unsupported_table_schema")


def resolve_schema(table_name, schema_catalog=None):
    if schema_catalog is None:
        schema = SCHEMAS.get(table_name)
    elif schema_catalog == CATALOG_ID:
        schema = SCHEMAS.get(table_name) or load_catalog(schema_catalog).get(table_name)
    elif schema_catalog == WOWDBDEFS_70205_CATALOG_ID:
        schema = load_catalog(schema_catalog).get(table_name)
    else:
        raise OracleError("invalid_invocation")
    if schema is None:
        raise OracleError("unsupported_table_schema")
    return schema


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
        """Mode 0..5 value extraction (DB2FileLoader.cpp:808-905).

        Compression 0 reads ``width // 8`` little-endian bytes, so an unsigned
        64-bit (``FT_LONG``) field is returned whole. Packed/pallet modes stay
        bounded to 32 bits because their storage is a single 32-bit word.
        """
        if array >= count or width == 0 or width > 64:
            raise OracleError("invalid_column_data")
        mask = (1 << width) - 1
        mode = self.compression
        if mode == 0:
            # Direct-value storage requires whole-byte widths and offsets.
            if self.offset % 8 or width % 8:
                raise OracleError("invalid_column_data")
            byte_width = width // 8
            at = self.offset // 8 + array * byte_width
            value = int.from_bytes(_slice(record, at, byte_width), "little")
        elif mode == 2:
            # CommonData copies only four bytes into native T; a 64-bit T
            # retains indeterminate upper bytes, not a defined zero extension.
            if width > 32 or count != 1:
                raise OracleError("invalid_column_data")
            value = self.common.get(record_id, self.a)
        else:
            if width > 32 or self.b == 0 or self.b > 32 or (mode != 4 and count != 1):
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


class _TextBudget:
    def __init__(self, limit):
        self.limit = limit
        self.used = 0

    def charge(self, size):
        if size > self.limit - self.used:
            raise OracleError("record_policy_limit")
        self.used += size


def _string_value(data, column, bits, record_start, strings_start, strings_end, budget, array=0,
                  require_non_null=False):
    if column.compression != 0:
        raise OracleError("invalid_column_data")
    if bits == 0:
        if require_non_null:
            # GetRecord.GetString returns nullptr; System.cpp:294-295 would
            # assign it to std::string, including rows later filtered by WDT.
            raise OracleError("invalid_record_string")
        return ""
    # DB2FileLoader.cpp:800: each pointer is relative to its own array slot.
    at = record_start + column.offset // 8 + 4 * array + bits
    if not strings_start <= at < strings_end:
        raise OracleError("invalid_record_string")
    # Search the original bytes: slicing the whole remaining block would copy
    # potentially megabytes even when the referenced string is empty.
    nul = data.find(b"\x00", at, strings_end)
    if nul < 0:
        raise OracleError("invalid_record_string")
    budget.charge(nul - at)
    try:
        return data[at:nul].decode("utf-8")
    except UnicodeDecodeError:
        raise OracleError("invalid_record_string")


def _signed_value(bits, width):
    mask = (1 << width) - 1
    value = bits & mask
    return value - (1 << width) if value >= (1 << (width - 1)) else value


@dataclasses.dataclass
class _Value:
    kind: str
    number: int = 0
    text: str = ""


def _parent_in_data_value(field, parent_id):
    """Value written into a parent index field that lives inside the data block.

    Mirrors the Rust ``parent_in_data_value`` / reference
    ``DB2FileLoader.cpp:650-712``: the write uses the field's own kind/width and
    a ParentId that does not fit is ``invalid_record_ids`` (the reference
    ``ASSERT``). FT_INT preserves the u32 bits, interpreted as signed when
    the reviewed field kind is signed.
    """
    if field.kind == "signed":
        if field.bits == 8:
            if parent_id > 0xFF:
                raise OracleError("invalid_record_ids")
            return _Value("signed", number=_signed_value(parent_id, 8))
        if field.bits == 16:
            if parent_id > 0xFFFF:
                raise OracleError("invalid_record_ids")
            return _Value("signed", number=_signed_value(parent_id, 16))
        return _Value("signed", number=_signed_value(parent_id, 32))
    if field.kind == "unsigned":
        if field.bits == 8:
            if parent_id > 0xFF:
                raise OracleError("invalid_record_ids")
            return _Value("unsigned", number=parent_id)
        if field.bits == 16:
            if parent_id > 0xFFFF:
                raise OracleError("invalid_record_ids")
            return _Value("unsigned", number=parent_id)
        return _Value("unsigned", number=parent_id)
    # A string/float parent field is not a reviewed shape.
    raise OracleError("unsupported_table_schema")


def _decode_sparse(data, header, sections, schema):
    """Independent transcription of pinned SparseImpl:1020-1570.

    The first subset has catalog IDs, scalar U8/U16/U32/I32 fields and no
    parent. Primitive widths determine sequential offsets; neither regular
    column bit offsets nor the conventional ID table determine values here.
    """
    if schema.parent >= 0 or schema.index >= 0 or any(
        field.count != 1
        or not (
            (field.kind == "unsigned" and field.bits in (8, 16, 32))
            or (field.kind == "signed" and field.bits == 32)
        )
        for field in schema.fields
    ):
        # Sparse AutoProduceData:1055 rejects appended parents even if empty.
        raise OracleError("unsupported_record_variant")
    if header.parent_lookup_count > 1:
        raise OracleError("unsupported_record_variant")
    if header.parent_lookup_count:
        raise OracleError("unsupported_table_schema")
    if (len(sections) > 1 or header.string_table_size
            or header.common_data_size or header.pallet_data_size):
        raise OracleError("unsupported_record_variant")
    values_per_row = len(schema.fields)
    value_count = header.record_count * values_per_row
    if header.record_count > RECORD_POLICY_LIMIT or value_count > VALUE_POLICY_LIMIT:
        raise OracleError("record_policy_limit")
    if header.min_id > header.max_id:
        raise OracleError("record_data_bounds")
    # Sparse allocates MaxId+1, not the regular maximum of IDs/copy targets.
    if header.max_id == 0xFFFFFFFF:
        raise OracleError("invalid_record_ids")
    extent = header.max_id + 1
    primitive_at = HEADER_SIZE + len(sections) * SECTION_SIZE
    primitive_end = primitive_at + header.field_count * 4
    _slice(data, primitive_at, header.field_count * 4)
    widths = []
    for field_number, field in enumerate(schema.fields):
        unused = _i16(data, primitive_at + field_number * 4)
        if (not 0 <= unused <= 31
                or (field.bits == 8 and unused != 24)
                or (field.bits == 16 and unused != 16)):
            raise OracleError("invalid_column_data")
        widths.append((4 - unused // 8, 32 - unused))
    if not sections:
        if header.record_count or header.column_meta_size:
            raise OracleError("unsupported_record_variant")
        if len(data) != primitive_end:
            raise OracleError("record_data_bounds")
        return {}
    section = sections[0]
    if section.tact or section.string_table_size or section.parent_lookup_data_size:
        raise OracleError("unsupported_record_variant")
    if section.copy_table_count > COPY_POLICY_LIMIT:
        raise OracleError("record_policy_limit")
    if section.record_count != header.record_count or section.catalog_data_count != header.record_count:
        raise OracleError("record_data_bounds")
    if header.column_meta_size not in (0, header.total_field_count * 24):
        raise OracleError("invalid_column_data")
    metadata_end = primitive_end + header.column_meta_size
    _slice(data, primitive_end, header.column_meta_size)
    if header.column_meta_size:
        for field_number in range(values_per_row):
            at = primitive_end + field_number * 24
            if _u32(data, at + 4) or _u32(data, at + 8):
                raise OracleError("unsupported_record_variant")
    start, end = section.file_offset, section.catalog_data_offset
    if start != metadata_end or end < start or end > len(data):
        raise OracleError("record_data_bounds")
    id_bytes = section.catalog_data_count * 4
    if section.id_table_size != id_bytes:
        raise OracleError("invalid_record_ids")
    copies_at = end + id_bytes
    entries_at = copies_at + section.copy_table_count * 8
    trailing_ids_at = entries_at + section.catalog_data_count * 6
    if trailing_ids_at + id_bytes != len(data):
        raise OracleError("record_data_bounds")
    rows = {}
    for ordinal in range(section.catalog_data_count):
        record_id = _u32(data, end + ordinal * 4)
        if record_id >= extent or record_id in rows:
            raise OracleError("invalid_record_ids")
        entry = entries_at + ordinal * 6
        at, size = _u32(data, entry), _u16(data, entry + 4)
        if at < start or at + size > end or not size:
            raise OracleError("record_data_bounds")
        record = _slice(data, at, size)
        cursor, fields = 0, []
        for field, (width, significant) in zip(schema.fields, widths):
            number = int.from_bytes(_slice(record, cursor, width), "little")
            if field.kind == "signed":
                number = _signed_value(number, significant)
            elif field.bits == 32:
                number &= (1 << significant) - 1
            fields.append([_Value(field.kind, number=number)])
            cursor += width
        # C++ sparse getters ignore trailing record padding and Header.RecordSize.
        rows[record_id] = fields
    for copy_number in range(section.copy_table_count):
        at = copies_at + copy_number * 8
        new, source = _u32(data, at), _u32(data, at + 4)
        if not source or source >= extent or new >= extent or source not in rows:
            continue
        if source == new:
            raise OracleError("unsupported_record_variant")
        if new not in rows:
            value_count += values_per_row
            if value_count > VALUE_POLICY_LIMIT:
                raise OracleError("record_policy_limit")
        rows[new] = [[dataclasses.replace(value) for value in group] for group in rows[source]]
    return dict(sorted(rows.items()))


def _decode_parsed(data, header, sections, schema, terrain_role=None, text_limit=None):
    if header.flags & SPARSE_FLAG:
        if terrain_role is not None:
            raise OracleError("unsupported_record_variant")
        return _decode_sparse(data, header, sections, schema)
    # DB2FileLoader.cpp:1767 rejects ParentLookupCount > 1; :1775 rejects a
    # lookup only when the reviewed schema has no parent. A parent-bearing
    # schema may have no lookup: in-record values remain intact and appended
    # parents keep their initialized zero (:480-505, :509-510).
    if header.parent_lookup_count > 1:
        raise OracleError("unsupported_record_variant")
    if header.parent_lookup_count == 1 and schema.parent < 0:
        raise OracleError("unsupported_table_schema")
    if len(sections) == 0:
        # DB2FileLoader.cpp:1782-1784 still reads primitive field metadata,
        # four bytes per file field; :1903-1912 checks the exact file end when
        # no section exists. Admit this complete minimal empty variant only.
        if header.record_count != 0:
            raise OracleError("unsupported_record_variant")
        if (
            header.string_table_size != 0
            or header.column_meta_size != 0
            or header.common_data_size != 0
            or header.pallet_data_size != 0
        ):
            raise OracleError("unsupported_record_variant")
        if len(data) != HEADER_SIZE + header.field_count * 4:
            raise OracleError("record_data_bounds")
        return {} if terrain_role is None else TerrainSources(terrain_role, (), (), 0,
            TEXT_POLICY_LIMIT if text_limit is None else text_limit)
    if len(sections) > 1:
        raise OracleError("unsupported_record_variant")
    section = sections[0]
    if section.tact or section.catalog_data_count != 0:
        raise OracleError("unsupported_record_variant")
    # Bound decoded scalar values, including arrays and an appended parent.
    # The same budget also applies to distinct rows added by the copy table.
    values_per_row = sum(field.count for field in schema.fields)
    if schema.parent >= len(schema.fields):
        values_per_row += 1
    value_count = header.record_count * values_per_row
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
        or (terrain_role is None and header.min_id > header.max_id)
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
    text_budget = _TextBudget(TEXT_POLICY_LIMIT if text_limit is None else text_limit)
    terrain_direct = []
    # Cache UTF-8 byte lengths by row so copies never re-encode Python strings.
    # Python shares immutable str objects, but the logical budget deliberately
    # mirrors Rust's owned String clones and bounds canonical rendered output.
    row_text_bytes = {}
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
        if record_id > 0xFFFFFFFF or (terrain_role is None and
                (record_id > header.max_id or record_id in rows)):
            raise OracleError("invalid_record_ids")
        fields = []
        text_before_row = text_budget.used
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
                                text_budget,
                                array,
                                require_non_null=(terrain_role in ("Map", "VMap") and field in (0, 1)),
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
        if terrain_role is not None:
            if terrain_role == "Map":
                projected = (fields[21][0].number, fields[1][0].text, fields[0][0].text)
            elif terrain_role == "VMap":
                parent = _signed_value(fields[13][0].number & 0xFFFF, 16)
                if parent < 0:
                    parent = _signed_value(fields[14][0].number & 0xFFFF, 16)
                projected = (fields[21][0].number, parent, fields[1][0].text, fields[0][0].text)
            elif terrain_role == "LiquidType":
                projected = (fields[3][0].number, fields[14][0].number)
            else:
                projected = (fields[0][0].number & 0xFFFFFFFF, fields[1][0].number)
            terrain_direct.append((record_id, projected))
            continue
        # Extra field at the end: grow the row to `parent` and default the
        # appended slot to a signed zero; filled from the parent block below.
        # When instead `parent < len(schema.fields)` the parent is an EXISTING
        # decoded column ("in data block"); its already-decoded value is kept
        # until the parent block overwrites it below.
        if schema.parent >= 0 and schema.parent >= len(schema.fields):
            while len(fields) <= schema.parent:
                fields.append([])
            fields[schema.parent] = [_Value("unsigned" if schema.appended_parent_unsigned else "signed", number=0)]
        ordinal_ids.append(record_id)
        rows[record_id] = fields
        row_text_bytes[record_id] = text_budget.used - text_before_row

    if terrain_role is not None:
        copies = tuple((_u32(data, copy_start + i * 8), _u32(data, copy_start + i * 8 + 4))
                       for i in range(section.copy_table_count))
        return TerrainSources(terrain_role, tuple(terrain_direct), copies,
                              text_budget.used, text_budget.limit)

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
            # Reference DB2FileLoader.cpp:650-712 `FillParentLookup`: when the
            # parent index lies INSIDE the file field count the value is written
            # with the field's own width (FT_BYTE -> u8, FT_SHORT -> u16,
            # FT_INT -> u32); a too-narrow field is `invalid_record_ids`. For the
            # extra-field-at-the-end variant the value is always a u32.
            if schema.parent < len(schema.fields):
                rows[rid][schema.parent][0] = _parent_in_data_value(
                    schema.fields[schema.parent], parent_id
                )
            elif schema.appended_parent_unsigned:
                rows[rid][schema.parent][0] = _Value("unsigned", number=parent_id)
            else:
                rows[rid][schema.parent][0] = _Value(
                    "signed", number=_signed_value(parent_id, 32)
                )

    # DB2FileLoader.cpp:955-977 GetMaxId includes every copy destination,
    # even when that copy will be skipped. AutoProduceData (:394-398) adds
    # one and DB2Store.cpp:101-107 passes this index-table size to
    # AutoProduceRecordCopies, not the number of physical file records.
    max_record_id = max(rows, default=0)
    for index in range(section.copy_table_count):
        new = _u32(data, copy_start + index * 8)
        max_record_id = max(max_record_id, new)
    # Fail closed on the reference's header assertion or u32 size overflow.
    if max_record_id > header.max_id or max_record_id == 0xFFFFFFFF:
        raise OracleError("invalid_record_ids")
    index_table_size = max_record_id + 1

    # AutoProduceRecordCopies (DB2FileLoader.cpp:613-634), in file order.
    for index in range(section.copy_table_count):
        entry = copy_start + index * 8
        new = _u32(data, entry)
        source = _u32(data, entry + 4)
        if (
            source == 0
            or source >= index_table_size
            or new >= index_table_size
            or source not in rows
        ):
            continue
        # The reference writes a destination pointer before reading its
        # source, so self-copies read uninitialized data. Its uint32 inline-id
        # write also corrupts adjacent fields when the reviewed id is narrower.
        # Neither case has defined output we can safely claim to reproduce.
        if source == new or (schema.index >= 0 and schema.fields[schema.index].bits != 32):
            raise OracleError("unsupported_record_variant")
        if new not in rows and (len(rows) + 1) * values_per_row > VALUE_POLICY_LIMIT:
            raise OracleError("record_policy_limit")
        # Charge before constructing the copy. Overwrites consume bytes too;
        # no refund permits repeated replacement to evade the work budget.
        text_budget.charge(row_text_bytes[source])
        fields = [[dataclasses.replace(value) for value in group] for group in rows[source]]
        if schema.index >= 0:
            fields[schema.index][0] = _Value("unsigned", number=new)
        rows[new] = fields
        row_text_bytes[new] = row_text_bytes[source]
    return dict(sorted(rows.items()))


@dataclasses.dataclass(frozen=True)
class TerrainSources:
    """Closed terrain projection; complete validated physical order, no assets IO.

    Fields: Map=(signed WDT, name, directory), Type=(sound, material),
    Material=(flags bits, signed LVF). Copies are (new, source) raw u32 pairs.
    """
    name: str
    direct_records: tuple
    copies: tuple
    decoded_utf8_bytes: int
    text_limit: int


def read_terrain_sources(data, name, *, _text_limit=None):
    """Pinned System.cpp275-389 consumer, separate from AutoProduceData rows."""
    identities = {"Map": (0xD43AFAC3, 26), "LiquidType": (0xD1ECEEC9, 21),
                  "LiquidMaterial": (0x98E5D7AA, 2)}
    if name not in identities:
        raise OracleError("unsupported_table_schema")
    schema = resolve_schema(name, CATALOG_ID)
    header, sections = parse(data)
    layout, count = identities[name]
    if (header.layout_hash != layout or header.field_count != count
            or header.total_field_count != count or schema.parent != -1 or schema.index != -1):
        raise OracleError("unsupported_table_schema")
    return _decode_parsed(data, header, sections, schema, name, _text_limit)


def read_vmap_sources(data, *, _text_limit=None):
    """Pinned vmapexport.cpp421-449 fields/copies; retain zero-WDT rows for graph analysis.

    ParentMapID field13 is a signed 16-bit word. CosmeticParentMapID field14
    replaces it only when negative. Physical source rows and append copies
    remain distinct from generic AutoProduceData and the terrain projection.
    """
    schema = resolve_schema("Map", CATALOG_ID)
    header, sections = parse(data)
    if (header.layout_hash != 0xD43AFAC3 or header.field_count != 26
            or header.total_field_count != 26 or schema.parent != -1 or schema.index != -1):
        raise OracleError("unsupported_table_schema")
    return _decode_parsed(data, header, sections, schema, "VMap", _text_limit)


def _utf8_length(text):
    # Count bytes without allocating a second encoded string.
    return sum(1 if ord(c) < 0x80 else 2 if ord(c) < 0x800 else 3 if ord(c) < 0x10000 else 4
               for c in text)


def materialize_terrain(sources, *, _row_limit=None):
    """Exact source-specific copy operations with bounded accumulation."""
    if sources.name in ("Map", "VMap"):
        limit = 200_000 if _row_limit is None else _row_limit
        rows = list(sources.direct_records)
        if len(rows) > limit:
            raise OracleError("record_policy_limit")
        direct_index = {rid: ordinal for ordinal, (rid, _) in enumerate(rows)}
        budget = _TextBudget(sources.text_limit)
        budget.charge(sources.decoded_utf8_bytes)
        for new, source in sources.copies:
            if source not in direct_index:
                continue
            if len(rows) >= limit:
                raise OracleError("record_policy_limit")
            fields = rows[direct_index[source]][1]
            text_at = 2 if sources.name == "VMap" else 1
            budget.charge(_utf8_length(fields[text_at]) + _utf8_length(fields[text_at + 1]))
            rows.append((new, fields))
        if sources.name == "VMap":
            return rows
        return [(rid, fields) for rid, fields in rows if fields[0] != 0]
    if sources.name not in ("LiquidType", "LiquidMaterial"):
        raise OracleError("unsupported_table_schema")
    limit = 300_000 if _row_limit is None else _row_limit
    rows = {}
    def assign(rid, value):
        if rid not in rows and len(rows) >= limit:
            raise OracleError("record_policy_limit")
        rows[rid] = value
    for rid, fields in sources.direct_records:
        assign(rid, fields)
    for new, source in sources.copies:
        if source not in rows:
            assign(source, (0, 0))
        assign(new, rows[source])
    return dict(sorted(rows.items()))


def decode(data, schema):
    """Decode bounded regular/numeric sparse WDC5 with a 64 MiB UTF-8 budget.

    Direct strings and applicable copies, including overwrites, consume the
    budget without refunds. Results below the budget are unchanged.
    """
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


def dump_file(path, table_name, schema_catalog=None):
    """Canonical local dump; additional schemas require explicit catalog opt-in."""
    schema = resolve_schema(table_name, schema_catalog)
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
    table_hash_ok = schema.table_hash is None or header.table_hash == schema.table_hash
    if (
        not table_hash_ok
        or header.layout_hash not in (schema.layout_hash, *schema.other_layout_hashes)
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
    schema_catalog = None
    index = 0
    while index < len(args):
        if args[index] == "--file" and path is None and index + 1 < len(args):
            path = args[index + 1]
            index += 2
        elif args[index] == "--table" and table is None and index + 1 < len(args):
            table = args[index + 1]
            index += 2
        elif args[index] == "--schema-catalog" and schema_catalog is None and index + 1 < len(args):
            schema_catalog = args[index + 1]
            index += 2
        else:
            sys.stderr.write("invalid_invocation\n")
            return 2
    if path is None or table is None:
        sys.stderr.write("invalid_invocation\n")
        return 2
    try:
        dump = dump_file(path, table, schema_catalog)
    except OracleError as error:
        sys.stderr.write(error.code + "\n")
        return error.exit_code
    sys.stdout.write(dump)
    return 0


if __name__ == "__main__":
    sys.exit(main())
