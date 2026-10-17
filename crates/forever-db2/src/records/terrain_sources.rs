// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original typed source projections and terrain copy replay.
// Modified 2026-10-11: independent VMap Map projection and source-only copy replay.
// Modified 2026-10-11: direct GameObjectDisplayInfo projection, without copy replay.
// Modified 2026-10-17: separately selected complete liquid readers for build70425.
// Modified 2026-10-18: separately selected complete liquid readers for build70450.
//! Pinned map_extractor/System.cpp:275-389 uses Load/GetRecordCopy, not
//! AutoProduceData, as does vmap4_extractor/vmapexport.cpp:411-466. These
//! closed projections validate all physical fields
//! and return ordered direct rows/copies only after complete validation. They
//! do not acquire files, select builds, write MAPS, or establish runtime parity.
//! Only the existing minimal empty and single unencrypted regular profiles are
//! admitted. UTF-8 and cumulative text limits remain explicit policies.
//! Map's selected Name/Directory pointers must be non-null; pointers to an
//! empty NUL-terminated string remain valid.
//! GameObjectDisplayInfo follows gameobject_extract.cpp:88-158: the snapshot
//! preserves all direct rows and raw FDID bits. Acquisition, zero-FDID filtering
//! and model dispatch belong to the caller; copies are validated but not replayed.
use super::{
    CharacterTable, DecodeLimits, DecodedTable, RecordError, StringMode, TEXT_POLICY_LIMIT,
    TextBudget, Value,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectRecord<T> {
    pub id: u32,
    pub fields: T,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordCopy {
    pub new_id: u32,
    pub source_id: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapFields {
    pub wdt_file_data_id: i32,
    pub name: String,
    pub directory: String,
}
/// Map fields consumed by the pinned VMap loader. This is separate from the
/// terrain projection because VMap needs parent relationships before WDT filtering.
#[derive(Clone, PartialEq, Eq)]
pub struct VMapFields {
    pub wdt_file_data_id: i32,
    /// ParentMapID, or CosmeticParentMapID when ParentMapID is negative.
    /// Both source fields are signed 16-bit values; the native GetUInt16 then
    /// int16 conversion preserves exactly these bits, including negative sentinels.
    pub parent_map_id: i16,
    pub name: String,
    pub directory: String,
}
/// Original mmaps_generator/PathGenerator.cpp LoadMap reads direct records
/// only, with last-direct-ID-wins in sMapStore; it never replays copy rows.
/// Keep both signed parents so the effective parent remains independently auditable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationMapFields {
    pub map_type: u8,
    pub instance_type: i8,
    pub parent_map_id: i16,
    pub cosmetic_parent_map_id: i16,
    pub flags: i32,
}
impl NavigationMapFields {
    pub const fn effective_parent(self) -> i16 {
        if self.parent_map_id < 0 {
            self.cosmetic_parent_map_id
        } else {
            self.parent_map_id
        }
    }
}
/// FileDataID is physically signed32, but the pinned consumer uses GetUInt32.
/// Preserve all bits, including zero and negative signed representations.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GameObjectFields {
    pub file_data_id: u32,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LiquidTypeFields {
    pub sound_bank: u8,
    pub material_id: u8,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LiquidMaterialFields {
    pub flags_bits: u32,
    pub lvf: i8,
}
/// Loaded by ExtractMaps startup even though ConvertADT does not consult it.
/// System.cpp uses GetUInt16 on the physically signed16 LiquidTypeID field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LiquidObjectFields {
    pub liquid_type_id: u16,
}

/// An immutable validated snapshot. Duplicate direct IDs and ordered raw copy
/// operations are retained. ID-table repair uses checked MinId + ordinal; no
/// AutoProduceData MaxId+1 index or header-ID range policy is imposed here.
pub struct SourceRecords<T> {
    direct: Vec<DirectRecord<T>>,
    copies: Vec<RecordCopy>,
    text: TextBudget,
}
impl<T> SourceRecords<T> {
    pub fn direct_records(&self) -> &[DirectRecord<T>] {
        &self.direct
    }
    pub fn copies(&self) -> &[RecordCopy] {
        &self.copies
    }
    /// Includes validated text in unselected physical fields. Replay carries
    /// this charge forward; neither discarded fields nor overwritten rows refund it.
    pub const fn decoded_utf8_bytes(&self) -> usize {
        self.text.used
    }
}

impl SourceRecords<GameObjectFields> {
    /// Actual retained vector capacities, in bytes. This closed numeric
    /// projection contains no separately allocated field values or strings.
    /// It excludes the fixed snapshot object and the caller's input buffer.
    pub fn resident_bytes(&self) -> Result<usize, RecordError> {
        self.direct
            .capacity()
            .checked_mul(size_of::<DirectRecord<GameObjectFields>>())
            .and_then(|direct| {
                self.copies
                    .capacity()
                    .checked_mul(size_of::<RecordCopy>())
                    .and_then(|copies| direct.checked_add(copies))
            })
            .ok_or(RecordError::Limit)
    }

    /// Transfer the validated physical rows and uninterpreted copies without
    /// allocating, cloning, applying copies or changing their original order.
    pub fn into_parts(self) -> (Vec<DirectRecord<GameObjectFields>>, Vec<RecordCopy>) {
        (self.direct, self.copies)
    }
}

pub fn read_map(data: &[u8]) -> Result<SourceRecords<MapFields>, RecordError> {
    match read(data, Role::Map, TEXT_POLICY_LIMIT)? {
        Collected::Map(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Validate the complete Map file and retain the VMap loader's physical rows
/// and ordered copy operations. This does not traverse the parent graph.
/// A planner must retain `direct_records().len()` before consuming this snapshot:
/// the native parent lookup, like the copy lookup, indexes direct rows only.
pub fn read_vmap_maps(data: &[u8]) -> Result<SourceRecords<VMapFields>, RecordError> {
    match read(data, Role::VMap, TEXT_POLICY_LIMIT)? {
        Collected::VMap(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Complete closed Map projection for navigation startup. Source strings are
/// validated by the shared decoder but need not be non-null: this consumer only
/// calls numeric getters. Copies remain validated inspection metadata.
pub fn read_navigation_maps(
    data: &[u8],
) -> Result<SourceRecords<NavigationMapFields>, RecordError> {
    match read(data, Role::Navigation, TEXT_POLICY_LIMIT)? {
        Collected::Navigation(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Validate the complete closed GameObjectDisplayInfo profile and collect direct
/// physical records in order, including duplicate IDs/FDIDs and zero FDIDs.
/// The native GetRecordCount/GetRecord loop never replays copies. This type has
/// no materialize operation: the validated copy table is inspection metadata.
/// ID-table repair, whole-file/resource gates and all-field decoding remain the
/// shared source-reader policies. Ignored GeoBox float bits need not be finite.
/// No model classification, filename generation or acquisition occurs here.
pub fn read_gameobject_display_info(
    data: &[u8],
) -> Result<SourceRecords<GameObjectFields>, RecordError> {
    match read(data, Role::GameObject, TEXT_POLICY_LIMIT)? {
        Collected::GameObject(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
pub fn read_liquid_types(data: &[u8]) -> Result<SourceRecords<LiquidTypeFields>, RecordError> {
    match read(data, Role::LiquidType, TEXT_POLICY_LIMIT)? {
        Collected::LiquidType(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
pub fn read_liquid_materials(
    data: &[u8],
) -> Result<SourceRecords<LiquidMaterialFields>, RecordError> {
    match read(data, Role::LiquidMaterial, TEXT_POLICY_LIMIT)? {
        Collected::LiquidMaterial(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Complete LiquidType projection using the separately pinned build70425 catalog.
/// The caller must independently admit the client build; identical wire schemas
/// do not identify a build. Existing complete-source profiles and limits apply.
pub fn read_liquid_types_70425(
    data: &[u8],
) -> Result<SourceRecords<LiquidTypeFields>, RecordError> {
    match read_liquids_70425(data, Role::LiquidType, TEXT_POLICY_LIMIT)? {
        Collected::LiquidType(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Complete LiquidMaterial projection using the separately pinned build70425
/// catalog, including its table-hash gate. This does not admit available sections.
pub fn read_liquid_materials_70425(
    data: &[u8],
) -> Result<SourceRecords<LiquidMaterialFields>, RecordError> {
    match read_liquids_70425(data, Role::LiquidMaterial, TEXT_POLICY_LIMIT)? {
        Collected::LiquidMaterial(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Complete LiquidType projection using the separately pinned build70450 catalog.
/// The caller must independently admit the client build; identical wire schemas
/// do not identify a build. Existing complete-source profiles and limits apply.
pub fn read_liquid_types_70450(
    data: &[u8],
) -> Result<SourceRecords<LiquidTypeFields>, RecordError> {
    match read_liquids_70450(data, Role::LiquidType, TEXT_POLICY_LIMIT)? {
        Collected::LiquidType(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
/// Complete LiquidMaterial projection using the separately pinned build70450
/// catalog, including its table-hash gate. This does not admit available sections.
pub fn read_liquid_materials_70450(
    data: &[u8],
) -> Result<SourceRecords<LiquidMaterialFields>, RecordError> {
    match read_liquids_70450(data, Role::LiquidMaterial, TEXT_POLICY_LIMIT)? {
        Collected::LiquidMaterial(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}
pub fn read_liquid_objects(data: &[u8]) -> Result<SourceRecords<LiquidObjectFields>, RecordError> {
    match read(data, Role::LiquidObject, TEXT_POLICY_LIMIT)? {
        Collected::LiquidObject(rows) => Ok(rows),
        _ => Err(RecordError::Schema),
    }
}

impl SourceRecords<MapFields> {
    /// Source-only ID lookup, append copies, then remove WDT == 0. Duplicate
    /// destinations/self-copies append; copy-created IDs never become sources.
    /// At most 200k retained rows; copied UTF-8 consumes the same 64 MiB budget.
    pub fn materialize(self) -> Result<Vec<DirectRecord<MapFields>>, RecordError> {
        self.materialize_with_limit(200_000)
    }
    pub(super) fn materialize_with_limit(
        mut self,
        limit: usize,
    ) -> Result<Vec<DirectRecord<MapFields>>, RecordError> {
        if self.direct.len() > limit {
            return Err(RecordError::Limit);
        }
        let mut direct_index = BTreeMap::new();
        for (ordinal, row) in self.direct.iter().enumerate() {
            direct_index.insert(row.id, ordinal);
        }
        for copy in self.copies {
            if let Some(&ordinal) = direct_index.get(&copy.source_id) {
                if self.direct.len() >= limit {
                    return Err(RecordError::Limit);
                }
                let source = &self.direct[ordinal].fields;
                let text = source
                    .name
                    .len()
                    .checked_add(source.directory.len())
                    .ok_or(RecordError::Limit)?;
                self.text.charge(text)?;
                let fields = source.clone();
                self.direct.try_reserve(1).map_err(|_| RecordError::Limit)?;
                self.direct.push(DirectRecord {
                    id: copy.new_id,
                    fields,
                });
            }
        }
        self.direct.retain(|row| row.fields.wdt_file_data_id != 0);
        Ok(self.direct)
    }
}
impl SourceRecords<VMapFields> {
    /// Replay vmapexport.cpp:421-449: direct duplicate IDs select the last
    /// physical source; copies append and never become sources. Missing sources
    /// are omitted; zero, self, duplicate and out-of-header-range destinations
    /// are retained. All WDT == 0 rows remain available to the later parent
    /// graph pass, which precedes the native WDT filter at lines 452-466.
    ///
    /// No graph traversal, graph repair or WDT filtering happens here. At most
    /// 200k rows may be retained; copied selected strings consume the same
    /// cumulative 64 MiB text budget as the complete physical decode.
    /// The original direct rows remain the unchanged prefix, followed by copies;
    /// callers must preserve that prefix length for direct-only parent lookup.
    pub fn materialize(self) -> Result<Vec<DirectRecord<VMapFields>>, RecordError> {
        self.materialize_with_limit(200_000)
    }
    pub(super) fn materialize_with_limit(
        mut self,
        limit: usize,
    ) -> Result<Vec<DirectRecord<VMapFields>>, RecordError> {
        if self.direct.len() > limit {
            return Err(RecordError::Limit);
        }
        let mut direct_index = BTreeMap::new();
        for (ordinal, row) in self.direct.iter().enumerate() {
            direct_index.insert(row.id, ordinal);
        }
        for copy in self.copies {
            if let Some(&ordinal) = direct_index.get(&copy.source_id) {
                if self.direct.len() >= limit {
                    return Err(RecordError::Limit);
                }
                let source = &self.direct[ordinal].fields;
                let text = source
                    .name
                    .len()
                    .checked_add(source.directory.len())
                    .ok_or(RecordError::Limit)?;
                self.text.charge(text)?;
                let fields = source.clone();
                self.direct.try_reserve(1).map_err(|_| RecordError::Limit)?;
                self.direct.push(DirectRecord {
                    id: copy.new_id,
                    fields,
                });
            }
        }
        Ok(self.direct)
    }
}
impl SourceRecords<LiquidTypeFields> {
    pub fn materialize(self) -> Result<BTreeMap<u32, LiquidTypeFields>, RecordError> {
        materialize_liquid(self, 300_000)
    }
}
impl SourceRecords<LiquidMaterialFields> {
    pub fn materialize(self) -> Result<BTreeMap<u32, LiquidMaterialFields>, RecordError> {
        materialize_liquid(self, 300_000)
    }
}
impl SourceRecords<LiquidObjectFields> {
    pub fn materialize(self) -> Result<BTreeMap<u32, LiquidObjectFields>, RecordError> {
        materialize_liquid(self, 300_000)
    }
}
pub(super) fn materialize_liquid<T: Copy + Default>(
    source: SourceRecords<T>,
    limit: usize,
) -> Result<BTreeMap<u32, T>, RecordError> {
    let mut rows = BTreeMap::new();
    for row in source.direct {
        insert_bounded(&mut rows, row.id, row.fields, limit)?;
    }
    // C++ operator[] on the RHS inserts a missing source before assigning the
    // LHS. Both new keys count, including zero; self-copy inserts at most once.
    for copy in source.copies {
        let value = match rows.get(&copy.source_id) {
            Some(value) => *value,
            None => {
                let value = T::default();
                insert_bounded(&mut rows, copy.source_id, value, limit)?;
                value
            }
        };
        insert_bounded(&mut rows, copy.new_id, value, limit)?;
    }
    Ok(rows)
}
fn insert_bounded<T>(
    rows: &mut BTreeMap<u32, T>,
    id: u32,
    value: T,
    limit: usize,
) -> Result<(), RecordError> {
    if !rows.contains_key(&id) && rows.len() >= limit {
        return Err(RecordError::Limit);
    }
    rows.insert(id, value);
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum Role {
    Map,
    VMap,
    Navigation,
    GameObject,
    LiquidType,
    LiquidMaterial,
    LiquidObject,
}
pub(super) enum Collector {
    Map(Vec<DirectRecord<MapFields>>),
    VMap(Vec<DirectRecord<VMapFields>>),
    Navigation(Vec<DirectRecord<NavigationMapFields>>),
    GameObject(Vec<DirectRecord<GameObjectFields>>),
    LiquidType(Vec<DirectRecord<LiquidTypeFields>>),
    LiquidMaterial(Vec<DirectRecord<LiquidMaterialFields>>),
    LiquidObject(Vec<DirectRecord<LiquidObjectFields>>),
}
pub(super) enum Collected {
    Map(SourceRecords<MapFields>),
    VMap(SourceRecords<VMapFields>),
    Navigation(SourceRecords<NavigationMapFields>),
    GameObject(SourceRecords<GameObjectFields>),
    LiquidType(SourceRecords<LiquidTypeFields>),
    LiquidMaterial(SourceRecords<LiquidMaterialFields>),
    LiquidObject(SourceRecords<LiquidObjectFields>),
}
pub(super) fn read(data: &[u8], role: Role, text_limit: usize) -> Result<Collected, RecordError> {
    let (table, collector) = match role {
        Role::Map => (CharacterTable::Map, Collector::Map(Vec::new())),
        Role::VMap => (CharacterTable::Map, Collector::VMap(Vec::new())),
        Role::Navigation => (CharacterTable::Map, Collector::Navigation(Vec::new())),
        Role::GameObject => (
            CharacterTable::from_catalog("GameObjectDisplayInfo")
                .map_err(|_| RecordError::Schema)?
                .ok_or(RecordError::Schema)?,
            Collector::GameObject(Vec::new()),
        ),
        Role::LiquidType => (
            CharacterTable::from_catalog("LiquidType")
                .map_err(|_| RecordError::Schema)?
                .ok_or(RecordError::Schema)?,
            Collector::LiquidType(Vec::new()),
        ),
        Role::LiquidMaterial => (
            CharacterTable::from_catalog("LiquidMaterial")
                .map_err(|_| RecordError::Schema)?
                .ok_or(RecordError::Schema)?,
            Collector::LiquidMaterial(Vec::new()),
        ),
        Role::LiquidObject => (
            CharacterTable::from_catalog("LiquidObject")
                .map_err(|_| RecordError::Schema)?
                .ok_or(RecordError::Schema)?,
            Collector::LiquidObject(Vec::new()),
        ),
    };
    read_selected(data, table, collector, text_limit)
}
pub(super) fn read_liquids_70425(
    data: &[u8],
    role: Role,
    text_limit: usize,
) -> Result<Collected, RecordError> {
    let (name, collector) = match role {
        Role::LiquidType => ("LiquidType", Collector::LiquidType(Vec::new())),
        Role::LiquidMaterial => ("LiquidMaterial", Collector::LiquidMaterial(Vec::new())),
        _ => return Err(RecordError::Schema),
    };
    let table = CharacterTable::from_catalog_id(super::catalog::WOWDBDEFS_70425_CATALOG_ID, name)
        .map_err(|_| RecordError::Schema)?
        .ok_or(RecordError::Schema)?;
    read_selected(data, table, collector, text_limit)
}
pub(super) fn read_liquids_70450(
    data: &[u8],
    role: Role,
    text_limit: usize,
) -> Result<Collected, RecordError> {
    let (name, collector) = match role {
        Role::LiquidType => ("LiquidType", Collector::LiquidType(Vec::new())),
        Role::LiquidMaterial => ("LiquidMaterial", Collector::LiquidMaterial(Vec::new())),
        _ => return Err(RecordError::Schema),
    };
    let table = CharacterTable::from_catalog_id(super::catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .map_err(|_| RecordError::Schema)?
        .ok_or(RecordError::Schema)?;
    read_selected(data, table, collector, text_limit)
}
fn read_selected(
    data: &[u8],
    table: CharacterTable,
    collector: Collector,
    text_limit: usize,
) -> Result<Collected, RecordError> {
    let result = DecodedTable::scan_regular_or_logical(
        data,
        table,
        None,
        text_limit,
        DecodeLimits::DEFAULT,
        StringMode::StrictUtf8,
        Some(collector),
    )?;
    result.terrain.ok_or(RecordError::Schema)
}
impl Collector {
    pub(super) fn requires_non_null_string(&self, field: usize) -> bool {
        // DB2FileLoader.cpp:798-803 returns nullptr for a zero offset.
        // System.cpp:294-295 and vmapexport.cpp:431-432 assign Name/Directory to std::string,
        // before filtering zero WDT IDs. Reject that undefined native case;
        // generic AutoProduceData decoding still normalizes null to empty.
        matches!(self, Self::Map(_) | Self::VMap(_)) && matches!(field, 0 | 1)
    }
    pub(super) fn push(&mut self, id: u32, mut fields: Vec<Vec<Value>>) -> Result<(), RecordError> {
        match self {
            Self::Map(rows) => {
                let wdt_file_data_id = signed(&fields, 21)?;
                let directory = text(&mut fields, 0)?;
                let name = text(&mut fields, 1)?;
                push(
                    rows,
                    DirectRecord {
                        id,
                        fields: MapFields {
                            wdt_file_data_id,
                            name,
                            directory,
                        },
                    },
                )
            }
            Self::VMap(rows) => {
                let wdt_file_data_id = signed(&fields, 21)?;
                // ExtractorDB2LoadInfo.h MapLoadInfo physical fields 13/14,
                // distinct from the flattened DB2Record lookup indices 14/15.
                let parent = short(&fields, 13)?;
                let parent_map_id = if parent < 0 {
                    short(&fields, 14)?
                } else {
                    parent
                };
                let directory = text(&mut fields, 0)?;
                let name = text(&mut fields, 1)?;
                push(
                    rows,
                    DirectRecord {
                        id,
                        fields: VMapFields {
                            wdt_file_data_id,
                            parent_map_id,
                            name,
                            directory,
                        },
                    },
                )
            }
            Self::Navigation(rows) => {
                let flags = match fields.get(25).map(Vec::as_slice) {
                    Some([Value::Signed(first), Value::Signed(_), Value::Signed(_)]) => *first,
                    _ => return Err(RecordError::Schema),
                };
                push(
                    rows,
                    DirectRecord {
                        id,
                        fields: NavigationMapFields {
                            map_type: byte(&fields, 7)?,
                            instance_type: i8::try_from(signed(&fields, 8)?)
                                .map_err(|_| RecordError::Schema)?,
                            parent_map_id: short(&fields, 13)?,
                            cosmetic_parent_map_id: short(&fields, 14)?,
                            flags,
                        },
                    },
                )
            }
            Self::GameObject(rows) => push(
                rows,
                DirectRecord {
                    id,
                    fields: GameObjectFields {
                        file_data_id: signed(&fields, 1)? as u32,
                    },
                },
            ),
            Self::LiquidType(rows) => push(
                rows,
                DirectRecord {
                    id,
                    fields: LiquidTypeFields {
                        sound_bank: byte(&fields, 3)?,
                        material_id: byte(&fields, 14)?,
                    },
                },
            ),
            Self::LiquidMaterial(rows) => push(
                rows,
                DirectRecord {
                    id,
                    fields: LiquidMaterialFields {
                        flags_bits: signed(&fields, 0)? as u32,
                        lvf: i8::try_from(signed(&fields, 1)?).map_err(|_| RecordError::Schema)?,
                    },
                },
            ),
            Self::LiquidObject(rows) => push(
                rows,
                DirectRecord {
                    id,
                    fields: LiquidObjectFields {
                        liquid_type_id: short(&fields, 2)? as u16,
                    },
                },
            ),
        }
    }
    pub(super) fn finish(self, copies: Vec<RecordCopy>, text: TextBudget) -> Collected {
        match self {
            Self::Map(direct) => Collected::Map(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::VMap(direct) => Collected::VMap(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::Navigation(direct) => Collected::Navigation(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::GameObject(direct) => Collected::GameObject(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::LiquidType(direct) => Collected::LiquidType(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::LiquidMaterial(direct) => Collected::LiquidMaterial(SourceRecords {
                direct,
                copies,
                text,
            }),
            Self::LiquidObject(direct) => Collected::LiquidObject(SourceRecords {
                direct,
                copies,
                text,
            }),
        }
    }
}
fn push<T>(rows: &mut Vec<T>, row: T) -> Result<(), RecordError> {
    rows.try_reserve(1).map_err(|_| RecordError::Limit)?;
    rows.push(row);
    Ok(())
}
fn signed(fields: &[Vec<Value>], field: usize) -> Result<i32, RecordError> {
    match fields.get(field).map(Vec::as_slice) {
        Some([Value::Signed(value)]) => Ok(*value),
        _ => Err(RecordError::Schema),
    }
}
fn byte(fields: &[Vec<Value>], field: usize) -> Result<u8, RecordError> {
    match fields.get(field).map(Vec::as_slice) {
        Some([Value::Unsigned(value)]) => u8::try_from(*value).map_err(|_| RecordError::Schema),
        _ => Err(RecordError::Schema),
    }
}
fn short(fields: &[Vec<Value>], field: usize) -> Result<i16, RecordError> {
    i16::try_from(signed(fields, field)?).map_err(|_| RecordError::Schema)
}
fn text(fields: &mut [Vec<Value>], field: usize) -> Result<String, RecordError> {
    let values = fields.get_mut(field).ok_or(RecordError::Schema)?;
    if values.len() != 1 {
        return Err(RecordError::Schema);
    }
    match values.pop() {
        Some(Value::Text(value)) => Ok(value),
        _ => Err(RecordError::Schema),
    }
}
