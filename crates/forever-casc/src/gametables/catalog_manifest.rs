// SPDX-License-Identifier: GPL-3.0-or-later
//! Catalog v2 manifest, separate from the unchanged six-table v1 format.
use super::{
    Facts, MAX_DATA_ROWS, MAX_FILE_BYTES, MAX_VALUES,
    catalog::{self, MAX_BATCH_BYTES, MAX_FILES, Table},
    manifest::{
        self, NUMERIC_POLICY, ORACLE, boolean, equal_number, equal_string, field, number, object,
        string,
    },
};
use crate::{
    Locale, sha256,
    verify::json::{self, Json},
};

pub(super) struct Artifact {
    pub table: Table,
    pub bytes: Option<usize>,
    pub sha256: Option<String>,
    pub facts: Option<Facts>,
    pub error: Option<String>,
}
impl Artifact {
    pub fn complete(table: Table, bytes: &[u8], facts: Option<Facts>) -> Self {
        Self {
            table,
            bytes: Some(bytes.len()),
            sha256: Some(sha256::hex(bytes)),
            facts,
            error: None,
        }
    }
    pub fn failed(table: Table, code: &'static str) -> Self {
        Self {
            table,
            bytes: None,
            sha256: None,
            facts: None,
            error: Some(code.to_owned()),
        }
    }
}

pub(super) struct Manifest {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub build: u32,
    pub locale: Locale,
    pub artifacts: Vec<Artifact>,
}
impl Manifest {
    pub fn complete(&self) -> bool {
        self.artifacts.len() == MAX_FILES
            && self
                .artifacts
                .iter()
                .all(|artifact| artifact.error.is_none())
    }
    pub fn inventory(&self) -> (usize, usize, String) {
        let mut lines = Vec::new();
        let mut total = 0;
        for artifact in &self.artifacts {
            if let (Some(bytes), Some(hash)) = (artifact.bytes, &artifact.sha256) {
                total += bytes;
                lines.push((
                    artifact.table.relative_path(),
                    format!("{hash}  {}\n", artifact.table.relative_path()),
                ));
            }
        }
        lines.sort_by_key(|(path, _)| *path);
        let count = lines.len();
        let inventory: String = lines.into_iter().map(|(_, line)| line).collect();
        (count, total, sha256::hex(inventory.as_bytes()))
    }
    pub fn render(&self) -> String {
        let (count, total, hash) = self.inventory();
        let artifacts = self.artifacts.iter().map(|artifact| {
            let table = artifact.table;
            let columns = table.value_columns().map_or_else(|| "null".to_owned(), |value| value.to_string());
            let numeric = match artifact.facts {
                Some(facts) => format!("\"data_rows\":{},\"consumer_rows_including_zero\":{},\"terminated_early\":{}", facts.data_rows, facts.consumer_rows_including_zero, facts.terminated_early),
                None => "\"data_rows\":null,\"consumer_rows_including_zero\":null,\"terminated_early\":null".to_owned(),
            };
            let outcome = match artifact.bytes {
                Some(bytes) => format!("\"status\":\"complete\",\"bytes\":{bytes},\"sha256\":\"{}\",{numeric},\"error_code\":null", artifact.sha256.as_deref().unwrap_or("")),
                None => format!("\"status\":\"error\",\"bytes\":null,\"sha256\":null,{numeric},\"error_code\":\"{}\"", artifact.error.as_deref().unwrap_or("internal")),
            };
            format!("{{\"name\":\"{}\",\"file_id\":{},\"relative_path\":\"{}\",\"validation_mode\":\"{}\",\"value_columns\":{columns},{outcome}}}", table.name(), table.file_id(), table.relative_path(), table.validation_mode())
        }).collect::<Vec<_>>().join(",");
        let text = format!(
            concat!(
                "{{\"manifest_version\":2,\"scope\":\"pinned_gt_catalog\",\"status\":\"{}\",",
                "\"product\":\"wow_classic_beta\",\"build\":{},\"locale\":\"{}\",",
                "\"extractor\":\"forever-gt-extract\",\"extractor_version\":\"{}\",\"oracle_commit\":\"{}\",",
                "\"catalog_sha256\":\"{}\",\"plan\":\"catalog\",\"numeric_policy\":\"{}\",",
                "\"max_files\":{},\"max_file_bytes\":{},\"max_batch_bytes\":{},",
                "\"offline\":true,\"downloaded_bytes\":0,\"paths_redacted\":true,",
                "\"full_client_extraction\":false,\"typed_loaders_executed\":false,",
                "\"artifacts\":[{}],\"integrity\":{{\"file_count\":{},\"total_bytes\":{},\"inventory_sha256\":\"{}\"}}}}\n"
            ),
            if self.complete() {
                "complete"
            } else {
                "partial"
            },
            self.build,
            self.locale.name(),
            env!("CARGO_PKG_VERSION"),
            ORACLE,
            catalog::fingerprint(),
            NUMERIC_POLICY,
            MAX_FILES,
            MAX_FILE_BYTES,
            MAX_BATCH_BYTES,
            artifacts,
            count,
            total,
            hash
        );
        match self.acquisition {
            Some(profile) => text.replacen(
                "\"manifest_version\":2",
                &format!(
                    "\"manifest_version\":3,\"acquisition\":{}",
                    profile.render()
                ),
                1,
            ),
            None => text,
        }
    }
}

