// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic stores, persistent startup/world cache.
use super::*;
use crate::{
    collision,
    local_store::tests::{CollisionFile, CollisionFixture},
};
use forever_db2::terrain_sources::{DirectRecord, VMapFields};
use forever_vmap::{placement::DirBin, wmo, wmo::raw};

#[derive(Default, PartialEq)]
struct Stage {
    models: Vec<(Status, Option<Vec<u8>>)>,
    placements: Vec<(Context, Vec<u8>)>,
}
impl Sink for Stage {
    fn model(&mut self, model: &Model) -> Result<(), SinkError> {
        self.models
            .push((model.status(), model.raw().map(<[u8]>::to_vec)));
        Ok(())
    }
    fn placements(&mut self, context: Context, bytes: &[u8]) -> Result<(), SinkError> {
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod diagnostic {
    use crate::local_store::{
        Error, ProductDiagnosticObserver, ProductDiagnosticPhase, ProductDiagnosticRequest,
        ProductDiagnosticRequestKind,
    };
    use std::{cell::RefCell, rc::Rc};

    pub type Requests = Rc<RefCell<Vec<ProductDiagnosticRequest>>>;
    pub struct Observer {
        requests: Requests,
        absences: Requests,
        phase: ProductDiagnosticPhase,
        ordinal: u32,
    }
    impl ProductDiagnosticObserver for Observer {
        fn set_phase(&mut self, phase: ProductDiagnosticPhase) {
            if self.phase != phase {
                self.phase = phase;
                self.ordinal = 0;
            }
        }
        fn begin_request(
            &mut self,
            request_kind: ProductDiagnosticRequestKind,
            file_data_id: Option<u32>,
        ) -> Option<ProductDiagnosticRequest> {
            let request = ProductDiagnosticRequest {
                phase: self.phase,
                request_kind,
                file_data_id,
                ordinal: self.ordinal,
            };
            self.ordinal = self.ordinal.saturating_add(1);
            self.requests.borrow_mut().push(request);
            Some(request)
        }
        fn tolerated_absence(&mut self, request: ProductDiagnosticRequest) {
            self.absences.borrow_mut().push(request);
        }
        fn source_failure(&mut self, _: ProductDiagnosticRequest, _: Error, _: bool) {}
        fn finish(&mut self, _: Option<&'static str>, _: bool) {}
    }
    pub fn recording() -> (Box<Observer>, Requests) {
        let (observer, requests, _) = recording_with_absences();
        (observer, requests)
    }
    pub fn recording_with_absences() -> (Box<Observer>, Requests, Requests) {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let absences = Rc::new(RefCell::new(Vec::new()));
        (
            Box::new(Observer {
                requests: requests.clone(),
                absences: absences.clone(),
                phase: ProductDiagnosticPhase::Other,
                ordinal: 0,
            }),
            requests,
            absences,
        )
    }
}

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn row(id: u32, wdt: i32, parent: i16) -> DirectRecord<VMapFields> {
    DirectRecord {
        id,
        fields: VMapFields {
            wdt_file_data_id: wdt,
            parent_map_id: parent,
            name: "Synthetic map".into(),
            directory: "Synthetic".into(),
        },
    }
}
fn maps(rows: &[DirectRecord<VMapFields>]) -> plan::Maps<'_> {
    plan::Maps::from_materialized(rows, rows.len(), plan::Limits::default()).unwrap()
}
fn wdt(tiles: &[(usize, usize, u32)], fdid: bool, global: Option<(u32, u32)>) -> Vec<u8> {
    wdt_globals(tiles, fdid, &global.into_iter().collect::<Vec<_>>())
}
fn global_wdt(id: u32, unique: u32) -> Vec<u8> {
    global_wdt_records(&[(id, unique)])
}
fn global_wdt_records(rows: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (id, unique) in rows {
        let mut source = [0; 64];
        put(&mut source, 0, *id);
        put(&mut source, 4, *unique);
        source[56..58].copy_from_slice(&8u16.to_le_bytes());
        bytes.extend(chunk(b"MODF", &source));
    }
    bytes
}
fn wdt_globals(tiles: &[(usize, usize, u32)], fdid: bool, globals: &[(u32, u32)]) -> Vec<u8> {
    let mut mphd = [0; 32];
    put(&mut mphd, 0, if fdid { 0x200 } else { 0 });
    let mut bytes = chunk(b"MPHD", &mphd);
    let mut main = vec![0; 32768];
    let mut maid = vec![0; 131072];
    for &(x, y, id) in tiles {
        put(&mut main, (y * 64 + x) * 8, 1);
        put(&mut maid, (y * 64 + x) * 32, 9999); // Root ADT is not acquired.
        put(&mut maid, (y * 64 + x) * 32 + 4, id);
        put(&mut maid, (y * 64 + x) * 32 + 8, 9998); // Neither is Obj1.
    }
    bytes.extend(chunk(b"MAIN", &main));
    if fdid {
        bytes.extend(chunk(b"MAID", &maid));
    }
    for &(id, unique) in globals {
        let mut source = [0; 64];
        put(&mut source, 0, id);
        put(&mut source, 4, unique);
        source[56..58].copy_from_slice(&8u16.to_le_bytes());
        bytes.extend(chunk(b"MODF", &source));
    }
    bytes
}
fn adt(id: u32, unique: u32) -> Vec<u8> {
    let mut record = [0; 36];
    put(&mut record, 0, id);
    put(&mut record, 4, unique);
    record[32..34].copy_from_slice(&1024u16.to_le_bytes());
    record[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &record)
}
fn empty_adt() -> Vec<u8> {
    chunk(b"MVER", &18u32.to_le_bytes())
}
fn m2(collision: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if collision {
        put(&mut bytes, 216, 3);
        put(&mut bytes, 220, 304);
        put(&mut bytes, 224, 3);
        put(&mut bytes, 228, 310);
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}
fn wmo_files() -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root.extend(chunk(b"MODI", &20u32.to_le_bytes()));
    let mut spawn = [0; 40];
    put(&mut spawn, 28, 1f32.to_bits());
    put(&mut spawn, 32, 1f32.to_bits());
    root.extend(chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put(&mut set, 24, 1);
    root.extend(chunk(b"MODS", &set));
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[0, 0]));
    group.extend(chunk(b"MOPY", &[8, 0]));
    group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    let mut vertices = Vec::new();
    for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
        vertices.extend_from_slice(&word.to_le_bytes());
    }
    group.extend(chunk(b"MOVT", &vertices));
    vec![
        (10, None, root),
        (20, None, m2(true)),
        (30, None, chunk(b"MOGP", &group)),
    ]
}
fn ids(bytes: &[u8]) -> Vec<u32> {
    DirBin::parse(bytes, placement::Limits::default())
        .unwrap()
        .records()
        .map(|value| value.id())
        .collect()
}
fn tile_outputs(stage: &Stage) -> Vec<&(Context, Vec<u8>)> {
    stage
        .placements
        .iter()
        .filter(|(context, _)| matches!(context.kind, Kind::Tile { .. }))
        .collect()
}

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|word| word.to_le_bytes()).collect()
}
fn wmo_with_unreferenced_missing_doodad() -> Vec<CollisionFile<'static>> {
    let mut files = wmo_with_missing_doodad(Some(20), true, false);
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[]));
    files[1].2 = chunk(b"MOGP", &group);
    files
}

fn wmo_with_missing_doodad(
    missing: Option<u32>,
    sets: bool,
    geometry: bool,
) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    let requests: Vec<_> = missing.into_iter().collect();
    root.extend(chunk(b"MODI", &words(&requests)));
    let mut spawns = Vec::new();
    for _ in &requests {
        let mut spawn = [0; 40];
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        spawns.extend_from_slice(&spawn);
    }
    root.extend(chunk(b"MODD", &spawns));
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, requests.len() as u32);
        root.extend(chunk(b"MODS", &set));
    }
    let references = if requests.is_empty() {
        Vec::new()
    } else {
        vec![0; requests.len() * 2]
    };
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &references));
    if geometry {
        group.extend(chunk(b"MOPY", &[8, 0]));
        group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        let mut vertices = Vec::new();
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            vertices.extend_from_slice(&word.to_le_bytes());
        }
        group.extend(chunk(b"MOVT", &vertices));
    }
    vec![(10, None, root), (30, None, chunk(b"MOGP", &group))]
}

fn wmo_children(requests: &[u32], children: &[(u32, Vec<u8>)]) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root.extend(chunk(b"MODI", &words(requests)));
    let mut spawns = Vec::new();
    for (index, _) in requests.iter().enumerate() {
        let mut spawn = [0; 40];
        put(&mut spawn, 0, index as u32);
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        spawns.extend_from_slice(&spawn);
    }
    root.extend(chunk(b"MODD", &spawns));
    let mut set = [0; 32];
    put(&mut set, 24, requests.len() as u32);
    root.extend(chunk(b"MODS", &set));
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    let references: Vec<u8> = (0..requests.len())
        .flat_map(|index| (index as u16).to_le_bytes())
        .collect();
    group.extend(chunk(b"MODR", &references));
    if !requests.is_empty() {
        group.extend(chunk(b"MOPY", &[8, 0]));
        group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        let mut vertices = Vec::new();
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            vertices.extend_from_slice(&word.to_le_bytes());
        }
        group.extend(chunk(b"MOVT", &vertices));
    }
    let mut files = vec![(10, None, root), (30, None, chunk(b"MOGP", &group))];
    for (id, bytes) in children {
        files.push((*id, None, bytes.clone()));
    }
    files
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
fn fixture(rows: &[(u32, u32)], mut files: Vec<CollisionFile<'_>>) -> CollisionFixture {
    files.push((gameobjects::TABLE_FILE_DATA_ID, None, db2(rows, &[])));
    CollisionFixture::new(&files)
}
fn startup(store: &mut LocalStore) -> gameobjects::Startup<'_> {
    gameobjects::extract_startup_fdid(store, gameobjects::Limits::default()).unwrap()
}
#[cfg(target_os = "linux")]
fn run_model_heavy_with_limits(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
) -> (Result<SharedReport, Error>, Stage) {
    let profile = collision::resources::Profile::ModelHeavyV1;
    let mut store = fixture.open(false);
    let mut context = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        profile.startup_limits(),
        profile,
    )
    .unwrap();
    let mut stage = Stage::default();
    let result = extract_shared(&mut context, &maps(rows), &mut stage, limits);
    if result.is_err() {
        assert_aborted(context);
    } else {
        context.finish().unwrap();
    }
    (result, stage)
}
#[cfg(target_os = "linux")]
fn model_heavy_fixture(model_count: usize) -> CollisionFixture {
    let mut records = Vec::with_capacity(model_count * 36);
    let mut files = vec![
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (201, None, Vec::new()),
    ];
    let model_source = m2(true);
    for index in 0..model_count {
        let id = 10_000 + index as u32;
        let mut record = [0; 36];
        put(&mut record, 0, id);
        put(&mut record, 4, index as u32 + 1);
        record[32..34].copy_from_slice(&1024u16.to_le_bytes());
        record[34..36].copy_from_slice(&0x40u16.to_le_bytes());
        records.extend_from_slice(&record);
        files.push((id, None, model_source.clone()));
    }
    files[1].2 = chunk(b"MDDF", &records);
    CollisionFixture::new_with_shared_content(&files)
}
fn assert_aborted(context: gameobjects::Startup<'_>) {
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::Aborted))
    );
}

