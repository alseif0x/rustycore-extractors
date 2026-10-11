// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in pinned catalog acquisition. All twenty files must complete; raw-only
//! entries cannot create numeric facts or optional-success omissions.
use super::{
    MAX_FILE_BYTES,
    batch::metadata_snapshot,
    catalog::{self, MAX_BATCH_BYTES, MAX_FILES, Table},
    catalog_manifest::{Artifact, Manifest},
    manifest, output,
};
use crate::{Error, Locale};
use std::path::Path;

pub struct Report {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub complete: bool,
    pub manifest: String,
    pub build: u32,
    pub locale: Locale,
    pub files_published: usize,
}
impl Report {
    pub fn json(&self) -> String {
        let text = format!(
            "{{\"report_version\":2,\"scope\":\"pinned_gt_catalog\",\"status\":\"{}\",\"all_requested_files_complete\":{},\"build\":{},\"locale\":\"{}\",\"plan\":\"catalog\",\"files_requested\":{},\"files_published\":{},\"full_client_extraction\":false,\"typed_loaders_executed\":false,\"offline\":true,\"paths_redacted\":true}}",
            if self.complete { "complete" } else { "partial" },
            self.complete,
            self.build,
            self.locale.name(),
            MAX_FILES,
            self.files_published
        );
        match self.acquisition {
            Some(profile) => text.replacen(
                "\"report_version\":2",
                &format!("\"report_version\":3,\"acquisition\":{}", profile.render()),
                1,
            ),
            None => text,
        }
    }
}

pub fn extract(
    client: &Path,
    destination: &Path,
    build: u32,
    locale: Locale,
    load: impl FnMut(Table, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    extract_with_profile(client, destination, build, locale, None, load)
}

/// Explicit acquisition provenance. The injected reader must use this profile;
/// it is a caller-supplied contract, not independent backend authentication.
pub fn extract_with_profile(
    client: &Path,
    destination: &Path,
    build: u32,
    locale: Locale,
    acquisition: Option<crate::acquisition::Profile>,
    mut load: impl FnMut(Table, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    let file_cap = acquisition.map_or(MAX_FILE_BYTES, |profile| profile.max_file_bytes() as usize);
    if file_cap == 0 || file_cap > MAX_FILE_BYTES {
        return Err("gt_policy_limit");
    }
    if !crate::SUPPORTED_CLIENTS.iter().any(|(_, id)| *id == build) {
        return Err("unsupported_client");
    }
    if crate::admit(client).map_err(|error| error.code())? != build {
        return Err("client_build_changed");
    }
    let metadata_before = metadata_snapshot(client)?;
    let directory = output::create_catalog(client, destination)?;
    let mut artifacts = Vec::with_capacity(MAX_FILES);
    let mut total = 0usize;
    for table in Table::ALL {
        if crate::admit(client).map_err(|error| error.code())? != build {
            return Err("client_build_changed");
        }
        let budget = file_cap.min(
            MAX_BATCH_BYTES
                .checked_sub(total)
                .ok_or("gt_policy_limit")?,
        );
        let result = if budget == 0 {
            Err("gt_policy_limit")
        } else {
            let loaded = load(table, budget as u32);
            if let Err(error) = loaded.as_ref()
                && error.is_input_changed()
            {
                return Err(error.code());
            }
            loaded
                .map_err(|error| error.code())
                .and_then(|(observed, bytes)| {
                    if observed != build {
                        return Err("client_build_changed");
                    }
                    if bytes.len() > budget {
                        return Err("gt_policy_limit");
                    }
                    total = total.checked_add(bytes.len()).ok_or("gt_policy_limit")?;
                    let facts = if table.value_columns().is_some() {
                        Some(
                            catalog::parse_numeric(&bytes, table)
                                .map_err(|error| error.code())?
                                .facts(),
                        )
                    } else {
                        None
                    };
                    output::publish(&directory.join("gt"), table.file_name(), &bytes)?;
                    Ok(Artifact::complete(table, &bytes, facts))
                })
        };
        if acquisition.is_some() && matches!(result, Err("client_build_changed")) {
            return Err("client_build_changed");
        }
        artifacts.push(match result {
            Ok(artifact) => artifact,
            Err(code) => Artifact::failed(table, code),
        });
    }
    if metadata_snapshot(client)? != metadata_before {
        return Err("client_metadata_changed");
    }
    if crate::admit(client).map_err(|error| error.code())? != build {
        return Err("client_build_changed");
    }
    let manifest = Manifest {
        acquisition,
        build,
        locale,
        artifacts,
    };
    let complete = manifest.complete();
    let text = manifest.render();
    output::publish(&directory, manifest::NAME, text.as_bytes())?;
    if complete {
        output::complete(&directory)?;
    }
    Ok(Report {
        acquisition,
        complete,
        manifest: text,
        build,
        locale,
        files_published: manifest.inventory().0,
    })
}
