// SPDX-License-Identifier: GPL-3.0-or-later
//! Target-admitted offline CASC read prototype, not a pure-Rust CASC engine.
pub mod acquisition;
mod admission;
pub mod archive_record;
pub mod batch;
pub mod blte;
pub mod build_config;
pub mod cameras;
pub mod collision;
pub mod encoding;
pub mod gametables;
pub mod integrity;
pub mod keys;
pub mod local_index;
pub mod local_store;
pub mod manifest;
pub mod options;
pub mod package;
pub mod root;
mod sha256;
pub mod terrain;
pub mod verify;
#[cfg(target_os = "linux")]
pub use admission::admit_db2_70374;
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
    BackendUnavailable,
    LocalStore(local_store::Error),
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
            Self::BackendUnavailable => "backend_unavailable",
            Self::LocalStore(error) => error.code(),
        }
    }
    pub const fn is_input_changed(self) -> bool {
        matches!(self, Self::LocalStore(local_store::Error::InputChanged))
    }
    pub fn exit_code(self) -> u8 {
        match self {
            Self::UnsupportedClient => 3,
            Self::Limit => 6,
            Self::InvalidId | Self::InvalidLocale | Self::BackendUnavailable => 2,
            Self::LocalStore(local_store::Error::UnsupportedClient) => 3,
            Self::LocalStore(
                local_store::Error::Limit
                | local_store::Error::Config(build_config::Error::Limit)
                | local_store::Error::Index(local_index::Error::Limit)
                | local_store::Error::Encoding(encoding::Error::Limit)
                | local_store::Error::Root(root::Error::Limit),
            ) => 6,
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
