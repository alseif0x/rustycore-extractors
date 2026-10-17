// SPDX-License-Identifier: GPL-3.0-or-later
// Authored GNU15-v2 integration controls. SOURCE ONLY until fresh qualification.
use super::*;
use crate::{Locale, local_index, local_store, local_store::tests::CollisionFixture};
use forever_vmap::{
    objects::{self, Objects},
    placement::{self, DirBin},
};
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Lookup;
impl LiquidLookup for Lookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
fn m2(collision: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if collision {
        put(&mut bytes, 216, 3);
        put(&mut bytes, 220, 304);
        put(&mut bytes, 224, 3);
        put(&mut bytes, 228, 310);
        for index in [0u16, 1, 2] {
            bytes.extend(index.to_le_bytes());
        }
        bytes.extend(words(&[
            0,
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
fn root(groups: &[u32], children: &[u32], sets: &[(u32, u32)]) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, groups.len() as u32);
    header[60] = 4;
    let mut bytes = chunk(b"MOHD", &header);
    bytes.extend(chunk(b"GFID", &words(groups)));
    bytes.extend(chunk(b"MODI", &words(children)));
    let mut spawns = vec![0; children.len() * 40];
    for index in 0..children.len() {
        put(&mut spawns, index * 40, index as u32);
        put(&mut spawns, index * 40 + 28, 1f32.to_bits());
        put(&mut spawns, index * 40 + 32, 1f32.to_bits());
    }
    bytes.extend(chunk(b"MODD", &spawns));
    let mut ranges = vec![0; sets.len() * 32];
    for (index, &(start, count)) in sets.iter().enumerate() {
        put(&mut ranges, index * 32 + 20, start);
        put(&mut ranges, index * 32 + 24, count);
    }
    bytes.extend(chunk(b"MODS", &ranges));
    bytes
}
fn group(skip: bool, references: &[u16]) -> Vec<u8> {
    let mut header = vec![0; 68];
    put(&mut header, 56, 99);
    put(&mut header, 8, if skip { 0x80 } else { 0 });
    header.extend(chunk(b"MOPY", &[8, 0]));
    header.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    header.extend(chunk(
        b"MOVT",
        &words(&[0, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0]),
    ));
    header.extend(chunk(
        b"MODR",
        &references
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    ));
    chunk(b"MOGP", &header)
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    spool: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-gnu15-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            spool: base.join("spool"),
            base,
        }
    }
    fn spool(&self, fixture: &CollisionFixture) -> Spool {
        Spool::create(
            &self.spool,
            super::super::spool::ProtectedInputs::capture(fixture.path(), &[]).unwrap(),
            super::super::spool::Limits::default(),
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn open(fixture: &CollisionFixture, build: u32) -> LocalStore {
    let options = local_store::OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: crate::root::Layout::Header50893V2,
        limits: local_store::Limits::default(),
    };
    match build {
        70374 => LocalStore::open_db2_70374(fixture.path(), options),
        70425 => LocalStore::open_db2_70425(fixture.path(), options),
        _ => unreachable!(),
    }
    .unwrap()
}
fn handle(outcome: Outcome) -> ModelHandle {
    match outcome {
        Outcome::Present(handle) => handle,
        other => panic!("{other:?}"),
    }
}
fn collect_references(
    root: Vec<u8>,
    groups: &[(u32, Vec<u8>)],
    children: &[u32],
    native: bool,
) -> Vec<u16> {
    let mut files = vec![(100, None, root)];
    files.extend(groups.iter().map(|(id, bytes)| (*id, None, bytes.clone())));
    files.extend(children.iter().map(|id| (*id, None, m2(true))));
    let fixture = CollisionFixture::for_build(&files, 70374);
    let private = Private::new();
    let mut store = open(&fixture, 70374);
    let mut collector = if native {
        Collector::new_native_gnu15_70374(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        )
    } else {
        Collector::new_70374(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        )
    }
    .unwrap();
    let model = handle(
        collector
            .extract(ModelRequest::Wmo(InputRef::FileDataId(100)))
            .unwrap(),
    );
    let references = collector
        .with_payload(model, |payload| {
            assert_eq!(
                payload.profile(),
                if native {
                    payload::COMPATIBLE_GNU15_PROFILE
                } else {
                    payload::COMPATIBLE_PROFILE
                }
            );
            Ok(payload.wmo().unwrap().references().collect())
        })
        .unwrap();
    let mut completed = collector.finish().unwrap();
    let replayed: Vec<_> = completed
        .with_payload(model, |payload| {
            assert_eq!(
                payload.profile(),
                if native {
                    payload::COMPATIBLE_GNU15_PROFILE
                } else {
                    payload::COMPATIBLE_PROFILE
                }
            );
            Ok(payload.wmo().unwrap().references().collect())
        })
        .unwrap();
    assert_eq!(replayed, references);
    references
}
#[test]
fn native_gnu15_v2_nonmonotonic_physical_modr_preserves_v1_sorted_profile() {
    let root = root(&[101], &[10, 11, 12], &[(0, 3)]);
    let groups = [(101, group(false, &[2, 0, 1, 2, 0]))];
    assert_eq!(
        collect_references(root.clone(), &groups, &[10, 11, 12], false),
        [0, 1, 2]
    );
    assert_eq!(
        collect_references(root, &groups, &[10, 11, 12], true),
        [1, 0, 2]
    );
}
#[test]
fn native_gnu15_v2_existing_bucket_chain_keeps_collision_order() {
    let children: Vec<_> = (10..24).collect();
    let root = root(&[101], &children, &[(0, 14)]);
    assert_eq!(
        collect_references(
            root,
            &[(101, group(false, &[0, 1, 13, 1]))],
            &children,
            true
        ),
        [1, 13, 0]
    );
}
#[test]
fn native_gnu15_v2_first_rehash_keeps_gfid_group_order_and_skips_contexts() {
    let children: Vec<_> = (10..24).collect();
    let first: Vec<_> = (0u16..7).collect();
    let mut last: Vec<_> = (7u16..14).collect();
    last.extend([0, 13, u16::MAX]);
    let root = root(&[101, 102, 103], &children, &[(0, 14)]);
    let expected: Vec<_> = [13].into_iter().chain(0u16..13).collect();
    assert_eq!(
        collect_references(
            root,
            &[
                (101, group(false, &first)),
                (102, group(true, &[13])),
                (103, group(false, &last))
            ],
            &children,
            true
        ),
        expected
    );
}
#[derive(Default)]
struct Sink {
    bytes: Vec<u8>,
}
impl placements::Sink for Sink {
    type Error = u8;
    fn stage(&mut self, batch: placements::Staged<'_>) -> Result<(), u8> {
        if !self.bytes.is_empty() || batch.bytes.len() > 8192 {
            return Err(1);
        }
        self.bytes
            .try_reserve_exact(batch.bytes.len())
            .map_err(|_| 2)?;
        if self.bytes.capacity() > 8192 {
            return Err(3);
        }
        self.bytes.extend_from_slice(batch.bytes);
        Ok(())
    }
}
type FixtureFile<'a> = (u32, Option<&'a [u8]>, Vec<u8>);

fn project(files: &[FixtureFile<'_>], build: u32, set: u16, native: bool) -> Vec<(u32, Vec<u8>)> {
    let fixture = CollisionFixture::for_build(files, build);
    let private = Private::new();
    let mut store = open(&fixture, build);
    let mut collector = match (build, native) {
        (70374, true) => Collector::new_native_gnu15_70374(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        ),
        (70425, true) => Collector::new_native_gnu15_70425(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        ),
        (70374, false) => Collector::new_70374(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        ),
        (70425, false) => Collector::new_70425(
            &mut store,
            &Lookup,
            Limits::default(),
            private.spool(&fixture),
        ),
        _ => unreachable!(),
    }
    .unwrap();
    let mut record = [0; 64];
    put(&mut record, 0, 100);
    put(&mut record, 4, 200);
    record[56..58].copy_from_slice(&8u16.to_le_bytes());
    record[58..60].copy_from_slice(&set.to_le_bytes());
    let bytes = chunk(b"MODF", &record);
    let objects = Objects::parse_adt(&bytes, objects::Limits::default()).unwrap();
    let mut session = placements::Session::new(
        &mut collector,
        Sink::default(),
        8192,
        super::super::placements::Limits {
            ids: placement::ids::Limits {
                max_memory_bytes: 1024 * 1024,
                ..placement::ids::Limits::default()
            },
            ..super::super::placements::Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        session.profile(),
        match (build, native) {
            (70374, true) => placements::PROFILE_GNU15_70374,
            (70425, true) => placements::PROFILE_GNU15_70425,
            (70374, false) => placements::PROFILE_70374,
            _ => placements::PROFILE_70425,
        }
    );
    session
        .project_to_sink(&[super::super::placements::Batch {
            objects: &objects,
            map_id: 7,
            original_map_id: 7,
        }])
        .unwrap();
    let entries = DirBin::parse(&session.sink().bytes, placement::Limits::default())
        .unwrap()
        .records()
        .map(|row| (row.id(), row.name().to_vec()))
        .collect();
    session.finish().unwrap();
    let completed = collector.finish().unwrap();
    assert_eq!(
        completed.profile(),
        match (build, native) {
            (70374, true) => PROFILE_GNU15_70374,
            (70425, true) => PROFILE_GNU15_70425,
            (70374, false) => PROFILE_70374,
            _ => PROFILE_70425,
        }
    );
    entries
}
#[test]
fn native_gnu15_v2_n03_child_name_id_association_filters_missing_and_nocollision_before_insert() {
    let files = [
        (100, None, root(&[101], &[10, 19, 20, 40], &[(0, 4)])),
        (101, None, group(false, &[0, 1, 2, 3])),
        (10, None, m2(true)),
        (20, None, m2(false)),
        (40, None, m2(true)),
    ];
    for build in [70374, 70425] {
        let old = project(&files, build, 0, false);
        let new = project(&files, build, 0, true);
        assert_eq!(
            old,
            [
                (200, b"FILE00000064.xxx".to_vec()),
                (u32::MAX - 1, b"FILE0000000A.xxx".to_vec()),
                (u32::MAX - 2, b"FILE00000028.xxx".to_vec())
            ]
        );
        assert_eq!(
            new,
            [
                (200, b"FILE00000064.xxx".to_vec()),
                (u32::MAX - 1, b"FILE00000028.xxx".to_vec()),
                (u32::MAX - 2, b"FILE0000000A.xxx".to_vec())
            ]
        );
    }
}
#[test]
fn native_gnu15_v2_n07_overlapping_sets_repeat_in_serialized_native_order() {
    let files = [
        (100, None, root(&[101], &[10, 11, 12], &[(0, 2), (1, 2)])),
        (101, None, group(false, &[2, 0, 1, 2, 0])),
        (10, None, m2(true)),
        (11, None, m2(true)),
        (12, None, m2(true)),
    ];
    let output = project(&files, 70374, 1, true);
    assert_eq!(
        output,
        [
            (200, b"FILE00000064.xxx".to_vec()),
            (u32::MAX - 1, b"FILE0000000B.xxx".to_vec()),
            (u32::MAX - 2, b"FILE0000000A.xxx".to_vec()),
            (u32::MAX - 3, b"FILE0000000B.xxx".to_vec()),
            (u32::MAX - 4, b"FILE0000000C.xxx".to_vec())
        ]
    );
}
