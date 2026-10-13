// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: synthetic native fixed-NTFS package namespace cases.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "forever-package-ns-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn output(&self) -> PathBuf {
        self.0.join("private")
    }
    fn root(&self) -> Root {
        Root::create(&self.output(), Limits::default()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn put(root: &mut Root, path: &str, bytes: &[u8]) -> Identity {
    root.with_new_file(path, |w| {
        w.write_all(bytes).map_err(|_| Error::Write)?;
        Ok::<_, Error>(())
    })
    .unwrap()
    .0
}
fn read(root: &Root, path: &str, expected: Option<Identity>) -> Result<Vec<u8>, Error> {
    root.with_read_file(path, expected, |r| {
        let mut b = Vec::new();
        r.read_to_end(&mut b).map_err(|_| Error::Read)?;
        Ok(b)
    })
    .map(|(_, b)| b)
}
#[test]
fn names_preserve_all_resident_component_spelling_and_control_lengths() {
    for name in [
        "enUS/SpellName.db2",
        "vmaps/GameObjectModels.dtree",
        "mmaps/0001.mmap",
        ".forever-package-lock",
        ".state/sealed/.collision.part",
        "a b/Case+Name(1).DB2",
    ] {
        validate_path(name, Limits::default()).unwrap();
    }
    let receipt = format!("{}.json", "a".repeat(64));
    assert_eq!(receipt.len(), 69);
    validate_path(&receipt, Limits::default()).unwrap();
    let pending = format!("{receipt}.pending");
    assert_eq!(pending.len(), 77);
    validate_path(&pending, Limits::default()).unwrap();
    let pending75 = format!(".{}.pending", "a".repeat(66));
    assert_eq!(pending75.len(), 75);
    validate_path(&pending75, Limits::default()).unwrap();
    validate_path(&"a".repeat(255), Limits::default()).unwrap();
    for name in [
        ".git", ".GIT/x", "a/../x", "con.txt", "CON .db2", "aux", "com1.x", "LPT9", "x.", "x ",
        "a\\b", "a:b", "a*", "a?", "a|b", "a\"b", "a<b", "a>b", "a\0", "español",
    ] {
        assert!(validate_path(name, Limits::default()).is_err(), "{name:?}");
    }
    assert!(validate_path(&"x".repeat(256), Limits::default()).is_err());
    assert!(validate_path(&vec!["x"; 17].join("/"), Limits::default()).is_err());
    assert!(validate_path(&vec!["x"; 16].join("/"), Limits::default()).is_ok());
}
#[test]
fn paths_and_downward_limits_are_bounded_before_io() {
    let limits = Limits {
        max_handles: MAX_HANDLES + 1,
        ..Limits::default()
    };
    assert_eq!(limits.validate().unwrap_err(), Error::Limit);
    let limits = Limits {
        max_entries: MAX_ENTRIES + 1,
        ..Limits::default()
    };
    assert_eq!(limits.validate().unwrap_err(), Error::Limit);
    let limits = Limits {
        max_metadata_bytes: 0,
        ..Limits::default()
    };
    assert_eq!(limits.validate().unwrap_err(), Error::Limit);
    assert!(
        validate_path(
            &format!("{}/{}/x", "a".repeat(255), "b".repeat(255)),
            Limits::default()
        )
        .is_err()
    );
}
#[test]
fn create_reopen_same_lease_complete_read_and_inspection() {
    let t = Temp::new();
    let mut root = t.root();
    let lock = root.lock(true).unwrap();
    root.create_dirs("dbc/enUS").unwrap();
    let identity = put(&mut root, "dbc/enUS/SpellName.db2", b"synthetic bytes");
    let entries = root.observe().unwrap();
    root.verify_membership(&entries).unwrap();
    lock.guard().unwrap();
    drop(lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let lock = root.lock(false).unwrap();
    root.observe().unwrap();
    let (found, (hash_bytes, inspection)) = root
        .with_read_file("dbc/enUS/SpellName.db2", Some(identity), |r| {
            let mut bytes = Vec::new();
            r.read_to_end(&mut bytes).map_err(|_| Error::Read)?;
            r.seek(SeekFrom::Start(0)).map_err(|_| Error::Read)?;
            let mut first = [0; 9];
            r.read_exact(&mut first).map_err(|_| Error::Read)?;
            Ok::<_, Error>((bytes, first))
        })
        .unwrap();
    assert_eq!(found, identity);
    assert_eq!(hash_bytes, b"synthetic bytes");
    assert_eq!(&inspection, b"synthetic");
    lock.guard().unwrap();
}
#[test]
fn printable_names_and_long_receipts_roundtrip_native() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    root.create_dirs("enUS").unwrap();
    for name in [
        "Spell.db2".to_owned(),
        "GameObjectModels.dtree".to_owned(),
        format!("{}.json", "a".repeat(64)),
        format!(".{}.pending", "b".repeat(66)),
        "c".repeat(255),
        "A + (x).db2".to_owned(),
    ] {
        let path = format!("enUS/{name}");
        let identity = put(&mut root, &path, b"x");
        assert_eq!(read(&root, &path, Some(identity)).unwrap(), b"x");
    }
    root.observe().unwrap();
}
#[test]
fn file_count_does_not_retain_all_handles_or_v1_clamp() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let base = root.stats().retained_handles;
    for i in 0..160 {
        put(&mut root, &format!("File{i:04}.db2"), b"x");
        assert_eq!(root.stats().retained_handles, base);
    }
    assert_eq!(root.observe().unwrap().len(), 161);
    assert_eq!(root.stats().retained_handles, base);
    assert_eq!(root.stats().bytes, 160);
}
#[test]
fn rename_is_identity_checked_and_never_replaces_destination() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    root.create_dirs("a/b").unwrap();
    let first = put(&mut root, "a.part", b"one");
    let other = put(&mut root, "a/b/final.db2", b"two");
    assert_eq!(
        root.rename_no_replace("a.part", "a/b/final.db2", first),
        Err(Error::Exists)
    );
    root.guard().unwrap();
    assert_eq!(read(&root, "a/b/final.db2", Some(other)).unwrap(), b"two");
    let after = root
        .rename_no_replace("a.part", "a/b/Spell.db2", first)
        .unwrap();
    assert!(after.same_object(first));
    assert_eq!(root.file_identity("a.part").unwrap(), None);
    assert_eq!(read(&root, "a/b/Spell.db2", Some(after)).unwrap(), b"one");
    root.observe().unwrap();
}
#[test]
fn identity_checked_remove_and_reopen_adoption() {
    let t = Temp::new();
    let mut root = t.root();
    let lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x.part", b"one");
    root.observe().unwrap();
    drop(lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let _lock = root.lock(true).unwrap();
    assert_eq!(root.remove_same("x.part", identity), Err(Error::Membership));
    root.observe().unwrap();
    root.remove_same("x.part", identity).unwrap();
    assert_eq!(root.file_identity("x.part").unwrap(), None);
    assert_eq!(root.stats().bytes, 0);
    root.observe().unwrap();
}
#[test]
fn wrong_expected_identity_poison_is_permanent() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "a", b"a");
    let wrong = put(&mut root, "b", b"b");
    assert_eq!(root.remove_same("a", wrong), Err(Error::Changed));
    assert_eq!(root.guard(), Err(Error::Changed));
    assert!(t.output().join("a").exists());
    assert_ne!(identity, wrong);
}
#[test]
fn case_alias_lookup_and_case_alias_directory_reject() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    root.create_dirs("enUS").unwrap();
    put(&mut root, "enUS/Spell.db2", b"x");
    assert_eq!(root.file_identity("enUS/spell.db2"), Err(Error::Alias));
    assert_eq!(root.guard(), Err(Error::Changed));
    drop(_lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let _lock = root.lock(true).unwrap();
    root.observe().unwrap();
    assert_eq!(root.create_dirs("enus"), Err(Error::Alias));
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn duplicate_create_does_not_poison_or_overwrite() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"x");
    assert_eq!(
        root.with_new_file("x", |_| Ok::<_, Error>(())),
        Err(Error::Exists)
    );
    root.guard().unwrap();
    assert_eq!(read(&root, "x", Some(identity)).unwrap(), b"x");
}
#[test]
fn complete_membership_detects_foreign_entry_and_missing_entry() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"x");
    let entries = root.observe().unwrap();
    root.remove_same("x", identity).unwrap();
    assert_eq!(root.verify_membership(&entries), Err(Error::Membership));
    assert_eq!(root.guard(), Err(Error::Changed));
    drop(_lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let _lock = root.lock(true).unwrap();
    let entries = root.observe().unwrap();
    put(&mut root, "extra", b"y");
    assert_eq!(root.verify_membership(&entries), Err(Error::Membership));
}
#[test]
fn unprivate_existing_root_and_foreign_entry_reject() {
    let t = Temp::new();
    fs::create_dir(t.output()).unwrap();
    assert!(matches!(
        Root::open(&t.output(), Limits::default()),
        Err(Error::Privacy)
    ));
    let other = Temp::new();
    let mut root = other.root();
    let _lock = root.lock(true).unwrap();
    fs::write(other.output().join("foreign"), b"x").unwrap();
    assert!(root.observe().is_err());
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn protected_owner_dacl_and_security_change_poison() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"x");
    assert_eq!(
        ffi::test_security_control(&root.root.file).unwrap() & 0x1004,
        0x1004
    );
    let file = OpenOptions::new()
        .access_mode(0x0006_0000)
        .share_mode(7)
        .open(t.output().join("x"))
        .unwrap();
    ffi::test_dacl(&file, &root.sid, 0x0012_0089).unwrap();
    assert_eq!(root.file_identity("x"), Err(Error::Privacy));
    ffi::test_dacl(&file, &root.sid, 0x001f_01ff).unwrap();
    drop(file);
    assert_eq!(root.guard(), Err(Error::Changed));
    assert_eq!(identity.size, 1);
}
#[test]
fn hardlinks_reject_and_poison_after_file_lease_closes() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    put(&mut root, "x", b"x");
    fs::hard_link(t.output().join("x"), t.0.join("alias")).unwrap();
    assert_eq!(root.observe(), Err(Error::HardLinks));
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn package_shared_readers_exclude_writer_and_writer_excludes_all() {
    let t = Temp::new();
    let mut root = t.root();
    let exclusive = root.lock(true).unwrap();
    let mut other = Root::open(&t.output(), Limits::default()).unwrap();
    assert!(matches!(other.lock(true), Err(Error::Busy)));
    assert!(matches!(other.lock(false), Err(Error::Busy)));
    root.guard().unwrap();
    drop(exclusive);
    let shared = root.lock(false).unwrap();
    let shared2 = other.lock(false).unwrap();
    assert_eq!(root.create_dirs("blocked"), Err(Error::Busy));
    let mut third = Root::open(&t.output(), Limits::default()).unwrap();
    assert!(matches!(third.lock(true), Err(Error::Busy)));
    shared.guard().unwrap();
    shared2.guard().unwrap();
    drop(shared);
    drop(shared2);
    let lock = third.lock(true).unwrap();
    lock.guard().unwrap();
}
#[test]
fn dropped_lock_cannot_authorize_mutation() {
    let t = Temp::new();
    let mut root = t.root();
    let lock = root.lock(true).unwrap();
    drop(lock);
    assert_eq!(root.create_dirs("x"), Err(Error::Busy));
    assert_eq!(
        root.with_new_file("x", |_| Ok::<_, Error>(())),
        Err(Error::Busy)
    );
    root.guard().unwrap();
}
#[test]
fn restart_empty_root_and_lock_only_initialization() {
    let t = Temp::new();
    let root = t.root();
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let lock = root.lock(true).unwrap();
    assert_eq!(root.observe().unwrap().len(), 1);
    drop(lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let _lock = root.lock(true).unwrap();
    root.observe().unwrap();
    put(&mut root, "new", b"new");
    root.observe().unwrap();
}
#[test]
fn streaming_write_file_and_total_budgets_retain_partial_failure() {
    let t = Temp::new();
    let mut root = Root::create(
        &t.output(),
        Limits {
            max_file_bytes: 3,
            max_total_bytes: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    let _lock = root.lock(true).unwrap();
    assert_eq!(
        root.with_new_file("x", |w| {
            w.write_all(b"four").map_err(|_| Error::Write)?;
            Ok::<_, Error>(())
        }),
        Err(Error::Write)
    );
    assert_eq!(root.guard(), Err(Error::Changed));
    assert!(t.output().join("x").exists());
}
#[test]
fn entry_and_directory_budgets_do_not_clamp_to_v1() {
    let t = Temp::new();
    let mut root = Root::create(
        &t.output(),
        Limits {
            max_entries: 2,
            max_directories: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    let _lock = root.lock(true).unwrap();
    assert_eq!(root.create_dirs("dir"), Err(Error::Limit));
    root.guard().unwrap();
    put(&mut root, "x", b"x");
    assert_eq!(
        root.with_new_file("y", |_| Ok::<_, Error>(())),
        Err(Error::Limit)
    );
    root.guard().unwrap();
}
#[test]
fn metadata_and_work_budgets_bound_observation() {
    let t = Temp::new();
    let mut root = t.root();
    let lock = root.lock(true).unwrap();
    put(&mut root, "x", b"x");
    drop(lock);
    drop(root);
    assert!(matches!(
        Root::open(
            &t.output(),
            Limits {
                max_metadata_bytes: 1,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        Root::open(
            &t.output(),
            Limits {
                max_work: 1,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    ));
}
#[test]
fn complete_read_required_and_failed_callback_poison() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    put(&mut root, "x", b"xy");
    assert_eq!(
        root.with_read_file("x", None, |r| {
            let mut b = [0];
            r.read_exact(&mut b).map_err(|_| Error::Read)?;
            Ok::<_, Error>(())
        }),
        Err(Error::Read)
    );
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn changed_file_metadata_is_not_adopted_as_expected_identity() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"x");
    fs::write(t.output().join("x"), b"longer").unwrap();
    assert_eq!(read(&root, "x", Some(identity)), Err(Error::Changed));
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn impersonation_is_rejected_without_filesystem_mutation() {
    let t = Temp::new();
    let root = t.root();
    let impersonation = ffi::test_impersonate().unwrap();
    assert_eq!(root.guard(), Err(Error::Impersonation));
    drop(impersonation);
    assert_eq!(root.guard(), Err(Error::Changed));
}
#[test]
fn protected_sources_and_git_markers_reject_before_creation() {
    let t = Temp::new();
    let client = t.0.join("Client");
    fs::create_dir(&client).unwrap();
    let inside = client.join("private");
    assert_eq!(protect_output(&inside, &[&client], &[]), Err(Error::Path));
    assert!(!inside.exists());
    let ordinary = t.0.join("ordinary");
    fs::create_dir(&ordinary).unwrap();
    fs::write(ordinary.join(".git"), b"gitdir: somewhere").unwrap();
    assert_eq!(
        protect_output(&ordinary.join("private"), &[], &[]),
        Err(Error::Path)
    );
    assert!(!ordinary.join("private").exists());
    let key = t.0.join("key.txt");
    fs::write(&key, b"synthetic").unwrap();
    let lease = ReadFile::open(
        &key,
        PathLimits::default(),
        super::super::LinkPolicy::Reject,
    )
    .unwrap();
    assert_eq!(protect_output(&t.0, &[], &[&lease]), Err(Error::Path));
    lease.verify_unchanged().unwrap();
}
#[test]
fn retained_root_prevents_rename_and_compatible_readers_work() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"x");
    assert!(fs::rename(t.output(), t.0.join("moved")).is_err());
    let file = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(t.output().join("x"))
        .unwrap();
    assert_eq!(file.metadata().unwrap().len(), 1);
    drop(file);
    assert_eq!(read(&root, "x", Some(identity)).unwrap(), b"x");
    root.guard().unwrap();
}

#[test]
fn caller_retained_metadata_is_admitted_before_observation_allocations() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    put(&mut root, "x", b"x");
    assert_eq!(root.observe_charged(MAX_METADATA), Err(Error::Limit));
    root.guard().unwrap();
    let expected = root.observe().unwrap();
    assert_eq!(
        root.verify_membership_charged(&expected, MAX_METADATA),
        Err(Error::Limit)
    );
    root.guard().unwrap();
    assert!(root.retained_metadata_bytes().unwrap() < MAX_METADATA);
}
#[test]
fn ordinary_drive_aliases_cannot_put_output_inside_or_around_protected_sources() {
    struct Alias(String);
    impl Drop for Alias {
        fn drop(&mut self) {
            let _ = std::process::Command::new("subst")
                .args([&self.0, "/D"])
                .status();
        }
    }
    let t = Temp::new();
    let client = t.0.join("Client");
    fs::create_dir(&client).unwrap();
    let drive = (b'R'..=b'Z')
        .map(|v| format!("{}:", char::from(v)))
        .find(|v| !Path::new(&format!("{v}\\")).exists())
        .expect("synthetic alias requires a free drive letter");
    let result = std::process::Command::new("subst")
        .arg(&drive)
        .arg(&t.0)
        .status()
        .unwrap();
    assert!(result.success(), "synthetic drive alias setup failed");
    let alias = Alias(drive);
    let output = PathBuf::from(format!("{}\\Client\\private", alias.0));
    assert_eq!(protect_output(&output, &[&client], &[]), Err(Error::Path));
    assert!(!client.join("private").exists());
    let containing = t.0.join("Containing");
    fs::create_dir(&containing).unwrap();
    let key = containing.join("key.txt");
    fs::write(&key, b"synthetic").unwrap();
    let source = ReadFile::open(
        &key,
        PathLimits::default(),
        super::super::LinkPolicy::Reject,
    )
    .unwrap();
    assert_eq!(
        protect_output(
            &PathBuf::from(format!("{}\\Containing", alias.0)),
            &[],
            &[&source]
        ),
        Err(Error::Path)
    );
    source.verify_unchanged().unwrap();
}

#[test]
fn ordinary_directory_junctions_reject_at_root_read_and_observation() {
    use std::os::windows::process::CommandExt;
    struct Junction(PathBuf);
    impl Junction {
        fn new(path: PathBuf, target: &Path) -> Self {
            // Junction creation uses no symbolic-link or administrator privilege.
            // Paths enter cmd as environment values inside fixed quoted arguments.
            let status = std::process::Command::new("cmd")
                .args(["/D", "/V:OFF", "/C"])
                .raw_arg(r#"mklink /J "%FOREVER_NS_JUNCTION%" "%FOREVER_NS_TARGET%" >nul"#)
                .env("FOREVER_NS_JUNCTION", &path)
                .env("FOREVER_NS_TARGET", target)
                .status()
                .unwrap();
            assert!(
                status.success(),
                "synthetic directory junction setup failed"
            );
            Self(path)
        }
    }
    impl Drop for Junction {
        fn drop(&mut self) {
            // Remove the junction itself, never recursively traverse its target.
            let _ = fs::remove_dir(&self.0);
        }
    }
    let t = Temp::new();
    let target = t.0.join("Target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("x"), b"synthetic target").unwrap();
    let root_link = Junction::new(t.0.join("RootJunction"), &target);
    assert!(matches!(
        Root::open(&root_link.0, Limits::default()),
        Err(Error::Reparse)
    ));
    let mut root = t.root();
    let lock = root.lock(true).unwrap();
    let child_link = Junction::new(t.output().join("ChildJunction"), &target);
    assert_eq!(read(&root, "ChildJunction/x", None), Err(Error::Reparse));
    assert_eq!(root.guard(), Err(Error::Changed));
    drop(lock);
    drop(root);
    let mut root = Root::open(&t.output(), Limits::default()).unwrap();
    let lock = root.lock(true).unwrap();
    assert_eq!(root.observe(), Err(Error::Reparse));
    assert_eq!(root.guard(), Err(Error::Changed));
    drop(lock);
    drop(root);
    drop(child_link);
    drop(root_link);
    assert_eq!(fs::read(target.join("x")).unwrap(), b"synthetic target");
}

#[test]
fn callback_gate_rejects_nested_reads_before_io_and_releases_on_all_exits() {
    let t = Temp::new();
    let depth = AbsolutePath::parse(t.output().to_str().unwrap(), PathLimits::default())
        .unwrap()
        .ancestor_count();
    // Root ancestors + root + lock + file + temporary check handle: one lease
    // fits exactly, while a second callback-held file would exceed this cap.
    let mut root = Root::create(
        &t.output(),
        Limits {
            max_handles: depth + 4,
            ..Limits::default()
        },
    )
    .unwrap();
    let lock = root.lock(true).unwrap();
    let identity = put(&mut root, "x", b"synthetic");
    let (_, bytes) = root
        .with_read_file("x", Some(identity), |r| {
            let before = root.stats().work;
            assert_eq!(
                root.with_read_file("x", Some(identity), |_| Ok::<_, Error>(())),
                Err(Error::Busy)
            );
            assert_eq!(root.file_identity("x"), Err(Error::Busy));
            assert_eq!(root.observe(), Err(Error::Busy));
            assert_eq!(root.guard(), Err(Error::Busy));
            assert_eq!(root.stats().work, before);
            let mut bytes = Vec::new();
            r.read_to_end(&mut bytes).map_err(|_| Error::Read)?;
            Ok::<_, Error>(bytes)
        })
        .unwrap();
    assert_eq!(bytes, b"synthetic");
    assert!(!root.callback_active.get());
    root.guard().unwrap();
    // Unwind releases the gate and all temporary file/path leases too.
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(Identity, ()), Error> =
            root.with_read_file("x", Some(identity), |_| panic!("synthetic callback unwind"));
    }));
    assert!(panic.is_err());
    assert!(!root.callback_active.get());
    root.guard().unwrap();
    assert_eq!(read(&root, "x", Some(identity)).unwrap(), b"synthetic");
    // A returned callback error still poisons the root's integrity capability,
    // but must not leave its independent reentrancy gate permanently engaged.
    assert_eq!(
        root.with_read_file("x", Some(identity), |_| Err::<(), Error>(Error::Read)),
        Err(Error::Read)
    );
    assert!(!root.callback_active.get());
    assert_eq!(root.guard(), Err(Error::Changed));
    drop(lock);
}

#[test]
fn rename_short_and_maximum_names_same_and_cross_directory() {
    let t = Temp::new();
    let mut root = t.root();
    let _lock = root.lock(true).unwrap();
    root.create_dirs("dir").unwrap();
    let destinations = [
        "a".to_owned(),
        "bb".to_owned(),
        "ccc".to_owned(),
        "dddd".to_owned(),
        "dir/a".to_owned(),
        "dir/bb".to_owned(),
        "dir/ccc".to_owned(),
        "dir/dddd".to_owned(),
        format!("dir/{}", "x".repeat(252)),
        format!("dir/{}", "y".repeat(255)),
    ];
    for (i, destination) in destinations.iter().enumerate() {
        let source = format!("pending-{i}.part");
        let before = put(&mut root, &source, b"synthetic rename");
        let after = root
            .rename_no_replace(&source, destination, before)
            .unwrap();
        assert!(after.same_object(before));
        assert_eq!(root.file_identity(&source).unwrap(), None);
        assert_eq!(
            read(&root, destination, Some(after)).unwrap(),
            b"synthetic rename"
        );
    }
    root.observe().unwrap();
    root.guard().unwrap();
}
