// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded parser for the WoW build-config subset.
// Format facts: CascFiles.cpp::LoadCKeyEntry/ParseFile_CdnBuild/
// FetchAndLoadConfigFile and CascLib.h::CASC_INVALID_SIZE at TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e. No upstream parser code imported.
//! Parse a complete build-config text bound to its independently supplied MD5.
//!
//! This pure parser performs no I/O, network request or target admission. The
//! expected build key must come from the caller's admitted `.build.info` row.
//! It checks the MD5 of every input byte before interpreting text. `build-uid`
//! and `build-name` are retained as metadata, never interpreted as permission to
//! change product, version or build. Hash fields are content identifiers, not
//! encryption keys; they are not included in errors or automatically logged.
//!
//! The reviewed WoW subset requires ROOT's CKey and ENCODING's CKey plus EKey.
//! Their optional `-size` lines hold a decimal content size and optional encoded
//! size. The pinned u32 sentinel `4294967295` means unknown, represented by
//! `None`; zero remains a declared zero. Missing sizes are also `None` and must
//! be resolved or explicitly rejected by the future bounded store reader.
//!
//! Relevant names are matched exactly, ignoring ASCII case, without the native
//! parser's `root*`/`encoding*` wildcard expansion. Any duplicate relevant field
//! rejects, including case aliases. Unknown well-formed assignments are ignored
//! and counted; this includes `root-config`, `encoding-config` and unrelated
//! source tags. Their values are never treated as alternate roots or keys.
//!
//! Syntax accepts LF/CRLF, spaces/tabs around `=`, blank lines and whole-line
//! `#` comments. Inline comments are not stripped. Other control characters,
//! malformed assignments or names longer than the native 79-byte variable-name
//! capacity reject. Byte, physical-line and assignment-count budgets apply;
//! even ignored assignments count. Strings borrow the input. Auxiliary manifest
//! references use a checked, assignment-bounded vector, never an array sized by
//! a VFS identifier. No config text is copied.

use crate::blte::md5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: u64,
    /// Includes blank and comment lines; a final newline adds no empty line.
    pub max_lines: usize,
    /// All assignments, including repeated unknown names, count toward this cap.
    pub max_keys: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentReference {
    pub content_key: [u8; 16],
    pub encoded_key: Option<[u8; 16]>,
    pub content_size: Option<u32>,
    pub encoded_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuxiliaryKind {
    Download,
    Install,
    Patch,
    Size,
    VfsRoot,
    Vfs(u32),
}

/// Pinned central-map references. A one-hash PATCH is EKey-only, unlike other
/// tags. `-config` remains opaque; exact names replace native wildcard matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxiliaryReference {
    pub kind: AuxiliaryKind,
    pub content_key: Option<[u8; 16]>,
    pub encoded_key: Option<[u8; 16]>,
    pub content_size: Option<u32>,
    pub encoded_size: Option<u32>,
    seen: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildConfig<'a> {
    pub root: ContentReference,
    /// `encoded_key` is always present for a successfully parsed config.
    pub encoding: ContentReference,
    pub build_uid: Option<&'a str>,
    pub build_name: Option<&'a str>,
    pub ignored_keys: usize,
    pub auxiliary: Vec<AuxiliaryReference>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Limit,
    Checksum,
    Text,
    Syntax,
    Duplicate,
    Hashes,
    Sizes,
    Missing,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "build_config_policy_limit",
            Self::Checksum => "build_config_checksum_mismatch",
            Self::Text => "build_config_invalid_text",
            Self::Syntax => "build_config_invalid_assignment",
            Self::Duplicate => "build_config_duplicate_field",
            Self::Hashes => "build_config_invalid_hashes",
            Self::Sizes => "build_config_invalid_sizes",
            Self::Missing => "build_config_missing_field",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

pub fn parse(
    bytes: &[u8],
    expected_build_key: [u8; 16],
    limits: Limits,
) -> Result<BuildConfig<'_>, Error> {
    if u64::try_from(bytes.len()).map_err(|_| Error::Limit)? > limits.max_bytes {
        return Err(Error::Limit);
    }
    if md5::digest(bytes) != expected_build_key {
        return Err(Error::Checksum);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Error::Text)?;
    let mut root = None;
    let mut encoding = None;
    let mut root_sizes = (None, None);
    let mut encoding_sizes = (None, None);
    let mut build_uid = None;
    let mut build_name = None;
    let mut ignored_keys = 0;
    let mut assignments = 0_usize;
    let mut seen = 0_u8;
    let mut auxiliary = Vec::new();
    const FIELDS: [&str; 6] = [
        "root",
        "encoding",
        "root-size",
        "encoding-size",
        "build-uid",
        "build-name",
    ];
    for (line_number, line) in text.lines().enumerate() {
        if line_number >= limits.max_lines {
            return Err(Error::Limit);
        }
        if line
            .bytes()
            .any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return Err(Error::Text);
        }
        let line = trim(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').ok_or(Error::Syntax)?;
        let name = trim(name);
        let value = trim(value);
        if name.is_empty()
            || name.len() > 79
            || value.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(Error::Syntax);
        }
        assignments = assignments.checked_add(1).ok_or(Error::Limit)?;
        if assignments > limits.max_keys {
            return Err(Error::Limit);
        }
        let Some(field) = FIELDS
            .iter()
            .position(|known| name.eq_ignore_ascii_case(known))
        else {
            if !parse_auxiliary(name, value, &mut auxiliary)? {
                ignored_keys += 1;
            }
            continue;
        };
        let bit = 1_u8 << field;
        if seen & bit != 0 {
            return Err(Error::Duplicate);
        }
        seen |= bit;
        match field {
            0 => root = Some(hashes(value, false)?),
            1 => encoding = Some(hashes(value, true)?),
            2 => root_sizes = sizes(value)?,
            3 => encoding_sizes = sizes(value)?,
            4 => build_uid = Some(value),
            5 => build_name = Some(value),
            _ => unreachable!(),
        }
    }
    let (root_key, root_encoded) = root.ok_or(Error::Missing)?;
    let (encoding_key, encoding_encoded) = encoding.ok_or(Error::Missing)?;
    Ok(BuildConfig {
        root: ContentReference {
            content_key: root_key,
            encoded_key: root_encoded,
            content_size: root_sizes.0,
            encoded_size: root_sizes.1,
        },
        encoding: ContentReference {
            content_key: encoding_key,
            encoded_key: encoding_encoded,
            content_size: encoding_sizes.0,
            encoded_size: encoding_sizes.1,
        },
        build_uid,
        build_name,
        ignored_keys,
        auxiliary,
    })
}

