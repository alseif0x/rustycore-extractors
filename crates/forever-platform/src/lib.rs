// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original platform filesystem primitives.

#[cfg(all(windows, not(target_arch = "x86_64")))]
compile_error!("forever-platform Windows profile requires x86_64");

pub mod path;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(windows)]
pub mod windows;

/// A static diagnostic; deliberately contains no filenames or OS error strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Limit,
    Open,
    Missing,
    FileType,
    Reparse,
    HardLinks,
    Identity,
    Changed,
    Read,
    Allocation,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Path => "platform_path",
            Self::Limit => "platform_limit",
            Self::Open => "platform_open",
            Self::Missing => "platform_missing",
            Self::FileType => "platform_file_type",
            Self::Reparse => "platform_reparse",
            Self::HardLinks => "platform_hardlinks",
            Self::Identity => "platform_identity",
            Self::Changed => "platform_changed",
            Self::Read => "platform_read",
            Self::Allocation => "platform_allocation",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
