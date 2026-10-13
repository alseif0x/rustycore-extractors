// SPDX-License-Identifier: GPL-3.0-or-later
//! Closed GT plans and an injected acquisition seam; no native dependency here.
use super::{GameTable, Group, MAX_FILE_BYTES, manifest, output, parse};
use crate::{Error, Locale};
use std::{io::Read, path::Path};

pub const MAX_BATCH_BYTES: usize = 6 * MAX_FILE_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plan {
    All,
    Initial,
    SpellValues,
}

impl Plan {
    pub const fn name(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Initial => "initial",
            Self::SpellValues => "spell-values",
        }
    }

    pub fn named(name: &str) -> Option<Self> {
        match name {
            "all" => Some(Self::All),
            "initial" => Some(Self::Initial),
            "spell-values" => Some(Self::SpellValues),
            _ => None,
        }
    }

    pub fn tables(self) -> Vec<GameTable> {
        match self {
            Self::All => GameTable::ALL.to_vec(),
            Self::Initial => Group::Initial.tables().to_vec(),
            Self::SpellValues => Group::SpellValues.tables().to_vec(),
        }
    }
}

pub struct Report {
    pub acquisition: Option<crate::acquisition::Profile>,
    pub complete: bool,
    pub manifest: String,
    pub build: u32,
    pub locale: Locale,
    pub plan: Plan,
    pub files_published: usize,
}

impl Report {
    pub fn json(&self) -> String {
        let text = format!(
            "{{\"report_version\":1,\"scope\":\"explicit_gt_batch\",\"status\":\"{}\",\"all_requested_files_complete\":{},\"build\":{},\"locale\":\"{}\",\"plan\":\"{}\",\"files_requested\":{},\"files_published\":{},\"full_client_extraction\":false,\"typed_loaders_executed\":false,\"offline\":true,\"paths_redacted\":true}}",
            if self.complete { "complete" } else { "partial" },
            self.complete,
            self.build,
            self.locale.name(),
            self.plan.name(),
            self.plan.tables().len(),
            self.files_published,
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

/// Acquire a complete closed group into a new private output. On partial
/// failure successful raw files remain with an incomplete marker; neither the
/// group nor any typed loader is reported accepted. A loader never returns a
/// usable prefix. Native callers must independently admit every file's build.
pub fn extract(
    plan: Plan,
    client: &Path,
    destination: &Path,
    build: u32,
    locale: Locale,
    load: impl FnMut(GameTable, u32) -> Result<(u32, Vec<u8>), Error>,
) -> Result<Report, &'static str> {
    extract_with_profile(plan, client, destination, build, locale, None, load)
}

/// Explicit acquisition provenance. The injected reader must use this profile;
/// it is a caller-supplied contract, not independent backend authentication.
#[allow(clippy::too_many_arguments)]
pub fn extract_with_profile(
    plan: Plan,
    client: &Path,
    destination: &Path,
    build: u32,
    locale: Locale,
    acquisition: Option<crate::acquisition::Profile>,
    mut load: impl FnMut(GameTable, u32) -> Result<(u32, Vec<u8>), Error>,
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
    let directory = output::create(client, destination)?;
    let mut entries = Vec::new();
    let mut total = 0usize;
    for table in plan.tables() {
        if crate::admit(client).map_err(|error| error.code())? != build {
            return Err("client_build_changed");
        }
        let remaining = MAX_BATCH_BYTES
            .checked_sub(total)
            .ok_or("gt_policy_limit")?;
        let budget = file_cap.min(remaining);
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
                    let parsed = parse(&bytes, table).map_err(|error| error.code())?;
                    output::publish(&directory.join("gt"), table.file_name(), &bytes)?;
                    Ok(manifest::Artifact::complete(table, &bytes, parsed.facts()))
                })
        };
        if acquisition.is_some() && matches!(result, Err("client_build_changed")) {
            return Err("client_build_changed");
        }
        entries.push(match result {
            Ok(artifact) => artifact,
            Err(code) => manifest::Artifact::failed(table, code),
        });
    }
    if metadata_snapshot(client)? != metadata_before {
        return Err("client_metadata_changed");
    }
    if crate::admit(client).map_err(|error| error.code())? != build {
        return Err("client_build_changed");
    }
    let manifest = manifest::Manifest {
        acquisition,
        build,
        locale,
        plan,
        artifacts: entries,
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
        plan,
        files_published: manifest.inventory().0,
    })
}

pub(super) fn metadata_snapshot(client: &Path) -> Result<Vec<u8>, &'static str> {
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
