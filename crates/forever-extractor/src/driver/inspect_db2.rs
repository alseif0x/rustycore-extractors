// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded, Linux-only structural inspection of selected build-70450 DB2 files.
//! The report contains header/catalogue facts and reader status only; no
//! content bytes, values, hashes, index prefixes, keys or archive locations.

use crate::{Error, plan::Plan};
use forever_db2::{
    CharacterTable, DecodedTable, HEADER_SIZE, Header, Metadata, SECTION_SIZE, catalog,
};
use std::{
    collections::BTreeSet,
    fmt::Write as FmtWrite,
    fs,
    io::{Cursor, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const PROFILE: &str = "forever-db2-inspection-contract-v1";
const SPARSE_FLAG: u16 = 1;
const MAX_REPORT_BYTES: usize = 4 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub(crate) struct Summary {
    pub declaration_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SectionFacts {
    index: usize,
    numeric_type: u32,
    flags: u32,
    record_count: u32,
    field_count: u32,
    record_size: u32,
    string_block_size: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Classification {
    outcome: &'static str,
    reason_code: Option<&'static str>,
    reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileFacts {
    file_data_id: u32,
    table_name: &'static str,
    total_decoded_size_bytes: Option<usize>,
    header_magic: &'static str,
    header_version: Option<u32>,
    header_flags: Option<u16>,
    section_count: Option<u32>,
    record_count: Option<u32>,
    field_count: Option<u32>,
    total_field_count: Option<u32>,
    min_id: Option<u32>,
    max_id: Option<u32>,
    packed_data_offset: Option<u32>,
    parent_lookup_count: Option<u32>,
    record_size: Option<u32>,
    string_block_size: Option<u32>,
    column_meta_size: Option<u32>,
    common_data_size: Option<u32>,
    pallet_data_size: Option<u32>,
    compression_kinds: Vec<u32>,
    sections: Vec<SectionFacts>,
    classification: Classification,
}

pub(crate) fn run(
    plan: &Plan,
    client: &Path,
    output: &Path,
    file_data_ids: &[u32],
) -> Result<Summary, Error> {
    validate_request(plan, file_data_ids, false)?;
    let declarations = resolve_declarations(plan, file_data_ids)?;
    let output = super::probe_encoding::output_path(client, output)?;
    let mut store = super::probe_encoding::open_keyless_store_70450(plan, client)?;
    if store.build().build != 70450 {
        return Err("product_build_mismatch".into());
    }

    let mut files = Vec::new();
    files
        .try_reserve_exact(declarations.len())
        .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    for declaration in declarations {
        let facts =
            match store.read_file_data_id_with_limit(declaration.file_data_id, MAX_FILE_BYTES) {
                Ok(file) => inspect_bytes(
                    plan,
                    declaration.file_data_id,
                    declaration.name,
                    &file.bytes,
                ),
                Err(error) => unavailable(declaration.file_data_id, declaration.name, error.code()),
            };
        files.push(facts);
    }
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    let report = render_report(plan, &files)?;
    atomic_write(client, &output, report.as_bytes())?;
    Ok(Summary {
        declaration_count: files.len(),
    })
}

fn validate_request(plan: &Plan, ids: &[u32], keys_supplied: bool) -> Result<(), Error> {
    super::admit(plan)?;
    if plan.build != 70450 || plan.schema_catalog != crate::plan::SchemaCatalog::WowDbDefs70450V1 {
        return Err("product_db2_inspection_profile".into());
    }
    if keys_supplied {
        return Err("product_keys_forbidden".into());
    }
    if ids.is_empty()
        || ids.len() > super::probe_encoding::MAX_REQUESTED_IDS
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("product_db2_inspection_file_data_id".into());
    }
    Ok(())
}

fn resolve_declarations(
    plan: &Plan,
    ids: &[u32],
) -> Result<Vec<&'static catalog::TableMetadata>, Error> {
    let declarations =
        catalog::tables_for(plan.schema_catalog.id()).ok_or("product_db2_inspection_profile")?;
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(ids.len())
        .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    for &id in ids {
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.file_data_id == id)
            .ok_or("product_db2_inspection_file_data_id")?;
        selected.push(declaration);
    }
    Ok(selected)
}

fn inspect_bytes(
    plan: &Plan,
    file_data_id: u32,
    table_name: &'static str,
    bytes: &[u8],
) -> FileFacts {
    let table = CharacterTable::from_catalog_id(plan.schema_catalog.id(), table_name)
        .ok()
        .flatten();
    let parsed_header = bytes
        .get(..HEADER_SIZE)
        .and_then(|prefix| <&[u8; HEADER_SIZE]>::try_from(prefix).ok())
        .and_then(|header| Header::parse(header).ok());
    let metadata = Metadata::read(&mut Cursor::new(bytes)).ok();
    let header_magic = if bytes.get(..4) == Some(b"WDC5") {
        "WDC5"
    } else {
        "unknown"
    };
    let header_version = word(bytes, 4);
    let sections = match (&parsed_header, &metadata) {
        (Some(header), Some(metadata)) => metadata
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| SectionFacts {
                index,
                // Inspector-defined numeric variant code: 0 regular WDC5,
                // 1 sparse WDC5, derived from Header.Flags bit 0.
                numeric_type: u32::from(header.flags & SPARSE_FLAG != 0),
                // The only per-section flag exposed by Metadata is whether the
                // opaque TACT lookup declaration is nonzero; do not emit it.
                flags: u32::from(section.tact_declared),
                record_count: section.record_count,
                field_count: header.field_count,
                record_size: header.record_size,
                string_block_size: section.string_table_size,
            })
            .collect(),
        _ => Vec::new(),
    };
    let compression_kinds = parsed_header
        .as_ref()
        .map_or_else(Vec::new, |header| compression_kinds(bytes, header));
    let classification = classify(plan, table_name, table, parsed_header.as_ref(), bytes);
    FileFacts {
        file_data_id,
        table_name,
        total_decoded_size_bytes: Some(bytes.len()),
        header_magic,
        header_version,
        header_flags: parsed_header.map(|header| header.flags),
        section_count: parsed_header.map(|header| header.section_count),
        record_count: parsed_header.map(|header| header.record_count),
        field_count: parsed_header.map(|header| header.field_count),
        total_field_count: parsed_header.map(|header| header.total_field_count),
        min_id: parsed_header.map(|header| header.min_id),
        max_id: parsed_header.map(|header| header.max_id),
        packed_data_offset: parsed_header.map(|header| header.packed_data_offset),
        parent_lookup_count: parsed_header.map(|header| header.parent_lookup_count),
        record_size: parsed_header.map(|header| header.record_size),
        string_block_size: parsed_header.map(|header| header.string_table_size),
        column_meta_size: parsed_header.map(|header| header.column_meta_size),
        common_data_size: parsed_header.map(|header| header.common_data_size),
        pallet_data_size: parsed_header.map(|header| header.pallet_data_size),
        compression_kinds,
        sections,
        classification,
    }
}

fn classify(
    plan: &Plan,
    table_name: &str,
    table: Option<CharacterTable>,
    header: Option<&Header>,
    bytes: &[u8],
) -> Classification {
    let Some(table) = table else {
        return rejected(
            "unsupported_table_schema",
            "catalogue declaration did not resolve to a reviewed reader",
        );
    };
    let row_reader = DecodedTable::read(bytes, table);
    let parent_index = catalog::resolve_for(plan.schema_catalog.id(), table_name)
        .ok()
        .flatten()
        .map(|declaration| declaration.metadata().parent_index)
        .unwrap_or(-1);
    match super::validate_db2(plan, table_name, bytes) {
        Ok(()) => match row_reader {
            Ok(_) => Classification {
                outcome: "accepted",
                reason_code: None,
                reason: "complete build-specific DB2 validation succeeded",
            },
            Err(error) => Classification {
                outcome: "accepted_raw_only",
                reason_code: Some(error.code()),
                reason: "product source validation succeeded without materializing decoded rows",
            },
        },
        Err(error) => {
            let sparse_parent = error.code == "unsupported_record_variant"
                && header.is_some_and(|header| header.flags & SPARSE_FLAG != 0)
                && parent_index >= 0
                && plan.schema_catalog == crate::plan::SchemaCatalog::WowDbDefs70450V1;
            let reason = if sparse_parent && header.is_some_and(|header| header.section_count > 1) {
                "sparse variant has multiple sections, outside the bounded parented sparse layout"
            } else if sparse_parent
                && header.is_some_and(|header| {
                    header.string_table_size != 0
                        || header.common_data_size != 0
                        || header.pallet_data_size != 0
                })
            {
                "sparse parented layout declares unsupported string, common, or pallet data"
            } else if sparse_parent {
                "sparse parent field is outside the build-70450 raw-source allowlist"
            } else {
                reason_for(error.code)
            };
            rejected(error.code, reason)
        }
    }
}

fn rejected(code: &'static str, reason: &'static str) -> Classification {
    Classification {
        outcome: "rejected",
        reason_code: Some(code),
        reason,
    }
}

fn reason_for(code: &'static str) -> &'static str {
    match code {
        "unsupported_record_variant" => {
            "the bounded record reader does not support this physical DB2 variant"
        }
        "unsupported_table_schema" => {
            "the file header does not match its build-70450 catalogue schema"
        }
        "record_data_bounds" => "a declared DB2 range is outside the complete decoded file",
        "invalid_record_metadata" => "the DB2 header or section metadata is invalid",
        "invalid_column_data" => "column metadata is unsupported or malformed",
        "record_policy_limit" => "the bounded DB2 reader policy limit was exceeded",
        _ => "the build-specific DB2 reader rejected the complete decoded file",
    }
}

fn unavailable(file_data_id: u32, table_name: &'static str, code: &'static str) -> FileFacts {
    FileFacts {
        file_data_id,
        table_name,
        total_decoded_size_bytes: None,
        header_magic: "unavailable",
        header_version: None,
        header_flags: None,
        section_count: None,
        record_count: None,
        field_count: None,
        total_field_count: None,
        min_id: None,
        max_id: None,
        packed_data_offset: None,
        parent_lookup_count: None,
        record_size: None,
        string_block_size: None,
        column_meta_size: None,
        common_data_size: None,
        pallet_data_size: None,
        compression_kinds: Vec::new(),
        sections: Vec::new(),
        classification: Classification {
            outcome: "unavailable",
            reason_code: Some(code),
            reason: "LocalStore did not provide a complete decoded file",
        },
    }
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn compression_kinds(bytes: &[u8], header: &Header) -> Vec<u32> {
    if header.column_meta_size == 0 || !header.column_meta_size.is_multiple_of(24) {
        return Vec::new();
    }
    let Some(start) = HEADER_SIZE
        .checked_add((header.section_count as usize).saturating_mul(SECTION_SIZE))
        .and_then(|offset| offset.checked_add((header.field_count as usize).checked_mul(4)?))
    else {
        return Vec::new();
    };
    let count = header.column_meta_size as usize / 24;
    let Some(end) = start.checked_add(header.column_meta_size as usize) else {
        return Vec::new();
    };
    if end > bytes.len() {
        return Vec::new();
    }
    let mut kinds = BTreeSet::new();
    for index in 0..count {
        if let Some(kind) = word(bytes, start + index * 24 + 8) {
            kinds.insert(kind);
        }
    }
    kinds.into_iter().collect()
}

fn render_report(plan: &Plan, files: &[FileFacts]) -> Result<String, Error> {
    let mut output = String::new();
    output
        .try_reserve(MAX_REPORT_BYTES.min(files.len().saturating_mul(4096) + 128))
        .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    write!(
        output,
        "{{\"schema\":\"{PROFILE}\",\"build\":{},\"locale\":{},\"keyless\":true,\"extracts_nothing\":true,\"files\":[",
        plan.build,
        quote(plan.locale.name())?
    )
    .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    for (index, facts) in files.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        render_file(&mut output, facts)?;
        if output.len() > MAX_REPORT_BYTES {
            return Err("product_db2_inspection_limit".into());
        }
    }
    output.push_str("]}\n");
    if output.len() > MAX_REPORT_BYTES {
        return Err("product_db2_inspection_limit".into());
    }
    Ok(output)
}

fn render_file(output: &mut String, facts: &FileFacts) -> Result<(), Error> {
    write!(
        output,
        "{{\"file_data_id\":{},\"table_name\":{},\"total_decoded_size_bytes\":{},\"header_magic\":{},\"header_version\":{},\"header_flags\":{},\"section_count\":{},\"record_count\":{},\"field_count\":{},\"total_field_count\":{},\"min_id\":{},\"max_id\":{},\"packed_data_offset\":{},\"parent_lookup_count\":{},\"record_size\":{},\"string_block_size\":{},\"column_meta_size\":{},\"common_data_size\":{},\"pallet_data_size\":{},\"compression_kinds\":[",
        facts.file_data_id,
        quote(facts.table_name)?,
        optional_number(facts.total_decoded_size_bytes.map(|value| value as u64)),
        quote(facts.header_magic)?,
        optional_number(facts.header_version.map(u64::from)),
        optional_number(facts.header_flags.map(u64::from)),
        optional_number(facts.section_count.map(u64::from)),
        optional_number(facts.record_count.map(u64::from)),
        optional_number(facts.field_count.map(u64::from)),
        optional_number(facts.total_field_count.map(u64::from)),
        optional_number(facts.min_id.map(u64::from)),
        optional_number(facts.max_id.map(u64::from)),
        optional_number(facts.packed_data_offset.map(u64::from)),
        optional_number(facts.parent_lookup_count.map(u64::from)),
        optional_number(facts.record_size.map(u64::from)),
        optional_number(facts.string_block_size.map(u64::from)),
        optional_number(facts.column_meta_size.map(u64::from)),
        optional_number(facts.common_data_size.map(u64::from)),
        optional_number(facts.pallet_data_size.map(u64::from)),
    )
    .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    for (index, kind) in facts.compression_kinds.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(output, "{kind}").map_err(|_| Error::from("product_db2_inspection_limit"))?;
    }
    output.push_str("],\"sections\":[");
    for (index, section) in facts.sections.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{{\"index\":{},\"numeric_type\":{},\"flags\":{},\"record_count\":{},\"field_count\":{},\"record_size\":{},\"string_block_size\":{}}}",
            section.index,
            section.numeric_type,
            section.flags,
            section.record_count,
            section.field_count,
            section.record_size,
            section.string_block_size,
        )
        .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    }
    write!(
        output,
        "],\"classification\":{{\"outcome\":{},\"reason_code\":{},\"reason\":{}}}}}",
        quote(facts.classification.outcome)?,
        facts
            .classification
            .reason_code
            .map(quote)
            .transpose()?
            .unwrap_or_else(|| "null".to_owned()),
        quote(facts.classification.reason)?,
    )
    .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    Ok(())
}

fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |value| value.to_string())
}

