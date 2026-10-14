// SPDX-License-Identifier: GPL-3.0-or-later
//! GT component admission, separate from backend acquisition capabilities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CatalogContract {
    #[default]
    Legacy,
    Build70338V1,
}
impl CatalogContract {
    pub const PROFILE: &str = "gt-catalog-70338-v1";

    /// Validate the caller's build before client, output or callback access.
    pub fn admit_build(self, build: u32) -> Result<(), &'static str> {
        let accepted = match self {
            Self::Legacy => matches!(build, 70170 | 70205),
            Self::Build70338V1 => build == 70338,
        };
        if accepted {
            Ok(())
        } else {
            Err("unsupported_client")
        }
    }
}