pub(super) fn decode(text: &str) -> Result<Manifest, ()> {
    manifest::depth_guard(text)?;
    let mut value = json::parse(text)?;
    let acquisition = match number(field(&value, "manifest_version")?)? {
        2 => None,
        3 => {
            let profile = crate::acquisition::Profile::decode(
                field(&value, "acquisition")?,
                MAX_FILE_BYTES as u32,
            )?;
            let Json::Object(entries) = &mut value else {
                return Err(());
            };
            entries.retain(|(key, _)| key != "acquisition");
            let version = entries
                .iter_mut()
                .find(|(key, _)| key == "manifest_version")
                .ok_or(())?;
            version.1 = Json::Number(2);
            Some(profile)
        }
        _ => return Err(()),
    };
    let file_cap = acquisition.map_or(MAX_FILE_BYTES, |profile| profile.max_file_bytes() as usize);
    object(
        &value,
        &[
            "manifest_version",
            "scope",
            "status",
            "product",
            "build",
            "locale",
            "extractor",
            "extractor_version",
            "oracle_commit",
            "catalog_sha256",
            "plan",
            "numeric_policy",
            "max_files",
            "max_file_bytes",
            "max_batch_bytes",
            "offline",
            "downloaded_bytes",
            "paths_redacted",
            "full_client_extraction",
            "typed_loaders_executed",
            "artifacts",
            "integrity",
        ],
    )?;
    equal_number(&value, "manifest_version", 2)?;
    for (key, expected) in [
        ("scope", "pinned_gt_catalog"),
        ("product", "wow_classic_beta"),
        ("extractor", "forever-gt-extract"),
        ("oracle_commit", ORACLE),
        ("plan", "catalog"),
        ("numeric_policy", NUMERIC_POLICY),
    ] {
        equal_string(&value, key, expected)?;
    }
    equal_string(&value, "catalog_sha256", &catalog::fingerprint())?;
    for (key, expected) in [
        ("max_files", MAX_FILES),
        ("max_file_bytes", MAX_FILE_BYTES),
        ("max_batch_bytes", MAX_BATCH_BYTES),
        ("downloaded_bytes", 0),
    ] {
        equal_number(&value, key, expected)?;
    }
    for (key, expected) in [
        ("offline", true),
        ("paths_redacted", true),
        ("full_client_extraction", false),
        ("typed_loaders_executed", false),
    ] {
        if boolean(field(&value, key)?)? != expected {
            return Err(());
        }
    }
    let version = string(field(&value, "extractor_version")?)?;
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
    {
        return Err(());
    }
    let build = u32::try_from(number(field(&value, "build")?)?).map_err(|_| ())?;
    if !crate::SUPPORTED_CLIENTS.iter().any(|(_, id)| *id == build) {
        return Err(());
    }
    let locale = Locale::parse(string(field(&value, "locale")?)?).map_err(|_| ())?;
    let Json::Array(items) = field(&value, "artifacts")? else {
        return Err(());
    };
    if items.len() != MAX_FILES {
        return Err(());
    }
    let mut artifacts = Vec::with_capacity(MAX_FILES);
    for (item, table) in items.iter().zip(Table::ALL) {
        object(
            item,
            &[
                "name",
                "file_id",
                "relative_path",
                "validation_mode",
                "value_columns",
                "status",
                "bytes",
                "sha256",
                "data_rows",
                "consumer_rows_including_zero",
                "terminated_early",
                "error_code",
            ],
        )?;
        equal_string(item, "name", table.name())?;
        equal_number(item, "file_id", table.file_id() as usize)?;
        equal_string(item, "relative_path", table.relative_path())?;
        equal_string(item, "validation_mode", table.validation_mode())?;
        match table.value_columns() {
            Some(columns) => equal_number(item, "value_columns", columns)?,
            None => nulls(item, &["value_columns"])?,
        }
        let artifact = match string(field(item, "status")?)? {
            "complete" => {
                nulls(item, &["error_code"])?;
                let hash = string(field(item, "sha256")?)?;
                if hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(());
                }
                let bytes = number(field(item, "bytes")?)?;
                if bytes > file_cap {
                    return Err(());
                }
                let facts = match table.value_columns() {
                    Some(columns) => {
                        let data_rows = number(field(item, "data_rows")?)?;
                        let rows = number(field(item, "consumer_rows_including_zero")?)?;
                        if data_rows > MAX_DATA_ROWS
                            || data_rows.checked_add(1) != Some(rows)
                            || rows.checked_mul(columns).ok_or(())? > MAX_VALUES
                        {
                            return Err(());
                        }
                        Some(Facts {
                            bytes,
                            value_columns: columns,
                            data_rows,
                            consumer_rows_including_zero: rows,
                            terminated_early: boolean(field(item, "terminated_early")?)?,
                        })
                    }
                    None => {
                        nulls(
                            item,
                            &[
                                "data_rows",
                                "consumer_rows_including_zero",
                                "terminated_early",
                            ],
                        )?;
                        None
                    }
                };
                Artifact {
                    table,
                    bytes: Some(bytes),
                    sha256: Some(hash.to_owned()),
                    facts,
                    error: None,
                }
            }
            "error" => {
                nulls(
                    item,
                    &[
                        "bytes",
                        "sha256",
                        "data_rows",
                        "consumer_rows_including_zero",
                        "terminated_early",
                    ],
                )?;
                let code = string(field(item, "error_code")?)?;
                if code.is_empty()
                    || code.len() > 80
                    || !code.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
                {
                    return Err(());
                }
                Artifact {
                    table,
                    bytes: None,
                    sha256: None,
                    facts: None,
                    error: Some(code.to_owned()),
                }
            }
            _ => return Err(()),
        };
        artifacts.push(artifact);
    }
    let manifest = Manifest {
        acquisition,
        build,
        locale,
        artifacts,
    };
    equal_string(
        &value,
        "status",
        if manifest.complete() {
            "complete"
        } else {
            "partial"
        },
    )?;
    let integrity = field(&value, "integrity")?;
    object(
        integrity,
        &["file_count", "total_bytes", "inventory_sha256"],
    )?;
    let (count, bytes, hash) = manifest.inventory();
    if bytes > MAX_BATCH_BYTES {
        return Err(());
    }
    equal_number(integrity, "file_count", count)?;
    equal_number(integrity, "total_bytes", bytes)?;
    equal_string(integrity, "inventory_sha256", &hash)?;
    Ok(manifest)
}

fn nulls(value: &Json, keys: &[&str]) -> Result<(), ()> {
    for key in keys {
        if !matches!(field(value, key)?, Json::Null) {
            return Err(());
        }
    }
    Ok(())
}