fn quote(value: &str) -> Result<String, Error> {
    if value.len() > 1024 {
        return Err("product_db2_inspection_limit".into());
    }
    let mut output = String::new();
    output
        .try_reserve(value.len().saturating_add(2))
        .map_err(|_| Error::from("product_db2_inspection_limit"))?;
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character < ' ' => {
                write!(output, "\\u{:04x}", character as u32)
                    .map_err(|_| Error::from("product_db2_inspection_limit"))?;
            }
            character => output.push(character),
        }
    }
    output.push('"');
    Ok(output)
}

struct TemporaryOutput {
    path: PathBuf,
    cleanup: bool,
}
impl Drop for TemporaryOutput {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn atomic_write(client: &Path, requested: &Path, bytes: &[u8]) -> Result<(), Error> {
    let target = super::probe_encoding::output_path(client, requested)?;
    let parent = target
        .parent()
        .ok_or_else(|| Error::from("product_diagnostic_path"))?
        .to_path_buf();
    let mut temp = None;
    for _ in 0..32 {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".forever-db2-inspection-{}-{sequence}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                temp = Some((path, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("product_db2_inspection_output".into()),
        }
    }
    let Some((temp_path, mut file)) = temp else {
        return Err("product_db2_inspection_output".into());
    };
    let mut cleanup = TemporaryOutput {
        path: temp_path,
        cleanup: true,
    };
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| Error::from("product_db2_inspection_output"))?;
    drop(file);
    let target = super::probe_encoding::output_path(client, &target)?;
    fs::rename(&cleanup.path, &target).map_err(|_| Error::from("product_db2_inspection_output"))?;
    cleanup.cleanup = false;
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| Error::from("product_db2_inspection_output"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    const INSPECT_PLAN: &str =
        include_str!("../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
    const V10_PLAN: &str =
        include_str!("../../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv");
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Lab(PathBuf);
    impl Lab {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "rce-inspect-db2-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn put16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn put32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn synthetic_parent_sparse() -> Vec<u8> {
        let table = catalog::resolve_for(
            catalog::WOWDBDEFS_70450_CATALOG_ID,
            "CollectableSourceEncounterSparse",
        )
        .unwrap()
        .unwrap()
        .metadata();
        let mut bytes = vec![0; HEADER_SIZE + table.file_field_count * 4];
        bytes[..4].copy_from_slice(b"WDC5");
        put32(&mut bytes, 4, 5);
        put32(&mut bytes, 136, 0);
        put32(&mut bytes, 140, table.file_field_count as u32);
        put32(&mut bytes, 144, 8);
        put32(&mut bytes, 148, 0);
        put32(&mut bytes, 152, table.table_hash.unwrap());
        put32(&mut bytes, 156, table.layout_hash);
        put32(&mut bytes, 160, 0);
        put32(&mut bytes, 164, 0);
        put16(&mut bytes, 172, 0x7);
        put16(&mut bytes, 174, table.index as u16);
        put32(&mut bytes, 176, table.file_field_count as u32);
        put32(&mut bytes, 184, 0);
        put32(&mut bytes, 200, 0);
        // Synthetic field metadata only: signed 32-bit then signed 16-bit.
        put16(&mut bytes, HEADER_SIZE, 0);
        put16(&mut bytes, HEADER_SIZE + 4, 16);
        bytes
    }

    #[test]
    fn synthetic_sparse_parent_matches_real_shape_and_is_redacted_deterministic() {
        let plan = Plan::parse(INSPECT_PLAN).unwrap();
        let bytes = synthetic_parent_sparse();
        let first = inspect_bytes(&plan, 5_152_908, "CollectableSourceEncounterSparse", &bytes);
        let second = inspect_bytes(&plan, 5_152_908, "CollectableSourceEncounterSparse", &bytes);
        assert_eq!(first, second);
        assert_eq!(first.header_magic, "WDC5");
        assert_eq!(first.header_version, Some(5));
        assert_eq!(bytes.len(), 212);
        assert_eq!(first.header_flags, Some(0x7));
        assert_eq!(first.record_count, Some(0));
        assert_eq!(first.field_count, Some(2));
        assert_eq!(first.total_field_count, Some(2));
        assert_eq!(first.record_size, Some(8));
        assert_eq!(first.section_count, Some(0));
        assert!(first.sections.is_empty());
        assert_eq!(first.classification.outcome, "accepted_raw_only");
        assert_eq!(
            first.classification.reason_code,
            Some("unsupported_record_variant")
        );
        assert_eq!(
            first.classification.reason,
            "product source validation succeeded without materializing decoded rows"
        );
        let report = render_report(&plan, std::slice::from_ref(&first)).unwrap();
        let expected = concat!(
            "{\"schema\":\"forever-db2-inspection-contract-v1\",\"build\":70450,\"locale\":\"esES\",\"keyless\":true,\"extracts_nothing\":true,\"files\":[",
            "{\"file_data_id\":5152908,\"table_name\":\"CollectableSourceEncounterSparse\",\"total_decoded_size_bytes\":212,\"header_magic\":\"WDC5\",\"header_version\":5,\"header_flags\":7,\"section_count\":0,\"record_count\":0,\"field_count\":2,\"total_field_count\":2,\"min_id\":0,\"max_id\":0,\"packed_data_offset\":0,\"parent_lookup_count\":0,\"record_size\":8,\"string_block_size\":0,\"column_meta_size\":0,\"common_data_size\":0,\"pallet_data_size\":0,\"compression_kinds\":[],\"sections\":[],",
            "\"classification\":{\"outcome\":\"accepted_raw_only\",\"reason_code\":\"unsupported_record_variant\",\"reason\":\"product source validation succeeded without materializing decoded rows\"}}]}\n"
        );
        assert_eq!(report, expected);
        assert_eq!(report, render_report(&plan, &[second]).unwrap());
        for forbidden in [
            "content_key",
            "encoding_key",
            "payload",
            "record_values",
            "hash",
            "index_prefix",
        ] {
            assert!(!report.contains(forbidden), "report contains {forbidden}");
        }
    }

    #[test]
    fn synthetic_unsupported_sparse_common_data_is_reported_as_rejected() {
        let plan = Plan::parse(INSPECT_PLAN).unwrap();
        let mut bytes = synthetic_parent_sparse();
        put32(&mut bytes, 192, 1);
        let facts = inspect_bytes(&plan, 5_152_908, "CollectableSourceEncounterSparse", &bytes);
        assert_eq!(facts.header_flags, Some(0x7));
        assert_eq!(facts.section_count, Some(0));
        assert_eq!(facts.classification.outcome, "rejected");
        assert_eq!(
            facts.classification.reason_code,
            Some("unsupported_record_variant")
        );
        assert_eq!(
            facts.classification.reason,
            "sparse parented layout declares unsupported string, common, or pallet data"
        );
    }

    #[test]
    fn inspector_refuses_wrong_profile_keys_and_unbounded_or_unknown_ids() {
        let plan = Plan::parse(INSPECT_PLAN).unwrap();
        assert_eq!(
            validate_request(&Plan::parse(V10_PLAN).unwrap(), &[1], false)
                .unwrap_err()
                .code,
            "product_db2_inspection_profile"
        );
        assert_eq!(
            validate_request(&plan, &[5_152_908], true)
                .unwrap_err()
                .code,
            "product_keys_forbidden"
        );
        assert_eq!(
            validate_request(&plan, &[], false).unwrap_err().code,
            "product_db2_inspection_file_data_id"
        );
        assert!(resolve_declarations(&plan, &[u32::MAX]).is_err());
        assert!(validate_request(&plan, &[5_152_908, 5_152_908], false).is_err());
    }

    #[test]
    fn output_path_refuses_client_and_git_checkout_descendants() {
        let lab = Lab::new();
        let client = lab.0.join("client");
        fs::create_dir(&client).unwrap();
        fs::create_dir_all(client.join("reports")).unwrap();
        assert!(
            super::super::probe_encoding::output_path(&client, &client.join("reports/report.json"))
                .is_err()
        );
        let checkout = lab.0.join("repo");
        fs::create_dir_all(checkout.join(".git")).unwrap();
        fs::create_dir_all(checkout.join("reports")).unwrap();
        assert!(
            super::super::probe_encoding::output_path(
                &client,
                &checkout.join("reports/report.json")
            )
            .is_err()
        );
    }
}
