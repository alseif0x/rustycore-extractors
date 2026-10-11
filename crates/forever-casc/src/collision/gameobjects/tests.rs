// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic CASC-to-startup integration fixtures.
use super::*;
use crate::local_store::{self, tests::CollisionFixture};
use forever_vmap::{
    gameobject_models::{Dtree, TempList},
    wmo::raw,
};

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
fn db2(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
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
fn m2(faces: bool) -> Vec<u8> {
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
fn wmo_root(group: u32, child: Option<u32>) -> Vec<u8> {
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
fn wmo_group(faces: bool, child: bool, liquid: bool) -> Vec<u8> {
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
fn temp_rows(data: &Data) -> Vec<(u32, Vec<u8>)> {
    TempList::parse(
        data.lists().temp_bytes(),
        gameobject_models::Limits::default(),
    )
    .unwrap()
    .rows()
    .map(|row| (row.display_id(), row.name().to_vec()))
    .collect()
}
fn error(fixture: &CollisionFixture, limits: Limits) -> Error {
    let mut store = fixture.open(false);
    extract_startup_fdid(&mut store, limits).err().unwrap()
}

#[test]
fn empty_table_builds_both_complete_lists_and_counts_its_read() {
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[], &[]))]);
    let mut store = fixture.open(false);
    let startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    assert_eq!(startup.data().lists().temp_bytes(), b"VMAP04E\0");
    assert_eq!(startup.data().lists().dtree_bytes(), b"VMAP_4.E");
    assert!(startup.data().outcomes().is_empty());
    assert_eq!(startup.data().source().file_data_id(), TABLE_FILE_DATA_ID);
    assert_eq!(startup.data().startup_facts().reads, 1);
    assert_eq!(startup.data().startup_facts().source_bytes, 496);
    let complete = startup.finish().unwrap();
    assert!(complete.extraction().models().is_empty());
    assert_eq!(complete.extraction().facts().reads, 1);
}

#[test]
fn zero_texture_no_collision_duplicates_and_order_are_explicit() {
    let table = db2(&[(7, 0), (9, 10), (9, 10), (8, 11), (3, 12)], &[]);
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, table.clone()),
        (10, None, m2(true)),
        (11, None, b"BLP2synthetic".to_vec()),
        (12, None, m2(false)),
    ]);
    let mut store = fixture.open(false);
    let startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    let outcomes = startup.data().outcomes();
    assert_eq!(
        outcomes.iter().map(RowOutcome::ordinal).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        outcomes
            .iter()
            .map(RowOutcome::display_id)
            .collect::<Vec<_>>(),
        [7, 9, 9, 8, 3]
    );
    assert_eq!(outcomes[0].outcome(), Outcome::ZeroFileDataId);
    assert_eq!(outcomes[1].outcome(), outcomes[2].outcome());
    assert!(matches!(outcomes[1].outcome(), Outcome::Collision(_)));
    assert_eq!(outcomes[3].outcome(), Outcome::Texture);
    assert!(matches!(outcomes[4].outcome(), Outcome::M2NoCollision(_)));
    assert_eq!(
        temp_rows(startup.data()),
        [
            (9, b"FILE0000000A.xxx".to_vec()),
            (9, b"FILE0000000A.xxx".to_vec())
        ]
    );
    let stats = startup.data().stats();
    assert_eq!(
        (
            stats.candidate_requests,
            stats.zero_fdids,
            stats.textures,
            stats.m2_no_collision,
            stats.collision_rows
        ),
        (4, 1, 1, 1, 2)
    );
    let complete = startup.finish().unwrap();
    assert_eq!(complete.extraction().facts().reads, 4);
    assert_eq!(
        complete.extraction().facts().source_bytes,
        table.len() + m2(true).len() + 13 + m2(false).len()
    );
    let rows = Dtree::parse(
        complete.data().lists().dtree_bytes(),
        gameobject_models::Limits::default(),
    )
    .unwrap();
    assert_eq!(rows.row_count(), 2);
    assert_eq!(
        rows.rows().map(|row| row.display_id()).collect::<Vec<_>>(),
        [9, 9]
    );
}

