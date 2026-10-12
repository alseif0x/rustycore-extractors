// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic complete spooled startup evidence.
use super::*;
use crate::{
    collision::{
        self, gameobjects as resident,
        spool::{Limits as SpoolLimits, ProtectedInputs},
    },
    local_store::tests::CollisionFixture,
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
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

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    path: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-spooled-startup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let path = base.join("spool");
        Self { base, path }
    }
    fn spool(&self, f: &CollisionFixture, limits: SpoolLimits) -> Spool {
        Spool::create(
            &self.path,
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            limits,
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap()
    }
}
fn outcomes(d: &Data) -> Vec<(usize, u32, u32, Outcome)> {
    d.outcomes()
        .iter()
        .map(|r| (r.ordinal(), r.display_id(), r.file_data_id(), r.outcome()))
        .collect()
}
fn compare(a: &Data, b: &Data) {
    assert_eq!(outcomes(a), outcomes(b));
    assert_eq!(a.lists().temp_bytes(), b.lists().temp_bytes());
    assert_eq!(a.lists().dtree_bytes(), b.lists().dtree_bytes());
    assert_eq!(a.startup_facts(), b.startup_facts());
    assert_eq!(
        (
            a.stats.direct_rows,
            a.stats.copy_records,
            a.stats.candidate_requests,
            a.stats.collision_rows,
            a.stats.textures,
            a.stats.m2_no_collision,
            a.stats.wmo_doodad_only
        ),
        (
            b.stats.direct_rows,
            b.stats.copy_records,
            b.stats.candidate_requests,
            b.stats.collision_rows,
            b.stats.textures,
            b.stats.m2_no_collision,
            b.stats.wmo_doodad_only
        )
    );
}
fn error(f: &CollisionFixture, limits: Limits) -> Error {
    let p = Private::new();
    let mut store = f.open(false);
    extract_startup_fdid(&mut store, limits, p.spool(f, SpoolLimits::default()))
        .err()
        .unwrap()
}
#[test]
fn empty_rows_keep_two_headers_and_no_fake_reload() {
    let f = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, db2(&[], &[]))]);
    let p = Private::new();
    let mut store = f.open(false);
    let context = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_records: 0,
                max_reloads: 0,
                max_reload_bytes: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let data = context.data().unwrap();
    assert_eq!(data.stats.resident_bytes, 16);
    assert_eq!(data.lists().temp_bytes(), gameobject_models::TEMP_MAGIC);
    assert_eq!(data.lists().dtree_bytes(), gameobject_models::DTREE_MAGIC);
    assert_eq!(data.startup_facts().reads, 1);
    let completed = context.finish().unwrap();
    assert_eq!(completed.extraction().stats().unwrap().spool.reloads, 0);
}
#[test]
fn physical_order_duplicates_omissions_and_copies_equal_resident() {
    let f = CollisionFixture::new(&[
        (
            TABLE_FILE_DATA_ID,
            None,
            db2(
                &[
                    (7, 0),
                    (8, 10),
                    (9, 20),
                    (8, 10),
                    (11, 30),
                    (12, 30),
                    (13, 40),
                ],
                &[(100, 8)],
            ),
        ),
        (10, None, m2(true)),
        (20, None, m2(false)),
        (30, None, b"BLP2synthetic".to_vec()),
        (40, None, m2(true)),
    ]);
    let mut store = f.open(false);
    let old = resident::extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    let p = Private::new();
    let mut store = f.open(false);
    let new = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap()
    .finish()
    .unwrap();
    compare(old.data(), new.data().unwrap());
    let stats = new.extraction().stats().unwrap().spool;
    assert_eq!(stats.reloads, 3);
    assert_eq!(new.data().unwrap().stats.copy_records, 1);
    assert_eq!(new.data().unwrap().stats.candidate_requests, 6);
}
#[test]
fn wmo_and_shared_child_have_exact_bytes_facts_and_one_reload_per_list_row() {
    let f = CollisionFixture::new(&[
        (
            TABLE_FILE_DATA_ID,
            None,
            db2(&[(1, 10), (2, 20), (3, 10)], &[]),
        ),
        (10, None, wmo_root(30, Some(20))),
        (20, None, m2(true)),
        (30, None, wmo_group(true, true, false)),
    ]);
    let mut store = f.open(false);
    let old = resident::extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    let p = Private::new();
    let mut store = f.open(false);
    let new = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap()
    .finish()
    .unwrap();
    compare(old.data(), new.data().unwrap());
    assert_eq!(new.extraction().stats().unwrap().spool.reloads, 3);
    assert_eq!(new.data().unwrap().startup_facts().dependency_requests, 1);
}
#[test]
fn doodadonly_omission_does_not_reload_root_or_child_for_lists() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
        (10, None, wmo_root(30, Some(20))),
        (20, None, m2(true)),
        (30, None, wmo_group(false, true, false)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let new = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_reloads: 0,
                max_reload_bytes: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert_eq!(new.data().unwrap().stats.wmo_doodad_only, 1);
    assert_eq!(new.data().unwrap().stats.collision_rows, 0);
    assert_eq!(new.extraction().stats().unwrap().spool.reloads, 0);
}
#[test]
fn malformed_complete_table_rejects_before_candidate_reads_and_keeps_db2_cause() {
    let mut table = db2(&[(1, 10)], &[]);
    table.push(0);
    let f = CollisionFixture::new(&[(TABLE_FILE_DATA_ID, None, table), (10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = SpoolingExtractor::new(
        &mut store,
        &EMPTY_LOOKUP,
        Limits::default().collision,
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let result = extract_data(&mut c, Limits::default());
    assert!(matches!(
        result.as_ref().err().unwrap().cause(),
        Cause::Startup(resident::Error::Db2(_))
    ));
    assert_eq!(c.facts().unwrap().reads, 1);
    let e = c
        .finish_external_operation(result)
        .map_err(bridge)
        .err()
        .unwrap();
    assert!(matches!(e.cause(), Cause::Startup(resident::Error::Db2(_))));
    assert!(c.facts().is_err());
}
#[test]
fn missing_table_or_model_has_no_prefix_or_fallback() {
    for f in [
        CollisionFixture::new(&[(10, None, m2(true))]),
        CollisionFixture::new(&[
            (TABLE_FILE_DATA_ID, None, db2(&[(1, 10), (2, 20)], &[])),
            (10, None, m2(true)),
        ]),
    ] {
        assert!(matches!(
            error(&f, Limits::default()).cause(),
            Cause::Collector(_)
        ));
    }
}
#[test]
fn row_copy_candidate_and_source_work_limits_remain_downward() {
    let table = db2(&[(1, 10), (2, 10)], &[(5, 1)]);
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, table.clone()),
        (10, None, m2(true)),
    ]);
    for limits in [
        Limits {
            max_direct_rows: 1,
            ..Limits::default()
        },
        Limits {
            max_copy_records: 0,
            ..Limits::default()
        },
        Limits {
            max_candidate_requests: 1,
            ..Limits::default()
        },
        Limits {
            collision: collision::Limits {
                max_reads: 1,
                ..collision::Limits::default()
            },
            ..Limits::default()
        },
        Limits {
            collision: collision::Limits {
                max_source_bytes: table.len(),
                ..collision::Limits::default()
            },
            ..Limits::default()
        },
    ] {
        assert!(error(&f, limits).code().contains("limit"));
    }
}
#[test]
fn whole_list_preflight_limits_reject_before_first_reload() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10), (2, 10)], &[])),
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
            max_raw_bytes: 1,
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
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = SpoolingExtractor::new(
            &mut store,
            &EMPTY_LOOKUP,
            Limits::default().collision,
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let result = extract_data(
            &mut c,
            Limits {
                lists,
                ..Limits::default()
            },
        );
        assert!(matches!(
            result.as_ref().err().unwrap().cause(),
            Cause::Lists(_)
        ));
        assert_eq!(c.stats().unwrap().spool.reloads, 0);
        assert!(c.finish_external_operation(result).is_err());
        assert!(c.facts().is_err());
    }
}
#[test]
fn list_callback_error_keeps_original_vertex_limit_code_and_consumes_each_attempt_once() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10), (2, 10)], &[])),
        (10, None, m2(true)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = SpoolingExtractor::new(
        &mut store,
        &EMPTY_LOOKUP,
        Limits::default().collision,
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let result = extract_data(
        &mut c,
        Limits {
            lists: gameobject_models::Limits {
                max_vertex_visits: 5,
                ..gameobject_models::Limits::default()
            },
            ..Limits::default()
        },
    );
    let e = result.as_ref().err().unwrap();
    assert_eq!(e.code(), "gameobject_models_policy_limit");
    assert!(matches!(
        e.cause(),
        Cause::Lists(streaming::Error::Codec(gameobject_models::Error::Limit))
    ));
    assert_eq!(c.stats().unwrap().spool.reloads, 2);
    let e = c
        .finish_external_operation(result)
        .map_err(bridge)
        .err()
        .unwrap();
    assert_eq!(e.code(), "gameobject_models_policy_limit");
    assert!(matches!(e.cause(), Cause::Lists(_)));
}
#[test]
fn reload_quota_sum_admission_does_not_consume_attempts() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10), (2, 10)], &[])),
        (10, None, m2(true)),
    ]);
    for limits in [
        SpoolLimits {
            max_reloads: 1,
            ..SpoolLimits::default()
        },
        SpoolLimits {
            max_reload_bytes: 399,
            ..SpoolLimits::default()
        },
    ] {
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = SpoolingExtractor::new(
            &mut store,
            &EMPTY_LOOKUP,
            Limits::default().collision,
            p.spool(&f, limits),
        )
        .unwrap();
        let result = extract_data(&mut c, Limits::default());
        assert!(result.is_err());
        assert_eq!(c.stats().unwrap().spool.reloads, 0);
        assert_eq!(c.stats().unwrap().spool.reload_bytes, 0);
        assert!(c.finish_external_operation(result).is_err());
    }
}
fn one_model<'a>(
    f: &CollisionFixture,
    p: &Private,
    store: &'a mut LocalStore,
) -> (SpoolingExtractor<'a, 'static, EmptyLookup>, Vec<RowOutcome>) {
    let mut c = SpoolingExtractor::new(
        store,
        &EMPTY_LOOKUP,
        Limits::default().collision,
        p.spool(f, SpoolLimits::default()),
    )
    .unwrap();
    let h = match c.extract_gameobject_fdid(10).unwrap() {
        GameObjectOutcome::Model(h) => h,
        _ => panic!("synthetic model"),
    };
    let rows = vec![
        RowOutcome {
            ordinal: 0,
            display_id: 1,
            file_data_id: 10,
            outcome: Outcome::Collision(h),
        },
        RowOutcome {
            ordinal: 1,
            display_id: 2,
            file_data_id: 10,
            outcome: Outcome::Collision(h),
        },
    ];
    c.reserve_external(resident::capacity(&rows).unwrap())
        .unwrap();
    (c, rows)
}
#[test]
fn exact_builder_plus_complete_record_tail_passes_but_one_less_rejects_before_reload() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    for deficit in [0usize, 1] {
        let p = Private::new();
        let mut store = f.open(false);
        let (mut c, rows) = one_model(&f, &p, &mut store);
        let h = match rows[0].outcome {
            Outcome::Collision(h) => h,
            _ => unreachable!(),
        };
        let shape = c.payload_shape(h).unwrap();
        let name = c.descriptor(h).unwrap().output_basename().to_vec();
        let metadata = [streaming::Metadata {
            display_id: 1,
            name: &name,
            model: Some(streaming::ModelInfo {
                class: streaming::Class::M2,
                raw_bytes: shape.raw_bytes.unwrap(),
            }),
        }; 2];
        let plan =
            streaming::Plan::preflight(&metadata, gameobject_models::Limits::default()).unwrap();
        let needed =
            2 * size_of::<streaming::Metadata<'_>>() + plan.planned_bytes() + shape.record_bytes
                - deficit;
        let extra = c.free_bytes().unwrap() - needed;
        c.reserve_external(extra).unwrap();
        let out = build_lists(&mut c, &rows, 2, gameobject_models::Limits::default());
        if deficit == 0 {
            assert!(out.is_ok());
            assert_eq!(c.stats().unwrap().spool.reloads, 2);
            assert_eq!(
                c.stats().unwrap().spool.reload_bytes,
                2 * shape.record_bytes
            )
        } else {
            assert!(out.is_err());
            assert_eq!(c.stats().unwrap().spool.reloads, 0)
        }
    }
}
#[test]
fn empty_builder_exact_sixteen_bytes_needs_no_fake_record_tail() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = SpoolingExtractor::new(
        &mut store,
        &EMPTY_LOOKUP,
        Limits::default().collision,
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let extra = c.free_bytes().unwrap() - 16;
    c.reserve_external(extra).unwrap();
    let out = build_lists(&mut c, &[], 0, gameobject_models::Limits::default()).unwrap();
    assert_eq!(out.stats().resident_bytes, 16);
    assert_eq!(c.stats().unwrap().spool.reloads, 0);
}
struct Later;
impl LiquidLookup for Later {
    fn material_id(&self, id: u32) -> Option<u8> {
        (id == 22).then_some(7)
    }
    fn material_flags(&self, id: u8) -> Option<u32> {
        (id == 7).then_some(2)
    }
}
fn liquid_flags(bytes: &[u8]) -> u32 {
    forever_vmap::wmo::raw::RawWmo::parse(
        bytes,
        forever_vmap::wmo::raw::ConversionLimits::default(),
    )
    .unwrap()
    .groups()
    .next()
    .unwrap()
    .liquid_flags()
}
#[test]
fn lookup_transition_keeps_same_handles_cache_counters_and_data() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
        (10, None, wmo_root(30, Some(20))),
        (20, None, m2(true)),
        (30, None, wmo_group(true, true, true)),
        (40, None, wmo_root(50, None)),
        (50, None, wmo_group(true, false, true)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut context = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let before = context.data().unwrap().startup_facts();
    let list = context.data().unwrap().lists().dtree_bytes().to_vec();
    let original = match context.data().unwrap().outcomes()[0].outcome() {
        Outcome::Collision(h) => h,
        _ => unreachable!(),
    };
    let original_raw = context
        .collector_mut()
        .with_payload(original, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    assert_eq!(liquid_flags(&original_raw), 2);
    let mut context = context.with_lookup(&Later).unwrap();
    assert_eq!(context.data().unwrap().lists().dtree_bytes(), list);
    assert_eq!(
        context.collector_mut().extract_gameobject_fdid(10).unwrap(),
        GameObjectOutcome::Model(original)
    );
    assert_eq!(context.collector_mut().facts().unwrap(), before);
    let later = context
        .collector_mut()
        .extract(collision::ModelRequest::Wmo(
            collision::InputRef::FileDataId(40),
        ))
        .unwrap();
    context
        .collector_mut()
        .with_payload(later, |p| {
            assert_eq!(liquid_flags(p.raw().unwrap()), 0);
            Ok(())
        })
        .unwrap();
    context
        .collector_mut()
        .with_payload(original, |p| {
            assert_eq!(p.raw().unwrap(), original_raw);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        context.collector_mut().facts().unwrap().reads,
        before.reads + 2
    );
    context.finish().unwrap();
}
#[test]
fn later_alias_error_poison_blocks_data_lookup_and_finish() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
        (10, None, m2(true)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    assert!(
        c.collector_mut()
            .extract(collision::ModelRequest::Wmo(
                collision::InputRef::FileDataId(10)
            ))
            .is_err()
    );
    assert!(c.data().is_err());
    assert!(c.with_lookup(&Later).is_err());
}
#[test]
fn source_change_and_record_change_reject_finish_and_keep_files() {
    for source in [true, false] {
        let f = CollisionFixture::new(&[
            (TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
            (10, None, m2(true)),
        ]);
        let p = Private::new();
        let mut store = f.open(false);
        let c = extract_startup_fdid(
            &mut store,
            Limits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        if source {
            f.change_metadata()
        } else {
            let file = p.path.join("00000000.payload");
            fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(file, b"changed").unwrap()
        };
        let e = c.finish().err().unwrap();
        assert!(e.code().contains("changed"));
        assert!(p.path.join("00000000.payload").exists());
    }
}
#[test]
fn final_pure_error_preserves_cause_and_changed_guard() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let (mut c, _) = one_model(&f, &p, &mut store);
    f.change_metadata();
    fs::set_permissions(
        p.path.join("00000000.payload"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let cause = Error::from(resident::Error::State);
    let e = c
        .finish_external_operation(Err::<(), _>(cause))
        .map_err(bridge)
        .unwrap_err();
    assert_eq!(e.cause(), cause.cause());
    assert!(e.guard().is_some());
    assert_eq!(e.code(), "local_store_input_changed");
    assert!(c.facts().is_err());
}
#[test]
fn completed_split_keeps_readiness_and_guarded_payload_reload() {
    let f = CollisionFixture::new(&[
        (TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
        (10, None, m2(true)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let completed = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap()
    .finish()
    .unwrap();
    let (data, mut extraction) = completed.into_parts();
    let h = match data.outcomes()[0].outcome() {
        Outcome::Collision(h) => h,
        _ => unreachable!(),
    };
    drop(data);
    extraction
        .with_payload(h, |p| {
            assert!(p.raw().is_some());
            Ok(())
        })
        .unwrap();
    fs::set_permissions(
        p.path.join("00000000.payload"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(extraction.with_payload(h, |_| Ok(())).is_err());
    assert!(extraction.ready().is_err());
}
fn large_m2() -> Vec<u8> {
    let mut b = m2(true);
    b.resize(310 + 16000 * 12, 0);
    put(&mut b, 224, 16000);
    b
}
#[test]
fn startup_cumulative_raw_exceeds_active_and_sameglobal_resident_rejects() {
    let rows: Vec<_> = (10..34).map(|id| (id, id)).collect();
    let mut files: Vec<_> = (10..34).map(|id| (id, None, large_m2())).collect();
    files.push((TABLE_FILE_DATA_ID, None, db2(&rows, &[])));
    let f = CollisionFixture::new(&files);
    let mut store = f.open(false);
    let reference = resident::extract_startup_fdid(&mut store, Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    let p = Private::new();
    let mut store = f.open(false);
    let cap = store.stats().resident_bytes + 3 * 1024 * 1024 + 512 * 1024;
    let limits = Limits {
        collision: collision::Limits {
            max_memory_bytes: cap,
            ..collision::Limits::default()
        },
        ..Limits::default()
    };
    let new = extract_startup_fdid(
        &mut store,
        limits,
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 3 * 1024 * 1024,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap()
    .finish()
    .unwrap();
    compare(reference.data(), new.data().unwrap());
    assert!(new.data().unwrap().startup_facts().raw_bytes > 3 * 1024 * 1024);
    assert_eq!(new.extraction().stats().unwrap().spool.reloads, 24);
    assert!(new.extraction().stats().unwrap().observed_payload_bytes <= 3 * 1024 * 1024);
    let mut store = f.open(false);
    assert!(resident::extract_startup_fdid(&mut store, limits).is_err());
}
#[test]
fn many_wmo_roots_exceed_active_while_full_record_reload_tail_is_retained() {
    let rows: Vec<_> = (10..22).map(|id| (id, id)).collect();
    let mut files: Vec<_> = (10..22)
        .map(|id| {
            let mut root = wmo_root(1000, Some(1001));
            root.extend(chunk(b"ZZZZ", &vec![0; 320 * 1024]));
            (id, None, root)
        })
        .collect();
    files.extend([
        (TABLE_FILE_DATA_ID, None, db2(&rows, &[])),
        (1000, None, wmo_group(true, true, false)),
        (1001, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(false);
    let new = extract_startup_fdid(
        &mut store,
        Limits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 3 * 1024 * 1024,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap()
    .finish()
    .unwrap();
    let stats = new.extraction().stats().unwrap();
    assert!(stats.spool.disk_bytes > 3 * 1024 * 1024);
    assert_eq!(stats.spool.reloads, 12);
    assert!(stats.spool.peak_reload_bytes > 320 * 1024);
    assert_eq!(new.data().unwrap().startup_facts().dependency_requests, 12);
}
#[test]
fn old_startup_error_remains_exhaustively_matchable() {
    fn exact(e: resident::Error) -> u8 {
        match e {
            resident::Error::InvalidLimits => 0,
            resident::Error::Limit => 1,
            resident::Error::Allocation => 2,
            resident::Error::State => 3,
            resident::Error::Db2(_) => 4,
            resident::Error::Collision(_) => 5,
            resident::Error::Lists(_) => 6,
        }
    }
    assert_eq!(exact(resident::Error::State), 3);
}