fn run_policy(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    missing_wdt: MissingWdtPolicy,
) -> (Result<(SharedReport, Vec<RootAbsent>), Error>, Stage) {
    run_policy_with_limits(fixture, rows, missing_wdt, SharedLimits::default())
}

fn run_policy_with_limits(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    missing_wdt: MissingWdtPolicy,
    limits: SharedLimits,
) -> (Result<(SharedReport, Vec<RootAbsent>), Error>, Stage) {
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result =
        extract_shared_with_policy(&mut context, &maps(rows), &mut stage, limits, missing_wdt);
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

type OmissionResult = (
    Result<(SharedReport, Vec<RootAbsent>, Vec<u32>), Error>,
    Stage,
);

fn run_omissions(fixture: CollisionFixture, rows: &[DirectRecord<VMapFields>]) -> OmissionResult {
    run_omissions_with_limits(fixture, rows, SharedLimits::default())
}

fn run_omissions_with_limits(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
) -> OmissionResult {
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result = extract_shared_with_omissions(
        &mut context,
        &maps(rows),
        &mut stage,
        limits,
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    );
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

type VacatedResult = (VacatedWmoResult, Stage);

fn run_vacated_wmo(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
) -> VacatedResult {
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result = extract_shared_with_vacated_wmo(&mut context, &maps(rows), &mut stage, limits);
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

fn run_empty_wmo(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
) -> VacatedResult {
    run_empty_wmo_with_names(fixture, rows, limits, false)
}

#[cfg(target_os = "linux")]
fn run_unselected_wmo(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
    names: bool,
) -> (UnselectedWmoResult, Stage) {
    let mut store = fixture.open(names);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result = extract_shared_with_unselected_wmo(&mut context, &maps(rows), &mut stage, limits);
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

#[cfg(target_os = "linux")]
fn unselected_geometry_fixture(
    fdid: u32,
    skip: bool,
    sets: bool,
    doodad: bool,
) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 24, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    root.extend(chunk(b"GFID", &words(&[30])));
    if doodad {
        root.extend(chunk(b"MODI", &words(&[20])));
        let mut spawn = [0; 40];
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        root.extend(chunk(b"MODD", &spawn));
    }
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, u32::from(doodad));
        root.extend(chunk(b"MODS", &set));
    }
    let mut group = vec![0; 68];
    put(&mut group, 8, if skip { 0x80 } else { 0 });
    put(&mut group, 56, 99);
    group.extend(chunk(b"MOPY", &[8, 0]));
    group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    let mut vertices = Vec::new();
    for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
        vertices.extend_from_slice(&word.to_le_bytes());
    }
    group.extend(chunk(b"MOVT", &vertices));
    let mut files = vec![
        (fdid, None, root),
        (30, None, chunk(b"MOGP", &group)),
        (101, None, wdt(&[], true, Some((fdid, 8)))),
    ];
    if doodad {
        files.push((20, None, m2(true)));
    }
    files
}

#[cfg(target_os = "linux")]
fn material_filtered_wmo_fixture(
    fdid: u32,
    groups: &[(u16, bool)],
    sets: bool,
    references: &[u16],
    liquid: bool,
) -> Vec<CollisionFile<'static>> {
    material_filtered_wmo_fixture_format(fdid, groups, sets, references, liquid, false)
}

#[cfg(target_os = "linux")]
fn material_filtered_wmo_fixture_format(
    fdid: u32,
    groups: &[(u16, bool)],
    sets: bool,
    references: &[u16],
    liquid: bool,
    wide_materials: bool,
) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, groups.len() as u32);
    put(&mut header, 16, u32::from(!references.is_empty()));
    put(&mut header, 20, u32::from(!references.is_empty()));
    put(&mut header, 24, u32::from(sets));
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    let group_ids: Vec<_> = (0..groups.len()).map(|i| 30 + i as u32).collect();
    root.extend(chunk(b"GFID", &words(&group_ids)));
    if !references.is_empty() {
        root.extend(chunk(b"MODI", &words(&[20])));
        let mut spawn = [0; 40];
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        root.extend(chunk(b"MODD", &spawn));
    }
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, u32::from(!references.is_empty()));
        root.extend(chunk(b"MODS", &set));
    }
    let mut files = vec![(fdid, None, root)];
    for (index, (flags, skip)) in groups.iter().copied().enumerate() {
        let mut group = vec![0; 68];
        put(&mut group, 8, if skip { 0x80 } else { 0 });
        put(&mut group, 56, 99);
        let reference_bytes: Vec<_> = references
            .iter()
            .flat_map(|reference| reference.to_le_bytes())
            .collect();
        group.extend(chunk(b"MODR", &reference_bytes));
        if wide_materials {
            let mut material = Vec::with_capacity(4);
            material.extend_from_slice(&flags.to_le_bytes());
            material.extend_from_slice(&0xabcd_u16.to_le_bytes());
            group.extend(chunk(b"MPY2", &material));
        } else {
            group.extend(chunk(b"MOPY", &flags.to_le_bytes()));
        }
        group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        let mut vertices = Vec::new();
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            vertices.extend_from_slice(&word.to_le_bytes());
        }
        group.extend(chunk(b"MOVT", &vertices));
        if liquid {
            group.extend(chunk(b"MLIQ", &[0; 30]));
        }
        files.push((30 + index as u32, None, chunk(b"MOGP", &group)));
    }
    files.push((101, None, global_wdt(fdid, 8)));
    if !references.is_empty() {
        files.push((20, None, m2(true)));
    }
    files
}

#[cfg(target_os = "linux")]
fn parsed_materials(files: &[CollisionFile<'_>], group_id: u32) -> Vec<wmo::TriangleMaterial> {
    let bytes = &files
        .iter()
        .find(|(id, _, _)| *id == group_id)
        .expect("synthetic group must be present")
        .2;
    let group = wmo::Group::parse(bytes, wmo::Limits::default()).unwrap();
    assert_eq!(group.index_count(), group.triangle_count() * 3);
    assert_eq!(group.vertex_count(), 3);
    (0..group.triangle_count())
        .map(|index| group.triangle_material(index).unwrap())
        .collect()
}

#[cfg(target_os = "linux")]
fn run_material_filtered_wmo(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
    names: bool,
) -> (MaterialFilteredWmoResult, Stage) {
    let mut store = fixture.open(names);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result =
        extract_shared_with_material_filtered_wmo(&mut context, &maps(rows), &mut stage, limits);
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

fn run_empty_wmo_with_names(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    limits: SharedLimits,
    names: bool,
) -> VacatedResult {
    let mut store = fixture.open(names);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let result = execute(
        context.collector_mut(),
        &maps(rows),
        &mut stage,
        limits,
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty,
    )
    .map(
        |(report, root_absent, missing_m2, missing_wmo, empty_wmo, _, _)| {
            (report, root_absent, missing_m2, missing_wmo, empty_wmo)
        },
    );
    if result.is_err() {
        context.collector_mut().abort();
    }
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage)
}

fn empty_wmo_fixture(
    fdid: u32,
    groups: Vec<(u32, Vec<u8>)>,
    requests: &[u32],
    references: &[u16],
    sets: bool,
    group_geometry: bool,
) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 16, requests.len() as u32);
    put(&mut header, 20, requests.len() as u32);
    put(&mut header, 24, u32::from(sets));
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    let group_ids: Vec<_> = groups.iter().map(|(id, _)| *id).collect();
    root.extend(chunk(b"GFID", &words(&group_ids)));
    if !requests.is_empty() {
        root.extend(chunk(b"MODI", &words(requests)));
        let mut spawns = Vec::new();
        for _ in requests {
            let mut spawn = [0; 40];
            put(&mut spawn, 28, 1f32.to_bits());
            put(&mut spawn, 32, 1f32.to_bits());
            spawns.extend_from_slice(&spawn);
        }
        root.extend(chunk(b"MODD", &spawns));
    }
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, requests.len() as u32);
        root.extend(chunk(b"MODS", &set));
    }
    let mut files = vec![(fdid, None, root)];
    for (id, mut group) in groups {
        group.extend(chunk(
            b"MODR",
            &references
                .iter()
                .flat_map(|index| index.to_le_bytes())
                .collect::<Vec<_>>(),
        ));
        if group_geometry {
            group.extend(chunk(b"MOPY", &[8, 0]));
            group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
            let mut vertices = Vec::new();
            for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
                vertices.extend_from_slice(&word.to_le_bytes());
            }
            group.extend(chunk(b"MOVT", &vertices));
        }
        files.push((id, None, chunk(b"MOGP", &group)));
    }
    files.push((101, None, global_wdt(fdid, 8)));
    files
}

fn intrinsic_empty_wmo_fixture() -> Vec<CollisionFile<'static>> {
    empty_wmo_fixture(10, vec![(30, vec![0; 68])], &[], &[], true, false)
}

fn named_wmo_wdt(name: &[u8], unique: u32) -> Vec<u8> {
    let mut named = name.to_vec();
    named.push(0);
    let mut source = [0; 64];
    put(&mut source, 4, unique);
    let mut bytes = chunk(b"MWMO", &named);
    bytes.extend(chunk(b"MODF", &source));
    bytes
}

fn missing_child_wmo_fixture() -> Vec<CollisionFile<'static>> {
    let mut root = wmo_with_missing_doodad(Some(20), true, false)
        .into_iter()
        .find(|file| file.0 == 10)
        .unwrap()
        .2;
    let header_at = root
        .windows(4)
        .position(|window| window == b"DHOM")
        .unwrap();
    put(&mut root, header_at + 8 + 24, 1);
    let gfid_at = root
        .windows(4)
        .position(|window| window == b"DIFG")
        .unwrap();
    put(&mut root, gfid_at + 8, 31);
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[0, 0]));
    vec![
        (11, None, root),
        (31, None, chunk(b"MOGP", &group)),
        (101, None, wdt_globals(&[], true, &[(11, 8)])),
    ]
}

fn empty_wmo_run(files: Vec<CollisionFile<'static>>, limits: SharedLimits) -> VacatedResult {
    run_empty_wmo(fixture(&[], files), &[row(1, 101, -1)], limits)
}