#[test]
fn source_copies_never_generate_acquisitions_or_list_rows() {
    let fixture = CollisionFixture::new(&[(
        TABLE_FILE_DATA_ID,
        None,
        db2(&[(7, 0)], &[(88, 7), (u32::MAX, 1234), (7, 7)]),
    )]);
    let mut store = fixture.open(false);
    let complete = extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(complete.data().stats().copy_records, 3);
    assert_eq!(complete.data().outcomes().len(), 1);
    assert!(temp_rows(complete.data()).is_empty());
    assert_eq!(complete.extraction().facts().reads, 1);
}

#[test]
fn wmo_collision_and_its_later_direct_doodad_reuse_one_cache() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 20)], &[])),
        (10, None, wmo_root(30, Some(20))),
        (20, None, m2(true)),
        (30, None, wmo_group(true, true, false)),
    ]);
    let mut store = fixture.open(false);
    let complete = extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(complete.extraction().facts().reads, 4);
    assert_eq!(complete.extraction().models().len(), 2);
    assert_eq!(
        temp_rows(complete.data()),
        [
            (7, b"FILE0000000A.xxx".to_vec()),
            (8, b"FILE00000014.xxx".to_vec())
        ]
    );
    assert_eq!(complete.data().lists().stats().wmo_rows, 1);
    assert_eq!(complete.data().lists().stats().m2_rows, 1);
}

#[test]
fn doodad_only_wmo_is_retained_but_omitted_from_both_lists() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 10)], &[])),
        (10, None, wmo_root(30, Some(20))),
        (20, None, m2(true)),
        (30, None, wmo_group(false, true, false)),
    ]);
    let mut store = fixture.open(false);
    let complete = extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(complete.extraction().facts().reads, 4);
    assert_eq!(complete.data().stats().wmo_doodad_only, 2);
    assert_eq!(
        complete.data().outcomes()[0].outcome(),
        complete.data().outcomes()[1].outcome()
    );
    assert!(matches!(
        complete.data().outcomes()[0].outcome(),
        Outcome::WmoDoodadOnly(_)
    ));
    assert!(temp_rows(complete.data()).is_empty());
    assert_eq!(complete.extraction().models().len(), 2);
}

#[test]
fn md21_classification_and_whole_source_scanner_both_execute() {
    let mut wrapped = b"MD21synthetic-envelope".to_vec();
    wrapped.extend(m2(true));
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10)], &[])),
        (10, None, wrapped),
    ]);
    let mut store = fixture.open(false);
    let result = extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(result.data().stats().collision_rows, 1);
    assert_eq!(
        result.extraction().models()[0]
            .source()
            .unwrap()
            .first_magic(),
        Some(*b"MD21")
    );
}

#[test]
fn missing_table_and_missing_candidate_are_terminal() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    assert_eq!(
        error(&fixture, Limits::default()),
        Error::Collision(super::super::Error::Store(local_store::Error::MissingFile))
    );
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[(7, 999)], &[]))]);
    assert_eq!(
        error(&fixture, Limits::default()),
        Error::Collision(super::super::Error::Store(local_store::Error::MissingFile))
    );
}

#[test]
fn complete_db2_validation_precedes_candidate_reads() {
    let mut table = db2(&[(7, 999)], &[]);
    table.pop();
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, table)]);
    assert!(matches!(error(&fixture, Limits::default()), Error::Db2(_)));
}

#[test]
fn late_unknown_and_malformed_models_never_return_startup() {
    for malformed in [b"WHAT".to_vec(), b"MD20".to_vec()] {
        let fixture = CollisionFixture::new(&[
            (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 11)], &[])),
            (10, None, m2(true)),
            (11, None, malformed),
        ]);
        assert!(matches!(
            error(&fixture, Limits::default()),
            Error::Collision(_)
        ));
    }
}

