// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline output verification, including rediscovery from the preserved DB2.
use super::{
    CameraContract, MAX_FILE_BYTES, MAX_FILES, MAX_SOURCE_BYTES, SOURCE_NAME,
    discover_with_contract, file_name, manifest, source_path,
};
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
    Discovery,
}
impl Code {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Invocation => "invalid_invocation",
            Self::Input => "input_io",
            Self::Manifest => "CAMERA_MANIFEST_INVALID",
            Self::Incomplete => "INCOMPLETE_RUN",
            Self::UnsafePath => "UNSAFE_PATH",
            Self::Inventory => "INVENTORY_MISMATCH",
            Self::Discovery => "CAMERA_DISCOVERY_MISMATCH",
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
    pub contract: CameraContract,
    pub acquisition: Option<crate::acquisition::Profile>,
    pub code: Code,
    pub build: Option<u32>,
    pub cameras_verified: usize,
}
impl Report {
    pub const fn failure(code: Code) -> Self {
        Self {
            contract: CameraContract::Legacy,
            acquisition: None,
            code,
            build: None,
            cameras_verified: 0,
        }
    }
    pub fn json(&self) -> String {
        let text = format!(
            "{{\"report_version\":1,\"scope\":\"forever_camera_verification\",\"status\":\"{}\",\"code\":\"{}\",\"exit_code\":{},\"build\":{},\"cameras_verified\":{},\"offline\":true,\"read_only\":true,\"paths_redacted\":true,\"full_client_extraction\":false,\"m2_parsed\":false,\"waypoints_generated\":false,\"runtime_accepted\":false}}",
            if self.code == Code::Verified {
                "verified"
            } else {
                "failed"
            },
            self.code.name(),
            self.code.exit(),
            self.build
                .map_or_else(|| "null".to_owned(), |n| n.to_string()),
            self.cameras_verified
        );
        if self.contract == CameraContract::Build70338V1 {
            return text.replacen(
                "\"report_version\":1",
                &format!(
                    "\"report_version\":3,\"component_profile\":\"{}\",\"acquisition\":{}",
                    CameraContract::PROFILE,
                    self.acquisition
                        .expect("70338 requires Rust local")
                        .render()
                ),
                1,
            );
        }
        match self.acquisition {
            Some(profile) => text.replacen(
                "\"report_version\":1",
                &format!("\"report_version\":2,\"acquisition\":{}", profile.render()),
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
        .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(Code::UnsafePath);
    }
    for ancestor in root.ancestors().filter(|path| !path.as_os_str().is_empty()) {
        if fs::symlink_metadata(ancestor)
            .map_err(|_| Code::Input)?
            .file_type()
            .is_symlink()
        {
            return Err(Code::UnsafePath);
        }
    }
    directory(root)?;
    let names = children(root, 6)?;
    pending(&names)?;
    if names.contains(manifest::MARKER) {
        return Err(Code::Incomplete);
    }
    let text = read(&root.join(manifest::NAME), manifest::MAX_MANIFEST_BYTES)?;
    let decoded = manifest::decode(std::str::from_utf8(&text).map_err(|_| Code::Manifest)?)
        .map_err(|_| Code::Manifest)?;
    if !decoded.complete() {
        return Err(Code::Incomplete);
    }
    if names != set(["cameras", "dbc", manifest::NAME]) {
        return Err(Code::Inventory);
    }
    let dbc = root.join("dbc");
    directory(&dbc)?;
    let dbc_names = children(&dbc, 3)?;
    pending(&dbc_names)?;
    if dbc_names != set([decoded.locale.name()]) {
        return Err(Code::Inventory);
    }
    let locale = dbc.join(decoded.locale.name());
    directory(&locale)?;
    let names = children(&locale, 3)?;
    pending(&names)?;
    if names != set([SOURCE_NAME]) {
        return Err(Code::Inventory);
    }
    let source = read(&root.join(source_path(decoded.locale)), MAX_SOURCE_BYTES)?;
    matching(&source, &decoded.source.file)?;
    let plan = discover_with_contract(decoded.contract, &source).map_err(|_| Code::Discovery)?;
    if decoded.source.discovery.as_ref() != Some(plan.facts()) {
        return Err(Code::Discovery);
    }
    let expected_ids: BTreeSet<_> = decoded.cameras.iter().map(|entry| entry.file_id).collect();
    if plan.file_ids() != &expected_ids {
        return Err(Code::Discovery);
    }
    let cameras = root.join("cameras");
    directory(&cameras)?;
    let names = children(&cameras, MAX_FILES + 2)?;
    pending(&names)?;
    let expected_names: BTreeSet<_> = plan.file_ids().iter().map(|&id| file_name(id)).collect();
    if names != expected_names {
        return Err(Code::Inventory);
    }
    for camera in &decoded.cameras {
        matching(
            &read(&cameras.join(file_name(camera.file_id)), MAX_FILE_BYTES)?,
            &camera.file,
        )?;
    }
    Ok(Report {
        contract: decoded.contract,
        acquisition: decoded.acquisition,
        code: Code::Verified,
        build: Some(decoded.build),
        cameras_verified: decoded.cameras.len(),
    })
}
fn matching(bytes: &[u8], outcome: &manifest::FileOutcome) -> Result<(), Code> {
    if Some(bytes.len()) != outcome.bytes
        || Some(sha256::hex(bytes)).as_ref() != outcome.sha256.as_ref()
    {
        return Err(Code::Inventory);
    }
    Ok(())
}
fn set<const N: usize>(names: [&str; N]) -> BTreeSet<String> {
    names.into_iter().map(str::to_owned).collect()
}
fn pending(names: &BTreeSet<String>) -> Result<(), Code> {
    if names.iter().any(|name| name.ends_with(".part")) {
        Err(Code::Incomplete)
    } else {
        Ok(())
    }
}
fn directory(path: &Path) -> Result<(), Code> {
    let meta = fs::symlink_metadata(path).map_err(|_| Code::Input)?;
    if meta.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !meta.is_dir() {
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
        names.insert(
            entry
                .file_name()
                .into_string()
                .map_err(|_| Code::UnsafePath)?,
        );
        if names.len() > limit {
            return Err(Code::Inventory);
        }
    }
    Ok(names)
}
fn read(path: &Path, max: usize) -> Result<Vec<u8>, Code> {
    let meta = fs::symlink_metadata(path).map_err(|_| Code::Input)?;
    if meta.file_type().is_symlink() {
        return Err(Code::UnsafePath);
    }
    if !meta.is_file() || meta.len() > max as u64 {
        return Err(Code::Input);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| Code::Input)?
        .take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Code::Input)?;
    if bytes.len() > max || bytes.len() as u64 != meta.len() {
        return Err(Code::Input);
    }
    Ok(bytes)
}
