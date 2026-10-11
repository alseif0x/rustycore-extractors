// SPDX-License-Identifier: GPL-3.0-or-later
//! GT output verification: offline, read-only, bounded, no client/CASC access.
use super::{MAX_FILE_BYTES, catalog, catalog_manifest, manifest, parse};
use crate::sha256;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    Verified,
    Invocation,
    Input,
    Manifest,
    Incomplete,
    UnsafePath,
    Inventory,
    Numeric,
}
impl Code {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Invocation => "invalid_invocation",
            Self::Input => "input_io",
            Self::Manifest => "GT_MANIFEST_INVALID",
            Self::Incomplete => "INCOMPLETE_RUN",
            Self::UnsafePath => "UNSAFE_PATH",
            Self::Inventory => "INVENTORY_MISMATCH",
            Self::Numeric => "GT_PARSE_MISMATCH",
        }
    }
    pub const fn exit(self) -> u8 {
        match self {
            Self::Verified => 0,
            Self::Invocation | Self::Manifest => 2,
            Self::Input => 4,
            Self::Incomplete => 8,
            _ => 9,
        }
    }
}

pub struct Report {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub code: Code,
    pub build: Option<u32>,
    pub artifacts_verified: usize,
}
impl Report {
    pub const fn failure(code: Code) -> Self {
        Self {
            acquisition: None,
            code,
            build: None,
            artifacts_verified: 0,
        }
    }
    pub fn json(&self) -> String {
        let text = format!(
            "{{\"report_version\":1,\"scope\":\"forever_gt_verification\",\"status\":\"{}\",\"code\":\"{}\",\"exit_code\":{},\"build\":{},\"artifacts_verified\":{},\"offline\":true,\"read_only\":true,\"paths_redacted\":true,\"typed_loaders_executed\":false,\"full_client_extraction\":false}}",
            if self.code == Code::Verified {
                "verified"
            } else {
                "failed"
            },
            self.code.name(),
            self.code.exit(),
            self.build
                .map_or_else(|| "null".to_owned(), |value| value.to_string()),
            self.artifacts_verified
        );
        match self.acquisition {
            Some(profile) => text.replacen(
                "\"report_version\":1",
                &format!("\"report_version\":3,\"acquisition\":{}", profile.render()),
                1,
            ),
            None => text,
        }
    }
}

pub fn verify(root: &Path) -> Report {
    match run(root) {
        Ok(report) => report,
        Err(code) => Report::failure(code),
    }
}

fn run(root: &Path) -> Result<Report, Code> {
    if root
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(Code::UnsafePath);
    }
    directory(root)?;
    let names = children(root, 8)?;
    if names.contains(manifest::MARKER) || names.iter().any(|name| name.ends_with(".part")) {
        return Err(Code::Incomplete);
    }
    let text = read(
        root.join(manifest::NAME).as_path(),
        manifest::MAX_MANIFEST_BYTES,
    )?;
    let text = std::str::from_utf8(&text).map_err(|_| Code::Manifest)?;
    let decoded = match manifest::version(text).map_err(|_| Code::Manifest)? {
        1 => Decoded::Batch(manifest::decode(text).map_err(|_| Code::Manifest)?),
        2 => Decoded::Catalog(catalog_manifest::decode(text).map_err(|_| Code::Manifest)?),
        3 => match manifest::decode(text) {
            Ok(value) => Decoded::Batch(value),
            Err(()) => {
                Decoded::Catalog(catalog_manifest::decode(text).map_err(|_| Code::Manifest)?)
            }
        },
        _ => return Err(Code::Manifest),
    };
    if !decoded.complete() {
        return Err(Code::Incomplete);
    }
    if names
        != ["gt".to_owned(), manifest::NAME.to_owned()]
            .into_iter()
            .collect()
    {
        return Err(Code::Inventory);
    }
    let gt = root.join("gt");
    directory(&gt)?;
    let files = children(&gt, decoded.file_limit())?;
    if files.iter().any(|name| name.ends_with(".part")) {
        return Err(Code::Incomplete);
    }
    if files != decoded.names() {
        return Err(Code::Inventory);
    }
    decoded.verify_files(&gt)?;
    Ok(Report {
        acquisition: decoded.acquisition(),
        code: Code::Verified,
        build: Some(decoded.build()),
        artifacts_verified: decoded.names().len(),
    })
}