#[test]
fn max_fdid_is_preserved_and_fails_acquisition_instead_of_zero_omission() {
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[(7, u32::MAX)], &[]))]);
    assert!(matches!(
        error(&fixture, Limits::default()),
        Error::Collision(_)
    ));
}

#[test]
fn row_copy_and_nonzero_request_limits_count_before_iteration() {
    let fixture =
        CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[(7, 0), (8, 0)], &[(9, 7)]))]);
    assert_eq!(
        error(
            &fixture,
            Limits {
                max_direct_rows: 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert_eq!(
        error(
            &fixture,
            Limits {
                max_copy_records: 0,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    let fixture =
        CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[(7, 999), (8, 999)], &[]))]);
    assert_eq!(
        error(
            &fixture,
            Limits {
                max_candidate_requests: 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
}

#[test]
fn duplicate_cache_hits_still_consume_candidate_and_collector_request_caps() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 10)], &[])),
        (10, None, m2(true)),
    ]);
    assert_eq!(
        error(
            &fixture,
            Limits {
                max_candidate_requests: 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    assert!(matches!(
        error(
            &fixture,
            Limits {
                collision: super::super::Limits {
                    max_requests: 1,
                    ..super::super::Limits::default()
                },
                ..Limits::default()
            }
        ),
        Error::Collision(super::super::Error::Limit)
    ));
}

#[test]
fn decoder_allowance_is_admitted_before_decode_and_includes_live_source() {
    let table = db2(&[], &[]);
    let allowance = 2 * 1024 * 1024 + 32 * table.len();
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, table.clone())]);
    assert_eq!(
        error(
            &fixture,
            Limits {
                max_decode_allowance_bytes: allowance - 1,
                ..Limits::default()
            }
        ),
        Error::Limit
    );
    let mut store = fixture.open(false);
    let memory_cap = store.stats().resident_bytes + table.len() + allowance - 1;
    let result = extract_startup_fdid(
        &mut store,
        Limits {
            collision: super::super::Limits {
                max_memory_bytes: memory_cap,
                ..super::super::Limits::default()
            },
            ..Limits::default()
        },
    );
    assert!(matches!(
        result.err(),
        Some(Error::Collision(super::super::Error::Limit))
    ));
}

#[test]
fn db2_source_read_bytes_and_calls_consume_global_collector_limits() {
    let table = db2(&[(7, 10)], &[]);
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, table.clone()),
        (10, None, m2(true)),
    ]);
    for collision in [
        super::super::Limits {
            max_reads: 1,
            ..super::super::Limits::default()
        },
        super::super::Limits {
            max_source_bytes: table.len(),
            ..super::super::Limits::default()
        },
    ] {
        assert_eq!(
            error(
                &fixture,
                Limits {
                    collision,
                    ..Limits::default()
                }
            ),
            Error::Collision(super::super::Error::Limit)
        );
    }
    assert!(matches!(
        error(
            &fixture,
            Limits {
                max_db2_bytes: table.len() - 1,
                ..Limits::default()
            }
        ),
        Error::Collision(_)
    ));
}

#[test]
fn list_limits_cover_repeated_raw_and_vertex_visits() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 10)], &[])),
        (10, None, m2(true)),
    ]);
    for lists in [
        gameobject_models::Limits {
            max_rows: 1,
            ..gameobject_models::Limits::default()
        },
        gameobject_models::Limits {
            max_raw_visit_bytes: 1,
            ..gameobject_models::Limits::default()
        },
        gameobject_models::Limits {
            max_vertex_visits: 5,
            ..gameobject_models::Limits::default()
        },
        gameobject_models::Limits {
            max_list_bytes: 8,
            ..gameobject_models::Limits::default()
        },
        gameobject_models::Limits {
            max_memory_bytes: 0,
            ..gameobject_models::Limits::default()
        },
    ] {
        assert!(matches!(
            error(
                &fixture,
                Limits {
                    lists,
                    ..Limits::default()
                }
            ),
            Error::Lists(gameobject_models::Error::Limit)
        ));
    }
}

