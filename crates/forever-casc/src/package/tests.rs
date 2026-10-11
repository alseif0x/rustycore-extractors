// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic common package lifecycle and rejection tests.
use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-package-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
    fn run(&self) -> Run {
        Run::create(&self.output(), ctx(), &[], Limits::default()).unwrap()
    }
    fn complete(&self) -> Summary {
        let mut run = self.run();
        run.put(Component::Navigation, "mmaps/0001.mmap", b"authored bytes")
            .unwrap();
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap()
    }
    fn verify(&self) -> Result<Summary, Error> {
        verify(&self.output(), Some(ctx()), Limits::default())
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn ctx() -> Context {
    Context {
        build: 70205,
        locale: Locale::EsEs,
        components: Component::Navigation.bit(),
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
    }
}
#[test]
fn inspection_is_rooted_readonly_and_holds_shared_lock() {
    let t = Temp::new();
    t.complete();
    let mut seen = 0;
    let summary = verify_with(
        &t.output(),
        Some(ctx()),
        Limits::default(),
        |artifact, file| {
            assert_eq!(artifact.path, "mmaps/0001.mmap");
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            assert_eq!(bytes, b"authored bytes");
            assert!(Run::resume(&t.output(), ctx(), &[], Limits::default()).is_err());
            seen += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(seen, 1);
    assert_eq!(summary, t.verify().unwrap());
}
#[test]
fn inspection_runs_only_after_complete_integrity_preflight_and_propagates_failure() {
    let t = Temp::new();
    t.complete();
    assert_eq!(
        verify_with(&t.output(), None, Limits::default(), |_, _| Err(
            Error::Schema
        ))
        .unwrap_err(),
        Error::Schema
    );
    fs::write(t.output().join("mmaps/0001.mmap"), b"corrupted bytes").unwrap();
    let mut seen = 0;
    assert!(
        verify_with(&t.output(), None, Limits::default(), |_, _| {
            seen += 1;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(seen, 0);
}
#[test]
fn inspection_rechecks_namespace_and_receipts_after_callback() {
    let t = Temp::new();
    t.complete();
    assert!(
        verify_with(&t.output(), None, Limits::default(), |_, _| {
            fs::write(t.output().join("mmaps/extra.mmap"), b"unexpected").unwrap();
            Ok(())
        })
        .is_err()
    );
    fs::remove_file(t.output().join("mmaps/extra.mmap")).unwrap();
    assert!(
        verify_with(&t.output(), None, Limits::default(), |artifact, _| {
            let receipt = fs::read_dir(t.output().join(".state/files"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            assert_eq!(artifact.path, "mmaps/0001.mmap");
            fs::write(receipt, b"invalid receipt").unwrap();
            Ok(())
        })
        .is_err()
    );
}
#[test]
fn complete_roundtrip_and_same_writer_idempotence() {
    let t = Temp::new();
    let mut r = t.run();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/0001.mmap", b"abc")
            .unwrap(),
        Put::Written
    );
    assert_eq!(
        r.put(Component::Navigation, "mmaps/0001.mmap", b"abc")
            .unwrap(),
        Put::AlreadyPresent
    );
    r.finish_component(Component::Navigation).unwrap();
    let s = r.finish().unwrap();
    assert_eq!((s.files, s.bytes), (1, 3));
    assert_eq!(t.verify().unwrap(), s);
    assert!(!t.output().join(INCOMPLETE).exists());
}
#[test]
fn every_component_is_explicitly_sealed_and_empty_is_admitted() {
    let t = Temp::new();
    let context = Context {
        components: 63,
        ..ctx()
    };
    let mut r = Run::create(&t.output(), context, &[], Limits::default()).unwrap();
    for c in Component::ALL {
        r.finish_component(c).unwrap();
    }
    let s = r.finish().unwrap();
    assert_eq!((s.files, s.bytes), (0, 0));
    assert_eq!(
        verify(&t.output(), Some(context), Limits::default()).unwrap(),
        s
    );
}
#[test]
fn all_six_prefixes_and_space_punctuation_names_roundtrip() {
    let t = Temp::new();
    let context = Context {
        components: 63,
        ..ctx()
    };
    let mut r = Run::create(&t.output(), context, &[], Limits::default()).unwrap();
    for (c, p) in [
        (Component::Db2, "dbc/esES/X.db2"),
        (Component::GameTables, "gt/xp.txt"),
        (Component::Cameras, "cameras/1.m2"),
        (Component::Terrain, "maps/0001_00_00.map"),
        (Component::Collision, "vmaps/a model (test)'s.vmo"),
        (Component::Navigation, "mmaps/0001.mmap"),
    ] {
        r.put(c, p, p.as_bytes()).unwrap();
        r.finish_component(c).unwrap();
    }
    assert_eq!(r.finish().unwrap().files, 6);
    assert_eq!(
        verify(&t.output(), Some(context), Limits::default())
            .unwrap()
            .files,
        6
    );
}
#[test]
fn context_parser_is_canonical_and_rejects_duplicate_extra_or_invalid_fields() {
    let c = ctx();
    assert_eq!(Context::parse(&c.json()).unwrap(), c);
    for s in [
        format!(" {}", c.json()),
        c.json()
            .replace("\"build\":70205", "\"build\":70205,\"build\":70205"),
        c.json()
            .replace("\"build\":70205", "\"build\":70205,\"extra\":0"),
        c.json().replace("70205", "51943"),
        c.json().replace("esES", "frFR"),
        c.json().replace("\"components\":32", "\"components\":0"),
    ] {
        assert!(Context::parse(&s).is_err());
    }
    assert_eq!(
        Context {
            source_sha256: [0; 32],
            ..c
        }
        .validate(),
        Err(Error::Context)
    );
}
#[test]
fn expected_context_mismatch_rejects() {
    let t = Temp::new();
    t.complete();
    assert_eq!(
        verify(
            &t.output(),
            Some(Context {
                plan_sha256: [4; 32],
                ..ctx()
            }),
            Limits::default()
        )
        .unwrap_err(),
        Error::Context
    );
}
#[test]
fn unsafe_artifact_paths_reject() {
    for p in [
        "",
        "/mmaps/x",
        "mmaps/../x",
        "mmaps//x",
        "mmaps/.x",
        "mmaps/a.",
        "mmaps/a ",
        "mmaps/a\\b",
        "mmaps/a\"b",
        "mmaps/x\n",
        "mmaps/é",
        "mmaps/a:b",
        "mmaps/a?b",
    ] {
        assert_eq!(model::path(p), Err(Error::Path), "{p:?}");
    }
    assert_eq!(model::path(&"a".repeat(513)), Err(Error::Path));
}
#[test]
fn drop_keeps_incomplete_and_verifier_refuses() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/x", b"x").unwrap();
    drop(r);
    assert!(t.output().join(INCOMPLETE).is_file());
    assert_eq!(t.verify().unwrap_err(), Error::Incomplete);
    assert_eq!(
        Run::create(&t.output(), ctx(), &[], Limits::default()).err(),
        Some(Error::Exists)
    );
}
#[test]
fn missing_component_seal_prevents_completion() {
    let t = Temp::new();
    let r = t.run();
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    assert_eq!(t.verify().unwrap_err(), Error::Incomplete);
}
#[test]
fn active_writer_and_external_exclusive_lock_reject_verification() {
    let t = Temp::new();
    let r = t.run();
    assert_eq!(t.verify().unwrap_err(), Error::Busy);
    drop(r);
    let u = Temp::new();
    u.complete();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(u.output().join(".state/lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert_eq!(u.verify().unwrap_err(), Error::Busy);
    drop(lock);
    u.verify().unwrap();
}
#[test]
fn conflicting_repeated_put_poison_is_permanent() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/x", b"x").unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/x", b"y").unwrap_err(),
        Error::Inventory
    );
    assert_eq!(
        r.put(Component::Navigation, "mmaps/y", b"y").unwrap_err(),
        Error::Incomplete
    );
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    assert_eq!(fs::read(t.output().join("mmaps/x")).unwrap(), b"x");
}
#[test]
fn write_after_seal_rejects_and_poison() {
    let t = Temp::new();
    let mut r = t.run();
    r.finish_component(Component::Navigation).unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/x", b"x").unwrap_err(),
        Error::Sealed
    );
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
}
#[test]
fn unselected_component_and_wrong_prefix_reject_before_payload() {
    for (c, p, e) in [
        (Component::Terrain, "maps/x", Error::Component),
        (Component::Navigation, "maps/x", Error::Path),
        (Component::Navigation, "dbc/esES/x", Error::Path),
    ] {
        let t = Temp::new();
        let mut r = t.run();
        assert_eq!(r.put(c, p, b"x").unwrap_err(), e);
        assert!(!t.output().join(p).exists());
    }
}
#[test]
fn existing_output_and_protected_input_are_preserved() {
    let t = Temp::new();
    fs::create_dir(t.output()).unwrap();
    fs::write(t.output().join("keep"), b"keep").unwrap();
    assert_eq!(
        Run::create(&t.output(), ctx(), &[], Limits::default()).err(),
        Some(Error::Exists)
    );
    assert_eq!(fs::read(t.output().join("keep")).unwrap(), b"keep");
    let path = t.0.join("protected");
    assert_eq!(
        Run::create(&path, ctx(), &[&t.0], Limits::default()).err(),
        Some(Error::Path)
    );
    assert!(!path.exists());
}
#[test]
fn output_inside_git_and_parent_symlink_are_rejected() {
    let t = Temp::new();
    fs::create_dir(t.0.join(".git")).unwrap();
    assert_eq!(
        Run::create(&t.output(), ctx(), &[], Limits::default()).err(),
        Some(Error::Path)
    );
    fs::remove_dir(t.0.join(".git")).unwrap();
    let alias = t.0.join("alias");
    std::os::unix::fs::symlink(&t.0, &alias).unwrap();
    assert_eq!(
        Run::create(&alias.join("output"), ctx(), &[], Limits::default()).err(),
        Some(Error::Path)
    );
}
#[test]
fn tamper_payload_and_missing_payload_reject() {
    for missing in [false, true] {
        let t = Temp::new();
        t.complete();
        let p = t.output().join("mmaps/0001.mmap");
        if missing {
            fs::remove_file(p).unwrap();
        } else {
            fs::write(p, b"authored bytez").unwrap();
        }
        assert!(t.verify().is_err());
    }
}
#[test]
fn extra_payload_and_extra_receipt_reject() {
    for p in [
        "mmaps/extra",
        ".state/files/0000000000000000000000000000000000000000000000000000000000000000.json",
        "extra",
    ] {
        let t = Temp::new();
        t.complete();
        fs::write(t.output().join(p), b"x").unwrap();
        assert_eq!(t.verify().unwrap_err(), Error::Inventory);
    }
}
#[test]
fn extra_empty_directory_rejects() {
    let t = Temp::new();
    t.complete();
    fs::create_dir(t.output().join("mmaps/unused")).unwrap();
    assert_eq!(t.verify().unwrap_err(), Error::Inventory);
}
#[test]
fn symlink_hardlink_and_fifo_payload_reject() {
    for kind in 0..3 {
        let t = Temp::new();
        t.complete();
        let p = t.output().join("mmaps/0001.mmap");
        let moved = t.0.join("saved");
        fs::rename(&p, &moved).unwrap();
        match kind {
            0 => std::os::unix::fs::symlink(&moved, &p).unwrap(),
            1 => fs::hard_link(&moved, &p).unwrap(),
            _ => {
                assert!(
                    std::process::Command::new("mkfifo")
                        .arg(&p)
                        .status()
                        .unwrap()
                        .success()
                );
            }
        }
        assert!(t.verify().is_err());
    }
}
#[test]
fn changed_receipt_context_lock_and_seal_reject() {
    for p in [".state/context.json", ".state/lock", ".state/sealed/mmaps"] {
        let t = Temp::new();
        t.complete();
        fs::write(t.output().join(p), b"changed").unwrap();
        assert!(t.verify().is_err());
    }
    let t = Temp::new();
    t.complete();
    let receipt = fs::read_dir(t.output().join(".state/files"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(receipt, b"{}").unwrap();
    assert_eq!(t.verify().unwrap_err(), Error::Inventory);
}
#[test]
fn malformed_manifest_never_passes() {
    for mode in 0..7 {
        let t = Temp::new();
        t.complete();
        let p = t.output().join(MANIFEST);
        let original = fs::read_to_string(&p).unwrap();
        let s = match mode {
            0 => original.replace("\"manifest_version\":1", "\"manifest_version\":2"),
            1 => original.replace("\"status\":\"complete\"", "\"status\":\"failed\""),
            2 => original.replace("\"bytes\":14", "\"bytes\":12"),
            3 => original.replace("\"artifacts\":[", "\"extra\":1,\"artifacts\":["),
            4 => original.replace("}\n]", "},\n]"),
            5 => format!("{original}junk"),
            _ => original.trim_end().to_owned(),
        };
        assert_ne!(s, original, "mutation must change mode {mode}");
        fs::write(p, s).unwrap();
        assert!(t.verify().is_err(), "mode {mode}");
    }
}
#[test]
fn marker_reappearance_rejects_completed_manifest() {
    let t = Temp::new();
    t.complete();
    fs::write(t.output().join(INCOMPLETE), b"incomplete\n").unwrap();
    assert_eq!(t.verify().unwrap_err(), Error::Incomplete);
}
#[test]
fn file_total_count_directory_and_manifest_limits_reject() {
    for mode in 0..5 {
        let t = Temp::new();
        let mut limits = Limits::default();
        match mode {
            0 => limits.file_bytes = 0,
            1 => limits.total_bytes = 0,
            2 => limits.files = 0,
            3 => limits.directories = 3,
            _ => limits.manifest_bytes = 1,
        };
        let mut r = Run::create(&t.output(), ctx(), &[], limits).unwrap();
        if mode < 4 {
            assert_eq!(
                r.put(Component::Navigation, "mmaps/x", b"x").unwrap_err(),
                Error::Limit
            );
            assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
        } else {
            r.finish_component(Component::Navigation).unwrap();
            assert_eq!(r.finish().unwrap_err(), Error::Limit);
        }
        assert_eq!(t.verify().unwrap_err(), Error::Incomplete);
    }
}
#[test]
fn invalid_context_limits_and_relative_output_do_not_create_stage() {
    let t = Temp::new();
    assert_eq!(
        Run::create(
            &t.output(),
            Context {
                build: 51943,
                ..ctx()
            },
            &[],
            Limits::default()
        )
        .err(),
        Some(Error::Context)
    );
    assert!(!t.output().exists());
    assert_eq!(
        Run::create(
            &t.output(),
            ctx(),
            &[],
            Limits {
                files: usize::MAX,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Run::create(Path::new("relative"), ctx(), &[], Limits::default()).err(),
        Some(Error::Path)
    );
}
#[test]
fn metadata_bound_covers_empty_and_growing_inventories() {
    let t = Temp::new();
    assert_eq!(
        Run::create(
            &t.output(),
            ctx(),
            &[],
            Limits {
                metadata_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let mut inventory = model::Inventory::new(Limits {
        metadata_bytes: 128 * 1024,
        ..Limits::default()
    });
    assert_eq!(
        inventory.push(Artifact {
            component: Component::Navigation,
            path: "mmaps/x".to_owned(),
            bytes: 1,
            sha256: [1; 32]
        }),
        Err(Error::Limit)
    );
}
#[test]
fn case_alias_files_and_directory_segments_cannot_finish() {
    for pair in [["mmaps/a", "mmaps/A"], ["mmaps/Dir/a", "mmaps/dir/b"]] {
        let t = Temp::new();
        let mut r = t.run();
        r.put(Component::Navigation, pair[0], b"x").unwrap();
        let second = r.put(Component::Navigation, pair[1], b"x");
        match second {
            Ok(_) => assert_eq!(
                r.finish_component(Component::Navigation).unwrap_err(),
                Error::Inventory
            ),
            Err(e) => assert_eq!(e, Error::Inventory),
        }
        assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    }
}
#[test]
fn file_replacement_and_context_or_lock_mutation_before_finish_reject() {
    for p in ["mmaps/x", ".state/context.json", ".state/lock"] {
        let t = Temp::new();
        let mut r = t.run();
        r.put(Component::Navigation, "mmaps/x", b"x").unwrap();
        r.finish_component(Component::Navigation).unwrap();
        let path = t.output().join(p);
        fs::remove_file(&path).unwrap();
        fs::write(path, b"changed").unwrap();
        assert!(r.finish().is_err());
        assert!(t.output().join(INCOMPLETE).is_file());
    }
}
#[test]
fn inventory_and_manifest_are_independent_of_put_order() {
    let a = Temp::new();
    let b = Temp::new();
    for (t, reverse) in [(&a, false), (&b, true)] {
        let mut r = t.run();
        let mut names = vec!["mmaps/a", "mmaps/B", "mmaps/c"];
        if reverse {
            names.reverse();
        }
        for p in names {
            r.put(Component::Navigation, p, p.as_bytes()).unwrap();
        }
        r.finish_component(Component::Navigation).unwrap();
        r.finish().unwrap();
    }
    assert_eq!(
        fs::read(a.output().join(MANIFEST)).unwrap(),
        fs::read(b.output().join(MANIFEST)).unwrap()
    );
    assert_eq!(a.verify().unwrap(), b.verify().unwrap());
}
#[test]
fn receipt_hash_uses_lowercase_path_identity() {
    let a = Artifact {
        component: Component::Navigation,
        path: "mmaps/A".to_owned(),
        bytes: 0,
        sha256: digest(b""),
    };
    assert_eq!(
        disk::receipt(&a),
        disk::receipt(&Artifact {
            path: "mmaps/a".to_owned(),
            ..a
        })
    );
}
