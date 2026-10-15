// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent literal WDC5/WDT/ADT synthetic fixtures.
// Reuses this project's authored shared-world test wire builder, not decoder metadata.
use super::*;
pub(crate) fn basic() -> Vec<CollisionFile<'static>> {
    vec![
        (MAP_ID, None, map_db2(&[(7, 100, -1)], &[])),
        (MATERIAL_ID, None, material_db2(&[(7, 0)], &[])),
        (OBJECT_ID, None, object_db2()),
        (TYPE_ID, None, type_db2(&[(22, 7)], &[])),
        (100, None, wdt(&[(3, 2, 201), (1, 4, 202)], true)),
        (201, None, adt(1.0)),
        (202, None, adt(2.0)),
    ]
}
pub(crate) fn object_db2() -> Vec<u8> {
    let mut row = vec![0; 12];
    row[8..10].copy_from_slice(&65535u16.to_le_bytes());
    table(
        0xCB0D39E8,
        &[(32, 1), (32, 1), (16, 1), (8, 1), (8, 1)],
        &[(9, row)],
        &[],
        &[(10, 9), (11, 10), (0, 0), (12, 99)],
    )
}
pub(crate) fn wdt(tiles: &[(u8, u8, u32)], modern: bool) -> Vec<u8> {
    let mut out = chunk(b"MVER", &18u32.to_le_bytes());
    let mut mphd = [0; 32];
    put(&mut mphd, 0, if modern { 0x200 } else { 0 });
    out.extend(chunk(b"MPHD", &mphd));
    let mut main = vec![0; 32768];
    let mut maid = vec![0; 131072];
    for &(x, y, id) in tiles {
        let at = usize::from(y) * 64 + usize::from(x);
        put(&mut main, at * 8, 1);
        put(&mut main, at * 8 + 4, 1234);
        put(&mut maid, at * 32, id);
        put(&mut maid, at * 32 + 4, 201); // Tempting Obj0 fallback must never be used.
    }
    out.extend(chunk(b"MAIN", &main));
    if modern {
        out.extend(chunk(b"MAID", &maid));
    }
    out
}
pub(crate) fn adt(height: f32) -> Vec<u8> {
    let mut out = chunk(b"MVER", &18u32.to_le_bytes());
    let mut cell = vec![0; 128];
    put(&mut cell, 52, 42);
    put(&mut cell, 112, height.to_bits());
    out.extend(chunk(b"MCNK", &cell));
    out
}
pub(crate) fn liquid_adt() -> Vec<u8> {
    let mut out = adt(1.0);
    let mut mh2o = vec![0; 3096];
    put(&mut mh2o, 0, 3072);
    put(&mut mh2o, 4, 1);
    mh2o[3072..3074].copy_from_slice(&22u16.to_le_bytes());
    put(&mut mh2o, 3076, 5f32.to_bits());
    put(&mut mh2o, 3080, 5f32.to_bits());
    mh2o[3086] = 8;
    mh2o[3087] = 8;
    out.extend(chunk(b"MH2O", &mh2o));
    out
}
pub(crate) fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|v| v.to_le_bytes()).collect()
}
pub(crate) fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}
const MAP: &[(u16, u16)] = &[
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
const TYPE: &[(u16, u16)] = &[
    (32, 1),
    (32, 6),
    (32, 1),
    (8, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (16, 1),
    (32, 1),
    (8, 1),
    (8, 1),
    (8, 1),
    (32, 1),
    (8, 6),
    (32, 3),
    (32, 38),
    (32, 4),
    (32, 4),
];
pub(crate) fn offset(fields: &[(u16, u16)], index: usize) -> usize {
    fields[..index]
        .iter()
        .map(|(b, n)| usize::from(b * n / 8))
        .sum()
}
pub(crate) fn size(fields: &[(u16, u16)]) -> usize {
    offset(fields, fields.len())
}
pub(crate) fn table(
    layout: u32,
    fields: &[(u16, u16)],
    rows: &[(u32, Vec<u8>)],
    strings: &[u8],
    copies: &[(u32, u32)],
) -> Vec<u8> {
    let start = 244 + fields.len() * 28;
    let mut bytes = vec![0; start];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, fields.len() as u32),
        (144, size(fields) as u32),
        (148, strings.len() as u32),
        (156, layout),
        (164, u32::MAX),
        (176, fields.len() as u32),
        (180, size(fields) as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
        (212, start as u32),
        (216, rows.len() as u32),
        (220, strings.len() as u32),
        (228, (rows.len() * 4) as u32),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (i, &(width, count)) in fields.iter().enumerate() {
        let primitive = 244 + i * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - width).to_le_bytes());
        bytes[primitive + 2..primitive + 4]
            .copy_from_slice(&(offset(fields, i) as u16).to_le_bytes());
        let col = 244 + fields.len() * 4 + i * 24;
        bytes[col..col + 2].copy_from_slice(&((offset(fields, i) * 8) as u16).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(width * count).to_le_bytes());
    }
    for (_, row) in rows {
        assert_eq!(row.len(), size(fields));
        bytes.extend(row);
    }
    bytes.extend(strings);
    for (id, _) in rows {
        bytes.extend(id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
}
pub(crate) fn map_db2(values: &[(u32, u32, i16)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut rows = Vec::new();
    let mut strings = Vec::new();
    for (ordinal, &(id, wdt, parent)) in values.iter().enumerate() {
        let mut row = vec![0; size(MAP)];
        for (field, text) in [(0, "Synthetic"), (1, "Name")] {
            let relative =
                (values.len() - ordinal) * size(MAP) + strings.len() - offset(MAP, field);
            put(&mut row, offset(MAP, field), relative as u32);
            strings.extend(text.as_bytes());
            strings.push(0);
        }
        put(&mut row, offset(MAP, 21), wdt);
        for field in [13, 14] {
            let at = offset(MAP, field);
            row[at..at + 2].copy_from_slice(&parent.to_le_bytes());
        }
        rows.push((id, row));
    }
    table(0xD43A_FAC3, MAP, &rows, &strings, copies)
}
pub(crate) fn material_db2(values: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, flags)| {
            let mut row = vec![0; 5];
            put(&mut row, 0, *flags);
            (*id, row)
        })
        .collect();
    table(0x98E5_D7AA, &[(32, 1), (8, 1)], &rows, &[], copies)
}
pub(crate) fn type_db2(values: &[(u32, u8)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, material)| {
            let mut row = vec![0; size(TYPE)];
            row[offset(TYPE, 14)] = *material;
            (*id, row)
        })
        .collect();
    table(0xD1EC_EEC9, TYPE, &rows, &[], copies)
}
