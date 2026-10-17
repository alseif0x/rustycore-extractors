// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: selected VMO phase over a genuine compatible world Stage.
//! No supplied identities, placements, reports or strict Completed conversion.
//! This intermediate owner authenticates compatible MAGIC2 RAW and reuses the
//! private selection/VMO/BIH/archive kernels. It does not produce final VMaps.
// Keep failure reporting allocation-free and retain every typed guard result.
#![expect(
    clippy::result_large_err,
    reason = "Preserve allocation-free Copy errors with typed causes and independent source/spool guards"
)]

use super::super::compatible as placement;
pub use super::{Guards, Kind, Limits, Stats};
use super::{PreparedState, WorkCause, WorkError};
use crate::collision::{
    Class,
    payload::{self, CollectorError},
    spool,
};
use forever_vmap::vmo;
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-compatible-selected-vmo-stage-v1";
pub const COVERAGE_PROFILE: &str = "linux-private-compatible-selected-vmo-stage-coverage-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E = ()> {
    Phase(placement::Cause),
    Kind(Kind),
    Collector(CollectorError),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E = ()> {
    pub cause: Cause<E>,
    pub guards: Guards,
}
impl<E: Copy> Error<E> {
    pub fn code(self) -> &'static str {
        let phase_source = match self.cause {
            Cause::Phase(c) => placement::model_source_changed(c, self.guards.collector),
            Cause::Collector(c) => {
                c == CollectorError::Collision(crate::collision::Error::Store(
                    crate::local_store::Error::InputChanged,
                ))
            }
            _ => false,
        };
        if phase_source
            || self.guards.collector
                == Some(CollectorError::Collision(crate::collision::Error::Store(
                    crate::local_store::Error::InputChanged,
                )))
        {
            return crate::local_store::Error::InputChanged.code();
        }
        if self.guards.collector == Some(CollectorError::Payload(payload::Error::Changed))
            || self.guards.placements
                == Some(super::super::StageError::Spool(payload::Error::Changed))
            || self.guards.archive == Some(payload::Error::Changed)
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Kind(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Callback(_) => Kind::Callback.code(),
            Cause::Phase(cause) => placement::Error {
                cause,
                guards: super::super::Guards {
                    collector: self.guards.collector,
                    stage: self.guards.placements,
                    ..Default::default()
                },
            }
            .code(),
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
fn error<E>(e: WorkError<placement::Cause, E>) -> Error<E> {
    Error {
        cause: match e.cause {
            WorkCause::Phase(e) => Cause::Phase(e),
            WorkCause::Kind(e) => Cause::Kind(e),
            WorkCause::Collector(e) => Cause::Collector(e),
            WorkCause::Callback(e) => Cause::Callback(e),
        },
        guards: e.guards,
    }
}
/// Only `prepare` can construct this genuine compatible model archive.
pub struct Prepared {
    state: PreparedState<placement::Staged>,
}
impl Prepared {
    pub const fn profile(&self) -> &'static str {
        if self.state.stage.world().has_coverage() {
            COVERAGE_PROFILE
        } else {
            PROFILE
        }
    }
    pub const fn stage(&self) -> &placement::Staged {
        &self.state.stage
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        if let Some(kind) = self.state.failed {
            return Err(error(WorkError::from(kind)));
        }
        Ok(self.state.stats)
    }
    pub fn models(&self) -> Result<impl Iterator<Item = (&[u8], Class)> + '_, Error> {
        self.stats()?;
        super::selected_models(&self.state.stage, &self.state.index)
            .map_err(|e| error(WorkError::from(e)))
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        self.state.work().verify_unchanged().map_err(error)
    }
    /// The same genuine traversal coverage, with VMO/archive/placement guards.
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
                    WorkCause::Phase(c) => WorkCause::Phase(c),
                    WorkCause::Kind(k) => WorkCause::Kind(k),
                    WorkCause::Collector(c) => WorkCause::Collector(c),
                    WorkCause::Callback(()) => WorkCause::Kind(Kind::Callback),
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
            .stage
            .with_coverage(callback_heap, call)
            .map_err(|e| super::WorkError {
                cause: match e.cause {
                    placement::AccessCause::Access(c) => WorkCause::Phase(c),
                    placement::AccessCause::Callback(c) => WorkCause::Callback(c),
                },
                guards: Guards {
                    collector: e.guards.collector,
                    placements: e.guards.stage,
                    archive: None,
                },
            });
        self.state.work().guarded(result).map_err(error)
    }
    /// All captures/VMO reader bytes and rejected values drop before final
    /// genuine collector, placement and selected-VMO archive guards.
    pub fn with_model<T, E: Copy>(
        &mut self,
        name: &[u8],
        callback_heap: usize,
        call: impl FnOnce(&vmo::View<'_>) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        self.state
            .work()
            .with_model(name, callback_heap, call)
            .map_err(error)
    }
}
pub fn prepare(
    stage: placement::Staged,
    repository: spool::Spool,
    limits: Limits,
) -> Result<Prepared, Error> {
    super::prepare_state(stage, repository, limits)
        .map(|state| Prepared { state })
        .map_err(error)
}

// Private package seam: same genuine compatible Stage/VMO state, no fake strict owner.
impl super::package::Owner for Prepared {
    type Phase = placement::Staged;
    fn view(&self) -> super::package::ModelView<'_, placement::Staged> {
        super::package::ModelView {
            phase: &self.state.stage,
            repository: &self.state.repository,
            index: &self.state.index,
            index_capacity: self.state.index.capacity(),
            limits: self.state.limits,
            base: self.state.base,
            stats: self.state.stats,
            failed: self.state.failed,
            topology: self.state.stage.world().topology(),
        }
    }
    fn model_work(&mut self) -> super::ModelWork<'_, placement::Staged> {
        self.state.work()
    }
    fn stage_stats(&self) -> Result<super::super::Stats, super::WorkError<placement::Cause>> {
        self.state
            .stage
            .stats()
            .map_err(super::phase::<placement::Staged, ()>)
    }
}
