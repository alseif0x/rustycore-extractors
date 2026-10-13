// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: offline versioned product orchestration.
//! Selected producer completion, plan binding and artifact validation do not
//! establish full real-client extraction, runtime compatibility or distribution.
#[cfg(any(target_os = "linux", windows))]
pub mod driver;
pub mod input;
pub mod plan;
pub mod producer;
#[cfg(any(target_os = "linux", windows))]
pub mod verify;

pub const CONTRACT: &str = "forever-product-v1";
pub const MAX_PLAN_BYTES: u64 = 128 * 1024;
pub const MAX_BINARY_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error {
    pub code: &'static str,
    pub component: Option<forever_casc::package::Component>,
}
impl From<&'static str> for Error {
    fn from(code: &'static str) -> Self {
        Self {
            code,
            component: None,
        }
    }
}
impl From<forever_casc::package::Error> for Error {
    fn from(error: forever_casc::package::Error) -> Self {
        error.code().into()
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for Error {}
