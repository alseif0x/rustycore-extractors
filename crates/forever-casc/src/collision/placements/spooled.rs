// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original bounded disk-backed serial placement consumer.
//! One shared collector and ID allocator; one complete WMO reload per physical
//! request, no M2/child reloads. Sink writes remain provisional. Borrowed Objects
//! and source residency must be charged by the caller; counters are not RSS.
use super::{Batch, Error as ProjectionError, Limits, MAX_MAPS, MAX_SOURCE_REQUESTS};
use crate::collision::{
    Class, ModelHandle, ModelRequest, Status,
    payload::{self, CallbackFailure, CollectorError, CompactView, Descriptor, SpoolingExtractor},
};
use forever_vmap::{
    objects::Source,
    placement::{self, Placement, ids},
    wmo::{self, liquid::LiquidLookup},
};
use std::ops::Range;

#[cfg(all(test, target_os = "linux"))]
mod tests;
#[cfg(target_os = "linux")]
pub const PROFILE: &str = "linux-private-spooled-serial-placements-v1";
#[cfg(windows)]
pub const PROFILE: &str = "windows-private-spooled-serial-placements-v1";
const SCRATCH: usize = 62 + placement::MAX_NAME_BYTES;
/// The owned sink must bound ALL retained heap (including side buffers and
/// growth overlap) by its declared allowance. Disk publication is external.
/// Neither a callback nor a successful Session finish seals prior output.
pub trait Sink {
    type Error: Copy;
    fn stage(&mut self, batch: Staged<'_>) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy)]
pub struct Staged<'a> {
    pub map_id: u32,
    pub original_map_id: u32,
    pub records: usize,
    pub bytes: &'a [u8],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    Projection(ProjectionError),
    Collector(CollectorError),
    Sink(E),
    State,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E> {
    cause: Cause<E>,
    guard: Option<CollectorError>,
}
fn changed(e: CollectorError) -> bool {
    matches!(
        e,
        CollectorError::Payload(payload::Error::Changed)
            | CollectorError::Collision(crate::collision::Error::Store(
                crate::local_store::Error::InputChanged
            ))
    )
}
impl<E: Copy> Error<E> {
    pub const fn cause(self) -> Cause<E> {
        self.cause
    }
    pub const fn guard(self) -> Option<CollectorError> {
        self.guard
    }
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guard
            && changed(e)
        {
            return e.code();
        }
        match self.cause {
            Cause::Projection(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Sink(_) => "collision_spooled_placements_sink",
            Cause::State => "collision_spooled_placements_state",
        }
    }
    fn with_guard(mut self, guard: Option<CollectorError>) -> Self {
        if let Some(e) = guard
            && (changed(e) || !self.guard.is_some_and(changed))
        {
            self.guard = Some(e);
        }
        self
    }
    fn state() -> Self {
        Self {
            cause: Cause::State,
            guard: None,
        }
    }
}
impl<E> From<ProjectionError> for Error<E> {
    fn from(e: ProjectionError) -> Self {
        Self {
            cause: Cause::Projection(e),
            guard: None,
        }
    }
}
impl<E> From<CollectorError> for Error<E> {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guard: None,
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
fn bridge<E: Copy>(e: CallbackFailure<Error<E>>) -> Error<E> {
    match e {
        CallbackFailure::Collector(e) => e.into(),
        CallbackFailure::Callback { cause, guard } => cause.with_guard(guard),
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counters {
    requests: usize,
    records: usize,
    bytes: usize,
    metadata: usize,
    batches: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub source_requests: usize,
    pub records: usize,
    pub output_bytes: usize,
    pub metadata_visits: usize,
    pub staged_batches: usize,
    pub ids: ids::Stats,
}
/// Owns IDs and sink while exclusively borrowing the original collector.
/// Full ID ceiling + sink allowance + record scratch are charged throughout.
/// Failure or early Drop poisons the collector and retains conservative charges.
/// Success drops owned memory BEFORE exact release, then checks both guards.
pub struct Session<'c, 's, 'l, L: LiquidLookup + ?Sized, S: Sink> {
    collector: &'c mut SpoolingExtractor<'s, 'l, L>,
    allocator: Option<ids::IdAllocator>,
    sink: Option<S>,
    limits: Limits,
    reservation: usize,
    counters: Counters,
    failed: Option<Error<S::Error>>,
    completed: bool,
}
impl<'c, 's, 'l, L: LiquidLookup + ?Sized, S: Sink> Session<'c, 's, 'l, L, S> {
    pub fn new(
        collector: &'c mut SpoolingExtractor<'s, 'l, L>,
        sink: S,
        sink_heap_allowance: usize,
        limits: Limits,
    ) -> Result<Self, Error<S::Error>> {
        Self::new_precharged(collector, sink, sink_heap_allowance, 0, limits)
    }
    // Trusted consuming transfer from the world owner. The exact prior Sink
    // reservation is retained, not released or charged twice. No public caller
    // can declare an unbacked prior charge. Failure retains conservative charges.
    pub(crate) fn new_precharged(
        collector: &'c mut SpoolingExtractor<'s, 'l, L>,
        sink: S,
        sink_heap_allowance: usize,
        precharged: usize,
        limits: Limits,
    ) -> Result<Self, Error<S::Error>> {
        let result = (|| {
            limits.validate_resources(collector.resources())?;
            if precharged > sink_heap_allowance {
                return Err(ProjectionError::Limit.into());
            }
            let reservation = super::add(
                super::add(limits.ids.max_memory_bytes, sink_heap_allowance)?,
                SCRATCH,
            )?;
            if reservation > limits.max_memory_bytes {
                return Err(ProjectionError::Limit.into());
            }
            let allocator = ids::IdAllocator::new(limits.ids).map_err(ProjectionError::Ids)?;
            collector.reserve_external(reservation - precharged)?;
            Ok((allocator, reservation))
        })();
        let (allocator, reservation) =
            match collector.finish_external_operation(result).map_err(bridge) {
                Ok(value) => value,
                Err(error) => {
                    // Even a rejected constructor owns its Sink argument. Check
                    // source/spool again after dropping any callback-owned state.
                    drop(sink);
                    return collector
                        .finish_external_operation::<_, Error<S::Error>>(Err(error))
                        .map_err(bridge);
                }
            };
        Ok(Self {
            collector,
            allocator: Some(allocator),
            sink: Some(sink),
            limits,
            reservation,
            counters: Counters::default(),
            failed: None,
            completed: false,
        })
    }
    fn checked<T>(&mut self, result: Result<T, Error<S::Error>>) -> Result<T, Error<S::Error>> {
        let result = self
            .collector
            .finish_external_operation(result)
            .map_err(bridge);
        if let Err(e) = result {
            self.failed = Some(e);
        }
        result
    }
    pub fn extract(&mut self, request: ModelRequest<'_>) -> Result<ModelHandle, Error<S::Error>> {
        let result = match self.failed {
            Some(e) => Err(e),
            None => self.collector.extract(request).map_err(Error::from),
        };
        self.checked(result)
    }
    pub fn request_count(&mut self) -> Result<usize, Error<S::Error>> {
        let result = self
            .failed
            .map_or_else(|| self.collector.request_len().map_err(Error::from), Err);
        self.checked(result)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error<S::Error>> {
        self.checked(self.failed.map_or(Ok(()), Err))
    }
    // Trusted world-runner operations. Source/heap ownership stays charged
    // until the runner drops actual storage and releases the exact amount.
    // No public caller may release the Session's ID or owned Sink allowance.
    pub(crate) fn run_read_source(
        &mut self,
        input: crate::collision::InputRef<'_>,
        cap: usize,
    ) -> Result<Vec<u8>, Error<S::Error>> {
        let result = self.failed.map_or_else(
            || {
                self.collector
                    .read_source_ref(input, cap)
                    .map(|v| v.0)
                    .map_err(Error::from)
            },
            Err,
        );
        self.checked(result)
    }
    pub(crate) fn run_reserve(&mut self, bytes: usize) -> Result<(), Error<S::Error>> {
        let result = self.failed.map_or_else(
            || self.collector.reserve_external(bytes).map_err(Error::from),
            Err,
        );
        self.checked(result)
    }
    pub(crate) fn run_release(&mut self, bytes: usize) -> Result<(), Error<S::Error>> {
        let result = self.failed.map_or_else(
            || self.collector.release_external(bytes).map_err(Error::from),
            Err,
        );
        self.checked(result)
    }
    pub(crate) fn run_release_source(&mut self, bytes: usize) -> Result<(), Error<S::Error>> {
        let result = self.failed.map_or_else(
            || self.collector.release_source(bytes).map_err(Error::from),
            Err,
        );
        self.checked(result)
    }
    pub(crate) fn run_free_bytes(&mut self) -> Result<usize, Error<S::Error>> {
        let result = self
            .failed
            .map_or_else(|| self.collector.free_bytes().map_err(Error::from), Err);
        self.checked(result)
    }
    pub(crate) fn run_facts(&mut self) -> Result<crate::collision::Facts, Error<S::Error>> {
        let result = self
            .failed
            .map_or_else(|| self.collector.facts().map_err(Error::from), Err);
        self.checked(result)
    }
    pub(crate) fn run_with_sink<T>(
        &mut self,
        call: impl FnOnce(&mut S) -> Result<T, S::Error>,
    ) -> Result<T, Error<S::Error>> {
        let result = match self.failed {
            Some(e) => Err(e),
            None => match self.sink.as_mut() {
                Some(sink) => call(sink).map_err(|cause| Error {
                    cause: Cause::Sink(cause),
                    guard: None,
                }),
                None => Err(Error::state()),
            },
        };
        self.checked(result)
    }
    /// Observation only; this does not transfer ownership/accounting or seal.
    pub fn sink(&self) -> &S {
        self.sink.as_ref().expect("live session sink")
    }
    pub fn stats(&self) -> Stats {
        self.snapshot(self.allocator.as_ref().expect("live session IDs").stats())
    }
    fn snapshot(&self, ids: ids::Stats) -> Stats {
        Stats {
            source_requests: self.counters.requests,
            records: self.counters.records,
            output_bytes: self.counters.bytes,
            metadata_visits: self.counters.metadata,
            staged_batches: self.counters.batches,
            ids,
        }
    }
    pub fn project_to_sink(
        &mut self,
        window: Range<usize>,
        batches: &[Batch<'_, '_>],
    ) -> Result<(), Error<S::Error>> {
        let result = self.project_inner(window, batches);
        self.checked(result)
    }
    fn project_inner(
        &mut self,
        window: Range<usize>,
        batches: &[Batch<'_, '_>],
    ) -> Result<(), Error<S::Error>> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        let count = preflight(self.collector, &window, batches, self.limits)?;
        self.counters.requests = bounded(
            self.counters.requests,
            count,
            self.limits.max_source_requests,
        )?;
        // ALL top-level identities/status/shapes above precede IDs or reloads.
        let mut reloads = 0;
        let mut reload_bytes = 0;
        let mut rmax = 0;
        for ordinal in window.clone() {
            let handle = self.collector.request_handle(ordinal)?;
            let shape = self.collector.payload_shape(handle)?;
            if shape.class == Class::Wmo {
                reloads = super::add(reloads, 1)?;
                reload_bytes = super::add(reload_bytes, shape.record_bytes)?;
                rmax = rmax.max(shape.record_bytes);
            }
        }
        self.collector.admit_reloads(reloads, reload_bytes)?;
        // Keep the whole largest current record available while reserving one
        // vector bucket. Fixed SCRATCH is already in the lifetime reservation.
        let bucket = self
            .collector
            .free_bytes()?
            .checked_sub(rmax)
            .ok_or(ProjectionError::Limit)?
            .min(self.limits.max_memory_bytes - self.reservation)
            .min(self.limits.max_output_bytes.saturating_mul(2));
        self.collector.reserve_external(bucket)?;
        let mut ordinal = window.start;
        for batch in batches {
            let mut bytes = Vec::new();
            let before = self.counters.records;
            for request in batch.objects.requests() {
                let handle = self.collector.request_handle(ordinal)?;
                ordinal += 1;
                let context = placement::Context {
                    map_id: batch.map_id,
                    original_map_id: batch.original_map_id,
                    global_wmo: request.global_wmo(),
                };
                let allocator = self.allocator.as_mut().ok_or_else(Error::state)?;
                let limits = self.limits;
                let counters = &mut self.counters;
                match request.source() {
                    Source::M2(source) => self
                        .collector
                        .with_descriptors_result(|view| {
                            let model = view.descriptor(handle).map_err(CollectorError::from)?;
                            if model.status() == Status::Collision {
                                let id = allocator
                                    .assign(source.unique_id(), 0, false)
                                    .map_err(ProjectionError::Ids)?;
                                let value = placement::project_m2(
                                    &source,
                                    id,
                                    model.output_basename(),
                                    context,
                                    placement::Limits::default(),
                                )
                                .map_err(ProjectionError::Projection)?;
                                emit(value, &mut bytes, counters, limits, bucket)?;
                            }
                            Ok(())
                        })
                        .map_err(bridge)?,
                    Source::Wmo(source) => self
                        .collector
                        .with_payload_descriptors_result(handle, |payload, view| {
                            let model = view.descriptor(handle).map_err(CollectorError::from)?;
                            if payload.raw().is_some() != (model.status() == Status::Collision) {
                                return Err(ProjectionError::Model.into());
                            }
                            let metadata = payload.wmo().ok_or(ProjectionError::Model)?;
                            validate_bindings(&view, metadata, model.status(), counters)?;
                            if model.status() == Status::Collision {
                                let id = allocator
                                    .assign(source.unique_id(), 0, true)
                                    .map_err(ProjectionError::Ids)?;
                                let value = placement::project_wmo(
                                    &source,
                                    id,
                                    model.output_basename(),
                                    context,
                                    placement::Limits::default(),
                                )
                                .map_err(ProjectionError::Projection)?;
                                emit(value, &mut bytes, counters, limits, bucket)?;
                            }
                            let mut doodad_id = 0;
                            for selected in metadata.set_references(source.doodad_set()) {
                                super::next_doodad(&mut doodad_id)?;
                                let spawn = metadata
                                    .root()
                                    .doodad_spawn(usize::from(selected.reference))
                                    .ok_or(ProjectionError::Binding)?;
                                let binding = metadata
                                    .binding(spawn.name_index)
                                    .ok_or(ProjectionError::Binding)?;
                                let child = view
                                    .descriptor(binding.model)
                                    .map_err(CollectorError::from)?;
                                if child.class() != Class::M2 || child.status() != Status::Collision
                                {
                                    return Err(ProjectionError::Binding.into());
                                }
                                let id = allocator
                                    .assign(source.unique_id(), doodad_id, false)
                                    .map_err(ProjectionError::Ids)?;
                                let value = placement::doodads::project(
                                    &source,
                                    &spawn,
                                    id,
                                    child.output_basename(),
                                    context,
                                    placement::Limits::default(),
                                )
                                .map_err(ProjectionError::Projection)?;
                                emit(value, &mut bytes, counters, limits, bucket)?;
                            }
                            Ok(())
                        })
                        .map_err(bridge)?,
                }
            }
            let result = self
                .sink
                .as_mut()
                .ok_or_else(Error::state)?
                .stage(Staged {
                    map_id: batch.map_id,
                    original_map_id: batch.original_map_id,
                    records: self.counters.records - before,
                    bytes: &bytes,
                })
                .map_err(|cause| Error {
                    cause: Cause::Sink(cause),
                    guard: None,
                });
            // The batch allocation stays charged through sink and both guards.
            self.collector
                .finish_external_operation(result)
                .map_err(bridge)?;
            self.counters.batches = super::add(self.counters.batches, 1)?;
            drop(bytes);
        }
        // No output buffers survive this release. Failed paths retain charges.
        self.collector.release_external(bucket)?;
        Ok(())
    }
    /// Drops the owned sink and allocator; does not publish staged output.
    pub fn finish(mut self) -> Result<Stats, Error<S::Error>> {
        if let Some(e) = self.failed {
            return self.checked(Err(e));
        }
        let stats = self.stats();
        drop(self.allocator.take());
        drop(self.sink.take());
        let result = self
            .collector
            .release_external(self.reservation)
            .map_err(Error::from);
        self.checked(result)?;
        self.completed = true;
        Ok(stats)
    }
}
impl<L: LiquidLookup + ?Sized, S: Sink> Drop for Session<'_, '_, '_, L, S> {
    fn drop(&mut self) {
        if !self.completed {
            drop(self.allocator.take());
            drop(self.sink.take());
            // No rollback/retry after partial IDs or provisional sink writes.
            let _ = self
                .collector
                .finish_external_operation::<(), _>(Err(Error::<S::Error>::state()));
        }
    }
}
fn bounded(value: usize, extra: usize, cap: usize) -> Result<usize, ProjectionError> {
    let next = super::add(value, extra)?;
    if next > cap {
        return Err(ProjectionError::Limit);
    }
    Ok(next)
}
fn descriptor_status(d: &Descriptor<'_>) -> Result<(), ProjectionError> {
    match (d.class(), d.status()) {
        (Class::M2, Status::Collision | Status::NoCollision)
        | (Class::Wmo, Status::Collision | Status::DoodadOnly) => Ok(()),
        _ => Err(ProjectionError::Model),
    }
}
fn preflight<E: Copy, L: LiquidLookup + ?Sized>(
    collector: &mut SpoolingExtractor<'_, '_, L>,
    window: &Range<usize>,
    batches: &[Batch<'_, '_>],
    limits: Limits,
) -> Result<usize, Error<E>> {
    if window.start > window.end || window.end > collector.request_len()? {
        return Err(ProjectionError::RequestCount.into());
    }
    if batches.len() > limits.max_batches {
        return Err(ProjectionError::Limit.into());
    }
    let mut maps = [0u32; MAX_MAPS];
    let mut map_count = 0;
    let mut count = 0;
    for batch in batches {
        count = bounded(
            count,
            batch.objects.requests().len(),
            limits.max_source_requests,
        )?;
        if !maps[..map_count].contains(&batch.map_id) {
            if map_count >= limits.max_maps {
                return Err(ProjectionError::Limit.into());
            }
            maps[map_count] = batch.map_id;
            map_count += 1;
        }
    }
    if count != window.end - window.start {
        return Err(ProjectionError::RequestCount.into());
    }
    collector
        .with_descriptors_result(|view| {
            if view.model_count() > limits.max_models {
                return Err(ProjectionError::Limit.into());
            }
            let mut ordinal = window.start;
            for batch in batches {
                for request in batch.objects.requests() {
                    let handle = view.request(ordinal).map_err(CollectorError::from)?;
                    ordinal += 1;
                    let class = match request.source() {
                        Source::M2(_) => Class::M2,
                        Source::Wmo(_) => Class::Wmo,
                    };
                    view.check_identity(handle, class, super::input(request.target()))?;
                    descriptor_status(&view.descriptor(handle).map_err(CollectorError::from)?)?;
                }
            }
            Ok(())
        })
        .map_err(bridge)?;
    // Readiness and immutable RAW/class shapes for EVERY top-level request.
    for ordinal in window.clone() {
        collector.payload_shape(collector.request_handle(ordinal)?)?;
    }
    Ok(count)
}
fn validate_bindings(
    view: &CompactView<'_>,
    metadata: &payload::Wmo<'_>,
    status: Status,
    counters: &mut Counters,
) -> Result<(), ProjectionError> {
    let root = metadata.root();
    let work = super::add(
        super::add(
            metadata.bindings().len(),
            metadata.references().len().saturating_mul(2),
        )?,
        super::add(
            super::add(metadata.set_count(), metadata.group_sources().len())?,
            root.doodad_name_bytes()
                .map_or(root.doodad_file_data_id_count(), <[u8]>::len),
        )?,
    )?;
    counters.metadata = bounded(counters.metadata, work, MAX_SOURCE_REQUESTS)?;
    if status == Status::DoodadOnly
        && (metadata.set_count() == 0 || metadata.references().len() == 0)
    {
        return Err(ProjectionError::Model);
    }
    let mut bindings = metadata.bindings();
    let mut previous = None;
    for request in wmo::doodads::requests(root) {
        let binding = bindings.next().ok_or(ProjectionError::Binding)?;
        if binding.name_index != request.name_index
            || previous.is_some_and(|p| p >= binding.name_index)
        {
            return Err(ProjectionError::Binding);
        }
        previous = Some(binding.name_index);
        let target = match request.target {
            wmo::doodads::Target::FileDataId(id) => crate::collision::InputRef::FileDataId(id),
            wmo::doodads::Target::Name(name) => crate::collision::InputRef::LogicalName(name),
        };
        view.check_identity(binding.model, Class::M2, target)?;
        descriptor_status(
            &view
                .descriptor(binding.model)
                .map_err(ProjectionError::Collision)?,
        )?;
    }
    if bindings.next().is_some() {
        return Err(ProjectionError::Binding);
    }
    Ok(())
}
fn emit(
    value: Placement<'_>,
    bytes: &mut Vec<u8>,
    counters: &mut Counters,
    limits: Limits,
    bucket: usize,
) -> Result<(), ProjectionError> {
    // Work remains consumed if any allocation/encoding later fails.
    counters.records = bounded(counters.records, 1, limits.max_records)?;
    let length = super::add(
        if value.bounds_bits().is_some() {
            62
        } else {
            38
        },
        value.name().len(),
    )?;
    counters.bytes = bounded(counters.bytes, length, limits.max_output_bytes)?;
    let required = super::add(bytes.len(), length)?;
    if required > bytes.capacity() {
        let next = required
            .max(bytes.capacity().saturating_mul(2))
            .min(limits.max_output_bytes);
        let old = bytes.capacity();
        if super::add(old, next)? > bucket {
            return Err(ProjectionError::Limit);
        }
        bytes
            .try_reserve_exact(next - bytes.len())
            .map_err(|_| ProjectionError::Allocation)?;
        if super::add(old, bytes.capacity())? > bucket {
            return Err(ProjectionError::Limit);
        }
    }
    let encoded = placement::encode(
        std::slice::from_ref(&value),
        placement::Limits {
            max_bytes: SCRATCH,
            max_records: 1,
            max_name_bytes: placement::MAX_NAME_BYTES,
        },
    )
    .map_err(ProjectionError::Projection)?;
    if encoded.capacity() > SCRATCH || encoded.len() != length {
        return Err(ProjectionError::Limit);
    }
    bytes.extend_from_slice(&encoded);
    Ok(())
}
