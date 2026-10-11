// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: exact synthetic interrupted publication states.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-package-resume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn out(&self) -> PathBuf {
        self.0.join("output")
    }
    fn run(&self) -> Run {
        Run::create(&self.out(), ctx(), &[], Default::default()).unwrap()
    }
    fn resume(&self) -> Result<Run, Error> {
        Run::resume(&self.out(), ctx(), &[], Default::default())
    }
    fn write(&self, name: &str, b: &[u8]) {
        let p = self.out().join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, b).unwrap();
    }
    fn link(&self, name: &str, temp: &str) {
        fs::hard_link(self.out().join(name), self.out().join(temp)).unwrap();
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
        locale: Locale::EnUs,
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
        components: Component::Navigation.bit(),
    }
}
fn artifact() -> Artifact {
    Artifact {
        component: Component::Navigation,
        path: "mmaps/dir/x".to_owned(),
        bytes: 3,
        sha256: digest(b"abc"),
    }
}
fn intent(r: &mut Run) {
    r.root
        .publish(
            TRANSACTION,
            transaction(ctx(), &artifact()).as_bytes(),
            &mut r.dirs,
            r.limits,
        )
        .unwrap();
}
fn complete(r: Run) -> Summary {
    let mut r = r;
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    r.finish_component(Component::Navigation).unwrap();
    r.finish().unwrap()
}
fn snapshot(t: &Temp) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, p: &Path, v: &mut Vec<(String, Vec<u8>)>) {
        for e in fs::read_dir(p).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                walk(root, &e.path(), v)
            } else if e.file_type().unwrap().is_file() {
                v.push((
                    e.path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    fs::read(e.path()).unwrap(),
                ));
            }
        }
    }
    let mut v = Vec::new();
    walk(&t.out(), &t.out(), &mut v);
    v.sort();
    v
}
#[test]
fn clean_checkpoint_resumes_replays_and_preserves_exact_manifest() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(r.completed_components().unwrap(), 0);
    assert_eq!(
        r.put(Component::Navigation, &artifact().path, b"abc")
            .unwrap(),
        Put::AlreadyPresent
    );
    let s = complete(r);
    assert_eq!(s.files, 1);
    assert_eq!(
        verify(&t.out(), Some(ctx()), Default::default()).unwrap(),
        s
    );
}
#[test]
fn sealed_component_resumes_and_can_be_skipped_or_replayed() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    r.finish_component(Component::Navigation).unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(r.completed_components().unwrap(), 32);
    assert_eq!(
        r.put(Component::Navigation, &artifact().path, b"abc")
            .unwrap(),
        Put::AlreadyPresent
    );
    r.finish_component(Component::Navigation).unwrap();
    assert_eq!(r.finish().unwrap().files, 1);
}
#[test]
fn intent_without_payload_requires_exact_replay_and_blocks_finish() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(
        r.finish_component(Component::Navigation),
        Err(Error::Incomplete)
    );
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    let r = t.resume().unwrap();
    assert_eq!(complete(r).files, 1);
}
#[test]
fn intent_rejects_changed_replayed_bytes_permanently() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(
        r.put(Component::Navigation, &artifact().path, b"abd")
            .unwrap_err(),
        Error::Inventory
    );
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    assert!(!t.out().join(&artifact().path).exists());
}
#[test]
fn torn_and_fully_written_unpublished_payloads_are_discarded_then_replayed() {
    for bytes in [&b"a"[..], &b"wrong"[..0], &b"abd"[..], &b"abc"[..]] {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        drop(r);
        let temp = pending(&artifact().path).unwrap();
        t.write(&temp, bytes);
        let r = t.resume().unwrap();
        assert!(!t.out().join(&temp).exists());
        assert_eq!(complete(r).files, 1);
    }
}
#[test]
fn payload_published_before_receipt_is_authenticated_and_adopted() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    r.root
        .publish(&artifact().path, b"abc", &mut r.dirs, r.limits)
        .unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(r.files, 1);
    assert!(!t.out().join(TRANSACTION).exists());
    assert_eq!(
        r.put(Component::Navigation, &artifact().path, b"abc")
            .unwrap(),
        Put::AlreadyPresent
    );
    complete(r);
}
#[test]
fn two_link_payload_publication_is_repaired() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    r.root
        .publish(&artifact().path, b"abc", &mut r.dirs, r.limits)
        .unwrap();
    drop(r);
    let temp = pending(&artifact().path).unwrap();
    t.link(&artifact().path, &temp);
    let r = t.resume().unwrap();
    assert!(!t.out().join(temp).exists());
    assert_eq!(
        fs::metadata(t.out().join(&artifact().path))
            .unwrap()
            .nlink(),
        1
    );
    complete(r);
}
#[test]
fn torn_receipt_before_publication_is_rebuilt_from_verified_payload() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    r.root
        .publish(&artifact().path, b"abc", &mut r.dirs, r.limits)
        .unwrap();
    drop(r);
    let name = receipt(&artifact());
    let temp = pending(&name).unwrap();
    t.write(&temp, b"{");
    let r = t.resume().unwrap();
    assert_eq!(
        fs::read(t.out().join(name)).unwrap(),
        artifact().json().as_bytes()
    );
    assert!(!t.out().join(temp).exists());
    complete(r);
}
#[test]
fn two_link_receipt_publication_is_repaired() {
    let t = Temp::new();
    let mut r = t.run();
    intent(&mut r);
    r.root
        .publish(&artifact().path, b"abc", &mut r.dirs, r.limits)
        .unwrap();
    let name = receipt(&artifact());
    r.root
        .publish(&name, artifact().json().as_bytes(), &mut r.dirs, r.limits)
        .unwrap();
    drop(r);
    let temp = pending(&name).unwrap();
    t.link(&name, &temp);
    let r = t.resume().unwrap();
    assert!(!t.out().join(temp).exists());
    assert_eq!(r.files, 1);
    complete(r);
}
#[test]
fn intent_pending_publication_pair_and_unpublished_prefix_recover() {
    for paired in [false, true] {
        let t = Temp::new();
        let mut r = t.run();
        let temp = pending(TRANSACTION).unwrap();
        if paired {
            intent(&mut r);
            drop(r);
            t.link(TRANSACTION, &temp);
        } else {
            drop(r);
            t.write(&temp, b"{\"transaction");
        }
        let r = t.resume().unwrap();
        assert!(!t.out().join(temp).exists());
        complete(r);
    }
}
#[test]
fn final_receipt_with_intent_left_behind_replays_idempotently() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    intent(&mut r);
    drop(r);
    let r = t.resume().unwrap();
    assert!(!t.out().join(TRANSACTION).exists());
    complete(r);
}
#[test]
fn component_seal_publication_pair_is_authenticated_and_repaired() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    r.finish_component(Component::Navigation).unwrap();
    drop(r);
    let name = ".state/sealed/mmaps";
    let temp = pending(name).unwrap();
    t.link(name, &temp);
    let r = t.resume().unwrap();
    assert_eq!(r.completed_components().unwrap(), 32);
    assert!(!t.out().join(temp).exists());
    r.finish().unwrap();
}
#[test]
fn unpublished_component_seal_requires_new_producer_completion() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    drop(r);
    t.write(".state/sealed/.mmaps.part", b"{");
    let r = t.resume().unwrap();
    assert_eq!(r.completed_components().unwrap(), 0);
    complete(r);
}
#[test]
fn full_manifest_with_incomplete_marker_finishes_without_rewriting() {
    let t = Temp::new();
    complete(t.run());
    let original = fs::read(t.out().join(MANIFEST)).unwrap();
    let inode = fs::metadata(t.out().join(MANIFEST)).unwrap().ino();
    t.write(INCOMPLETE, b"incomplete\n");
    let r = t.resume().unwrap();
    r.finish().unwrap();
    assert_eq!(fs::read(t.out().join(MANIFEST)).unwrap(), original);
    assert_eq!(fs::metadata(t.out().join(MANIFEST)).unwrap().ino(), inode);
}
#[test]
fn two_link_final_manifest_publication_is_repaired() {
    let t = Temp::new();
    complete(t.run());
    t.write(INCOMPLETE, b"incomplete\n");
    t.link(MANIFEST, ".state/package.part");
    let r = t.resume().unwrap();
    assert!(!t.out().join(".state/package.part").exists());
    r.finish().unwrap();
    verify(&t.out(), Some(ctx()), Default::default()).unwrap();
}
#[test]
fn partial_unpublished_manifest_is_discarded_and_regenerated() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    r.finish_component(Component::Navigation).unwrap();
    drop(r);
    t.write(".state/package.part", b"{\"manifest_version\":1");
    let r = t.resume().unwrap();
    assert!(!t.out().join(".state/package.part").exists());
    r.finish().unwrap();
}
#[test]
fn context_change_active_writer_and_complete_stage_reject_without_mutation() {
    let t = Temp::new();
    let r = t.run();
    let before = snapshot(&t);
    assert_eq!(t.resume().err(), Some(Error::Busy));
    assert_eq!(snapshot(&t), before);
    drop(r);
    assert_eq!(
        Run::resume(
            &t.out(),
            Context {
                plan_sha256: [4; 32],
                ..ctx()
            },
            &[],
            Default::default()
        )
        .err(),
        Some(Error::Context)
    );
    assert_eq!(snapshot(&t), before);
    complete(t.resume().unwrap());
    let before = snapshot(&t);
    assert_eq!(t.resume().err(), Some(Error::Sealed));
    assert_eq!(snapshot(&t), before);
}
#[test]
fn legacy_stage_and_unknown_state_versions_reject_without_migration() {
    for version in [None, Some(3)] {
        let t = Temp::new();
        drop(t.run());
        if let Some(n) = version {
            t.write(
                PROTOCOL,
                protocol_json(ctx()).replace("\":2", "\":3").as_bytes(),
            );
            assert_eq!(n, 3);
        } else {
            fs::remove_file(t.out().join(PROTOCOL)).unwrap();
        }
        let before = snapshot(&t);
        assert_eq!(t.resume().err(), Some(Error::StateVersion));
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn unknown_payloads_or_temporaries_prevent_all_cleanup() {
    for name in [
        "mmaps/extra",
        "mmaps/.extra.part",
        ".state/extra",
        ".state/files/.unknown.part",
    ] {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        drop(r);
        t.write(&pending(&artifact().path).unwrap(), b"a");
        t.write(name, b"unknown");
        let before = snapshot(&t);
        assert!(t.resume().is_err());
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn tampered_final_payload_and_receipt_prevent_all_cleanup() {
    for name in [artifact().path, receipt(&artifact())] {
        let t = Temp::new();
        let mut r = t.run();
        r.put(Component::Navigation, &artifact().path, b"abc")
            .unwrap();
        intent(&mut r);
        drop(r);
        t.write(&pending(TRANSACTION).unwrap(), b"{");
        t.write(&name, b"bad");
        let before = snapshot(&t);
        assert!(t.resume().is_err());
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn conflicting_pending_inode_or_extra_hardlink_reject_without_cleanup() {
    for hardlink in [false, true] {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        r.root
            .publish(&artifact().path, b"abc", &mut r.dirs, r.limits)
            .unwrap();
        drop(r);
        let temp = pending(&artifact().path).unwrap();
        if hardlink {
            t.link(&artifact().path, &temp);
            fs::hard_link(t.out().join(&artifact().path), t.0.join("outside-link")).unwrap();
        } else {
            t.write(&temp, b"abc");
        }
        let before = snapshot(&t);
        assert!(t.resume().is_err());
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn symlink_directory_and_fifo_pending_files_are_never_removed() {
    for mode in 0..3 {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        drop(r);
        let temp = pending(&artifact().path).unwrap();
        let path = t.out().join(&temp);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        match mode {
            0 => std::os::unix::fs::symlink(t.0.join("missing"), &path).unwrap(),
            1 => fs::create_dir(&path).unwrap(),
            _ => assert!(
                std::process::Command::new("mkfifo")
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success()
            ),
        }
        assert!(t.resume().is_err());
        assert!(fs::symlink_metadata(path).is_ok());
    }
}
#[test]
fn oversized_pending_and_contradictory_receipt_are_preserved() {
    for mode in 0..2 {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        drop(r);
        if mode == 0 {
            t.write(&pending(&artifact().path).unwrap(), b"longer");
        } else {
            t.write(&receipt(&artifact()), artifact().json().as_bytes());
        }
        let before = snapshot(&t);
        assert!(t.resume().is_err());
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn resource_limits_apply_before_cleanup_and_include_pending_artifact() {
    for mode in 0..4 {
        let t = Temp::new();
        let mut r = t.run();
        intent(&mut r);
        drop(r);
        t.write(&pending(&artifact().path).unwrap(), b"a");
        let before = snapshot(&t);
        let mut limits = Limits::default();
        match mode {
            0 => limits.files = 0,
            1 => limits.total_bytes = 2,
            2 => limits.directories = 3,
            _ => limits.metadata_bytes = 128 * 1024,
        };
        assert_eq!(
            Run::resume(&t.out(), ctx(), &[], limits).err(),
            Some(Error::Limit)
        );
        assert_eq!(snapshot(&t), before);
    }
}
#[test]
fn resume_enforces_git_and_protected_input_boundaries() {
    let t = Temp::new();
    drop(t.run());
    assert_eq!(
        Run::resume(&t.out(), ctx(), &[&t.0], Default::default()).err(),
        Some(Error::Path)
    );
    fs::create_dir(t.0.join(".git")).unwrap();
    assert_eq!(t.resume().err(), Some(Error::Path));
}
#[test]
fn clean_v1_completed_packages_still_verify_without_protocol_file() {
    let t = Temp::new();
    let s = complete(t.run());
    fs::remove_file(t.out().join(PROTOCOL)).unwrap();
    assert_eq!(
        verify(&t.out(), Some(ctx()), Default::default()).unwrap(),
        s
    );
}
#[test]
fn unrelated_finished_component_survives_pending_component_recovery() {
    let t = Temp::new();
    let context = Context {
        components: Component::Navigation.bit() | Component::Terrain.bit(),
        ..ctx()
    };
    let mut r = Run::create(&t.out(), context, &[], Default::default()).unwrap();
    r.put(Component::Terrain, "maps/x", b"terrain").unwrap();
    r.finish_component(Component::Terrain).unwrap();
    r.root
        .publish(
            TRANSACTION,
            transaction(context, &artifact()).as_bytes(),
            &mut r.dirs,
            r.limits,
        )
        .unwrap();
    drop(r);
    let mut r = Run::resume(&t.out(), context, &[], Default::default()).unwrap();
    assert_eq!(r.completed_components().unwrap(), 8);
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    r.finish_component(Component::Navigation).unwrap();
    assert_eq!(r.finish().unwrap().files, 2);
}
#[test]
fn earlier_completed_files_can_replay_before_outstanding_transaction() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/earlier", b"earlier")
        .unwrap();
    intent(&mut r);
    drop(r);
    let mut r = t.resume().unwrap();
    assert_eq!(
        r.put(Component::Navigation, "mmaps/earlier", b"earlier")
            .unwrap(),
        Put::AlreadyPresent
    );
    assert_eq!(complete(r).files, 2);
}
#[test]
fn retained_unsealed_files_must_all_be_reemitted_before_completion() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    r.put(Component::Navigation, "mmaps/b", b"b").unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    assert_eq!(
        r.finish_component(Component::Navigation),
        Err(Error::Incomplete)
    );
    assert_eq!(r.finish().unwrap_err(), Error::Incomplete);
    let mut r = t.resume().unwrap();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    r.put(Component::Navigation, "mmaps/b", b"b").unwrap();
    r.finish_component(Component::Navigation).unwrap();
    r.finish().unwrap();
}
#[test]
fn retained_unsealed_component_does_not_block_a_completed_component_seal() {
    let t = Temp::new();
    let context = Context {
        components: Component::Navigation.bit() | Component::Terrain.bit(),
        ..ctx()
    };
    let mut r = Run::create(&t.out(), context, &[], Default::default()).unwrap();
    r.put(Component::Terrain, "maps/a", b"a").unwrap();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    drop(r);
    let mut r = Run::resume(&t.out(), context, &[], Default::default()).unwrap();
    r.put(Component::Terrain, "maps/a", b"a").unwrap();
    r.finish_component(Component::Terrain).unwrap();
    r.put(Component::Navigation, "mmaps/a", b"a").unwrap();
    r.finish_component(Component::Navigation).unwrap();
    assert_eq!(r.finish().unwrap().files, 2);
}
#[test]
fn replay_metadata_index_is_charged_and_released_after_coverage() {
    let t = Temp::new();
    let mut r = t.run();
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    drop(r);
    let mut r = t.resume().unwrap();
    assert!(r.inventory_limits().metadata_bytes < r.limits.metadata_bytes);
    r.put(Component::Navigation, &artifact().path, b"abc")
        .unwrap();
    assert_eq!(r.inventory_limits().metadata_bytes, r.limits.metadata_bytes);
    complete(r);
}
