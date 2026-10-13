// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: canonical rooted package inventories and mutation guards.
// Modified 2026-10-12: seed distinct metadata times before mutation snapshots.
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
        let p = std::env::temp_dir().join(format!(
            "rce-navigation-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::create_dir(p.join("maps")).unwrap();
        fs::create_dir(p.join("vmaps")).unwrap();
        Self(p)
    }
    fn maps(&self) -> PathBuf {
        self.0.join("maps")
    }
    fn vmaps(&self) -> PathBuf {
        self.0.join("vmaps")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
// Close the attribute writer before any readonly snapshot is captured.
pub(super) fn seed_write_time(path: &Path) {
    use std::os::windows::fs::OpenOptionsExt;
    let past = std::time::UNIX_EPOCH + std::time::Duration::from_secs(86_400);
    let file = fs::OpenOptions::new()
        .access_mode(0x0100) // FILE_WRITE_ATTRIBUTES
        .share_mode(7)
        .custom_flags(0x0200_0000) // FILE_FLAG_BACKUP_SEMANTICS
        .open(path)
        .unwrap();
    file.set_times(fs::FileTimes::new().set_modified(past))
        .unwrap();
    drop(file);
    assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), past);
}
fn row(id: u32, parent: i16, cosmetic: i16) -> DirectRecord<NavigationMapFields> {
    DirectRecord {
        id,
        fields: NavigationMapFields {
            map_type: 0,
            instance_type: 0,
            parent_map_id: parent,
            cosmetic_parent_map_id: cosmetic,
            flags: 0,
        },
    }
}
fn list(build: u32, tiles: &[(u8, u8)]) -> Vec<u8> {
    let mut b = vec![b'0'; 4108];
    b[..4].copy_from_slice(b"MAPS");
    b[4..8].copy_from_slice(&10u32.to_le_bytes());
    b[8..12].copy_from_slice(&build.to_le_bytes());
    for &(r, c) in tiles {
        b[12 + 4095 - (r as usize * 64 + c as usize)] = b'1';
    }
    b
}
fn package(
    t: &Temp,
    rows: &[DirectRecord<NavigationMapFields>],
    options: Options,
    limits: Limits,
) -> Result<Package, Error> {
    Package::open_rows(&t.maps(), &t.vmaps(), rows, 70205, options, limits)
}
fn basic(t: &Temp) -> Package {
    package(t, &[row(1, -1, -1)], Options::default(), Limits::default()).unwrap()
}
fn add_list(t: &Temp, id: u32, tiles: &[(u8, u8)]) {
    fs::write(
        t.maps().join(format!("{id:04}.tilelist")),
        list(70205, tiles),
    )
    .unwrap();
}
fn add_vmaps(t: &Temp, id: u32, tiles: &[(u8, u8)]) {
    let d = t.vmaps().join(format!("{id:04}"));
    fs::create_dir(&d).unwrap();
    for &(r, c) in tiles {
        fs::write(d.join(format!("{id:04}_{r:02}_{c:02}.vmtile")), []).unwrap();
    }
}
#[test]
fn complete_union_components_empty_dirs_and_list_hashes() {
    let t = Temp::new();
    add_list(&t, 1, &[(63, 0), (0, 63), (7, 8)]);
    add_vmaps(&t, 1, &[(7, 8), (6, 9)]);
    add_vmaps(&t, 2, &[]);
    fs::write(
        t.maps().join("0001_00_63.map"),
        b"inventory does not decode terrain",
    )
    .unwrap();
    fs::write(
        t.vmaps().join("ignored.vmo"),
        b"inventory does not decode models",
    )
    .unwrap();
    let mut p = package(
        &t,
        &[row(1, -1, -1), row(2, -1, -1)],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    let m = p.plan().map(1).unwrap();
    assert_eq!(m.tile_count(), 4);
    assert_eq!(p.plan().map(2).unwrap().tile_count(), 0);
    let both = Coordinate { row: 7, column: 8 };
    assert!(m.terrain_contains(both) && m.collision_contains(both));
    assert!(m.collision_contains(Coordinate { row: 6, column: 9 }));
    assert_eq!(p.tilelists().len(), 1);
    let mut h = hash::Sha256::new();
    h.update(&list(70205, &[(63, 0), (0, 63), (7, 8)]));
    assert_eq!(
        p.tilelists()[0],
        TileListIdentity {
            map_id: 1,
            sha256: h.finish()
        }
    );
    p.verify_unchanged().unwrap();
    assert!(!p.failed());
    assert_eq!(p.statistics().descriptors, 5);
    assert_eq!(p.statistics().tilelist_bytes, 4108);
}
#[test]
fn empty_package_is_valid_and_keeps_complete_source_graph() {
    let t = Temp::new();
    let p = basic(&t);
    assert!(p.plan().maps().is_empty());
    assert_eq!(p.plan().parent_rows().count(), 1);
    assert!(p.tilelists().is_empty());
}
#[test]
fn ancestor_inventory_and_selected_map_override_filters() {
    let t = Temp::new();
    add_list(&t, 1, &[(63, 63)]);
    add_list(&t, 3, &[(0, 0)]);
    let mut rows = [row(1, -1, -1), row(2, -1, 1), row(3, 2, -1), row(4, -1, -1)];
    rows[0].fields.flags = 2;
    rows[2].fields.map_type = 3;
    rows[2].fields.instance_type = 3;
    let p = package(
        &t,
        &rows,
        Options {
            map: Some(3),
            skip_continents: true,
            skip_junk: true,
            skip_battlegrounds: true,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        p.plan()
            .maps()
            .iter()
            .filter(|m| m.is_output())
            .map(|m| m.map_id())
            .collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(p.plan().map(3).unwrap().root_map_id(), 1);
}
#[test]
fn excluded_maps_do_not_read_corrupt_lists_or_traverse_children() {
    let t = Temp::new();
    fs::write(t.maps().join("0001.tilelist"), b"bad").unwrap();
    let d = t.vmaps().join("0001");
    fs::create_dir(&d).unwrap();
    fs::write(d.join("malformed.vmtile"), b"bad").unwrap();
    let mut rows = [row(1, -1, -1)];
    rows[0].fields.flags = 2;
    let p = package(
        &t,
        &rows,
        Options {
            skip_junk: true,
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    assert!(p.plan().maps().is_empty());
    assert_eq!(p.statistics().tilelists, 0);
    assert_eq!(p.statistics().entries, 2);
}
#[test]
fn unknown_map_and_missing_ancestor_inventory_reject() {
    let t = Temp::new();
    add_vmaps(&t, 2, &[]);
    assert!(matches!(
        package(
            &t,
            &[row(1, -1, -1)],
            Default::default(),
            Default::default()
        ),
        Err(Error::Inventory(super::super::Error::UnknownMap))
    ));
    let t = Temp::new();
    add_list(&t, 2, &[(0, 0)]);
    assert!(matches!(
        package(
            &t,
            &[row(1, -1, -1), row(2, 1, -1)],
            Default::default(),
            Default::default()
        ),
        Err(Error::Inventory(super::super::Error::MissingInventory))
    ));
}
#[test]
fn corrupt_truncated_wrong_build_or_noncanonical_tilelist_is_fatal() {
    for mode in 0..5 {
        let t = Temp::new();
        let mut b = list(70205, &[(0, 0)]);
        match mode {
            0 => {
                b.pop();
            }
            1 => b.push(0),
            2 => b[0] = 0,
            3 => b[8] = 0,
            4 => b[12] = b'x',
            _ => unreachable!(),
        };
        fs::write(t.maps().join("0001.tilelist"), b).unwrap();
        assert!(matches!(
            package(
                &t,
                &[row(1, -1, -1)],
                Default::default(),
                Default::default()
            ),
            Err(Error::Inventory(super::super::Error::TileList))
        ));
    }
}
#[test]
fn canonical_filename_domain_rejects_ambiguous_names_and_coordinates() {
    for name in [
        "1.tilelist",
        "10000.tilelist",
        "0001-extra.tilelist",
        "xxxx.tilelist",
    ] {
        let t = Temp::new();
        fs::write(t.maps().join(name), list(70205, &[])).unwrap();
        assert!(matches!(
            package(
                &t,
                &[row(1, -1, -1)],
                Default::default(),
                Default::default()
            ),
            Err(Error::Filename)
        ));
    }
    for name in [
        "0002_00_00.vmtile",
        "0001_64_00.vmtile",
        "0001_00_64.vmtile",
        "0001_a0_00.vmtile",
        "x.vmtile",
    ] {
        let t = Temp::new();
        add_vmaps(&t, 1, &[]);
        fs::write(t.vmaps().join("0001").join(name), []).unwrap();
        assert!(matches!(
            package(
                &t,
                &[row(1, -1, -1)],
                Default::default(),
                Default::default()
            ),
            Err(Error::Filename)
        ));
    }
    let t = Temp::new();
    fs::create_dir(t.vmaps().join("1")).unwrap();
    assert!(matches!(
        package(
            &t,
            &[row(1, -1, -1)],
            Default::default(),
            Default::default()
        ),
        Err(Error::Filename)
    ));
}

#[test]
fn observed_file_changes_and_deletion_permanently_poison() {
    for remove in [false, true] {
        let t = Temp::new();
        add_list(&t, 1, &[(0, 0)]);
        seed_write_time(&t.maps().join("0001.tilelist"));
        seed_write_time(&t.maps());
        let mut p = basic(&t);
        let name = t.maps().join("0001.tilelist");
        if remove {
            fs::remove_file(name).unwrap();
        } else {
            fs::write(name, list(70205, &[(0, 1)])).unwrap();
        }
        assert_eq!(p.verify_unchanged(), Err(Error::InputChanged));
        assert!(p.failed());
        assert_eq!(p.verify_unchanged(), Err(Error::InputChanged));
    }
}
#[test]
fn new_root_entry_child_entry_and_root_replacement_poison() {
    for mode in 0..3 {
        let t = Temp::new();
        add_list(&t, 1, &[(0, 0)]);
        add_vmaps(&t, 1, &[]);
        seed_write_time(&t.maps());
        seed_write_time(&t.vmaps().join("0001"));
        let mut p = basic(&t);
        match mode {
            0 => fs::write(t.maps().join("new"), []).unwrap(),
            1 => fs::write(t.vmaps().join("0001/new"), []).unwrap(),
            2 => {
                fs::rename(t.maps(), t.0.join("old-maps")).unwrap();
                fs::create_dir(t.maps()).unwrap();
            }
            _ => unreachable!(),
        };
        assert_eq!(p.verify_unchanged(), Err(Error::InputChanged));
        assert!(p.failed());
    }
}
#[test]
fn invalid_limits_fail_before_opening_any_root() {
    let mut a = Limits::default();
    a.entries += 1;
    let mut b = Limits::default();
    b.tilelists += 1;
    let mut c = Limits::default();
    c.terrain_source.max_entries += 1;
    let mut d = Limits::default();
    d.collision_source.max_entries += 1;
    let mut e = Limits::default();
    e.store.max_maps += 1;
    let f = Limits {
        heap_bytes: 0,
        ..Default::default()
    };
    for l in [a, b, c, d, e, f] {
        assert!(
            Package::open_rows(
                Path::new("/missing-maps"),
                Path::new("/missing-vmaps"),
                &[row(1, -1, -1)],
                70205,
                Default::default(),
                l
            )
            .is_err()
        );
    }
}
#[test]
fn entries_lists_descriptors_metadata_and_owned_capacity_are_bounded() {
    let t = Temp::new();
    add_list(&t, 1, &[(0, 0)]);
    add_vmaps(&t, 1, &[(0, 0)]);
    let p = basic(&t);
    let peak = p.statistics().peak_capacity;
    for l in [
        Limits {
            entries: 0,
            ..Default::default()
        },
        Limits {
            tilelists: 0,
            ..Default::default()
        },
        Limits {
            descriptors: 0,
            ..Default::default()
        },
        Limits {
            metadata_calls: 0,
            ..Default::default()
        },
        Limits {
            heap_bytes: peak - 1,
            ..Default::default()
        },
    ] {
        assert!(package(&t, &[row(1, -1, -1)], Default::default(), l).is_err());
    }
    let p = package(
        &t,
        &[row(1, -1, -1)],
        Default::default(),
        Limits {
            heap_bytes: peak,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.statistics().peak_capacity, peak);
}
#[test]
fn bound_empty_tile_runs_and_detects_preexisting_package_mutation() {
    let t = Temp::new();
    add_list(&t, 1, &[(0, 0)]);
    fs::write(
        t.maps().join("0001_00_00.map"),
        crate::terrain::tests::fixture(70205, 0, 0, false),
    )
    .unwrap();
    let mut p = basic(&t);
    let r = pipeline::Request {
        tile: crate::tilebuilder::Request {
            map_id: 1,
            tile: Coordinate { row: 0, column: 0 },
            big_base_unit: true,
            slopes: Default::default(),
        },
        skip_liquids: false,
    };
    let types = models::LiquidTypes::new(&[]).unwrap();
    assert!(matches!(
        p.run_tile(types, &[], r, Default::default()),
        Ok(pipeline::Outcome::Empty {
            reason: pipeline::EmptyReason::NoVertices,
            ..
        })
    ));
    fs::write(
        t.maps().join("0001_00_00.map"),
        crate::terrain::tests::fixture(70205, 1, 0, false),
    )
    .unwrap();
    assert!(matches!(
        p.run_tile(types, &[], r, Default::default()),
        Err(Error::InputChanged)
    ));
    assert!(p.failed());
}
#[test]
fn invalid_pipeline_limits_do_not_mutate_inventory_verification_stats() {
    let t = Temp::new();
    let mut p = basic(&t);
    let stats = p.statistics();
    let r = pipeline::Request {
        tile: crate::tilebuilder::Request {
            map_id: 1,
            tile: Coordinate { row: 0, column: 0 },
            big_base_unit: true,
            slopes: Default::default(),
        },
        skip_liquids: false,
    };
    let mut l = pipeline::Limits::default();
    l.heap_bytes += 1;
    assert!(matches!(
        p.run_tile(models::LiquidTypes::new(&[]).unwrap(), &[], r, l),
        Err(Error::Pipeline(pipeline::Error::Limit))
    ));
    assert_eq!(p.statistics(), stats);
}

#[test]
fn strict_windows_profile_rejects_unobservable_unused_names_and_relative_roots() {
    let t = Temp::new();
    fs::write(t.maps().join("unused-ñ"), b"unused").unwrap();
    assert!(matches!(
        package(
            &t,
            &[row(1, -1, -1)],
            Options::default(),
            Default::default()
        ),
        Err(Error::Filename)
    ));
    assert!(matches!(
        Package::open_rows(
            Path::new("relative"),
            &t.vmaps(),
            &[],
            70205,
            Options::default(),
            Default::default()
        ),
        Err(Error::Path)
    ));
}
#[test]
fn observed_read_failure_has_priority_over_later_guard_budget_exhaustion() {
    let t = Temp::new();
    add_list(&t, 1, &[]);
    let mut p = basic(&t);
    let file = t.maps().join("0001.tilelist");
    let expected = forever_platform::windows::ReadFile::open(
        &file,
        Default::default(),
        forever_platform::windows::LinkPolicy::Allow,
    )
    .unwrap()
    .identity();
    fs::write(&file, b"replacement").unwrap();
    p.budget.used = 0;
    p.budget.limits.metadata_calls = ((t.maps().components().count() + 2) * 64
        + (t.vmaps().components().count() + 2) * 64
        + (file.components().count() + 2) * 128) as u64;
    assert_eq!(
        p.guard
            .read_list(0, b"0001.tilelist", expected, &mut p.budget),
        Err(Error::InputChanged)
    );
}

#[test]
fn pipeline_changes_survive_final_limit_and_permanently_poison_package() {
    use store::{Error as S, SourceError as SE};
    use terrain::load::{Error as T, SourceError as TE};
    for error in [
        Error::InputChanged,
        Error::Pipeline(pipeline::Error::Terrain(T::Source(TE::InputChanged))),
        Error::Pipeline(pipeline::Error::Store(S::Source(SE::InputChanged))),
        Error::Pipeline(pipeline::Error::Store(S::Poisoned)),
        Error::Pipeline(pipeline::Error::Models(models::LoadedError::Store(
            S::Source(SE::InputChanged),
        ))),
        Error::Pipeline(pipeline::Error::Models(models::LoadedError::Store(
            S::Poisoned,
        ))),
    ] {
        let t = Temp::new();
        let mut p = basic(&t);
        p.budget.limits.metadata_calls = 0;
        let result: Result<(), Error> = p.finish_tile(Err(error));
        assert_eq!(result, Err(error));
        assert!(p.failed());
        assert_eq!(p.verify_unchanged(), Err(Error::InputChanged));
    }
}
#[test]
fn final_limit_without_observed_change_remains_nonpoisoning() {
    for body in [
        Ok(()),
        Err(Error::Pipeline(pipeline::Error::MissingTerrain)),
    ] {
        let t = Temp::new();
        let mut p = basic(&t);
        p.budget.limits.metadata_calls = 0;
        assert_eq!(p.finish_tile(body), Err(Error::Limit));
        assert!(!p.failed());
    }
}
