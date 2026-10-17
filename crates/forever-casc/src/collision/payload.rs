// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-11: original explicit per-model spooling collector.
//! The old owned Extractor remains unchanged in behavior. This route persists
//! every completed model immediately (including recursive M2 children), drops
//! its RAW and WMO root/metadata vectors and retains only compact descriptors.
//! A callback may reload one complete payload under the active/global ceilings.
//! Returned callback data is caller-owned and outside this collector's budget.
//! The separate gameobjects::spooled consumer composes bounded startup/lists.
//! No world/assembly/product publication, restart/resume or full-world
//! acceptance is provided.

use super::{
    Class, Error as CollisionError, Extractor, Facts, GameObjectOutcome, Limits as CollisionLimits,
    Model, ModelHandle, ModelRequest, NameBinding, Origin, SourceFacts, Status, add, capacity,
    spool::{self, Record, RecordWriter, Spool},
};
use crate::local_store::LocalStore;
use forever_vmap::wmo::{self, doodads, liquid::LiquidLookup};

pub const PROFILE: &str = "linux-private-collision-payload-spool-v1";
const MAGIC: &[u8; 8] = b"RCEPLD1\0";
pub const COMPATIBLE_PROFILE: &str = "linux-private-compatible-collision-payload-spool-v1";
const COMPATIBLE_MAGIC: &[u8; 8] = b"RCEPLC1\0";
/// Separately selected GNU libstdc++15 reference traversal; never v1 wire data.
pub const COMPATIBLE_GNU15_PROFILE: &str =
    "linux-private-compatible-collision-payload-spool-gnu15-native-order-v2";
