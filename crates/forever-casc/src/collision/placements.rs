// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: completed collision/object-placement bridge, no I/O.
// Modified 2026-10-11: serial ID sessions across complete projection calls.
//! Complete extraction requests must exactly match the supplied ordered object
//! batches, including class and canonical origin. One serial ID allocator spans
//! all batches and, with Session, successive calls. A missing/error result never
//! returns earlier output batches from that call and poisons the session.
//! Doodad references use the existing sorted deterministic selection order;
//! this is not a claim about C++ unordered_set iteration or thread scheduling.
//! Repeated placements consume serial IDs as fresh calls; parent-cache replay
//! without counter consumption, acquisition, publication and assembly are absent.
use super::{Class, Extraction, ExtractionView, InputRef, Model, Origin, Status, names};
use forever_vmap::{
    objects::{Objects, Source, Target},
    placement::{self, Placement, ids},
    wmo,
};
#[cfg(test)]
mod tests;

pub const MAX_BATCHES: usize = 4096;
pub const MAX_MAPS: usize = 256;
pub const MAX_MODELS: usize = 4096;
pub const MAX_SOURCE_REQUESTS: usize = 1_000_000;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Bridge limits apply independently to each complete projection call. `ids`
/// limits and allocator state apply across the entire lifetime of a Session.
pub struct Limits {
    pub max_batches: usize,
    pub max_maps: usize,
    pub max_models: usize,
    pub max_source_requests: usize,
    pub max_records: usize,
    pub max_output_bytes: usize,
    pub max_memory_bytes: usize,
    pub ids: ids::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_batches: MAX_BATCHES,
            max_maps: MAX_MAPS,
            max_models: MAX_MODELS,
            max_source_requests: MAX_SOURCE_REQUESTS,
            max_records: MAX_RECORDS,
            max_output_bytes: MAX_OUTPUT_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            ids: ids::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_batches > MAX_BATCHES
            || self.max_maps > MAX_MAPS
            || self.max_models > MAX_MODELS
            || self.max_source_requests > MAX_SOURCE_REQUESTS
            || self.max_records > MAX_RECORDS
            || self.max_output_bytes > MAX_OUTPUT_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}