#[cfg(target_os = "linux")]
type PolicyObservation = (
    Result<(SharedReport, Vec<RootAbsent>), Error>,
    Stage,
    diagnostic::Requests,
    diagnostic::Requests,
);

#[cfg(target_os = "linux")]
fn run_policy_observed(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    missing_wdt: MissingWdtPolicy,
) -> PolicyObservation {
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let (observer, requests, absences) = diagnostic::recording_with_absences();
    context
        .collector_mut()
        .store
        .set_product_diagnostic_observer(Some(observer));
    let mut stage = Stage::default();
    let result = extract_shared_with_policy(
        &mut context,
        &maps(rows),
        &mut stage,
        SharedLimits::default(),
        missing_wdt,
    );
    let finish = context.finish().err();
    assert_eq!(result.is_err(), finish.is_some());
    (result, stage, requests, absences)
}

#[cfg(target_os = "linux")]
fn run_with_diagnostic(
    fixture: CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    observe: bool,
) -> (SharedReport, Stage, diagnostic::Requests) {
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let (observer, requests) = diagnostic::recording();
    if observe {
        context
            .collector_mut()
            .store
            .set_product_diagnostic_observer(Some(observer));
    }
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    context.finish().unwrap();
    (report, stage, requests)
}

#[cfg(target_os = "linux")]
#[test]
fn shared_diagnostic_classifies_wdt_adt_and_root_m2_without_changing_execution() {
    use crate::local_store::{
        ProductDiagnosticPhase as Phase, ProductDiagnosticRequest as Request,
        ProductDiagnosticRequestKind as RequestKind,
    };

    let rows = [row(1, 101, -1)];
    let make_fixture = || {
        fixture(
            &[],
            vec![
                (10, None, m2(true)),
                (101, None, wdt(&[(0, 0, 201)], true, None)),
                (201, None, adt(10, 7)),
            ],
        )
    };
    let (baseline_report, baseline_stage, _) = run_with_diagnostic(make_fixture(), &rows, false);
    let (report, stage, requests) = run_with_diagnostic(make_fixture(), &rows, true);
    assert_eq!(report, baseline_report);
    assert!(stage == baseline_stage);
    assert_eq!(
        requests.borrow().as_slice(),
        [
            Request {
                phase: Phase::Placements,
                request_kind: RequestKind::Wdt,
                file_data_id: Some(101),
                ordinal: 0,
            },
            Request {
                phase: Phase::Placements,
                request_kind: RequestKind::TileAdt,
                file_data_id: Some(201),
                ordinal: 0,
            },
            Request {
                phase: Phase::Models,
                request_kind: RequestKind::RootM2,
                file_data_id: Some(10),
                ordinal: 0,
            },
        ]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn shared_diagnostic_classifies_wmo_root_children_and_groups() {
    use crate::local_store::{
        ProductDiagnosticPhase as Phase, ProductDiagnosticRequest as Request,
        ProductDiagnosticRequestKind as RequestKind,
    };

    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 8)))));
    let rows = [row(1, 101, -1)];
    let (_, _, requests) = run_with_diagnostic(fixture(&[], files), &rows, true);
    assert_eq!(
        requests.borrow().as_slice(),
        [
            Request {
                phase: Phase::Placements,
                request_kind: RequestKind::Wdt,
                file_data_id: Some(101),
                ordinal: 0,
            },
            Request {
                phase: Phase::Models,
                request_kind: RequestKind::WmoRoot,
                file_data_id: Some(10),
                ordinal: 0,
            },
            Request {
                phase: Phase::Models,
                request_kind: RequestKind::Child,
                file_data_id: Some(20),
                ordinal: 1,
            },
            Request {
                phase: Phase::Models,
                request_kind: RequestKind::WmoGroup,
                file_data_id: Some(30),
                ordinal: 2,
            },
        ]
    );
}

#[test]
fn startup_cache_and_current_request_window_are_shared_across_two_tiles() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, adt(10, 7)),
            (202, None, adt(10, 8)),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let initial = context.collector_mut().models()[0].raw().unwrap().to_vec();
    let before_external = context.collector_mut().external;
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.collector_before.reads, 2);
    assert_eq!(report.collector_after.reads, 5);
    assert_eq!(report.run.model_reads, 0);
    assert_eq!(report.run.model_source_bytes, 0);
    assert_eq!(
        (report.request_calls_before, report.request_calls_after),
        (1, 3)
    );
    assert_eq!(
        (
            report.run.model_callbacks,
            report.run.own_tiles,
            report.run.ids.calls
        ),
        (1, 2, 2)
    );
    assert_eq!(stage.models[0].1.as_deref(), Some(initial.as_slice()));
    let tiles = tile_outputs(&stage);
    assert_eq!(ids(&tiles[0].1), [u32::MAX - 1]);
    assert_eq!(ids(&tiles[1].1), [u32::MAX - 2]);
    assert_eq!(context.collector_mut().external, before_external);
    assert_eq!(context.collector_mut().external_sources, 0);
    let completed = context.finish().unwrap();
    assert_eq!(completed.extraction().requests().len(), 3);
    assert_eq!(completed.extraction().models().len(), 1);
}

#[test]
fn dependency_handles_remain_valid_when_only_wmo_request_window_is_projected() {
    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 8)))));
    let fixture = fixture(&[(1, 10)], files);
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.model_reads, 0);
    assert_eq!(report.run.model_callbacks, 2);
    assert_eq!(report.run.placement_records, 2);
    assert_eq!(report.run.ids.calls, 2);
    assert_eq!(ids(&stage.placements[0].1), [8, u32::MAX - 1]);
    assert_eq!(context.finish().unwrap().extraction().models().len(), 2);
}

struct Visual;
impl LiquidLookup for Visual {
    fn material_id(&self, _: u32) -> Option<u8> {
        Some(0)
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        Some(2)
    }
}
#[test]
fn lookup_transition_preserves_startup_raw_and_only_new_models_use_loaded_tables() {
    let mut files = wmo_files();
    // Header-only liquid 22 in both groups. Their RAW differs only because the
    // first root is cached during empty-lookup startup, before the transition.
    put(&mut files[2].2, 8 + 52, 22);
    let mut second_root = files[0].2.clone();
    // GFID is the single word after MVER12 + MOHD72 + GFID8.
    put(&mut second_root, 92, 31);
    let second_group = files[2].2.clone();
    files.extend([
        (11, None, second_root),
        (31, None, second_group),
        (101, None, wdt(&[], true, Some((10, 3)))),
        (102, None, wdt(&[], true, Some((11, 4)))),
    ]);
    let fixture = fixture(&[(1, 10)], files);
    let rows = [row(1, 101, -1), row(2, 102, -1)];
    let mut store = fixture.open(false);
    let initial = startup(&mut store);
    let mut context = initial.with_lookup(&Visual).unwrap();
    let before = context.collector_mut().models()[0].raw().unwrap().to_vec();
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.model_reads, 2);
    assert_eq!(stage.models.len(), 3); // cached root+dependency; new root.
    assert_eq!(stage.models[0].1.as_deref(), Some(before.as_slice()));
    let raw_a = raw::RawWmo::parse(
        stage.models[0].1.as_ref().unwrap(),
        raw::ConversionLimits::default(),
    )
    .unwrap();
    let raw_b = raw::RawWmo::parse(
        stage.models[2].1.as_ref().unwrap(),
        raw::ConversionLimits::default(),
    )
    .unwrap();
    assert_eq!(raw_a.groups().next().unwrap().liquid_flags(), 2);
    assert_eq!(raw_b.groups().next().unwrap().liquid_flags(), 0);
    context.finish().unwrap();
}

#[test]
fn own_parent_cache_replay_keeps_ids_and_does_not_fill_grandchild() {
    let rows = [row(3, 103, 2), row(2, 102, 1), row(1, 101, -1)];
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(2, 1, 201), (3, 1, 202)], true, None)),
            (102, None, wdt(&[], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(
        (
            report.run.own_tiles,
            report.run.replayed_tiles,
            report.run.empty_replays,
            report.run.cached_tiles
        ),
        (2, 2, 1, 2)
    );
    assert_eq!(report.run.ids.calls, 1);
    let tiles = tile_outputs(&stage);
    assert_eq!(
        tiles.iter().map(|(c, _)| c.map_id).collect::<Vec<_>>(),
        [1, 1, 2, 2]
    );
    assert_eq!(ids(&tiles[0].1), ids(&tiles[2].1));
    assert_eq!(tiles[2].1[0], 2);
    assert!(tiles[3].1.is_empty());
    assert_eq!(context.collector_mut().external_sources, 0);
    context.finish().unwrap();
}

#[test]
fn own_empty_tile_blocks_parent_and_is_replayed_by_its_child() {
    let rows = [row(1, 101, -1), row(2, 102, 1), row(3, 103, 2)];
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 202)], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    let tiles = tile_outputs(&stage);
    assert_eq!(tiles.len(), 3);
    assert!(!tiles[1].0.replayed);
    assert!(tiles[1].1.is_empty());
    assert!(tiles[2].0.replayed);
    assert_eq!(tiles[2].0.original_map_id, 2);
    assert!(tiles[2].1.is_empty());
    assert_eq!((report.run.cached_tiles, report.run.ids.calls), (2, 1));
    context.finish().unwrap();
}

#[test]
fn physical_globals_precede_depth_order_tiles_and_are_not_inherited() {
    let rows = [row(2, 102, 1), row(1, 101, -1)];
    let mut files = wmo_files();
    files.extend([
        (101, None, wdt(&[(0, 0, 201)], true, Some((10, 10)))),
        (102, None, wdt(&[], true, None)),
        (201, None, adt(20, 11)),
    ]);
    let fixture = fixture(&[], files);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(stage.placements[0].0.map_id, 2);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(ids(&stage.placements[1].1), [10, u32::MAX - 1]);
    let tiles = tile_outputs(&stage);
    assert_eq!(ids(&tiles[0].1), [u32::MAX - 2]);
    assert_eq!(ids(&tiles[1].1), [u32::MAX - 2]);
    assert_eq!(report.run.ids.calls, 3);
    assert_eq!(report.run.model_callbacks, 2);
    context.finish().unwrap();
}

#[test]
fn no_collision_startup_models_are_delivered_once_and_placements_consume_no_id() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(false)),
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (201, None, adt(10, 3)),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(report.run.ids.calls, 0);
    assert_eq!(stage.models[0].0, Status::NoCollision);
    assert!(tile_outputs(&stage)[0].1.is_empty());
    context.finish().unwrap();
}

#[test]
fn second_run_is_rejected_even_after_lookup_transition() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    extract_shared(
        &mut context,
        &maps(&[]),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    let mut context = context.with_lookup(&Visual).unwrap();
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut stage,
            SharedLimits::default()
        ),
        Err(Error::Collision(collision::Error::State))
    );
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::State))
    );
}

