// SPDX-License-Identifier: GPL-3.0-or-later
// Authored literal WDC5 sources; no client bytes or production metadata writer.
use super::*;
const FIELDS: &[(u16, u16)] = &[
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 2),
    (8, 1),
    (8, 1),
    (8, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (8, 1),
    (32, 1),
    (16, 1),
    (8, 1),
    (16, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 3),
];
fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn offset(i: usize) -> usize {
    FIELDS[..i]
        .iter()
        .map(|(w, n)| usize::from(w * n / 8))
        .sum()
}
pub(crate) fn table(rows: &[(u32, u32, i16, i16)]) -> Vec<u8> {
    let start = 244 + FIELDS.len() * 28;
    let stride = offset(FIELDS.len());
    let mut b = vec![0; start];
    b[..4].copy_from_slice(b"WDC5");
    let mut texts = Vec::new();
    let mut records = Vec::new();
    for (i, &(_, wdt, parent, cosmetic)) in rows.iter().enumerate() {
        let mut row = vec![0; stride];
        for (field, text) in [(0, b"Synthetic".as_slice()), (1, b"Map".as_slice())] {
            put(
                &mut row,
                offset(field),
                ((rows.len() - i) * stride + texts.len() - offset(field)) as u32,
            );
            texts.extend_from_slice(text);
            texts.push(0);
        }
        put(&mut row, offset(21), wdt);
        for (field, value) in [(13, parent), (14, cosmetic)] {
            let at = offset(field);
            row[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }
        records.extend(row);
    }
    for (at, n) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, FIELDS.len() as u32),
        (144, stride as u32),
        (148, texts.len() as u32),
        (156, 0xD43AFAC3),
        (164, u32::MAX),
        (176, FIELDS.len() as u32),
        (180, stride as u32),
        (188, (FIELDS.len() * 24) as u32),
        (200, 1),
        (212, start as u32),
        (216, rows.len() as u32),
        (220, texts.len() as u32),
        (228, (rows.len() * 4) as u32),
    ] {
        put(&mut b, at, n);
    }
    b[172..174].copy_from_slice(&4u16.to_le_bytes());
    b[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (i, &(width, count)) in FIELDS.iter().enumerate() {
        let primitive = 244 + i * 4;
        b[primitive..primitive + 2].copy_from_slice(&(32 - width).to_le_bytes());
        b[primitive + 2..primitive + 4].copy_from_slice(&(offset(i) as u16).to_le_bytes());
        let col = 244 + FIELDS.len() * 4 + i * 24;
        b[col..col + 2].copy_from_slice(&((offset(i) * 8) as u16).to_le_bytes());
        b[col + 2..col + 4].copy_from_slice(&(width * count).to_le_bytes());
    }
    b.extend(records);
    b.extend(texts);
    for (id, _, _, _) in rows {
        b.extend(id.to_le_bytes());
    }
    b
}
#[test]
fn physical_order_and_only_effective_ancestors_survive() {
    let b = table(&[
        (9, 999, -1, -1),
        (17, 117, -1, 7),
        (7, 101, 1, 9),
        (1, 100, -1, -1),
    ]);
    let domain = Domain::derive(&b, 17, true).unwrap();
    assert_eq!(domain.ids(), [17, 7, 1]);
    assert!(!domain.contains(9));
    assert!(domain.resident_bytes() <= MAX_DOMAIN_BYTES);
    assert_eq!(Domain::derive(&b, 7, false).unwrap().ids(), [7, 1]);
    assert_eq!(Domain::derive(&b, 1, false).unwrap().ids(), [1]);
}
#[test]
fn complete_source_digest_includes_excluded_rows() {
    let b = table(&[(9, 999, -1, -1), (7, 101, 1, -1), (1, 100, -1, -1)]);
    let domain = Domain::derive(&b, 7, false).unwrap();
    assert_eq!(domain.verify_source(&b), Ok(()));
    let changed = table(&[(9, 998, -1, -1), (7, 101, 1, -1), (1, 100, -1, -1)]);
    assert_eq!(
        domain.verify_source(&changed).unwrap_err().code,
        "product_map_domain_changed"
    );
}
#[test]
fn unknown_or_inactive_members_and_invalid_complete_graph_reject() {
    for (rows, id) in [
        (vec![(7, 101, 1, -1), (1, 0, -1, -1)], 7),
        (vec![(1, 100, -1, -1)], 7),
        (vec![(9, 999, 99, -1), (7, 101, -1, -1)], 7),
        (vec![(9, 999, 9, -1), (7, 101, -1, -1)], 7),
    ] {
        assert!(Domain::derive(&table(&rows), id, false).is_err());
    }
}
#[test]
fn source_eof_and_input_budget_are_closed() {
    let mut b = table(&[(1, 100, -1, -1)]);
    b.push(0);
    assert!(Domain::derive(&b, 1, false).is_err());
    assert_eq!(
        Domain::derive(&vec![0; MAX_MAP_BYTES + 1], 1, false)
            .err()
            .unwrap()
            .code,
        "product_map_domain_limit"
    );
}
#[test]
fn compact_capacity_arithmetic_is_checked() {
    assert!(compact_bytes(usize::MAX, 1).is_err());
    assert!(compact_bytes(1, usize::MAX).is_err());
}

#[test]
fn numeric_navigation_view_preserves_original_direct_order_and_both_parents() {
    let bytes = table(&[
        (9, 999, -1, -1),
        (17, 117, -1, 7),
        (7, 101, 1, 9),
        (1, 100, -1, -1),
    ]);
    let domain = Domain::derive(&bytes, 17, true).unwrap();
    let source = source::read_navigation_maps(&bytes).unwrap();
    let rows = domain.navigation_rows(&source).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        [17, 7, 1]
    );
    for row in &rows {
        assert_eq!(
            row,
            source
                .direct_records()
                .iter()
                .find(|original| original.id == row.id)
                .unwrap()
        );
    }
    assert_eq!(rows[1].fields.parent_map_id, 1);
    assert_eq!(rows[1].fields.cosmetic_parent_map_id, 9);
    assert!(domain.navigation_resident_bytes(rows.capacity()).unwrap() <= MAX_DOMAIN_BYTES);
}
#[test]
fn numeric_navigation_view_rejects_duplicate_missing_or_changed_selected_rows() {
    let bytes = table(&[(7, 101, 1, -1), (1, 100, -1, -1)]);
    let domain = Domain::derive(&bytes, 7, true).unwrap();
    for rows in [
        vec![(7, 101, 1, -1)],
        vec![(7, 101, 1, -1), (7, 101, 1, -1), (1, 100, -1, -1)],
        vec![(7, 101, 2, -1), (1, 100, -1, -1)],
    ] {
        let source = source::read_navigation_maps(&table(&rows)).unwrap();
        assert_eq!(
            domain.navigation_rows(&source).unwrap_err().code,
            "product_map_domain_projection"
        );
    }
}
#[test]
fn numeric_navigation_view_capacity_adds_to_compact_domain_limit() {
    let domain = Domain::derive(&table(&[(1, 100, -1, -1)]), 1, true).unwrap();
    assert_eq!(
        domain
            .navigation_resident_bytes(usize::MAX)
            .unwrap_err()
            .code,
        "product_map_domain_limit"
    );
    let excessive =
        MAX_DOMAIN_BYTES / size_of::<source::DirectRecord<source::NavigationMapFields>>() + 1;
    assert_eq!(
        domain
            .navigation_resident_bytes(excessive)
            .unwrap_err()
            .code,
        "product_map_domain_limit"
    );
}
