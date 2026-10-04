// SPDX-License-Identifier: GPL-3.0-or-later
//! Target-admitted offline CASC read prototype, not a pure-Rust CASC engine.
mod admission;
pub mod batch;
pub mod manifest;
pub mod options;
mod sha256;
pub use admission::{PRODUCT, SUPPORTED_CLIENTS, admit};

#[cfg(feature = "native")]
#[allow(unsafe_code)]
mod ffi;
#[cfg(feature = "native")]
pub use ffi::CascFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Input,
    Metadata,
    UnsupportedClient,
    AmbiguousClient,
    Native,
    MissingFile,
    Encrypted,
    Incomplete,
    Limit,
    InvalidId,
    InvalidLocale,
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Input => "input_io",
            Self::Metadata => "invalid_build_metadata",
            Self::UnsupportedClient => "unsupported_client",
            Self::AmbiguousClient => "ambiguous_active_client",
            Self::Native => "native_read_failed",
            Self::MissingFile => "file_unavailable",
            Self::Encrypted => "encrypted_key_unavailable",
            Self::Incomplete => "local_file_incomplete",
            Self::Limit => "read_policy_limit",
            Self::InvalidId => "invalid_file_data_id",
            Self::InvalidLocale => "unsupported_locale",
        }
    }
    pub fn exit_code(self) -> u8 {
        match self {
            Self::UnsupportedClient => 3,
            Self::Limit => 6,
            Self::InvalidId | Self::InvalidLocale => 2,
            _ => 4,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    EsEs,
}
impl Locale {
    pub fn parse(value: &str) -> Result<Self, Error> {
        match value {
            "enUS" => Ok(Self::EnUs),
            "esES" => Ok(Self::EsEs),
            _ => Err(Error::InvalidLocale),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::EnUs => "enUS",
            Self::EsEs => "esES",
        }
    }
    #[cfg(feature = "native")]
    fn mask(self) -> u32 {
        match self {
            Self::EnUs => 2,
            Self::EsEs => 0x80,
        }
    }
}
