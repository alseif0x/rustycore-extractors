// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: isolated synthetic Windows VMap read-source fixtures.
// Modified 2026-10-12: isolate the renamed source beneath a private parent.
use super::*;
use crate::store::{self, memory};
use std::{
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let base = std::env::temp_dir();
        assert!(!base.ancestors().any(|p| p.join(".git").exists()));
        let path = base.join(format!(
            "rce-vmap-win-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fn open(&self) -> RootedSource {
        RootedSource::new(&self.0, FsLimits::default()).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn model(name: &[u8]) -> Artifact<'_> {
    Artifact::Model { name }
}
fn read(
    source: &mut RootedSource,
    artifact: Artifact<'_>,
    cap: usize,
) -> Result<Option<Vec<u8>>, SourceError> {
    let limits = store::Limits {
        max_file_bytes: cap,
        ..store::Limits::default()
    };
    let mut memory = memory::Memory::new(0, 512 * 1024 * 1024);
    let mut work = memory::Work::new(limits);
    match store::source::read(source, artifact, limits, &mut memory, &mut work) {
        Ok(v) => Ok(v),
        Err(store::Error::Source(e)) => Err(e),
        Err(_) => Err(SourceError::Limit),
    }
}
#[test]
fn complete_and_empty_reads_repeated_digest_and_no_retained_leases() {
    // Give the renamed source its own private parent. Other concurrent tests
    // can hold ancestor leases on the common temporary directory.
    let container = Directory::new();
    let dir = Directory(container.0.join("source"));
    fs::create_dir(&dir.0).unwrap();
    let bytes: Vec<_> = (0..33001).map(|n| (n % 251) as u8).collect();
    dir.file("Model.xxx.vmo", &bytes);
    dir.file("empty.vmo", b"");
    let mut source = dir.open();
    for _ in 0..3 {
        assert_eq!(
            read(&mut source, model(b"Model.xxx"), bytes.len())
                .unwrap()
                .unwrap(),
            bytes
        );
    }
    assert_eq!(
        read(&mut source, model(b"empty"), 0).unwrap(),
        Some(Vec::new())
    );
    source.verify_unchanged().unwrap();
    assert_eq!(source.stats().entries, 2);
    assert_eq!(source.stats().read_bytes, (bytes.len() * 3) as u64);
    let writer = fs::OpenOptions::new()
        .write(true)
        .open(dir.0.join("Model.xxx.vmo"))
        .unwrap();
    drop(writer);
    let moved = dir.0.with_extension("moved");
    fs::rename(&dir.0, &moved).unwrap();
    fs::rename(&moved, &dir.0).unwrap();
}
#[test]
fn exact_numeric_paths_include_full_map_and_coordinate_domain() {
    let dir = Directory::new();
    dir.file("4294967295/4294967295.vmtree", b"tree");
    dir.file("4294967295/4294967295_32767_255.vmtile", b"tile");
    dir.file("4294967295/4294967295_32767_255.vmtileidx", b"index");
    let mut source = dir.open();
    for (artifact, expected) in [
        (Artifact::Tree { map_id: u32::MAX }, b"tree".as_slice()),
        (
            Artifact::Tile {
                map_id: u32::MAX,
                x: 32767,
                y: 255,
            },
            b"tile".as_slice(),
        ),
        (
            Artifact::Index {
                map_id: u32::MAX,
                x: 32767,
                y: 255,
            },
            b"index".as_slice(),
        ),
    ] {
        assert_eq!(read(&mut source, artifact, 64).unwrap().unwrap(), expected);
    }
    assert_eq!(source.stats().entries, 4);
    source.verify_unchanged().unwrap();
    assert_eq!(
        read(
            &mut source,
            Artifact::Tile {
                map_id: 1,
                x: 32768,
                y: 0
            },
            64
        ),
        Err(SourceError::Io)
    );
}
#[test]
fn case_alias_never_reads_a_different_exact_artifact_name() {
    let dir = Directory::new();
    dir.file("Case.xxx.vmo", b"exact");
    let mut source = dir.open();
    assert_eq!(
        read(&mut source, model(b"case.xxx"), 64),
        Err(SourceError::Io)
    );
    assert_eq!(source.stats().read_bytes, 0);
    assert!(!source.failed());
    assert_eq!(
        read(&mut source, model(b"Case.xxx"), 64).unwrap().unwrap(),
        b"exact"
    );
}
#[test]
fn unsupported_windows_model_names_reject_before_io() {
    let dir = Directory::new();
    let mut source = dir.open();
    for name in [
        b"".as_slice(),
        b".",
        b"..",
        b"a/b",
        b"a\\b",
        b"a\0b",
        b"\xff",
        b"CON",
        b"x:y",
        b"a?",
        b"a*",
        b"a|",
        b"a\"",
        b"a\x01",
    ] {
        assert_eq!(read(&mut source, model(name), 64), Err(SourceError::Io));
    }
    assert_eq!(
        read(&mut source, model(&[b'x'; 252]), 1),
        Err(SourceError::Io)
    );
    assert_eq!(source.stats().read_bytes, 0);
}
#[test]
fn unsupported_roots_are_rejected_without_canonicalization() {
    for root in [
        "relative",
        "C:relative",
        "C:\\",
        "C:/normal",
        "\\\\?\\C:\\root",
        "\\\\server\\share\\root",
        "C:\\a\\..\\b",
        "C:\\a\\.\\b",
        "C:\\a\\",
        "C:\\a ",
        "C:\\a.",
    ] {
        assert!(matches!(
            RootedSource::new(Path::new(root), FsLimits::default()),
            Err(SourceError::Io)
        ));
    }
}
#[test]
fn genuine_missing_files_and_directories_are_recorded_and_verified() {
    let dir = Directory::new();
    dir.file("0002/other", b"data");
    let mut source = dir.open();
    assert_eq!(read(&mut source, model(b"missing"), 64).unwrap(), None);
    assert_eq!(
        read(&mut source, Artifact::Tree { map_id: 1 }, 64).unwrap(),
        None
    );
    assert_eq!(
        read(&mut source, Artifact::Tree { map_id: 2 }, 64).unwrap(),
        None
    );
    assert_eq!(source.stats().entries, 5);
    source.verify_unchanged().unwrap();
}
#[test]
fn missing_to_present_poison_is_permanent() {
    let dir = Directory::new();
    let mut source = dir.open();
    assert_eq!(read(&mut source, model(b"missing"), 1).unwrap(), None);
    dir.file("missing.vmo", b"x");
    assert_eq!(
        read(&mut source, model(b"missing"), 1),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
    fs::remove_file(dir.0.join("missing.vmo")).unwrap();
    source.limits.max_verify_steps = 0;
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
}
#[test]
fn unrelated_root_entry_change_invalidates_full_directory_snapshot() {
    let dir = Directory::new();
    dir.file("model.vmo", b"x");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 1).unwrap();
    dir.file("other", b"y");
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    assert!(source.failed());
}
#[test]
fn unrelated_nested_entry_change_invalidates_map_snapshot() {
    let dir = Directory::new();
    dir.file("0001/0001.vmtree", b"tree");
    let mut source = dir.open();
    read(&mut source, Artifact::Tree { map_id: 1 }, 4).unwrap();
    dir.file("0001/other", b"y");
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
}
#[test]
fn same_length_content_change_invalidates_file_snapshot() {
    let dir = Directory::new();
    dir.file("model.vmo", b"one");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 3).unwrap();
    dir.file("model.vmo", b"two");
    assert_eq!(
        read(&mut source, model(b"model"), 3),
        Err(SourceError::InputChanged)
    );
}
#[test]
fn repeat_digest_rejects_even_when_metadata_snapshot_is_supplied_current() {
    let dir = Directory::new();
    dir.file("model.vmo", b"one");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 3).unwrap();
    let State::File(_, digest) = source.entries[0].state else {
        panic!("synthetic state");
    };
    dir.file("model.vmo", b"two");
    let lease = ReadFile::open(
        &dir.0.join("model.vmo"),
        PlatformLimits::default(),
        LinkPolicy::Allow,
    )
    .unwrap();
    let current = lease.identity();
    drop(lease);
    source.entries[0].state = State::File(current, digest);
    source.verify_unchanged().unwrap();
    assert_eq!(
        read(&mut source, model(b"model"), 3),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
}
#[test]
fn missing_directory_creation_and_file_deletion_poison() {
    let dir = Directory::new();
    let mut source = dir.open();
    read(&mut source, Artifact::Tree { map_id: 7 }, 16).unwrap();
    dir.file("0007/0007.vmtree", b"t");
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    let mut source = dir.open();
    read(&mut source, Artifact::Tree { map_id: 7 }, 16).unwrap();
    fs::remove_file(dir.0.join("0007/0007.vmtree")).unwrap();
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
}
#[test]
fn non_regular_files_and_existing_writer_are_not_missing() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = Directory::new();
    fs::create_dir(dir.0.join("directory.vmo")).unwrap();
    dir.file("file.vmo", b"x");
    let mut source = dir.open();
    assert_eq!(
        read(&mut source, model(b"directory"), 1),
        Err(SourceError::Io)
    );
    let writer = fs::OpenOptions::new()
        .write(true)
        .share_mode(7)
        .open(dir.0.join("file.vmo"))
        .unwrap();
    assert_eq!(read(&mut source, model(b"file"), 1), Err(SourceError::Io));
    assert!(!source.failed());
    drop(writer);
    assert_eq!(read(&mut source, model(b"file"), 1).unwrap().unwrap(), b"x");
}
#[test]
fn downward_file_work_enumeration_and_memory_limits_preserve_state() {
    let dir = Directory::new();
    dir.file("a.vmo", b"abc");
    dir.file("b.vmo", b"abc");
    let mut source = dir.open();
    assert_eq!(read(&mut source, model(b"a"), 2), Err(SourceError::Limit));
    assert!(!source.failed());
    assert_eq!(source.stats().entries, 0);
    source.limits.max_entries = 1;
    assert_eq!(read(&mut source, model(b"a"), 3), Err(SourceError::Limit));
    assert_eq!(source.stats().read_bytes, 0);
    source.limits.max_entries = MAX_ENTRIES;
    source.limits.max_verify_steps = 0;
    assert_eq!(read(&mut source, model(b"a"), 3), Err(SourceError::Limit));
    assert!(source.stats().verify_steps <= MAX_VERIFY_STEPS as u64);
    source.limits.max_verify_steps = MAX_VERIFY_STEPS;
    source.limits.max_metadata_bytes = source.stats().resident_bytes + LEASE_BYTES + 2;
    assert_eq!(read(&mut source, model(b"a"), 3), Err(SourceError::Limit));
    assert!(!source.failed());
}
#[test]
fn entry_growth_charges_old_new_overlap_and_lease_reservation() {
    let dir = Directory::new();
    for i in 0..9 {
        dir.file(&format!("m{i}.vmo"), b"x");
    }
    let mut baseline = dir.open();
    for i in 0..9 {
        read(&mut baseline, model(format!("m{i}").as_bytes()), 1).unwrap();
    }
    let peak = baseline.stats().peak_bytes;
    assert!(peak >= baseline.root_path.capacity() + LEASE_BYTES + (8 + 16) * size_of::<Entry>());
    for cap in [peak - 1, peak] {
        let mut source = RootedSource::new(
            &dir.0,
            FsLimits {
                max_metadata_bytes: cap,
                ..FsLimits::default()
            },
        )
        .unwrap();
        for i in 0..8 {
            read(&mut source, model(format!("m{i}").as_bytes()), 1).unwrap();
        }
        assert_eq!(read(&mut source, model(b"m8"), 1).is_ok(), cap == peak);
        assert!(source.stats().peak_bytes <= cap);
    }
}
#[test]
fn limits_are_downward_only_and_reserve_platform_storage_before_open() {
    let dir = Directory::new();
    for limits in [
        FsLimits {
            max_metadata_bytes: MAX_METADATA_BYTES + 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_entries: MAX_ENTRIES + 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_root_bytes: MAX_ROOT_BYTES + 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_verify_steps: MAX_VERIFY_STEPS + 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_metadata_bytes: LEASE_BYTES - 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_root_bytes: 0,
            ..FsLimits::default()
        },
    ] {
        assert!(matches!(
            RootedSource::new(&dir.0, limits),
            Err(SourceError::Limit)
        ));
    }
}
#[test]
fn observed_change_wins_over_later_operation_limits() {
    let dir = Directory::new();
    let mut source = dir.open();
    source.identity.file_index ^= 1;
    let unit = (source
        .root_path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .filter(|&&b| b == b'\\')
        .count()
        + 2)
        * 32;
    let mut work = Work { used: 0, cap: unit };
    let result = source.guard_root(&mut work);
    assert_eq!(result, Err(SourceError::InputChanged));
    assert_eq!(
        source.finish_operation(result, work),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
}
#[test]
fn full_128mib_file_uses_bounded_chunks_and_real_eof() {
    let dir = Directory::new();
    let path = dir.0.join("boundary.vmo");
    let mut file = fs::File::create(&path).unwrap();
    let block = [0x61; 65536];
    for _ in 0..2048 {
        file.write_all(&block).unwrap();
    }
    drop(file);
    let mut source = dir.open();
    let result = read(
        &mut source,
        model(b"boundary"),
        super::super::MAX_FILE_BYTES,
    )
    .unwrap()
    .unwrap();
    assert_eq!(result.len(), super::super::MAX_FILE_BYTES);
    assert!(result.iter().all(|&b| b == 0x61));
    assert_eq!(source.stats().read_bytes, result.len() as u64);
    assert!(source.stats().peak_bytes <= source.limits.max_metadata_bytes);
    source.verify_unchanged().unwrap();
}

// Original project-authored complete synthetic model/placement helper facts.
fn word(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn words(out: &mut Vec<u8>, v: impl IntoIterator<Item = u32>) {
    for v in v {
        word(out, v)
    }
}
fn row(id: u32, flags: u8, name: &[u8], bounds: [f32; 6]) -> Vec<u8> {
    let mut out = vec![flags, 7];
    word(&mut out, id);
    words(&mut out, [0u32; 6]);
    word(&mut out, 1f32.to_bits());
    words(&mut out, bounds.map(f32::to_bits));
    word(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    out
}
fn bounds() -> [f32; 6] {
    [0., 0., -1., 1., 1., 1.]
}
fn m2() -> Vec<u8> {
    let mut out = crate::RAW_MAGIC.to_vec();
    words(&mut out, [3, 1, 0, 1, 0, 0]);
    words(&mut out, [-1f32, -1., -1., 2., 2., 2.].map(f32::to_bits));
    word(&mut out, 0);
    out.extend_from_slice(b"GRP ");
    words(&mut out, [8, 1, 3]);
    out.extend_from_slice(b"INDX");
    words(&mut out, [10, 3, 0, 1, 2]);
    out.extend_from_slice(b"VERT");
    words(&mut out, [40, 3]);
    words(
        &mut out,
        [0f32, 0., 0., 1., 0., 0., 0., 1., 0.].map(f32::to_bits),
    );
    let raw = crate::RawM2::parse(&out, crate::Limits::default()).unwrap();
    crate::vmo::convert_m2(&raw, crate::vmo::Limits::default())
        .unwrap()
        .into_bytes()
}

#[test]
fn complete_static_store_load_query_without_source_io_then_poison() {
    let dir = Directory::new();
    let bytes = row(9, 1, b"model", bounds());
    let input =
        crate::placement::DirBin::parse(&bytes, crate::placement::Limits::default()).unwrap();
    let rows: Vec<_> = input.records().collect();
    let map = crate::assembly::build(1, &rows, crate::assembly::Limits::default()).unwrap();
    dir.file("0001/0001.vmtree", map.tree_bytes());
    for tile in map.tiles() {
        dir.file(
            &format!("0001/0001_{:02}_{:02}.vmtileidx", tile.x(), tile.y()),
            tile.index_bytes(),
        );
        if let Some(payload) = tile.payload() {
            dir.file(
                &format!("0001/0001_{:02}_{:02}.vmtile", tile.x(), tile.y()),
                payload,
            );
        }
    }
    dir.file("model.vmo", &m2());
    let mut store = store::StaticStore::new(
        dir.open(),
        &[store::MapParent {
            map_id: 1,
            parent: None,
            cosmetic_parent: None,
        }],
        store::Options::default(),
        store::Limits::default(),
    )
    .unwrap();
    store.load_tile(1, 0, 0).unwrap();
    assert_eq!(store.stats().models, 1);
    let before = store.source.stats();
    dir.file("model.vmo", b"synthetic changed source");
    let mut budget = crate::query::Budget::new(crate::query::Limits::default()).unwrap();
    let _ = store
        .line_of_sight_internal(1, [0.2, 0.2, 1.], [0.2, 0.2, -1.], false, &mut budget)
        .unwrap();
    assert_eq!(store.source.stats(), before);
    assert_eq!(
        store.verify_source(),
        Err(store::Error::Source(SourceError::InputChanged))
    );
    assert!(store.is_poisoned());
    assert_eq!(store.load_tile(1, 0, 0), Err(store::Error::Poisoned));
}

#[test]
fn file_larger_than_128mib_rejects_before_content_read() {
    let dir = Directory::new();
    let file = fs::File::create(dir.0.join("over.vmo")).unwrap();
    file.set_len(super::super::MAX_FILE_BYTES as u64 + 1)
        .unwrap();
    drop(file);
    let mut source = dir.open();
    assert_eq!(
        read(&mut source, model(b"over"), super::super::MAX_FILE_BYTES),
        Err(SourceError::Limit)
    );
    assert_eq!(source.stats().read_bytes, 0);
    assert!(!source.failed());
}

#[test]
fn observed_open_changes_and_recorded_artifact_failures_win_over_guard_limits() {
    let dir = Directory::new();
    dir.file("model.vmo", b"x");
    let key = request(model(b"model")).unwrap().full;
    for (recorded, error) in [
        (false, PlatformError::Changed),
        (false, PlatformError::Read),
        (true, PlatformError::Open),
        (true, PlatformError::FileType),
        (true, PlatformError::Reparse),
    ] {
        let mut source = dir.open();
        if recorded {
            read(&mut source, model(b"model"), 1).unwrap();
        }
        let mut work = Work { used: 0, cap: 0 };
        let result = source.failed_open(&key, error, None, &mut work);
        assert!(matches!(result, Err(SourceError::InputChanged)));
        assert_eq!(work.used, 0);
        assert!(matches!(
            source.finish_operation(result, work),
            Err(SourceError::InputChanged)
        ));
        assert!(source.failed());
        assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    }
    for (recorded, error) in [
        (false, PlatformError::Open),
        (true, PlatformError::Limit),
        (true, PlatformError::Allocation),
    ] {
        let mut source = dir.open();
        if recorded {
            read(&mut source, model(b"model"), 1).unwrap();
        }
        let mut work = Work { used: 0, cap: 0 };
        let result = source.failed_open(&key, error, None, &mut work);
        assert!(matches!(
            source.finish_operation(result, work),
            Err(SourceError::Limit)
        ));
        assert!(!source.failed());
    }
}
