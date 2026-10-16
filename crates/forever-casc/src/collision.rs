// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded offline acquisition-to-raw collision pipeline.
//! Reads complete local files and converts them now; no files are published.
//! All outputs stay owned until successful finish. Names require the existing
//! explicit LocalStore opt-in, and the legacy acquisition profile is unchanged.
//! Every source acquisition is complete and verified; missing doodads, groups,
//! malformed data, ambiguous basename aliases and budgets fail the collector.
//! An M2 without collision is retained explicitly but cannot validate a doodad.
//! A WMO without raw geometry requires both sets and valid group references.
//! The caller supplies liquid lookups; this module never loads DB2 implicitly.
//! Lookup transitions retain the same store borrow, cache, counters and failures.
//! GameObject requests classify the first four bytes of the complete verified
//! source. Typed requests retain their existing parser admission rules.
//! Neither placement publication, VMap assembly nor runtime loading occurs here.
pub mod assemble;
#[cfg(target_os = "linux")]
pub mod available_70374;
#[cfg(target_os = "linux")]
pub mod compatible;
pub mod gameobjects;
#[cfg(target_os = "linux")]
pub mod inventory_spool;
mod names;
#[cfg(target_os = "linux")]
pub mod payload;
pub mod placements;
pub mod plan;
#[cfg(target_os = "linux")]
pub mod resources;
pub mod run;
#[cfg(target_os = "linux")]
pub mod spool;
#[cfg(target_os = "linux")]
mod target;
#[cfg(test)]
mod tests;
pub mod world;

use crate::local_store::{self, LocalStore};
use forever_vmap::{
    M2Collision, RawM2,
    wmo::{self, doodads, liquid::LiquidLookup, raw},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    M2,
    Wmo,
}
#[derive(Clone, Copy)]
pub enum InputRef<'a> {
    FileDataId(u32),
    LogicalName(&'a [u8]),
}
#[derive(Clone, Copy)]
pub enum ModelRequest<'a> {
    M2(InputRef<'a>),
    Wmo(InputRef<'a>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelHandle(usize);
impl ModelHandle {
    /// Stable insertion ordinal within this extraction's model collection.
    pub const fn index(self) -> usize {
        self.0
    }
}
/// Classification of a complete GameObject source. Textures create no model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectOutcome {
    Model(ModelHandle),
    Texture,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Collision,
    NoCollision,
    DoodadOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_file_bytes: usize,
    pub max_reads: usize,
    pub max_models: usize,
    /// Top-level typed and GameObject calls, including textures and cache hits.
    pub max_requests: usize,
    /// Cumulative MODN/MODI requests, including cache hits; MODI zero is absent.
    pub max_dependency_requests: usize,
    /// Cumulative complete acquired bytes, without refunds when sources drop.
    pub max_source_bytes: usize,
    /// Sum of retained raw lengths; a reused cache entry is counted only once.
    pub max_raw_bytes: usize,
    /// Store resident memory plus all live collector vector capacities and
    /// current acquisition/conversion scratch. Excludes stack/allocator overhead
    /// and the caller-owned lookup, whose lifetime is outside this collection.
    pub max_memory_bytes: usize,
    pub m2: forever_vmap::Limits,
    pub wmo: wmo::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024 * 1024,
            max_reads: 4096,
            max_models: 4096,
            max_requests: 4096,
            max_dependency_requests: 4096,
            max_source_bytes: 256 * 1024 * 1024,
            max_raw_bytes: 256 * 1024 * 1024,
            max_memory_bytes: 1024 * 1024 * 1024,
            m2: forever_vmap::Limits::default(),
            wmo: wmo::Limits::default(),
        }
    }
}
impl Limits {
    #[cfg(target_os = "linux")]
    fn validate_resources(self, profile: resources::Profile) -> Result<(), Error> {
        let cap = profile.collector_limits();
        if !profile.supported()
            || self.max_reads > cap.max_reads
            || self.max_models > cap.max_models
            || self.max_requests > cap.max_requests
            || self.max_dependency_requests > cap.max_dependency_requests
            || self.max_source_bytes > cap.max_source_bytes
            || self.max_raw_bytes > cap.max_raw_bytes
        {
            return Err(Error::InvalidLimits);
        }
        let d = Self::default();
        let mut bounded = self;
        bounded.max_reads = bounded.max_reads.min(d.max_reads);
        bounded.max_models = bounded.max_models.min(d.max_models);
        bounded.max_requests = bounded.max_requests.min(d.max_requests);
        bounded.max_dependency_requests = bounded
            .max_dependency_requests
            .min(d.max_dependency_requests);
        bounded.max_source_bytes = bounded.max_source_bytes.min(d.max_source_bytes);
        bounded.max_raw_bytes = bounded.max_raw_bytes.min(d.max_raw_bytes);
        bounded.validate()
    }
    fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.max_file_bytes > d.max_file_bytes
            || self.max_reads > d.max_reads
            || self.max_models > d.max_models
            || self.max_requests > d.max_requests
            || self.max_dependency_requests > d.max_dependency_requests
            || self.max_source_bytes > d.max_source_bytes
            || self.max_raw_bytes > d.max_raw_bytes
            || self.max_memory_bytes > d.max_memory_bytes
            || self.m2.max_input_bytes > d.m2.max_input_bytes
            || self.m2.max_vertices > d.m2.max_vertices
            || self.m2.max_indices > d.m2.max_indices
            || self.wmo.max_input_bytes > d.wmo.max_input_bytes
            || self.wmo.max_vertices > d.wmo.max_vertices
            || self.wmo.max_indices > d.wmo.max_indices
            || self.wmo.max_chunks > d.wmo.max_chunks
            || self.wmo.max_groups > d.wmo.max_groups
            || self.wmo.max_metadata_records > d.wmo.max_metadata_records
            || self.wmo.max_projection_bytes > d.wmo.max_projection_bytes
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}