struct Reject {
    models: bool,
}
impl Sink for Reject {
    fn model(&mut self, _: &Model) -> Result<(), SinkError> {
        if self.models {
            Err(SinkError::Rejected)
        } else {
            Ok(())
        }
    }
    fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
        Err(SinkError::Rejected)
    }
}
#[test]
fn model_sink_failure_poison_prevents_finish_transition_and_new_reads() {
    let fixture = fixture(&[(1, 10)], vec![(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut Reject { models: true },
            SharedLimits::default()
        ),
        Err(Error::Sink)
    );
    let before = context.collector_mut().facts();
    assert_eq!(
        context
            .collector_mut()
            .extract(ModelRequest::M2(InputRef::FileDataId(10)))
            .err(),
        Some(collision::Error::Aborted)
    );
    assert_eq!(context.collector_mut().facts(), before);
    assert_eq!(
        context.with_lookup(&Visual).err(),
        Some(gameobjects::Error::Collision(collision::Error::Aborted))
    );
}
#[test]
fn placements_sink_failure_poison_survives_finish() {
    let fixture = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut Reject { models: false },
            SharedLimits::default()
        ),
        Err(Error::Sink)
    );
    assert_aborted(context);
}
#[test]
fn missing_own_adt_is_terminal_instead_of_parent_fallback() {
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 999)], true, None)),
            (201, None, empty_adt()),
        ],
    );
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let error = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap_err();
    let Error::Collision(cause) = error else {
        panic!("expected source error")
    };
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(cause))
    );
}
#[test]
fn malformed_late_adt_poison_cannot_publish_earlier_success() {
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, empty_adt()),
            (202, None, b"bad".to_vec()),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    assert!(matches!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut stage,
            SharedLimits::default()
        ),
        Err(Error::Objects(_))
    ));
    assert_eq!(tile_outputs(&stage).len(), 1);
    assert_aborted(context);
}
#[test]
fn named_obj0_requires_explicit_store_names_and_is_not_model_normalized() {
    let name = b"World\\Maps\\Synthetic\\Synthetic_0_0_obj0.adt";
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 0)], false, None)),
            (201, Some(name), empty_adt()),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(true);
    let mut context = startup(&mut store);
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.object_adts, 1);
    context.finish().unwrap();
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert!(matches!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut Stage::default(),
            SharedLimits::default()
        ),
        Err(Error::Collision(_))
    ));
    assert!(context.finish().is_err());
}
#[test]
fn cumulative_read_and_request_caps_include_startup_and_prior_tiles() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, adt(10, 1)),
            (202, None, adt(10, 2)),
        ],
    );
    let rows = [row(1, 101, -1)];
    for requests in [false, true] {
        let mut limits = gameobjects::Limits::default();
        if requests {
            limits.collision.max_requests = 2;
        } else {
            limits.collision.max_reads = 3;
        }
        let mut store = fixture.open(false);
        let mut context = gameobjects::extract_startup_fdid(&mut store, limits).unwrap();
        assert_eq!(
            extract_shared(
                &mut context,
                &maps(&rows),
                &mut Stage::default(),
                SharedLimits::default()
            )
            .err(),
            Some(Error::Collision(collision::Error::Limit))
        );
        assert_eq!(
            context.finish().err(),
            Some(gameobjects::Error::Collision(collision::Error::Limit))
        );
    }
}
#[test]
fn component_reductions_fail_whole_run_and_poison() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[], true, None)),
            (201, None, adt(10, 1)),
        ],
    );
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    for variant in 0..8 {
        let mut limits = SharedLimits::default();
        match variant {
            0 => limits.max_maps = 1,
            1 => limits.max_tile_positions = 8191,
            2 => limits.max_file_bytes = 1,
            3 => limits.max_wdt_memory_bytes = 1,
            4 => limits.max_cache_memory_bytes = 0,
            5 => limits.max_output_bytes = 0,
            6 => limits.placements.max_memory_bytes = 0,
            _ => limits.objects.max_memory_bytes = 0,
        }
        let mut store = fixture.open(false);
        let mut context = startup(&mut store);
        assert!(
            extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits).is_err(),
            "variant {variant}"
        );
        assert!(context.finish().is_err());
    }
}
#[test]
fn external_lookup_reservations_survive_success_and_are_not_released_by_runner() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    context.collector_mut().reserve_external(12345).unwrap();
    let before = context.collector_mut().external;
    extract_shared(
        &mut context,
        &maps(&[]),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(context.collector_mut().external, before);
    context.collector_mut().release_external(12345).unwrap();
    context.finish().unwrap();
}
#[test]
fn final_store_guard_after_sink_mutation_poison_preserves_input_changed() {
    struct Change<'a>(&'a CollisionFixture);
    impl Sink for Change<'_> {
        fn model(&mut self, _: &Model) -> Result<(), SinkError> {
            self.0.change_metadata();
            Ok(())
        }
        fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
            Ok(())
        }
    }
    let fixture = fixture(&[(1, 10)], vec![(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut Change(&fixture),
            SharedLimits::default()
        )
        .err(),
        Some(Error::Collision(collision::Error::Store(
            local_store::Error::InputChanged
        )))
    );
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::Store(
            local_store::Error::InputChanged
        )))
    );
}
#[test]
fn invalid_upward_limits_are_terminal_before_any_world_read() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let before = context.collector_mut().facts();
    let limits = SharedLimits {
        max_maps: MAX_MAPS + 1,
        ..SharedLimits::default()
    };
    assert_eq!(
        extract_shared(&mut context, &maps(&[]), &mut Stage::default(), limits),
        Err(Error::InvalidLimits)
    );
    assert_eq!(context.collector_mut().facts(), before);
    assert_aborted(context);
}

#[test]
fn zero_wdt_parser_memory_accepts_borrowed_tables_but_rejects_global_descriptors() {
    let rows = [row(1, 101, -1)];
    let no_globals = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let mut store = no_globals.open(false);
    let mut context = startup(&mut store);
    let mut limits = SharedLimits::default();
    limits.wdt.max_memory_bytes = 0;
    extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits).unwrap();
    context.finish().unwrap();

    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 100)))));
    let with_globals = fixture(&[], files);
    let mut store = with_globals.open(false);
    let mut context = startup(&mut store);
    let control = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(control.run.placement_records, 2);
    context.finish().unwrap();
    let mut store = with_globals.open(false);
    let mut context = startup(&mut store);
    assert!(matches!(
        extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits),
        Err(Error::Plan(_))
    ));
    assert_aborted(context);
}

#[test]
fn ninth_parent_tile_grows_cache_descriptors_with_overlap_then_replays_all_ids() {
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let tiles: Vec<_> = (0..9).map(|y| (0, y, 201 + y as u32)).collect();
    let mut files = vec![
        (101, None, wdt(&tiles, true, None)),
        (102, None, wdt(&[], true, None)),
        (10, None, m2(true)),
    ];
    files.extend((0..9).map(|n| (201 + n, None, adt(10, 100 + n))));
    let fixture = fixture(&[], files);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let before_external = context.collector_mut().external;
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!((report.run.cached_tiles, report.run.replayed_tiles), (9, 9));
    assert_eq!(report.run.ids.calls, 9);
    assert_eq!(report.run.model_reads, 1);
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(report.collector_after.reads, 13);
    let outputs = tile_outputs(&stage);
    assert_eq!(outputs.len(), 18);
    for (own, inherited) in outputs[..9].iter().zip(&outputs[9..]) {
        assert_eq!(ids(&own.1), ids(&inherited.1));
        assert_eq!(own.0.map_id, 1);
        assert_eq!(inherited.0.map_id, 2);
        assert_eq!(own.1[0], 0);
        assert_eq!(inherited.1[0], 2);
        assert_eq!(&own.1[1..], &inherited.1[1..]);
    }
    // The ninth insertion allocates sixteen descriptors while the old eight
    // are still live. Cache values also remain live during that growth.
    assert_eq!(
        report.run.cache_peak_bytes - report.run.cache_resident_bytes,
        8 * size_of::<TileCache>()
    );
    assert_eq!(context.collector_mut().external, before_external);
    assert_eq!(context.collector_mut().external_sources, 0);
    context.finish().unwrap();

    // The exact measured overlap ceiling succeeds. One byte less fails even
    // though the eventual resident cache would fit: no earlier tile is success.
    let ceiling = report.run.cache_peak_bytes;
    assert!(ceiling > report.run.cache_resident_bytes);
    for cap in [ceiling, ceiling - 1] {
        let mut store = fixture.open(false);
        let mut context = startup(&mut store);
        let mut stage = Stage::default();
        let result = extract_shared(
            &mut context,
            &maps(&rows),
            &mut stage,
            SharedLimits {
                max_cache_memory_bytes: cap,
                ..SharedLimits::default()
            },
        );
        if cap == ceiling {
            let report = result.unwrap();
            assert_eq!(report.run.cache_peak_bytes, ceiling);
            assert_eq!(report.run.replayed_tiles, 9);
            context.finish().unwrap();
        } else {
            assert_eq!(result, Err(Error::Limit));
            assert_eq!(tile_outputs(&stage).len(), 9);
            assert_aborted(context);
        }
    }
}

