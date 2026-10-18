// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn plan() -> Plan {
    let mut plan = Plan::parse(include_str!(
        "../../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv"
    ))
    .unwrap();
    plan.locale = Locale::EnUs;
    plan
}

fn db2_artifact(name: &str) -> Artifact {
    artifact(
        Component::Db2,
        &format!("dbc/enUS/{name}.db2"),
        b"metadata only",
    )
}

#[test]
fn v16_metadata_admission_exact_set_caps_nonempty_and_component() {
    let p = plan();
    let a = db2_artifact("AccountStoreCategory");
    let admit = |artifacts: &[Artifact]| super::super::v10::admit_metadata(&p, artifacts);
    assert!(admit(std::slice::from_ref(&a)).is_ok());
    assert_eq!(admit(&[]).unwrap_err().code, "product_inventory");

    for (bytes, code) in [
        (0, "product_empty_artifact"),
        (16 * 1024 * 1024 + 1, "product_file_limit"),
    ] {
        let mut invalid = a.clone();
        invalid.bytes = bytes;
        assert_eq!(admit(&[invalid]).unwrap_err().code, code);
    }

    let mut wrong_component = a.clone();
    wrong_component.component = Component::Cameras;
    assert_eq!(
        admit(&[wrong_component]).unwrap_err().code,
        "product_inventory"
    );

    let mut wrong_locale = a.clone();
    wrong_locale.path = "dbc/esES/AccountStoreCategory.db2".into();
    assert_eq!(
        admit(&[wrong_locale]).unwrap_err().code,
        "product_artifact_name"
    );

    let other_name = catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID)
        .unwrap()
        .iter()
        .find(|table| table.name != "AccountStoreCategory")
        .unwrap()
        .name;
    let unselected = db2_artifact(other_name);
    assert_eq!(admit(&[unselected]).unwrap_err().code, "product_inventory");
    assert_eq!(
        admit(&[a.clone(), a]).unwrap_err().code,
        "product_inventory"
    );
}

#[test]
fn v16_metadata_admission_checks_the_16_gib_declared_sum_boundary() {
    let catalog = catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID).unwrap();
    for count in [1024usize, 1025] {
        let mut p = plan();
        p.db2 = catalog
            .iter()
            .map(|metadata| Entry {
                name: metadata.name.into(),
                id: metadata.file_data_id,
            })
            .collect();
        p.db2.sort_by(|left, right| left.name.cmp(&right.name));
        p.db2.truncate(count);
        p.validate().unwrap();

        let artifacts: Vec<_> = p
            .db2
            .iter()
            .map(|entry| {
                let mut item = db2_artifact(&entry.name);
                item.bytes = 16 * 1024 * 1024;
                item
            })
            .collect();
        let result = super::super::v10::admit_metadata(&p, &artifacts);
        if count == 1024 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err().code, "product_db2_total_limit");
        }
    }
}

#[test]
fn v16_verify_routes_through_metadata_preflight_before_payload_inspection() {
    let p = plan();
    let lab = Lab::new();
    let oversized = vec![0; 16 * 1024 * 1024 + 1];
    lab.publish(context(&p), &[db2_file("AccountStoreCategory", oversized)]);

    let error = verify(&p, &lab.output(), None).unwrap_err();
    // The v16 metadata preflight reports product_file_limit. If verify fell
    // through to the post-read DB2 inspector, it would report product_inventory.
    assert_eq!(error.code, "product_file_limit");
}
