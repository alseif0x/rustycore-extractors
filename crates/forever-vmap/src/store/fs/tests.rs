// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: exclusively synthetic private filesystem fixtures.
use super::*;
use crate::store::{self, memory};
use std::os::unix::fs::{DirBuilderExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let base = std::env::temp_dir();
        assert!(!base.ancestors().any(|p| p.join(".git").exists()));
        let path = base.join(format!(
            "rce-vmap-fs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn file(&self, path: &str, data: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, data).unwrap();
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
fn read(
    source: &mut RootedSource,
    artifact: Artifact<'_>,
    cap: usize,
) -> Result<Option<Vec<u8>>, SourceError> {
    let limits = store::Limits {
        max_file_bytes: cap,
        ..store::Limits::default()
    };
    let mut memory = memory::Memory::new(0, 128 * 1024 * 1024);
    let mut work = memory::Work::new(limits);
    match store::source::read(source, artifact, limits, &mut memory, &mut work) {
        Ok(v) => Ok(v),
        Err(store::Error::Source(e)) => Err(e),
        Err(_) => Err(SourceError::Limit),
    }
}
fn model<'a>(name: &'a [u8]) -> Artifact<'a> {
    Artifact::Model { name }
}

#[test]
fn model_complete_read_and_repeated_sha_snapshot_cross_16k_boundary() {
    let dir = Directory::new();
    let data: Vec<_> = (0..33001).map(|i| (i % 251) as u8).collect();
    dir.file("Model.xxx.vmo", &data);
    let mut source = dir.open();
    for _ in 0..3 {
        assert_eq!(
            read(&mut source, model(b"Model.xxx"), data.len())
                .unwrap()
                .unwrap(),
            data
        );
    }
    source.verify_unchanged().unwrap();
    assert_eq!(source.stats().entries, 1);
    assert_eq!(source.stats().read_bytes, (data.len() * 3) as u64);
    assert!(!source.failed());
}
#[test]
fn exact_nested_numeric_paths_preserve_maximum_map_and_coordinate_domain() {
    let dir = Directory::new();
    dir.file("4294967295/4294967295.vmtree", b"tree");
    dir.file("4294967295/4294967295_32767_255.vmtile", b"tile");
    dir.file("4294967295/4294967295_32767_255.vmtileidx", b"index");
    let mut source = dir.open();
    for (artifact, bytes) in [
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
        assert_eq!(read(&mut source, artifact, 64).unwrap().unwrap(), bytes);
    }
    assert_eq!(source.stats().entries, 4);
    assert!(
        source
            .entries
            .windows(2)
            .all(|v| v[0].path.bytes() < v[1].path.bytes())
    );
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
fn unsafe_model_components_are_rejected_without_normalization() {
    let dir = Directory::new();
    dir.file("Case.xxx.vmo", b"a");
    let mut source = dir.open();
    for name in [
        b"".as_slice(),
        b".",
        b"..",
        b"a/b",
        b"a\\b",
        b"a\0b",
        b"\xff",
    ] {
        assert_eq!(read(&mut source, model(name), 64), Err(SourceError::Io));
    }
    assert_eq!(
        read(&mut source, model(&[b'x'; 501]), 64),
        Err(SourceError::Io)
    );
    assert!(read(&mut source, model(b"case.xxx"), 64).unwrap().is_none());
    assert_eq!(
        read(&mut source, model(b"Case.xxx"), 64).unwrap().unwrap(),
        b"a"
    );
}
#[test]
fn root_and_nested_symlinks_and_nonregular_files_reject() {
    let dir = Directory::new();
    dir.file("target/model.vmo", b"a");
    symlink(dir.0.join("target"), dir.0.join("alias")).unwrap();
    symlink(dir.0.join("target/model.vmo"), dir.0.join("model.vmo")).unwrap();
    symlink(dir.0.join("target"), dir.0.join("0001")).unwrap();
    fs::create_dir(dir.0.join("directory.vmo")).unwrap();
    assert!(matches!(
        RootedSource::new(&dir.0.join("alias"), FsLimits::default()),
        Err(SourceError::Io)
    ));
    assert!(matches!(
        RootedSource::new(&dir.0.join("alias/"), FsLimits::default()),
        Err(SourceError::Io)
    ));
    fs::create_dir(dir.0.join("target/child")).unwrap();
    assert!(matches!(
        RootedSource::new(&dir.0.join("alias/child"), FsLimits::default()),
        Err(SourceError::Io)
    ));
    let mut source = dir.open();
    assert_eq!(read(&mut source, model(b"model"), 64), Err(SourceError::Io));
    assert_eq!(
        read(&mut source, model(b"directory"), 64),
        Err(SourceError::Io)
    );
    assert_eq!(
        read(&mut source, Artifact::Tree { map_id: 1 }, 64),
        Err(SourceError::Io)
    );
}
#[test]
fn missing_file_and_missing_map_directory_are_recorded() {
    let dir = Directory::new();
    let mut source = dir.open();
    assert!(read(&mut source, model(b"absent"), 64).unwrap().is_none());
    assert!(
        read(
            &mut source,
            Artifact::Tile {
                map_id: 9,
                x: 3,
                y: 4
            },
            64
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(source.stats().entries, 3);
    source.verify_unchanged().unwrap();
    dir.file("0009/0009_03_04.vmtile", b"new");
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    assert!(source.failed());
}
#[test]
fn absent_child_becoming_present_invalidates_recorded_map_directory() {
    let dir = Directory::new();
    fs::create_dir(dir.0.join("0001")).unwrap();
    let mut source = dir.open();
    let tile = Artifact::Tile {
        map_id: 1,
        x: 0,
        y: 0,
    };
    assert!(read(&mut source, tile, 64).unwrap().is_none());
    dir.file("0001/0001_00_00.vmtile", b"new");
    assert_eq!(read(&mut source, tile, 64), Err(SourceError::InputChanged));
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
}
#[test]
fn same_size_inplace_change_is_terminal_even_after_bytes_restored() {
    let dir = Directory::new();
    dir.file("model.vmo", b"original");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 64).unwrap();
    // Exercise metadata verification without relying on clock granularity.
    // Post-read verification does not rehash contents or freeze the filesystem.
    let changed = dir.0.join("model.vmo");
    let captured = fs::metadata(&changed).unwrap().modified().unwrap();
    fs::write(&changed, b"modified").unwrap();
    fs::File::open(&changed)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(captured + std::time::Duration::from_secs(1)))
        .unwrap();
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    fs::write(dir.0.join("model.vmo"), b"original").unwrap();
    assert_eq!(
        read(&mut source, model(b"model"), 64),
        Err(SourceError::InputChanged)
    );
}
#[test]
fn root_rename_cannot_be_hidden_by_retained_descriptor() {
    let dir = Directory::new();
    dir.file("model.vmo", b"data");
    let mut source = dir.open();
    let moved = dir.0.with_extension("moved");
    fs::rename(&dir.0, &moved).unwrap();
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    fs::rename(moved, &dir.0).unwrap();
}
#[test]
fn map_directory_replacement_is_not_followed_for_cached_children() {
    let dir = Directory::new();
    dir.file("0001/0001.vmtree", b"original");
    let mut source = dir.open();
    read(&mut source, Artifact::Tree { map_id: 1 }, 64).unwrap();
    fs::rename(dir.0.join("0001"), dir.0.join("old")).unwrap();
    dir.file("0001/0001.vmtree", b"original");
    assert_eq!(
        read(&mut source, Artifact::Tree { map_id: 1 }, 64),
        Err(SourceError::InputChanged)
    );
}
#[test]
fn reread_digest_is_independent_of_metadata_only_verification() {
    let dir = Directory::new();
    dir.file("model.vmo", b"aaaa");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 64).unwrap();
    fs::write(dir.0.join("model.vmo"), b"bbbb").unwrap();
    // Simulate an unchanged metadata observation without changing production
    // behavior: retain the original digest but supply current metadata. This
    // explicitly demonstrates verify_unchanged's metadata-only scope.
    let State::File(_, digest) = source.entries[0].state else {
        panic!("file snapshot");
    };
    source.entries[0].state = State::File(
        Identity::of(&fs::metadata(dir.0.join("model.vmo")).unwrap()),
        digest,
    );
    source.verify_unchanged().unwrap();
    assert_eq!(
        read(&mut source, model(b"model"), 64),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
}
#[test]
fn empty_regular_file_is_present_and_size_limit_rejects_before_transfer() {
    let dir = Directory::new();
    dir.file("empty.vmo", b"");
    dir.file("large.vmo", &[1; 100]);
    let mut source = dir.open();
    assert_eq!(
        read(&mut source, model(b"empty"), 0).unwrap().unwrap(),
        Vec::<u8>::new()
    );
    assert_eq!(
        read(&mut source, model(b"large"), 99),
        Err(SourceError::Limit)
    );
    assert_eq!(source.stats().read_bytes, 0);
    assert_eq!(source.stats().entries, 1);
    assert_eq!(
        read(&mut source, model(b"large"), 100).unwrap().unwrap(),
        [1; 100]
    );
}
#[test]
fn metadata_snapshot_entry_and_root_limits_are_reducible() {
    let dir = Directory::new();
    for limits in [
        FsLimits {
            max_root_bytes: 0,
            ..FsLimits::default()
        },
        FsLimits {
            max_metadata_bytes: 0,
            ..FsLimits::default()
        },
        FsLimits {
            max_entries: MAX_ENTRIES + 1,
            ..FsLimits::default()
        },
        FsLimits {
            max_verify_steps: MAX_VERIFY_STEPS + 1,
            ..FsLimits::default()
        },
    ] {
        assert!(matches!(
            RootedSource::new(&dir.0, limits),
            Err(SourceError::Limit)
        ));
    }
    let mut source = RootedSource::new(
        &dir.0,
        FsLimits {
            max_entries: 0,
            ..FsLimits::default()
        },
    )
    .unwrap();
    assert_eq!(
        read(&mut source, model(b"absent"), 1),
        Err(SourceError::Limit)
    );
    assert_eq!(source.stats().entries, 0);
}
#[test]
fn verification_work_is_checked_before_metadata_steps() {
    let dir = Directory::new();
    dir.file("model.vmo", b"a");
    let mut source = dir.open();
    read(&mut source, model(b"model"), 10).unwrap();
    source.limits.max_verify_steps = 4;
    assert_eq!(source.verify_unchanged(), Err(SourceError::Limit));
    assert!(!source.failed());
    source.limits.max_verify_steps = 5;
    source.verify_unchanged().unwrap();
    source.limits.max_verify_steps = 0;
    let bytes = source.stats().read_bytes;
    assert_eq!(
        read(&mut source, model(b"model"), 10),
        Err(SourceError::Limit)
    );
    assert_eq!(source.stats().read_bytes, bytes);
}
#[test]
fn ninth_snapshot_growth_charges_old_new_overlap_without_losing_prior_rows() {
    let dir = Directory::new();
    for i in 0..9 {
        dir.file(&format!("m{i}.vmo"), b"a");
    }
    let mut baseline = dir.open();
    for i in 0..9 {
        read(&mut baseline, model(format!("m{i}").as_bytes()), 1).unwrap();
    }
    assert_eq!(baseline.entries.capacity(), 16);
    let peak = baseline.stats().peak_bytes;
    assert_eq!(
        peak,
        baseline.root_path.capacity() + (8 + 16) * size_of::<Entry>()
    );
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
        let result = read(&mut source, model(b"m8"), 1);
        assert_eq!(result.is_ok(), cap == peak);
        assert_eq!(source.stats().entries, if cap == peak { 9 } else { 8 });
        assert!(source.stats().resident_bytes <= cap && source.stats().peak_bytes <= cap);
        source.verify_unchanged().unwrap();
    }
}

#[test]
fn observed_tail_change_survives_exhausted_guard_budget_and_poison_is_sticky() {
    let dir = Directory::new();
    let mut source = dir.open();
    assert_eq!(source.observe_tail(0), Ok(()));
    assert!(!source.failed());
    assert_eq!(source.observe_tail(1), Err(SourceError::InputChanged));
    assert_eq!(source.stats().read_bytes, 1);
    let mut work = Work { used: 4, cap: 4 };
    let later_guard = work.step();
    assert_eq!(later_guard, Err(SourceError::Limit));
    assert_eq!(
        source.finish_operation(later_guard, work),
        Err(SourceError::InputChanged)
    );
    assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    assert_eq!(
        read(&mut source, model(b"absent"), 0),
        Err(SourceError::InputChanged)
    );
}

#[test]
fn held_root_change_is_compared_before_next_guard_reservation() {
    let dir = Directory::new();
    let mut source = dir.open();
    // Deterministic identity witness: no filesystem race or sleep is required.
    source.identity.ino ^= 1;
    let mut work = Work { used: 0, cap: 1 };
    let result = source.guard_root(&mut work);
    assert_eq!(result, Err(SourceError::InputChanged));
    assert_eq!(work.used, 1);
    assert_eq!(
        source.finish_operation(result, work),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
}

#[test]
fn held_directory_change_is_compared_before_next_guard_reservation() {
    let dir = Directory::new();
    dir.file("0001/0001.vmtree", b"tree");
    let mut source = dir.open();
    let file = open_dir(&dir.0.join("0001")).unwrap();
    let mut identity = Identity::of(&file.metadata().unwrap());
    identity.ino ^= 1;
    let key = request(Artifact::Tree { map_id: 1 })
        .unwrap()
        .directory
        .unwrap();
    let mut work = Work { used: 0, cap: 1 };
    let result = source.guard_after(Some(&file), Some((key, identity)), &mut work);
    assert_eq!(result, Err(SourceError::InputChanged));
    assert_eq!(work.used, 1);
    assert_eq!(
        source.finish_operation(result, work),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
}
