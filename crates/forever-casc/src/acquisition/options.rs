// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: pure common acquisition argument validation.
use super::Profile;
use crate::{local_index, options::CliError, root};
use std::ffi::OsStr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    Native,
    RustLocal(Profile),
}
impl Selection {
    pub const fn is_rust_local(self) -> bool {
        matches!(self, Self::RustLocal(_))
    }
    pub const fn profile(self) -> Option<Profile> {
        match self {
            Self::Native => None,
            Self::RustLocal(profile) => Some(profile),
        }
    }
    /// Check binary capability before admitting or opening client/output paths.
    /// Parsing itself remains independent of the build's optional features.
    pub fn ensure_available(self) -> Result<(), CliError> {
        if self == Self::Native && !cfg!(feature = "native") {
            Err(CliError::new("backend_unavailable", 2))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct OptionsBuilder {
    rust_local: Option<bool>,
    index_layout: Option<local_index::Layout>,
    root_layout: Option<root::Layout>,
    max_file_bytes: Option<u32>,
}
impl OptionsBuilder {
    /// Consume a recognized pair; return false for component-specific options.
    /// Empty/non-UTF8 values, duplicates and unknown enum spellings reject.
    pub fn push(&mut self, key: &str, value: &OsStr) -> Result<bool, CliError> {
        if !matches!(
            key,
            "--backend" | "--index-layout" | "--root-layout" | "--max-file-bytes"
        ) {
            return Ok(false);
        }
        let value = value
            .to_str()
            .filter(|v| !v.is_empty())
            .ok_or(CliError::INVALID_INVOCATION)?;
        match key {
            "--backend" => set(
                &mut self.rust_local,
                match value {
                    "native" => false,
                    "rust-local" => true,
                    _ => return Err(CliError::INVALID_INVOCATION),
                },
            )?,
            "--index-layout" => set(
                &mut self.index_layout,
                index_layout(value).ok_or(CliError::INVALID_INVOCATION)?,
            )?,
            "--root-layout" => set(
                &mut self.root_layout,
                root_layout(value).ok_or(CliError::INVALID_INVOCATION)?,
            )?,
            "--max-file-bytes" => {
                if !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(CliError::INVALID_INVOCATION);
                }
                let count = value
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or(CliError::INVALID_INVOCATION)?;
                set(&mut self.max_file_bytes, count)?;
            }
            _ => unreachable!(),
        }
        Ok(true)
    }
    pub fn finish(self, component_cap: u32) -> Result<Selection, CliError> {
        if !self.rust_local.unwrap_or(false) {
            if self.index_layout.is_some()
                || self.root_layout.is_some()
                || self.max_file_bytes.is_some()
            {
                return Err(CliError::INVALID_INVOCATION);
            }
            return Ok(Selection::Native);
        }
        let max_file_bytes = self.max_file_bytes.unwrap_or(component_cap);
        if component_cap == 0
            || component_cap > super::profile::MAX_COMPONENT_BYTES
            || max_file_bytes > component_cap
        {
            return Err(CliError::INVALID_INVOCATION);
        }
        Ok(Selection::RustLocal(Profile {
            index_layout: self.index_layout.ok_or(CliError::INVALID_INVOCATION)?,
            root_layout: self.root_layout.ok_or(CliError::INVALID_INVOCATION)?,
            max_file_bytes,
        }))
    }
}
fn set<T>(slot: &mut Option<T>, value: T) -> Result<(), CliError> {
    if slot.is_some() {
        Err(CliError::INVALID_INVOCATION)
    } else {
        *slot = Some(value);
        Ok(())
    }
}
pub(super) fn index_layout(value: &str) -> Option<local_index::Layout> {
    match value {
        "contiguous" => Some(local_index::Layout::Contiguous),
        "paged" => Some(local_index::Layout::Paged),
        _ => None,
    }
}
pub(super) fn root_layout(value: &str) -> Option<root::Layout> {
    match value {
        "headerless-18125" => Some(root::Layout::Headerless18125),
        "header-30080" => Some(root::Layout::Header30080),
        "header-50893-v1" => Some(root::Layout::Header50893V1),
        "header-50893-v2" => Some(root::Layout::Header50893V2),
        _ => None,
    }
}
pub(super) const fn index_name(value: local_index::Layout) -> &'static str {
    match value {
        local_index::Layout::Contiguous => "contiguous",
        local_index::Layout::Paged => "paged",
    }
}
pub(super) const fn root_name(value: root::Layout) -> &'static str {
    match value {
        root::Layout::Headerless18125 => "headerless-18125",
        root::Layout::Header30080 => "header-30080",
        root::Layout::Header50893V1 => "header-50893-v1",
        root::Layout::Header50893V2 => "header-50893-v2",
    }
}
