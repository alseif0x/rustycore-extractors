// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: synthetic native Windows NTFS capability tests.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "forever-output-{}-{}-{}",
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
    fn parent(&self) -> ReadDirectory {
        ReadDirectory::open(&self.0, crate::path::Limits::default()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn private_create_write_flush_and_relative_rename() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    let top = root.root();
    let dir = root.create_directory(top, "stage").unwrap();
    let file = root.create_file(top, "input.tmp").unwrap();
    root.append(file, b"one").unwrap();
    root.flush(file).unwrap();
    root.rename(file, dir, "final.bin").unwrap();
    root.append(file, b"two").unwrap();
    root.verify_unchanged().unwrap();
    assert!(!tmp.0.join("private/input.tmp").exists());
    // A compatible owner-SID reader is permitted; write/delete exclusion is not
    // an exclusion of all reads, and this API makes no such claim.
    let mut read = OpenOptions::new()
        .read(true)
        .share_mode(7)
        .open(tmp.0.join("private/stage/final.bin"))
        .unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut read, &mut bytes).unwrap();
    assert_eq!(bytes, b"onetwo");
    drop(read);
    assert_eq!(root.stats().objects, 3);
    assert_eq!(root.stats().retained_handles, 5);
    assert_eq!(root.stats().charged_bytes, 6);
    for e in &root.entries {
        ffi::private(e.file(), &root.sid).unwrap();
    }
    drop(root);
    assert_eq!(
        fs::read(tmp.0.join("private/stage/final.bin")).unwrap(),
        b"onetwo"
    );
}
#[test]
fn competing_file_write_delete_and_rename_are_denied() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    let file = root.create_file(root.root(), "file").unwrap();
    root.append(file, b"data").unwrap();
    let path = tmp.0.join("private/file");
    assert!(
        OpenOptions::new()
            .write(true)
            .share_mode(7)
            .open(&path)
            .is_err()
    );
    assert!(
        OpenOptions::new()
            .append(true)
            .share_mode(7)
            .open(&path)
            .is_err()
    );
    assert!(fs::remove_file(&path).is_err());
    assert!(fs::rename(&path, tmp.0.join("private/else")).is_err());
    root.verify_unchanged().unwrap();
}
#[test]
fn parent_and_live_directory_deletion_are_denied() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    assert!(fs::rename(&tmp.0, tmp.0.with_extension("moved")).is_err());
    assert!(fs::rename(tmp.0.join("private"), tmp.0.join("else")).is_err());
    assert!(fs::remove_dir(tmp.0.join("private")).is_err());
    root.verify_unchanged().unwrap();
}
#[test]
fn rename_never_overwrites_known_destination() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    let a = root.create_file(root.root(), "a").unwrap();
    let b = root.create_file(root.root(), "b").unwrap();
    root.append(a, b"a").unwrap();
    root.append(b, b"b").unwrap();
    assert_eq!(root.rename(a, root.root(), "b"), Err(Error::Exists));
    root.verify_unchanged().unwrap();
}
#[test]
fn native_collision_retains_objects_and_poisons_root() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    let a = root.create_file(root.root(), "a").unwrap();
    root.append(a, b"original").unwrap();
    fs::write(tmp.0.join("private/external"), b"retained").unwrap();
    assert_eq!(root.rename(a, root.root(), "external"), Err(Error::Exists));
    assert_eq!(root.verify_unchanged(), Err(Error::Changed));
    drop(root);
    assert_eq!(
        fs::read(tmp.0.join("private/external")).unwrap(),
        b"retained"
    );
    assert_eq!(fs::read(tmp.0.join("private/a")).unwrap(), b"original");
}
#[test]
fn dacl_changed_then_restored_stays_poisoned() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    let mutation = OpenOptions::new()
        // Protected-DACL updates may query the existing descriptor as well as
        // setting it. Keep these metadata rights separate from data writes.
        .access_mode(0x0002_0000 | 0x0004_0000)
        .share_mode(7)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(tmp.0.join("private"))
        .unwrap();
    let original_control = ffi::test_security_control(root.entries[0].file()).unwrap();
    ffi::test_dacl(&mutation, &root.sid, 0x0012_0089).unwrap();
    assert!(root.verify_unchanged().is_err());
    ffi::test_dacl(&mutation, &root.sid, 0x001f_01ff).unwrap();
    assert_eq!(
        ffi::test_security_control(root.entries[0].file()).unwrap(),
        original_control,
        "restored descriptor control must match exactly"
    );
    ffi::private(root.entries[0].file(), &root.sid).unwrap();
    assert_eq!(root.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn tokens_cannot_cross_private_roots() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut a = PrivateRoot::create(&parent, "a", Limits::default()).unwrap();
    let mut b = PrivateRoot::create(&parent, "b", Limits::default()).unwrap();
    let file = a.create_file(a.root(), "file").unwrap();
    assert_eq!(b.append(file, b"bad"), Err(Error::Token));
    assert_eq!(b.create_file(a.root(), "bad"), Err(Error::Token));
    assert_eq!(a.rename(file, b.root(), "bad"), Err(Error::Token));
    a.verify_unchanged().unwrap();
    b.verify_unchanged().unwrap();
}
#[test]
fn budgets_reject_before_mutation_and_leave_valid_root() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let limits = Limits {
        max_objects: 3,
        max_depth: 1,
        max_file_bytes: 3,
        max_total_bytes: 4,
        max_write_bytes: 2,
        ..Limits::default()
    };
    let mut root = PrivateRoot::create(&parent, "private", limits).unwrap();
    let file = root.create_file(root.root(), "file").unwrap();
    root.append(file, b"ab").unwrap();
    assert_eq!(root.append(file, b"abc"), Err(Error::Limit));
    assert_eq!(root.append(file, b"cd"), Err(Error::Limit));
    let dir = root.create_directory(root.root(), "dir").unwrap();
    assert_eq!(root.create_file(dir, "deep"), Err(Error::Limit));
    assert_eq!(root.create_file(root.root(), "extra"), Err(Error::Limit));
    root.verify_unchanged().unwrap();
    assert_eq!(root.stats().charged_bytes, 2);
}
#[test]
fn operation_budget_counts_mutation_admission_only() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(
        &parent,
        "private",
        Limits {
            max_operations: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    let file = root.create_file(root.root(), "empty").unwrap();
    assert_eq!(root.append(file, b"x"), Err(Error::Limit));
    root.verify_unchanged().unwrap();
    root.verify_unchanged().unwrap();
    assert_eq!(root.stats().operations, 2);
}
#[test]
fn conservative_names_and_downward_limits() {
    for name in [
        "", ".", "..", ".git", "CON", "con", "com1.ext", "lpt9", "a:", "a/b", "a\\b", "a ", "a.",
        "UPPER", "a~1", "é",
    ] {
        assert_eq!(Name::parse(name, 64).err(), Some(Error::Path), "{name}");
    }
    assert!(Name::parse("normal-file_1.bin", 64).is_ok());
    assert!(Name::parse(&"a".repeat(64), 64).is_ok());
    assert_eq!(Name::parse(&"a".repeat(65), 64).err(), Some(Error::Path));
    assert_eq!(
        Limits {
            max_objects: 129,
            ..Limits::default()
        }
        .validate()
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Limits {
            max_operations: 16_385,
            ..Limits::default()
        }
        .validate()
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(path_units(32758, 1), Ok(32760));
    assert_eq!(path_units(32759, 1), Err(Error::Limit));
    assert_eq!(path_units(usize::MAX, 1), Err(Error::Limit));
}
#[test]
fn existing_case_alias_root_is_never_adopted() {
    let tmp = Temp::new();
    fs::create_dir(tmp.0.join("PRIVATE")).unwrap();
    let parent = tmp.parent();
    assert!(matches!(
        PrivateRoot::create(&parent, "private", Limits::default()),
        Err(Error::Exists)
    ));
    assert_eq!(fs::read_dir(&tmp.0).unwrap().count(), 1);
}
#[test]
fn unknown_child_creation_is_observation_not_inventory_closure() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    fs::write(tmp.0.join("private/untracked"), b"owner-can-add").unwrap();
    root.verify_unchanged().unwrap();
    assert_eq!(root.stats().objects, 1);
}
pub(super) fn privileged_reparse_root_is_never_adopted() {
    let tmp = Temp::new();
    fs::create_dir(tmp.0.join("target")).unwrap();
    std::os::windows::fs::symlink_dir(tmp.0.join("target"), tmp.0.join("private")).unwrap();
    let parent = tmp.parent();
    assert!(matches!(
        PrivateRoot::create(&parent, "private", Limits::default()),
        Err(Error::Exists)
    ));
}

#[test]
fn independent_powershell_dacl_oracle() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    root.create_file(root.root(), "file").unwrap();
    // Independent managed ACL reader. Only static exit status is exposed;
    // fixture paths are passed as a child-only environment value, never code.
    let script = r#"$ErrorActionPreference='Stop'; $stage=1; try {
      $sid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User;
      $stage=2; $acl=Get-Acl -LiteralPath $env:RCE_OUTPUT_ACL_PATH;
      $stage=3;
      if ($acl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) {exit 81};
      $stage=4; if (-not $acl.AreAccessRulesProtected) {exit 82};
      $stage=5; $inherited=@($acl.GetAccessRules($false,$true,[System.Security.Principal.SecurityIdentifier]));
      if ($inherited.Count -ne 0) {exit 86};
      $stage=6; $rules=@($acl.GetAccessRules($true,$false,[System.Security.Principal.SecurityIdentifier]));
      if ($rules.Count -ne 1) {exit 83};
      $stage=7; $rule=$rules[0];
      if ($rule.IdentityReference.Value -ne $sid.Value -or $rule.AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rule.FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl -or $rule.InheritanceFlags -ne [System.Security.AccessControl.InheritanceFlags]::None -or $rule.PropagationFlags -ne [System.Security.AccessControl.PropagationFlags]::None -or $rule.IsInherited) {exit 84};
      exit 0;
    } catch {
      $exception=$_.Exception;
      for ($i=0; $i -lt 8 -and $null -ne $exception.InnerException; $i++) { $exception=$exception.InnerException };
      $native=-1; if ($exception -is [System.ComponentModel.Win32Exception]) { $native=$exception.NativeErrorCode };
      [Console]::Out.WriteLine(('{0} {1}' -f [long]$exception.HResult,[long]$native));
      exit (90+$stage)
    }"#;
    // Probe while held only to retain numeric diagnostics about a path-reader
    // acquisition failure. No sharing cause is inferred from its exit status.
    let live = acl_oracle(script, &tmp.0.join("private"));
    if !live.0.success() {
        eprintln!(
            "live ACL path probe: static_status={:?}, numeric_exception={:?}",
            live.0.code(),
            live.1
        );
    }
    root.verify_unchanged().unwrap();
    drop(root);
    drop(parent);
    // Independent oracle scope: persisted exact owner/DACL after all leases are
    // closed. The other tests exercise compatible owner reads while held.
    for (fixture, path) in [tmp.0.join("private"), tmp.0.join("private/file")]
        .into_iter()
        .enumerate()
    {
        let (status, numeric) = acl_oracle(script, &path);
        assert!(
            status.success(),
            "persisted ACL oracle rejected private DACL: fixture={fixture}, static_status={:?}, numeric_exception={numeric:?}",
            status.code()
        );
    }
}