/// Availability-only failures without another nested LocalStore error payload.
/// Store failures retain the existing `Error::Store` variant and typed details.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AvailableError {
    Blte(crate::blte::Error),
    KeysPresent,
    Build,
    AmbiguousEncoding,
    UnsupportedArchive,
}
#[cfg(target_os = "linux")]
impl AvailableError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Blte(e) => e.code(),
            Self::KeysPresent => "available_source_keys_not_admitted",
            Self::Build => "available_source_build_not_admitted",
            Self::AmbiguousEncoding => "available_source_ambiguous_encoding",
            Self::UnsupportedArchive => "available_source_archive_not_admitted",
        }
    }
}
#[cfg(target_os = "linux")]
impl std::fmt::Display for AvailableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
#[cfg(target_os = "linux")]
impl std::error::Error for AvailableError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    Name,
    GroupName,
    Alias,
    Cycle,
    NoCollision,
    GameObjectMagic,
    State,
    Aborted,
    Store(local_store::Error),
    #[cfg(target_os = "linux")]
    Available(AvailableError),
    M2(forever_vmap::Error),
    Wmo(wmo::Error),
}
impl Error {
    /// Flatten existing store failures; no allocation or loss of candidate facts.
    #[cfg(target_os = "linux")]
    pub const fn from_available(error: local_store::availability::Error) -> Self {
        use local_store::availability::Error as SourceError;
        match error {
            SourceError::Store(e) => Self::Store(e),
            SourceError::Blte(e) => Self::Available(AvailableError::Blte(e)),
            SourceError::KeysPresent => Self::Available(AvailableError::KeysPresent),
            SourceError::Build => Self::Available(AvailableError::Build),
            SourceError::AmbiguousEncoding => Self::Available(AvailableError::AmbiguousEncoding),
            SourceError::UnsupportedArchive => Self::Available(AvailableError::UnsupportedArchive),
        }
    }
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_invalid_limits",
            Self::Limit => "collision_policy_limit",
            Self::Allocation => "collision_allocation",
            Self::Name => "collision_invalid_name",
            Self::GroupName => "collision_group_name",
            Self::Alias => "collision_output_alias",
            Self::Cycle => "collision_cycle",
            Self::NoCollision => "collision_no_valid_collision",
            Self::GameObjectMagic => "collision_gameobject_magic",
            Self::State => "collision_invalid_state",
            Self::Aborted => "collision_external_run_failure",
            Self::Store(e) => e.code(),
            #[cfg(target_os = "linux")]
            Self::Available(e) => e.code(),
            Self::M2(e) => e.code(),
            Self::Wmo(e) => e.code(),
        }
    }
}
#[cfg(target_os = "linux")]
impl From<local_store::availability::Error> for Error {
    fn from(error: local_store::availability::Error) -> Self {
        Self::from_available(error)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

enum Origin {
    Id(u32),
    Name(Vec<u8>),
}
pub struct SourceFacts {
    id: u32,
    key: [u8; 16],
    bytes: usize,
    first_magic: Option<[u8; 4]>,
}
impl SourceFacts {
    pub const fn file_data_id(&self) -> u32 {
        self.id
    }
    pub const fn content_key(&self) -> [u8; 16] {
        self.key
    }
    pub const fn byte_count(&self) -> usize {
        self.bytes
    }
    /// First four source bytes, including dependency reads; never a later chunk.
    pub const fn first_magic(&self) -> Option<[u8; 4]> {
        self.first_magic
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameBinding {
    pub name_index: u32,
    pub model: ModelHandle,
}
pub struct WmoMetadata {
    root_bytes: Vec<u8>,
    bindings: Vec<NameBinding>,
    selection: doodads::Selection,
    groups: Vec<SourceFacts>,
    limits: wmo::Limits,
}
impl WmoMetadata {
    /// Immutable complete source retains MODS names/padding, MODD bits and the
    /// original MODN offsets/MODI ordinals, without self-referential borrows.
    pub fn root(&self) -> Result<wmo::Root<'_>, wmo::Error> {
        wmo::Root::parse(&self.root_bytes, self.limits)
    }
    pub fn bindings(&self) -> &[NameBinding] {
        &self.bindings
    }
    pub const fn selection(&self) -> &doodads::Selection {
        &self.selection
    }
    pub fn group_sources(&self) -> &[SourceFacts] {
        &self.groups
    }
}
pub struct Model {
    class: Class,
    name: Vec<u8>,
    origin: Origin,
    pending: bool,
    status: Status,
    raw: Option<Vec<u8>>,
    source: Option<SourceFacts>,
    wmo: Option<WmoMetadata>,
}
impl Model {
    pub const fn class(&self) -> Class {
        self.class
    }
    pub fn output_basename(&self) -> &[u8] {
        &self.name
    }
    pub const fn status(&self) -> Status {
        self.status
    }
    pub fn raw(&self) -> Option<&[u8]> {
        self.raw.as_deref()
    }
    pub fn source(&self) -> Option<&SourceFacts> {
        self.source.as_ref()
    }
    pub fn wmo_metadata(&self) -> Option<&WmoMetadata> {
        self.wmo.as_ref()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub build: u32,
    pub reads: usize,
    pub dependency_requests: usize,
    pub source_bytes: usize,
    pub raw_bytes: usize,
    pub models: usize,
}
pub struct Extraction {
    models: Vec<Model>,
    requests: Vec<ModelHandle>,
    facts: Facts,
    resident_bytes: usize,
    store_resident_bytes: usize,
}
#[derive(Clone, Copy)]
pub(crate) struct ExtractionView<'a> {
    models: &'a [Model],
    requests: &'a [ModelHandle],
}
impl<'a> ExtractionView<'a> {
    fn model(self, handle: ModelHandle) -> Option<&'a Model> {
        self.models.get(handle.0)
    }
}
impl Extraction {
    fn view(&self) -> ExtractionView<'_> {
        ExtractionView {
            models: &self.models,
            requests: &self.requests,
        }
    }
    pub fn model(&self, handle: ModelHandle) -> Option<&Model> {
        self.models.get(handle.0)
    }
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    /// Successful model requests in call order. Texture calls have no handle
    /// and are absent; the live collector counts them in `request_calls()`.
    pub fn requests(&self) -> &[ModelHandle] {
        &self.requests
    }
    pub const fn facts(&self) -> Facts {
        self.facts
    }
    /// Final owned Vec capacities after all external/source transfers. This
    /// includes models, metadata, RAW and request handles, not LocalStore,
    /// caller-owned startup/world data, stack or allocator overhead.
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    /// Snapshot of the separately owned LocalStore's resident capacities at
    /// finish. Charge it while that store remains alive; it is not owned here.
    pub const fn store_resident_bytes(&self) -> usize {
        self.store_resident_bytes
    }
}

pub struct Extractor<'s, 'l, L: LiquidLookup + ?Sized> {
    store: &'s mut LocalStore,
    lookup: &'l L,
    limits: Limits,
    models: Vec<Model>,
    requests: Vec<ModelHandle>,
    live: usize,
    external: usize,
    external_sources: usize,
    request_calls: usize,
    shared_started: bool,
    reads: usize,
    dependency_requests: usize,
    source_bytes: usize,
    raw_bytes: usize,
    failed: Option<Error>,
    #[cfg(target_os = "linux")]
    payloads: Option<payload::Repository>,
    // Compatible cache lives in its separate owner but participates in this
    // exact descriptor partition and external residency ledger.
    #[cfg(target_os = "linux")]
    compatible_descriptors: usize,
}
impl<'s, 'l, L: LiquidLookup + ?Sized> Extractor<'s, 'l, L> {
    pub fn new(store: &'s mut LocalStore, lookup: &'l L, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        Self::new_validated(store, lookup, limits)
    }
    fn new_validated(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
    ) -> Result<Self, Error> {
        if store.stats().resident_bytes > limits.max_memory_bytes {
            return Err(Error::Limit);
        }
        store.verify_unchanged().map_err(Error::Store)?;
        Ok(Self {
            store,
            lookup,
            limits,
            models: Vec::new(),
            requests: Vec::new(),
            live: 0,
            external: 0,
            external_sources: 0,
            request_calls: 0,
            shared_started: false,
            reads: 0,
            dependency_requests: 0,
            source_bytes: 0,
            raw_bytes: 0,
            failed: None,
            #[cfg(target_os = "linux")]
            payloads: None,
            #[cfg(target_os = "linux")]
            compatible_descriptors: 0,
        })
    }
    /// Acquire and convert now, retaining everything internally. After any
    /// error, subsequent calls return that same error without further reads.
    pub fn extract(&mut self, request: ModelRequest<'_>) -> Result<ModelHandle, Error> {
        let result = (|| {
            self.begin_request()?;
            let (class, input) = match request {
                ModelRequest::M2(r) => (Class::M2, r),
                ModelRequest::Wmo(r) => (Class::Wmo, r),
            };
            let handle = self.obtain(class, input)?;
            self.record_request(handle)?;
            Ok(handle)
        })();
        self.remember(result)
    }
    /// Classify and convert the same complete, verified FDID source. REVM is
    /// WMO, MD20/MD21 are M2, and BLP2 is an explicit texture omission. Unknown
    /// or short sources poison the collector. Cache hits classify the stored
    /// source magic and enforce the same immutable origin and model class.
    /// Textures have no model/cache entry and repeated requests may reread them.
    pub fn extract_gameobject_fdid(&mut self, id: u32) -> Result<GameObjectOutcome, Error> {
        let result = (|| {
            self.begin_request()?;
            let outcome = self.obtain_gameobject(id)?;
            if let GameObjectOutcome::Model(handle) = outcome {
                self.record_request(handle)?;
            }
            Ok(outcome)
        })();
        self.remember(result)
    }
    /// Top-level calls charged to `max_requests`, including texture omissions.
    /// Dependency requests have their own counter and do not consume this cap.
    pub const fn request_calls(&self) -> usize {
        self.request_calls
    }
    fn begin_request(&mut self) -> Result<(), Error> {
        self.ready()?;
        if self.request_calls >= self.limits.max_requests {
            return Err(Error::Limit);
        }
        self.request_calls += 1;
        Ok(())
    }
    fn record_request(&mut self, handle: ModelHandle) -> Result<(), Error> {
        let cap = self.vector_reservation_cap(&self.requests, 1, true)?;
        reserve(&mut self.requests, 1, &mut self.live, cap)?;
        self.requests.push(handle);
        #[cfg(target_os = "linux")]
        if self.payloads.is_some() {
            let compact = self.compact_descriptors()?;
            self.payloads
                .as_mut()
                .ok_or(Error::State)?
                .admit_descriptors(compact)
                .map_err(|_| Error::Aborted)?;
        }
        Ok(())
    }
    fn ready(&self) -> Result<(), Error> {
        self.failed.map_or(Ok(()), Err)
    }
    fn remember<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        if let Err(error) = result {
            self.failed = Some(error);
        }
        result
    }
    /// Change only the lookup borrowed for future uncached conversions. The
    /// store borrow, existing raw bytes, handles, budgets and reservations move
    /// unchanged; previously converted WMO liquids are never recomputed.
    pub fn with_lookup<'next, N: LiquidLookup + ?Sized>(
        mut self,
        lookup: &'next N,
    ) -> Result<Extractor<'s, 'next, N>, Error> {
        self.verify()?;
        Ok(Extractor {
            store: self.store,
            lookup,
            limits: self.limits,
            models: self.models,
            requests: self.requests,
            live: self.live,
            external: self.external,
            external_sources: self.external_sources,
            request_calls: self.request_calls,
            shared_started: self.shared_started,
            reads: self.reads,
            dependency_requests: self.dependency_requests,
            source_bytes: self.source_bytes,
            raw_bytes: self.raw_bytes,
            failed: self.failed,
            #[cfg(target_os = "linux")]
            payloads: self.payloads,
            #[cfg(target_os = "linux")]
            compatible_descriptors: self.compatible_descriptors,
        })
    }
    /// Recheck the store snapshot and transfer all retained results together.
    /// A failed collector never exposes earlier successful conversions. Startup
    /// callers must transfer/release their separately owned reservations first.
    pub fn finish(mut self) -> Result<Extraction, Error> {
        self.verify()?;
        if self.external != 0 || self.external_sources != 0 {
            return Err(Error::State);
        }
        let facts = self.facts();
        Ok(Extraction {
            models: self.models,
            requests: self.requests,
            facts,
            resident_bytes: self.live,
            store_resident_bytes: self.store.stats().resident_bytes,
        })
    }
    /// Abort an enclosing operation without replacing an earlier read failure.
    pub(crate) fn abort(&mut self) -> Error {
        *self.failed.get_or_insert(Error::Aborted)
    }
    pub(crate) fn begin_shared_run(&mut self) -> Result<(), Error> {
        let result = (|| {
            self.verify()?;
            if self.shared_started {
                return Err(Error::State);
            }
            self.shared_started = true;
            Ok(())
        })();
        self.remember(result)
    }
    pub(crate) fn request_len(&self) -> usize {
        self.requests.len()
    }
    pub(crate) fn models(&self) -> &[Model] {
        &self.models
    }
    pub(crate) fn request_view(
        &self,
        start: usize,
        end: usize,
    ) -> Result<ExtractionView<'_>, Error> {
        self.ready()?;
        let requests = self.requests.get(start..end).ok_or(Error::State)?;
        Ok(ExtractionView {
            models: &self.models,
            requests,
        })
    }
    pub(crate) fn model(&self, handle: ModelHandle) -> Option<&Model> {
        self.models.get(handle.0)
    }
    pub(crate) fn facts(&self) -> Facts {
        Facts {
            build: self.store.build().build,
            reads: self.reads,
            dependency_requests: self.dependency_requests,
            source_bytes: self.source_bytes,
            raw_bytes: self.raw_bytes,
            models: self.models.len(),
        }
    }
    pub(crate) fn verify(&mut self) -> Result<(), Error> {
        let result = self
            .ready()
            .and_then(|()| self.store.verify_unchanged().map_err(Error::Store));
        self.remember(result)
    }
    /// Partition the existing collector budget before an enclosing phase starts.
    /// Only a downward change is legal. Existing store/owned/external capacities
    /// must fit before the new ceiling is installed; failure is terminal.
    pub(crate) fn constrain_memory(&mut self, max_memory_bytes: usize) -> Result<(), Error> {
        let result = (|| {
            self.verify()?;
            if max_memory_bytes > self.limits.max_memory_bytes {
                return Err(Error::InvalidLimits);
            }
            if add(self.live, self.store.stats().resident_bytes)? > max_memory_bytes {
                return Err(Error::Limit);
            }
            self.limits.max_memory_bytes = max_memory_bytes;
            Ok(())
        })();
        self.remember(result)
    }
    pub(crate) const fn memory_limit(&self) -> usize {
        self.limits.max_memory_bytes
    }
    /// Collector vector capacities plus explicit startup reservations; excludes
    /// the store, which is additionally charged by `free_bytes` and every read.
    pub(crate) const fn retained_bytes(&self) -> usize {
        self.live
    }
    pub(crate) fn free_bytes(&self) -> Result<usize, Error> {
        self.ready()?;
        self.free()
    }
    /// Charge caller-owned startup buffers, preserving the charge on transition.
    pub(crate) fn reserve_external(&mut self, bytes: usize) -> Result<(), Error> {
        let result = (|| {
            self.ready()?;
            let external = add(self.external, bytes)?;
            self.charge(bytes)?;
            self.external = external;
            Ok(())
        })();
        self.remember(result)
    }
    pub(crate) fn release_external(&mut self, bytes: usize) -> Result<(), Error> {
        let result = (|| {
            self.ready()?;
            self.external = self.external.checked_sub(bytes).ok_or(Error::State)?;
            self.release(bytes);
            Ok(())
        })();
        self.remember(result)
    }
    /// Complete startup input read through the same budgeted store and guards.
    /// Its capacity stays charged until `release_source`, independently of
    /// external reservations. `cap` can only reduce the normal per-read limit.
    pub(crate) fn read_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), Error> {
        self.read_source_ref(InputRef::FileDataId(id), cap)
    }
    /// Complete source acquisition without model-name normalization. Named
    /// WDT/ADT inputs use only LocalStore's explicit enabled name route.
    pub(crate) fn read_source_ref(
        &mut self,
        input: InputRef<'_>,
        cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), Error> {
        let result = (|| {
            self.ready()?;
            if let InputRef::FileDataId(id) = input {
                names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
            }
            let read = self.read_capped(input, cap)?;
            self.external_sources = add(self.external_sources, read.0.capacity())?;
            Ok(read)
        })();
        self.remember(result)
    }
    /// Explicit disabled-key70374 source read. The declared address space
    /// counts toward the cumulative source ceiling, including encrypted holes;
    /// actual source residency stays charged until release_source. No complete
    /// SourceFacts or VerifiedFile is created for this capability.
    #[cfg(target_os = "linux")]
    pub(crate) fn read_available_source(
        &mut self,
        id: u32,
        requested_cap: usize,
    ) -> Result<local_store::availability::AvailableSource, Error> {
        self.read_available_source_target(id, requested_cap, target::Target::Build70374)
    }
    #[cfg(target_os = "linux")]
    fn read_available_source_target(
        &mut self,
        id: u32,
        requested_cap: usize,
        target: target::Target,
    ) -> Result<local_store::availability::AvailableSource, Error> {
        let result = (|| {
            self.ready()?;
            names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
            if self.reads >= self.limits.max_reads {
                return Err(Error::Limit);
            }
            let cap = requested_cap.min(self.limits.max_file_bytes).min(
                self.limits
                    .max_source_bytes
                    .checked_sub(self.source_bytes)
                    .ok_or(Error::Limit)?,
            );
            if cap == 0 {
                return Err(Error::Limit);
            }
            let memory_limit = add(self.store.stats().resident_bytes, self.available_total()?)?;
            self.reads += 1;
            let file = match target {
                target::Target::Build70374 => {
                    self.store.read_file_data_id_available_with_external_budget(
                        id,
                        cap as u64,
                        self.live,
                        memory_limit,
                    )
                }
                target::Target::Build70425 => self
                    .store
                    .read_file_data_id_available_70425_with_external_budget(
                        id,
                        cap as u64,
                        self.live,
                        memory_limit,
                    ),
            }
            .map_err(Error::from_available)?;
            let declared = usize::try_from(file.declared_bytes()).map_err(|_| Error::Limit)?;
            if declared > cap {
                return Err(Error::Limit);
            }
            let resident = file.resident_bytes().map_err(Error::from_available)?;
            self.charge(resident)?;
            self.external_sources = add(self.external_sources, resident)?;
            self.source_bytes = add(self.source_bytes, declared)?;
            Ok(file)
        })();
        self.remember(result)
    }
    pub(crate) fn release_source(&mut self, capacity: usize) -> Result<(), Error> {
        let result = (|| {
            self.ready()?;
            self.external_sources = self
                .external_sources
                .checked_sub(capacity)
                .ok_or(Error::State)?;
            self.release(capacity);
            Ok(())
        })();
        self.remember(result)
    }
    fn vector_reservation_cap<T>(
        &self,
        values: &Vec<T>,
        additional: usize,
        compact_vector: bool,
    ) -> Result<usize, Error> {
        let cap = self.available_total()?;
        #[cfg(target_os = "linux")]
        if let Some(repository) = &self.payloads
            && add(values.len(), additional)? > values.capacity()
        {
            let old = capacity(values)?;
            let mut cap = if compact_vector {
                self.limits
                    .max_memory_bytes
                    .checked_sub(self.store.stats().resident_bytes)
                    .and_then(|n| n.checked_sub(repository.resident_bytes()))
                    .ok_or(Error::Limit)?
            } else {
                cap
            };
            cap = cap.checked_sub(old).ok_or(Error::Limit)?;
            if compact_vector {
                let compact = self.compact_descriptors()?;
                let remaining = repository
                    .descriptor_limit()
                    .checked_sub(repository.resident_bytes())
                    .and_then(|n| n.checked_sub(compact))
                    .ok_or(Error::Limit)?;
                cap = cap.min(
                    add(self.live, remaining)?
                        .checked_sub(old)
                        .ok_or(Error::Limit)?,
                );
            }
            return Ok(cap);
        }
        #[cfg(not(target_os = "linux"))]
        let _ = (values, additional, compact_vector);
        Ok(cap)
    }
    #[cfg(target_os = "linux")]
    fn compact_descriptors(&self) -> Result<usize, Error> {
        add(
            payload::compact_bytes(&self.models, &self.requests)?,
            self.compatible_descriptors,
        )
    }
    fn available_total(&self) -> Result<usize, Error> {
        let total = self
            .limits
            .max_memory_bytes
            .checked_sub(self.store.stats().resident_bytes)
            .ok_or(Error::Limit)?;
        #[cfg(target_os = "linux")]
        if let Some(repository) = &self.payloads {
            let compact = self.compact_descriptors()?;
            repository
                .check_descriptors(compact)
                .map_err(|_| Error::Aborted)?;
            return Ok(total
                .checked_sub(repository.resident_bytes())
                .ok_or(Error::Limit)?
                .min(add(compact, repository.active_limit())?));
        }
        Ok(total)
    }
    fn free(&self) -> Result<usize, Error> {
        self.available_total()?
            .checked_sub(self.live)
            .ok_or(Error::Limit)
    }
    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.live = add(self.live, bytes)?;
        if self.live > self.available_total()? {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn release(&mut self, bytes: usize) {
        self.live -= bytes;
    }
    fn obtain(&mut self, class: Class, input: InputRef<'_>) -> Result<ModelHandle, Error> {
        let prepared = names::Prepared::new(input, class)?;
        self.store.verify_unchanged().map_err(Error::Store)?;
        if let Some((index, model)) = self
            .models
            .iter()
            .enumerate()
            .find(|(_, m)| m.name == prepared.name())
        {
            let same = match (&model.origin, prepared.id) {
                (Origin::Id(a), Some(b)) => *a == b,
                (Origin::Name(a), None) => a == prepared.canonical(),
                _ => false,
            };
            if model.pending {
                return Err(Error::Cycle);
            }
            if model.class != class || !same {
                return Err(Error::Alias);
            }
            return Ok(ModelHandle(index));
        }
        let handle = self.insert_model(class, &prepared)?;
        let route = prepared.id.map_or(
            InputRef::LogicalName(prepared.source()),
            InputRef::FileDataId,
        );
        let (bytes, facts) = self.read(route)?;
        self.complete_model(handle, bytes, facts)?;
        Ok(handle)
    }
    fn insert_model(
        &mut self,
        class: Class,
        prepared: &names::Prepared,
    ) -> Result<ModelHandle, Error> {
        if self.models.len() >= self.limits.max_models {
            return Err(Error::Limit);
        }
        let cap = self.available_total()?;
        #[cfg(target_os = "linux")]
        let cap = if let Some(repository) = &self.payloads {
            let compact = self.compact_descriptors()?;
            let remaining = repository
                .descriptor_limit()
                .checked_sub(repository.resident_bytes())
                .and_then(|n| n.checked_sub(compact))
                .ok_or(Error::Limit)?;
            let global = self
                .limits
                .max_memory_bytes
                .checked_sub(self.store.stats().resident_bytes)
                .and_then(|n| n.checked_sub(repository.resident_bytes()))
                .ok_or(Error::Limit)?;
            global.min(add(self.live, remaining)?)
        } else {
            cap
        };
        let name = copy(prepared.name(), &mut self.live, cap)?;
        let origin = match prepared.id {
            Some(id) => Origin::Id(id),
            None => Origin::Name(copy(prepared.canonical(), &mut self.live, cap)?),
        };
        #[cfg(target_os = "linux")]
        let cap = if self.payloads.is_some() && self.models.len() == self.models.capacity() {
            // Admit full new Model allocation while old allocation and the
            // just-created name/origin vectors are still resident.
            cap.checked_sub(capacity(&self.models)?)
                .ok_or(Error::Limit)?
        } else {
            cap
        };
        reserve(&mut self.models, 1, &mut self.live, cap)?;
        let handle = ModelHandle(self.models.len());
        self.models.push(Model {
            class,
            name,
            origin,
            pending: true,
            status: Status::NoCollision,
            raw: None,
            source: None,
            wmo: None,
        });
        #[cfg(target_os = "linux")]
        if self.payloads.is_some() {
            let compact = self.compact_descriptors()?;
            let repository_bytes = self.payloads.as_ref().ok_or(Error::State)?.resident_bytes();
            let budget = self
                .limits
                .max_memory_bytes
                .checked_sub(self.store.stats().resident_bytes)
                .and_then(|n| n.checked_sub(self.live))
                .and_then(|n| n.checked_sub(repository_bytes))
                .ok_or(Error::Limit)?;
            self.payloads
                .as_mut()
                .ok_or(Error::State)?
                .prepare(self.models.len(), compact, budget)
                .map_err(|_| Error::Aborted)?;
            self.available_total()?;
        }
        Ok(handle)
    }
    fn complete_model(
        &mut self,
        handle: ModelHandle,
        bytes: Vec<u8>,
        facts: SourceFacts,
    ) -> Result<(), Error> {
        self.models[handle.0].source = Some(facts);
        match self.models[handle.0].class {
            Class::M2 => self.convert_m2(handle, bytes)?,
            Class::Wmo => self.convert_wmo(handle, bytes)?,
        }
        self.models[handle.0].pending = false;
        #[cfg(target_os = "linux")]
        if self.payloads.is_some() {
            let compact = self.compact_descriptors()?;
            let repository = self.payloads.as_mut().ok_or(Error::State)?;
            // Persist each completed child/model at its conversion boundary.
            // No successful RAW/root/selection accumulates behind the wrapper.
            let released = repository
                .store(handle, &mut self.models[handle.0], compact, self.live)
                .map_err(|_| Error::Aborted)?;
            self.release(released);
            self.available_total()?;
        }
        Ok(())
    }
    fn obtain_gameobject(&mut self, id: u32) -> Result<GameObjectOutcome, Error> {
        // FDID names are class-independent; this also applies the existing
        // zero/MAX policy without inventing a logical-name fallback.
        let prepared = names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
        self.store.verify_unchanged().map_err(Error::Store)?;
        if let Some((index, model)) = self
            .models
            .iter()
            .enumerate()
            .find(|(_, m)| m.name == prepared.name())
        {
            if model.pending {
                return Err(Error::Cycle);
            }
            if !matches!(model.origin, Origin::Id(original) if original == id) {
                return Err(Error::Alias);
            }
            let kind = classify(model.source.as_ref().ok_or(Error::State)?.first_magic)?;
            return match kind {
                None => Ok(GameObjectOutcome::Texture),
                Some(class) if class == model.class => {
                    Ok(GameObjectOutcome::Model(ModelHandle(index)))
                }
                Some(_) => Err(Error::Alias),
            };
        }
        #[cfg(target_os = "linux")]
        let provisional = if self.payloads.is_some() {
            Some(self.insert_model(Class::M2, &prepared)?)
        } else {
            None
        };
        #[cfg(not(target_os = "linux"))]
        let provisional: Option<ModelHandle> = None;
        let (bytes, facts) = self.read(InputRef::FileDataId(id))?;
        let Some(class) = classify(facts.first_magic)? else {
            self.release(bytes.capacity());
            if let Some(handle) = provisional {
                if handle.index() + 1 != self.models.len() {
                    return Err(Error::State);
                }
                let model = self.models.pop().ok_or(Error::State)?;
                self.release(model.name.capacity());
                if let Origin::Name(name) = model.origin {
                    self.release(name.capacity());
                }
                // Slot capacities remain reserved. Read/request/source work is
                // never refunded; a texture does not create a model handle.
            }
            return Ok(GameObjectOutcome::Texture);
        };
        let handle = if let Some(handle) = provisional {
            self.models[handle.index()].class = class;
            handle
        } else {
            self.insert_model(class, &prepared)?
        };
        self.complete_model(handle, bytes, facts)?;
        Ok(GameObjectOutcome::Model(handle))
    }
    fn read(&mut self, input: InputRef<'_>) -> Result<(Vec<u8>, SourceFacts), Error> {
        self.read_capped(input, self.limits.max_file_bytes)
    }
    fn read_capped(
        &mut self,
        input: InputRef<'_>,
        requested_cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), Error> {
        self.read_capped_policy(input, requested_cap, false)?
            .ok_or(Error::Store(local_store::Error::MissingFile))
    }
    // Only compatible's already-normalized model requests select allow_missing.
    // Required sources keep the strict adapter and its sticky error handling.
    fn read_capped_policy(
        &mut self,
        input: InputRef<'_>,
        requested_cap: usize,
        allow_missing: bool,
    ) -> Result<Option<(Vec<u8>, SourceFacts)>, Error> {
        if self.reads >= self.limits.max_reads {
            return Err(Error::Limit);
        }
        let cap = requested_cap.min(self.limits.max_file_bytes).min(
            self.limits
                .max_source_bytes
                .checked_sub(self.source_bytes)
                .ok_or(Error::Limit)?,
        );
        if cap == 0 {
            return Err(Error::Limit);
        }
        self.reads += 1;
        // Identical to the resident ceiling on the original route; the explicit
        // spool route additionally partitions current scratch from descriptors.
        let memory_limit = add(self.store.stats().resident_bytes, self.available_total()?)?;
        let file = match input {
            InputRef::FileDataId(id) => self.store.read_file_data_id_with_external_budget(
                id,
                cap as u64,
                self.live,
                memory_limit,
            ),
            InputRef::LogicalName(name) => {
                self.store
                    .read_name_with_external_budget(name, cap as u64, self.live, memory_limit)
            }
        };
        let file = match file {
            Err(local_store::Error::MissingFile) if allow_missing => {
                self.store.verify_unchanged().map_err(Error::Store)?;
                return Ok(None);
            }
            result => result.map_err(Error::Store)?,
        };
        if file.bytes.len() > cap {
            return Err(Error::Limit);
        }
        self.charge(file.bytes.capacity())?;
        self.source_bytes = add(self.source_bytes, file.bytes.len())?;
        let facts = SourceFacts {
            id: file.file_data_id,
            key: file.content_key,
            bytes: file.bytes.len(),
            first_magic: file.bytes.get(..4).and_then(|b| b.try_into().ok()),
        };
        Ok(Some((file.bytes, facts)))
    }
    fn convert_m2(&mut self, handle: ModelHandle, bytes: Vec<u8>) -> Result<(), Error> {
        if let Some(output) = self.convert_m2_bytes(&bytes)? {
            self.models[handle.0].status = Status::Collision;
            self.models[handle.0].raw = Some(output);
        }
        self.release(bytes.capacity());
        Ok(())
    }
    fn convert_m2_bytes(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if let Some(parsed) = M2Collision::parse(bytes, self.limits.m2).map_err(Error::M2)? {
            let cap = self.free()?.min(forever_vmap::MAX_RAW_BYTES).min(
                self.limits
                    .max_raw_bytes
                    .checked_sub(self.raw_bytes)
                    .ok_or(Error::Limit)?,
            );
            let output = parsed.write_raw(cap).map_err(Error::M2)?;
            self.charge(output.capacity())?;
            RawM2::parse(&output, forever_vmap::Limits::default()).map_err(Error::M2)?;
            self.raw_bytes = add(self.raw_bytes, output.len())?;
            Ok(Some(output))
        } else {
            Ok(None)
        }
    }
    fn convert_wmo(&mut self, handle: ModelHandle, bytes: Vec<u8>) -> Result<(), Error> {
        let root = wmo::Root::parse(&bytes, self.limits.wmo).map_err(Error::Wmo)?;
        let mut bindings = Vec::new();
        let mut valid = Vec::new();
        for request in doodads::requests(&root) {
            if self.dependency_requests >= self.limits.max_dependency_requests {
                return Err(Error::Limit);
            }
            self.dependency_requests += 1;
            let input = match request.target {
                doodads::Target::FileDataId(id) => InputRef::FileDataId(id),
                doodads::Target::Name(name) => InputRef::LogicalName(name),
            };
            let child = self.obtain(Class::M2, input)?;
            let cap = self.available_total()?;
            let reservation = cap.min(self.vector_reservation_cap(&bindings, 1, false)?);
            reserve(&mut bindings, 1, &mut self.live, reservation)?;
            bindings.push(NameBinding {
                name_index: request.name_index,
                model: child,
            });
            if self.models[child.0].status == Status::Collision {
                let reservation = cap.min(self.vector_reservation_cap(&valid, 1, false)?);
                reserve(&mut valid, 1, &mut self.live, reservation)?;
                valid.push(request.name_index);
            }
        }
        let mut group_bytes = Vec::new();
        let mut group_facts = Vec::new();
        for id in root.detailed_group_ids() {
            let (source, facts) = self.read(InputRef::FileDataId(id))?;
            let cap = self.available_total()?;
            let reservation = cap.min(self.vector_reservation_cap(&group_bytes, 1, false)?);
            reserve(&mut group_bytes, 1, &mut self.live, reservation)?;
            group_bytes.push(source);
            let reservation = cap.min(self.vector_reservation_cap(&group_facts, 1, false)?);
            reserve(&mut group_facts, 1, &mut self.live, reservation)?;
            group_facts.push(facts);
        }
        let (selection, output, _) =
            self.convert_wmo_geometry(&root, &group_bytes, &group_facts, &valid, &[])?;
        let status = match selection.outcome(output.is_some()) {
            doodads::Outcome::Ok => Status::Collision,
            doodads::Outcome::OkNoCollision => Status::DoodadOnly,
            doodads::Outcome::FailedNoCollision => return Err(Error::NoCollision),
        };
        self.models[handle.0].raw = output;
        for source in &group_bytes {
            self.release(source.capacity());
        }
        self.release(capacity(&group_bytes)?);
        drop(group_bytes);
        self.release(capacity(&valid)?);
        self.models[handle.0].status = status;
        self.models[handle.0].wmo = Some(WmoMetadata {
            root_bytes: bytes,
            bindings,
            selection,
            groups: group_facts,
            limits: self.limits.wmo,
        });
        Ok(())
    }
    // Shared parser/selection/RAW kernel. Compatible acquisition must supply
    // every required group before entering this function.
    fn convert_wmo_geometry(
        &mut self,
        root: &wmo::Root<'_>,
        group_bytes: &[Vec<u8>],
        group_facts: &[SourceFacts],
        valid: &[u32],
        omitted: &[u32],
    ) -> Result<(doodads::Selection, Option<Vec<u8>>, bool), Error> {
        let mut groups = Vec::new();
        let mut inputs = Vec::new();
        let cap = self.available_total()?;
        let reservation =
            cap.min(self.vector_reservation_cap(&groups, group_bytes.len(), false)?);
        reserve(&mut groups, group_bytes.len(), &mut self.live, reservation)?;
        for source in group_bytes {
            groups.push(wmo::Group::parse(source, self.limits.wmo).map_err(Error::Wmo)?);
        }
        let reservation = cap.min(self.vector_reservation_cap(&inputs, groups.len(), false)?);
        reserve(&mut inputs, groups.len(), &mut self.live, reservation)?;
        for (source, group) in group_facts.iter().zip(&groups) {
            inputs.push(wmo::GroupInput {
                file_data_id: source.id,
                group,
            });
        }
        let selection = doodads::resolve(
            root,
            &inputs,
            valid,
            wmo::Limits {
                max_projection_bytes: self.limits.wmo.max_projection_bytes.min(self.free()?),
                ..self.limits.wmo
            },
        )
        .map_err(Error::Wmo)?;
        self.charge(selection.resident_bytes())?;
        // Missing dependencies are never supplied as successful valid M2s to
        // resolve(). Inspect only actual non-skipped group references to tell
        // omission-derived emptiness from an intrinsically empty model. This
        // makes no claim about geometry in an unavailable dependency.
        let omitted_usable = if omitted.is_empty() || root.doodad_set_count() == 0 {
            false
        } else {
            let contexts = wmo::resolve_contexts(
                root,
                &inputs,
                wmo::Limits {
                    max_projection_bytes: self.limits.wmo.max_projection_bytes.min(self.free()?),
                    ..self.limits.wmo
                },
            )
            .map_err(Error::Wmo)?;
            let context_bytes = capacity(&contexts)?;
            self.charge(context_bytes)?;
            let referenced = contexts
                .iter()
                .filter(|c| c.skip_reason().is_none())
                .any(|context| {
                    let group = context.group();
                    (0..group.doodad_reference_count()).any(|i| {
                        group
                            .doodad_reference(i)
                            .and_then(|r| root.doodad_spawn(usize::from(r)))
                            .is_some_and(|spawn| omitted.binary_search(&spawn.name_index).is_ok())
                    })
                });
            drop(contexts);
            self.release(context_bytes);
            referenced
        };
        let converted = raw::convert(
            root,
            &inputs,
            self.lookup,
            raw::ConversionLimits {
                source: self.limits.wmo,
                max_raw_bytes: raw::MAX_RAW_BYTES.min(
                    self.limits
                        .max_raw_bytes
                        .checked_sub(self.raw_bytes)
                        .ok_or(Error::Limit)?,
                ),
                max_memory_bytes: raw::MAX_MEMORY_BYTES.min(self.free()?),
            },
        )
        .map_err(Error::Wmo)?;
        let output = converted.map(|v| v.into_bytes());
        if let Some(output) = &output {
            self.charge(output.capacity())?;
            self.raw_bytes = add(self.raw_bytes, output.len())?;
        }
        self.release(capacity(&inputs)?);
        drop(inputs);
        self.release(capacity(&groups)?);
        drop(groups);
        Ok((selection, output, omitted_usable))
    }
}

fn classify(magic: Option<[u8; 4]>) -> Result<Option<Class>, Error> {
    match magic.as_ref() {
        Some(b"REVM") => Ok(Some(Class::Wmo)),
        Some(b"MD20" | b"MD21") => Ok(Some(Class::M2)),
        Some(b"BLP2") => Ok(None),
        _ => Err(Error::GameObjectMagic),
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
    live: &mut usize,
    cap: usize,
) -> Result<(), Error> {
    let required = add(v.len(), additional)?;
    if required <= v.capacity() {
        return Ok(());
    }
    let old = capacity(v)?;
    let requested = required.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
    if add(*live, requested - old)? > cap {
        return Err(Error::Limit);
    }
    v.try_reserve_exact(additional)
        .map_err(|_| Error::Allocation)?;
    *live = add(*live, capacity(v)? - old)?;
    if *live > cap {
        return Err(Error::Limit);
    }
    Ok(())
}
fn copy(bytes: &[u8], live: &mut usize, cap: usize) -> Result<Vec<u8>, Error> {
    let mut v = Vec::new();
    reserve(&mut v, bytes.len(), live, cap)?;
    v.extend_from_slice(bytes);
    Ok(v)
}
