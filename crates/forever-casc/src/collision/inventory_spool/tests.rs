// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original typed inventory synthetic/guard/resource cases.
use super::*;
use std::{
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Paths {
    base: PathBuf,
    source: PathBuf,
    path: PathBuf,
}
impl Paths {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let source = base.join("source");
        let parent = base.join("private");
        for p in [&source, &parent] {
            fs::DirBuilder::new().mode(0o700).create(p).unwrap();
        }
        Self {
            path: parent.join("namespace"),
            base,
            source,
        }
    }
    fn create(&self, limits: spool::Limits) -> spool::Spool {
        spool::Spool::create(
            &self.path,
            spool::ProtectedInputs::capture(&self.source, &[]).unwrap(),
            limits,
        )
        .unwrap()
    }
}
impl Drop for Paths {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
use std::{fs, os::unix::fs::PermissionsExt};
fn replace_source(path: &std::path::Path) {
    fs::rename(path, path.with_extension("old")).unwrap();
    fs::DirBuilder::new().mode(0o700).create(path).unwrap();
}
fn inventory(p: &Paths) -> Inventory {
    Inventory::new(p.create(spool::Limits::default()), Limits::default()).unwrap()
}
fn tile() -> Tile {
    Tile {
        map_id: 70205,
        x: 24,
        y: 39,
    }
}
fn word(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn row(flags: u8, id: u32) -> Vec<u8> {
    let mut b = vec![flags, 7];
    word(&mut b, id);
    for x in [1f32, 2., 3., 0., 0., 0., 1.] {
        word(&mut b, x.to_bits());
    }
    if flags & 1 != 0 {
        for x in [0f32, 0., 0., 1., 1., 1.] {
            word(&mut b, x.to_bits());
        }
    }
    word(&mut b, 5);
    b.extend_from_slice(b"x.m2x");
    b
}
fn wdt() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(b"REVM");
    word(&mut b, 4);
    word(&mut b, 18);
    b
}
#[test]
fn complete_wdt_binding_reload_and_private_header() {
    let p = Paths::new();
    let mut s = inventory(&p);
    let b = wdt();
    s.insert_wdt(9, 123, &b).unwrap();
    assert_eq!(
        s.with_wdt(9, 123, 0, |w| Ok::<_, ()>(w.stats().global_records))
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read(p.path.join("00000000.payload")).unwrap(),
        [
            Entry::header_for(Key::Wdt(9), 123, 0, b.len()).as_slice(),
            &b
        ]
        .concat()
    );
    assert_eq!(s.stats().unwrap().repository.reloads, 1);
    assert!(!s.failed());
}
#[test]
fn cache_preserves_all_bits_except_parent_and_matches_pure_replay() {
    for flags in [0, 1, 2, 3, 4, 6, 7] {
        let p = Paths::new();
        let mut s = inventory(&p);
        let b = [row(flags, 8), row(flags, 9)].concat();
        let pure = cache::Cached::from_bytes(&b, cache::Limits::default()).unwrap();
        s.insert_own_tile(tile(), &b).unwrap();
        for destination in [tile().map_id, 1] {
            assert_eq!(
                s.replay_own_tile(tile(), destination).unwrap(),
                pure.replay(destination, tile().map_id, cache::Limits::default())
                    .unwrap()
            );
        }
        assert_eq!(s.stats().unwrap().entries, 1);
    }
}
#[test]
fn empty_cache_is_present_and_blocks_absent_interpretation() {
    let p = Paths::new();
    let mut s = inventory(&p);
    assert!(!s.contains_own_tile(tile()).unwrap());
    s.insert_own_tile(tile(), &[]).unwrap();
    assert!(s.contains_own_tile(tile()).unwrap());
    assert_eq!(s.replay_own_tile(tile(), 1).unwrap(), Vec::<u8>::new());
    assert!(!s.failed());
}
#[test]
fn out_of_order_keys_and_model_free_inventories_do_not_alias() {
    let p = Paths::new();
    let mut s = inventory(&p);
    let mut keys = Vec::new();
    for id in [7, 2, 99, 0] {
        let k = Tile {
            map_id: id,
            x: 63,
            y: 63,
        };
        s.insert_own_tile(k, &row(0, id)).unwrap();
        s.insert_wdt(id, id + 1, &wdt()).unwrap();
        keys.push(k);
    }
    for k in keys {
        assert_eq!(
            DirBin::parse(
                &s.replay_own_tile(k, k.map_id).unwrap(),
                placement::Limits::default()
            )
            .unwrap()
            .record_count(),
            1
        );
        assert_eq!(
            s.with_wdt(k.map_id, k.map_id + 1, 0, |w| Ok::<_, ()>(w.flags()))
                .unwrap(),
            0
        );
    }
}
#[test]
fn duplicate_wdt_is_terminal_with_first_file_retained() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    assert_eq!(s.insert_wdt(1, 3, &wdt()), Err(Error::Duplicate));
    assert!(s.failed());
    assert_eq!(s.verify_unchanged(), Err(Error::Duplicate));
    assert_eq!(s.stats().unwrap().entries, 1);
}
#[test]
fn duplicate_own_empty_cache_is_terminal() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_own_tile(tile(), &[]).unwrap();
    assert_eq!(s.insert_own_tile(tile(), &[]), Err(Error::Duplicate));
    assert!(s.failed());
}
#[test]
fn wrong_fdid_never_invokes_callback() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let e = s
        .with_wdt(1, 3, 0, |_| -> Result<(), ()> { panic!("must not invoke") })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Access(Error::Binding));
    assert!(s.failed());
    assert_eq!(s.stats().unwrap().repository.reloads, 0);
}
#[test]
fn malformed_wdt_never_writes_a_record() {
    for b in [b"bad".as_slice(), &[0; 7], b"REVM\x04\0\0\0\x12\0\0"] {
        let p = Paths::new();
        let mut s = inventory(&p);
        assert!(matches!(s.insert_wdt(1, 2, b), Err(Error::Wdt(_))));
        assert!(s.failed());
        assert_eq!(s.stats().unwrap().entries, 0);
        assert_eq!(s.stats().unwrap().repository.records, 0);
    }
}
#[test]
fn malformed_cache_never_writes_a_record() {
    for b in [b"bad".as_slice(), &row(0, 1)[..20]] {
        let p = Paths::new();
        let mut s = inventory(&p);
        assert!(matches!(s.insert_own_tile(tile(), b), Err(Error::Cache(_))));
        assert_eq!(s.stats().unwrap().repository.records, 0);
    }
}
#[test]
fn invalid_tile_and_zero_wdt_fdid_reject() {
    for x in [64, 255] {
        let p = Paths::new();
        let mut s = inventory(&p);
        assert_eq!(
            s.insert_own_tile(Tile { x, ..tile() }, &[]),
            Err(Error::Binding)
        );
    }
    let p = Paths::new();
    let mut s = inventory(&p);
    assert_eq!(s.insert_wdt(1, 0, &wdt()), Err(Error::Binding));
}
#[test]
fn missing_requested_cache_is_terminal() {
    let p = Paths::new();
    let mut s = inventory(&p);
    assert_eq!(s.replay_own_tile(tile(), 1), Err(Error::Missing));
    assert!(s.failed());
}
#[test]
fn callback_cause_is_retained_and_inventory_is_poisoned() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let e = s.with_wdt(1, 2, 0, |_| Err::<(), _>(42u8)).unwrap_err();
    assert_eq!(e.cause, Cause::Callback(42));
    assert_eq!(e.guard, None);
    assert_eq!(e.code(), Error::Callback.code());
    assert_eq!(s.verify_unchanged(), Err(Error::Callback));
}
#[test]
fn callback_cause_survives_independent_source_change() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let e = s
        .with_wdt(1, 2, 0, |_| {
            replace_source(&p.source);
            Err::<(), _>(43u8)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(43));
    assert_eq!(e.guard, Some(Error::Spool(payload::Error::Changed)));
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert!(s.failed());
}
#[test]
fn modified_record_rejects_before_callback_and_is_sticky() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let path = p.path.join("00000000.payload");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, b"changed").unwrap();
    let e = s
        .with_wdt(1, 2, 0, |_| -> Result<(), ()> { panic!("must not invoke") })
        .unwrap_err();
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert_eq!(
        s.verify_unchanged(),
        Err(Error::Spool(payload::Error::Changed))
    );
}
#[test]
fn envelope_binds_kind_map_fdid_count_extent_and_reserved_bytes() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let entry = s.entries[0];
    let bytes = fs::read(p.path.join("00000000.payload")).unwrap();
    for at in [0, 8, 9, 12, 16, 20, 28] {
        let mut b = bytes.clone();
        b[at] ^= 1;
        assert_eq!(entry.body(&b), Err(Error::Binding));
    }
    for len in 0..bytes.len() {
        assert_eq!(entry.body(&bytes[..len]), Err(Error::Binding));
    }
    let mut b = bytes.clone();
    b.push(0);
    assert_eq!(entry.body(&b), Err(Error::Binding));
}
#[test]
fn entry_and_disk_limits_are_terminal_without_partial_success() {
    let p = Paths::new();
    let mut s = Inventory::new(
        p.create(spool::Limits::default()),
        Limits {
            max_entries: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    s.insert_wdt(1, 2, &wdt()).unwrap();
    assert_eq!(s.insert_own_tile(tile(), &[]), Err(Error::Limit));
    assert_eq!(s.stats().unwrap().entries, 1);
    let p = Paths::new();
    let mut s = Inventory::new(
        p.create(spool::Limits {
            max_disk_bytes: HEADER - 1,
            ..spool::Limits::default()
        }),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        s.insert_own_tile(tile(), &[]),
        Err(Error::Spool(payload::Error::Limit))
    );
    assert_eq!(s.stats().unwrap().entries, 0);
}
#[test]
fn callback_reservation_refuses_before_repository_reload() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let e = s
        .with_wdt(1, 2, MAX_ACTIVE_BYTES, |_| Ok::<_, ()>(()))
        .unwrap_err();
    assert_eq!(e.cause, Cause::Access(Error::Limit));
    assert_eq!(s.stats().unwrap().repository.reloads, 0);
}
#[test]
fn limits_only_reduce_and_nonempty_repository_cannot_be_reinterpreted() {
    for limits in [
        Limits {
            max_entries: MAX_ENTRIES + 1,
            ..Limits::default()
        },
        Limits {
            max_heap_bytes: MAX_HEAP_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_active_bytes: MAX_ACTIVE_BYTES + 1,
            ..Limits::default()
        },
    ] {
        let p = Paths::new();
        assert!(matches!(
            Inventory::new(p.create(spool::Limits::default()), limits),
            Err(Error::InvalidLimits)
        ));
    }
    let p = Paths::new();
    let mut repo = p.create(spool::Limits::default());
    repo.write(1, |w| w.bytes(b"x")).unwrap();
    assert!(matches!(
        Inventory::new(repo, Limits::default()),
        Err(Error::State)
    ));
}
#[test]
fn heap_reservation_and_peak_are_bounded() {
    let p = Paths::new();
    let mut s = inventory(&p);
    let baseline = s.stats().unwrap().reserved_heap_bytes;
    assert_eq!(
        baseline,
        2 * spool::Limits::default().max_descriptor_bytes + GUARD_ALLOWANCE
    );
    for id in 0..8 {
        s.insert_wdt(id, id + 1, &wdt()).unwrap();
    }
    let stats = s.stats().unwrap();
    assert!(stats.reserved_heap_bytes > baseline);
    assert!(stats.peak_admitted_heap_bytes <= MAX_HEAP_BYTES);
    assert!(stats.peak_admitted_active_bytes <= MAX_ACTIVE_BYTES);
}
#[test]
fn callback_capture_drops_before_final_setup_failure_guards() {
    struct Change(std::path::PathBuf);
    impl Drop for Change {
        fn drop(&mut self) {
            replace_source(&self.0);
            fs::write(self.0.join("closure-drop"), b"x").unwrap();
        }
    }
    let p = Paths::new();
    let mut s = inventory(&p);
    let change = Change(p.source.clone());
    let e = s
        .with_wdt(99, 1, 0, move |_| {
            drop(change);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Access(Error::Missing));
    assert_eq!(e.guard, Some(Error::Spool(payload::Error::Changed)));
    assert!(p.source.join("closure-drop").exists());
}
#[test]
fn returned_value_drops_before_error_is_returned_after_source_change() {
    struct Change(std::path::PathBuf);
    impl Drop for Change {
        fn drop(&mut self) {
            fs::write(self.0.join("value-drop"), b"x").unwrap();
        }
    }
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    let result = s.with_wdt(1, 2, 0, |_| {
        replace_source(&p.source);
        Ok::<_, u8>(Change(p.source.clone()))
    });
    let e = match result {
        Ok(_) => panic!("source change admitted"),
        Err(e) => e,
    };
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert!(p.source.join("value-drop").exists());
    assert!(s.failed());
}
#[test]
fn exact_replay_active_admission_and_one_byte_below() {
    let b = row(0, 1);
    let active = HEADER + 3 * b.len();
    for cap in [active - 1, active] {
        let p = Paths::new();
        let mut s = Inventory::new(
            p.create(spool::Limits::default()),
            Limits {
                max_active_bytes: cap,
                cache: cache::Limits {
                    max_memory_bytes: 2 * b.len(),
                    ..cache::Limits::default()
                },
                ..Limits::default()
            },
        )
        .unwrap();
        if cap < active {
            assert_eq!(s.insert_own_tile(tile(), &b), Err(Error::Limit));
        } else {
            s.insert_own_tile(tile(), &b).unwrap();
            assert_eq!(s.replay_own_tile(tile(), tile().map_id).unwrap(), b);
            assert_eq!(s.stats().unwrap().peak_admitted_active_bytes, active);
        }
    }
}
#[test]
fn complete_fdid_wdt_main_and_maid_survive_disk_roundtrip() {
    let mut b = wdt();
    for (tag, body) in [
        (b"DHPM", {
            let mut v = vec![0u8; 32];
            v[..4].copy_from_slice(&0x200u32.to_le_bytes());
            v
        }),
        (b"NIAM", {
            let mut v = vec![0u8; 4096 * 8];
            v[0..4].copy_from_slice(&1u32.to_le_bytes());
            v[4095 * 8..4095 * 8 + 4].copy_from_slice(&1u32.to_le_bytes());
            v
        }),
        (b"DIAM", {
            let mut v = vec![0u8; 4096 * 32];
            v[4..8].copy_from_slice(&77u32.to_le_bytes());
            v[4095 * 32 + 4..4095 * 32 + 8].copy_from_slice(&88u32.to_le_bytes());
            v
        }),
    ] {
        b.extend_from_slice(tag);
        word(&mut b, body.len() as u32);
        b.extend_from_slice(&body);
    }
    let original = plan::Wdt::parse(&b, plan::Limits::default())
        .unwrap()
        .stats();
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(70205, 12345, &b).unwrap();
    assert_eq!(
        s.with_wdt(70205, 12345, 0, |w| Ok::<_, ()>(w.stats()))
            .unwrap(),
        original
    );
    assert_eq!(original.present_tiles, 2);
}
#[test]
fn source_anchor_does_not_authenticate_unread_sibling_contents() {
    let p = Paths::new();
    let mut s = inventory(&p);
    s.insert_wdt(1, 2, &wdt()).unwrap();
    fs::write(p.source.join("unread-sibling"), b"x").unwrap();
    s.verify_unchanged().unwrap();
    assert_eq!(s.with_wdt(1, 2, 0, |w| Ok::<_, ()>(w.flags())).unwrap(), 0);
}
#[test]
fn declared_key_file_change_is_detected_without_reading_key_bytes() {
    let p = Paths::new();
    let key = p.base.join("declared-key");
    fs::write(&key, b"authored-placeholder").unwrap();
    let repo = spool::Spool::create(
        &p.path,
        spool::ProtectedInputs::capture(&p.source, &[&key]).unwrap(),
        spool::Limits::default(),
    )
    .unwrap();
    let mut s = Inventory::new(repo, Limits::default()).unwrap();
    s.insert_wdt(1, 2, &wdt()).unwrap();
    fs::write(&key, b"changed-authored-placeholder").unwrap();
    assert_eq!(
        s.verify_unchanged(),
        Err(Error::Spool(payload::Error::Changed))
    );
}
