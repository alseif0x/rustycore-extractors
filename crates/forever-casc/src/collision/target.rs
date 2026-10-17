// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: closed build admission, independent of omission policy.
use super::{Error, LocalStore};

/// Target admission is independent of the omission policy and DB2 coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Build70374,
    Build70425,
    Build70450,
}
impl Target {
    pub(super) fn check(self, store: &LocalStore) -> Result<(), Error> {
        match self {
            Self::Build70374 => store.check_available_70374(),
            Self::Build70425 => store.check_available_70425(),
            Self::Build70450 => store.check_available_70450(),
        }
        .map_err(Error::from_available)
    }
}
