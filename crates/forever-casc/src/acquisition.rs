// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit shared acquisition selection and session.
//! Common acquisition infrastructure for versioned DB2, GT and camera batches.
//! Native remains the default and has no new provenance payload. Rust local
//! requires explicit index/ROOT layouts and never falls back to native or remote
//! IO. Open a session before creating outputs and reuse it for the entire batch.

mod options;
mod profile;

use crate::{Error, Locale, local_store};
pub use options::{OptionsBuilder, Selection};
pub use profile::{Profile, ProfileError};
use std::path::Path;

/// One owned backend session. A detected input change poisons all future reads,
/// in addition to the batch caller's obligation to abort and retain its marker.
pub struct Session {
    backend: Backend,
    build: u32,
    profile: Option<Profile>,
    changed: bool,
}
enum Backend {
    #[cfg(feature = "native")]
    Native {
        client: std::path::PathBuf,
        locale: Locale,
    },
    RustLocal(Box<local_store::LocalStore>),
    #[cfg(test)]
    Scripted {
        calls: usize,
    },
}
impl Session {
    pub fn open(client: &Path, locale: Locale, selection: Selection) -> Result<Self, Error> {
        match selection {
            Selection::Native => {
                #[cfg(feature = "native")]
                {
                    let build = crate::admit(client)?;
                    Ok(Self {
                        backend: Backend::Native {
                            client: client.to_path_buf(),
                            locale,
                        },
                        build,
                        profile: None,
                        changed: false,
                    })
                }
                #[cfg(not(feature = "native"))]
                {
                    Err(Error::BackendUnavailable)
                }
            }
            Selection::RustLocal(profile) => {
                let store = local_store::LocalStore::open(
                    client,
                    local_store::OpenOptions {
                        locale,
                        index_layouts: [profile.index_layout; 16],
                        root_layout: profile.root_layout,
                        limits: profile.limits(),
                    },
                )
                .map_err(Error::LocalStore)?;
                let build = store.build().build;
                Ok(Self {
                    backend: Backend::RustLocal(Box::new(store)),
                    build,
                    profile: Some(profile),
                    changed: false,
                })
            }
        }
    }
    pub const fn build(&self) -> u32 {
        self.build
    }
    pub const fn profile(&self) -> Option<Profile> {
        self.profile
    }
    pub fn read(&mut self, id: u32, budget: u32) -> Result<(u32, Vec<u8>), Error> {
        if self.changed {
            return Err(Error::LocalStore(local_store::Error::InputChanged));
        }
        let result = match &mut self.backend {
            #[cfg(feature = "native")]
            Backend::Native { client, locale } => {
                let file = crate::CascFile::open(client, *locale, id)?;
                let build = file.build();
                file.read_all(budget).map(|bytes| (build, bytes))
            }
            Backend::RustLocal(store) => store
                .read_file_data_id_with_limit(id, u64::from(budget))
                .map(|file| (self.build, file.bytes))
                .map_err(Error::LocalStore),
            #[cfg(test)]
            Backend::Scripted { calls } => {
                *calls += 1;
                Err(Error::LocalStore(local_store::Error::InputChanged))
            }
        };
        if result.as_ref().is_err_and(|error| error.is_input_changed()) {
            self.changed = true;
        }
        result
    }
}

#[cfg(test)]
mod tests;