#[test]
fn missing_wdt_reject_is_unchanged_and_root_absence_is_explicit() {
    let rows = [row(1, 101, -1)];
    let (strict, stage) = run_policy(fixture(&[], vec![]), &rows, MissingWdtPolicy::Reject);
    assert_eq!(
        strict,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::MissingFile
        )))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let (result, stage) = run_policy(
        fixture(&[], vec![]),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    let (report, absent) = result.unwrap();
    assert_eq!(
        absent,
        [RootAbsent {
            map_id: 1,
            wdt_file_data_id: 101,
        }]
    );
    assert_eq!((report.run.maps, report.run.wdt_files), (1, 0));
    assert_eq!(report.run.maps, report.run.wdt_files + absent.len() as u64);
    assert_eq!(report.run.tile_positions, 4096);
    assert_eq!((report.run.ids.calls, report.run.model_callbacks), (0, 0));
    assert_eq!((report.run.object_adts, report.run.own_tiles), (0, 0));
    assert_eq!(
        (report.collector_before.reads, report.collector_after.reads),
        (1, 2)
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[test]
fn root_absent_parent_blocks_replay_and_emits_no_parent_only_work() {
    let rows = [row(1, 101, -1), row(2, 202, 1), row(3, 303, 2)];
    let files = vec![
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (201, None, adt(10, 7)),
        (303, None, wdt(&[], true, None)),
        (10, None, m2(true)),
    ];
    let (result, stage) = run_policy(
        fixture(&[], files),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    let (report, absent) = result.unwrap();
    assert_eq!(
        absent,
        [RootAbsent {
            map_id: 2,
            wdt_file_data_id: 202,
        }]
    );
    assert_eq!((report.run.maps, report.run.wdt_files), (3, 2));
    assert_eq!((report.run.own_tiles, report.run.replayed_tiles), (1, 0));
    assert_eq!((report.run.cached_tiles, report.run.ids.calls), (1, 1));
    assert_eq!((report.run.model_reads, report.run.model_callbacks), (1, 1));
    let contexts: Vec<_> = stage
        .placements
        .iter()
        .map(|(context, _)| (context.map_id, context.replayed))
        .collect();
    assert_eq!(contexts, [(1, false), (3, false), (1, false)]);
    assert!(contexts.iter().all(|(map, _)| *map != 2));
    let tiles = tile_outputs(&stage);
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0].0.map_id, 1);
    assert!(!tiles[0].0.replayed);
}

#[test]
fn present_empty_map_keeps_its_global_proof_and_has_no_omission() {
    let rows = [row(1, 101, -1)];
    let files = vec![(101, None, wdt(&[], true, None))];
    let (result, stage) = run_policy(
        fixture(&[], files),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    let (report, absent) = result.unwrap();
    assert!(absent.is_empty());
    assert_eq!((report.run.maps, report.run.wdt_files), (1, 1));
    assert_eq!(
        (report.run.placement_callbacks, report.run.ids.calls),
        (1, 0)
    );
    assert_eq!(stage.placements.len(), 1);
    assert_eq!(stage.placements[0].0.kind, Kind::Global);
    assert!(stage.placements[0].1.is_empty());
}

#[test]
fn wdt_zero_is_inactive_not_a_root_absence_declaration() {
    let rows = [row(7, 0, -1)];
    let (result, stage) = run_policy(
        fixture(&[], vec![]),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    let (report, absent) = result.unwrap();
    assert_eq!((report.run.maps, report.run.wdt_files), (0, 0));
    assert_eq!(report.run.tile_positions, 0);
    assert!(absent.is_empty());
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn root_present_or_encrypted_wdt_is_never_declared_absent() {
    let rows = [row(1, 101, -1)];
    let files = || {
        vec![
            (101, None, wdt(&[], true, None)),
            (gameobjects::TABLE_FILE_DATA_ID, None, db2(&[], &[])),
        ]
    };
    for (name, make) in [
        (
            "ineligible",
            CollisionFixture::with_ineligible_root
                as fn(&[CollisionFile<'_>], u32, u32) -> CollisionFixture,
        ),
        ("central", CollisionFixture::with_central_absence),
        ("encoding", CollisionFixture::with_encoding_absence),
    ] {
        let (result, stage) = run_policy(
            make(&files(), 70205, 101),
            &rows,
            MissingWdtPolicy::DeclareRootAbsent,
        );
        assert!(
            matches!(result, Err(Error::Collision(collision::Error::Store(_)))),
            "{name}"
        );
        assert!(stage.placements.is_empty(), "{name}");
    }

    let encrypted =
        CollisionFixture::with_available_holes_for_build(&files(), &[(101, 0..1)], 70205);
    let (result, stage) = run_policy(encrypted, &rows, MissingWdtPolicy::DeclareRootAbsent);
    assert!(matches!(
        result,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::NoLocalEncoding(_)
        )))
    ));
    assert!(stage.placements.is_empty());
}

#[test]
fn malformed_wdt_missing_adt_and_missing_model_remain_fatal() {
    let rows = [row(1, 101, -1)];
    let (malformed, stage) = run_policy(
        fixture(&[], vec![(101, None, b"broken".to_vec())]),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    assert!(malformed.is_err());
    assert!(stage.placements.is_empty());

    let (missing_adt, _) = run_policy(
        fixture(&[], vec![(101, None, wdt(&[(0, 0, 201)], true, None))]),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    assert_eq!(
        missing_adt,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::MissingFile
        )))
    );

    let (missing_model, _) = run_policy(
        fixture(
            &[],
            vec![
                (101, None, wdt(&[(0, 0, 201)], true, None)),
                (201, None, adt(10, 7)),
            ],
        ),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    assert_eq!(
        missing_model,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::MissingFile
        )))
    );
}

