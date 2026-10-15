// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic CASC shared-world integration.
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use forever_vmap::{placement, wmo::raw};

fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
// Literal numeric WDC5 fixture, independent of generated schema objects.
pub(in crate::collision) fn gameobject_db2(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = vec![0; 496];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, 9),
        (144, 52),
        (156, 0x16D1_DD61),
        (164, u32::MAX),
        (176, 9),
        (180, 52),
        (188, 216),
        (200, 1),
        (212, 496),
        (216, rows.len() as u32),
        (228, rows.len() as u32 * 4),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (field, (offset, bits, count)) in [
        (0u16, 32u16, 6u16),
        (24, 32, 1),
        (28, 16, 1),
        (30, 32, 1),
        (34, 32, 1),
        (38, 32, 1),
        (42, 32, 1),
        (46, 32, 1),
        (50, 16, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let primitive = 244 + field * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - bits).to_le_bytes());
        bytes[primitive + 2..primitive + 4].copy_from_slice(&offset.to_le_bytes());
        let col = 280 + field * 24;
        bytes[col..col + 2].copy_from_slice(&(offset * 8).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(bits * count).to_le_bytes());
    }
    for (_, fdid) in rows {
        let mut record = [0; 52];
        put(&mut record, 24, *fdid);
        bytes.extend_from_slice(&record);
    }
    for (id, _) in rows {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
}
pub(in crate::collision) fn m2(faces: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if faces {
        for (at, value) in [(216, 3), (220, 304), (224, 3), (228, 310)] {
            put(&mut bytes, at, value);
        }
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        bytes.extend(words(&[
            0x8000_0000,
            0,
            0,
            1f32.to_bits(),
            0,
            0,
            0,
            1f32.to_bits(),
            0,
        ]));
    }
    bytes
}
pub(in crate::collision) fn wmo_root(group: u32, child: Option<u32>) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut bytes = chunk(b"MVER", &17u32.to_le_bytes());
    bytes.extend(chunk(b"MOHD", &header));
    bytes.extend(chunk(b"GFID", &group.to_le_bytes()));
    if let Some(child) = child {
        bytes.extend(chunk(b"MODI", &child.to_le_bytes()));
        let mut spawn = [0; 40];
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        bytes.extend(chunk(b"MODD", &spawn));
        let mut set = [0; 32];
        put(&mut set, 24, 1);
        bytes.extend(chunk(b"MODS", &set));
    }
    bytes
}
pub(in crate::collision) fn wmo_group(faces: bool, child: bool, liquid: bool) -> Vec<u8> {
    let mut header = vec![0; 68];
    put(&mut header, 56, 99);
    if liquid {
        put(&mut header, 52, 22);
    }
    if faces {
        header.extend(chunk(b"MOPY", &[8, 0]));
        header.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        header.extend(chunk(
            b"MOVT",
            &words(&[0, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0]),
        ));
    }
    if child {
        header.extend(chunk(b"MODR", &[0, 0]));
    }
    chunk(b"MOGP", &header)
}

