// SPDX-License-Identifier: GPL-3.0-or-later
//! Private camera manifest: complete source and exact discovered file set.
use super::{
    DISCOVERY_POLICY, DiscoveryFacts, MAX_FILE_BYTES, MAX_FILES, MAX_SOURCE_BYTES, MAX_TOTAL_BYTES,
    ORACLE, SOURCE_FILE_ID, SOURCE_LAYOUT, SOURCE_NAME, file_id_set_hash, relative_path,
    source_path,
};
use crate::{
    Locale, sha256,
    verify::json::{self, Json},
};
use std::collections::BTreeSet;

pub const NAME: &str = "forever-camera-manifest.json";
pub const MARKER: &str = ".rustycore-forever-camera-incomplete.json";
pub const MAX_MANIFEST_BYTES: usize = 2 * 1024 * 1024;

pub(super) struct FileOutcome {
    pub bytes: Option<usize>,
    pub sha256: Option<String>,
    pub error: Option<String>,
}
impl FileOutcome {
    pub fn complete(bytes: &[u8]) -> Self {
        Self {
            bytes: Some(bytes.len()),
            sha256: Some(sha256::hex(bytes)),
            error: None,
        }
    }
    pub fn failed(code: &'static str) -> Self {
        Self {
            bytes: None,
            sha256: None,
            error: Some(code.to_owned()),
        }
    }
    fn render(&self) -> String {
        match self.bytes {
            Some(bytes) => format!(
                "\"status\":\"complete\",\"bytes\":{bytes},\"sha256\":\"{}\",\"error_code\":null",
                self.sha256.as_deref().unwrap_or("")
            ),
            None => format!(
                "\"status\":\"error\",\"bytes\":null,\"sha256\":null,\"error_code\":\"{}\"",
                self.error.as_deref().unwrap_or("internal")
            ),
        }
    }
}
pub(super) struct Source {
    pub file: FileOutcome,
    pub discovery: Option<DiscoveryFacts>,
}
pub(super) struct Camera {
    pub file_id: u32,
    pub file: FileOutcome,
}
pub(super) struct Manifest {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub build: u32,
    pub locale: Locale,
    pub source: Source,
    pub cameras: Vec<Camera>,
}
impl Manifest {
    pub fn complete(&self) -> bool {
        self.source.discovery.is_some()
            && self.source.file.error.is_none()
            && self.cameras.iter().all(|entry| entry.file.error.is_none())
    }
    pub fn inventory(&self) -> Result<(usize, usize, String), &'static str> {
        let mut paths = Vec::new();
        let mut total = 0usize;
        if let (Some(bytes), Some(hash)) = (self.source.file.bytes, &self.source.file.sha256) {
            total = add_inventory_bytes(total, bytes)?;
            paths.push((source_path(self.locale), hash));
        }
        for entry in &self.cameras {
            if let (Some(bytes), Some(hash)) = (entry.file.bytes, &entry.file.sha256) {
                total = add_inventory_bytes(total, bytes)?;
                paths.push((relative_path(entry.file_id), hash));
            }
        }
        paths.sort_by(|left, right| left.0.cmp(&right.0));
        let count = paths.len();
        let inventory: String = paths
            .into_iter()
            .map(|(path, hash)| format!("{hash}  {path}\n"))
            .collect();
        Ok((count, total, sha256::hex(inventory.as_bytes())))
    }
    pub fn render(&self) -> Result<String, &'static str> {
        let (count, total, hash) = self.inventory()?;
        let discovery = match &self.source.discovery {
            Some(facts) => format!(
                "\"direct_records\":{},\"unique_file_ids\":{},\"file_id_set_sha256\":\"{}\"",
                facts.direct_records, facts.unique_file_ids, facts.file_id_set_sha256
            ),
            None => "\"direct_records\":null,\"unique_file_ids\":null,\"file_id_set_sha256\":null"
                .to_owned(),
        };
        let source = format!(
            "{{\"name\":\"{}\",\"file_id\":{},\"relative_path\":\"{}\",\"layout_hash\":{},{},{discovery}}}",
            SOURCE_NAME,
            SOURCE_FILE_ID,
            source_path(self.locale),
            SOURCE_LAYOUT,
            self.source.file.render()
        );
        let cameras = self
            .cameras
            .iter()
            .map(|entry| {
                format!(
                    "{{\"file_id\":{},\"relative_path\":\"{}\",{}}}",
                    entry.file_id,
                    relative_path(entry.file_id),
                    entry.file.render()
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let text = format!(
            concat!(
                "{{\"manifest_version\":1,\"scope\":\"complete_camera_files\",\"status\":\"{}\",",
                "\"product\":\"wow_classic_beta\",\"build\":{},\"locale\":\"{}\",\"extractor\":\"forever-camera-extract\",\"extractor_version\":\"{}\",",
                "\"oracle_commit\":\"{}\",\"db2_metadata_sha256\":\"{}\",\"discovery_policy\":\"{}\",",
                "\"max_source_bytes\":{},\"max_file_bytes\":{},\"max_files\":{},\"max_total_bytes\":{},",
                "\"offline\":true,\"downloaded_bytes\":0,\"paths_redacted\":true,\"full_client_extraction\":false,",
                "\"m2_parsed\":false,\"waypoints_generated\":false,\"runtime_accepted\":false,",
                "\"source\":{},\"cameras\":[{}],\"integrity\":{{\"file_count\":{},\"total_bytes\":{},\"inventory_sha256\":\"{}\"}}}}\n"
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
            forever_db2::catalog::SOURCE_SHA256,
            DISCOVERY_POLICY,
            MAX_SOURCE_BYTES,
            MAX_FILE_BYTES,
            MAX_FILES,
            MAX_TOTAL_BYTES,
            source,
            cameras,
            count,
            total,
            hash
        );
        Ok(match self.acquisition {
            Some(profile) => text.replacen(
                "\"manifest_version\":1",
                &format!(
                    "\"manifest_version\":2,\"acquisition\":{}",
                    profile.render()
                ),
                1,
            ),
            None => text,
        })
    }
}

fn add_inventory_bytes(total: usize, bytes: usize) -> Result<usize, &'static str> {
    total
        .checked_add(bytes)
        .filter(|total| *total <= MAX_TOTAL_BYTES)
        .ok_or("camera_policy_limit")
}

pub(super) fn decode(text: &str) -> Result<Manifest, ()> {
    depth_guard(text)?;
    let mut value = json::parse(text)?;
    let acquisition = match number(field(&value, "manifest_version")?)? {
        1 => None,
        2 => {
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
            "db2_metadata_sha256",
            "discovery_policy",
            "max_source_bytes",
            "max_file_bytes",
            "max_files",
            "max_total_bytes",
            "offline",
            "downloaded_bytes",
            "paths_redacted",
            "full_client_extraction",
            "m2_parsed",
            "waypoints_generated",
            "runtime_accepted",
            "source",
            "cameras",
            "integrity",
        ],
    )?;
    for (key, n) in [
        ("manifest_version", 1),
        ("max_source_bytes", MAX_SOURCE_BYTES),
        ("max_file_bytes", MAX_FILE_BYTES),
        ("max_files", MAX_FILES),
        ("max_total_bytes", MAX_TOTAL_BYTES),
        ("downloaded_bytes", 0),
    ] {
        equal_number(&value, key, n)?;
    }
    for (key, s) in [
        ("scope", "complete_camera_files"),
        ("product", "wow_classic_beta"),
        ("extractor", "forever-camera-extract"),
        ("oracle_commit", ORACLE),
        ("db2_metadata_sha256", forever_db2::catalog::SOURCE_SHA256),
        ("discovery_policy", DISCOVERY_POLICY),
    ] {
        equal_string(&value, key, s)?;
    }
    for (key, b) in [
        ("offline", true),
        ("paths_redacted", true),
        ("full_client_extraction", false),
        ("m2_parsed", false),
        ("waypoints_generated", false),
        ("runtime_accepted", false),
    ] {
        if boolean(field(&value, key)?)? != b {
            return Err(());
        }
    }
    let version = string(field(&value, "extractor_version")?)?;
    if version.is_empty()
        || version.len() > 64
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
    {
        return Err(());
    }
    let build = u32::try_from(number(field(&value, "build")?)?).map_err(|_| ())?;
    if !crate::SUPPORTED_CLIENTS.iter().any(|(_, id)| *id == build) {
        return Err(());
    }
    let locale = Locale::parse(string(field(&value, "locale")?)?).map_err(|_| ())?;
    let src = field(&value, "source")?;
    object(
        src,
        &[
            "name",
            "file_id",
            "relative_path",
            "layout_hash",
            "status",
            "bytes",
            "sha256",
            "error_code",
            "direct_records",
            "unique_file_ids",
            "file_id_set_sha256",
        ],
    )?;
    equal_string(src, "name", SOURCE_NAME)?;
    equal_number(src, "file_id", SOURCE_FILE_ID as usize)?;
    equal_string(src, "relative_path", &source_path(locale))?;
    equal_number(src, "layout_hash", SOURCE_LAYOUT as usize)?;
    let source_file = outcome(src, file_cap.min(MAX_SOURCE_BYTES))?;
    let discovery = if source_file.bytes.is_some() {
        let records = number(field(src, "direct_records")?)?;
        let unique = number(field(src, "unique_file_ids")?)?;
        if records > 100_000 || unique > MAX_FILES || unique > records {
            return Err(());
        }
        let hash = string(field(src, "file_id_set_sha256")?)?;
        valid_hash(hash)?;
        Some(DiscoveryFacts {
            direct_records: records as u32,
            unique_file_ids: unique,
            file_id_set_sha256: hash.to_owned(),
        })
    } else {
        nulls(
            src,
            &["direct_records", "unique_file_ids", "file_id_set_sha256"],
        )?;
        None
    };
    let Json::Array(items) = field(&value, "cameras")? else {
        return Err(());
    };
    if items.len() > MAX_FILES
        || items.len() != discovery.as_ref().map_or(0, |facts| facts.unique_file_ids)
    {
        return Err(());
    }
    let mut cameras = Vec::with_capacity(items.len());
    let mut ids = BTreeSet::new();
    let mut previous = None;
    for item in items {
        object(
            item,
            &[
                "file_id",
                "relative_path",
                "status",
                "bytes",
                "sha256",
                "error_code",
            ],
        )?;
        let id = u32::try_from(number(field(item, "file_id")?)?).map_err(|_| ())?;
        if previous.is_some_and(|last| id <= last) {
            return Err(());
        }
        previous = Some(id);
        ids.insert(id);
        equal_string(item, "relative_path", &relative_path(id))?;
        cameras.push(Camera {
            file_id: id,
            file: outcome(item, file_cap)?,
        });
    }
    if let Some(facts) = &discovery
        && facts.file_id_set_sha256 != file_id_set_hash(&ids)
    {
        return Err(());
    }
    let manifest = Manifest {
        acquisition,
        build,
        locale,
        source: Source {
            file: source_file,
            discovery,
        },
        cameras,
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
    let (count, total, hash) = manifest.inventory().map_err(|_| ())?;
    equal_number(integrity, "file_count", count)?;
    equal_number(integrity, "total_bytes", total)?;
    equal_string(integrity, "inventory_sha256", &hash)?;
    Ok(manifest)
}
fn outcome(value: &Json, max: usize) -> Result<FileOutcome, ()> {
    match string(field(value, "status")?)? {
        "complete" => {
            nulls(value, &["error_code"])?;
            let bytes = number(field(value, "bytes")?)?;
            if bytes > max {
                return Err(());
            }
            let hash = string(field(value, "sha256")?)?;
            valid_hash(hash)?;
            Ok(FileOutcome {
                bytes: Some(bytes),
                sha256: Some(hash.to_owned()),
                error: None,
            })
        }
        "error" => {
            nulls(value, &["bytes", "sha256"])?;
            let code = string(field(value, "error_code")?)?;
            if code.is_empty()
                || code.len() > 80
                || !code
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            {
                return Err(());
            }
            Ok(FileOutcome {
                bytes: None,
                sha256: None,
                error: Some(code.to_owned()),
            })
        }
        _ => Err(()),
    }
}
fn valid_hash(hash: &str) -> Result<(), ()> {
    if hash.len() == 64
        && hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        Ok(())
    } else {
        Err(())
    }
}
fn nulls(value: &Json, keys: &[&str]) -> Result<(), ()> {
    for key in keys {
        if !matches!(field(value, key)?, Json::Null) {
            return Err(());
        }
    }
    Ok(())
}

fn object(value: &Json, keys: &[&str]) -> Result<(), ()> {
    let Json::Object(entries) = value else {
        return Err(());
    };
    if entries.len() != keys.len() || entries.iter().any(|(key, _)| !keys.contains(&key.as_str())) {
        return Err(());
    }
    Ok(())
}
fn field<'a>(value: &'a Json, key: &str) -> Result<&'a Json, ()> {
    let Json::Object(entries) = value else {
        return Err(());
    };
    entries
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
        .ok_or(())
}
fn string(value: &Json) -> Result<&str, ()> {
    if let Json::String(value) = value {
        Ok(value)
    } else {
        Err(())
    }
}
fn number(value: &Json) -> Result<usize, ()> {
    if let Json::Number(value) = value {
        usize::try_from(*value).map_err(|_| ())
    } else {
        Err(())
    }
}
fn boolean(value: &Json) -> Result<bool, ()> {
    if let Json::Bool(value) = value {
        Ok(*value)
    } else {
        Err(())
    }
}
fn equal_string(value: &Json, key: &str, expected: &str) -> Result<(), ()> {
    if string(field(value, key)?)? == expected {
        Ok(())
    } else {
        Err(())
    }
}
fn equal_number(value: &Json, key: &str, expected: usize) -> Result<(), ()> {
    if number(field(value, key)?)? == expected {
        Ok(())
    } else {
        Err(())
    }
}
fn depth_guard(text: &str) -> Result<(), ()> {
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