const COMPATIBLE_GNU15_MAGIC: &[u8; 8] = b"RCEPLC2\0";
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Strict,
    CompatibleV1,
    CompatibleGnu15V2,
}
impl Mode {
    const fn compatible(self) -> bool {
        !matches!(self, Self::Strict)
    }
    const fn native_order(self) -> bool {
        matches!(self, Self::CompatibleGnu15V2)
    }
    const fn magic(self) -> &'static [u8; 8] {
        match self {
            Self::Strict => MAGIC,
            Self::CompatibleV1 => COMPATIBLE_MAGIC,
            Self::CompatibleGnu15V2 => COMPATIBLE_GNU15_MAGIC,
        }
    }
}
const COMPATIBLE_BINDING: usize = 16;
const HEADER: usize = 52;
const BINDING: usize = 12;
const GROUP: usize = 33;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    BackendUnavailable,
    Limit,
    Allocation,
    Path,
    Privacy,
    Mount,
    Alias,
    Exists,
    Io,
    Changed,
    Format,
    State,
    Callback,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_payload_invalid_limits",
            Self::BackendUnavailable => "collision_payload_backend_unavailable",
            Self::Limit => "collision_payload_limit",
            Self::Allocation => "collision_payload_allocation",
            Self::Path => "collision_payload_path",
            Self::Privacy => "collision_payload_privacy",
            Self::Mount => "collision_payload_mount",
            Self::Alias => "collision_payload_input_alias",
            Self::Exists => "collision_payload_exists",
            Self::Io => "collision_payload_io",
            Self::Changed => "collision_payload_changed",
            Self::Format => "collision_payload_format",
            Self::State => "collision_payload_state",
            Self::Callback => "collision_payload_callback",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// New route error only; the original collision::Error enum stays exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectorError {
    Collision(CollisionError),
    Payload(Error),
}
impl CollectorError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Collision(e) => e.code(),
            Self::Payload(e) => e.code(),
        }
    }
}
impl From<CollisionError> for CollectorError {
    fn from(e: CollisionError) -> Self {
        Self::Collision(e)
    }
}
impl From<Error> for CollectorError {
    fn from(e: Error) -> Self {
        Self::Payload(e)
    }
}
impl std::fmt::Display for CollectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for CollectorError {}
fn plus(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn times(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
fn count(b: &[u8]) -> Result<usize, Error> {
    usize::try_from(u64::from_le_bytes(b.try_into().map_err(|_| Error::Format)?))
        .map_err(|_| Error::Format)
}
fn u32_at(b: &[u8]) -> u32 {
    u32::from_le_bytes(b.try_into().expect("checked fixed slice"))
}
fn u64_at(b: &[u8]) -> u64 {
    u64::from_le_bytes(b.try_into().expect("checked fixed slice"))
}

pub(crate) fn compact_bytes(
    models: &Vec<Model>,
    requests: &Vec<ModelHandle>,
) -> Result<usize, CollisionError> {
    models
        .iter()
        .try_fold(add(capacity(models)?, capacity(requests)?)?, |n, m| {
            let n = add(n, m.name.capacity())?;
            match &m.origin {
                Origin::Id(_) => Ok(n),
                Origin::Name(name) => add(n, name.capacity()),
            }
        })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadShape {
    pub class: Class,
    pub raw_bytes: Option<usize>,
    /// Entire record: RAW plus header and any WMO root/metadata.
    pub record_bytes: usize,
}
struct Mapping {
    model: ModelHandle,
    record: Record,
    shape: PayloadShape,
}
/// Preserves an operation error even when final guards also fail.
pub(crate) enum CallbackFailure<E> {
    Collector(CollectorError),
    Callback {
        cause: E,
        guard: Option<CollectorError>,
    },
}
pub(crate) struct Repository {
    spool: Spool,
    mapping: Vec<Mapping>,
    wmo: wmo::Limits,
    mode: Mode,
    observed_active: usize,
    failed: Option<Error>,
}
impl Repository {
    pub(super) fn identity_limit(&self) -> usize {
        self.spool.limits().max_records
    }
    pub(super) fn enable_compatible(&mut self) -> Result<(), Error> {
        if !self.mapping.is_empty() {
            return Err(Error::State);
        }
        self.mode = Mode::CompatibleV1;
        Ok(())
    }
    pub(super) fn enable_compatible_native_order(&mut self) -> Result<(), Error> {
        if !self.mapping.is_empty() {
            return Err(Error::State);
        }
        self.mode = Mode::CompatibleGnu15V2;
        Ok(())
    }

    pub(crate) fn resident_bytes(&self) -> usize {
        // Each term was checked before admission. Overflow is rejected at every
        // mutation; this accessor cannot introduce a new allocation.
        self.spool
            .resident_bytes()
            .expect("admitted spool descriptors")
            + self.mapping.capacity() * size_of::<Mapping>()
    }
    pub(crate) fn active_limit(&self) -> usize {
        self.spool.limits().max_active_bytes
    }
    pub(crate) fn descriptor_limit(&self) -> usize {
        self.spool.limits().max_descriptor_bytes
    }
    fn remember<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        if let Err(e) = result {
            self.failed.get_or_insert(e);
        }
        result
    }
    pub(crate) fn admit_descriptors(&mut self, compact: usize) -> Result<(), Error> {
        let result = self.check_descriptors(compact);
        self.remember(result)
    }
    fn failure(&self) -> Option<Error> {
        self.failed.or(self.spool.failure())
    }

    pub(crate) fn check_descriptors(&self, compact: usize) -> Result<(), Error> {
        if plus(self.resident_bytes(), compact)? > self.spool.limits().max_descriptor_bytes {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    pub(crate) fn prepare(
        &mut self,
        count: usize,
        compact: usize,
        budget: usize,
    ) -> Result<(), Error> {
        let result = (|| {
            if let Some(e) = self.failure() {
                return Err(e);
            }
            let before = self.resident_bytes();
            if count > self.spool.limits().max_records {
                return Err(Error::Limit);
            }
            if count > self.mapping.capacity() {
                let full = times(count, size_of::<Mapping>())?;
                if full > budget || plus(plus(before, full)?, compact)? > self.descriptor_limit() {
                    return Err(Error::Limit);
                }
                self.mapping
                    .try_reserve_exact(count - self.mapping.len())
                    .map_err(|_| Error::Allocation)?;
                let actual = times(self.mapping.capacity(), size_of::<Mapping>())?;
                if actual > budget
                    || plus(plus(before, actual)?, compact)? > self.descriptor_limit()
                {
                    return Err(Error::Limit);
                }
            }
            let spent = self
                .resident_bytes()
                .checked_sub(before)
                .ok_or(Error::State)?;
            let global = budget.checked_sub(spent).ok_or(Error::Limit)?;
            let descriptors = self
                .descriptor_limit()
                .checked_sub(plus(self.resident_bytes(), compact)?)
                .ok_or(Error::Limit)?;
            self.spool.prepare_records(count, global.min(descriptors))?;
            self.check_descriptors(compact)
        })();
        self.remember(result)
    }
    pub(crate) fn store(
        &mut self,
        handle: ModelHandle,
        model: &mut Model,
        compact: usize,
        live: usize,
    ) -> Result<usize, Error> {
        let result = self.store_inner(handle, model, compact, live, None, None);
        self.remember(result)
    }
    pub(super) fn store_compatible(
        &mut self,
        handle: ModelHandle,
        model: &mut Model,
        compact: usize,
        live: usize,
        bindings: Option<&[super::compatible::Binding]>,
        native_references: Option<&[u16]>,
    ) -> Result<usize, Error> {
        if !self.mode.compatible() {
            return Err(Error::State);
        }
        let result = self.store_inner(handle, model, compact, live, bindings, native_references);
        self.remember(result)
    }
    fn store_inner(
        &mut self,
        handle: ModelHandle,
        model: &mut Model,
        compact: usize,
        live: usize,
        compatible_bindings: Option<&[super::compatible::Binding]>,
        native_references: Option<&[u16]>,
    ) -> Result<usize, Error> {
        if self.mapping.iter().any(|m| m.model == handle) {
            return Err(Error::State);
        }
        self.observed_active = self
            .observed_active
            .max(live.checked_sub(compact).ok_or(Error::State)?);
        let (raw, root, bindings, refs, groups) = match &model.wmo {
            Some(w) => (
                model.raw.as_deref().unwrap_or(&[]),
                w.root_bytes.as_slice(),
                w.bindings.as_slice(),
                native_references.unwrap_or_else(|| w.selection.references()),
                w.groups.as_slice(),
            ),
            None => (
                model.raw.as_deref().unwrap_or(&[]),
                &[][..],
                &[][..],
                &[][..],
                &[][..],
            ),
        };
        if native_references.is_some() != (self.mode.native_order() && model.wmo.is_some()) {
            return Err(Error::State);
        }
        if model.wmo.is_some() && self.mode.compatible() != compatible_bindings.is_some() {
            return Err(Error::State);
        }
        let binding_count = compatible_bindings.map_or(bindings.len(), <[_]>::len);
        let binding_width = if self.mode.compatible() {
            COMPATIBLE_BINDING
        } else {
            BINDING
        };
        let size = [
            raw.len(),
            root.len(),
            times(binding_count, binding_width)?,
            times(refs.len(), 2)?,
            times(groups.len(), GROUP)?,
        ]
        .into_iter()
        .try_fold(HEADER, plus)?;
        if self.mapping.len() >= self.mapping.capacity() {
            return Err(Error::State);
        }
        self.check_descriptors(compact)?;
        let mut header = [0; HEADER];
        header[..8].copy_from_slice(self.mode.magic());
        let flags = u32::from(model.raw.is_some()) | (u32::from(model.wmo.is_some()) << 1);
        header[8..12].copy_from_slice(&flags.to_le_bytes());
        for (part, n) in header[12..].as_chunks_mut::<8>().0.iter_mut().zip([
            raw.len(),
            root.len(),
            binding_count,
            refs.len(),
            groups.len(),
        ]) {
            part.copy_from_slice(&(n as u64).to_le_bytes());
        }
        let record = self.spool.write(size, |out| {
            out.bytes(&header)?;
            out.bytes(raw)?;
            out.bytes(root)?;
            if let Some(bindings) = compatible_bindings {
                for b in bindings {
                    out.bytes(&b.name_index.to_le_bytes())?;
                    let (tag, handle) = match b.disposition {
                        super::compatible::Disposition::Present(h) => (0u32, h.index() as u64),
                        super::compatible::Disposition::Missing => (1u32, 0),
                    };
                    out.bytes(&tag.to_le_bytes())?;
                    out.bytes(&handle.to_le_bytes())?;
                }
            } else {
                for b in bindings {
                    out.bytes(&b.name_index.to_le_bytes())?;
                    out.bytes(&(b.model.index() as u64).to_le_bytes())?;
                }
            }
            for r in refs {
                out.bytes(&r.to_le_bytes())?;
            }
            for g in groups {
                write_source(out, g)?;
            }
            Ok(())
        })?;
        self.mapping.push(Mapping {
            model: handle,
            record,
            shape: PayloadShape {
                class: model.class,
                raw_bytes: model.raw.as_ref().map(Vec::len),
                record_bytes: size,
            },
        });
        self.check_descriptors(compact)?;
        let mut released = 0;
        if let Some(raw) = model.raw.take() {
            released = plus(released, raw.capacity())?;
        }
        if let Some(w) = model.wmo.take() {
            released = plus(released, w.root_bytes.capacity())?;
            released = plus(
                released,
                times(w.bindings.capacity(), size_of::<NameBinding>())?,
            )?;
            released = plus(released, w.selection.resident_bytes())?;
            released = plus(
                released,
                times(w.groups.capacity(), size_of::<SourceFacts>())?,
            )?;
        }
        Ok(released)
    }
    fn with_payload<T>(
        &mut self,
        handle: ModelHandle,
        budget: usize,
        call: impl FnOnce(Payload<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if self.mode.compatible() {
            return Err(Error::Format);
        }
        self.with_record_payload(handle, budget, false, call)
    }
    fn with_record_payload<T>(
        &mut self,
        handle: ModelHandle,
        budget: usize,
        compatible: bool,
        call: impl FnOnce(Payload<'_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let result = (|| {
            if let Some(e) = self.failure() {
                return Err(e);
            }
            let mapping = self
                .mapping
                .iter()
                .find(|m| m.model == handle)
                .ok_or(Error::State)?;
            let (record, shape) = (mapping.record, mapping.shape);
            let limits = self.wmo;
            let actual_mode = self.mode;
            self.spool.with_record(record, budget, |bytes| {
                if actual_mode.compatible() != compatible {
                    return Err(Error::Format);
                }
                let payload = if compatible {
                    Payload::parse_mode(bytes, limits, actual_mode)?
                } else {
                    Payload::parse(bytes, limits)?
                };
                if bytes.len() != shape.record_bytes
                    || payload.raw().map(<[u8]>::len) != shape.raw_bytes
                    || payload.wmo().is_some() != (shape.class == Class::Wmo)
                {
                    return Err(Error::Format);
                }
                call(payload)
            })
        })();
        self.remember(result)
    }
}
fn write_source(out: &mut RecordWriter<'_>, g: &SourceFacts) -> Result<(), Error> {
    out.bytes(&g.id.to_le_bytes())?;
    out.bytes(&g.key)?;
    out.bytes(&(g.bytes as u64).to_le_bytes())?;
    out.bytes(&[u8::from(g.first_magic.is_some())])?;
    out.bytes(&g.first_magic.unwrap_or([0; 4]))
}
// Exactly the u16 domain, stack-only; it is not a new logical heap allowance.
// v1 continues to require strict ascending references in its original branch.
fn validate_native_references(bytes: &[u8], spawn_count: usize) -> Result<(), Error> {
    let (references, remainder) = bytes.as_chunks::<2>();
    if !remainder.is_empty() || references.len() > 65536 {
        return Err(Error::Format);
    }
    let mut seen = [0u8; 8192];
    for b in references {
        let reference = usize::from(u16::from_le_bytes(*b));
        let mask = 1u8 << (reference % 8);
        if reference >= spawn_count || seen[reference / 8] & mask != 0 {
            return Err(Error::Format);
        }
        seen[reference / 8] |= mask;
    }
    Ok(())
}
/// A borrowed payload, never Debug. Its lifetime is confined to one guarded
/// callback. Root/source/key facts remain explicit accessors, never diagnostics.
pub struct Payload<'a> {
    mode: Mode,
    raw: Option<&'a [u8]>,
    wmo: Option<Wmo<'a>>,
}
impl<'a> Payload<'a> {
    fn parse(bytes: &'a [u8], limits: wmo::Limits) -> Result<Self, Error> {
        Self::parse_profile(bytes, limits, false)
    }
    fn parse_profile(
        bytes: &'a [u8],
        limits: wmo::Limits,
        compatible: bool,
    ) -> Result<Self, Error> {
        Self::parse_mode(
            bytes,
            limits,
            if compatible {
                Mode::CompatibleV1
            } else {
                Mode::Strict
            },
        )
    }
    fn parse_mode(bytes: &'a [u8], limits: wmo::Limits, mode: Mode) -> Result<Self, Error> {
        let compatible = mode.compatible();
        let magic = mode.magic();
        let binding_width = if compatible {
            COMPATIBLE_BINDING
        } else {
            BINDING
        };
        if bytes.len() < HEADER || &bytes[..8] != magic {
            return Err(Error::Format);
        }
        let flags = u32_at(&bytes[8..12]);
        if flags > 3 {
            return Err(Error::Format);
        }
        let mut n = [0usize; 5];
        for (out, b) in n
            .iter_mut()
            .zip(bytes[12..HEADER].as_chunks::<8>().0.iter())
        {
            *out = count(b)?;
        }
        let [raw_len, root_len, bindings_len, refs_len, groups_len] = n;
        let lengths = [
            raw_len,
            root_len,
            times(bindings_len, binding_width)?,
            times(refs_len, 2)?,
            times(groups_len, GROUP)?,
        ];
        let end = lengths.into_iter().try_fold(HEADER, plus)?;
        if end != bytes.len() {
            return Err(Error::Format);
        }
        if flags & 1 == 0 && raw_len != 0 || flags & 2 == 0 && n[1..].iter().any(|n| *n != 0) {
            return Err(Error::Format);
        }
        let mut at = HEADER;
        let mut parts = [&[][..]; 5];
        for (out, len) in parts.iter_mut().zip(lengths) {
            *out = &bytes[at..at + len];
            at += len;
        }
        let wmo = if flags & 2 != 0 {
            let root = wmo::Root::parse(parts[1], limits).map_err(|_| Error::Format)?;
            if bindings_len > limits.max_metadata_records
                || refs_len > 65536
                || groups_len > limits.max_groups
            {
                return Err(Error::Format);
            }
            for i in 0..root.doodad_set_count() {
                let set = root.doodad_set(i).ok_or(Error::Format)?;
                set.start_index
                    .checked_add(set.count)
                    .ok_or(Error::Format)?;
            }
            let mut previous = None;
            for b in parts[2].chunks_exact(binding_width) {
                if compatible {
                    let tag = u32_at(&b[4..8]);
                    let h = count(&b[8..16])?;
                    if tag > 1 || tag == 0 && h > 4095 || tag == 1 && h != 0 {
                        return Err(Error::Format);
                    }
                } else if count(&b[4..])? > 4095 {
                    return Err(Error::Format);
                }
            }
            if mode.native_order() {
                validate_native_references(parts[3], root.doodad_spawn_count())?;
            } else {
                for b in parts[3].as_chunks::<2>().0.iter() {
                    let v = u16::from_le_bytes(*b);
                    if previous.is_some_and(|p| p >= v)
                        || usize::from(v) >= root.doodad_spawn_count()
                    {
                        return Err(Error::Format);
                    }
                    previous = Some(v);
                }
            }
            for g in parts[4].as_chunks::<GROUP>().0.iter() {
                if g[28] > 1 || g[28] == 0 && g[29..] != [0; 4] {
                    return Err(Error::Format);
                }
                count(&g[20..28])?;
            }
            Some(Wmo {
                root,
                bindings: parts[2],
                references: parts[3],
                groups: parts[4],
            })
        } else {
            None
        };
        Ok(Self {
            mode,
            raw: (flags & 1 != 0).then_some(parts[0]),
            wmo,
        })
    }
    pub const fn raw(&self) -> Option<&'a [u8]> {
        self.raw
    }
    pub fn wmo(&self) -> Option<&Wmo<'a>> {
        self.wmo.as_ref()
    }
}
pub struct Wmo<'a> {
    root: wmo::Root<'a>,
    bindings: &'a [u8],
    references: &'a [u8],
    groups: &'a [u8],
}
impl<'a> Wmo<'a> {
    pub(crate) fn binding(&self, name_index: u32) -> Option<NameBinding> {
        let values = self.bindings.as_chunks::<BINDING>().0;
        let at = values
            .binary_search_by_key(&name_index, |b| u32_at(&b[..4]))
            .ok()?;
        let b = values[at];
        Some(NameBinding {
            name_index,
            model: ModelHandle(u64_at(&b[4..]) as usize),
        })
    }
    pub const fn root(&self) -> &wmo::Root<'a> {
        &self.root
    }
    pub fn bindings(&self) -> impl ExactSizeIterator<Item = NameBinding> + '_ {
        self.bindings
            .as_chunks::<BINDING>()
            .0
            .iter()
            .map(|b| NameBinding {
                name_index: u32_at(&b[..4]),
                model: ModelHandle(u64_at(&b[4..]) as usize),
            })
    }
    pub fn references(&self) -> impl ExactSizeIterator<Item = u16> + '_ {
        self.references
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn set_count(&self) -> usize {
        self.root.doodad_set_count()
    }
    pub fn set_references(
        &self,
        requested: u16,
    ) -> impl Iterator<Item = doodads::SetReference> + '_ {
        let extra = usize::from(requested);
        [
            (self.root.doodad_set_count() != 0).then_some(0),
            (extra != 0 && extra < self.root.doodad_set_count()).then_some(extra),
        ]
        .into_iter()
        .flatten()
        .flat_map(move |set_index| {
            let set = self.root.doodad_set(set_index).expect("parsed root set");
            self.references()
                .filter(move |r| {
                    u32::from(*r) >= set.start_index && u32::from(*r) < set.start_index + set.count
                })
                .map(move |reference| doodads::SetReference {
                    set_index,
                    reference,
                })
        })
    }
    pub fn group_sources(&self) -> impl ExactSizeIterator<Item = SourceFacts> + '_ {
        self.groups
            .as_chunks::<GROUP>()
            .0
            .iter()
            .map(|b| SourceFacts {
                id: u32_at(&b[..4]),
                key: b[4..20].try_into().expect("fixed group key"),
                bytes: u64_at(&b[20..28]) as usize,
                first_magic: (b[28] != 0).then(|| b[29..33].try_into().expect("fixed magic")),
            })
    }
}
/// Distinct wire/profile view; never passes compatible bytes to strict Wmo.
pub struct CompatiblePayload<'a> {
    payload: Payload<'a>,
}
impl<'a> CompatiblePayload<'a> {
    pub const fn profile(&self) -> &'static str {
        if self.payload.mode.native_order() {
            COMPATIBLE_GNU15_PROFILE
        } else {
            COMPATIBLE_PROFILE
        }
    }
    fn validate(payload: Payload<'a>, models: &[Model]) -> Result<Self, Error> {
        if let Some(w) = payload.wmo() {
            let view = CompactView {
                models,
                requests: &[],
            };
            let (binding_records, remainder) = w.bindings.as_chunks::<COMPATIBLE_BINDING>();
            if !remainder.is_empty() {
                return Err(Error::Format);
            }
            let mut bindings = binding_records.iter();
            let mut previous = None;
            for request in doodads::requests(w.root()) {
                let b = bindings.next().ok_or(Error::Format)?;
                let name_index = u32_at(&b[..4]);
                if name_index != request.name_index || previous.is_some_and(|p| p >= name_index) {
                    return Err(Error::Format);
                }
                previous = Some(name_index);
                let input = match request.target {
                    doodads::Target::FileDataId(id) => super::InputRef::FileDataId(id),
                    doodads::Target::Name(n) => super::InputRef::LogicalName(n),
                };
                super::names::Prepared::new(input, Class::M2).map_err(|_| Error::Format)?;
                if u32_at(&b[4..8]) == 0 {
                    let handle = ModelHandle(count(&b[8..16])?);
                    view.check_identity(handle, Class::M2, input)
                        .map_err(|_| Error::Format)?;
                }
            }
            if bindings.next().is_some() {
                return Err(Error::Format);
            }
            let mut groups = w.group_sources();
            for id in w.root().detailed_group_ids() {
                super::names::Prepared::new(super::InputRef::FileDataId(id), Class::M2)
                    .map_err(|_| Error::Format)?;
                let group = groups.next().ok_or(Error::Format)?;
                if group.file_data_id() != id {
                    return Err(Error::Format);
                }
            }
            if groups.next().is_some() {
                return Err(Error::Format);
            }
            // Every selected MODD reference must resolve to a real collision M2.
            for reference in w.references() {
                let spawn = w
                    .root()
                    .doodad_spawn(usize::from(reference))
                    .ok_or(Error::Format)?;
                let values = w.bindings.as_chunks::<COMPATIBLE_BINDING>().0;
                let at = values
                    .binary_search_by_key(&spawn.name_index, |b| u32_at(&b[..4]))
                    .map_err(|_| Error::Format)?;
                let b = values[at];
                if u32_at(&b[4..8]) != 0 {
                    return Err(Error::Format);
                }
                let model = models.get(count(&b[8..16])?).ok_or(Error::Format)?;
                if model.class != Class::M2 || model.status != Status::Collision || model.pending {
                    return Err(Error::Format);
                }
            }
        }
        Ok(Self { payload })
    }
    pub const fn raw(&self) -> Option<&'a [u8]> {
        self.payload.raw
    }
    pub fn wmo(&self) -> Option<CompatibleWmo<'_, 'a>> {
        self.payload.wmo.as_ref().map(|wmo| CompatibleWmo { wmo })
    }
}
pub struct CompatibleWmo<'v, 'a> {
    wmo: &'v Wmo<'a>,
}
impl<'v, 'a> CompatibleWmo<'v, 'a> {
    pub fn root(&self) -> &wmo::Root<'a> {
        self.wmo.root()
    }
    pub fn bindings(&self) -> impl ExactSizeIterator<Item = super::compatible::Binding> + '_ {
        self.wmo
            .bindings
            .as_chunks::<COMPATIBLE_BINDING>()
            .0
            .iter()
            .map(|b| super::compatible::Binding {
                name_index: u32_at(&b[..4]),
                disposition: if u32_at(&b[4..8]) == 0 {
                    super::compatible::Disposition::Present(ModelHandle(u64_at(&b[8..16]) as usize))
                } else {
                    super::compatible::Disposition::Missing
                },
            })
    }
    pub(crate) fn binding(&self, name_index: u32) -> Option<super::compatible::Binding> {
        let values = self.wmo.bindings.as_chunks::<COMPATIBLE_BINDING>().0;
        let at = values
            .binary_search_by_key(&name_index, |b| u32_at(&b[..4]))
            .ok()?;
        let b = values[at];
        Some(super::compatible::Binding {
            name_index,
            disposition: if u32_at(&b[4..8]) == 0 {
                super::compatible::Disposition::Present(ModelHandle(u64_at(&b[8..16]) as usize))
            } else {
                super::compatible::Disposition::Missing
            },
        })
    }
    pub fn set_count(&self) -> usize {
        self.wmo.set_count()
    }
    pub fn set_references(
        &self,
        requested: u16,
    ) -> impl Iterator<Item = doodads::SetReference> + '_ {
        self.wmo.set_references(requested)
    }
    pub fn references(&self) -> impl ExactSizeIterator<Item = u16> + '_ {
        self.wmo.references()
    }
    pub fn group_sources(&self) -> impl ExactSizeIterator<Item = SourceFacts> + '_ {
        self.wmo.group_sources()
    }
}
/// Compact facts borrow the collector and never expose an unloaded raw as if it
/// were absent. To inspect presence/content, use with_payload explicitly.
pub struct Descriptor<'a> {
    model: &'a Model,
}
impl<'a> Descriptor<'a> {
    pub const fn class(&self) -> Class {
        self.model.class
    }
    pub const fn status(&self) -> Status {
        self.model.status
    }
    pub fn output_basename(&self) -> &'a [u8] {
        &self.model.name
    }
    /// Original typed route; logical names borrow the retained canonical name.
    /// This metadata accessor performs no acquisition or payload verification.
    pub fn origin(&self) -> super::InputRef<'a> {
        match &self.model.origin {
            super::Origin::Id(id) => super::InputRef::FileDataId(*id),
            super::Origin::Name(name) => super::InputRef::LogicalName(name),
        }
    }
    pub fn source(&self) -> Option<&'a SourceFacts> {
        self.model.source.as_ref()
    }
}
/// Scoped, immutable compact lookup. No RAW/root view or cloned name storage.
/// Kept crate-private so callers cannot bypass the guarded collector protocol.
pub(crate) struct CompactView<'a> {
    models: &'a [Model],
    requests: &'a [ModelHandle],
}
impl<'a> CompactView<'a> {
    pub(crate) fn model_count(&self) -> usize {
        self.models.len()
    }
    pub(crate) fn request(&self, ordinal: usize) -> Result<ModelHandle, CollisionError> {
        self.requests
            .get(ordinal)
            .copied()
            .ok_or(CollisionError::State)
    }
    pub(crate) fn descriptor(&self, handle: ModelHandle) -> Result<Descriptor<'a>, CollisionError> {
        let model = self
            .models
            .get(handle.index())
            .ok_or(CollisionError::State)?;
        if model.pending {
            return Err(CollisionError::State);
        }
        Ok(Descriptor { model })
    }
    pub(crate) fn check_identity(
        &self,
        handle: ModelHandle,
        class: Class,
        input: super::InputRef<'_>,
    ) -> Result<(), super::placements::Error> {
        let descriptor = self
            .descriptor(handle)
            .map_err(super::placements::Error::Collision)?;
        let model = descriptor.model;
        let prepared = super::names::Prepared::new(input, class)
            .map_err(super::placements::Error::Collision)?;
        let same = match (&model.origin, prepared.id) {
            (super::Origin::Id(a), Some(b)) => *a == b,
            (super::Origin::Name(a), None) => a == prepared.canonical(),
            _ => false,
        };
        if model.class != class || !same || model.output_basename() != prepared.name() {
            return Err(super::placements::Error::Identity);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub compact_bytes: usize,
    pub repository_bytes: usize,
    /// Observed retained payload capacities at persistence checkpoints, not
    /// converter scratch peak, allocator overhead, RSS or all process memory.
    pub observed_payload_bytes: usize,
    pub active_ceiling: usize,
    pub spool: spool::Stats,
}
/// Explicit new route; cannot be passed accidentally to old owned consumers.
pub(crate) enum LookupTransition<T, O> {
    Ready(T),
    Failed(O, CollectorError),
}
pub struct SpoolingExtractor<'s, 'l, L: LiquidLookup + ?Sized> {
    pub(super) inner: Extractor<'s, 'l, L>,
    failed: Option<CollectorError>,
}
impl<'s, 'l, L: LiquidLookup + ?Sized> SpoolingExtractor<'s, 'l, L> {
    pub fn new(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: CollisionLimits,
        mut spool: Spool,
    ) -> Result<Self, CollectorError> {
        spool.verify_unchanged()?;
        limits.validate_resources(spool.resources())?;
        let mut inner = Extractor::new_validated(store, lookup, limits)?;
        inner.payloads = Some(Repository {
            spool,
            mapping: Vec::new(),
            wmo: limits.wmo,
            mode: Mode::Strict,
            observed_active: 0,
            failed: None,
        });
        inner.available_total()?;
        Ok(Self {
            inner,
            failed: None,
        })
    }
    pub fn backend(&self) -> spool::BackendProfile {
        self.inner
            .payloads
            .as_ref()
            .expect("spooled collector")
            .spool
            .backend()
    }
    pub fn resources(&self) -> super::resources::Profile {
        self.inner
            .payloads
            .as_ref()
            .expect("spooled collector")
            .spool
            .resources()
    }
    fn ready(&self) -> Result<(), CollectorError> {
        self.failed.map_or(Ok(()), Err)?;
        if let Some(e) = self.inner.payloads.as_ref().and_then(Repository::failure) {
            return Err(e.into());
        }
        self.inner.ready().map_err(CollectorError::Collision)
    }
    pub(super) fn checked<T>(
        &mut self,
        result: Result<T, CollisionError>,
    ) -> Result<T, CollectorError> {
        // Both guards execute even after failure. Changed invalidates all data.
        let source = self
            .inner
            .store
            .verify_unchanged()
            .map_err(|e| CollectorError::Collision(CollisionError::Store(e)));
        let storage = self
            .inner
            .payloads
            .as_mut()
            .ok_or(CollectorError::Collision(CollisionError::State))
            .and_then(|r| r.spool.verify_unchanged().map_err(CollectorError::Payload));
        let result = result.map_err(|e| {
            self.inner
                .payloads
                .as_ref()
                .and_then(Repository::failure)
                .map_or(CollectorError::Collision(e), CollectorError::Payload)
        });
        let changed = match (source, storage) {
            (
                Err(CollectorError::Collision(CollisionError::Store(
                    crate::local_store::Error::InputChanged,
                ))),
                _,
            ) => Some(CollectorError::Collision(CollisionError::Store(
                crate::local_store::Error::InputChanged,
            ))),
            (_, Err(CollectorError::Payload(Error::Changed))) => {
                Some(CollectorError::Payload(Error::Changed))
            }
            _ => None,
        };
        let result = if let Some(e) = changed {
            Err(e)
        } else {
            source
                .and(storage)
                .and(self.failed.map_or(Ok(()), Err))
                .and(result)
        };
        if let Err(e) = result {
            self.failed = Some(e);
            self.inner.failed.get_or_insert(CollisionError::Aborted);
        }
        result
    }
    pub fn extract(&mut self, request: ModelRequest<'_>) -> Result<ModelHandle, CollectorError> {
        let result = self.inner.extract(request);
        self.checked(result)
    }
    pub fn extract_gameobject_fdid(
        &mut self,
        id: u32,
    ) -> Result<GameObjectOutcome, CollectorError> {
        let result = self.inner.extract_gameobject_fdid(id);
        self.checked(result)
    }
    pub fn descriptor(&self, handle: ModelHandle) -> Result<Descriptor<'_>, CollectorError> {
        self.ready()?;
        Ok(Descriptor {
            model: self
                .inner
                .models
                .get(handle.index())
                .ok_or(CollisionError::State)?,
        })
    }
    pub fn with_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(Payload<'_>) -> Result<T, Error>,
    ) -> Result<T, CollectorError> {
        let result = (|| {
            self.inner.ready()?;
            let budget = self.inner.free()?;
            self.inner
                .payloads
                .as_mut()
                .ok_or(CollisionError::State)?
                .with_payload(handle, budget, call)
                .map_err(|_| CollisionError::Aborted)
        })();
        self.checked(result)
    }
    pub(super) fn with_compatible_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(CompatiblePayload<'_>) -> Result<T, Error>,
    ) -> Result<T, CollectorError> {
        let result = (|| {
            self.inner.ready()?;
            let budget = self.inner.free()?;
            let models = &self.inner.models;
            self.inner
                .payloads
                .as_mut()
                .ok_or(CollisionError::State)?
                .with_record_payload(handle, budget, true, |p| {
                    call(CompatiblePayload::validate(p, models)?)
                })
                .map_err(|_| CollisionError::Aborted)
        })();
        self.checked(result)
    }
    pub(crate) fn with_raw_result<T, E>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(Option<&[u8]>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let compatible = self
            .inner
            .payloads
            .as_ref()
            .is_some_and(|r| r.mode.compatible());
        if !compatible {
            return self.with_payload_result(handle, |p| call(p.raw()));
        }
        let mut cause = None;
        let result = self.with_compatible_payload(handle, |p| match call(p.raw()) {
            Ok(v) => Ok(v),
            Err(e) => {
                cause = Some(e);
                Err(Error::Callback)
            }
        });
        match (result, cause) {
            (Ok(v), None) => Ok(v),
            (Err(e), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: (e != CollectorError::Payload(Error::Callback)).then_some(e),
            }),
            (Err(e), None) => Err(CallbackFailure::Collector(e)),
            (Ok(_), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: Some(CollisionError::State.into()),
            }),
        }
    }
    pub fn payload_shape(&self, handle: ModelHandle) -> Result<PayloadShape, CollectorError> {
        self.ready()?;
        let model = self
            .inner
            .models
            .get(handle.index())
            .ok_or(CollisionError::State)?;
        let shape = self
            .inner
            .payloads
            .as_ref()
            .ok_or(CollisionError::State)?
            .mapping
            .iter()
            .find(|m| m.model == handle)
            .ok_or(CollisionError::State)?
            .shape;
        if model.pending
            || model.class != shape.class
            || (model.status == Status::Collision) != shape.raw_bytes.is_some()
        {
            return Err(CollisionError::State.into());
        }
        Ok(shape)
    }
    pub(crate) fn admit_reloads(
        &mut self,
        count: usize,
        bytes: usize,
    ) -> Result<(), CollectorError> {
        let result = (|| {
            self.inner.ready()?;
            let repo = self.inner.payloads.as_ref().ok_or(CollisionError::State)?;
            let stats = repo.spool.stats().map_err(|_| CollisionError::State)?;
            let limits = repo.spool.limits();
            if add(stats.reloads, count)? > limits.max_reloads
                || add(stats.reload_bytes, bytes)? > limits.max_reload_bytes
            {
                return Err(CollisionError::Limit);
            }
            Ok(())
        })();
        self.checked(result)
    }
    pub(crate) fn request_len(&self) -> Result<usize, CollectorError> {
        self.ready()?;
        Ok(self.inner.requests.len())
    }
    pub(crate) fn request_handle(&self, ordinal: usize) -> Result<ModelHandle, CollectorError> {
        self.ready()?;
        self.inner
            .requests
            .get(ordinal)
            .copied()
            .ok_or(CollisionError::State.into())
    }
    pub(crate) fn with_descriptors_result<T, E>(
        &mut self,
        call: impl FnOnce(CompactView<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        self.ready().map_err(CallbackFailure::Collector)?;
        let result = call(CompactView {
            models: &self.inner.models,
            requests: &self.inner.requests,
        });
        self.finish_external_operation(result)
    }
    /// Repository is the sole mutable borrow during the callback. Names and
    /// handles borrow immutable compact state; the complete record stays scoped.
    pub(crate) fn with_payload_descriptors_result<T, E>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(Payload<'_>, CompactView<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let mut cause = None;
        let result = (|| {
            self.inner.ready()?;
            let budget = self.inner.free()?;
            let view = CompactView {
                models: &self.inner.models,
                requests: &self.inner.requests,
            };
            self.inner
                .payloads
                .as_mut()
                .ok_or(CollisionError::State)?
                .with_payload(handle, budget, |payload| match call(payload, view) {
                    Ok(value) => Ok(value),
                    Err(error) => {
                        cause = Some(error);
                        Err(Error::Callback)
                    }
                })
                .map_err(|_| CollisionError::Aborted)
        })();
        let result = self.checked(result);
        match (result, cause) {
            (Ok(value), None) => Ok(value),
            (Err(error), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: (error != CollectorError::Payload(Error::Callback)).then_some(error),
            }),
            (Err(error), None) => Err(CallbackFailure::Collector(error)),
            (Ok(_), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: Some(CollectorError::Collision(CollisionError::State)),
            }),
        }
    }
    /// Compatible wire validation and compact descriptor borrow share the same
    /// immutable models; callbacks cannot manufacture missing-child bindings.
    pub(crate) fn with_compatible_payload_descriptors_result<T, E>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(CompatiblePayload<'_>, CompactView<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let mut cause = None;
        let result = (|| {
            self.inner.ready()?;
            let budget = self.inner.free()?;
            let view = CompactView {
                models: &self.inner.models,
                requests: &self.inner.requests,
            };
            self.inner
                .payloads
                .as_mut()
                .ok_or(CollisionError::State)?
                .with_record_payload(handle, budget, true, |payload| {
                    match call(CompatiblePayload::validate(payload, view.models)?, view) {
                        Ok(value) => Ok(value),
                        Err(error) => {
                            cause = Some(error);
                            Err(Error::Callback)
                        }
                    }
                })
                .map_err(|_| CollisionError::Aborted)
        })();
        let result = self.checked(result);
        match (result, cause) {
            (Ok(value), None) => Ok(value),
            (Err(error), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: (error != CollectorError::Payload(Error::Callback)).then_some(error),
            }),
            (Err(error), None) => Err(CallbackFailure::Collector(error)),
            (Ok(_), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: Some(CollectorError::Collision(CollisionError::State)),
            }),
        }
    }
    pub(crate) fn read_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), CollectorError> {
        let r = self.inner.read_source(id, cap);
        self.checked(r)
    }
    // Same collector/source ledger and sticky source/repository guards as the
    // complete source bridge; no complete SourceFacts are manufactured.
    pub(crate) fn read_available_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<crate::local_store::availability::AvailableSource, CollectorError> {
        let result = self.inner.read_available_source(id, cap);
        self.checked(result)
    }
    pub(crate) fn read_source_ref(
        &mut self,
        input: super::InputRef<'_>,
        cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), CollectorError> {
        let result = self.inner.read_source_ref(input, cap);
        self.checked(result)
    }
    pub(crate) fn begin_shared_run(&mut self) -> Result<(), CollectorError> {
        let result = self.inner.begin_shared_run();
        self.checked(result)
    }
    pub(crate) fn release_source(&mut self, bytes: usize) -> Result<(), CollectorError> {
        let r = self.inner.release_source(bytes);
        self.checked(r)
    }
    pub(crate) fn reserve_external(&mut self, bytes: usize) -> Result<(), CollectorError> {
        let r = self.inner.reserve_external(bytes);
        self.checked(r)
    }
    pub(crate) fn release_external(&mut self, bytes: usize) -> Result<(), CollectorError> {
        let r = self.inner.release_external(bytes);
        self.checked(r)
    }
    /// Closed assembly stages partition a disjoint reservation before world IO.
    /// Always lowers the existing ceiling; both authenticated guards still run.
    pub(crate) fn constrain_memory_for_stage(
        &mut self,
        ceiling: usize,
    ) -> Result<(), CollectorError> {
        let limit = ceiling.min(self.inner.memory_limit());
        let result = self
            .inner
            .constrain_memory(limit)
            .and_then(|()| self.inner.free_bytes().map(|_| ()));
        self.checked(result)
    }
    pub(crate) const fn memory_ceiling(&self) -> usize {
        self.inner.memory_limit()
    }
    pub(crate) fn free_bytes(&self) -> Result<usize, CollectorError> {
        self.ready()?;
        self.inner.free_bytes().map_err(CollectorError::Collision)
    }
    pub(crate) fn retained_bytes(&self) -> Result<usize, CollectorError> {
        self.ready()?;
        Ok(add(
            self.inner.retained_bytes(),
            self.inner
                .payloads
                .as_ref()
                .ok_or(CollisionError::State)?
                .resident_bytes(),
        )?)
    }
    pub(crate) fn with_payload_result<T, E>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(Payload<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let mut cause = None;
        let result = self.with_payload(handle, |p| match call(p) {
            Ok(v) => Ok(v),
            Err(e) => {
                cause = Some(e);
                Err(Error::Callback)
            }
        });
        match (result, cause) {
            (Ok(v), None) => Ok(v),
            (Err(e), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: (e != CollectorError::Payload(Error::Callback)).then_some(e),
            }),
            (Err(e), None) => Err(CallbackFailure::Collector(e)),
            (Ok(_), Some(cause)) => Err(CallbackFailure::Callback {
                cause,
                guard: Some(CollectorError::Collision(CollisionError::State)),
            }),
        }
    }
    /// Always executes both final guards; operation failures poison this owner.
    pub(crate) fn finish_external_operation<T, E>(
        &mut self,
        result: Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let guard = self.verify_unchanged();
        match result {
            Ok(v) => guard.map(|()| v).map_err(CallbackFailure::Collector),
            Err(cause) => {
                self.failed
                    .get_or_insert(CollectorError::Collision(CollisionError::Aborted));
                self.inner.failed.get_or_insert(CollisionError::Aborted);
                Err(CallbackFailure::Callback {
                    cause,
                    guard: guard.err(),
                })
            }
        }
    }
    pub fn verify_unchanged(&mut self) -> Result<(), CollectorError> {
        let result = self.inner.ready();
        self.checked(result)
    }
    pub fn facts(&self) -> Result<Facts, CollectorError> {
        self.ready()?;
        Ok(self.inner.facts())
    }
    pub const fn request_calls(&self) -> usize {
        self.inner.request_calls
    }
    pub fn stats(&self) -> Result<Stats, CollectorError> {
        Ok(stats(
            &self.inner.models,
            &self.inner.requests,
            self.inner.payloads.as_ref().ok_or(CollisionError::State)?,
        )?)
    }
    pub fn with_lookup<'next, N: LiquidLookup + ?Sized>(
        mut self,
        lookup: &'next N,
    ) -> Result<SpoolingExtractor<'s, 'next, N>, CollectorError> {
        self.verify_unchanged()?;
        Ok(SpoolingExtractor {
            inner: self.inner.with_lookup(lookup)?,
            failed: self.failed,
        })
    }
    // A failed transition retains its original owner so an owned Sink can be
    // destroyed before final source/spool guards. Public with_lookup is unchanged.
    pub(crate) fn try_with_lookup<'next, N: LiquidLookup + ?Sized>(
        mut self,
        lookup: &'next N,
    ) -> LookupTransition<SpoolingExtractor<'s, 'next, N>, Self> {
        if let Err(e) = self.verify_unchanged() {
            return LookupTransition::Failed(self, e);
        }
        // The combined guard above checks source, payload and sticky state.
        // Rebind only the borrowed lookup; every owned field/counter is moved.
        let old = self.inner;
        let inner = super::Extractor {
            store: old.store,
            lookup,
            limits: old.limits,
            models: old.models,
            requests: old.requests,
            live: old.live,
            external: old.external,
            external_sources: old.external_sources,
            request_calls: old.request_calls,
            shared_started: old.shared_started,
            reads: old.reads,
            dependency_requests: old.dependency_requests,
            source_bytes: old.source_bytes,
            raw_bytes: old.raw_bytes,
            failed: old.failed,
            payloads: old.payloads,
            compatible_descriptors: old.compatible_descriptors,
        };
        LookupTransition::Ready(SpoolingExtractor {
            inner,
            failed: self.failed,
        })
    }
    pub fn finish(self) -> Result<SpoolingExtraction, CollectorError> {
        self.finish_with_retained_external(0)
    }
    // Only the startup owner can transfer its actual Data charge; no public
    // constructor can reduce it. Original finish still requires external==0.
    pub(crate) fn finish_with_retained_external(
        mut self,
        bytes: usize,
    ) -> Result<SpoolingExtraction, CollectorError> {
        self.verify_unchanged()?;
        if self.inner.external != bytes || self.inner.external_sources != 0 {
            return Err(CollisionError::State.into());
        }
        let facts = self.inner.facts();
        let compact = compact_bytes(&self.inner.models, &self.inner.requests)?;
        let repository = self.inner.payloads.take().ok_or(CollisionError::State)?;
        repository.check_descriptors(add(compact, self.inner.compatible_descriptors)?)?;
        let store_resident = self.inner.store.stats().resident_bytes;
        let memory = self.inner.limits.max_memory_bytes;
        Ok(SpoolingExtraction {
            models: self.inner.models,
            requests: self.inner.requests,
            facts,
            repository,
            store_resident,
            memory,
            external_resident: bytes,
            compatible_descriptors: self.inner.compatible_descriptors,
            failed: None,
        })
    }
}
fn stats(
    models: &Vec<Model>,
    requests: &Vec<ModelHandle>,
    repository: &Repository,
) -> Result<Stats, CollisionError> {
    let spool = repository
        .spool
        .stats()
        .map_err(|_| CollisionError::Aborted)?;
    Ok(Stats {
        compact_bytes: compact_bytes(models, requests)?,
        repository_bytes: repository.resident_bytes(),
        observed_payload_bytes: repository.observed_active,
        active_ceiling: repository.active_limit(),
        spool,
    })
}
/// No owned RAW/root/selection vectors are hidden here. The last LocalStore
/// reservation remains conservatively charged, without subsequent store I/O.
pub struct SpoolingExtraction {
    models: Vec<Model>,
    requests: Vec<ModelHandle>,
    facts: Facts,
    repository: Repository,
    store_resident: usize,
    memory: usize,
    external_resident: usize,
    compatible_descriptors: usize,
    failed: Option<CollectorError>,
}
impl SpoolingExtraction {
    pub const fn backend(&self) -> spool::BackendProfile {
        self.repository.spool.backend()
    }
    pub const fn resources(&self) -> super::resources::Profile {
        self.repository.spool.resources()
    }
    /// Immutable completed-phase source accounting, including the caller-owned
    /// store residency recorded at finish and the exact retained Data/topology.
    /// No external charge can be removed or forged through this read-only API.
    pub(crate) fn phase_resident_bytes(&self) -> Result<usize, CollectorError> {
        self.ready()?;
        let stats = self.stats()?;
        Ok(plus(
            plus(
                plus(stats.compact_bytes, stats.repository_bytes)?,
                self.external_resident,
            )?,
            self.store_resident,
        )?)
    }
    pub(crate) fn phase_compact(&self) -> Result<CompactView<'_>, CollectorError> {
        self.ready()?;
        Ok(CompactView {
            models: &self.models,
            requests: &self.requests,
        })
    }
    pub(crate) fn phase_shape(&self, handle: ModelHandle) -> Result<PayloadShape, CollectorError> {
        self.ready()?;
        let model = self
            .models
            .get(handle.index())
            .ok_or(CollisionError::State)?;
        let shape = self
            .repository
            .mapping
            .iter()
            .find(|m| m.model == handle)
            .ok_or(CollisionError::State)?
            .shape;
        if model.pending
            || model.class != shape.class
            || (model.status == Status::Collision) != shape.raw_bytes.is_some()
        {
            return Err(CollisionError::State.into());
        }
        Ok(shape)
    }
    /// Borrow metadata from a genuine compatible owner without any payload IO.
    /// The declared heap covers captures, callback work and a returned owned T.
    /// An uncalled capture or rejected T drops before the LAST spool guard.
    pub(crate) fn with_metadata_phase<T, E: Copy>(
        &mut self,
        callback_heap: usize,
        call: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let setup = (|| {
            self.metadata_guard()?;
            self.ready()?;
            self.memory
                .checked_sub(self.phase_resident_bytes()?)
                .and_then(|n| n.checked_sub(callback_heap))
                .ok_or(CollisionError::Limit)?;
            Ok::<(), CollectorError>(())
        })();
        if let Err(e) = setup {
            drop(call);
            let e = self.metadata_guard().err().unwrap_or(e);
            self.failed = Some(e);
            return Err(CallbackFailure::Collector(e));
        }
        let outcome = call();
        match outcome {
            Err(cause) => {
                let guard = self.metadata_guard().err();
                self.failed = Some(guard.unwrap_or(CollectorError::Payload(Error::Callback)));
                Err(CallbackFailure::Callback { cause, guard })
            }
            Ok(value) => match self.metadata_guard() {
                Ok(()) => Ok(value),
                Err(e) => {
                    drop(value);
                    let e = self.metadata_guard().err().unwrap_or(e);
                    self.failed = Some(e);
                    Err(CallbackFailure::Collector(e))
                }
            },
        }
    }
    pub(crate) fn metadata_guard(&mut self) -> Result<(), CollectorError> {
        let source = CollectorError::Collision(CollisionError::Store(
            crate::local_store::Error::InputChanged,
        ));
        let previous = self.failed;
        let current = self.verify_unchanged();
        if previous == Some(source) {
            self.failed = Some(source);
            Err(source)
        } else {
            current
        }
    }
    /// Closed phases admit caller conversion/retained allowance before reload.
    /// This reduces only the remaining collector memory budget; the repository
    /// active ceiling continues to cover the source record, not caller geometry.
    /// Both must also fit the enclosing phase ledger enforced by the caller.
    pub(crate) fn with_payload_phase<T, E: Copy>(
        &mut self,
        handle: ModelHandle,
        source_allowance: usize,
        callback_heap_allowance: usize,
        call: impl FnOnce(Payload<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let setup = (|| {
            self.verify_unchanged()?;
            self.ready()?;
            let shape = self.phase_shape(handle)?;
            let memory = self
                .memory
                .checked_sub(self.phase_resident_bytes()?)
                .and_then(|n| n.checked_sub(callback_heap_allowance))
                .ok_or(CollisionError::Limit)?;
            let active = self
                .repository
                .active_limit()
                .checked_sub(
                    self.external_resident
                        .checked_sub(self.compatible_descriptors)
                        .ok_or(CollisionError::State)?,
                )
                .ok_or(CollisionError::Limit)?;
            let budget = memory.min(active).min(source_allowance);
            if shape.record_bytes > budget {
                return Err(CollisionError::Limit.into());
            }
            Ok(budget)
        })();
        let budget = match setup {
            Ok(n) => n,
            Err(e) => {
                // A refused callback may own a mutating Drop. Check after it.
                drop(call);
                let final_error = self.verify_unchanged().err().unwrap_or(e);
                self.failed = Some(final_error);
                return Err(CallbackFailure::Collector(final_error));
            }
        };
        let mut outcome = None;
        let io = self
            .repository
            .with_payload(handle, budget, |payload| {
                outcome = Some(call(payload));
                Ok(())
            })
            .map_err(CollectorError::Payload);
        match outcome {
            Some(Err(cause)) => {
                let guard = self.verify_unchanged().err().or(io.err());
                self.failed = Some(guard.unwrap_or(CollectorError::Payload(Error::Callback)));
                Err(CallbackFailure::Callback { cause, guard })
            }
            Some(Ok(value)) => {
                let guard = self.verify_unchanged();
                match io.and(guard) {
                    Ok(()) => Ok(value),
                    Err(e) => {
                        drop(value);
                        let final_error = self.verify_unchanged().err().unwrap_or(e);
                        self.failed = Some(final_error);
                        Err(CallbackFailure::Collector(final_error))
                    }
                }
            }
            None => {
                let e = self
                    .verify_unchanged()
                    .err()
                    .or(io.err())
                    .unwrap_or(CollectorError::Payload(Error::State));
                self.failed = Some(e);
                Err(CallbackFailure::Collector(e))
            }
        }
    }
    /// Compatible final phases retain the same closed memory and guard contract.
    /// MAGIC2/bindings/native order are validated against this genuine collection.
    /// This reduces only the remaining collector memory budget; the repository
    /// active ceiling continues to cover the source record, not caller geometry.
    /// Both must also fit the enclosing phase ledger enforced by the caller.
    pub(crate) fn with_compatible_payload_phase<T, E: Copy>(
        &mut self,
        handle: ModelHandle,
        source_allowance: usize,
        callback_heap_allowance: usize,
        call: impl FnOnce(CompatiblePayload<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackFailure<E>> {
        let setup = (|| {
            self.verify_unchanged()?;
            self.ready()?;
            let shape = self.phase_shape(handle)?;
            let memory = self
                .memory
                .checked_sub(self.phase_resident_bytes()?)
                .and_then(|n| n.checked_sub(callback_heap_allowance))
                .ok_or(CollisionError::Limit)?;
            let active = self
                .repository
                .active_limit()
                .checked_sub(
                    self.external_resident
                        .checked_sub(self.compatible_descriptors)
                        .ok_or(CollisionError::State)?,
                )
                .ok_or(CollisionError::Limit)?;
            let budget = memory.min(active).min(source_allowance);
            if shape.record_bytes > budget {
                return Err(CollisionError::Limit.into());
            }
            Ok(budget)
        })();
        let budget = match setup {
            Ok(n) => n,
            Err(e) => {
                // A refused callback may own a mutating Drop. Check after it.
                drop(call);
                let final_error = self.verify_unchanged().err().unwrap_or(e);
                self.failed = Some(final_error);
                return Err(CallbackFailure::Collector(final_error));
            }
        };
        let mut outcome = None;
        let io = self
            .repository
            .with_record_payload(handle, budget, true, |payload| {
                outcome = Some(call(CompatiblePayload::validate(payload, &self.models)?));
                Ok(())
            })
            .map_err(CollectorError::Payload);
        match outcome {
            Some(Err(cause)) => {
                let guard = self.verify_unchanged().err().or(io.err());
                self.failed = Some(guard.unwrap_or(CollectorError::Payload(Error::Callback)));
                Err(CallbackFailure::Callback { cause, guard })
            }
            Some(Ok(value)) => {
                let guard = self.verify_unchanged();
                match io.and(guard) {
                    Ok(()) => Ok(value),
                    Err(e) => {
                        drop(value);
                        let final_error = self.verify_unchanged().err().unwrap_or(e);
                        self.failed = Some(final_error);
                        Err(CallbackFailure::Collector(final_error))
                    }
                }
            }
            None => {
                let e = self
                    .verify_unchanged()
                    .err()
                    .or(io.err())
                    .unwrap_or(CollectorError::Payload(Error::State));
                self.failed = Some(e);
                Err(CallbackFailure::Collector(e))
            }
        }
    }
    #[cfg(test)]
    pub(crate) const fn retained_external_for_test(&self) -> usize {
        self.external_resident
    }
    pub(crate) fn ready(&self) -> Result<(), CollectorError> {
        self.failed.map_or(Ok(()), Err)?;
        self.repository.failure().map_or(Ok(()), |e| Err(e.into()))
    }
    pub fn descriptor(&self, handle: ModelHandle) -> Result<Descriptor<'_>, CollectorError> {
        self.failed.map_or(Ok(()), Err)?;
        Ok(Descriptor {
            model: self
                .models
                .get(handle.index())
                .ok_or(CollisionError::State)?,
        })
    }
    pub fn requests(&self) -> Result<&[ModelHandle], CollectorError> {
        self.failed.map_or(Ok(()), Err)?;
        Ok(&self.requests)
    }
    pub const fn facts(&self) -> Facts {
        self.facts
    }
    pub fn stats(&self) -> Result<Stats, CollectorError> {
        Ok(stats(&self.models, &self.requests, &self.repository)?)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), CollectorError> {
        let guard = self
            .repository
            .spool
            .verify_unchanged()
            .map_err(CollectorError::Payload);
        let result = if guard == Err(CollectorError::Payload(Error::Changed)) {
            guard
        } else {
            self.failed.map_or(Ok(()), Err).and(guard)
        };
        if let Err(e) = result {
            self.failed = Some(e);
        }
        result
    }
    pub fn with_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(Payload<'_>) -> Result<T, Error>,
    ) -> Result<T, CollectorError> {
        let result = (|| {
            self.verify_unchanged()?;
            let compact = compact_bytes(&self.models, &self.requests)?;
            let budget = self
                .memory
                .checked_sub(self.store_resident)
                .and_then(|n| n.checked_sub(compact))
                .and_then(|n| n.checked_sub(self.repository.resident_bytes()))
                .and_then(|n| n.checked_sub(self.external_resident))
                .ok_or(CollisionError::Limit)?;
            let active = self
                .repository
                .active_limit()
                .checked_sub(
                    self.external_resident
                        .checked_sub(self.compatible_descriptors)
                        .ok_or(CollisionError::State)?,
                )
                .ok_or(CollisionError::Limit)?;
            Ok(self
                .repository
                .with_payload(handle, budget.min(active), call)?)
        })();
        if let Err(e) = result {
            self.failed = Some(e);
        }
        result
    }
    pub(super) fn with_compatible_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(CompatiblePayload<'_>) -> Result<T, Error>,
    ) -> Result<T, CollectorError> {
        let result = (|| {
            self.verify_unchanged()?;
            let compact = compact_bytes(&self.models, &self.requests)?;
            let budget = self
                .memory
                .checked_sub(self.store_resident)
                .and_then(|n| n.checked_sub(compact))
                .and_then(|n| n.checked_sub(self.repository.resident_bytes()))
                .and_then(|n| n.checked_sub(self.external_resident))
                .ok_or(CollisionError::Limit)?;
            let active = self
                .repository
                .active_limit()
                .checked_sub(
                    self.external_resident
                        .checked_sub(self.compatible_descriptors)
                        .ok_or(CollisionError::State)?,
                )
                .ok_or(CollisionError::Limit)?;
            Ok(self
                .repository
                .with_record_payload(handle, budget.min(active), true, |p| {
                    call(CompatiblePayload::validate(p, &self.models)?)
                })?)
        })();
        if let Err(e) = result {
            self.failed = Some(e);
        }
        result
    }
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_order_cases {
    use super::*;

    fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
    fn frame(mode: Mode, references: &[u16]) -> Vec<u8> {
        let mut root_header = [0; 64];
        root_header[60] = 4;
        let mut root = chunk(b"MOHD", &root_header);
        root.extend(chunk(b"GFID", &[]));
        root.extend(chunk(b"MODI", &[0; 12]));
        let mut spawns = [0; 120];
        for index in 0..3 {
            spawns[index * 40..index * 40 + 4].copy_from_slice(&(index as u32).to_le_bytes());
            spawns[index * 40 + 32..index * 40 + 36].copy_from_slice(&1f32.to_le_bytes());
        }
        root.extend(chunk(b"MODD", &spawns));
        let mut bytes = vec![0; HEADER];
        bytes[..8].copy_from_slice(mode.magic());
        bytes[8..12].copy_from_slice(&2u32.to_le_bytes());
        for (slot, value) in bytes[12..].as_chunks_mut::<8>().0.iter_mut().zip([
            0,
            root.len(),
            0,
            references.len(),
            0,
        ]) {
            slot.copy_from_slice(&(value as u64).to_le_bytes());
        }
        bytes.extend(root);
        for reference in references {
            bytes.extend(reference.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn native_gnu15_payload_v1_rejects_v2_magic_without_fallback() {
        let bytes = frame(Mode::CompatibleGnu15V2, &[0, 1, 2]);
        assert!(Payload::parse_profile(&bytes, wmo::Limits::default(), true).is_err());
        assert!(Payload::parse(&bytes, wmo::Limits::default()).is_err());
        assert!(
            Payload::parse_mode(&bytes, wmo::Limits::default(), Mode::CompatibleGnu15V2).is_ok()
        );
    }
    #[test]
    fn native_gnu15_payload_v2_rejects_v1_magic_without_fallback() {
        let bytes = frame(Mode::CompatibleV1, &[0, 1, 2]);
        assert!(Payload::parse_profile(&bytes, wmo::Limits::default(), true).is_ok());
        assert!(
            Payload::parse_mode(&bytes, wmo::Limits::default(), Mode::CompatibleGnu15V2).is_err()
        );
    }
    #[test]
    fn native_gnu15_payload_accepts_unique_nonmonotonic_refs_and_v1_stays_sorted() {
        let bytes = frame(Mode::CompatibleGnu15V2, &[2, 0, 1]);
        let payload =
            Payload::parse_mode(&bytes, wmo::Limits::default(), Mode::CompatibleGnu15V2).unwrap();
        assert_eq!(
            payload.wmo().unwrap().references().collect::<Vec<_>>(),
            [2, 0, 1]
        );
        let old = frame(Mode::CompatibleV1, &[2, 0, 1]);
        assert!(Payload::parse_profile(&old, wmo::Limits::default(), true).is_err());
    }
    #[test]
    fn native_gnu15_payload_rejects_duplicate_refs() {
        let bytes = frame(Mode::CompatibleGnu15V2, &[2, 0, 2]);
        assert!(
            Payload::parse_mode(&bytes, wmo::Limits::default(), Mode::CompatibleGnu15V2).is_err()
        );
    }
    #[test]
    fn native_gnu15_payload_rejects_reference_outside_spawn_count() {
        let bytes = frame(Mode::CompatibleGnu15V2, &[3]);
        assert!(
            Payload::parse_mode(&bytes, wmo::Limits::default(), Mode::CompatibleGnu15V2).is_err()
        );
    }
    #[test]
    fn native_gnu15_payload_bitmap_accepts_entire_u16_domain_in_reverse() {
        let bytes: Vec<_> = (0..=u16::MAX).rev().flat_map(u16::to_le_bytes).collect();
        assert!(validate_native_references(&bytes, 65536).is_ok());
        assert!(validate_native_references(&bytes, 65535).is_err());
    }
    #[test]
    fn native_gnu15_payload_bitmap_rejects_odd_and_overdomain_lengths() {
        assert!(validate_native_references(&[0], 1).is_err());
        assert!(validate_native_references(&vec![0; 65537 * 2], 65536).is_err());
        assert!(validate_native_references(&[], 0).is_ok());
    }
}
