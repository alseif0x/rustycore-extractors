// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic synthetic 70450 esES source with WDT, M2, and material-filtered-WMO omissions.
use forever_casc::{
    collision::plan::{MapSelection, Maps},
    package::PartialEncryptedSection,
};
use forever_db2::{
    availability::{self, Source, Unavailable},
    catalog::{self, FieldType},
};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
};

#[path = "../../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../../forever-casc/src/blte/md5.rs"]
mod md5;

pub const EMPTY_MAP: u32 = 1;
pub const POPULATED_MAP: u32 = 2;
pub const WDT_ZERO_CHILD: u32 = 3;
pub const ROOT_ABSENT_PARENT: u32 = 4;
pub const PRESENT_CHILD: u32 = 5;
pub const ROOT_ABSENT_OTHER: u32 = 6;
pub const INACTIVE_MAP: u32 = 7;
pub const COPIED_INACTIVE_MAP: u32 = 8;
pub const HIDDEN_MAP: u32 = 99;
pub const EMPTY_WDT: u32 = 100;
pub const POPULATED_WDT: u32 = 101;
pub const ROOT_ABSENT_PARENT_WDT: u32 = 104;
pub const CHILD_WDT: u32 = 105;
pub const ROOT_ABSENT_OTHER_WDT: u32 = 106;
pub const POPULATED_ADT: u32 = 201;
pub const CHILD_ADT: u32 = 202;
pub const MODEL: u32 = 300;
pub const USEFUL_REFERENCE_MODEL: u32 = 301;
pub const MISSING_M2: u32 = 970793;
pub const WMO_ROOT: u32 = 400;
pub const WMO_GROUP: u32 = 401;
pub const MISSING_WMO: u32 = 462694;
pub const MISSING_WMO_GROUP: u32 = 462696;
pub const UNSELECTED_WMO: u32 = 462695;
pub const UNSELECTED_WMO_GROUP: u32 = 462700;
pub const EMPTY_WMO: u32 = 462697;
pub const EMPTY_WMO_GROUP: u32 = 462698;
pub const MATERIAL_FILTERED_WMO: u32 = 462695;
pub const MATERIAL_FILTERED_WMO_MIXED: u32 = 462701;
pub const MATERIAL_FILTERED_WMO_GROUP: u32 = 462702;
pub const MATERIAL_FILTERED_WMO_SKIPPED_GROUP: u32 = 462703;
pub const LOGICAL_EMPTY_WMO_ID: u32 = 462699;
pub const LOGICAL_EMPTY_WMO: &[u8] = b"World\\LogicalEmpty.wmo";
pub const ACTIVE_MAPS: usize = 5;
pub const PRESENT_WDT_FILES: usize = 3;
pub const ROOT_ABSENT_MAPS: usize = 2;
pub const MATERIALIZED_ROWS: u64 = 8;
const LOOKUP: u64 = 0x1122_3344_5566_7788;

pub fn model_name(id: u32) -> String {
    format!("FILE{id:08X}.xxx")
}

pub struct PartialMap {
    pub bytes: Vec<u8>,
    pub hidden_start: usize,
    pub declaration: PartialEncryptedSection,
}

struct Bytes<'a>(&'a [u8]);
impl Source for Bytes<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(&self, offset: u64, size: usize) -> Result<&[u8], Unavailable> {
        let start = usize::try_from(offset).map_err(|_| Unavailable)?;
        self.0
            .get(start..start.checked_add(size).ok_or(Unavailable)?)
            .ok_or(Unavailable)
    }
}

fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn field_width(kind: FieldType) -> usize {
    match kind {
        FieldType::String | FieldType::NonLocalizedString | FieldType::Int | FieldType::Float => 32,
        FieldType::Byte => 8,
        FieldType::Short => 16,
        FieldType::Long => 64,
    }
}

fn write_value(bytes: &mut [u8], at: usize, size: usize, value: i64) {
    bytes[at..at + size].copy_from_slice(&value.to_le_bytes()[..size]);
}

