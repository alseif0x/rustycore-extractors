// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v27 per-table Map declaration boundaries; no client data.
use super::*;

fn section(size: u64) -> package::PartialEncryptedSection {
    package::PartialEncryptedSection {
        lookup: "0123456789abcdef".into(),
        frame_ordinal: 0,
        decoded_size: size,
    }
}

fn map_declaration(sizes: &[u64]) -> package::PartialEncryptedTable {
    package::PartialEncryptedTable {
        path: "dbc/esES/Map.db2".into(),
        sections: sizes.iter().map(|size| section(*size)).collect(),
    }
}

#[test]
fn v27_map_uses_per_table_multi_frame_coverage_and_rejects_global_declarations() {
    let structural = BTreeMap::from([("dbc/esES/Map.db2".to_owned(), vec![5])]);
    let map = map_declaration(&[2, 3]);
    verify_v26_partial_declarations(&structural, &[], std::slice::from_ref(&map)).unwrap();

    assert_eq!(
        verify_v26_partial_declarations(&structural, &[section(5)], &[map])
            .unwrap_err()
            .code,
        "product_partial_manifest_scope"
    );
}

#[test]
fn v27_map_declaration_is_mandatory_and_exact() {
    let structural = BTreeMap::from([("dbc/esES/Map.db2".to_owned(), vec![5])]);
    assert_eq!(
        verify_v26_partial_declarations(&structural, &[], &[])
            .unwrap_err()
            .code,
        "product_partial_undeclared"
    );
    assert_eq!(
        verify_v26_partial_declarations(&structural, &[], &[map_declaration(&[2, 2])])
            .unwrap_err()
            .code,
        "product_partial_section_count"
    );
    assert_eq!(
        verify_v26_partial_declarations(
            &structural,
            &[],
            &[map_declaration(&[2, 3]), map_declaration(&[2, 3])],
        )
        .unwrap_err()
        .code,
        "product_partial_declaration_extra"
    );
}
