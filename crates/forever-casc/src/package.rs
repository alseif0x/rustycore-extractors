// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: common versioned artifact-integrity package contract.
//! A complete package proves selected producer completion and byte inventory,
//! not semantic format/client/runtime compatibility. No CASC or network access.
#[cfg(any(target_os = "linux", windows))]
use crate::sha256;
use crate::{
    Locale,
    verify::json::{self, Json},
};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(any(target_os = "linux", windows))]
use std::io::BufRead;
#[cfg(any(target_os = "linux", windows))]
use std::{
    io::{self, Read, Write},
    path::Path,
};
#[cfg(target_os = "linux")]
mod disk;
mod model;
#[cfg(target_os = "linux")]
pub use disk::{Put, Run, verify, verify_with};
#[cfg(windows)]
pub mod windows;
pub use model::{Artifact, Component, Context, Limits, Summary};
#[cfg(windows)]
pub use windows::{Put, Run, verify, verify_with};
#[cfg(all(test, target_os = "linux"))]
mod tests;
pub const MANIFEST: &str = "forever-package.json";
pub const INCOMPLETE: &str = ".forever-package-incomplete";
const LINE: usize = 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    Context,
    Path,
    Exists,
    Busy,
    Io,
    Changed,
    Schema,
    Incomplete,
    Inventory,
    Digest,
    Component,
    Sealed,
    Unsupported,
    StateVersion,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "package_limit",
            Self::Allocation => "package_allocation",
            Self::Context => "package_context",
            Self::Path => "package_path",
            Self::Exists => "package_exists",
            Self::Busy => "package_busy",
            Self::Io => "package_io",
            Self::Changed => "package_input_changed",
            Self::Schema => "package_schema",
            Self::Incomplete => "package_incomplete",
            Self::Inventory => "package_inventory",
            Self::Digest => "package_digest",
            Self::Component => "package_component",
            Self::Sealed => "package_component_sealed",
            Self::Unsupported => "package_platform",
            Self::StateVersion => "package_state_version",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}
#[cfg(any(target_os = "linux", windows))]
fn digest(b: &[u8]) -> [u8; 32] {
    let mut h = sha256::Sha256::new();
    h.update(b);
    h.finish()
}
fn unhex(s: &str) -> Result<[u8; 32], Error> {
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Schema);
    }
    let mut out = [0; 32];
    for (i, v) in out.iter_mut().enumerate() {
        *v = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|_| Error::Schema)?;
    }
    Ok(out)
}
fn value<'a>(v: &'a Json, key: &str) -> Result<&'a Json, Error> {
    let Json::Object(fields) = v else {
        return Err(Error::Schema);
    };
    let mut found = fields.iter().filter(|(k, _)| k == key);
    let v = &found.next().ok_or(Error::Schema)?.1;
    if found.next().is_some() {
        return Err(Error::Schema);
    }
    Ok(v)
}
fn string<'a>(v: &'a Json, key: &str) -> Result<&'a str, Error> {
    match value(v, key)? {
        Json::String(s) => Ok(s),
        _ => Err(Error::Schema),
    }
}
fn number(v: &Json, key: &str) -> Result<u64, Error> {
    match value(v, key)? {
        Json::Number(n) if *n >= 0 => Ok(*n as u64),
        _ => Err(Error::Schema),
    }
}
fn parse(s: &str) -> Result<Json, Error> {
    if s.len() > LINE {
        return Err(Error::Limit);
    }
    json::parse(s).map_err(|_| Error::Schema)
}
#[cfg(any(target_os = "linux", windows))]
fn line<R: BufRead>(r: &mut R) -> Result<Vec<u8>, Error> {
    let mut v = Vec::new();
    std::io::Read::take(&mut *r, (LINE + 1) as u64)
        .read_until(b'\n', &mut v)
        .map_err(|_| Error::Io)?;
    if v.len() > LINE {
        return Err(Error::Limit);
    }
    if v.last() != Some(&b'\n') {
        return Err(Error::Schema);
    }
    Ok(v)
}