fn hex16(value: [u8; 16]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn table_id(name: &str) -> u32 {
    catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id
}

fn complete_db2(name: &str, rows: &[(u32, u32)]) -> Vec<u8> {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata();
    let fields = &metadata.fields[..metadata.file_field_count];
    let mut offsets = Vec::new();
    let mut bit_offset = 0usize;
    for field in fields {
        offsets.push(bit_offset);
        bit_offset += field_width(field.kind) * usize::from(field.array_size);
    }
    let record_size = bit_offset.div_ceil(8).max(8);
    let mut records = Vec::new();
    for (row_index, (row_id, value)) in rows.iter().enumerate() {
        let mut record = vec![0; record_size];
        for (field_index, field) in fields.iter().enumerate() {
            if matches!(
                field.kind,
                FieldType::String | FieldType::NonLocalizedString
            ) {
                for array_index in 0..usize::from(field.array_size) {
                    let at = offsets[field_index] / 8 + array_index * 4;
                    let relative = (rows.len() - row_index) * record_size - at;
                    put32(&mut record, at, relative as u32);
                }
            }
        }
        if name == "LiquidType" {
            let at = offsets[14] / 8;
            write_value(
                &mut record,
                at,
                field_width(fields[14].kind) / 8,
                i64::from(*value),
            );
        }
        if metadata.index >= 0 {
            let index = usize::try_from(metadata.index).unwrap();
            put32(&mut record, offsets[index] / 8, *row_id);
        }
        records.extend_from_slice(&record);
    }
    const HEADER: usize = 204;
    const SECTION: usize = 40;
    let column_meta_at = HEADER + SECTION + fields.len() * 4;
    let record_at = column_meta_at + fields.len() * 24;
    let strings = b"synthetic\0";
    let id_bytes = if metadata.index < 0 {
        rows.len() * 4
    } else {
        0
    };
    let mut bytes = vec![0; record_at];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, fields.len() as u32),
        (144, record_size as u32),
        (148, strings.len() as u32),
        (152, metadata.table_hash.unwrap_or(0)),
        (156, metadata.layout_hash),
        (160, 0),
        (164, u32::MAX),
        (176, metadata.field_count as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    put32(&mut bytes, HEADER + 8, record_at as u32);
    put32(&mut bytes, HEADER + 12, rows.len() as u32);
    put32(&mut bytes, HEADER + 16, strings.len() as u32);
    put32(&mut bytes, HEADER + 24, id_bytes as u32);
    for (field_index, field) in fields.iter().enumerate() {
        let at = column_meta_at + field_index * 24;
        put16(&mut bytes, at, offsets[field_index] as u16);
        put32(&mut bytes, at + 16, field_width(field.kind) as u32);
        put32(&mut bytes, at + 20, u32::from(field.array_size));
    }
    bytes.extend_from_slice(&records);
    bytes.extend_from_slice(strings);
    if metadata.index < 0 {
        for (row_id, _) in rows {
            bytes.extend_from_slice(&row_id.to_le_bytes());
        }
    }
    bytes
}

pub fn partial_map() -> PartialMap {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
        .unwrap()
        .unwrap()
        .metadata();
    let fields = &metadata.fields[..metadata.file_field_count];
    let mut offsets = Vec::new();
    let mut bits = 0usize;
    for field in fields {
        offsets.push(bits);
        bits += field_width(field.kind) * usize::from(field.array_size);
    }
    let record_size = bits.div_ceil(8).max(8);
    let rows = [
        (EMPTY_MAP, EMPTY_WDT, -1_i16, -1_i16),
        (POPULATED_MAP, POPULATED_WDT, EMPTY_MAP as i16, -1_i16),
        (WDT_ZERO_CHILD, 0, POPULATED_MAP as i16, -1_i16),
        (
            ROOT_ABSENT_PARENT,
            ROOT_ABSENT_PARENT_WDT,
            POPULATED_MAP as i16,
            -1_i16,
        ),
        (PRESENT_CHILD, CHILD_WDT, ROOT_ABSENT_PARENT as i16, -1_i16),
        (ROOT_ABSENT_OTHER, ROOT_ABSENT_OTHER_WDT, -1_i16, -1_i16),
        (INACTIVE_MAP, 0, -1_i16, -1_i16),
    ];
    let total_records = rows.len() + 1;
    let hidden_size = record_size + 1 + 4;

    const HEADER: usize = 204;
    const SECTION: usize = 40;
    let column_meta_at = HEADER + 2 * SECTION + fields.len() * 4;
    let metadata_end = column_meta_at + fields.len() * 24;
    let mut bytes = vec![0; metadata_end];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, total_records as u32),
        (140, fields.len() as u32),
        (144, record_size as u32),
        (148, 2),
        (152, metadata.table_hash.unwrap_or(0)),
        (156, metadata.layout_hash),
        (160, 0),
        (164, HIDDEN_MAP),
        (176, metadata.field_count as u32),
        (188, (fields.len() * 24) as u32),
        (200, 2),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    for (field_index, field) in fields.iter().enumerate() {
        let at = column_meta_at + field_index * 24;
        put16(&mut bytes, at, offsets[field_index] as u16);
        put32(&mut bytes, at + 16, field_width(field.kind) as u32);
        put32(&mut bytes, at + 20, u32::from(field.array_size));
    }
    bytes[HEADER + 40..HEADER + 48].copy_from_slice(&LOOKUP.to_le_bytes());

    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&HIDDEN_MAP.to_le_bytes());
    let data_start = bytes.len();
    put32(&mut bytes, HEADER + 8, data_start as u32);
    put32(&mut bytes, HEADER + 12, rows.len() as u32);
    put32(&mut bytes, HEADER + 16, 1);
    put32(&mut bytes, HEADER + 24, (rows.len() * 4) as u32);
    put32(&mut bytes, HEADER + 36, 1);

    for (ordinal, (_, wdt, parent, cosmetic_parent)) in rows.iter().enumerate() {
        let mut record = vec![0; record_size];
        for (field_index, field) in fields.iter().enumerate() {
            if matches!(
                field.kind,
                FieldType::String | FieldType::NonLocalizedString
            ) {
                for array_index in 0..usize::from(field.array_size) {
                    let at = offsets[field_index] / 8 + array_index * 4;
                    let relative = (total_records - ordinal) * record_size - at;
                    put32(&mut record, at, relative as u32);
                }
            }
        }
        for (field, value) in [
            (13, i64::from(*parent)),
            (14, i64::from(*cosmetic_parent)),
            (21, i64::from(*wdt)),
        ] {
            let at = offsets[field] / 8;
            write_value(&mut record, at, field_width(fields[field].kind) / 8, value);
        }
        bytes.extend_from_slice(&record);
    }
    bytes.push(0);
    for (id, _, _, _) in rows {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes.extend_from_slice(&COPIED_INACTIVE_MAP.to_le_bytes());
    bytes.extend_from_slice(&INACTIVE_MAP.to_le_bytes());

    let hidden_start = bytes.len();
    put32(&mut bytes, HEADER + 40 + 8, hidden_start as u32);
    put32(&mut bytes, HEADER + 40 + 12, 1);
    put32(&mut bytes, HEADER + 40 + 16, 1);
    put32(&mut bytes, HEADER + 40 + 24, 4);
    bytes.resize(hidden_start + hidden_size, 0);
    assert_eq!(
        hidden_start,
        data_start + rows.len() * record_size + 1 + rows.len() * 4 + 8
    );

    let available = availability::read_vmap_maps_70450(&Bytes(&bytes)).unwrap();
    assert_eq!(available.direct_records().len(), rows.len());
    assert_eq!(available.copies().len(), 1);
    assert_eq!(available.sections().len(), 2);
    assert_eq!(available.sections()[0].declared_records, 7);
    assert_eq!(available.sections()[0].declared_copies, 1);
    assert_eq!(available.sections()[1].declared_records, 1);
    let materialized = available.materialize().unwrap();
    assert_eq!(
        materialized
            .rows()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6, 7, 8]
    );
    for id in [WDT_ZERO_CHILD, INACTIVE_MAP, COPIED_INACTIVE_MAP] {
        assert_eq!(
            materialized
                .rows()
                .iter()
                .find(|row| row.id == id)
                .unwrap()
                .fields
                .wdt_file_data_id,
            0
        );
    }
    let maps = Maps::from_materialized_selection(
        materialized.rows(),
        materialized.direct_records().len(),
        MapSelection::All,
        Default::default(),
    )
    .unwrap();
    assert_eq!(maps.selection(), MapSelection::All);
    assert_eq!(maps.stats().rows, 8);
    assert_eq!(maps.stats().active_maps, ACTIVE_MAPS);
    assert_eq!(
        maps.selected_ids().collect::<Vec<_>>(),
        [
            EMPTY_MAP,
            POPULATED_MAP,
            ROOT_ABSENT_PARENT,
            PRESENT_CHILD,
            ROOT_ABSENT_OTHER
        ]
    );
    assert!(maps.get(EMPTY_MAP).unwrap().is_parent());
    assert!(maps.get(POPULATED_MAP).unwrap().is_parent());
    assert!(maps.get(ROOT_ABSENT_PARENT).unwrap().is_parent());
    assert!(!maps.get(PRESENT_CHILD).unwrap().is_parent());
    assert!(!maps.get(ROOT_ABSENT_OTHER).unwrap().is_parent());
    assert_eq!(maps.get(PRESENT_CHILD).unwrap().depth(), 3);
    for id in [WDT_ZERO_CHILD, INACTIVE_MAP, COPIED_INACTIVE_MAP] {
        assert!(!maps.contains_selected(id));
    }
    assert!(maps.get(HIDDEN_MAP).is_none());

    PartialMap {
        bytes,
        hidden_start,
        declaration: PartialEncryptedSection {
            lookup: format!("{LOOKUP:016x}"),
            frame_ordinal: 1,
            decoded_size: hidden_size as u64,
        },
    }
}

pub fn assert_sealed_map(bytes: &[u8]) {
    let expected = partial_map();
    assert_eq!(bytes, expected.bytes);
    assert!(bytes[expected.hidden_start..].iter().all(|byte| *byte == 0));
}

pub fn assert_partial_manifest(text: &str) {
    let declaration = partial_map().declaration;
    let table = format!(
        "\"partial_encrypted_tables\":[{{\"path\":\"dbc/esES/Map.db2\",\"sections\":[{{\"lookup\":\"{}\",\"frame_ordinal\":1,\"decoded_size\":{}}}]}}]",
        declaration.lookup, declaration.decoded_size
    );
    assert!(text.contains(&table), "{text}");
    assert!(!text.contains("\"partial_encrypted_sections\":"));
    assert_eq!(
        text.matches("\"path\":\"dbc/esES/Map.db2\",\"sections\":")
            .count(),
        1
    );
    assert_eq!(
        text.matches(&format!("\"lookup\":\"{}\"", declaration.lookup))
            .count(),
        1
    );
}

