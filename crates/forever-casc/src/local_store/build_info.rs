// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original strict, owned metadata admission for local Rust IO.

use super::{AdmittedBuild, Error};
use crate::{PRODUCT, SUPPORTED_CLIENTS};

const COLUMNS: [&str; 6] = [
    "Product!STRING:0",
    "Active!DEC:1",
    "Version!STRING:0",
    "Build Key!HEX:16",
    "CDN Key!HEX:16",
    "Tags!STRING:0",
];

pub(super) fn parse(bytes: &[u8], max_rows: usize) -> Result<AdmittedBuild, Error> {
    parse_supported(bytes, max_rows, SUPPORTED_CLIENTS)
}

#[cfg(target_os = "linux")]
pub(super) fn parse_db2_70374(bytes: &[u8], max_rows: usize) -> Result<AdmittedBuild, Error> {
    parse_supported(bytes, max_rows, crate::admission::DB2_70374_CLIENTS)
}

#[cfg(target_os = "linux")]
pub(super) fn parse_db2_70425(bytes: &[u8], max_rows: usize) -> Result<AdmittedBuild, Error> {
    parse_supported(bytes, max_rows, crate::admission::DB2_70425_CLIENTS)
}

fn parse_supported(
    bytes: &[u8],
    max_rows: usize,
    supported: &'static [(&'static str, u32)],
) -> Result<AdmittedBuild, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| Error::BuildMetadata)?;
    if text
        .bytes()
        .any(|b| b.is_ascii_control() && !matches!(b, b'\r' | b'\n' | b'\t'))
    {
        return Err(Error::BuildMetadata);
    }
    let mut lines = text.lines();
    let mut fields = [""; 64];
    let count = columns(lines.next().ok_or(Error::BuildMetadata)?, &mut fields)?;
    let mut indexes = [0; 6];
    for (slot, required) in COLUMNS.iter().enumerate() {
        let base = required.split('!').next().ok_or(Error::BuildMetadata)?;
        let mut found = None;
        for (index, field) in fields[..count].iter().enumerate() {
            if field.split('!').next() == Some(base) {
                if found.is_some() || field != required {
                    return Err(Error::BuildMetadata);
                }
                found = Some(index);
            }
        }
        indexes[slot] = found.ok_or(Error::BuildMetadata)?;
    }
    let mut selected = None;
    let mut rows = 0_usize;
    for line in lines.filter(|line| !line.is_empty()) {
        rows = rows.checked_add(1).ok_or(Error::Limit)?;
        if rows > max_rows {
            return Err(Error::Limit);
        }
        let mut row = [""; 64];
        if columns(line, &mut row)? != count
            || row[indexes[0]].is_empty()
            || !matches!(row[indexes[1]], "0" | "1")
        {
            return Err(Error::BuildMetadata);
        }
        if row[indexes[0]] == PRODUCT && row[indexes[1]] == "1" {
            if selected.is_some() {
                return Err(Error::AmbiguousClient);
            }
            selected = Some((
                row[indexes[2]],
                row[indexes[3]],
                row[indexes[4]],
                row[indexes[5]],
            ));
        }
    }
    let (version, build_key, cdn_key, tags) = selected.ok_or(Error::UnsupportedClient)?;
    let (version, build) = supported
        .iter()
        .find(|(candidate, _)| *candidate == version)
        .ok_or(Error::UnsupportedClient)?;
    if tags.len() < 4 || !tags.is_ascii() {
        return Err(Error::BuildMetadata);
    }
    let mut owned_tags = String::new();
    owned_tags
        .try_reserve_exact(tags.len())
        .map_err(|_| Error::Limit)?;
    owned_tags.push_str(tags);
    Ok(AdmittedBuild {
        product: PRODUCT,
        version,
        build: *build,
        build_key: hash(build_key)?,
        cdn_key: hash(cdn_key)?,
        tags: owned_tags,
        config_build_uid: None,
        config_build_name: None,
    })
}

fn columns<'a>(line: &'a str, output: &mut [&'a str; 64]) -> Result<usize, Error> {
    let mut count = 0;
    for value in line.split('|') {
        if count == output.len() {
            return Err(Error::BuildMetadata);
        }
        output[count] = value;
        count += 1;
    }
    Ok(count)
}

fn hash(value: &str) -> Result<[u8; 16], Error> {
    if value.len() != 32 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::BuildMetadata);
    }
    let mut key = [0; 16];
    for (slot, pair) in key.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        let nibble = |b: u8| {
            if b.is_ascii_digit() {
                b - b'0'
            } else {
                b.to_ascii_lowercase() - b'a' + 10
            }
        };
        *slot = nibble(pair[0]) * 16 + nibble(pair[1]);
    }
    Ok(key)
}
