// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
use super::*;
use crate::driver;
#[path = "fixture.rs"]
mod fixture;
use fixture::Fixture;
fn extract(f: &Fixture, resume: bool) -> forever_casc::package::Summary {
    driver::extract_disk(
        &f.plan(),
        &f.client,
        &f.output,
        [3; 32],
        resume,
        None,
        &f.scratch,
    )
    .unwrap()
}
#[test]
fn product70425_genuine_startup_world_models_package_and_offline_links() {
    let f = Fixture::new(&[(0, 100, -1)], &Fixture::extra_m2(90, 17));
    let summary = extract(&f, false);
    assert!(summary.files >= 7);
    let a = crate::verify::verify(&f.plan(), &f.output, Some(summary.context)).unwrap();
    let b = crate::verify::verify_links(&f.plan(), &f.output, Some(summary.context)).unwrap();
    assert_eq!(a, b);
    assert!(f.output.join("vmaps/FILE0000005A.xxx.vmo").is_file());
}
#[test]
fn product70425_missing_wdt_is_unknown_with_no_tree_or_proof() {
    let f = Fixture::missing_wdt();
    let summary = extract(&f, false);
    assert_eq!(summary.files, 2);
    crate::verify::verify(&f.plan(), &f.output, Some(summary.context)).unwrap();
    assert!(!f.output.join("vmaps/0000/0000.vmtree").exists());
}
#[test]
fn product70425_resume_recomputes_with_fresh_six_repositories() {
    let f = Fixture::new(&[(0, 100, -1)], &Fixture::extra_m2(90, 17));
    let first = extract(&f, false);
    // Model interruption during final package closeout: the collision producer
    // has sealed its artifacts, but the package still carries its incomplete marker.
    std::fs::write(
        f.output.join(forever_casc::package::INCOMPLETE),
        b"incomplete\n",
    )
    .unwrap();
    let second = extract(&f, true);
    assert_eq!(first, second);
    let runs = std::fs::read_dir(&f.scratch)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(runs.len(), 2);
    for run in runs {
        assert_eq!(std::fs::read_dir(run.path()).unwrap().count(), 6);
    }
}
#[test]
fn product70425_parent_replay_and_known_empty_barrier_verify_complete() {
    for barrier in [false, true] {
        let maps = if barrier {
            vec![(0, 100, -1), (1, 200, 0), (2, 300, 1)]
        } else {
            vec![(0, 100, -1), (1, 200, 0)]
        };
        let extra = if barrier {
            Fixture::empty_barrier()
        } else {
            Fixture::inherited()
        };
        let f = Fixture::new(&maps, &extra);
        let s = extract(&f, false);
        crate::verify::verify_links(&f.plan(), &f.output, Some(s.context)).unwrap();
        if barrier {
            let b = std::fs::read(f.output.join(coverage::proof_path(2))).unwrap();
            assert!(coverage::proof(&b, 2).unwrap().is_empty());
        }
    }
}
#[test]
fn product70425_complete_no_collision_wmo_does_not_abort_following_m2() {
    let f = Fixture::new(&[(0, 100, -1)], &Fixture::no_collision_then_m2());
    let s = extract(&f, false);
    crate::verify::verify(&f.plan(), &f.output, Some(s.context)).unwrap();
    let b = std::fs::read(f.output.join(coverage::proof_path(0))).unwrap();
    let dir =
        forever_vmap::placement::DirBin::parse(coverage::proof(&b, 0).unwrap(), Default::default())
            .unwrap();
    assert_eq!(dir.record_count(), 1);
    assert_eq!(dir.records().next().unwrap().id(), u32::MAX - 1);
    assert!(!f.output.join("vmaps/FILE0000005B.xxx.vmo").exists());
}
#[test]
fn product70425_malformed_present_adt_retains_failure_without_package_seal() {
    let mut extra = Fixture::extra_m2(90, 17);
    extra[1].2 = b"malformed".to_vec();
    let f = Fixture::new(&[(0, 100, -1)], &extra);
    assert!(
        driver::extract_disk(
            &f.plan(),
            &f.client,
            &f.output,
            [3; 32],
            false,
            None,
            &f.scratch
        )
        .is_err()
    );
    assert!(crate::verify::verify(&f.plan(), &f.output, None).is_err());
    assert_eq!(std::fs::read_dir(&f.scratch).unwrap().count(), 1);
}
#[test]
fn product70425_unknown_selected_map_rejects_without_fallback() {
    let f = Fixture::missing_wdt();
    let p = crate::plan::Plan {
        map_domain: Some(99),
        ..f.plan()
    };
    assert!(
        driver::extract_disk(&p, &f.client, &f.output, [3; 32], false, None, &f.scratch).is_err()
    );
    assert!(crate::verify::verify(&p, &f.output, None).is_err());
}
