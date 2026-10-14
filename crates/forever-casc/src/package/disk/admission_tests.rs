// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-14: sealed comparison and complete metadata admission guards.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const SOURCE: &str = "dbc/esES/CinematicCamera.db2";
const CAMERA: &str = "cameras/authored.m2";
const BYTES: &[u8] = b"authored source";

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-package-admission-{}-{}",
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
        Run::create(&self.output(), context(), &[], Limits::default()).unwrap()
    }
    fn sealed(&self) -> Run {
        let mut run = self.run();
        run.put(Component::Db2, SOURCE, BYTES).unwrap();
        run.finish_component(Component::Db2).unwrap();
        run
    }
    fn complete(&self, reverse: bool) -> Summary {
        let mut run = self.run();
        for (c, p, b) in if reverse {
            [
                (Component::Cameras, CAMERA, b"camera".as_slice()),
                (Component::Db2, SOURCE, BYTES),
            ]
        } else {
            [
                (Component::Db2, SOURCE, BYTES),
                (Component::Cameras, CAMERA, b"camera".as_slice()),
            ]
        } {
            run.put(c, p, b).unwrap();
        }
        run.finish_component(Component::Db2).unwrap();
        run.finish_component(Component::Cameras).unwrap();
        run.finish().unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn context() -> Context {
    Context {
        build: 70338,
        locale: Locale::EsEs,
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
        components: Component::Db2.bit() | Component::Cameras.bit(),
    }
}
fn expected() -> Artifact {
    Artifact {
        component: Component::Db2,
        path: SOURCE.to_owned(),
        bytes: BYTES.len() as u64,
        sha256: digest(BYTES),
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Entry {
    path: PathBuf,
    mode: u32,
    ino: u64,
    links: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
    bytes: Vec<u8>,
}
fn snapshot(root: &Path) -> Vec<Entry> {
    fn walk(root: &Path, path: &Path, out: &mut Vec<Entry>) {
        let m = fs::symlink_metadata(path).unwrap();
        let bytes = if m.is_file() {
            fs::read(path).unwrap()
        } else if m.file_type().is_symlink() {
            fs::read_link(path)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else {
            Vec::new()
        };
        out.push(Entry {
            path: path.strip_prefix(root).unwrap().to_owned(),
            mode: m.mode(),
            ino: m.ino(),
            links: m.nlink(),
            len: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
            bytes,
        });
        if m.is_dir() {
            for e in fs::read_dir(path).unwrap() {
                walk(root, &e.unwrap().path(), out);
            }
        }
    }
    let mut entries = Vec::new();
    walk(root, root, &mut entries);
    entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    entries
}
fn counters(run: &Run) -> (usize, u64, usize, Option<(usize, bool)>) {
    (
        run.files,
        run.bytes,
        run.dirs,
        run.replay
            .as_ref()
            .map(|r| (r.bytes(), r.pending(Component::Cameras))),
    )
}
fn reject_readonly(t: &Temp, run: &mut Run, fact: &Artifact) -> Error {
    let before = snapshot(&t.output());
    let counts = counters(run);
    let error = run.verify_sealed_artifact(fact).unwrap_err();
    assert!(run.failed);
    assert_eq!(counts, counters(run));
    assert_eq!(before, snapshot(&t.output()));
    assert_eq!(
        run.put(Component::Cameras, CAMERA, b"camera"),
        Err(Error::Incomplete)
    );
    assert_eq!(
        run.finish_component(Component::Cameras),
        Err(Error::Incomplete)
    );
    assert_eq!(before, snapshot(&t.output()));
    error
}

#[test]
fn sealed_comparison_fresh_is_readonly_and_does_not_change_counters() {
    let t = Temp::new();
    let mut run = t.sealed();
    let before = snapshot(&t.output());
    let counts = counters(&run);
    run.verify_sealed_artifact(&expected()).unwrap();
    run.verify_sealed_artifact(&expected()).unwrap();
    assert_eq!(before, snapshot(&t.output()));
    assert_eq!(counts, counters(&run));
    assert!(!run.failed);
    run.finish_component(Component::Cameras).unwrap();
    assert_eq!(run.finish().unwrap().files, 1);
}

#[test]
fn sealed_comparison_resume_preserves_unfinished_camera_replay() {
    let t = Temp::new();
    let mut run = t.sealed();
    run.put(Component::Cameras, CAMERA, b"camera").unwrap();
    drop(run);
    let mut run = Run::resume(&t.output(), context(), &[], Limits::default()).unwrap();
    let before = snapshot(&t.output());
    let counts = counters(&run);
    assert!(run.replay.as_ref().unwrap().pending(Component::Cameras));
    run.verify_sealed_artifact(&expected()).unwrap();
    assert_eq!(before, snapshot(&t.output()));
    assert_eq!(counts, counters(&run));
    run.put(Component::Cameras, CAMERA, b"camera").unwrap();
    run.finish_component(Component::Cameras).unwrap();
    assert_eq!(run.finish().unwrap().files, 2);
}

#[test]
fn sealed_comparison_requires_receipt_and_seal_without_publication() {
    for published in [false, true] {
        let t = Temp::new();
        let mut run = t.run();
        if published {
            run.put(Component::Db2, SOURCE, BYTES).unwrap();
        } else {
            run.finish_component(Component::Db2).unwrap();
        }
        reject_readonly(&t, &mut run, &expected());
        assert!(!t.output().join(SOURCE).exists() || published);
    }
}

#[test]
fn sealed_comparison_rejects_mismatched_caller_identity_and_invalid_paths() {
    for kind in 0..6 {
        let t = Temp::new();
        let mut run = t.sealed();
        let mut a = expected();
        match kind {
            0 => a.bytes += 1,
            1 => a.sha256[0] ^= 1,
            2 => a.path = "dbc/esES/Missing.db2".to_owned(),
            3 => a.path = "../escape".to_owned(),
            4 => a.component = Component::Cameras,
            _ => a.bytes = Limits::default().file_bytes + 1,
        }
        reject_readonly(&t, &mut run, &a);
    }
}

#[test]
fn sealed_comparison_missing_or_tampered_publication_state_poison_readonly() {
    for kind in 0..9 {
        let t = Temp::new();
        let mut run = t.sealed();
        let receipt = t.output().join(receipt(&expected()));
        let seal = t.output().join(".state/sealed/db2");
        match kind {
            0 => fs::remove_file(receipt).unwrap(),
            1 => fs::write(receipt, b"changed").unwrap(),
            2 => fs::remove_file(seal).unwrap(),
            3 => fs::write(seal, b"changed").unwrap(),
            4 => fs::remove_file(t.output().join(SOURCE)).unwrap(),
            5 => fs::write(t.output().join(SOURCE), b"changed source").unwrap(),
            6 => fs::write(t.output().join(".state/context.json"), b"changed").unwrap(),
            7 => fs::write(t.output().join(recover::PROTOCOL), b"changed").unwrap(),
            _ => {
                fs::remove_file(t.output().join(".state/lock")).unwrap();
                fs::write(t.output().join(".state/lock"), b"").unwrap();
            }
        }
        reject_readonly(&t, &mut run, &expected());
    }
}

#[test]
fn sealed_comparison_rejects_symlink_hardlink_and_fifo_without_writes() {
    for kind in 0..3 {
        let t = Temp::new();
        let mut run = t.sealed();
        let payload = t.output().join(SOURCE);
        let saved = t.0.join("saved");
        fs::rename(&payload, &saved).unwrap();
        match kind {
            0 => std::os::unix::fs::symlink(&saved, &payload).unwrap(),
            1 => fs::hard_link(&saved, &payload).unwrap(),
            _ => assert!(
                std::process::Command::new("mkfifo")
                    .arg(&payload)
                    .status()
                    .unwrap()
                    .success()
            ),
        }
        reject_readonly(&t, &mut run, &expected());
    }
}

#[test]
fn sealed_comparison_already_failed_writer_never_reads_or_publishes() {
    let t = Temp::new();
    let mut run = t.sealed();
    run.failed = true;
    fs::remove_file(t.output().join(SOURCE)).unwrap();
    assert_eq!(
        reject_readonly(&t, &mut run, &expected()),
        Error::Incomplete
    );
}

#[test]
fn inventory_preflight_admits_whole_declared_total_before_any_payload_read() {
    for reverse in [false, true] {
        let t = Temp::new();
        t.complete(reverse);
        // A missing payload proves the metadata rejection precedes even hashing
        // payload bytes, rather than only preceding the inspection callback.
        fs::remove_file(t.output().join(CAMERA)).unwrap();
        let mut admitted = 0;
        let mut inspected = 0;
        let error = verify_with_preflight(
            &t.output(),
            Some(context()),
            Limits::default(),
            |facts| {
                admitted += 1;
                assert_eq!(facts.len(), 2);
                assert_eq!(facts[0].component, Component::Cameras);
                assert_eq!(facts[1].component, Component::Db2);
                let total = facts
                    .iter()
                    .try_fold(0u64, |n, a| n.checked_add(a.bytes).ok_or(Error::Limit))?;
                if total > BYTES.len() as u64 {
                    return Err(Error::Limit);
                }
                Ok(())
            },
            |_, _| {
                inspected += 1;
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error, Error::Limit);
        assert_eq!((admitted, inspected), (1, 0));
    }
}

#[test]
fn inventory_preflight_valid_success_matches_legacy_and_is_readonly() {
    let t = Temp::new();
    let summary = t.complete(false);
    let before = snapshot(&t.output());
    let mut admitted = 0;
    let mut inspected = 0;
    let actual = verify_with_preflight(
        &t.output(),
        None,
        Limits::default(),
        |facts| {
            admitted += 1;
            assert_eq!(facts.len(), 2);
            Ok(())
        },
        |a, f| {
            inspected += 1;
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes).unwrap();
            assert_eq!(digest(&bytes), a.sha256);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!((admitted, inspected), (1, 2));
    assert_eq!(summary, actual);
    assert_eq!(
        actual,
        verify(&t.output(), None, Limits::default()).unwrap()
    );
    assert_eq!(before, snapshot(&t.output()));
}

#[test]
fn inventory_preflight_invalid_receipts_or_seals_never_admit() {
    for kind in 0..3 {
        let t = Temp::new();
        t.complete(false);
        let path = match kind {
            0 => t.output().join(MANIFEST),
            1 => t.output().join(receipt(&expected())),
            _ => t.output().join(".state/sealed/db2"),
        };
        fs::write(path, b"changed").unwrap();
        let mut called = false;
        assert!(
            verify_with_preflight(
                &t.output(),
                None,
                Limits::default(),
                |_| {
                    called = true;
                    Ok(())
                },
                |_, _| panic!("payload inspection must not run")
            )
            .is_err()
        );
        assert!(!called);
    }
}

#[test]
fn inventory_preflight_mutation_is_rejected_before_payload_inspection() {
    let t = Temp::new();
    t.complete(false);
    assert!(
        verify_with_preflight(
            &t.output(),
            None,
            Limits::default(),
            |_| {
                fs::write(t.output().join(CAMERA), b"changed").unwrap();
                Ok(())
            },
            |_, _| panic!("payload inspection must not run")
        )
        .is_err()
    );
}

#[test]
fn inventory_preflight_keeps_post_inspection_namespace_checks() {
    let t = Temp::new();
    t.complete(false);
    assert!(
        verify_with_preflight(
            &t.output(),
            None,
            Limits::default(),
            |_| Ok(()),
            |_, _| {
                fs::write(t.output().join("cameras/extra"), b"extra").unwrap();
                Ok(())
            }
        )
        .is_err()
    );
}