// Independent literal schema widths/array counts. No production metadata or
// decoder internals are used to construct these complete regular WDC5 files.
const MAP: &[(u16, u16)] = &[
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 2),
    (8, 1),
    (8, 1),
    (8, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (8, 1),
    (32, 1),
    (16, 1),
    (8, 1),
    (16, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 3),
];
const TYPE: &[(u16, u16)] = &[
    (32, 1),
    (32, 6),
    (32, 1),
    (8, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (16, 1),
    (32, 1),
    (8, 1),
    (8, 1),
    (8, 1),
    (32, 1),
    (8, 6),
    (32, 3),
    (32, 38),
    (32, 4),
    (32, 4),
];
fn offset(fields: &[(u16, u16)], index: usize) -> usize {
    fields[..index]
        .iter()
        .map(|(b, n)| usize::from(b * n / 8))
        .sum()
}
fn size(fields: &[(u16, u16)]) -> usize {
    offset(fields, fields.len())
}
fn table(
    layout: u32,
    fields: &[(u16, u16)],
    rows: &[(u32, Vec<u8>)],
    strings: &[u8],
    copies: &[(u32, u32)],
) -> Vec<u8> {
    let start = 244 + fields.len() * 28;
    let mut bytes = vec![0; start];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, fields.len() as u32),
        (144, size(fields) as u32),
        (148, strings.len() as u32),
        (156, layout),
        (164, u32::MAX),
        (176, fields.len() as u32),
        (180, size(fields) as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
        (212, start as u32),
        (216, rows.len() as u32),
        (220, strings.len() as u32),
        (228, (rows.len() * 4) as u32),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (i, &(width, count)) in fields.iter().enumerate() {
        let primitive = 244 + i * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - width).to_le_bytes());
        bytes[primitive + 2..primitive + 4]
            .copy_from_slice(&(offset(fields, i) as u16).to_le_bytes());
        let col = 244 + fields.len() * 4 + i * 24;
        bytes[col..col + 2].copy_from_slice(&((offset(fields, i) * 8) as u16).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(width * count).to_le_bytes());
    }
    for (_, row) in rows {
        assert_eq!(row.len(), size(fields));
        bytes.extend(row);
    }
    bytes.extend(strings);
    for (id, _) in rows {
        bytes.extend(id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
}
pub(in crate::collision) fn map_db2(values: &[(u32, u32, i16)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut rows = Vec::new();
    let mut strings = Vec::new();
    for (ordinal, &(id, wdt, parent)) in values.iter().enumerate() {
        let mut row = vec![0; size(MAP)];
        for (field, text) in [(0, "Synthetic"), (1, "Name")] {
            let relative =
                (values.len() - ordinal) * size(MAP) + strings.len() - offset(MAP, field);
            put(&mut row, offset(MAP, field), relative as u32);
            strings.extend(text.as_bytes());
            strings.push(0);
        }
        put(&mut row, offset(MAP, 21), wdt);
        for field in [13, 14] {
            let at = offset(MAP, field);
            row[at..at + 2].copy_from_slice(&parent.to_le_bytes());
        }
        rows.push((id, row));
    }
    table(0xD43A_FAC3, MAP, &rows, &strings, copies)
}
fn material_db2(values: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, flags)| {
            let mut row = vec![0; 5];
            put(&mut row, 0, *flags);
            (*id, row)
        })
        .collect();
    table(0x98E5_D7AA, &[(32, 1), (8, 1)], &rows, &[], copies)
}
fn type_db2(values: &[(u32, u8)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, material)| {
            let mut row = vec![0; size(TYPE)];
            row[offset(TYPE, 14)] = *material;
            (*id, row)
        })
        .collect();
    table(0xD1EC_EEC9, TYPE, &rows, &[], copies)
}
pub(in crate::collision) fn files(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    vec![
        (
            gameobjects::TABLE_FILE_DATA_ID,
            None,
            gameobject_db2(&[], &[]),
        ),
        (MAP_FILE_DATA_ID, None, map_db2(maps, &[])),
        (MATERIAL_FILE_DATA_ID, None, material_db2(&[(7, 2)], &[])),
        (TYPE_FILE_DATA_ID, None, type_db2(&[(22, 7)], &[])),
    ]
}
pub(in crate::collision) fn wdt(global: Option<u32>, tile: Option<u32>) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Some(id) = global {
        bytes.extend(wmo_adt(id, 7));
    }
    if let Some(id) = tile {
        let mut mphd = [0; 32];
        put(&mut mphd, 0, 0x200);
        bytes.extend(chunk(b"MPHD", &mphd));
        let mut main = vec![0; 32768];
        put(&mut main, 0, 1);
        bytes.extend(chunk(b"MAIN", &main));
        let mut maid = vec![0; 131072];
        put(&mut maid, 4, id);
        bytes.extend(chunk(b"MAID", &maid));
    }
    if bytes.is_empty() {
        bytes.extend(chunk(b"MVER", &99u32.to_le_bytes()));
    }
    bytes
}
pub(in crate::collision) fn wmo_adt(id: u32, unique: u32) -> Vec<u8> {
    let mut row = [0; 64];
    put(&mut row, 0, id);
    put(&mut row, 4, unique);
    row[56..58].copy_from_slice(&8u16.to_le_bytes());
    chunk(b"MODF", &row)
}
pub(in crate::collision) fn m2_adt(id: u32, unique: u32) -> Vec<u8> {
    let mut row = [0; 36];
    put(&mut row, 0, id);
    put(&mut row, 4, unique);
    row[32..34].copy_from_slice(&1024u16.to_le_bytes());
    row[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &row)
}
#[derive(Default)]
struct Stage {
    placements: Vec<(run::Context, Vec<u8>)>,
    fail: bool,
}
impl run::Sink for Stage {
    fn model(&mut self, _: &super::super::Model) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        Ok(())
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}
fn perform(f: &CollisionFixture, limits: Limits) -> Result<Completed, Error> {
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    extract(startup, &mut Stage::default(), limits)
}

