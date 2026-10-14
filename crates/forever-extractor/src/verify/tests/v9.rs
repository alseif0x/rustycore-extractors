// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
fn plan() -> Plan {
    let mut p = Plan::parse(include_str!(
        "../../../../../tests/contracts/fixtures/forever-product-v9/disabled.tsv"
    ))
    .unwrap();
    p.locale = Locale::EnUs;
    p
}
fn source(id: Option<u32>) -> Vec<u8> {
    let mut bytes = id.map_or_else(|| empty_db2("CinematicCamera"), camera_source);
    put32(&mut bytes, 152, cameras::CameraContract::TABLE_HASH);
    bytes
}
fn files(id: Option<u32>) -> Vec<(Component, String, Vec<u8>)> {
    let mut files = gt_files();
    files.push(db2_file("CinematicCamera", source(id)));
    if let Some(id) = id {
        files.push((
            Component::Cameras,
            cameras::relative_path(id),
            b"opaque authored camera\0\xff".to_vec(),
        ));
    }
    files
}
fn metadata(id: Option<u32>) -> Vec<Artifact> {
    files(id)
        .into_iter()
        .map(|(c, p, b)| artifact(c, &p, &b))
        .collect()
}
fn admit(a: &[Artifact]) -> Result<(), Error> {
    super::super::v9::admit_metadata(&plan(), a)
}
#[test]
fn v9_complete_triple_intrinsic_verification() {
    let lab = Lab::new();
    let p = plan();
    let summary = lab.publish(context(&p), &files(Some(11)));
    assert_eq!(
        super::super::verify(&p, &lab.output(), None).unwrap(),
        summary
    );
    assert_eq!(summary.files, 22);
}
#[test]
fn v9_zero_row_source_completes_without_camera_artifacts_or_directory() {
    let lab = Lab::new();
    let p = plan();
    lab.publish(context(&p), &files(None));
    assert!(super::super::verify(&p, &lab.output(), None).is_ok());
    assert!(!lab.output().join("cameras").exists());
}
#[test]
fn v9_target_source_hash_and_exact_camera_set_are_required() {
    for kind in 0..3 {
        let lab = Lab::new();
        let p = plan();
        let mut f = files(Some(11));
        if kind == 0 {
            let (_, _, b) = f.iter_mut().find(|(c, _, _)| *c == Component::Db2).unwrap();
            put32(b, 152, 0);
        } else if kind == 1 {
            f.retain(|(c, _, _)| *c != Component::Cameras);
        } else {
            f.push((
                Component::Cameras,
                cameras::relative_path(12),
                b"extra".to_vec(),
            ));
        }
        lab.publish(context(&p), &f);
        assert!(super::super::verify(&p, &lab.output(), None).is_err());
    }
}
#[test]
fn v9_metadata_total_includes_source_before_any_payload_independent_of_order() {
    let mut a = metadata(None);
    let source_bytes = a
        .iter()
        .find(|a| a.component == Component::Db2)
        .unwrap()
        .bytes;
    for id in 0..16 {
        a.push(Artifact {
            component: Component::Cameras,
            path: cameras::relative_path(id),
            bytes: cameras::MAX_FILE_BYTES as u64,
            sha256: [0; 32],
        });
    }
    a.last_mut().unwrap().bytes -= source_bytes;
    assert!(admit(&a).is_ok());
    a.reverse();
    assert!(admit(&a).is_ok());
    a[0].bytes += 1;
    assert_eq!(admit(&a).unwrap_err().code, "camera_policy_limit");
    a.reverse();
    assert_eq!(admit(&a).unwrap_err().code, "camera_policy_limit");
}
#[test]
fn v9_declared_component_file_caps_and_nonempty_are_preflighted() {
    for component in [Component::Db2, Component::GameTables, Component::Cameras] {
        let mut a = metadata(Some(11));
        let i = a.iter().position(|a| a.component == component).unwrap();
        a[i].bytes = 0;
        assert_eq!(admit(&a).unwrap_err().code, "product_empty_artifact");
        a[i].bytes = if component == Component::GameTables {
            4 * 1024 * 1024 + 1
        } else {
            16 * 1024 * 1024 + 1
        };
        assert_eq!(admit(&a).unwrap_err().code, "product_file_limit");
    }
}
#[test]
fn v9_metadata_camera_count_boundary_without_payloads() {
    let mut a = metadata(None);
    for id in 0..4096 {
        a.push(Artifact {
            component: Component::Cameras,
            path: cameras::relative_path(id),
            bytes: 1,
            sha256: [0; 32],
        });
    }
    assert!(admit(&a).is_ok());
    a.push(Artifact {
        component: Component::Cameras,
        path: cameras::relative_path(4096),
        bytes: 1,
        sha256: [0; 32],
    });
    assert_eq!(admit(&a).unwrap_err().code, "camera_policy_limit");
}
#[test]
fn v9_metadata_requires_source_catalog_and_all_twenty_gt_names() {
    let mut a = metadata(None);
    a.retain(|a| a.component != Component::Db2);
    assert_eq!(admit(&a).unwrap_err().code, "product_camera_source");
    let mut a = metadata(None);
    a.remove(0);
    assert_eq!(admit(&a).unwrap_err().code, "product_inventory");
    let mut a = metadata(Some(11));
    a.last_mut().unwrap().path = "cameras/FILE0000000b.xxx".into();
    assert_eq!(admit(&a).unwrap_err().code, "product_artifact_name");
}
#[test]
fn v9_metadata_failure_precedes_malformed_payload_parsing() {
    let lab = Lab::new();
    let p = plan();
    let mut f = files(Some(11));
    f.iter_mut()
        .find(|(c, _, _)| *c == Component::Db2)
        .unwrap()
        .2 = b"notDB2".to_vec();
    f.iter_mut()
        .find(|(c, _, _)| *c == Component::Cameras)
        .unwrap()
        .2
        .clear();
    lab.publish(context(&p), &f);
    assert_eq!(
        super::super::verify(&p, &lab.output(), None)
            .unwrap_err()
            .code,
        "product_empty_artifact"
    );
}