/// Version-specific schema validation precedes a shared output-tree policy.
enum Decoded {
    Batch(manifest::Manifest),
    Catalog(catalog_manifest::Manifest),
}
impl Decoded {
    fn acquisition(&self) -> Option<crate::acquisition::Profile> {
        match self {
            Self::Batch(value) => value.acquisition,
            Self::Catalog(value) => value.acquisition,
        }
    }
    fn complete(&self) -> bool {
        match self {
            Self::Batch(value) => value.complete(),
            Self::Catalog(value) => value.complete(),
        }
    }
    fn build(&self) -> u32 {
        match self {
            Self::Batch(value) => value.build,
            Self::Catalog(value) => value.build,
        }
    }
    fn file_limit(&self) -> usize {
        match self {
            Self::Batch(_) => 8,
            Self::Catalog(_) => catalog::MAX_FILES + 2,
        }
    }
    fn names(&self) -> BTreeSet<String> {
        match self {
            Self::Batch(value) => value
                .plan
                .tables()
                .iter()
                .map(|table| table.file_name().to_owned())
                .collect(),
            Self::Catalog(_) => catalog::Table::ALL
                .into_iter()
                .map(|table| table.file_name().to_owned())
                .collect(),
        }
    }
    fn verify_files(&self, gt: &Path) -> Result<(), Code> {
        match self {
            Self::Batch(decoded) => {
                for artifact in &decoded.artifacts {
                    let bytes = read(&gt.join(artifact.table.file_name()), MAX_FILE_BYTES)?;
                    let facts = artifact.facts.ok_or(Code::Manifest)?;
                    if bytes.len() != facts.bytes
                        || Some(sha256::hex(&bytes)).as_ref() != artifact.sha256.as_ref()
                    {
                        return Err(Code::Inventory);
                    }
                    let parsed = parse(&bytes, artifact.table).map_err(|_| Code::Numeric)?;
                    if parsed.facts() != facts {
                        return Err(Code::Numeric);
                    }
                }
            }
            Self::Catalog(decoded) => {
                for artifact in &decoded.artifacts {
                    let bytes = read(&gt.join(artifact.table.file_name()), MAX_FILE_BYTES)?;
                    if Some(bytes.len()) != artifact.bytes
                        || Some(sha256::hex(&bytes)).as_ref() != artifact.sha256.as_ref()
                    {
                        return Err(Code::Inventory);
                    }
                    if artifact.table.value_columns().is_some() {
                        let parsed = catalog::parse_numeric(&bytes, artifact.table)
                            .map_err(|_| Code::Numeric)?;
                        if Some(parsed.facts()) != artifact.facts {
                            return Err(Code::Numeric);
                        }
                    } else if artifact.facts.is_some() {
                        return Err(Code::Manifest);
                    }
                }
            }
        }
        Ok(())
    }
}

fn directory(path: &Path) -> Result<(), Code> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Code::Input)?;
    if metadata.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !metadata.is_dir() {
        return Err(Code::Input);
    }
    Ok(())
}

fn children(path: &Path, limit: usize) -> Result<BTreeSet<String>, Code> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(path).map_err(|_| Code::Input)? {
        let entry = entry.map_err(|_| Code::Input)?;
        if entry.file_type().map_err(|_| Code::Input)?.is_symlink() {
            return Err(Code::UnsafePath);
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Code::UnsafePath)?;
        names.insert(name);
        // Fixed plan bound, with room to classify pending files before inventory.
        if names.len() > limit {
            return Err(Code::Inventory);
        }
    }
    Ok(names)
}

fn read(path: &Path, max: usize) -> Result<Vec<u8>, Code> {
    let metadata = fs::symlink_metadata(path).map_err(|_| Code::Input)?;
    if metadata.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !metadata.is_file() || metadata.len() > max as u64 {
        return Err(Code::Input);
    }
    let file = File::open(path).map_err(|_| Code::Input)?;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Code::Input)?;
    if bytes.len() > max || bytes.len() as u64 != metadata.len() {
        return Err(Code::Input);
    }
    Ok(bytes)
}