pub fn set_parent(bytes: &mut [u8], row_index: usize, parent: i16) {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
        .unwrap()
        .unwrap()
        .metadata();
    let fields = &metadata.fields[..metadata.file_field_count];
    let offset = fields[..13]
        .iter()
        .map(|field| field_width(field.kind) * usize::from(field.array_size))
        .sum::<usize>()
        / 8;
    let data_start = u32::from_le_bytes(bytes[212..216].try_into().unwrap()) as usize;
    let record_size = u32::from_le_bytes(bytes[144..148].try_into().unwrap()) as usize;
    let at = data_start + row_index * record_size + offset;
    write_value(
        bytes,
        at,
        field_width(fields[13].kind) / 8,
        i64::from(parent),
    );
}

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn collision_wdt(adt: Option<u32>) -> Vec<u8> {
    let mut mphd = vec![0; 32];
    put32(&mut mphd, 0, 0x200);
    let mut main = vec![0; 64 * 64 * 8];
    let mut maid = vec![0; 64 * 64 * 32];
    if let Some(adt) = adt {
        put32(&mut main, 0, 1);
        put32(&mut maid, 4, adt);
    }
    let mut bytes = chunk(b"MPHD", &mphd);
    bytes.extend_from_slice(&chunk(b"MAIN", &main));
    bytes.extend_from_slice(&chunk(b"MAID", &maid));
    bytes
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum AdtMode {
    DoodadOmission,
    #[default]
    VacatedWmo,
    DirectM2Absent,
    MissingWmo,
    MissingWmoGroup,
    LogicalM2Name,
    EmptyWmo,
    EmptyWmoOnly,
    NoOmissions,
    EmptyWmoGeometry,
    EmptyWmoMissingGroup,
    EmptyWmoMissingSets,
    EmptyWmoUsefulBindings,
    EmptyWmoLogicalOrigin,
    VacatedWmoOnly,
    UnusedMissingDoodad,
    UnselectedWmoOnly,
    UnselectedWmoMixed,
    UnselectedWmoSelectedGeometry,
    UnselectedWmoUsefulBindings,
    UnselectedWmoLocalMissingM2,
    UnselectedWmoFilteredGeometry,
    UnselectedWmoUsefulReference,
    UnselectedWmoLogicalEquivalent,
    UnselectedWmoMissingSets,
    UnselectedWmoLogicalOrigin,
    MaterialFilteredWmoOnly,
    MaterialFilteredWmoMixed,
    MaterialFilteredWmoSelectedGeometry,
    MaterialFilteredWmoKeptSurface,
    MaterialFilteredWmoUsefulBindings,
    MaterialFilteredWmoLocalMissingM2,
    MaterialFilteredWmoUsefulReference,
    MaterialFilteredWmoMissingSets,
    MaterialFilteredWmoLogicalOrigin,
    MaterialFilteredWmoLiquid,
    MaterialFilteredWmoSkippedGroup,
}

fn mddf(id: u32, unique: u32, fdid: bool) -> [u8; 36] {
    let mut row = [0; 36];
    put32(&mut row, 0, id);
    put32(&mut row, 4, unique);
    row[32..34].copy_from_slice(&1024_u16.to_le_bytes());
    if fdid {
        row[34..36].copy_from_slice(&0x40_u16.to_le_bytes());
    }
    row
}

fn modf(id: u32, unique: u32) -> [u8; 64] {
    let mut row = [0; 64];
    put32(&mut row, 0, id);
    put32(&mut row, 4, unique);
    row[56..58].copy_from_slice(&8_u16.to_le_bytes());
    row
}

fn collision_adt(unique: u32, mode: AdtMode) -> Vec<u8> {
    let mut bytes = Vec::new();
    if mode == AdtMode::DirectM2Absent {
        bytes.extend_from_slice(&chunk(b"MDDF", &mddf(MISSING_M2, unique + 100, true)));
    }
    if matches!(mode, AdtMode::EmptyWmo | AdtMode::EmptyWmoOnly) {
        let id = if mode == AdtMode::EmptyWmoOnly {
            MISSING_WMO
        } else {
            EMPTY_WMO
        };
        bytes.extend_from_slice(&chunk(b"MODF", &modf(id, unique)));
        return bytes;
    }
    if matches!(
        mode,
        AdtMode::UnselectedWmoLogicalOrigin | AdtMode::MaterialFilteredWmoLogicalOrigin
    ) {
        bytes.extend_from_slice(&chunk(b"MWMO", LOGICAL_EMPTY_WMO));
        let mut logical = modf(0, unique + 2);
        logical[56..58].fill(0);
        logical[60..62].fill(0);
        bytes.extend_from_slice(&chunk(b"MODF", &logical));
        return bytes;
    }

    bytes.extend_from_slice(&chunk(b"MODF", &modf(WMO_ROOT, unique)));
    if matches!(
        mode,
        AdtMode::VacatedWmo | AdtMode::UnselectedWmoMixed | AdtMode::MaterialFilteredWmoMixed
    ) {
        bytes.extend_from_slice(&chunk(b"MODF", &modf(MISSING_WMO, unique + 1)));
    }
    if matches!(
        mode,
        AdtMode::UnselectedWmoOnly
            | AdtMode::UnselectedWmoMixed
            | AdtMode::UnselectedWmoSelectedGeometry
            | AdtMode::UnselectedWmoUsefulBindings
            | AdtMode::UnselectedWmoLocalMissingM2
            | AdtMode::UnselectedWmoFilteredGeometry
            | AdtMode::UnselectedWmoUsefulReference
            | AdtMode::UnselectedWmoLogicalEquivalent
            | AdtMode::UnselectedWmoMissingSets
            | AdtMode::MaterialFilteredWmoMixed
    ) {
        bytes.extend_from_slice(&chunk(b"MODF", &modf(UNSELECTED_WMO, unique + 2)));
    }
    if matches!(
        mode,
        AdtMode::VacatedWmo
            | AdtMode::UnselectedWmoMixed
            | AdtMode::MaterialFilteredWmoMixed
            | AdtMode::EmptyWmoGeometry
            | AdtMode::EmptyWmoMissingGroup
            | AdtMode::EmptyWmoMissingSets
            | AdtMode::EmptyWmoUsefulBindings
    ) {
        bytes.extend_from_slice(&chunk(b"MODF", &modf(EMPTY_WMO, unique + 3)));
    }
    if matches!(
        mode,
        AdtMode::MaterialFilteredWmoOnly
            | AdtMode::MaterialFilteredWmoMixed
            | AdtMode::MaterialFilteredWmoSelectedGeometry
            | AdtMode::MaterialFilteredWmoKeptSurface
            | AdtMode::MaterialFilteredWmoUsefulBindings
            | AdtMode::MaterialFilteredWmoLocalMissingM2
            | AdtMode::MaterialFilteredWmoUsefulReference
            | AdtMode::MaterialFilteredWmoMissingSets
            | AdtMode::MaterialFilteredWmoLiquid
            | AdtMode::MaterialFilteredWmoSkippedGroup
    ) {
        let material_wmo = if mode == AdtMode::MaterialFilteredWmoMixed {
            MATERIAL_FILTERED_WMO_MIXED
        } else {
            MATERIAL_FILTERED_WMO
        };
        bytes.extend_from_slice(&chunk(b"MODF", &modf(material_wmo, unique + 4)));
    }
    if mode == AdtMode::EmptyWmoLogicalOrigin {
        bytes.extend_from_slice(&chunk(b"MWMO", LOGICAL_EMPTY_WMO));
        let mut logical = modf(0, unique + 3);
        logical[56..58].fill(0);
        logical[60..62].fill(0);
        bytes.extend_from_slice(&chunk(b"MODF", &logical));
    }
    bytes
}

fn collision_m2() -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    for (at, value) in [(216, 3), (220, 304), (224, 3), (228, 310)] {
        put32(&mut bytes, at, value);
    }
    bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

fn wmo_root(mode: AdtMode) -> Vec<u8> {
    if matches!(
        mode,
        AdtMode::UnselectedWmoOnly
            | AdtMode::UnselectedWmoSelectedGeometry
            | AdtMode::UnselectedWmoUsefulBindings
            | AdtMode::UnselectedWmoLocalMissingM2
            | AdtMode::UnselectedWmoFilteredGeometry
            | AdtMode::UnselectedWmoUsefulReference
            | AdtMode::UnselectedWmoMissingSets
            | AdtMode::UnselectedWmoLogicalOrigin
            | AdtMode::UnselectedWmoLogicalEquivalent
    ) {
        return wmo_root_with(WMO_GROUP, &[MODEL, MISSING_M2], AdtMode::NoOmissions, 1);
    }
    if mode == AdtMode::UnselectedWmoMixed {
        return wmo_root_with(WMO_GROUP, &[MODEL, MISSING_M2], AdtMode::VacatedWmo, 1);
    }
    let spawn_ids = match mode {
        AdtMode::UnusedMissingDoodad => vec![MISSING_M2],
        AdtMode::EmptyWmoUsefulBindings | AdtMode::NoOmissions => vec![MODEL],
        AdtMode::UnselectedWmoLogicalOrigin | AdtMode::UnselectedWmoLogicalEquivalent => {
            vec![MODEL, MISSING_M2]
        }
        _ => vec![MODEL, MISSING_M2],
    };
    let present_mode = if matches!(
        mode,
        AdtMode::EmptyWmoMissingSets | AdtMode::MaterialFilteredWmoMissingSets
    ) {
        AdtMode::DoodadOmission
    } else {
        mode
    };
    wmo_root_with(WMO_GROUP, &spawn_ids, present_mode, 1)
}

pub fn unselected_wmo_source() -> (Vec<u8>, Vec<u8>) {
    (
        unselected_wmo_root(AdtMode::UnselectedWmoOnly),
        unselected_wmo_group(AdtMode::UnselectedWmoOnly),
    )
}

pub fn unselected_wmo_logical_source() -> (Vec<u8>, Vec<u8>) {
    (
        unselected_wmo_root(AdtMode::UnselectedWmoOnly),
        unselected_wmo_group(AdtMode::UnselectedWmoOnly),
    )
}

fn unselected_wmo_root(mode: AdtMode) -> Vec<u8> {
    let spawn_ids = if matches!(
        mode,
        AdtMode::UnselectedWmoLocalMissingM2 | AdtMode::UnselectedWmoLogicalOrigin
    ) {
        &[MISSING_M2][..]
    } else if mode == AdtMode::UnselectedWmoUsefulReference {
        &[USEFUL_REFERENCE_MODEL][..]
    } else if mode == AdtMode::UnselectedWmoUsefulBindings {
        &[MODEL][..]
    } else {
        &[]
    };
    let set_count = usize::from(mode != AdtMode::UnselectedWmoMissingSets);
    let set_mode = if mode == AdtMode::UnselectedWmoMissingSets {
        mode
    } else {
        AdtMode::UnselectedWmoOnly
    };
    wmo_root_with(UNSELECTED_WMO_GROUP, spawn_ids, set_mode, set_count)
}

fn missing_wmo_root() -> Vec<u8> {
    wmo_root_with(MISSING_WMO_GROUP, &[MISSING_M2], AdtMode::VacatedWmo, 1)
}

fn empty_wmo_root(mode: AdtMode) -> Vec<u8> {
    let spawn_ids = if mode == AdtMode::EmptyWmoUsefulBindings {
        &[MODEL][..]
    } else {
        &[]
    };
    wmo_root_with(
        EMPTY_WMO_GROUP,
        spawn_ids,
        mode,
        usize::from(mode != AdtMode::EmptyWmoMissingSets),
    )
}

fn wmo_root_with(group_id: u32, spawn_ids: &[u32], mode: AdtMode, set_count: usize) -> Vec<u8> {
    wmo_root_with_groups(&[group_id], spawn_ids, mode, set_count)
}

fn wmo_root_with_groups(
    group_ids: &[u32],
    spawn_ids: &[u32],
    mode: AdtMode,
    set_count: usize,
) -> Vec<u8> {
    let mut header = [0; 64];
    put32(&mut header, 4, group_ids.len() as u32);
    put32(&mut header, 32, 777);
    put32(&mut header, 24, set_count as u32);
    header[60] = 4;
    let mut bytes = chunk(b"MVER", &17_u32.to_le_bytes());
    bytes.extend_from_slice(&chunk(b"MOHD", &header));
    let ids: Vec<_> = group_ids.iter().flat_map(|id| id.to_le_bytes()).collect();
    bytes.extend_from_slice(&chunk(b"GFID", &ids));
    if mode == AdtMode::LogicalM2Name {
        bytes.extend_from_slice(&chunk(b"MODN", b"missing-logical.m2\0"));
    } else {
        let ids: Vec<_> = spawn_ids.iter().flat_map(|id| id.to_le_bytes()).collect();
        bytes.extend_from_slice(&chunk(b"MODI", &ids));
    }
    let mut spawns = vec![0; 40 * spawn_ids.len()];
    for (index, spawn) in spawns.as_chunks_mut::<40>().0.iter_mut().enumerate() {
        put32(spawn, 0, index as u32);
        put32(spawn, 28, 1.0_f32.to_bits());
        put32(spawn, 32, 1.0_f32.to_bits());
    }
    bytes.extend_from_slice(&chunk(b"MODD", &spawns));
    let mut sets = vec![0; set_count * 32];
    for set in sets.as_chunks_mut::<32>().0 {
        put32(set, 24, spawn_ids.len() as u32);
    }
    if !matches!(
        mode,
        AdtMode::EmptyWmoMissingSets
            | AdtMode::UnselectedWmoMissingSets
            | AdtMode::MaterialFilteredWmoMissingSets
    ) {
        bytes.extend_from_slice(&chunk(b"MODS", &sets));
    }
    let parsed = forever_vmap::wmo::Root::parse(&bytes, Default::default()).unwrap();
    assert_eq!(parsed.doodad_spawn_count(), spawn_ids.len());
    assert_eq!(parsed.doodad_set_count(), set_count);
    assert_eq!(
        parsed.detailed_group_ids().collect::<Vec<_>>(),
        group_ids.to_vec()
    );
    bytes
}

fn wmo_group_for(spawn_count: usize, geometry: bool) -> Vec<u8> {
    wmo_group_with(spawn_count, geometry, false, 8)
}

fn material_filtered_wmo_root(mode: AdtMode) -> Vec<u8> {
    let spawn_ids = match mode {
        AdtMode::MaterialFilteredWmoLocalMissingM2 => &[MISSING_M2][..],
        AdtMode::MaterialFilteredWmoUsefulBindings => &[MODEL][..],
        AdtMode::MaterialFilteredWmoUsefulReference => &[USEFUL_REFERENCE_MODEL][..],
        _ => &[],
    };
    let group_ids = if matches!(
        mode,
        AdtMode::MaterialFilteredWmoSkippedGroup | AdtMode::MaterialFilteredWmoUsefulReference
    ) {
        &[
            MATERIAL_FILTERED_WMO_GROUP,
            MATERIAL_FILTERED_WMO_SKIPPED_GROUP,
        ][..]
    } else {
        &[MATERIAL_FILTERED_WMO_GROUP][..]
    };
    let set_count = usize::from(mode != AdtMode::MaterialFilteredWmoMissingSets);
    wmo_root_with_groups(group_ids, spawn_ids, mode, set_count)
}

fn material_filtered_wmo_group(mode: AdtMode) -> Vec<u8> {
    let references = usize::from(matches!(
        mode,
        AdtMode::MaterialFilteredWmoUsefulBindings
            | AdtMode::MaterialFilteredWmoUsefulReference
            | AdtMode::MaterialFilteredWmoLocalMissingM2
    ));
    let flags = match mode {
        AdtMode::MaterialFilteredWmoSelectedGeometry => vec![0x08],
        AdtMode::MaterialFilteredWmoKeptSurface => vec![0x20],
        AdtMode::MaterialFilteredWmoUsefulBindings
        | AdtMode::MaterialFilteredWmoUsefulReference
        | AdtMode::MaterialFilteredWmoLocalMissingM2 => vec![0x04],
        _ => vec![0x26, 0x66],
    };
    material_filtered_group_with(
        references,
        &flags,
        false,
        mode == AdtMode::MaterialFilteredWmoLiquid,
    )
}

fn material_filtered_wmo_mixed_group() -> Vec<u8> {
    material_filtered_group_with(0, &[0x26, 0x66], false, false)
}

fn material_filtered_wmo_selected_group() -> Vec<u8> {
    material_filtered_group_with(0, &[0x08], false, false)
}

fn material_filtered_wmo_useful_binding_group() -> Vec<u8> {
    material_filtered_group_with(1, &[0x04], false, false)
}

fn material_filtered_wmo_useful_reference_groups() -> (Vec<u8>, Vec<u8>) {
    (
        material_filtered_group_with(1, &[0x04], false, false),
        material_filtered_group_with(1, &[0x26], true, false),
    )
}

pub fn material_filtered_wmo_source() -> (Vec<u8>, Vec<u8>) {
    (
        material_filtered_wmo_root(AdtMode::MaterialFilteredWmoOnly),
        material_filtered_wmo_group(AdtMode::MaterialFilteredWmoOnly),
    )
}

pub fn material_filtered_wmo_selected_geometry_source() -> (Vec<u8>, Vec<u8>) {
    (
        material_filtered_wmo_root(AdtMode::MaterialFilteredWmoSelectedGeometry),
        material_filtered_wmo_selected_group(),
    )
}

pub fn material_filtered_wmo_kept_surface_source() -> (Vec<u8>, Vec<u8>) {
    (
        material_filtered_wmo_root(AdtMode::MaterialFilteredWmoKeptSurface),
        material_filtered_wmo_group(AdtMode::MaterialFilteredWmoKeptSurface),
    )
}

pub fn material_filtered_wmo_useful_reference_source() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let (selected, skipped) = material_filtered_wmo_useful_reference_groups();
    (
        material_filtered_wmo_root(AdtMode::MaterialFilteredWmoUsefulReference),
        selected,
        skipped,
    )
}

