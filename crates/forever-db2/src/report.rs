// SPDX-License-Identifier: GPL-3.0-or-later
use forever_db2::Metadata;
use std::fmt::Write;

pub fn failure(code: &str, json: bool) {
    // All codes are controlled literals. Never interpolate a path/library error.
    if json {
        println!("{{\"report_version\":1,\"status\":\"error\",\"code\":\"{code}\"}}");
    } else {
        eprintln!("Inspection failed: {code}");
    }
}

pub fn success(metadata: &Metadata, json: bool) {
    if !json {
        println!(
            "WDC5/version 5: {} declared records, {} fields, {} sections.\n\
            Metadata readable only; client build, schema and record data NOT validated.",
            metadata.header.record_count,
            metadata.header.field_count,
            metadata.sections.len()
        );
        return;
    }
    let h = metadata.header;
    let mut output = format!(
        "{{\"report_version\":1,\"status\":\"success\",\"scope\":\"metadata_only\",\
        \"offline\":true,\"read_only\":true,\"client_build_verified\":false,\
        \"table_schema_validated\":false,\"record_data_validated\":false,\
        \"header\":{{\"magic\":\"WDC5\",\"version\":5,\"record_count\":{},\
        \"field_count\":{},\"record_size\":{},\"string_table_size\":{},\
        \"table_hash\":{},\"layout_hash\":{},\"min_id\":{},\"max_id\":{},\
        \"locale_mask\":{},\"flags\":{},\"index_field\":{},\"total_field_count\":{},\
        \"packed_data_offset\":{},\"parent_lookup_count\":{},\"column_meta_size\":{},\
        \"common_data_size\":{},\"pallet_data_size\":{},\"section_count\":{}}},\"sections\":[",
        h.record_count,
        h.field_count,
        h.record_size,
        h.string_table_size,
        h.table_hash,
        h.layout_hash,
        h.min_id,
        h.max_id,
        h.locale_mask,
        h.flags,
        h.index_field,
        h.total_field_count,
        h.packed_data_offset,
        h.parent_lookup_count,
        h.column_meta_size,
        h.common_data_size,
        h.pallet_data_size,
        h.section_count
    );
    for (index, s) in metadata.sections.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        write!(
            output,
            "{{\"tact_declared\":{},\"file_offset\":{},\"record_count\":{},\
            \"string_table_size\":{},\"catalog_data_offset\":{},\"id_table_size\":{},\
            \"parent_lookup_data_size\":{},\"catalog_data_count\":{},\"copy_table_count\":{}}}",
            s.tact_declared,
            s.file_offset,
            s.record_count,
            s.string_table_size,
            s.catalog_data_offset,
            s.id_table_size,
            s.parent_lookup_data_size,
            s.catalog_data_count,
            s.copy_table_count
        )
        .unwrap();
    }
    output.push_str("]}");
    println!("{output}");
}