#[test]
fn complete_empty_world_preserves_lists_and_releases_all_own_reservations() {
    let f = CollisionFixture::new(&files(&[]));
    let out = perform(&f, Limits::default()).unwrap();
    assert_eq!(out.startup().extraction().facts().reads, 4);
    assert_eq!(
        out.tables().each_ref().map(|t| t.source().file_data_id()),
        [MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID]
    );
    assert_eq!(out.map_stats().active_maps, 0);
    assert_eq!(out.lookup_stats().materials, 1);
    assert_eq!(out.lookup_stats().types, 1);
    assert_eq!(out.startup().data().lists().temp_bytes(), b"VMAP04E\0");
}
#[test]
fn cached_startup_wmo_keeps_empty_lookup_new_wmo_uses_loaded_visual_only() {
    let mut fs = files(&[(1, 101, -1)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, wmo_root(11, None)),
        (11, None, wmo_group(true, false, true)),
        (20, None, wmo_root(21, None)),
        (21, None, wmo_group(true, false, true)),
        (101, None, wdt(Some(10), Some(201))),
        (201, None, wmo_adt(20, 8)),
    ]);
    let f = CollisionFixture::new(&fs);
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let cached = startup
        .model(match startup.data().outcomes()[0].outcome() {
            gameobjects::Outcome::Collision(h) => h,
            _ => panic!(),
        })
        .unwrap()
        .raw()
        .unwrap()
        .to_vec();
    let mut stage = Stage::default();
    let out = extract(startup, &mut stage, Limits::default()).unwrap();
    let models = out.startup().extraction().models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].raw().unwrap(), cached);
    let flags: Vec<_> = models
        .iter()
        .map(|m| {
            raw::RawWmo::parse(m.raw().unwrap(), raw::ConversionLimits::default())
                .unwrap()
                .groups()
                .next()
                .unwrap()
                .liquid_flags()
        })
        .collect();
    assert_eq!(flags, [2, 0]);
    assert_eq!(out.startup().extraction().facts().reads, 10);
    assert_eq!(stage.placements.len(), 2);
    assert_eq!(out.report().run.model_callbacks, 2);
}
#[test]
fn cached_m2_repeated_tiles_preserve_single_model_and_shared_ids() {
    let mut fs = files(&[(1, 101, -1), (2, 102, 1)]);
    fs[0].2 = gameobject_db2(&[(1, 10), (2, 10)], &[]);
    fs.extend([
        (10, None, m2(true)),
        (101, None, wdt(None, Some(201))),
        (102, None, wdt(None, None)),
        (201, None, m2_adt(10, 9)),
    ]);
    let f = CollisionFixture::new(&fs);
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut stage = Stage::default();
    let out = extract(startup, &mut stage, Limits::default()).unwrap();
    assert_eq!(out.startup().extraction().facts().models, 1);
    assert_eq!(out.report().run.replayed_tiles, 1);
    let tiles: Vec<_> = stage
        .placements
        .iter()
        .filter(|(c, _)| matches!(c.kind, run::Kind::Tile { .. }))
        .collect();
    assert_eq!(tiles.len(), 2);
    let ids: Vec<_> = tiles
        .iter()
        .map(|(_, b)| {
            placement::DirBin::parse(b, placement::Limits::default())
                .unwrap()
                .records()
                .next()
                .unwrap()
                .id()
        })
        .collect();
    assert_eq!(ids[0], ids[1]);
}
#[test]
fn liquid_union_matches_accepted_replay_duplicates_missing_zero_self_chains_overwrites() {
    let copies = [
        (3, 1),
        (4, 3),
        (9, 99),
        (0, 0),
        (8, 0),
        (1, 4),
        (99, 2),
        (7, 7),
    ];
    let bytes = material_db2(&[(1, 1), (1, 2), (2, u32::MAX)], &copies);
    let snapshot = source::read_liquid_materials(&bytes).unwrap();
    let expected = source::read_liquid_materials(&bytes)
        .unwrap()
        .materialize()
        .unwrap();
    let f = CollisionFixture::new(&files(&[]));
    let mut store = f.open(false);
    let mut startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let rows = replay(&mut startup, &snapshot, Limits::default(), 0).unwrap();
    assert_eq!(
        rows.iter()
            .copied()
            .collect::<std::collections::BTreeMap<_, _>>(),
        expected
    );
    assert_eq!(rows.iter().find(|r| r.0 == 9).unwrap().1.flags_bits, 0);
    let bytes = capacity(&rows).unwrap();
    drop(rows);
    startup.collector_mut().release_external(bytes).unwrap();
    startup.finish().unwrap();
}
#[test]
fn copied_map_child_uses_saved_direct_prefix_and_retains_inactive_rows() {
    let mut fs = files(&[]);
    fs[1].2 = map_db2(&[(1, 0, -1)], &[(2, 1)]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.map_stats().rows, 2);
    assert_eq!(out.map_stats().direct_rows, 1);
    assert_eq!(out.map_stats().active_maps, 0);
    assert_eq!(out.tables()[0].copy_records, 1);
}
#[test]
fn parent_existing_only_as_copy_is_not_authenticated_direct_parent() {
    let mut fs = files(&[]);
    fs[1].2 = map_db2(&[(1, 0, -1), (3, 0, 2)], &[(2, 1)]);
    assert_eq!(
        perform(&CollisionFixture::new(&fs), Limits::default()).err(),
        Some(Error::Plan(plan::Error::CopiedParent))
    );
}
#[test]
fn duplicate_map_id_is_terminal_instead_of_silent_alias() {
    let mut fs = files(&[]);
    fs[1].2 = map_db2(&[(1, 0, -1), (1, 0, -1)], &[]);
    assert_eq!(
        perform(&CollisionFixture::new(&fs), Limits::default()).err(),
        Some(Error::Plan(plan::Error::DuplicateId))
    );
}
#[test]
fn missing_each_required_table_has_no_completed_fallback() {
    for id in [MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID] {
        let mut fs = files(&[]);
        fs.retain(|r| r.0 != id);
        assert!(matches!(
            perform(&CollisionFixture::new(&fs), Limits::default()),
            Err(Error::Collision(_))
        ));
    }
}
#[test]
fn malformed_late_type_and_map_null_pointer_never_reach_runner() {
    for id in [MAP_FILE_DATA_ID, TYPE_FILE_DATA_ID] {
        let mut fs = files(&[(1, 101, -1)]);
        if id == MAP_FILE_DATA_ID {
            let start = 244 + MAP.len() * 28;
            put(&mut fs[1].2, start, 0);
        } else {
            fs[3].2.push(0);
        }
        assert!(matches!(
            perform(&CollisionFixture::new(&fs), Limits::default()),
            Err(Error::Db2(_))
        ));
    }
}
#[test]
fn missing_wdt_is_fatal_after_complete_tables() {
    let f = CollisionFixture::new(&files(&[(1, 101, -1)]));
    assert!(matches!(perform(&f, Limits::default()), Err(Error::Run(_))));
}
#[test]
fn limits_only_reduce_and_table_caps_fail_whole_operation() {
    let f = CollisionFixture::new(&files(&[]));
    for limits in [
        Limits {
            max_db2_bytes: MAX_DB2_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_lookup_rows: MAX_LOOKUP_ROWS + 1,
            ..Limits::default()
        },
        Limits {
            max_table_memory_bytes: MAX_TABLE_MEMORY_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(perform(&f, limits).err(), Some(Error::InvalidLimits));
    }
    for limits in [
        Limits {
            max_db2_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_decode_allowance_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_lookup_rows: 0,
            ..Limits::default()
        },
        Limits {
            max_table_memory_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_direct_rows: 0,
            ..Limits::default()
        },
    ] {
        assert!(perform(&f, limits).is_err());
    }
}
#[test]
fn unique_lookup_cap_counts_missing_rhs_even_when_destination_exists() {
    let mut fs = files(&[]);
    fs[2].2 = material_db2(&[(7, 2)], &[(7, 99)]);
    let f = CollisionFixture::new(&fs);
    assert_eq!(
        perform(
            &f,
            Limits {
                max_lookup_rows: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let out = perform(
        &f,
        Limits {
            max_lookup_rows: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(out.lookup_stats().materials, 2);
}
#[test]
fn copy_and_map_row_reductions_do_not_return_partial_results() {
    let mut fs = files(&[]);
    fs[1].2 = map_db2(&[(1, 0, -1)], &[(2, 1)]);
    let f = CollisionFixture::new(&fs);
    assert_eq!(
        perform(
            &f,
            Limits {
                max_copy_records: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        perform(
            &f,
            Limits {
                maps: plan::Limits {
                    max_rows: 1,
                    ..plan::Limits::default()
                },
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn collector_cumulative_reads_include_all_four_db2_tables() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = f.open(false);
    let startup = gameobjects::extract_startup_fdid(
        &mut store,
        gameobjects::Limits {
            collision: super::super::Limits {
                max_reads: 3,
                ..super::super::Limits::default()
            },
            ..gameobjects::Limits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        extract(startup, &mut Stage::default(), Limits::default()),
        Err(Error::Collision(_))
    ));
}
#[test]
fn sink_failure_and_final_source_mutation_cannot_produce_completed() {
    let mut fs = files(&[(1, 101, -1)]);
    fs.push((101, None, wdt(None, None)));
    let f = CollisionFixture::new(&fs);
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    assert!(matches!(
        extract(
            startup,
            &mut Stage {
                fail: true,
                ..Stage::default()
            },
            Limits::default()
        ),
        Err(Error::Run(run::Error::Sink))
    ));
    struct Mutate<'a>(&'a CollisionFixture);
    impl run::Sink for Mutate<'_> {
        fn model(&mut self, _: &super::super::Model) -> Result<(), run::SinkError> {
            Ok(())
        }
        fn placements(&mut self, _: run::Context, _: &[u8]) -> Result<(), run::SinkError> {
            self.0.change_metadata();
            Ok(())
        }
    }
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    assert!(extract(startup, &mut Mutate(&f), Limits::default()).is_err());
}

#[test]
fn owned_topology_preserves_physical_inactive_and_copy_rows_with_direct_prefix() {
    let mut fs = files(&[]);
    fs[1].2 = map_db2(&[(9, 0, 4), (4, 0, -1), (7, 0, 9)], &[(10, 9)]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    let graph = out.topology();
    assert_eq!(graph.direct_count(), 3);
    assert_eq!(
        graph
            .all()
            .iter()
            .map(TopologyEntry::id)
            .collect::<Vec<_>>(),
        [9, 4, 7, 10]
    );
    assert_eq!(
        graph
            .all()
            .iter()
            .map(TopologyEntry::physical_ordinal)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(
        graph
            .all()
            .iter()
            .map(TopologyEntry::depth)
            .collect::<Vec<_>>(),
        [1, 0, 2, 1]
    );
    assert!(graph.all().iter().all(|entry| !entry.active()));
    assert_eq!(graph.get(10).unwrap().parent_map_id(), Some(4));
    assert_eq!(graph.get(7).unwrap().parent_map_id(), Some(9));
    assert!(graph.get(4).unwrap().is_parent());
    assert!(graph.get(9).unwrap().is_parent());
    assert!(!graph.get(10).unwrap().is_parent());
    assert!(graph.get(0).is_none());
    assert_eq!(
        graph.resident_bytes(),
        capacity(&graph.rows).unwrap() + capacity(&graph.by_id).unwrap()
    );
    assert_eq!(out.report().run.maps, 0);
}
#[test]
fn topology_preserves_active_high_bit_wdt_and_all_graph_metadata() {
    let id = 0x8000_0065;
    let mut fs = files(&[(8, id, -1)]);
    fs.push((id, None, wdt(None, None)));
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    let entry = out.topology().get(8).unwrap();
    assert!(entry.active());
    assert_eq!(entry.wdt_file_data_id(), id);
    assert_eq!(entry.parent_map_id(), None);
    assert_eq!(entry.physical_ordinal(), 0);
    assert_eq!(entry.depth(), 0);
    assert_eq!(out.report().run.wdt_files, 1);
}
#[test]
fn completed_resident_is_final_extraction_plus_data_plus_topology_not_startup_snapshot() {
    let mut fs = files(&[(1, 101, -1)]);
    fs.extend([
        (101, None, wdt(None, Some(201))),
        (201, None, m2_adt(10, 7)),
        (10, None, m2(true)),
    ]);
    let fixture = CollisionFixture::new(&fs);
    let mut store = fixture.open(false);
    let store_resident = store.stats().resident_bytes;
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let initial = startup.data().stats().collector_retained_bytes;
    let out = extract(startup, &mut Stage::default(), Limits::default()).unwrap();
    assert_eq!(
        out.resident_bytes(),
        out.startup().extraction().resident_bytes()
            + out.startup().data().stats().resident_bytes
            + out.topology().resident_bytes()
    );
    assert!(out.resident_bytes() > initial);
    assert_eq!(out.store_resident_bytes(), store_resident);
    assert_eq!(out.startup().extraction().models().len(), 1);
    assert_eq!(out.startup().extraction().facts().reads, 7);
}
#[test]
fn topology_snapshot_preflights_capacity_and_keeps_charge_until_transfer() {
    let rows = vec![source::DirectRecord {
        id: 2,
        fields: source::VMapFields {
            wdt_file_data_id: 0,
            parent_map_id: -1,
            name: String::new(),
            directory: String::new(),
        },
    }];
    let maps = plan::Maps::from_materialized(&rows, 1, plan::Limits::default()).unwrap();
    let fixture = CollisionFixture::new(&files(&[]));
    let mut store = fixture.open(false);
    let mut startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let before = startup.collector_mut().retained_bytes();
    let requested = size_of::<TopologyEntry>() + size_of::<usize>();
    assert_eq!(
        Topology::snapshot(&maps, startup.collector_mut(), requested - 1).err(),
        Some(Error::Limit)
    );
    assert_eq!(startup.collector_mut().retained_bytes(), before);
    let topology = Topology::snapshot(&maps, startup.collector_mut(), requested).unwrap();
    let owned = topology.resident_bytes();
    assert_eq!(startup.collector_mut().retained_bytes(), before + owned);
    drop(topology);
    startup.collector_mut().release_external(owned).unwrap();
    startup.finish().unwrap();
}
#[test]
fn topology_overlap_participates_in_table_component_cap() {
    let fixture = CollisionFixture::new(&files(&[(9, 0, -1), (8, 0, 9)]));
    let out = perform(&fixture, Limits::default()).unwrap();
    let tables = out
        .tables()
        .iter()
        .map(|facts| facts.retained_bytes)
        .sum::<usize>();
    let graph = out.map_stats().resident_bytes;
    let topology = out.topology().resident_bytes();
    assert!(topology > 0);
    let ceiling = tables + graph + topology;
    let exact = perform(
        &fixture,
        Limits {
            max_table_memory_bytes: ceiling,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(exact.topology().resident_bytes(), topology);
    assert_eq!(
        perform(
            &fixture,
            Limits {
                max_table_memory_bytes: ceiling - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