#[test]
fn root_absence_descriptors_share_the_existing_wdt_memory_ceiling() {
    let rows = [row(1, 101, -1)];
    let ceiling = size_of::<SharedMapState>() + size_of::<RootAbsent>() + 1;
    for cap in [ceiling, ceiling - 1] {
        let (result, stage) = run_policy_with_limits(
            fixture(&[], vec![]),
            &rows,
            MissingWdtPolicy::DeclareRootAbsent,
            SharedLimits {
                max_wdt_memory_bytes: cap,
                ..SharedLimits::default()
            },
        );
        if cap == ceiling {
            let (report, absent) = result.unwrap();
            assert_eq!(absent.len(), 1);
            assert!(absent.capacity() * size_of::<RootAbsent>() <= ceiling);
            assert_eq!(report.run.wdt_resident_bytes, size_of::<SharedMapState>());
            assert_eq!(report.run.maps, report.run.wdt_files + 1);
        } else {
            assert!(result.is_err());
            assert!(stage.placements.is_empty());
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn root_absence_diagnostic_tolerates_only_the_proven_raw_absence() {
    use crate::local_store::{
        ProductDiagnosticPhase as Phase, ProductDiagnosticRequest as Request,
        ProductDiagnosticRequestKind as RequestKind,
    };

    let rows = [row(1, 101, -1)];
    let (result, _, requests, absences) = run_policy_observed(
        fixture(&[], vec![]),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    result.unwrap();
    let expected = [Request {
        phase: Phase::Placements,
        request_kind: RequestKind::Wdt,
        file_data_id: Some(101),
        ordinal: 0,
    }];
    assert_eq!(requests.borrow().as_slice(), expected);
    assert_eq!(absences.borrow().as_slice(), expected);

    let files = [
        (101, None, wdt(&[], true, None)),
        (gameobjects::TABLE_FILE_DATA_ID, None, db2(&[], &[])),
    ];
    let fixture = CollisionFixture::with_ineligible_root(&files, 70205, 101);
    let (result, _, requests, absences) =
        run_policy_observed(fixture, &rows, MissingWdtPolicy::DeclareRootAbsent);
    assert!(result.is_err());
    assert_eq!(requests.borrow().as_slice(), expected);
    assert!(absences.borrow().is_empty());
}

#[test]
fn wmo_doodad_root_absence_skips_only_missing_children_and_deduplicates() {
    let rows = [row(1, 101, -1)];
    let files = wmo_children(&[20, 20, 21], &[(21, m2(true))]);
    let (result, stage) = run_omissions(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
    );
    let (report, root_absent, missing_m2) = result.unwrap();
    assert!(root_absent.is_empty());
    assert_eq!(missing_m2, [20]);
    assert_eq!((report.run.maps, report.run.wdt_files), (1, 1));
    assert_eq!(report.run.model_reads, 3);
    assert_eq!(
        (report.run.model_callbacks, report.run.collision_models),
        (2, 2)
    );
    assert_eq!((report.run.placement_records, report.run.ids.calls), (2, 2));
    assert_eq!(
        report.collector_after.dependency_requests - report.collector_before.dependency_requests,
        3
    );
    assert_eq!(report.request_calls_after - report.request_calls_before, 1);
    assert_eq!(stage.models.len(), 2);
    assert!(
        stage
            .models
            .iter()
            .all(|(status, raw)| *status == Status::Collision && raw.is_some())
    );
    assert_eq!(stage.placements.len(), 1);
    let placements = DirBin::parse(&stage.placements[0].1, placement::Limits::default()).unwrap();
    let mut names: Vec<_> = placements
        .records()
        .map(|record| record.name().to_vec())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [b"FILE0000000A.xxx".to_vec(), b"FILE00000015.xxx".to_vec()]
    );
    assert!(!names.contains(&b"FILE00000014.xxx".to_vec()));
}

#[test]
fn intrinsic_empty_wmo_is_reported_separately_and_preserves_handles_without_output() {
    let files = intrinsic_empty_wmo_fixture();
    let (result, stage) = empty_wmo_run(files.clone(), SharedLimits::default());
    let (report, root_absent, missing_m2, missing_wmo, empty_wmo) = result.unwrap();
    assert!(root_absent.is_empty());
    assert!(missing_m2.is_empty());
    assert!(missing_wmo.is_empty());
    assert_eq!(empty_wmo, [10]);
    assert_eq!(stage.models, [(Status::Vacated, None)]);
    assert_eq!(stage.placements.len(), 1);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!((report.run.placement_records, report.run.ids.calls), (0, 0));

    let (repeat, repeated_stage) = empty_wmo_run(files, SharedLimits::default());
    assert_eq!(repeat.unwrap().4, [10]);
    assert_eq!(repeated_stage.models, stage.models);
    assert_eq!(repeated_stage.placements, stage.placements);
}

#[test]
fn intrinsic_empty_wmo_and_missing_child_are_distinct_and_both_allowed_only_in_v34() {
    let empty_files = intrinsic_empty_wmo_fixture();
    let missing_files = missing_child_wmo_fixture();
    let mixed = empty_files
        .into_iter()
        .filter(|file| file.0 != 101)
        .chain(missing_files.into_iter().filter(|file| file.0 != 101))
        .chain([(101, None, wdt_globals(&[], true, &[(10, 8), (11, 9)]))])
        .collect();
    let (result, stage) = run_empty_wmo(
        fixture(&[], mixed),
        &[row(1, 101, -1)],
        SharedLimits::default(),
    );
    let (_, absent, missing_m2, missing_wmo, empty_wmo) = result.unwrap();
    assert!(absent.is_empty());
    assert_eq!(missing_m2, [20]);
    assert_eq!(missing_wmo, [11]);
    assert_eq!(empty_wmo, [10]);
    assert!(missing_wmo.windows(2).all(|ids| ids[0] < ids[1]));
    assert!(empty_wmo.windows(2).all(|ids| ids[0] < ids[1]));
    assert!(missing_wmo.iter().all(|id| !empty_wmo.contains(id)));
    assert_eq!(
        stage.models,
        [(Status::Vacated, None), (Status::Vacated, None)]
    );
    assert!(stage.placements.iter().all(|(_, bytes)| bytes.is_empty()));

    let (_, old_stage) = run_vacated_wmo(
        fixture(
            &[],
            empty_wmo_fixture(10, vec![(30, vec![0; 68])], &[], &[], true, false),
        ),
        &[row(1, 101, -1)],
        SharedLimits::default(),
    );
    assert!(old_stage.models.is_empty());
}

#[test]
fn intrinsic_empty_wmo_requires_every_source_condition_and_fails_useful_residue() {
    let rows = [row(1, 101, -1)];
    let missing_set = empty_wmo_fixture(10, vec![(30, vec![0; 68])], &[], &[], false, false);
    let mut empty_with_child_request =
        empty_wmo_fixture(10, vec![(30, vec![0; 68])], &[20], &[], true, false);
    empty_with_child_request.push((20, None, m2(false)));
    let mut unreferenced_missing_child = missing_child_wmo_fixture();
    let group = unreferenced_missing_child
        .iter_mut()
        .find(|file| file.0 == 31)
        .unwrap();
    group.2 = chunk(b"MOGP", &[0; 68]);
    let mut referenced_missing_child = missing_child_wmo_fixture();
    referenced_missing_child.retain(|file| file.0 != 20);
    let mut useful_group_residue = intrinsic_empty_wmo_fixture();
    useful_group_residue
        .iter_mut()
        .find(|file| file.0 == 30)
        .unwrap()
        .2 = chunk(b"MOGP", &{
        let mut group = vec![0; 68];
        put(&mut group, 56, 99);
        let liquid = vec![0; 30];
        group.extend(chunk(b"MLIQ", &liquid));
        group
    });
    for files in [
        missing_set,
        empty_with_child_request,
        unreferenced_missing_child,
        useful_group_residue,
    ] {
        let (result, stage) = run_empty_wmo(fixture(&[], files), &rows, SharedLimits::default());
        assert_eq!(
            result.err(),
            Some(Error::Collision(collision::Error::NoCollision))
        );
        assert!(stage.models.is_empty() && stage.placements.is_empty());
    }
    let (result, stage) = run_empty_wmo(
        fixture(&[], referenced_missing_child),
        &rows,
        SharedLimits::default(),
    );
    let (_, _, missing_m2, missing_wmo, empty_wmo) = result.unwrap();
    assert_eq!(missing_m2, [20]);
    assert_eq!(missing_wmo, [11]);
    assert!(empty_wmo.is_empty());
    assert_eq!(stage.models, [(Status::Vacated, None)]);
    assert_eq!(stage.placements.len(), 1);
    assert!(stage.placements[0].1.is_empty());
}

#[test]
fn intrinsic_empty_wmo_rejects_logical_origin_and_both_declaration_limits() {
    let name = b"A/empty.wmo";
    let mut wmo_files = intrinsic_empty_wmo_fixture();
    let root = wmo_files.iter_mut().find(|file| file.0 == 10).unwrap();
    root.0 = 201;
    root.1 = Some(name.as_slice());
    wmo_files.retain(|file| file.0 != 101);
    let mut files = vec![(101, None, named_wmo_wdt(name, 8))];
    files.extend(wmo_files);
    let (result, stage) = run_empty_wmo_with_names(
        fixture(&[], files),
        &[row(1, 101, -1)],
        SharedLimits::default(),
        true,
    );
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let files = intrinsic_empty_wmo_fixture();
    let limits = SharedLimits {
        placements: placements::Limits {
            max_models: 1,
            ..placements::Limits::default()
        },
        max_wdt_memory_bytes: std::mem::size_of::<SharedMapState>()
            + std::mem::size_of::<RootAbsent>()
            + global_wdt(10, 8).capacity()
            + 3 * std::mem::size_of::<u32>(),
        ..SharedLimits::default()
    };
    assert!(empty_wmo_run(files.clone(), limits).0.is_ok());
    let (limited, stage) = empty_wmo_run(
        files,
        SharedLimits {
            max_wdt_memory_bytes: limits.max_wdt_memory_bytes - 1,
            ..limits
        },
    );
    assert_eq!(limited.err(), Some(Error::Limit));
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v35_unselected_geometry_is_separate_from_intrinsic_empty_and_preserves_handles() {
    let rows = [row(1, 101, -1)];
    let files = unselected_geometry_fixture(10, true, true, false);
    let (result, stage) = run_unselected_wmo(
        fixture(&[], files.clone()),
        &rows,
        SharedLimits::default(),
        false,
    );
    let (report, root_absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo) = result.unwrap();
    assert!(root_absent.is_empty());
    assert!(missing_m2.is_empty());
    assert!(missing_wmo.is_empty());
    assert!(empty_wmo.is_empty());
    assert_eq!(unselected_wmo, [10]);
    assert_eq!(stage.models, [(Status::Vacated, None)]);
    assert_eq!(stage.placements.len(), 1);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!((report.run.placement_records, report.run.ids.calls), (0, 0));

    let (repeat, repeated_stage) =
        run_unselected_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
    assert_eq!(repeat.unwrap().5, [10]);
    assert_eq!(repeated_stage.models, stage.models);
    assert_eq!(repeated_stage.placements, stage.placements);
}

#[cfg(target_os = "linux")]
#[test]
fn v35_empty_output_with_selected_geometry_or_missing_sets_remains_fatal() {
    let rows = [row(1, 101, -1)];
    let (selected, stage) = run_unselected_wmo(
        fixture(&[], unselected_geometry_fixture(10, false, true, false)),
        &rows,
        SharedLimits::default(),
        false,
    );
    let (_, _, _, _, empty_wmo, unselected_wmo) = selected.unwrap();
    assert!(empty_wmo.is_empty() && unselected_wmo.is_empty());
    assert_eq!(stage.models.len(), 1);
    assert_eq!(stage.models[0].0, Status::Collision);
    assert!(stage.models[0].1.is_some());
    assert!(stage.placements.iter().any(|(_, bytes)| !bytes.is_empty()));

    let mut filtered_geometry = unselected_geometry_fixture(10, false, true, false);
    let group = &mut filtered_geometry
        .iter_mut()
        .find(|file| file.0 == 30)
        .unwrap()
        .2;
    group[84..86].copy_from_slice(&0u16.to_le_bytes());
    let (filtered, stage) = run_unselected_wmo(
        fixture(&[], filtered_geometry),
        &rows,
        SharedLimits::default(),
        false,
    );
    assert_eq!(
        filtered.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let (missing_set, stage) = run_unselected_wmo(
        fixture(&[], unselected_geometry_fixture(10, true, false, false)),
        &rows,
        SharedLimits::default(),
        false,
    );
    assert_eq!(
        missing_set.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v35_doodad_binding_and_dependency_failures_are_not_reclassified() {
    let rows = [row(1, 101, -1)];
    let selected = unselected_geometry_fixture(10, true, true, true);
    let (result, stage) = run_unselected_wmo(
        fixture(&[], selected),
        &rows,
        SharedLimits::default(),
        false,
    );
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut named = unselected_geometry_fixture(201, true, true, false);
    let root = named.iter_mut().find(|file| file.0 == 201).unwrap();
    root.0 = 202;
    root.1 = Some(b"A/unselected.wmo");
    named.retain(|file| file.0 != 101);
    named.push((101, None, named_wmo_wdt(b"A/unselected.wmo", 8)));
    let (result, stage) =
        run_unselected_wmo(fixture(&[], named), &rows, SharedLimits::default(), true);
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v35_unselected_list_obeys_count_and_memory_limits() {
    let rows = [row(1, 101, -1)];
    let files = unselected_geometry_fixture(10, true, true, false);
    let maximum = placements::Limits {
        max_models: 1,
        ..placements::Limits::default()
    };
    let wdt_bytes = 40 + 32_776 + 131_080 + 72;
    let accepted = SharedLimits {
        placements: maximum,
        max_wdt_memory_bytes: std::mem::size_of::<SharedMapState>()
            + std::mem::size_of::<RootAbsent>()
            + wdt_bytes
            + 4 * std::mem::size_of::<u32>(),
        ..SharedLimits::default()
    };
    let (result, _) = run_unselected_wmo(fixture(&[], files.clone()), &rows, accepted, false);
    assert!(result.is_ok(), "{result:?}");
    let (limited, stage) = run_unselected_wmo(
        fixture(&[], files),
        &rows,
        SharedLimits {
            max_wdt_memory_bytes: accepted.max_wdt_memory_bytes - 1,
            ..accepted
        },
        false,
    );
    assert_eq!(limited.err(), Some(Error::Limit));
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v36_material_filtered_geometry_is_a_disjoint_fdid_omission() {
    let rows = [row(1, 101, -1)];
    for wide_materials in [false, true] {
        let (first_flags, second_flags) = if wide_materials {
            (0x8126, 0x8166)
        } else {
            (0x26, 0x66)
        };
        let files = material_filtered_wmo_fixture_format(
            10,
            &[(first_flags, false), (second_flags, false)],
            true,
            &[],
            false,
            wide_materials,
        );
        let first = parsed_materials(&files, 30);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].flags, first_flags);
        let second = parsed_materials(&files, 31);
        assert_eq!(second[0].flags, second_flags);
        if wide_materials {
            assert_eq!(first[0].material, 0xabcd);
            assert_eq!(second[0].material, 0xabcd);
        } else {
            assert_eq!(first[0].material, 0);
            assert_eq!(second[0].material, 0);
        }
        let (result, stage) =
            run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
        let (_, _, _, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
        assert!(missing_wmo.is_empty() && empty_wmo.is_empty() && unselected_wmo.is_empty());
        assert_eq!(filtered, [10]);
        assert_eq!(stage.models, [(Status::Vacated, None)]);
        assert!(stage.placements.iter().all(|(_, bytes)| bytes.is_empty()));
    }

    for flags in [0x8020, 0x8008] {
        let files =
            material_filtered_wmo_fixture_format(10, &[(flags, false)], true, &[], false, true);
        let parsed = parsed_materials(&files, 30);
        assert_eq!(parsed[0].flags, flags);
        assert_eq!(parsed[0].material, 0xabcd);
        assert!(!collision::material_filtered(parsed[0].flags));
        let (result, stage) =
            run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
        let (_, _, _, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
        assert!(missing_wmo.is_empty() && empty_wmo.is_empty() && unselected_wmo.is_empty());
        assert!(filtered.is_empty());
        assert_eq!(stage.models[0].0, Status::Collision);
        assert!(stage.models[0].1.is_some());
        assert!(stage.placements.iter().any(|(_, bytes)| !bytes.is_empty()));
    }

    let files =
        material_filtered_wmo_fixture(10, &[(0x26, false), (0x66, false)], true, &[], false);
    let (result, stage) = run_material_filtered_wmo(
        fixture(&[], files.clone()),
        &rows,
        SharedLimits::default(),
        false,
    );
    let (report, absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo, filtered) =
        result.unwrap();
    assert!(absent.is_empty());
    assert!(missing_m2.is_empty());
    assert!(missing_wmo.is_empty());
    assert!(empty_wmo.is_empty());
    assert!(unselected_wmo.is_empty());
    assert_eq!(filtered, [10]);
    assert_eq!(stage.models, [(Status::Vacated, None)]);
    assert_eq!(stage.placements.len(), 1);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!((report.run.placement_records, report.run.ids.calls), (0, 0));

    let (repeat, repeated_stage) =
        run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
    assert_eq!(repeat.unwrap().6, [10]);
    assert_eq!(repeated_stage.models, stage.models);
    assert_eq!(repeated_stage.placements, stage.placements);

    let empty = material_filtered_wmo_fixture(10, &[], true, &[], false);
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], empty), &rows, SharedLimits::default(), false);
    let (_, _, _, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
    assert!(missing_wmo.is_empty() && unselected_wmo.is_empty() && filtered.is_empty());
    assert_eq!(empty_wmo, [10]);
    assert_eq!(stage.models, [(Status::Vacated, None)]);
}

#[cfg(target_os = "linux")]
#[test]
fn v36_material_filtered_list_obeys_count_and_memory_limits() {
    let rows = [row(1, 101, -1)];
    let mut files = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    files.retain(|file| file.0 != 101);
    files.push((101, None, wdt(&[], true, Some((10, 8)))));
    let wdt_bytes = 40 + 32_776 + 131_080 + 72;
    let exact = SharedLimits {
        placements: placements::Limits {
            max_models: 1,
            ..placements::Limits::default()
        },
        max_wdt_memory_bytes: std::mem::size_of::<SharedMapState>()
            + std::mem::size_of::<RootAbsent>()
            + wdt_bytes
            + 5 * std::mem::size_of::<u32>(),
        ..SharedLimits::default()
    };
    let (accepted, _) = run_material_filtered_wmo(fixture(&[], files.clone()), &rows, exact, false);
    assert_eq!(accepted.unwrap().6, [10]);
    let (limited, stage) = run_material_filtered_wmo(
        fixture(&[], files),
        &rows,
        SharedLimits {
            max_wdt_memory_bytes: exact.max_wdt_memory_bytes - 1,
            ..exact
        },
        false,
    );
    assert_eq!(limited.err(), Some(Error::Limit));
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut files = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    files.retain(|file| file.0 != 101);
    let mut second = material_filtered_wmo_fixture(11, &[(0x26, false)], true, &[], false);
    second.retain(|file| file.0 != 101);
    let root = second.iter_mut().find(|file| file.0 == 11).unwrap();
    let group_ids = root.2.windows(4).position(|tag| tag == b"DIFG").unwrap();
    put(&mut root.2, group_ids + 8, 31);
    second.iter_mut().find(|file| file.0 == 30).unwrap().0 = 31;
    files.extend(second);
    files.push((101, None, wdt_globals(&[], true, &[(10, 8), (11, 9)])));
    let (limited, stage) = run_material_filtered_wmo(
        fixture(&[], files),
        &rows,
        SharedLimits {
            placements: placements::Limits {
                max_models: 1,
                ..placements::Limits::default()
            },
            ..SharedLimits::default()
        },
        false,
    );
    assert_eq!(
        limited.err(),
        Some(Error::Collision(collision::Error::Limit))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v36_repeated_global_reference_reuses_the_material_filtered_handle() {
    let rows = [row(1, 101, -1)];
    let mut files = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    files.retain(|file| file.0 != 101);
    files.push((101, None, wdt_globals(&[], true, &[(10, 8), (10, 9)])));
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
    let (report, absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo, filtered) =
        result.unwrap();
    assert!(absent.is_empty());
    assert!(missing_m2.is_empty() && missing_wmo.is_empty());
    assert!(empty_wmo.is_empty() && unselected_wmo.is_empty());
    assert_eq!(filtered, [10]);
    assert_eq!(report.request_calls_after - report.request_calls_before, 2);
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(stage.models, [(Status::Vacated, None)]);
}

#[cfg(target_os = "linux")]
#[test]
fn v36_omission_categories_remain_disjoint() {
    let rows = [row(1, 101, -1)];
    let mut files = intrinsic_empty_wmo_fixture();
    files.retain(|file| file.0 != 101);
    let mut missing = missing_child_wmo_fixture();
    missing.retain(|file| file.0 != 101);
    files.extend(missing);
    let mut unselected = unselected_geometry_fixture(12, true, true, false);
    unselected.retain(|file| file.0 != 101);
    let root = unselected.iter_mut().find(|file| file.0 == 12).unwrap();
    let group_ids = root.2.windows(4).position(|tag| tag == b"DIFG").unwrap();
    put(&mut root.2, group_ids + 8, 32);
    unselected.iter_mut().find(|file| file.0 == 30).unwrap().0 = 32;
    files.extend(unselected);
    let mut filtered = material_filtered_wmo_fixture(13, &[(0x26, false)], true, &[], false);
    filtered.retain(|file| file.0 != 101);
    let root = filtered.iter_mut().find(|file| file.0 == 13).unwrap();
    let group_ids = root.2.windows(4).position(|tag| tag == b"DIFG").unwrap();
    put(&mut root.2, group_ids + 8, 33);
    filtered.iter_mut().find(|file| file.0 == 30).unwrap().0 = 33;
    files.extend(filtered);
    files.push((
        101,
        None,
        wdt_globals(&[], true, &[(10, 8), (11, 9), (12, 10), (13, 11)]),
    ));

    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
    let (_, absent, missing_m2, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
    assert!(absent.is_empty());
    assert_eq!(missing_m2, [20]);
    assert_eq!(missing_wmo, [11]);
    assert_eq!(empty_wmo, [10]);
    assert_eq!(unselected_wmo, [12]);
    assert_eq!(filtered, [13]);
    let categories: [&[u32]; 5] = [
        &missing_m2,
        &missing_wmo,
        &empty_wmo,
        &unselected_wmo,
        &filtered,
    ];
    for (index, category) in categories.iter().enumerate() {
        for other in &categories[index + 1..] {
            assert!(category.iter().all(|id| !other.contains(id)));
        }
    }
    assert_eq!(stage.models.len(), 4);
    assert!(
        stage
            .models
            .iter()
            .all(|model| *model == (Status::Vacated, None))
    );
    assert!(stage.placements.iter().all(|(_, bytes)| bytes.is_empty()));
}

#[cfg(target_os = "linux")]
#[test]
fn v36_requires_every_material_filtered_condition_and_keeps_errors_fatal() {
    let rows = [row(1, 101, -1)];
    let useful = material_filtered_wmo_fixture(10, &[(0x20, false)], true, &[], false);
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], useful), &rows, SharedLimits::default(), false);
    let (_, _, _, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
    assert!(missing_wmo.is_empty() && empty_wmo.is_empty());
    assert!(unselected_wmo.is_empty() && filtered.is_empty());
    assert_eq!(stage.models[0].0, Status::Collision);
    assert!(stage.models[0].1.is_some());
    assert!(stage.placements.iter().any(|(_, bytes)| !bytes.is_empty()));

    let skipped = material_filtered_wmo_fixture(10, &[(0x26, true)], true, &[], false);
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], skipped), &rows, SharedLimits::default(), false);
    let (_, _, _, missing_wmo, empty_wmo, unselected_wmo, filtered) = result.unwrap();
    assert!(missing_wmo.is_empty() && empty_wmo.is_empty());
    assert_eq!(unselected_wmo, [10]);
    assert!(filtered.is_empty());
    assert_eq!(stage.models, [(Status::Vacated, None)]);

    let cases = [
        material_filtered_wmo_fixture(10, &[(0x26, false), (0x26, true)], true, &[], false),
        material_filtered_wmo_fixture(10, &[(0x26, false)], false, &[], false),
        material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], true),
    ];
    for files in cases {
        let (result, stage) =
            run_material_filtered_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
        assert_eq!(
            result.err(),
            Some(Error::Collision(collision::Error::NoCollision))
        );
        assert!(stage.models.is_empty() && stage.placements.is_empty());
    }

    let referenced = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[0], false);
    let (result, stage) = run_material_filtered_wmo(
        fixture(&[], referenced),
        &rows,
        SharedLimits::default(),
        false,
    );
    let (_, _, missing_m2, _, _, _, filtered) = result.unwrap();
    assert!(missing_m2.is_empty() && filtered.is_empty());
    assert_eq!(stage.models[0].0, Status::DoodadOnly);

    let mut no_local_m2 = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[0], false);
    no_local_m2.retain(|file| file.0 != 20);
    let (result, stage) = run_material_filtered_wmo(
        fixture(&[], no_local_m2),
        &rows,
        SharedLimits::default(),
        false,
    );
    let (_, _, missing_m2, _, _, _, filtered) = result.unwrap();
    assert_eq!(missing_m2, [20]);
    assert!(filtered.is_empty());
    assert_eq!(stage.models, [(Status::Vacated, None)]);

    let (result, stage) = run_material_filtered_wmo(
        fixture(
            &[],
            material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false),
        ),
        &rows,
        SharedLimits {
            placements: placements::Limits {
                max_models: 0,
                ..placements::Limits::default()
            },
            ..SharedLimits::default()
        },
        false,
    );
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::Limit))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut logical = material_filtered_wmo_fixture(201, &[(0x26, false)], true, &[], false);
    logical.retain(|file| file.0 != 101);
    logical.iter_mut().find(|file| file.0 == 201).unwrap().1 = Some(b"A/filtered.wmo");
    logical.push((101, None, named_wmo_wdt(b"A/filtered.wmo", 8)));
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], logical), &rows, SharedLimits::default(), true);
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut malformed = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    malformed.iter_mut().find(|file| file.0 == 30).unwrap().2 = b"bad group".to_vec();
    let (result, stage) = run_material_filtered_wmo(
        fixture(&[], malformed),
        &rows,
        SharedLimits::default(),
        false,
    );
    assert!(result.is_err());
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut missing = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    missing.retain(|file| file.0 != 30);
    let (result, stage) =
        run_material_filtered_wmo(fixture(&[], missing), &rows, SharedLimits::default(), false);
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::Store(
            local_store::Error::MissingFile
        )))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn v35_and_earlier_policies_do_not_admit_material_filtered_geometry() {
    let rows = [row(1, 101, -1)];
    let files = material_filtered_wmo_fixture(10, &[(0x26, false)], true, &[], false);
    let (result, stage) =
        run_unselected_wmo(fixture(&[], files), &rows, SharedLimits::default(), false);
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[test]
fn material_flags_match_the_production_material_filter() {
    for flags in [0x26, 0x66] {
        assert!(collision::material_filtered(flags));
        assert!(!(flags & 0x08 != 0 || (flags & 0x20 != 0 && flags & 0x04 == 0)));
    }
    let flags = 0x20;
    assert!(!collision::material_filtered(flags));
    assert!(flags & 0x08 != 0 || (flags & 0x20 != 0 && flags & 0x04 == 0));
}

#[test]
fn wmo_with_no_geometry_after_missing_child_is_declared_vacated_without_output() {
    let rows = [row(1, 101, -1)];
    let files = wmo_with_missing_doodad(Some(20), true, false)
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_vacated_wmo(fixture(&[], files), &rows, SharedLimits::default());
    let (report, root_absent, missing_m2, missing_wmo, empty_wmo) = result.unwrap();
    assert!(root_absent.is_empty());
    assert_eq!(missing_m2, [20]);
    assert_eq!(missing_wmo, [10]);
    assert!(empty_wmo.is_empty());
    assert_eq!(stage.models.len(), 1);
    assert_eq!(stage.models[0], (Status::Vacated, None));
    assert_eq!(stage.placements.len(), 1);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!((report.run.placement_records, report.run.ids.calls), (0, 0));
}

#[test]
fn vacated_wmo_requires_every_declared_condition_and_old_policy_stays_fatal() {
    let rows = [row(1, 101, -1)];
    let files = wmo_with_unreferenced_missing_doodad()
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_vacated_wmo(fixture(&[], files), &rows, SharedLimits::default());
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let mut no_missing_child = wmo_with_missing_doodad(Some(20), true, false);
    no_missing_child.push((20, None, m2(false)));
    let files = no_missing_child
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_vacated_wmo(fixture(&[], files), &rows, SharedLimits::default());
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let files = wmo_with_missing_doodad(Some(20), false, false)
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_vacated_wmo(fixture(&[], files), &rows, SharedLimits::default());
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());

    let files = wmo_with_missing_doodad(Some(20), true, true)
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_vacated_wmo(fixture(&[], files), &rows, SharedLimits::default());
    let (_, _, missing_m2, missing_wmo, empty_wmo) = result.unwrap();
    assert_eq!(missing_m2, [20]);
    assert!(missing_wmo.is_empty());
    assert!(empty_wmo.is_empty());
    assert_eq!(stage.models.len(), 1);
    assert_eq!(stage.models[0].0, Status::Collision);
    assert!(stage.models[0].1.is_some());

    let files = wmo_with_missing_doodad(Some(20), true, false)
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let (result, stage) = run_omissions(fixture(&[], files), &rows);
    assert_eq!(
        result.err(),
        Some(Error::Collision(collision::Error::NoCollision))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[test]
fn wmo_doodad_missing_child_remains_fatal_under_previous_policy() {
    let rows = [row(1, 101, -1)];
    let files = wmo_children(&[20, 21], &[(21, m2(true))]);
    let (result, stage) = run_policy(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
        MissingWdtPolicy::DeclareRootAbsent,
    );
    assert_eq!(
        result,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::MissingFile
        )))
    );
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[test]
fn wmo_doodad_no_collision_is_not_reclassified_as_root_absence() {
    let rows = [row(1, 101, -1)];
    let files = wmo_children(&[20], &[(20, m2(false))]);
    let (result, stage) = run_omissions(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
    );
    let (report, _, missing_m2) = result.unwrap();
    assert!(missing_m2.is_empty());
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!(report.run.model_callbacks, 2);
    assert!(
        stage
            .models
            .iter()
            .any(|(status, _)| *status == Status::NoCollision)
    );
}

