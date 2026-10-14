// SPDX-License-Identifier: GPL-3.0-or-later
//! Camera component identity, separate from physical acquisition profiles.
use crate::verify::json::{self, Json};
use forever_db2::catalog;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraContract {
    #[default]
    Legacy,
    Build70338V1,
}
impl CameraContract {
    pub const PROFILE: &str = "camera-raw-70338-v1";
    pub const TABLE_HASH: u32 = 0xC2CE_FDF9;
    pub const DEFINITION_PATH: &str = "definitions/CinematicCamera.dbd";
    pub const DEFINITION_BLOB: &str = "3676f4e583d5c53c4eda065db9c322f9d45921f0";
    pub const DEFINITION_SHA256: &str =
        "21f07cf5a2cdba30bec06895cb68e7467b58ab20cbee474f256473d2f70ca9d6";

    /// Validate an explicit source build before filesystem/callback access.
    pub fn admit_build(self, build: u32) -> Result<(), &'static str> {
        let accepted = match self {
            Self::Legacy => matches!(build, 70170 | 70205),
            Self::Build70338V1 => build == 70338,
        };
        if accepted {
            Ok(())
        } else {
            Err("unsupported_client")
        }
    }
}

/// Closed public target facts, not acquired client paths or camera IDs.
pub(super) fn catalog_json() -> String {
    format!(
        concat!(
            "{{\"id\":\"{}\",\"source_commit\":\"{}\",\"source_manifest_blob\":\"{}\",\"source_manifest_sha256\":\"{}\",\"facts_sha256\":\"{}\",",
            "\"table_hash\":{},\"layout_hash\":{},\"file_data_id\":{},\"physical_field\":{},\"field_name\":\"FileDataID\",\"field_type\":\"u32\",\"file_field_count\":5,\"decoded_scalar_count\":7,\"external_id\":true,",
            "\"definition_path\":\"{}\",\"definition_blob\":\"{}\",\"definition_sha256\":\"{}\"}}"
        ),
        catalog::WOWDBDEFS_70338_CATALOG_ID,
        catalog::WOWDBDEFS_70338_SOURCE_COMMIT,
        catalog::WOWDBDEFS_70338_SOURCE_BLOB,
        catalog::WOWDBDEFS_70338_SOURCE_SHA256,
        catalog::WOWDBDEFS_70338_FACTS_SHA256,
        CameraContract::TABLE_HASH,
        super::SOURCE_LAYOUT,
        super::SOURCE_FILE_ID,
        super::FILE_ID_FIELD,
        CameraContract::DEFINITION_PATH,
        CameraContract::DEFINITION_BLOB,
        CameraContract::DEFINITION_SHA256
    )
}
pub(super) fn validate_catalog(value: &Json) -> Result<(), ()> {
    let Json::Object(expected) = json::parse(&catalog_json())? else {
        return Err(());
    };
    let Json::Object(actual) = value else {
        return Err(());
    };
    if actual.len() != expected.len()
        || expected.iter().any(|(key, value)| {
            actual.iter().filter(|(k, _)| k == key).count() != 1
                || !actual.iter().any(|(k, v)| k == key && v == value)
        })
    {
        return Err(());
    }
    Ok(())
}
