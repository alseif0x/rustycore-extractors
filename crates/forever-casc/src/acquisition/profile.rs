// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: deterministic, strict acquisition provenance.
use super::options::{index_layout, index_name, root_layout, root_name};
use crate::{
    local_index, local_store, root,
    verify::json::{self, Json},
};

const PROFILE: &str = "local-v7-ckey-root-nz-v1";
pub(super) const MAX_COMPONENT_BYTES: u32 = 16 * 1024 * 1024;

/// Closed CLI profile. Source identifiers, paths and client bytes are omitted.
/// Physical choices and every effective store policy bound are deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub(super) index_layout: local_index::Layout,
    pub(super) root_layout: root::Layout,
    pub(super) max_file_bytes: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileError;
impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid_acquisition_profile")
    }
}
impl std::error::Error for ProfileError {}

impl Profile {
    pub const fn max_file_bytes(self) -> u32 {
        self.max_file_bytes
    }
    pub(super) fn limits(self) -> local_store::Limits {
        local_store::Limits {
            max_decoded_file_bytes: u64::from(self.max_file_bytes),
            ..local_store::Limits::default()
        }
    }
    /// One JSON object, suitable for the new manifest's `acquisition` property.
    pub fn render(self) -> String {
        let indexes = std::iter::repeat_n(format!("\"{}\"", index_name(self.index_layout)), 16)
            .collect::<Vec<_>>()
            .join(",");
        let limits = self
            .limit_fields()
            .into_iter()
            .map(|(name, value)| format!("\"{name}\":{value}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"backend\":\"rust-local\",\"profile\":\"{PROFILE}\",\"index_layouts\":[{indexes}],\"root_layout\":\"{}\",\"offline\":true,\"complete_ckey_md5\":true,\"ekey_validation\":\"prefix9\",\"limits\":{{{limits}}}}}",
            root_name(self.root_layout)
        )
    }
    /// Parse a standalone profile with bounded size and nesting. Component
    /// manifests use `decode` after their own bounded complete JSON parsing.
    pub fn parse(text: &str, component_cap: u32) -> Result<Self, ProfileError> {
        if text.len() > 8192 || !bounded_depth(text) {
            return Err(ProfileError);
        }
        let value = json::parse(text).map_err(|_| ProfileError)?;
        Self::decode(&value, component_cap).map_err(|_| ProfileError)
    }
    pub(crate) fn decode(value: &Json, component_cap: u32) -> Result<Self, ()> {
        let object = object(
            value,
            &[
                "backend",
                "profile",
                "index_layouts",
                "root_layout",
                "offline",
                "complete_ckey_md5",
                "ekey_validation",
                "limits",
            ],
        )?;
        if string(field(object, "backend")?)? != "rust-local"
            || string(field(object, "profile")?)? != PROFILE
            || field(object, "offline")? != &Json::Bool(true)
            || field(object, "complete_ckey_md5")? != &Json::Bool(true)
            || string(field(object, "ekey_validation")?)? != "prefix9"
        {
            return Err(());
        }
        let Json::Array(layouts) = field(object, "index_layouts")? else {
            return Err(());
        };
        if layouts.len() != 16 {
            return Err(());
        }
        let index = index_layout(string(&layouts[0])?).ok_or(())?;
        if layouts
            .iter()
            .any(|value| string(value).ok() != Some(index_name(index)))
        {
            return Err(());
        }
        let root = root_layout(string(field(object, "root_layout")?)?).ok_or(())?;
        let Json::Object(limits) = field(object, "limits")? else {
            return Err(());
        };
        let file_bytes = number(field(limits, "file_bytes")?)?;
        if file_bytes == 0 || file_bytes > u64::from(component_cap.min(MAX_COMPONENT_BYTES)) {
            return Err(());
        }
        let profile = Self {
            index_layout: index,
            root_layout: root,
            max_file_bytes: file_bytes as u32,
        };
        let expected = profile.limit_fields();
        if limits.len() != expected.len() {
            return Err(());
        }
        for (name, expected) in expected {
            if number(field(limits, name)?)? != expected {
                return Err(());
            }
        }
        // The complete JSON parser rejects duplicate keys. Also defend direct
        // crate callers constructing Json manually, rather than trusting parse.
        for (index, (name, _)) in limits.iter().enumerate() {
            if limits[..index].iter().any(|(other, _)| other == name) {
                return Err(());
            }
        }
        Ok(profile)
    }
    fn limit_fields(self) -> [(&'static str, u64); 29] {
        let l = self.limits();
        [
            ("resident_bytes", l.max_resident_bytes as u64),
            ("scratch_bytes", l.max_scratch_bytes as u64),
            ("combined_bytes", l.max_memory_bytes as u64),
            ("build_info_bytes", l.max_build_info_bytes),
            ("build_info_rows", l.max_build_info_rows as u64),
            ("path_bytes", l.max_path_bytes as u64),
            ("config_bytes", l.config.max_bytes),
            ("config_lines", l.config.max_lines as u64),
            ("config_keys", l.config.max_keys as u64),
            ("directory_entries", l.max_directory_entries as u64),
            ("index_files", l.max_index_files as u64),
            ("index_bytes", l.index.max_bytes),
            ("index_entries", l.index.max_entries as u64),
            ("index_pages", l.index.max_pages as u64),
            ("encoding_bytes", l.encoding.max_bytes),
            ("encoding_pages", l.encoding.max_pages as u64),
            ("encoding_page_bytes", l.encoding.max_page_bytes as u64),
            ("encoding_espec_bytes", l.encoding.max_espec_bytes as u64),
            ("encoding_tail_bytes", l.encoding.max_tail_bytes as u64),
            ("encoding_entries", l.encoding.max_entries as u64),
            ("encoding_keys", l.encoding.max_encoded_keys as u64),
            ("root_bytes", l.root.max_bytes),
            ("root_header_bytes", l.root.max_header_bytes as u64),
            ("root_blocks", l.root.max_blocks as u64),
            ("root_entries", l.root.max_entries as u64),
            ("archive_span_bytes", l.max_archive_span_bytes),
            ("file_bytes", l.max_decoded_file_bytes),
            ("candidate_attempts", l.max_candidate_attempts as u64),
            ("blte_frames", u64::from(l.max_blte_frames)),
        ]
    }
}
fn field<'a>(object: &'a [(String, Json)], name: &str) -> Result<&'a Json, ()> {
    object
        .iter()
        .find(|(key, _)| key == name)
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
fn number(value: &Json) -> Result<u64, ()> {
    if let Json::Number(value) = value {
        u64::try_from(*value).map_err(|_| ())
    } else {
        Err(())
    }
}
fn object<'a>(value: &'a Json, names: &[&str]) -> Result<&'a [(String, Json)], ()> {
    let Json::Object(object) = value else {
        return Err(());
    };
    if object.len() != names.len() {
        return Err(());
    }
    for (index, (name, _)) in object.iter().enumerate() {
        if !names.contains(&name.as_str()) || object[..index].iter().any(|(other, _)| other == name)
        {
            return Err(());
        }
    }
    Ok(object)
}
fn bounded_depth(text: &str) -> bool {
    let mut depth = 0_u32;
    let mut string = false;
    let mut escape = false;
    for byte in text.bytes() {
        if string {
            if escape {
                escape = false;
            } else if byte == b'\\' {
                escape = true;
            } else if byte == b'"' {
                string = false;
            }
        } else {
            match byte {
                b'"' => string = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 4 {
                        return false;
                    }
                }
                b'}' | b']' => {
                    let Some(next) = depth.checked_sub(1) else {
                        return false;
                    };
                    depth = next;
                }
                _ => {}
            }
        }
    }
    depth == 0 && !string
}