#[test]
fn wmo_doodad_unsorted_root_absences_are_canonicalized_at_completion() {
    let rows = [row(1, 101, -1)];
    let files = wmo_children(&[20, 19, 21], &[(21, m2(true))]);
    let (result, stage) = run_omissions(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
    );
    let (report, root_absent, missing_m2) = result.unwrap();
    assert!(root_absent.is_empty());
    assert_eq!(missing_m2, [19, 20]);
    assert_eq!(report.run.model_reads, 3);
    assert_eq!(
        (report.run.model_callbacks, report.run.collision_models),
        (2, 2)
    );
    assert_eq!((report.run.placement_records, report.run.ids.calls), (2, 2));
    assert_eq!(
        report.collector_after.dependency_requests - report.collector_before.dependency_requests,
        3
    );
    assert_eq!(report.request_calls_after - report.request_calls_before, 1);
    assert_eq!(stage.models.len(), 2);
    assert_eq!(stage.placements.len(), 1);
    let placements = DirBin::parse(&stage.placements[0].1, placement::Limits::default()).unwrap();
    let mut names: Vec<_> = placements
        .records()
        .map(|record| record.name().to_vec())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [b"FILE0000000A.xxx".to_vec(), b"FILE00000015.xxx".to_vec()]
    );
    assert!(!names.contains(&b"FILE00000014.xxx".to_vec()));
    assert!(!names.contains(&b"FILE00000013.xxx".to_vec()));
}