// The child script emits at most two decimal exception codes on failure. Never
// format raw child bytes: even unexpected provider output remains redacted.
fn acl_oracle(
    script: &str,
    path: &std::path::Path,
) -> (std::process::ExitStatus, Option<[i64; 2]>) {
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("RCE_OUTPUT_ACL_PATH", path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("native PowerShell ACL oracle must exist");
    let numeric = if output.stdout.len() <= 128 {
        std::str::from_utf8(&output.stdout).ok().and_then(|text| {
            let mut words = text.split_ascii_whitespace();
            let first = words.next()?.parse::<i64>().ok()?;
            let second = words.next()?.parse::<i64>().ok()?;
            if words.next().is_some() {
                None
            } else {
                Some([first, second])
            }
        })
    } else {
        None
    };
    (output.status, numeric)
}

#[test]
fn thread_impersonation_rejects_before_create_and_poisons_existing_root() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", Limits::default()).unwrap();
    {
        let _guard = ffi::test_impersonate().unwrap();
        assert!(matches!(
            PrivateRoot::create(&parent, "blocked", Limits::default()),
            Err(Error::Impersonation)
        ));
        assert_eq!(root.verify_unchanged(), Err(Error::Impersonation));
    }
    assert!(!tmp.0.join("blocked").exists());
    assert_eq!(root.verify_unchanged(), Err(Error::Changed));
}
