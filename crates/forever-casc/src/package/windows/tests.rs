// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: synthetic native-Windows package lifecycle/recovery cases.
// Cross-compilation checks these cases; native NTFS execution is separate evidence.
use super::*;
use std::fs;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-win-package-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
    fn run(&self) -> Run {
        Run::create(&self.output(), ctx(), &[], Limits::default()).unwrap()
    }
    fn resume(&self) -> Result<Run, Error> {
        Run::resume(&self.output(), ctx(), &[], Limits::default())
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
        components: 63,
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
    }
}
fn fact(path: &str) -> Artifact {
    Artifact {
        component: Component::Navigation,
        path: path.to_owned(),
        bytes: 4,
        sha256: digest(b"data"),
    }
}
fn partial(run: &mut Run, name: &str, bytes: &[u8]) {
    run.root
        .with_new_file(name, |w| w.write_all(bytes).map_err(|_| Error::Io))
        .unwrap();
}
fn intent(run: &mut Run, a: &Artifact) {
    publish(
        &mut run.root,
        TRANSACTION,
        INTENT_PART,
        transaction(ctx(), a).as_bytes(),
    )
    .unwrap();
}
fn finish(mut run: Run) -> Summary {
    for c in Component::ALL {
        run.finish_component(c).unwrap();
    }
    run.finish().unwrap()
}
#[test]
fn all_six_component_prefixes_create_reopen_and_readonly_inspect() {
    let t = Temp::new();
    let mut run = t.run();
    for (c, path) in [
        (Component::Db2, "dbc/esES/Map.db2"),
        (Component::GameTables, "gt/Armor.txt"),
        (Component::Cameras, "cameras/A.m2"),
        (Component::Terrain, "maps/0001_00_00.map"),
        (Component::Collision, "vmaps/1/1.vmtree"),
        (Component::Navigation, "mmaps/0001.mmap"),
    ] {
        assert_eq!(run.put(c, path, b"authored"), Ok(Put::Written));
    }
    let s = finish(run);
    assert_eq!(s.files, 6);
    assert_eq!(t.verify(), Ok(s));
    let mut count = 0;
    assert_eq!(
        verify_with(&t.output(), Some(ctx()), Limits::default(), |_, r| {
            let mut b = Vec::new();
            r.read_to_end(&mut b).unwrap();
            assert_eq!(b, b"authored");
            count += 1;
            Ok(())
        }),
        Ok(s)
    );
    assert_eq!(count, 6);
    assert!(matches!(t.resume(), Err(Error::Sealed)));
}
#[test]
fn exact_long_case_preserved_product_names() {
    let t = Temp::new();
    let mut run = t.run();
    let leaf = "A".repeat(255);
    let p = format!("mmaps/{leaf}");
    assert_eq!(
        run.put(Component::Navigation, &p, b"data"),
        Ok(Put::Written)
    );
    let p = format!(
        "vmaps/{}/{}/{}",
        "B".repeat(240),
        "C".repeat(240),
        "D".repeat(24)
    );
    assert_eq!(p.len(), 512);
    assert_eq!(run.put(Component::Collision, &p, b"data"), Ok(Put::Written));
    let p = format!("maps/{}/file", vec!["x"; 14].join("/"));
    assert_eq!(run.put(Component::Terrain, &p, b"data"), Ok(Put::Written));
    finish(run);
    assert_eq!(t.verify().unwrap().files, 3);
}
#[test]
fn rejects_aliases_reserved_names_depth_and_no_overwrite() {
    for bad in [
        "mmaps/CON.mmap".to_owned(),
        format!("mmaps/{}", "x".repeat(256)),
        format!("mmaps/{}/f", vec!["x"; 15].join("/")),
    ] {
        let t = Temp::new();
        let mut r = t.run();
        assert!(r.put(Component::Navigation, &bad, b"data").is_err());
        assert!(t.verify().is_err());
    }
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/Upper.mmap", b"data")
        .unwrap();
    assert!(
        r.put(Component::Navigation, "mmaps/upper.mmap", b"data")
            .is_err()
    );
    drop(r);
    assert!(t.verify().is_err());
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Ok(Put::AlreadyPresent)
    );
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"else"),
        Err(Error::Inventory)
    );
    drop(r);
    assert_eq!(fs::read(t.output().join("mmaps/a")).unwrap(), b"data");
}
#[test]
fn restart_replays_every_distinct_unsealed_fact() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    r.put(Component::Navigation, "mmaps/b", b"data").unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Ok(Put::AlreadyPresent)
    );
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Ok(Put::AlreadyPresent)
    );
    assert_eq!(
        r.finish_component(Component::Navigation),
        Err(Error::Incomplete)
    );
    drop(r);
    let mut r = t.resume().unwrap();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    r.put(Component::Navigation, "mmaps/b", b"data").unwrap();
    finish(r);
    t.verify().unwrap();
}
#[test]
fn sealed_components_skip_replay_and_reject_new_files() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    r.finish_component(Component::Navigation).unwrap();
    drop(r);
    let r = t.resume().unwrap();
    assert_eq!(
        r.completed_components().unwrap(),
        Component::Navigation.bit()
    );
    finish(r);
    t.verify().unwrap();
    let t = Temp::new();
    let mut r = t.run();
    r.finish_component(Component::Navigation).unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Err(Error::Sealed)
    );
}
#[test]
fn every_payload_journal_transition_recovers_with_exact_replay() {
    for stage in 0..6 {
        let t = Temp::new();
        let mut r = t.run();
        let a = fact("mmaps/a");
        intent(&mut r, &a);
        match stage {
            0 => {}
            1 => partial(&mut r, PAYLOAD_PART, b"da"),
            _ => {
                publish(&mut r.root, &a.path, PAYLOAD_PART, b"data").unwrap();
                if stage == 3 {
                    partial(&mut r, RECEIPT_PART, b"{");
                } else if stage >= 4 {
                    publish(&mut r.root, &receipt(&a), RECEIPT_PART, a.json().as_bytes()).unwrap();
                    if stage == 5 {
                        let e = entry(&observe(&r.root).unwrap(), TRANSACTION)
                            .unwrap()
                            .identity;
                        r.root.remove_same(TRANSACTION, e).unwrap();
                    }
                }
            }
        }
        drop(r);
        let mut r = t.resume().unwrap();
        assert_eq!(
            r.put(Component::Navigation, &a.path, b"data"),
            Ok(if stage < 2 {
                Put::Written
            } else {
                Put::AlreadyPresent
            })
        );
        finish(r);
        assert_eq!(t.verify().unwrap().files, 1);
    }
}
#[test]
fn partial_intent_seal_and_manifest_recover_without_adoption() {
    let t = Temp::new();
    let mut r = t.run();
    partial(&mut r, INTENT_PART, b"{");
    drop(r);
    let r = t.resume().unwrap();
    finish(r);
    t.verify().unwrap();
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    partial(&mut r, ".state/sealed/.mmaps.part", b"{");
    drop(r);
    let mut r = t.resume().unwrap();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    finish(r);
    t.verify().unwrap();
    for final_manifest in [false, true] {
        let t = Temp::new();
        let mut r = t.run();
        for c in Component::ALL {
            r.finish_component(c).unwrap();
        }
        if final_manifest {
            let es = observe(&r.root).unwrap();
            let inv = collect(&r.root, &es, ctx(), Limits::default()).unwrap();
            publish_manifest(&mut r.root, ctx(), &inv, Limits::default()).unwrap();
        } else {
            partial(&mut r, MANIFEST_PART, b"{");
        }
        drop(r);
        finish(t.resume().unwrap());
        t.verify().unwrap();
    }
}
#[test]
fn preflight_rejects_unknown_inventory_before_any_recovery_deletion() {
    let t = Temp::new();
    let mut r = t.run();
    let a = fact("mmaps/a");
    intent(&mut r, &a);
    partial(&mut r, PAYLOAD_PART, b"da");
    partial(&mut r, "unknown", b"extra");
    drop(r);
    assert!(t.resume().is_err());
    assert_eq!(fs::read(t.output().join(PAYLOAD_PART)).unwrap(), b"da");
    assert_eq!(fs::read(t.output().join("unknown")).unwrap(), b"extra");
}
#[test]
fn context_version_and_conflicting_states_leave_pending_untouched() {
    let t = Temp::new();
    let mut r = t.run();
    partial(&mut r, INTENT_PART, b"{");
    drop(r);
    let mut other = ctx();
    other.plan_sha256 = [9; 32];
    assert!(matches!(
        Run::resume(&t.output(), other, &[], Limits::default()),
        Err(Error::Context)
    ));
    assert_eq!(fs::read(t.output().join(INTENT_PART)).unwrap(), b"{");
    let t = Temp::new();
    drop(t.run());
    fs::write(t.output().join(PROTOCOL), b"{\"state_version\":2}").unwrap();
    assert!(matches!(t.resume(), Err(Error::StateVersion)));
    let t = Temp::new();
    let mut r = t.run();
    let a = fact("mmaps/a");
    intent(&mut r, &a);
    publish(&mut r.root, &a.path, PAYLOAD_PART, b"data").unwrap();
    partial(&mut r, PAYLOAD_PART, b"d");
    drop(r);
    assert!(t.resume().is_err());
    assert_eq!(fs::read(t.output().join(PAYLOAD_PART)).unwrap(), b"d");
}
#[test]
fn receipt_without_payload_and_excess_partial_are_terminal() {
    for large in [false, true] {
        let t = Temp::new();
        let mut r = t.run();
        let a = fact("mmaps/a");
        intent(&mut r, &a);
        if large {
            partial(&mut r, PAYLOAD_PART, b"data!");
        } else {
            publish(&mut r.root, &receipt(&a), RECEIPT_PART, a.json().as_bytes()).unwrap();
        }
        drop(r);
        assert!(t.resume().is_err());
    }
}
#[test]
fn locks_exclude_writer_and_shared_verifier_while_writing() {
    let t = Temp::new();
    let r = t.run();
    assert!(matches!(t.resume(), Err(Error::Busy)));
    assert!(matches!(t.verify(), Err(Error::Busy)));
    drop(r);
    finish(t.resume().unwrap());
    verify_with(&t.output(), None, Limits::default(), |_, _| Ok(())).unwrap();
}
#[test]
fn callback_has_shared_lock_and_runs_after_integrity_preflight() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    finish(r);
    verify_with(&t.output(), None, Limits::default(), |_, _| {
        assert!(matches!(t.resume(), Err(Error::Busy)));
        Ok(())
    })
    .unwrap();
    fs::write(t.output().join("mmaps/a"), b"else").unwrap();
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
fn common_downward_file_byte_total_directory_metadata_and_manifest_limits() {
    let t = Temp::new();
    let mut r = Run::create(
        &t.output(),
        ctx(),
        &[],
        Limits {
            files: 1,
            total_bytes: 4,
            file_bytes: 4,
            ..Limits::default()
        },
    )
    .unwrap();
    r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/b", b"x"),
        Err(Error::Limit)
    );
    let t = Temp::new();
    let mut r = Run::create(
        &t.output(),
        ctx(),
        &[],
        Limits {
            file_bytes: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Err(Error::Limit)
    );
    let t = Temp::new();
    assert!(matches!(
        Run::create(
            &t.output(),
            ctx(),
            &[],
            Limits {
                directories: 2,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    ));
    let t = Temp::new();
    assert!(matches!(
        Run::create(
            &t.output(),
            ctx(),
            &[],
            Limits {
                metadata_bytes: 128 * 1024 - 1,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    ));
    let t = Temp::new();
    let mut r = Run::create(
        &t.output(),
        ctx(),
        &[],
        Limits {
            manifest_bytes: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    for c in Component::ALL {
        r.finish_component(c).unwrap();
    }
    assert_eq!(r.finish(), Err(Error::Limit));
    assert!(t.output().join(INCOMPLETE).exists());
    assert!(!t.output().join(MANIFEST).exists());
}

#[test]
fn every_interrupted_empty_initialization_boundary_has_explicit_recovery() {
    for stage in 0..10 {
        let t = Temp::new();
        let mut root = Root::create(&t.output(), native_limits(Limits::default())).unwrap();
        let lock = root.lock(true).unwrap();
        if stage >= 1 {
            root.create_dirs(".state").unwrap();
        }
        if stage >= 2 {
            root.create_dirs(".state/files").unwrap();
        }
        if stage >= 3 {
            root.create_dirs(".state/sealed").unwrap();
        }
        if stage == 4 {
            root.with_new_file(".state/incomplete.part", |w| {
                w.write_all(b"inc").map_err(|_| Error::Io)
            })
            .unwrap();
        }
        if stage >= 5 {
            publish(
                &mut root,
                INCOMPLETE,
                ".state/incomplete.part",
                b"incomplete\n",
            )
            .unwrap();
        }
        if stage == 6 {
            root.with_new_file(".state/context.part", |w| {
                w.write_all(b"{").map_err(|_| Error::Io)
            })
            .unwrap();
        }
        if stage >= 7 {
            publish(
                &mut root,
                ".state/context.json",
                ".state/context.part",
                ctx().json().as_bytes(),
            )
            .unwrap();
        }
        if stage == 8 {
            root.with_new_file(".state/protocol.part", |w| {
                w.write_all(b"{").map_err(|_| Error::Io)
            })
            .unwrap();
        }
        if stage >= 9 {
            publish(
                &mut root,
                PROTOCOL,
                ".state/protocol.part",
                protocol_json(ctx()).as_bytes(),
            )
            .unwrap();
        }
        drop(lock);
        drop(root);
        finish(t.resume().unwrap());
        t.verify().unwrap();
    }
}
#[test]
fn interrupted_initialization_preflights_unknown_files_before_cleanup() {
    let t = Temp::new();
    let mut root = Root::create(&t.output(), native_limits(Limits::default())).unwrap();
    let lock = root.lock(true).unwrap();
    root.create_dirs(".state").unwrap();
    root.with_new_file(".state/context.part", |w| {
        w.write_all(b"{").map_err(|_| Error::Io)
    })
    .unwrap();
    root.with_new_file("unknown", |w| {
        w.write_all(b"authored").map_err(|_| Error::Io)
    })
    .unwrap();
    drop(lock);
    drop(root);
    assert!(t.resume().is_err());
    assert_eq!(
        fs::read(t.output().join(".state/context.part")).unwrap(),
        b"{"
    );
    assert!(!t.output().join(".state/context.json").exists());
}

#[test]
fn empty_create_before_lock_recovers_but_missing_initialized_lock_never_recreates() {
    let t = Temp::new();
    drop(Root::create(&t.output(), native_limits(Limits::default())).unwrap());
    finish(t.resume().unwrap());
    t.verify().unwrap();
    let t = Temp::new();
    drop(t.run());
    fs::remove_file(t.output().join(LOCK)).unwrap();
    assert!(matches!(t.resume(), Err(Error::Inventory)));
    assert!(!t.output().join(LOCK).exists());
    let t = Temp::new();
    finish(t.run());
    fs::remove_file(t.output().join(LOCK)).unwrap();
    assert!(t.resume().is_err());
    assert!(!t.output().join(LOCK).exists());
}
#[test]
fn foreign_existing_payload_is_not_given_a_new_intent() {
    let t = Temp::new();
    let mut r = t.run();
    r.root.create_dirs("mmaps").unwrap();
    partial(&mut r, "mmaps/a", b"data");
    assert_eq!(
        r.put(Component::Navigation, "mmaps/a", b"data"),
        Err(Error::Exists)
    );
    drop(r);
    assert!(!t.output().join(TRANSACTION).exists());
    assert_eq!(fs::read(t.output().join("mmaps/a")).unwrap(), b"data");
    assert!(t.resume().is_err());
}

#[test]
fn recovery_postflight_rejects_new_foreign_facts_and_changed_retained_payloads() {
    for change in 0..3 {
        let t = Temp::new();
        let mut r = t.run();
        r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
        drop(r);
        let output = t.output();
        let result = recover::resume_for_test(&output, ctx(), Limits::default(), |root| {
            if change == 0 {
                publish(root, "unknown", ".state/foreign.part", b"new")?;
            } else if change == 1 {
                let a = fact("mmaps/foreign");
                publish(root, &a.path, PAYLOAD_PART, b"data")?;
                publish(root, &receipt(&a), RECEIPT_PART, a.json().as_bytes())?;
            } else {
                fs::write(output.join("mmaps/a"), b"else").map_err(|_| Error::Io)?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert!(output.join(INCOMPLETE).exists());
        assert!(!output.join(MANIFEST).exists());
    }
}
#[test]
fn final_publication_keeps_original_payload_and_namespace_identities() {
    for changed_payload in [false, true] {
        let t = Temp::new();
        let mut r = t.run();
        r.put(Component::Navigation, "mmaps/a", b"data").unwrap();
        for c in Component::ALL {
            r.finish_component(c).unwrap();
        }
        let output = t.output();
        assert!(
            r.finish_for_test(|root| {
                if changed_payload {
                    fs::write(output.join("mmaps/a"), b"else").map_err(|_| Error::Io)?;
                } else {
                    publish(root, "unknown", ".state/foreign.part", b"new")?;
                }
                Ok(())
            })
            .is_err()
        );
        assert!(output.join(INCOMPLETE).exists());
        assert!(output.join(MANIFEST).exists());
        assert!(t.verify().is_err());
    }
}

#[test]
fn interrupted_initialization_never_adopts_new_manifest_or_replaced_control() {
    for replace_control in [false, true] {
        let t = Temp::new();
        drop(Root::create(&t.output(), native_limits(Limits::default())).unwrap());
        let output = t.output();
        let result = recover::resume_for_test(&output, ctx(), Limits::default(), |root| {
            if replace_control {
                let name = ".state/context.json";
                let identity = root.file_identity(name)?.ok_or(Error::Inventory)?;
                root.remove_same(name, identity)?;
                publish(root, name, ".state/context.part", ctx().json().as_bytes())?;
            } else {
                publish(root, MANIFEST, ".state/foreign.part", b"{}")?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert!(output.join(INCOMPLETE).exists());
        assert!(t.verify().is_err());
    }
}
