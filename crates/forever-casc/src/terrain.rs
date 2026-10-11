// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original source-backed terrain orchestration contracts.
//! Terrain planning and complete offline LocalStore-to-MAPS execution.
//! The runner loads complete startup DB2s and WDT/root ADTs, converts tiles and
//! emits whole-map tile lists to a provisional sink. Publication/manifest and
//! full real-client/runtime acceptance remain the caller's separate boundary.

pub mod plan;
pub mod run;
pub mod tilelist;