/// Metadata and Objects are borrowed and excluded from this bridge's own memory
/// budget. The global-WMO flag comes from each validated source request.
pub struct Batch<'o, 'data> {
    pub objects: &'o Objects<'data>,
    pub map_id: u32,
    pub original_map_id: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    RequestCount,
    Identity,
    Model,
    Binding,
    DoodadCount,
    Collision(super::Error),
    Projection(placement::Error),
    Ids(ids::Error),
    Wmo(wmo::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_placements_invalid_limits",
            Self::Limit => "collision_placements_policy_limit",
            Self::Allocation => "collision_placements_allocation",
            Self::RequestCount => "collision_placements_request_count",
            Self::Identity => "collision_placements_identity",
            Self::Model => "collision_placements_model",
            Self::Binding => "collision_placements_binding",
            Self::DoodadCount => "collision_placements_doodad_count",
            Self::Collision(e) => e.code(),
            Self::Projection(e) => e.code(),
            Self::Ids(e) => e.code(),
            Self::Wmo(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
pub struct BatchOutput {
    map_id: u32,
    original_map_id: u32,
    records: usize,
    bytes: Vec<u8>,
}
impl BatchOutput {
    pub const fn map_id(&self) -> u32 {
        self.map_id
    }
    pub const fn original_map_id(&self) -> u32 {
        self.original_map_id
    }
    pub const fn record_count(&self) -> usize {
        self.records
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub source_requests: usize,
    pub records: usize,
    pub maps: usize,
    pub output_bytes: usize,
    pub resident_bytes: usize,
    pub charged_peak_bytes: usize,
    /// Cumulative allocator snapshot, including previous successful calls.
    pub ids: ids::Stats,
}
pub struct Output {
    batches: Vec<BatchOutput>,
    stats: Stats,
}
impl Output {
    pub fn batches(&self) -> &[BatchOutput] {
        &self.batches
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}

/// Serial placement projection with one ID allocator across calls. Every error
/// is sticky; failed calls expose no partial Output and cannot be retried with
/// the partially advanced allocator. Already returned outputs remain valid.
///
/// Per-call memory reserves the full configured ID memory ceiling (128 MiB by
/// default), including allocator growth overlap. Outputs retained by the caller
/// from prior calls, borrowed Extraction/Objects, stack and allocator overhead
/// are excluded. This is neither a process RSS bound nor a cumulative output
/// cap, filename cache, or acquisition cache across calls.
pub struct Session {
    limits: Limits,
    allocator: ids::IdAllocator,
    failed: Option<Error>,
}
impl Session {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let allocator = ids::IdAllocator::new(limits.ids).map_err(Error::Ids)?;
        Ok(Self {
            limits,
            allocator,
            failed: None,
        })
    }
    pub fn project(
        &mut self,
        extraction: &Extraction,
        batches: &[Batch<'_, '_>],
    ) -> Result<Output, Error> {
        self.project_view(extraction.view(), batches)
    }
    /// Every dependency handle resolves against all models; only this view's
    /// ordered request window is projected. The caller owns its live collector.
    pub(crate) fn project_view(
        &mut self,
        extraction: ExtractionView<'_>,
        batches: &[Batch<'_, '_>],
    ) -> Result<Output, Error> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        let result = project(extraction, batches, self.limits, &mut self.allocator);
        if let Err(error) = &result {
            self.failed = Some(*error);
        }
        result
    }
    pub const fn id_stats(&self) -> ids::Stats {
        self.allocator.stats()
    }
    pub const fn failed(&self) -> Option<Error> {
        self.failed
    }
}

/// Reserve the full configured ID memory ceiling, even for empty output; this
/// conservatively covers every later allocator rehash without weakening its
/// API. Reduce `ids.max_memory_bytes` together with a small bridge memory cap.
/// Other charged capacities: root-view cache, one batch's placement scratch,
/// retained output descriptors/bytes and overlap during vector growth. Borrowed
/// Extraction/Objects, stack and allocator overhead are excluded.
pub fn build(
    extraction: &Extraction,
    batches: &[Batch<'_, '_>],
    limits: Limits,
) -> Result<Output, Error> {
    Session::new(limits)?.project(extraction, batches)
}
fn project(
    extraction: ExtractionView<'_>,
    batches: &[Batch<'_, '_>],
    limits: Limits,
    allocator: &mut ids::IdAllocator,
) -> Result<Output, Error> {
    if batches.len() > limits.max_batches || extraction.models.len() > limits.max_models {
        return Err(Error::Limit);
    }
    let mut maps = [0u32; MAX_MAPS];
    let mut map_count = 0;
    let mut expected = 0usize;
    for batch in batches {
        expected = add(expected, batch.objects.requests().len())?;
        if expected > limits.max_source_requests {
            return Err(Error::Limit);
        }
        if !maps[..map_count].contains(&batch.map_id) {
            if map_count >= limits.max_maps {
                return Err(Error::Limit);
            }
            maps[map_count] = batch.map_id;
            map_count += 1;
        }
    }
    if expected != extraction.requests.len() {
        return Err(Error::RequestCount);
    }
    let mut ordinal = 0;
    for batch in batches {
        for request in batch.objects.requests() {
            let model = model_for(extraction, ordinal)?;
            let class = match request.source() {
                Source::M2(_) => Class::M2,
                Source::Wmo(_) => Class::Wmo,
            };
            check_identity(model, class, input(request.target()))?;
            check_status(model)?;
            ordinal += 1;
        }
    }
    let mut budget = Budget {
        live: limits.ids.max_memory_bytes,
        peak: limits.ids.max_memory_bytes,
        cap: limits.max_memory_bytes,
    };
    budget.ensure(0)?;
    let mut roots = Vec::new();
    reserve(
        &mut roots,
        extraction.models.len(),
        extraction.models.len(),
        &mut budget,
    )?;
    roots.resize_with(extraction.models.len(), || None);
    let mut output = Vec::new();
    reserve(&mut output, batches.len(), batches.len(), &mut budget)?;
    let mut records = 0;
    let mut bytes_total = 0;
    ordinal = 0;
    for batch in batches {
        let mut projected = Vec::new();
        for request in batch.objects.requests() {
            let handle = extraction.requests[ordinal];
            let model = model_for(extraction, ordinal)?;
            ordinal += 1;
            let context = placement::Context {
                map_id: batch.map_id,
                original_map_id: batch.original_map_id,
                global_wmo: request.global_wmo(),
            };
            match request.source() {
                Source::M2(source) => {
                    if model.status == Status::Collision {
                        let id = allocator
                            .assign(source.unique_id(), 0, false)
                            .map_err(Error::Ids)?;
                        let value = placement::project_m2(
                            &source,
                            id,
                            model.output_basename(),
                            context,
                            placement::Limits::default(),
                        )
                        .map_err(Error::Projection)?;
                        push(
                            &mut projected,
                            value,
                            &mut records,
                            &mut bytes_total,
                            limits,
                            &mut budget,
                        )?;
                    }
                }
                Source::Wmo(source) => {
                    let metadata = model.wmo_metadata().ok_or(Error::Model)?;
                    if roots[handle.index()].is_none() {
                        let root = metadata.root().map_err(Error::Wmo)?;
                        check_bindings(extraction, metadata, &root)?;
                        roots[handle.index()] = Some(root);
                    }
                    let root = roots[handle.index()].as_ref().ok_or(Error::Model)?;
                    if model.status == Status::Collision {
                        let id = allocator
                            .assign(source.unique_id(), 0, true)
                            .map_err(Error::Ids)?;
                        let value = placement::project_wmo(
                            &source,
                            id,
                            model.output_basename(),
                            context,
                            placement::Limits::default(),
                        )
                        .map_err(Error::Projection)?;
                        push(
                            &mut projected,
                            value,
                            &mut records,
                            &mut bytes_total,
                            limits,
                            &mut budget,
                        )?;
                    }
                    let mut doodad_id = 0u16;
                    for selected in metadata.selection().set_references(source.doodad_set()) {
                        next_doodad(&mut doodad_id)?;
                        let spawn = root
                            .doodad_spawn(usize::from(selected.reference))
                            .ok_or(Error::Binding)?;
                        let at = metadata
                            .bindings()
                            .binary_search_by_key(&spawn.name_index, |b| b.name_index)
                            .map_err(|_| Error::Binding)?;
                        let child = extraction
                            .model(metadata.bindings()[at].model)
                            .ok_or(Error::Binding)?;
                        if child.class != Class::M2 || child.status != Status::Collision {
                            return Err(Error::Binding);
                        }
                        let id = allocator
                            .assign(source.unique_id(), doodad_id, false)
                            .map_err(Error::Ids)?;
                        let value = placement::doodads::project(
                            &source,
                            &spawn,
                            id,
                            child.output_basename(),
                            context,
                            placement::Limits::default(),
                        )
                        .map_err(Error::Projection)?;
                        push(
                            &mut projected,
                            value,
                            &mut records,
                            &mut bytes_total,
                            limits,
                            &mut budget,
                        )?;
                    }
                }
            }
        }
        let bytes = placement::encode(
            &projected,
            placement::Limits {
                max_bytes: limits.max_output_bytes.min(budget.free()?),
                max_records: limits.max_records,
                max_name_bytes: placement::MAX_NAME_BYTES,
            },
        )
        .map_err(Error::Projection)?;
        budget.charge(bytes.capacity())?;
        output.push(BatchOutput {
            map_id: batch.map_id,
            original_map_id: batch.original_map_id,
            records: projected.len(),
            bytes,
        });
        budget.live -= capacity(&projected)?;
    }
    let resident = output
        .iter()
        .try_fold(capacity(&output)?, |sum, b| add(sum, b.bytes.capacity()))?;
    let stats = Stats {
        source_requests: expected,
        records,
        maps: map_count,
        output_bytes: bytes_total,
        resident_bytes: resident,
        charged_peak_bytes: budget.peak,
        ids: allocator.stats(),
    };
    Ok(Output {
        batches: output,
        stats,
    })
}
fn input(target: Target<'_>) -> InputRef<'_> {
    match target {
        Target::FileDataId(id) => InputRef::FileDataId(id),
        Target::Name(name) => InputRef::LogicalName(name),
    }
}
fn model_for(extraction: ExtractionView<'_>, ordinal: usize) -> Result<&Model, Error> {
    extraction
        .requests
        .get(ordinal)
        .and_then(|h| extraction.model(*h))
        .ok_or(Error::Identity)
}
fn check_identity(model: &Model, class: Class, input: InputRef<'_>) -> Result<(), Error> {
    let prepared = names::Prepared::new(input, class).map_err(Error::Collision)?;
    let same = match (&model.origin, prepared.id) {
        (Origin::Id(a), Some(b)) => *a == b,
        (Origin::Name(a), None) => a == prepared.canonical(),
        _ => false,
    };
    if model.class != class || model.pending || !same || model.output_basename() != prepared.name()
    {
        return Err(Error::Identity);
    }
    Ok(())
}
fn check_status(model: &Model) -> Result<(), Error> {
    match (
        model.class,
        model.status,
        model.raw.is_some(),
        model.wmo.is_some(),
    ) {
        (Class::M2, Status::Collision, true, false)
        | (Class::M2, Status::NoCollision, false, false)
        | (Class::Wmo, Status::Collision, true, true) => Ok(()),
        (Class::Wmo, Status::DoodadOnly, false, true)
            if model.wmo.as_ref().is_some_and(|metadata| {
                metadata.selection.outcome(false) == wmo::doodads::Outcome::OkNoCollision
            }) =>
        {
            Ok(())
        }
        _ => Err(Error::Model),
    }
}
fn next_doodad(value: &mut u16) -> Result<u16, Error> {
    let next = value.checked_add(1).ok_or(Error::DoodadCount)?;
    *value = next;
    Ok(next)
}
fn check_bindings(
    extraction: ExtractionView<'_>,
    metadata: &super::WmoMetadata,
    root: &wmo::Root<'_>,
) -> Result<(), Error> {
    let mut count = 0;
    for request in wmo::doodads::requests(root) {
        let binding = metadata.bindings().get(count).ok_or(Error::Binding)?;
        if binding.name_index != request.name_index {
            return Err(Error::Binding);
        }
        let child = extraction.model(binding.model).ok_or(Error::Binding)?;
        let input = match request.target {
            wmo::doodads::Target::FileDataId(id) => InputRef::FileDataId(id),
            wmo::doodads::Target::Name(name) => InputRef::LogicalName(name),
        };
        check_identity(child, Class::M2, input)?;
        check_status(child)?;
        count += 1;
    }
    if count != metadata.bindings().len() {
        return Err(Error::Binding);
    }
    Ok(())
}
fn push<'a>(
    records: &mut Vec<Placement<'a>>,
    value: Placement<'a>,
    total: &mut usize,
    bytes: &mut usize,
    limits: Limits,
    budget: &mut Budget,
) -> Result<(), Error> {
    if *total >= limits.max_records {
        return Err(Error::Limit);
    }
    let length = add(
        if value.bounds_bits().is_some() {
            62
        } else {
            38
        },
        value.name().len(),
    )?;
    let next = add(*bytes, length)?;
    if next > limits.max_output_bytes {
        return Err(Error::Limit);
    }
    reserve(records, 1, limits.max_records, budget)?;
    records.push(value);
    *total += 1;
    *bytes = next;
    Ok(())
}
struct Budget {
    live: usize,
    peak: usize,
    cap: usize,
}
impl Budget {
    fn ensure(&self, extra: usize) -> Result<usize, Error> {
        let total = add(self.live, extra)?;
        if total > self.cap {
            return Err(Error::Limit);
        }
        Ok(total)
    }
    fn free(&self) -> Result<usize, Error> {
        self.cap.checked_sub(self.live).ok_or(Error::Limit)
    }
    fn charge(&mut self, n: usize) -> Result<(), Error> {
        self.live = self.ensure(n)?;
        self.peak = self.peak.max(self.live);
        Ok(())
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn capacity<T>(v: &Vec<T>) -> Result<usize, Error> {
    v.capacity().checked_mul(size_of::<T>()).ok_or(Error::Limit)
}
fn reserve<T>(
    v: &mut Vec<T>,
    additional: usize,
    count_cap: usize,
    budget: &mut Budget,
) -> Result<(), Error> {
    let required = add(v.len(), additional)?;
    if required <= v.capacity() {
        return Ok(());
    }
    if required > count_cap {
        return Err(Error::Limit);
    }
    let next = required
        .max(v.capacity().saturating_mul(2))
        .max(8)
        .min(count_cap);
    budget.ensure(next.checked_mul(size_of::<T>()).ok_or(Error::Limit)?)?;
    let old = capacity(v)?;
    v.try_reserve_exact(next - v.len())
        .map_err(|_| Error::Allocation)?;
    let actual = capacity(v)?;
    budget.peak = budget.peak.max(budget.ensure(actual)?);
    budget.live = budget.live - old + actual;
    Ok(())
}
