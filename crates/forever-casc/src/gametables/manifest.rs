// SPDX-License-Identifier: GPL-3.0-or-later
//! Separate GT v1 manifest. Numeric cells and client paths never enter JSON.
use super::{
    Facts, GameTable, MAX_DATA_ROWS, MAX_FILE_BYTES, MAX_VALUES,
    batch::{MAX_BATCH_BYTES, Plan},
};
use crate::{
    Locale, sha256,
    verify::json::{self, Json},
};

pub const NAME: &str = "forever-gt-manifest.json";
pub const MARKER: &str = ".rustycore-forever-gt-incomplete.json";
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const ORACLE: &str = "02245dcd245e7433e524577656177723d3e4992e";
pub const NUMERIC_POLICY: &str = "finite_decimal_f64_to_f32_v1";

pub(super) struct Artifact {
    pub table: GameTable,
    pub sha256: Option<String>,
    pub facts: Option<Facts>,
    pub error: Option<String>,
}

impl Artifact {
    pub fn complete(table: GameTable, bytes: &[u8], facts: Facts) -> Self {
        Self {
            table,
            sha256: Some(sha256::hex(bytes)),
            facts: Some(facts),
            error: None,
        }
    }
    pub fn failed(table: GameTable, code: &'static str) -> Self {
        Self {
            table,
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
    pub plan: Plan,
    pub artifacts: Vec<Artifact>,
}

impl Manifest {
    pub fn complete(&self) -> bool {
        self.artifacts
            .iter()
            .all(|artifact| artifact.error.is_none())
    }

    pub fn inventory(&self) -> (usize, usize, String) {
        let mut lines = Vec::new();
        let mut bytes = 0;
        for artifact in &self.artifacts {
            if let (Some(hash), Some(facts)) = (&artifact.sha256, artifact.facts) {
                bytes += facts.bytes;
                lines.push((
                    artifact.table.relative_path(),
                    format!("{hash}  {}\n", artifact.table.relative_path()),
                ));
            }
        }
        lines.sort_by_key(|(path, _)| *path);
        let count = lines.len();
        let inventory: String = lines.into_iter().map(|(_, line)| line).collect();
        (count, bytes, sha256::hex(inventory.as_bytes()))
    }

    pub fn render(&self) -> String {
        let (count, bytes, hash) = self.inventory();
        let mut artifacts = Vec::new();
        for artifact in &self.artifacts {
            let table = artifact.table;
            let outcome = match artifact.facts {
                Some(facts) => format!(
                    "\"status\":\"complete\",\"bytes\":{},\"sha256\":\"{}\",\"data_rows\":{},\"consumer_rows_including_zero\":{},\"terminated_early\":{},\"error_code\":null",
                    facts.bytes,
                    artifact.sha256.as_deref().unwrap_or(""),
                    facts.data_rows,
                    facts.consumer_rows_including_zero,
                    facts.terminated_early
                ),
                None => format!(
                    "\"status\":\"error\",\"bytes\":null,\"sha256\":null,\"data_rows\":null,\"consumer_rows_including_zero\":null,\"terminated_early\":null,\"error_code\":\"{}\"",
                    artifact.error.as_deref().unwrap_or("internal")
                ),
            };
            artifacts.push(format!("{{\"name\":\"{}\",\"file_id\":{},\"relative_path\":\"{}\",\"value_columns\":{},{outcome}}}", table.name(), table.file_id(), table.relative_path(), table.value_columns()));
        }
        let text = format!(
            concat!(
                "{{\"manifest_version\":1,\"scope\":\"explicit_gt_batch\",\"status\":\"{}\",",
                "\"product\":\"wow_classic_beta\",\"build\":{},\"locale\":\"{}\",",
                "\"extractor\":\"forever-gt-extract\",\"extractor_version\":\"{}\",\"oracle_commit\":\"{}\",",
                "\"plan\":\"{}\",\"numeric_policy\":\"{}\",",
                "\"max_file_bytes\":{},\"max_batch_bytes\":{},",
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
            self.plan.name(),
            NUMERIC_POLICY,
            MAX_FILE_BYTES,
            MAX_BATCH_BYTES,
            artifacts.join(","),
            count,
            bytes,
            hash
        );
        match self.acquisition {
            Some(profile) => text.replacen(
                "\"manifest_version\":1",
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

/// Strict schema validation with bounded JSON recursion before the shared
/// parser runs. Key order/whitespace do not affect acceptance; unknown or
/// duplicate keys, noninteger counts and unknown artifact paths fail closed.
pub(super) fn decode(text: &str) -> Result<Manifest, ()> {
    depth_guard(text)?;
    let mut value = json::parse(text)?;
    let acquisition = match number(field(&value, "manifest_version")?)? {
        1 => None,
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
            version.1 = Json::Number(1);
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
            "plan",
            "numeric_policy",
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
    equal_number(&value, "manifest_version", 1)?;
    equal_string(&value, "scope", "explicit_gt_batch")?;
    equal_string(&value, "product", "wow_classic_beta")?;
    equal_string(&value, "extractor", "forever-gt-extract")?;
    equal_string(&value, "oracle_commit", ORACLE)?;
    equal_string(&value, "numeric_policy", NUMERIC_POLICY)?;
    equal_number(&value, "max_file_bytes", MAX_FILE_BYTES)?;
    equal_number(&value, "max_batch_bytes", MAX_BATCH_BYTES)?;
    equal_number(&value, "downloaded_bytes", 0)?;
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
    if version.len() > 64
        || version.is_empty()
        || !version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-+".contains(&byte))
    {
        return Err(());
    }
    let build = u32::try_from(number(field(&value, "build")?)?).map_err(|_| ())?;
    super::contract::CatalogContract::Legacy
        .admit_build(build)
        .map_err(|_| ())?;
    let locale = Locale::parse(string(field(&value, "locale")?)?).map_err(|_| ())?;
    let plan = Plan::named(string(field(&value, "plan")?)?).ok_or(())?;
    let Json::Array(values) = field(&value, "artifacts")? else {
        return Err(());
    };
    let tables = plan.tables();
    if values.len() != tables.len() {
        return Err(());
    }
    let mut artifacts = Vec::new();
    for (item, table) in values.iter().zip(tables) {
        object(
            item,
            &[
                "name",
                "file_id",
                "relative_path",
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
        equal_number(item, "value_columns", table.value_columns())?;
        let artifact = match string(field(item, "status")?)? {
            "complete" => {
                if !matches!(field(item, "error_code")?, Json::Null) {
                    return Err(());
                }
                let hash = string(field(item, "sha256")?)?;
                if hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(());
                }
                let bytes = number(field(item, "bytes")?)?;
                let data_rows = number(field(item, "data_rows")?)?;
                let rows = number(field(item, "consumer_rows_including_zero")?)?;
                if bytes > file_cap
                    || data_rows > MAX_DATA_ROWS
                    || data_rows.checked_add(1) != Some(rows)
                    || rows.checked_mul(table.value_columns()).ok_or(())? > MAX_VALUES
                {
                    return Err(());
                }
                Artifact {
                    table,
                    sha256: Some(hash.to_owned()),
                    facts: Some(Facts {
                        bytes,
                        value_columns: table.value_columns(),
                        data_rows,
                        consumer_rows_including_zero: rows,
                        terminated_early: boolean(field(item, "terminated_early")?)?,
                    }),
                    error: None,
                }
            }
            "error" => {
                for key in [
                    "bytes",
                    "sha256",
                    "data_rows",
                    "consumer_rows_including_zero",
                    "terminated_early",
                ] {
                    if !matches!(field(item, key)?, Json::Null) {
                        return Err(());
                    }
                }
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
        plan,
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

pub(super) fn object(value: &Json, keys: &[&str]) -> Result<(), ()> {
    let Json::Object(entries) = value else {
        return Err(());
    };
    if entries.len() != keys.len() || entries.iter().any(|(key, _)| !keys.contains(&key.as_str())) {
        return Err(());
    }
    Ok(())
}
pub(super) fn field<'a>(value: &'a Json, key: &str) -> Result<&'a Json, ()> {
    let Json::Object(entries) = value else {
        return Err(());
    };
    entries
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
        .ok_or(())
}
pub(super) fn string(value: &Json) -> Result<&str, ()> {
    if let Json::String(value) = value {
        Ok(value)
    } else {
        Err(())
    }
}
pub(super) fn number(value: &Json) -> Result<usize, ()> {
    if let Json::Number(value) = value {
        usize::try_from(*value).map_err(|_| ())
    } else {
        Err(())
    }
}
pub(super) fn boolean(value: &Json) -> Result<bool, ()> {
    if let Json::Bool(value) = value {
        Ok(*value)
    } else {
        Err(())
    }
}
pub(super) fn equal_string(value: &Json, key: &str, expected: &str) -> Result<(), ()> {
    if string(field(value, key)?)? == expected {
        Ok(())
    } else {
        Err(())
    }
}
pub(super) fn equal_number(value: &Json, key: &str, expected: usize) -> Result<(), ()> {
    if number(field(value, key)?)? == expected {
        Ok(())
    } else {
        Err(())
    }
}
pub(super) fn depth_guard(text: &str) -> Result<(), ()> {
    if text.len() > MAX_MANIFEST_BYTES {
        return Err(());
    }
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for byte in text.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 8 {
                        return Err(());
                    }
                }
                b'}' | b']' => depth = depth.checked_sub(1).ok_or(())?,
                _ => {}
            }
        }
    }
    if depth != 0 || quoted {
        return Err(());
    }
    Ok(())
}

/// Version dispatch does not relax either manifest's exact schema.
pub(super) fn version(text: &str) -> Result<usize, ()> {
    depth_guard(text)?;
    let value = json::parse(text)?;
    number(field(&value, "manifest_version")?)
}