#[test]
fn wmo_doodad_direct_m2_adt_root_group_and_malformed_sources_are_fatal() {
    let rows = [row(1, 101, -1)];
    for files in [
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (201, None, adt(10, 7)),
        ],
        vec![(101, None, wdt(&[(0, 0, 201)], true, None))],
    ] {
        let (result, _) = run_omissions(fixture(&[], files), &rows);
        assert_eq!(
            result,
            Err(Error::Collision(collision::Error::Store(
                local_store::Error::MissingFile
            )))
        );
    }

    for missing in [10, 30] {
        let mut files = wmo_children(&[20, 21], &[(21, m2(true))]);
        files.retain(|(id, _, _)| *id != missing);
        let (result, _) = run_omissions(
            fixture(
                &[],
                files
                    .into_iter()
                    .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                    .collect(),
            ),
            &rows,
        );
        assert_eq!(
            result,
            Err(Error::Collision(collision::Error::Store(
                local_store::Error::MissingFile
            )))
        );
    }

    let files = wmo_children(&[20, 21], &[(21, b"not an M2".to_vec())]);
    let (result, _) = run_omissions(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
    );
    assert!(result.is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn wmo_doodad_encrypted_present_child_is_not_omitted() {
    let rows = [row(1, 101, -1)];
    let files: Vec<_> = wmo_children(&[20, 21], &[(21, m2(true))])
        .into_iter()
        .chain([
            (101, None, wdt(&[], true, Some((10, 8)))),
            (gameobjects::TABLE_FILE_DATA_ID, None, db2(&[], &[])),
        ])
        .collect();
    let fixture = CollisionFixture::with_available_holes_for_build(&files, &[(21, 0..1)], 70205);
    let (result, _) = run_omissions(fixture, &rows);
    assert!(matches!(
        result,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::NoLocalEncoding(_)
        )))
    ));
}

#[test]
fn wmo_doodad_guard_change_is_fatal() {
    let rows = [row(1, 101, -1)];
    let files: Vec<_> = wmo_children(&[20, 21], &[(21, m2(true))])
        .into_iter()
        .chain([(101, None, wdt(&[], true, Some((10, 8))))])
        .collect();
    let fixture = fixture(&[], files);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    fixture.change_metadata();
    let mut stage = Stage::default();
    let result = extract_shared_with_omissions(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
        MissingWdtPolicy::DeclareRootAbsent,
        MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
    );
    assert_eq!(
        result,
        Err(Error::Collision(collision::Error::Store(
            local_store::Error::InputChanged
        )))
    );
    assert!(context.finish().is_err());
}

#[test]
fn wmo_doodad_omission_limit_is_bounded() {
    let rows = [row(1, 101, -1)];
    for maximum in [2, 1] {
        let files = wmo_children(&[20, 19, 21], &[(21, m2(true))]);
        let (result, _) = run_omissions_with_limits(
            fixture(
                &[],
                files
                    .into_iter()
                    .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                    .collect(),
            ),
            &rows,
            SharedLimits {
                placements: placements::Limits {
                    max_models: maximum,
                    ..placements::Limits::default()
                },
                ..SharedLimits::default()
            },
        );
        if maximum == 2 {
            let (_, _, missing_m2) = result.unwrap();
            assert_eq!(missing_m2, [19, 20]);
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn third_wmo_doodad_omission_hits_limit_before_callbacks() {
    let rows = [row(1, 101, -1)];
    let files = wmo_children(&[20, 19, 18], &[]);
    let (result, stage) = run_omissions_with_limits(
        fixture(
            &[],
            files
                .into_iter()
                .chain([(101, None, wdt(&[], true, Some((10, 8))))])
                .collect(),
        ),
        &rows,
        SharedLimits {
            placements: placements::Limits {
                max_models: 2,
                ..placements::Limits::default()
            },
            ..SharedLimits::default()
        },
    );
    assert_eq!(result, Err(Error::Collision(collision::Error::Limit)));
    assert!(stage.models.is_empty() && stage.placements.is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn model_heavy_shared_accepts_exact_tile_model_inventory_and_rejects_one_over() {
    const MODEL_COUNT: usize = 4100;
    let rows = [row(1, 101, -1)];
    let effective_model_limit = collision::resources::Profile::ModelHeavyV1
        .stage_limits()
        .world
        .world
        .run
        .placements
        .max_models;
    assert_eq!(effective_model_limit, 65_536);
    let effective_limits = SharedLimits {
        placements: placements::Limits {
            max_models: effective_model_limit,
            ..placements::Limits::default()
        },
        ..SharedLimits::default()
    };
    let (result, stage) =
        run_model_heavy_with_limits(model_heavy_fixture(MODEL_COUNT), &rows, effective_limits);
    let report = result.unwrap();
    assert_eq!(report.run.maps, 1);
    assert_eq!(report.run.wdt_files, 1);
    assert_eq!(report.run.object_adts, 1);
    assert_eq!(report.run.own_tiles, 1);
    assert_eq!(report.run.model_callbacks, MODEL_COUNT as u64);
    assert_eq!(report.run.collision_models, MODEL_COUNT as u64);
    assert_eq!(report.run.placement_callbacks, 2);
    assert_eq!(report.run.placement_records, MODEL_COUNT as u64);
    assert_eq!(stage.models.len(), MODEL_COUNT);
    assert_eq!(stage.placements.len(), 2);
    assert_eq!(stage.placements[0].0.kind, Kind::Global);
    assert!(ids(&stage.placements[0].1).is_empty());
    assert_eq!(stage.placements[1].0.kind, Kind::Tile { x: 0, y: 0 });
    assert_eq!(ids(&stage.placements[1].1).len(), MODEL_COUNT);

    let one_over_limits = SharedLimits {
        placements: placements::Limits {
            max_models: MODEL_COUNT,
            ..placements::Limits::default()
        },
        ..SharedLimits::default()
    };
    let (result, stage) =
        run_model_heavy_with_limits(model_heavy_fixture(MODEL_COUNT + 1), &rows, one_over_limits);
    assert_eq!(result, Err(Error::Placements(placements::Error::Limit)));
    assert_eq!(stage.models.len(), MODEL_COUNT + 1);
    assert_eq!(stage.placements.len(), 1);
    assert_eq!(stage.placements[0].0.kind, Kind::Global);
    assert!(ids(&stage.placements[0].1).is_empty());
}