#[test]
fn limits_only_reduce_and_are_validated_before_store_guards() {
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[], &[]))]);
    let mut store = fixture.open(false);
    fixture.change_metadata();
    assert_eq!(
        extract_startup_fdid(
            &mut store,
            Limits {
                max_direct_rows: MAX_DIRECT_ROWS + 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}

#[test]
fn changed_inputs_reject_final_transfer_after_successful_startup() {
    let fixture = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[], &[]))]);
    let mut store = fixture.open(false);
    let startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    fixture.change_metadata();
    assert_eq!(
        startup.finish().err(),
        Some(Error::Collision(super::super::Error::Store(
            local_store::Error::InputChanged
        )))
    );
}

struct VisualOnly;
impl LiquidLookup for VisualOnly {
    fn material_id(&self, id: u32) -> Option<u8> {
        (id == 22).then_some(7)
    }
    fn material_flags(&self, id: u8) -> Option<u32> {
        (id == 7).then_some(2)
    }
}
fn liquid_flags(model: &Model) -> u32 {
    raw::RawWmo::parse(model.raw().unwrap(), raw::ConversionLimits::default())
        .unwrap()
        .groups()
        .next()
        .unwrap()
        .liquid_flags()
}

#[test]
fn lookup_transition_preserves_cached_startup_geometry_and_handles() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10)], &[])),
        (10, None, wmo_root(30, None)),
        (30, None, wmo_group(true, false, true)),
        (11, None, wmo_root(31, None)),
        (31, None, wmo_group(true, false, true)),
    ]);
    let mut store = fixture.open(false);
    let startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    let Outcome::Collision(original) = startup.data().outcomes()[0].outcome() else {
        panic!()
    };
    assert_eq!(liquid_flags(startup.model(original).unwrap()), 2);
    let old_bytes = startup.model(original).unwrap().raw().unwrap().to_vec();
    let old_lists = startup.data().lists().dtree_bytes().to_vec();
    let mut transitioned = startup.with_lookup(&VisualOnly).unwrap();
    assert_eq!(
        transitioned
            .collector_mut()
            .extract_gameobject_fdid(10)
            .unwrap(),
        GameObjectOutcome::Model(original)
    );
    let later = transitioned
        .collector_mut()
        .extract(super::super::ModelRequest::Wmo(
            super::super::InputRef::FileDataId(11),
        ))
        .unwrap();
    assert_eq!(
        transitioned.model(original).unwrap().raw().unwrap(),
        old_bytes
    );
    assert_eq!(liquid_flags(transitioned.model(later).unwrap()), 0);
    assert_eq!(transitioned.data().lists().dtree_bytes(), old_lists);
    let complete = transitioned.finish().unwrap();
    assert_eq!(complete.extraction().facts().reads, 5);
    assert_eq!(
        complete.extraction().requests(),
        &[original, original, later]
    );
    assert_eq!(complete.data().startup_facts().reads, 3);
}

#[test]
fn failed_later_phase_poison_survives_context_and_lookup_transfer() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10)], &[])),
        (10, None, m2(true)),
    ]);
    let mut store = fixture.open(false);
    let mut startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    assert!(
        startup
            .collector_mut()
            .extract_gameobject_fdid(999)
            .is_err()
    );
    assert!(startup.with_lookup(&VisualOnly).is_err());
}

#[test]
fn retained_capacity_charge_matches_data_and_releases_on_finish() {
    let fixture = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(7, 10), (8, 0)], &[])),
        (10, None, m2(true)),
    ]);
    let mut store = fixture.open(false);
    let startup = extract_startup_fdid(&mut store, Limits::default()).unwrap();
    assert_eq!(
        startup.data.stats.resident_bytes,
        startup.data.outcomes.capacity() * size_of::<RowOutcome>()
            + startup.data.lists.stats().resident_bytes
    );
    assert!(startup.collector.retained_bytes() >= startup.data.stats.resident_bytes);
    assert!(startup.finish().is_ok());
}
