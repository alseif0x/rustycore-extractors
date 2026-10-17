// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: genuine compatible selected-model owner -> final VMaps.
//! Closed library inventory for the actually captured WDT domain. Missing WDT
//! and omitted available DB2 sections remain unknown; no full-world/product claim.
//! No caller placements, model identities, reports or strict owner conversion.
// Keep failure reporting allocation-free and retain every typed guard result.
#![expect(
    clippy::result_large_err,
    reason = "Preserve allocation-free Copy errors with typed causes and independent source/spool guards"
)]

use super::super::compatible as models;
pub use super::{Guards, Kind, Limits, MapView, Stats};
use crate::collision::{
    assemble as resident,
    payload::{self, CollectorError},
    spool,
};
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-compatible-assembled-vmap-package-v1";
pub const COVERAGE_PROFILE: &str = "linux-private-compatible-assembled-vmap-package-coverage-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E = ()> {
    Models(models::Cause),
    Kind(Kind),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E = ()> {
    pub cause: Cause<E>,
    pub guards: Guards,
}
fn source(c: Option<models::Cause>, collector: Option<CollectorError>) -> bool {
    let changed = CollectorError::Collision(crate::collision::Error::Store(
        crate::local_store::Error::InputChanged,
    ));
    collector == Some(changed)
        || match c {
            Some(models::Cause::Phase(c)) => {
                super::super::super::compatible::model_source_changed(c, collector)
            }
            Some(models::Cause::Collector(c)) => c == changed,
            _ => false,
        }
}
impl<E: Copy> Error<E> {
    pub fn code(self) -> &'static str {
        let cause = match self.cause {
            Cause::Models(c) => Some(c),
            _ => None,
        };
        if source(cause, self.guards.models.collector) {
            return crate::local_store::Error::InputChanged.code();
        }
        if self.guards.maps == Some(payload::Error::Changed)
            || self.guards.models.collector
                == Some(CollectorError::Payload(payload::Error::Changed))
            || self.guards.models.archive == Some(payload::Error::Changed)
            || self.guards.models.placements
                == Some(super::super::super::StageError::Spool(
                    payload::Error::Changed,
                ))
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Models(cause) => models::Error {
                cause,
                guards: self.guards.models,
            }
            .code(),
            Cause::Kind(e) => e.code(),
            Cause::Callback(_) => Kind::Callback.code(),
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
fn error<E>(e: super::WorkError<super::super::super::compatible::Cause, E>) -> Error<E> {
    Error {
        cause: match e.cause {
            super::WorkCause::Models(c) => Cause::Models(match c {
                super::super::WorkCause::Phase(e) => models::Cause::Phase(e),
                super::super::WorkCause::Kind(e) => models::Cause::Kind(e),
                super::super::WorkCause::Collector(e) => models::Cause::Collector(e),
                super::super::WorkCause::Callback(()) => models::Cause::Callback(()),
            }),
            super::WorkCause::Kind(e) => Cause::Kind(e),
            super::WorkCause::Callback(e) => Cause::Callback(e),
        },
        guards: e.guards,
    }
}
/// Only `assemble` constructs this owner from a genuine compatible Prepared.
/// It retains that owner and every RAW/orphan/stage/model/map reservation.
pub struct Completed {
    state: super::PackageState<models::Prepared>,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        if self.state.models.stage().world().has_coverage() {
            COVERAGE_PROFILE
        } else {
            PROFILE
        }
    }
    pub const fn prepared(&self) -> &models::Prepared {
        &self.state.models
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        super::phase_stats(&self.state.models, self.state.failed, self.state.stats).map_err(error)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        self.state.work().verify_unchanged().map_err(error)
    }
    /// Stable inventory of genuine final artifacts; no current source IO.
    pub fn artifacts(
        &self,
    ) -> Result<impl Iterator<Item = (resident::ArtifactName<'_>, usize)> + '_, Error> {
        super::artifact_inventory(
            &self.state.models,
            &self.state.maps,
            self.state.failed,
            self.state.stats,
        )
        .map_err(error)
    }
    /// Borrow the genuine world coverage under every retained package guard.
    /// No bundle reload, source acquisition or second traversal occurs.
    pub fn with_coverage<T, E: Copy>(
        &mut self,
        callback_heap: usize,
        call: impl FnOnce(
            crate::collision::run::compatible_inventory::coverage::View<'_>,
        ) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        let setup = (|| {
            let mut work = self.state.work();
            work.verify_unchanged().map_err(|e| super::WorkError {
                cause: match e.cause {
                    super::WorkCause::Models(c) => super::WorkCause::Models(c),
                    super::WorkCause::Kind(k) => super::WorkCause::Kind(k),
                    super::WorkCause::Callback(()) => super::WorkCause::Kind(Kind::Callback),
                },
                guards: e.guards,
            })?;
            work.admit(callback_heap).map_err(super::WorkError::from)
        })();
        if let Err(e) = setup {
            drop(call);
            return self.state.work().guarded(Err(e)).map_err(error);
        }
        let result = self
            .state
            .models
            .with_coverage(callback_heap, call)
            .map_err(|e| super::WorkError {
                cause: match e.cause {
                    models::Cause::Phase(c) => {
                        super::WorkCause::Models(super::super::WorkCause::Phase(c))
                    }
                    models::Cause::Kind(k) => {
                        super::WorkCause::Models(super::super::WorkCause::Kind(k))
                    }
                    models::Cause::Collector(c) => {
                        super::WorkCause::Models(super::super::WorkCause::Collector(c))
                    }
                    models::Cause::Callback(c) => super::WorkCause::Callback(c),
                },
                guards: Guards {
                    models: e.guards,
                    maps: None,
                },
            });
        self.state.work().guarded(result).map_err(error)
    }
    /// Same checked RCEPKG1 bundle reader. Coverage authority remains in this
    /// compatible owner; MapView never creates strict source/table facts.
    pub fn with_map<T, E: Copy>(
        &mut self,
        id: u32,
        callback_heap: usize,
        call: impl FnOnce(&MapView<'_>) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        self.state
            .work()
            .with_map(id, callback_heap, call)
            .map_err(error)
    }
    pub fn with_model<T, E: Copy>(
        &mut self,
        name: &[u8],
        callback_heap: usize,
        call: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        self.state
            .work()
            .with_model(name, callback_heap, call)
            .map_err(error)
    }
    /// One read per selected VMO/map bundle; captures and rejected values drop
    /// before independent collector/placement/VMO/map guards. No publication.
    pub fn visit<E: Copy>(
        &mut self,
        callback_heap: usize,
        call: impl FnMut(resident::Artifact<'_>) -> Result<(), E>,
    ) -> Result<(), Error<E>> {
        self.state.work().visit(callback_heap, call).map_err(error)
    }
}
pub fn assemble(
    models: models::Prepared,
    repository: spool::Spool,
    limits: Limits,
) -> Result<Completed, Error> {
    super::assemble_state(models, repository, limits)
        .map(|state| Completed { state })
        .map_err(error)
}
