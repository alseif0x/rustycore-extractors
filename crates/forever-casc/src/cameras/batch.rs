// SPDX-License-Identifier: GPL-3.0-or-later
//! Injected complete-file pipeline; native code supplies a strict offline reader.
use super::{
    CameraPlan, MAX_FILE_BYTES, MAX_SOURCE_BYTES, MAX_TOTAL_BYTES, SOURCE_FILE_ID, SOURCE_NAME,
    discover, file_name,
    manifest::{self, Camera, FileOutcome, Manifest, Source},
    output,
};
use crate::{Error, Locale};
use std::{io::Read, path::Path};

pub struct Report {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub complete: bool,
    pub manifest: String,
    pub build: u32,
    pub locale: Locale,
    pub discovered_files: Option<usize>,
    pub files_published: usize,
}
impl Report {
    pub fn json(&self) -> String {
        let text = format!(
            "{{\"report_version\":1,\"scope\":\"complete_camera_files\",\"status\":\"{}\",\"all_requested_files_complete\":{},\"build\":{},\"locale\":\"{}\",\"discovered_camera_files\":{},\"files_published_including_source\":{},\"offline\":true,\"paths_redacted\":true,\"full_client_extraction\":false,\"m2_parsed\":false,\"waypoints_generated\":false,\"runtime_accepted\":false}}",
            if self.complete { "complete" } else { "partial" },
            self.complete,
            self.build,
            self.locale.name(),
            self.discovered_files
                .map_or_else(|| "null".to_owned(), |n| n.to_string()),
            self.files_published
        );
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

/// The reader must return complete bytes only. Storage locale selects CASC
/// root; the existing native bridge passes locale 0 to CascOpenFile. This
/// module never requests keys, remote storage, prefix reads or M2 conversion.
pub fn extract(
    client: &Path,
    destination: &Path,
    build: u32,
    locale: Locale,
    load: impl FnMut(u32, u32) -> Result<(u32, Vec<u8>), Error>,
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
    mut load: impl FnMut(u32, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    let file_cap = acquisition.map_or(MAX_FILE_BYTES, |profile| profile.max_file_bytes() as usize);
    if file_cap == 0 || file_cap > MAX_FILE_BYTES {
        return Err("camera_policy_limit");
    }
    if !crate::SUPPORTED_CLIENTS.iter().any(|(_, id)| *id == build) {
        return Err("unsupported_client");
    }
    same_build(client, build)?;
    let before = metadata_snapshot(client)?;
    let directory = output::create(client, destination, locale)?;
    let mut total = 0usize;
    same_build(client, build)?;
    let source_budget = file_cap.min(MAX_SOURCE_BYTES);
    let loaded = load(SOURCE_FILE_ID, source_budget as u32);
    if let Err(error) = loaded.as_ref()
        && error.is_input_changed()
    {
        return Err(error.code());
    }
    let source_result = loaded.map_err(|e| e.code()).and_then(|(observed, bytes)| {
        if observed != build {
            return Err("client_build_changed");
        }
        if bytes.len() > source_budget {
            return Err("camera_policy_limit");
        }
        total = bytes.len();
        let plan = discover(&bytes)?;
        output::publish(
            &directory.join("dbc").join(locale.name()),
            SOURCE_NAME,
            &bytes,
        )?;
        Ok((FileOutcome::complete(&bytes), plan))
    });
    if acquisition.is_some() && matches!(source_result, Err("client_build_changed")) {
        return Err("client_build_changed");
    }
    let (source, plan): (Source, Option<CameraPlan>) = match source_result {
        Ok((file, plan)) => (
            Source {
                file,
                discovery: Some(plan.facts().clone()),
            },
            Some(plan),
        ),
        Err(code) => (
            Source {
                file: FileOutcome::failed(code),
                discovery: None,
            },
            None,
        ),
    };
    let mut cameras = Vec::new();
    if let Some(plan) = &plan {
        for &id in plan.file_ids() {
            same_build(client, build)?;
            let budget = remaining_budget(total)?.min(file_cap);
            let outcome = if budget == 0 {
                Err("camera_policy_limit")
            } else {
                let loaded = load(id, budget as u32);
                if let Err(error) = loaded.as_ref()
                    && error.is_input_changed()
                {
                    return Err(error.code());
                }
                loaded.map_err(|e| e.code()).and_then(|(observed, bytes)| {
                    if observed != build {
                        return Err("client_build_changed");
                    }
                    if bytes.len() > budget {
                        return Err("camera_policy_limit");
                    }
                    total = total
                        .checked_add(bytes.len())
                        .ok_or("camera_policy_limit")?;
                    output::publish(&directory.join("cameras"), &file_name(id), &bytes)?;
                    Ok(FileOutcome::complete(&bytes))
                })
            };
            if acquisition.is_some() && matches!(outcome, Err("client_build_changed")) {
                return Err("client_build_changed");
            }
            cameras.push(Camera {
                file_id: id,
                file: match outcome {
                    Ok(file) => file,
                    Err(code) => FileOutcome::failed(code),
                },
            });
        }
    }
    if metadata_snapshot(client)? != before {
        return Err("client_metadata_changed");
    }
    same_build(client, build)?;
    let manifest = Manifest {
        acquisition,
        build,
        locale,
        source,
        cameras,
    };
    let complete = manifest.complete();
    let text = manifest.render()?;
    let files_published = manifest.inventory()?.0;
    if text.len() > manifest::MAX_MANIFEST_BYTES {
        return Err("camera_policy_limit");
    }
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
        discovered_files: plan.as_ref().map(|p| p.file_ids().len()),
        files_published,
    })
}
pub(super) fn remaining_budget(total: usize) -> Result<usize, &'static str> {
    Ok(MAX_FILE_BYTES.min(
        MAX_TOTAL_BYTES
            .checked_sub(total)
            .ok_or("camera_policy_limit")?,
    ))
}
fn same_build(client: &Path, build: u32) -> Result<(), &'static str> {
    if crate::admit(client).map_err(|e| e.code())? != build {
        return Err("client_build_changed");
    }
    Ok(())
}
fn metadata_snapshot(client: &Path) -> Result<Vec<u8>, &'static str> {
    let file = std::fs::File::open(client.join(".build.info")).map_err(|_| "input_io")?;
    let mut bytes = Vec::new();
    file.take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "input_io")?;
    if bytes.len() > 256 * 1024 {
        return Err("read_policy_limit");
    }
    Ok(bytes)
}