fn parse_auxiliary(
    name: &str,
    value: &str,
    references: &mut Vec<AuxiliaryReference>,
) -> Result<bool, Error> {
    let (base, is_size) = if name.len() >= 5 && name[name.len() - 5..].eq_ignore_ascii_case("-size")
    {
        (&name[..name.len() - 5], true)
    } else {
        (name, false)
    };
    let kind = if base.eq_ignore_ascii_case("download") {
        AuxiliaryKind::Download
    } else if base.eq_ignore_ascii_case("install") {
        AuxiliaryKind::Install
    } else if base.eq_ignore_ascii_case("patch") {
        AuxiliaryKind::Patch
    } else if base.eq_ignore_ascii_case("size") {
        AuxiliaryKind::Size
    } else if base.eq_ignore_ascii_case("vfs-root") {
        AuxiliaryKind::VfsRoot
    } else if base.len() > 4
        && base[..4].eq_ignore_ascii_case("vfs-")
        && base[4..].bytes().all(|b| b.is_ascii_digit())
    {
        let id = base[4..].parse::<u32>().map_err(|_| Error::Syntax)?;
        if id == 0 {
            return Err(Error::Syntax);
        }
        AuxiliaryKind::Vfs(id)
    } else {
        return Ok(false);
    };
    let position = if let Some(position) = references.iter().position(|item| item.kind == kind) {
        position
    } else {
        references.try_reserve_exact(1).map_err(|_| Error::Limit)?;
        references.push(AuxiliaryReference {
            kind,
            content_key: None,
            encoded_key: None,
            content_size: None,
            encoded_size: None,
            seen: 0,
        });
        references.len() - 1
    };
    let reference = &mut references[position];
    let bit = if is_size { 2 } else { 1 };
    if reference.seen & bit != 0 {
        return Err(Error::Duplicate);
    }
    reference.seen |= bit;
    if is_size {
        (reference.content_size, reference.encoded_size) = sizes(value)?;
    } else {
        let (content, encoded) = hashes(value, false)?;
        if kind == AuxiliaryKind::Patch && encoded.is_none() {
            reference.encoded_key = Some(content);
        } else {
            reference.content_key = Some(content);
            reference.encoded_key = encoded;
        }
    }
    Ok(true)
}

fn trim(text: &str) -> &str {
    text.trim_matches([' ', '\t'])
}

fn hashes(value: &str, encoded_required: bool) -> Result<([u8; 16], Option<[u8; 16]>), Error> {
    let mut tokens = value.split_ascii_whitespace();
    let content = hash(tokens.next().ok_or(Error::Hashes)?)?;
    let encoded = tokens.next().map(hash).transpose()?;
    if tokens.next().is_some() || (encoded_required && encoded.is_none()) {
        return Err(Error::Hashes);
    }
    Ok((content, encoded))
}

fn hash(token: &str) -> Result<[u8; 16], Error> {
    let bytes = token.as_bytes();
    if bytes.len() != 32 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(Error::Hashes);
    }
    let mut hash = [0; 16];
    for (output, pair) in hash.iter_mut().zip(bytes.as_chunks::<2>().0) {
        *output = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(hash)
}

fn nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        _ => byte.to_ascii_lowercase() - b'a' + 10,
    }
}

fn sizes(value: &str) -> Result<(Option<u32>, Option<u32>), Error> {
    let mut tokens = value.split_ascii_whitespace();
    let content = size(tokens.next().ok_or(Error::Sizes)?)?;
    let encoded = tokens.next().map(size).transpose()?.flatten();
    if tokens.next().is_some() {
        return Err(Error::Sizes);
    }
    Ok((content, encoded))
}

fn size(token: &str) -> Result<Option<u32>, Error> {
    if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Sizes);
    }
    let value = token.parse::<u32>().map_err(|_| Error::Sizes)?;
    Ok((value != u32::MAX).then_some(value))
}

#[cfg(test)]
mod tests;