pub fn assert_material_filtered_wmo_material_controls(
    root_bytes: &[u8],
    group_bytes: &[u8],
    expected_flags: u16,
) {
    use forever_vmap::wmo::{self, GroupInput};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(
        root.detailed_group_ids().collect::<Vec<_>>(),
        [MATERIAL_FILTERED_WMO_GROUP]
    );
    assert_eq!(root.doodad_spawn_count(), 0);
    let group = wmo::Group::parse(group_bytes, Default::default()).unwrap();
    assert_eq!(group.triangle_count(), 1);
    assert_eq!(group.triangle_material(0).unwrap().flags, expected_flags);
    assert!(group.liquid().is_none());
    let inputs = [GroupInput {
        file_data_id: MATERIAL_FILTERED_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert!(contexts[0].skip_reason().is_none());
    let geometry = contexts[0]
        .project_geometry(Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(geometry.triangle_count(), 1);
}

pub fn assert_material_filtered_wmo_source(root_bytes: &[u8], group_bytes: &[u8]) {
    use forever_vmap::wmo::{self, GroupInput};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(
        root.detailed_group_ids().collect::<Vec<_>>(),
        [MATERIAL_FILTERED_WMO_GROUP]
    );
    assert_eq!(root.doodad_set_count(), 1);
    assert_eq!(root.doodad_spawn_count(), 0);
    let group = wmo::Group::parse(group_bytes, Default::default()).unwrap();
    assert_eq!(group.triangle_count(), 2);
    assert_eq!(group.doodad_reference_count(), 0);
    assert!(group.liquid().is_none());
    assert_eq!(
        (0..group.triangle_count())
            .map(|index| group.triangle_material(index).unwrap().flags)
            .collect::<Vec<_>>(),
        [0x26, 0x66]
    );
    let inputs = [GroupInput {
        file_data_id: MATERIAL_FILTERED_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert_eq!(contexts[0].skip_reason(), None);
    let geometry = contexts[0]
        .project_geometry(Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(geometry.triangle_count(), 0);
    let selection = wmo::doodads::resolve(&root, &inputs, &[], Default::default()).unwrap();
    assert!(selection.references().is_empty());
    assert_eq!(selection.set_count(), 1);
    assert_eq!(
        selection.outcome(false),
        wmo::doodads::Outcome::FailedNoCollision
    );
}

pub fn assert_material_filtered_wmo_useful_reference_source(
    root_bytes: &[u8],
    selected_bytes: &[u8],
    skipped_bytes: &[u8],
) {
    use forever_vmap::wmo::{self, GroupInput, SkipReason};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(root.doodad_spawn_count(), 1);
    assert_eq!(root.doodad_file_data_id(0), Some(USEFUL_REFERENCE_MODEL));
    let selected = wmo::Group::parse(selected_bytes, Default::default()).unwrap();
    assert_eq!(selected.doodad_reference_count(), 1);
    assert_eq!(selected.doodad_reference(0), Some(0));
    let skipped = wmo::Group::parse(skipped_bytes, Default::default()).unwrap();
    assert_eq!(skipped.doodad_reference_count(), 1);
    assert_eq!(skipped.doodad_reference(0), Some(0));
    let inputs = [
        GroupInput {
            file_data_id: MATERIAL_FILTERED_WMO_GROUP,
            group: &selected,
        },
        GroupInput {
            file_data_id: MATERIAL_FILTERED_WMO_SKIPPED_GROUP,
            group: &skipped,
        },
    ];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert!(contexts[0].skip_reason().is_none());
    assert_eq!(
        contexts[0]
            .project_geometry(Default::default())
            .unwrap()
            .unwrap()
            .triangle_count(),
        0
    );
    assert_eq!(contexts[1].skip_reason(), Some(SkipReason::Unreachable));
    assert!(
        contexts[1]
            .project_geometry(Default::default())
            .unwrap()
            .is_none()
    );
    let selection = wmo::doodads::resolve(&root, &inputs, &[0], Default::default()).unwrap();
    assert_eq!(selection.references(), [0]);
    assert_eq!(selection.set_count(), 1);
    assert_eq!(
        selection.outcome(false),
        wmo::doodads::Outcome::OkNoCollision
    );
}

fn material_filtered_group_with(
    spawn_count: usize,
    triangle_flags: &[u8],
    skip_selection: bool,
    liquid: bool,
) -> Vec<u8> {
    let mut header = vec![0; 68];
    if skip_selection {
        put32(&mut header, 8, 0x80);
    }
    put32(&mut header, 56, 99);
    let references: Vec<u8> = (0..spawn_count as u16).flat_map(u16::to_le_bytes).collect();
    header.extend_from_slice(&chunk(b"MODR", &references));
    let materials: Vec<_> = triangle_flags
        .iter()
        .flat_map(|flags| [*flags, 8])
        .collect();
    header.extend_from_slice(&chunk(b"MOPY", &materials));
    let indices: Vec<_> = triangle_flags
        .iter()
        .flat_map(|_| [0_u16, 1, 2])
        .flat_map(u16::to_le_bytes)
        .collect();
    header.extend_from_slice(&chunk(b"MOVI", &indices));
    let mut vertices = Vec::new();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        vertices.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    header.extend_from_slice(&chunk(b"MOVT", &vertices));
    if liquid {
        header.extend_from_slice(&chunk(b"MLIQ", &[0; 30]));
    }
    let bytes = chunk(b"MOGP", &header);
    let parsed = forever_vmap::wmo::Group::parse(&bytes, Default::default()).unwrap();
    assert_eq!(parsed.doodad_reference_count(), spawn_count);
    assert_eq!(parsed.triangle_count(), triangle_flags.len());
    for (index, flags) in triangle_flags.iter().enumerate() {
        assert_eq!(
            parsed.triangle_material(index).unwrap().flags,
            u16::from(*flags)
        );
    }
    bytes
}

fn unselected_wmo_group(mode: AdtMode) -> Vec<u8> {
    let references = usize::from(matches!(
        mode,
        AdtMode::UnselectedWmoUsefulBindings
            | AdtMode::UnselectedWmoLocalMissingM2
            | AdtMode::UnselectedWmoUsefulReference
    ));
    let triangle_flags = match mode {
        AdtMode::UnselectedWmoFilteredGeometry => 0x04,
        AdtMode::UnselectedWmoUsefulReference => 0,
        _ => 8,
    };
    wmo_group_with(
        references,
        true,
        !matches!(
            mode,
            AdtMode::UnselectedWmoSelectedGeometry
                | AdtMode::UnselectedWmoFilteredGeometry
                | AdtMode::UnselectedWmoUsefulReference
        ),
        triangle_flags,
    )
}

fn wmo_group_with(
    spawn_count: usize,
    geometry: bool,
    skip_selection: bool,
    triangle_flags: u8,
) -> Vec<u8> {
    let mut header = vec![0; 68];
    if skip_selection {
        put32(&mut header, 8, 0x80);
    }
    put32(&mut header, 56, 99);
    let references: Vec<u8> = (0..spawn_count as u16).flat_map(u16::to_le_bytes).collect();
    header.extend_from_slice(&chunk(b"MODR", &references));
    if geometry {
        header.extend_from_slice(&chunk(b"MOPY", &[triangle_flags, 8]));
        header.extend_from_slice(&chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        let mut vertices = Vec::new();
        for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            vertices.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        header.extend_from_slice(&chunk(b"MOVT", &vertices));
    }
    let bytes = chunk(b"MOGP", &header);
    let parsed = forever_vmap::wmo::Group::parse(&bytes, Default::default()).unwrap();
    assert_eq!(parsed.doodad_reference_count(), spawn_count);
    if geometry {
        assert_eq!(
            parsed.triangle_material(0).unwrap().flags,
            u16::from(triangle_flags)
        );
    }
    bytes
}

pub fn unselected_wmo_local_missing_m2_source() -> (Vec<u8>, Vec<u8>) {
    (
        unselected_wmo_root(AdtMode::UnselectedWmoLocalMissingM2),
        unselected_wmo_group(AdtMode::UnselectedWmoLocalMissingM2),
    )
}

pub fn unselected_wmo_filtered_geometry_source() -> (Vec<u8>, Vec<u8>) {
    (
        unselected_wmo_root(AdtMode::UnselectedWmoFilteredGeometry),
        unselected_wmo_group(AdtMode::UnselectedWmoFilteredGeometry),
    )
}

pub fn unselected_wmo_useful_reference_source() -> (Vec<u8>, Vec<u8>) {
    (
        unselected_wmo_root(AdtMode::UnselectedWmoUsefulReference),
        unselected_wmo_group(AdtMode::UnselectedWmoUsefulReference),
    )
}

pub fn unselected_wmo_logical_equivalent_source() -> (Vec<u8>, Vec<u8>) {
    unselected_wmo_source()
}

pub fn assert_filtered_unselected_wmo_source(root_bytes: &[u8], group_bytes: &[u8]) {
    use forever_vmap::wmo::{self, GroupInput};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(root.doodad_spawn_count(), 0);
    assert_eq!(root.doodad_set_count(), 1);
    let group = wmo::Group::parse(group_bytes, Default::default()).unwrap();
    let inputs = [GroupInput {
        file_data_id: UNSELECTED_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert!(contexts[0].skip_reason().is_none());
    assert_eq!(group.triangle_count(), 1);
    assert_eq!(group.triangle_material(0).unwrap().flags, 0x04);
    let geometry = contexts[0]
        .project_geometry(Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(geometry.triangle_count(), 0);
    let selection = wmo::doodads::resolve(&root, &inputs, &[], Default::default()).unwrap();
    assert!(selection.references().is_empty());
    assert!(selection.set_count() > 0);
}

pub fn assert_useful_unselected_wmo_reference_source(root_bytes: &[u8], group_bytes: &[u8]) {
    use forever_vmap::wmo::{self, GroupInput};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(root.doodad_spawn_count(), 1);
    assert_eq!(root.doodad_set_count(), 1);
    let group = wmo::Group::parse(group_bytes, Default::default()).unwrap();
    let inputs = [GroupInput {
        file_data_id: UNSELECTED_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert!(contexts[0].skip_reason().is_none());
    assert_eq!(group.triangle_count(), 1);
    assert_eq!(group.triangle_material(0).unwrap().flags, 0);
    assert_eq!(group.doodad_reference_count(), 1);
    assert_eq!(group.doodad_reference(0), Some(0));
    assert_eq!(root.doodad_file_data_id(0), Some(USEFUL_REFERENCE_MODEL));
    let geometry = contexts[0]
        .project_geometry(Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(geometry.triangle_count(), 0);
    let selection = wmo::doodads::resolve(&root, &inputs, &[0], Default::default()).unwrap();
    assert_eq!(selection.references(), [0]);
    assert_eq!(selection.set_count(), 1);
    assert_eq!(
        selection.outcome(false),
        wmo::doodads::Outcome::OkNoCollision
    );
}

pub fn assert_logical_equivalent_unselected_wmo_source(root_bytes: &[u8], group_bytes: &[u8]) {
    let expected = unselected_wmo_source();
    assert_eq!(root_bytes, expected.0);
    assert_eq!(group_bytes, expected.1);
    assert_unselected_wmo_source(root_bytes, group_bytes);
}

pub fn assert_unselected_wmo_source(root_bytes: &[u8], group_bytes: &[u8]) {
    use forever_vmap::wmo::{self, GroupInput, SkipReason};
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert!(root.has_gfid());
    assert_eq!(
        root.detailed_group_ids().collect::<Vec<_>>(),
        [UNSELECTED_WMO_GROUP]
    );
    assert!(root.doodad_set_count() > 0);
    assert_eq!(root.doodad_spawn_count(), 0);
    assert_eq!(root.doodad_name_bytes(), None);
    assert_eq!(root.doodad_file_data_id_count(), 0);

    let group = wmo::Group::parse(group_bytes, Default::default()).unwrap();
    assert_eq!(group.triangle_count(), 1);
    assert_eq!(group.index_count(), 3);
    assert_eq!(group.vertex_count(), 3);
    assert_eq!(group.triangle_material(0).unwrap().flags, 8);
    assert_eq!(group.doodad_reference_count(), 0);
    let inputs = [GroupInput {
        file_data_id: UNSELECTED_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &inputs, Default::default()).unwrap();
    assert_eq!(contexts.len(), 1);
    assert_eq!(contexts[0].skip_reason(), Some(SkipReason::Unreachable));
    assert!(
        contexts[0]
            .project_geometry(Default::default())
            .unwrap()
            .is_none()
    );
    let selection = wmo::doodads::resolve(&root, &inputs, &[], Default::default()).unwrap();
    assert!(selection.references().is_empty());
    assert!(selection.set_count() > 0);
}

fn assert_empty_wmo_source(inputs: &[(u32, Vec<u8>)], mode: AdtMode) {
    use forever_vmap::wmo::{self, GroupInput};
    if mode == AdtMode::UnselectedWmoLogicalOrigin {
        return;
    }
    if !matches!(
        mode,
        AdtMode::EmptyWmoOnly
            | AdtMode::EmptyWmoGeometry
            | AdtMode::EmptyWmoMissingGroup
            | AdtMode::EmptyWmoMissingSets
            | AdtMode::EmptyWmoUsefulBindings
    ) {
        return;
    }
    let id = if mode == AdtMode::EmptyWmoOnly {
        MISSING_WMO
    } else {
        EMPTY_WMO
    };
    let root_bytes = &inputs.iter().find(|(fdid, _)| *fdid == id).unwrap().1;
    let root = wmo::Root::parse(root_bytes, Default::default()).unwrap();
    assert_eq!(
        root.detailed_group_ids().collect::<Vec<_>>(),
        [EMPTY_WMO_GROUP]
    );
    assert_eq!(
        root.doodad_set_count(),
        usize::from(mode != AdtMode::EmptyWmoMissingSets)
    );
    assert_eq!(
        root.doodad_spawn_count(),
        usize::from(mode == AdtMode::EmptyWmoUsefulBindings)
    );
    let group = inputs.iter().find(|(fdid, _)| *fdid == EMPTY_WMO_GROUP);
    if mode == AdtMode::EmptyWmoMissingGroup {
        assert!(group.is_none());
        return;
    }
    let group = wmo::Group::parse(&group.unwrap().1, Default::default()).unwrap();
    assert_eq!(
        group.triangle_count(),
        usize::from(mode == AdtMode::EmptyWmoGeometry)
    );
    let group_inputs = [GroupInput {
        file_data_id: EMPTY_WMO_GROUP,
        group: &group,
    }];
    let contexts = wmo::resolve_contexts(&root, &group_inputs, Default::default()).unwrap();
    let geometry = contexts[0]
        .project_geometry(Default::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        geometry.triangle_count(),
        usize::from(mode == AdtMode::EmptyWmoGeometry)
    );
    assert_eq!(group.doodad_reference_count(), 0);
    if mode == AdtMode::EmptyWmoUsefulBindings {
        assert_eq!(root.doodad_file_data_id(0), Some(MODEL));
        assert!(inputs.iter().any(|(fdid, _)| *fdid == MODEL));
        let selection = wmo::doodads::resolve(
            &root,
            &[GroupInput {
                file_data_id: EMPTY_WMO_GROUP,
                group: &group,
            }],
            &[0],
            Default::default(),
        )
        .unwrap();
        assert!(selection.references().is_empty());
    }
}

fn normal_record(bytes: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(bytes);
    record
}

fn root_file(rows: &[(u32, u32, [u8; 16], Option<u64>)]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    let named = rows.iter().filter(|row| row.3.is_some()).count() as u32;
    for value in [20_u32, 2, rows.len() as u32, named] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for (file_data_id, locale_flags, ckey, name_hash) in rows {
        let content_flags = if name_hash.is_some() { 0 } else { 0x1000_0000 };
        for value in [1_u32, *locale_flags, content_flags, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&file_data_id.to_le_bytes());
        bytes.extend_from_slice(ckey);
        if let Some(name_hash) = name_hash {
            bytes.extend_from_slice(&name_hash.to_le_bytes());
        }
    }
    bytes
}

fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
    let mut page = Vec::new();
    for (ckey, size, ekey) in rows {
        page.extend_from_slice(&1_u16.to_le_bytes());
        page.extend_from_slice(&size.to_be_bytes());
        page.extend_from_slice(ckey);
        page.extend_from_slice(ekey);
    }
    assert!(page.len() <= 1024);
    page.resize(1024, 0);
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(&rows[0].0);
    bytes.extend_from_slice(&md5::digest(&page));
    bytes.extend_from_slice(&page);
    bytes
}

fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
    let mut row = [0; 18];
    row[..9].copy_from_slice(&key[..9]);
    row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
    row[14..].copy_from_slice(&size.to_le_bytes());
    row
}

fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
    let hash = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&hash.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(rows.len() * 18).unwrap().to_le_bytes());
    let mut checksum = (0, 0);
    for row in rows {
        checksum = jenkins::hashlittle2(row, checksum.0, checksum.1);
    }
    bytes.extend_from_slice(&checksum.0.to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(row);
    }
    bytes
}

fn partial_blte(map: &[u8], hidden_start: usize, hidden_size: usize) -> (Vec<u8>, [u8; 16]) {
    let mut normal = vec![b'N'];
    normal.extend_from_slice(&map[..hidden_start]);
    let mut encrypted = vec![b'E', 8];
    encrypted.extend_from_slice(&LOOKUP.to_le_bytes());
    encrypted.push(4);
    encrypted.extend_from_slice(&[0x55; 4]);
    encrypted.push(b'S');
    encrypted.extend_from_slice(&[0xa5; 16]);
    let header_size = 12 + 2 * 24;
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(header_size as u32).to_be_bytes());
    encoded.extend_from_slice(&[15, 0, 0, 2]);
    for (frame, decoded_size) in [
        (&normal[..], hidden_start as u32),
        (&encrypted[..], hidden_size as u32),
    ] {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&decoded_size.to_be_bytes());
        encoded.extend_from_slice(&md5::digest(frame));
    }
    let ekey = md5::digest(&encoded);
    encoded.extend_from_slice(&normal);
    encoded.extend_from_slice(&encrypted);
    (encoded, ekey)
}

fn private_dirs(path: &Path) {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[derive(Clone, Copy, Default)]
pub struct ClientMutation {
    pub omit_root: Option<u32>,
    pub ineligible_root: Option<u32>,
    pub omit_central: Option<u32>,
    pub omit_encoding: Option<u32>,
    pub malformed_wdt: Option<u32>,
    pub malformed_model: Option<u32>,
    pub populated_adt_mode: AdtMode,
}

pub struct WrittenClient {
    pub archive_offsets: BTreeMap<u32, u64>,
}

pub fn write_client(client: &Path) -> WrittenClient {
    write_client_with(client, ClientMutation::default())
}

pub fn write_client_with(client: &Path, mutation: ClientMutation) -> WrittenClient {
    write_client_with_logical_name(client, mutation, false)
}

pub fn write_client_with_logical_name(
    client: &Path,
    mutation: ClientMutation,
    logical_empty_wmo: bool,
) -> WrittenClient {
    let map = partial_map();
    let (map_encoded, map_ekey) = partial_blte(
        &map.bytes,
        map.hidden_start,
        map.declaration.decoded_size as usize,
    );
    let logical_empty_origin =
        logical_empty_wmo || mutation.populated_adt_mode == AdtMode::EmptyWmoLogicalOrigin;
    let logical_unselected_origin = matches!(
        mutation.populated_adt_mode,
        AdtMode::UnselectedWmoLogicalOrigin | AdtMode::UnselectedWmoLogicalEquivalent
    );
    let logical_material_filtered_origin =
        mutation.populated_adt_mode == AdtMode::MaterialFilteredWmoLogicalOrigin;
    let logical_origin =
        logical_empty_origin || logical_unselected_origin || logical_material_filtered_origin;
    let mut inputs = vec![
        (table_id("Map"), map.bytes),
        (
            table_id("LiquidMaterial"),
            complete_db2("LiquidMaterial", &[(7, 0)]),
        ),
        (
            table_id("LiquidType"),
            complete_db2("LiquidType", &[(22, 7)]),
        ),
        (
            table_id("GameObjectDisplayInfo"),
            b"poisonous synthetic GODI: any read must fail".to_vec(),
        ),
        (
            table_id("LiquidObject"),
            b"poisonous synthetic LiquidObject: any read must fail".to_vec(),
        ),
        (EMPTY_WDT, collision_wdt(None)),
        (POPULATED_WDT, collision_wdt(Some(POPULATED_ADT))),
        (CHILD_WDT, collision_wdt(Some(CHILD_ADT))),
        (
            POPULATED_ADT,
            if logical_origin {
                let mut logical_name = LOGICAL_EMPTY_WMO.to_vec();
                logical_name.push(0);
                let mut adt = chunk(b"MWMO", &logical_name);
                let mut placement = modf(0, 42);
                placement[56..58].fill(0);
                placement[60..62].fill(0);
                adt.extend_from_slice(&chunk(b"MODF", &placement));
                adt
            } else {
                collision_adt(42, mutation.populated_adt_mode)
            },
        ),
        (CHILD_ADT, collision_adt(43, AdtMode::DoodadOmission)),
        (MODEL, collision_m2()),
    ];
    let mode = mutation.populated_adt_mode;
    if matches!(
        mode,
        AdtMode::UnselectedWmoUsefulReference | AdtMode::MaterialFilteredWmoUsefulReference
    ) {
        let mut model = collision_m2();
        let last_vertex_z = model.len() - 4;
        model[last_vertex_z..].copy_from_slice(&1.0_f32.to_bits().to_le_bytes());
        inputs.push((USEFUL_REFERENCE_MODEL, model));
    }
    if logical_empty_origin {
        inputs.push((WMO_ROOT, wmo_root(AdtMode::EmptyWmoUsefulBindings)));
        inputs.push((WMO_GROUP, wmo_group_for(1, true)));
        inputs.push((LOGICAL_EMPTY_WMO_ID, empty_wmo_root(AdtMode::EmptyWmo)));
        inputs.push((EMPTY_WMO_GROUP, wmo_group_for(0, false)));
    } else if logical_unselected_origin {
        let (root, group) = unselected_wmo_source();
        inputs.push((LOGICAL_EMPTY_WMO_ID, root));
        inputs.push((UNSELECTED_WMO_GROUP, group));
    } else if logical_material_filtered_origin {
        inputs.push((LOGICAL_EMPTY_WMO_ID, material_filtered_wmo_root(mode)));
        inputs.push((
            MATERIAL_FILTERED_WMO_GROUP,
            material_filtered_wmo_group(mode),
        ));
    } else {
        if matches!(mode, AdtMode::EmptyWmo | AdtMode::EmptyWmoOnly) {
            inputs.push((WMO_ROOT, wmo_root(AdtMode::EmptyWmoUsefulBindings)));
            inputs.push((WMO_GROUP, wmo_group_for(1, true)));
        } else if mode != AdtMode::MissingWmo {
            inputs.push((WMO_ROOT, wmo_root(mode)));
        }
        if !matches!(
            mode,
            AdtMode::EmptyWmo | AdtMode::EmptyWmoOnly | AdtMode::MissingWmoGroup
        ) {
            let references = match mode {
                AdtMode::UnusedMissingDoodad => 0,
                AdtMode::EmptyWmoUsefulBindings | AdtMode::NoOmissions => 1,
                _ => 2,
            };
            inputs.push((
                WMO_GROUP,
                wmo_group_for(references, mode != AdtMode::UnusedMissingDoodad),
            ));
        }
        if mode == AdtMode::EmptyWmoOnly {
            inputs.push((MISSING_WMO, empty_wmo_root(AdtMode::EmptyWmo)));
            inputs.push((EMPTY_WMO_GROUP, wmo_group_for(0, false)));
        } else if mode == AdtMode::VacatedWmo {
            inputs.push((MISSING_WMO, missing_wmo_root()));
            inputs.push((MISSING_WMO_GROUP, wmo_group_for(1, false)));
            inputs.push((EMPTY_WMO, empty_wmo_root(AdtMode::EmptyWmo)));
            inputs.push((EMPTY_WMO_GROUP, wmo_group_for(0, false)));
        } else if matches!(
            mode,
            AdtMode::UnselectedWmoOnly
                | AdtMode::UnselectedWmoSelectedGeometry
                | AdtMode::UnselectedWmoUsefulBindings
                | AdtMode::UnselectedWmoLocalMissingM2
                | AdtMode::UnselectedWmoFilteredGeometry
                | AdtMode::UnselectedWmoUsefulReference
                | AdtMode::UnselectedWmoMissingSets
        ) {
            inputs.push((UNSELECTED_WMO, unselected_wmo_root(mode)));
            inputs.push((UNSELECTED_WMO_GROUP, unselected_wmo_group(mode)));
        } else if matches!(
            mode,
            AdtMode::MaterialFilteredWmoOnly
                | AdtMode::MaterialFilteredWmoSelectedGeometry
                | AdtMode::MaterialFilteredWmoKeptSurface
                | AdtMode::MaterialFilteredWmoUsefulBindings
                | AdtMode::MaterialFilteredWmoLocalMissingM2
                | AdtMode::MaterialFilteredWmoUsefulReference
                | AdtMode::MaterialFilteredWmoMissingSets
                | AdtMode::MaterialFilteredWmoLogicalOrigin
                | AdtMode::MaterialFilteredWmoLiquid
                | AdtMode::MaterialFilteredWmoSkippedGroup
        ) {
            inputs.push((MATERIAL_FILTERED_WMO, material_filtered_wmo_root(mode)));
            inputs.push((
                MATERIAL_FILTERED_WMO_GROUP,
                material_filtered_wmo_group(mode),
            ));
            if matches!(
                mode,
                AdtMode::MaterialFilteredWmoSkippedGroup
                    | AdtMode::MaterialFilteredWmoUsefulReference
            ) {
                let skipped_references =
                    usize::from(mode == AdtMode::MaterialFilteredWmoUsefulReference);
                inputs.push((
                    MATERIAL_FILTERED_WMO_SKIPPED_GROUP,
                    material_filtered_group_with(skipped_references, &[0x26], true, false),
                ));
            }
        } else if mode == AdtMode::MaterialFilteredWmoMixed {
            inputs.push((MISSING_WMO, missing_wmo_root()));
            inputs.push((MISSING_WMO_GROUP, wmo_group_for(1, false)));
            inputs.push((EMPTY_WMO, empty_wmo_root(AdtMode::EmptyWmo)));
            inputs.push((EMPTY_WMO_GROUP, wmo_group_for(0, false)));
            inputs.push((
                UNSELECTED_WMO,
                unselected_wmo_root(AdtMode::UnselectedWmoOnly),
            ));
            inputs.push((
                UNSELECTED_WMO_GROUP,
                unselected_wmo_group(AdtMode::UnselectedWmoOnly),
            ));
            inputs.push((
                MATERIAL_FILTERED_WMO_MIXED,
                material_filtered_wmo_root(mode),
            ));
            inputs.push((
                MATERIAL_FILTERED_WMO_GROUP,
                material_filtered_wmo_mixed_group(),
            ));
        } else if mode == AdtMode::UnselectedWmoMixed {
            inputs.push((UNSELECTED_WMO, unselected_wmo_root(mode)));
            inputs.push((UNSELECTED_WMO_GROUP, unselected_wmo_group(mode)));
            inputs.push((MISSING_WMO, missing_wmo_root()));
            inputs.push((MISSING_WMO_GROUP, wmo_group_for(1, false)));
            inputs.push((EMPTY_WMO, empty_wmo_root(AdtMode::EmptyWmo)));
            inputs.push((EMPTY_WMO_GROUP, wmo_group_for(0, false)));
        } else if mode == AdtMode::VacatedWmoOnly {
            inputs.push((MISSING_WMO, missing_wmo_root()));
            inputs.push((MISSING_WMO_GROUP, wmo_group_for(1, false)));
        } else if matches!(
            mode,
            AdtMode::EmptyWmo
                | AdtMode::EmptyWmoGeometry
                | AdtMode::EmptyWmoMissingGroup
                | AdtMode::EmptyWmoMissingSets
                | AdtMode::EmptyWmoUsefulBindings
        ) {
            inputs.push((EMPTY_WMO, empty_wmo_root(mode)));
            if mode != AdtMode::EmptyWmoMissingGroup {
                inputs.push((
                    EMPTY_WMO_GROUP,
                    wmo_group_for(0, mode == AdtMode::EmptyWmoGeometry),
                ));
            }
        }
    }
    assert_empty_wmo_source(&inputs, mode);
    let mut root_rows = Vec::new();
    let mut source_routes = Vec::new();
    let mut records = Vec::new();
    for (ordinal, (file_data_id, bytes)) in inputs.into_iter().enumerate() {
        if mutation.omit_root == Some(file_data_id) {
            continue;
        }
        let bytes = if mutation.malformed_wdt == Some(file_data_id) {
            b"malformed synthetic WDT".to_vec()
        } else if mutation.malformed_model == Some(file_data_id) {
            b"malformed synthetic model".to_vec()
        } else {
            bytes
        };
        let ckey = md5::digest(&bytes);
        let (ekey, record) = if file_data_id == table_id("Map") {
            (map_ekey, map_encoded.clone())
        } else {
            let ekey = [0x80_u8.wrapping_add(ordinal as u8); 16];
            (ekey, normal_record(&bytes))
        };
        if mutation.omit_encoding != Some(file_data_id) {
            source_routes.push((ckey, bytes.len() as u32, ekey));
        }
        let locale_flags = if mutation.ineligible_root == Some(file_data_id) {
            2
        } else {
            0
        };
        let name_hash = logical_origin
            .then(|| {
                (file_data_id == LOGICAL_EMPTY_WMO_ID)
                    .then(|| forever_casc::root::name::hash_name(LOGICAL_EMPTY_WMO, 1024).unwrap())
            })
            .flatten();
        root_rows.push((file_data_id, locale_flags, ckey, name_hash));
        if mutation.omit_central != Some(file_data_id) {
            records.push((Some(file_data_id), ekey, record));
        }
    }
    let root = root_file(&root_rows);
    let root_ckey = md5::digest(&root);
    let root_ekey = [0x33; 16];
    let mut routes = vec![(root_ckey, root.len() as u32, root_ekey)];
    routes.extend(source_routes);
    let encoding = encoding_file(&routes);
    let encoding_ckey = md5::digest(&encoding);
    let encoding_ekey = [0x44; 16];
    records.push((None, root_ekey, normal_record(&root)));
    records.push((None, encoding_ekey, normal_record(&encoding)));
    let config = format!(
        "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70450\n",
        hex16(root_ckey),
        hex16(encoding_ckey),
        hex16(encoding_ekey),
        encoding.len(),
        encoding.len() + 39
    )
    .into_bytes();
    let config_ckey = md5::digest(&config);
    let config_name = hex16(config_ckey);
    let data = client.join("Data/data");
    let config_dir = client
        .join("Data/config")
        .join(&config_name[..2])
        .join(&config_name[2..4]);
    private_dirs(&data);
    private_dirs(&config_dir);
    fs::write(config_dir.join(&config_name), &config).unwrap();
    fs::write(
        client.join(".build.info"),
        format!(
            "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|esES\n",
            hex16(config_ckey),
            hex16([0x21; 16])
        ),
    )
    .unwrap();
    let mut archive = Vec::new();
    let mut archive_offsets = BTreeMap::new();
    let mut index_rows = Vec::new();
    for (file_data_id, ekey, record) in records {
        let offset = archive.len() as u64;
        if let Some(id) = file_data_id {
            archive_offsets.insert(id, offset);
        }
        archive.extend_from_slice(&[0xa5; 30]);
        archive.extend_from_slice(&record);
        index_rows.push(index_entry(ekey, offset, (30 + record.len()) as u32));
    }
    index_rows.sort_unstable_by_key(|row| row[..9].to_vec());
    fs::write(data.join("data.000"), archive).unwrap();
    for bucket in 0..16_u8 {
        let rows = if bucket == 0 {
            index_rows.clone()
        } else {
            vec![index_entry([0xe0 + bucket; 16], 0, 39)]
        };
        fs::write(
            data.join(format!("{bucket:02x}00000001.idx")),
            index_file(bucket, &rows),
        )
        .unwrap();
    }
    WrittenClient { archive_offsets }
}

pub fn corrupt_input(client: &Path, written: &WrittenClient, file_data_id: u32) {
    let archive_path = client.join("Data/data/data.000");
    let mut archive = fs::read(&archive_path).unwrap();
    let offset = written.archive_offsets[&file_data_id] as usize;
    let corrupt_at = offset + 30 + 8 + 1;
    archive[corrupt_at] = b'X';
    fs::write(archive_path, archive).unwrap();
}
