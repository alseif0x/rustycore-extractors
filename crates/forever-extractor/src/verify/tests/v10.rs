// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
fn plan() -> Plan {
    let mut p = Plan::parse(include_str!(
        "../../../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv"
    ))
    .unwrap();
    p.locale = Locale::EnUs;
    p
}
fn source() -> Vec<u8> {
    let mut b = empty_db2("CinematicCamera");
    put32(&mut b, 152, 0xC2CEFDF9);
    b
}
#[test]
fn v10_complete_db2_only_intrinsic_and_links() {
    let lab = Lab::new();
    let p = plan();
    let s = lab.publish(context(&p), &[db2_file("CinematicCamera", source())]);
    assert_eq!(verify(&p, &lab.output(), None).unwrap(), s);
    assert_eq!(
        super::super::verify_links(&p, &lab.output(), None).unwrap(),
        s
    );
    assert_eq!(s.files, 1);
}
#[test]
fn v10_metadata_admission_exact_set_caps_nonempty_and_component() {
    let p = plan();
    let a = artifact(Component::Db2, "dbc/enUS/CinematicCamera.db2", &source());
    let admit = |v: &[Artifact]| super::super::v10::admit_metadata(&p, v);
    assert!(admit(std::slice::from_ref(&a)).is_ok());
    assert_eq!(admit(&[]).unwrap_err().code, "product_inventory");
    for (bytes, code) in [
        (0, "product_empty_artifact"),
        (16 * 1024 * 1024 + 1, "product_file_limit"),
    ] {
        let mut x = a.clone();
        x.bytes = bytes;
        assert_eq!(admit(&[x]).unwrap_err().code, code);
    }
    let mut x = a.clone();
    x.component = Component::Cameras;
    assert_eq!(admit(&[x]).unwrap_err().code, "product_inventory");
    let mut x = a.clone();
    x.path = "dbc/esES/CinematicCamera.db2".into();
    assert_eq!(admit(&[x]).unwrap_err().code, "product_artifact_name");
    assert_eq!(
        admit(&[a.clone(), a]).unwrap_err().code,
        "product_inventory"
    );
}
#[test]
fn v10_bad_target_hash_or_unselected_table_fails_complete_package() {
    for kind in 0..2 {
        let lab = Lab::new();
        let p = plan();
        let mut b = source();
        let name = if kind == 0 {
            put32(&mut b, 152, 0);
            "CinematicCamera"
        } else {
            "TactKey"
        };
        lab.publish(context(&p), &[db2_file(name, b)]);
        assert!(verify(&p, &lab.output(), None).is_err());
    }
}
#[test]
fn v10_declared_total_boundary_is_checked_before_payloads() {
    let catalog =
        forever_db2::catalog::tables_for(forever_db2::catalog::WOWDBDEFS_70374_CATALOG_ID).unwrap();
    for count in [1024usize, 1025] {
        let mut p = plan();
        p.db2 = catalog[..count]
            .iter()
            .map(|m| Entry {
                name: m.name.into(),
                id: m.file_data_id,
            })
            .collect();
        let a: Vec<_> = p
            .db2
            .iter()
            .map(|e| {
                let mut x = artifact(
                    Component::Db2,
                    &format!("dbc/enUS/{}.db2", e.name),
                    b"metadata only",
                );
                x.bytes = 16 * 1024 * 1024;
                x
            })
            .collect();
        let result = super::super::v10::admit_metadata(&p, &a);
        if count == 1024 {
            assert!(result.is_ok())
        } else {
            assert_eq!(result.unwrap_err().code, "product_db2_total_limit")
        }
    }
}
