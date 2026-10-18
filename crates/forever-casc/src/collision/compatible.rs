// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-16: opt-in staged complete-model acquisition.
// Modified 2026-10-17: GNU15-only complete no-collision WMO omission.
//! Typed source absence becomes an omission. The explicit GNU15-v2 profile also
//! omits complete WMOs without usable collision, preserving later serial work.
//! This owner remains separate from strict APIs and final packaging.
use super::{
    Class, Error, Extractor, Facts, InputRef, Limits, Model, ModelHandle, ModelRequest, Origin,
    SourceFacts, Status, WmoMetadata, add, capacity, classify, names, payload, reserve,
    spool::Spool, target::Target,
};
use crate::local_store::LocalStore;
use forever_vmap::wmo::{self, doodads, liquid::LiquidLookup};

mod native_order;
pub mod startup;
#[cfg(test)]
mod tests;
/// Same-owner available Map/complete liquid world preparation; no traversal.
pub mod world;
/// Separately selected serial projection using this compatible collector.
pub use super::placements::compatible as placements;

pub const PROFILE_70374: &str = "linux70374-disabled-key-compatible-model-spool-v1";
pub const PROFILE_70425: &str = "linux70425-disabled-key-compatible-model-spool-v1";
pub const PROFILE_70450: &str = "linux70450-disabled-key-compatible-model-spool-v1";
pub const PROFILE_GNU15_70374: &str =
    "linux70374-disabled-key-compatible-model-spool-gnu15-native-order-v2";
pub const PROFILE_GNU15_70425: &str =
    "linux70425-disabled-key-compatible-model-spool-gnu15-native-order-v2";
pub const PROFILE_GNU15_70450: &str =
    "linux70450-disabled-key-compatible-model-spool-gnu15-native-order-v2";

pub(in crate::collision) const fn model_profile_for(
    target: Target,
    native_order: bool,
) -> &'static str {
    match (target, native_order) {
        (Target::Build70374, false) => PROFILE_70374,
        (Target::Build70425, false) => PROFILE_70425,
        (Target::Build70450, false) => PROFILE_70450,
        (Target::Build70374, true) => PROFILE_GNU15_70374,
        (Target::Build70425, true) => PROFILE_GNU15_70425,
        (Target::Build70450, true) => PROFILE_GNU15_70450,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Omission {
    MissingRoot,
    MissingRequiredGroup,
    NoUsableCollisionAfterOmission,
    /// Complete root and required groups, without geometry or usable sets.
    /// Child routes may still be absent or unreferenced; no child completeness
    /// or unknown geometry is inferred. Only the GNU15-v2 profile emits this.
    CompleteNoCollision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Present(ModelHandle),
    Omitted(Omission),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameObjectOutcome {
    Model(ModelHandle),
    Texture,
    Omitted(Omission),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Present(ModelHandle),
    Missing,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub name_index: u32,
    pub disposition: Disposition,
}
type CachedOutcome = (Outcome, Option<[u8; 4]>);

struct Converted {
    status: Status,
    source: SourceFacts,
    raw: Option<Vec<u8>>,
    wmo: Option<WmoMetadata>,
    bindings: Option<Vec<Binding>>,
    native_references: Option<Vec<u16>>,
}
struct Identity {
    class: Class,
    name: Vec<u8>,
    origin: Origin,
}
enum Slot {
    Vacant,
    Building(Identity),
    Complete(ModelHandle),
    Omitted {
        identity: Identity,
        reason: Omission,
        magic: Option<[u8; 4]>,
    },
}
#[derive(Default)]
struct State {
    native_order: bool,
    entries: Vec<Slot>,
    resident: usize,
}
impl State {
    // Full new allocation overlaps old capacity. Every negative identity shares
    // the same descriptor partition as completed models; there is no eviction.
    fn charge<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        n: usize,
    ) -> Result<(), Error> {
        let next = add(e.compatible_descriptors, n)?;
        e.payloads
            .as_ref()
            .ok_or(Error::State)?
            .check_descriptors(add(payload::compact_bytes(&e.models, &e.requests)?, next)?)
            .map_err(|_| Error::Limit)?;
        e.compatible_descriptors = next;
        e.reserve_external(n)?;
        self.resident = add(self.resident, n)?;
        Ok(())
    }
    fn release<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        n: usize,
    ) -> Result<(), Error> {
        self.resident = self.resident.checked_sub(n).ok_or(Error::State)?;
        e.compatible_descriptors = e
            .compatible_descriptors
            .checked_sub(n)
            .ok_or(Error::State)?;
        e.release_external(n)
    }
    fn room<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
    ) -> Result<(), Error> {
        if self.entries.len()
            >= e.limits
                .max_models
                .min(e.payloads.as_ref().ok_or(Error::State)?.identity_limit())
        {
            return Err(Error::Limit);
        }
        if self.entries.len() == self.entries.capacity() {
            let n = add(self.entries.len(), 1)?
                .checked_mul(size_of::<Slot>())
                .ok_or(Error::Limit)?;
            self.charge(e, n)?;
            let old = capacity(&self.entries)?;
            self.entries
                .try_reserve_exact(1)
                .map_err(|_| Error::Allocation)?;
            let actual = capacity(&self.entries)?;
            if actual > n {
                self.charge(e, actual - n)?;
            }
            self.release(e, old)?;
        }
        Ok(())
    }
    fn bytes<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        input: &[u8],
    ) -> Result<Vec<u8>, Error> {
        self.charge(e, input.len())?;
        let mut out = Vec::new();
        out.try_reserve_exact(input.len())
            .map_err(|_| Error::Allocation)?;
        if out.capacity() > input.len() {
            self.charge(e, out.capacity() - input.len())?;
        }
        out.extend_from_slice(input);
        Ok(out)
    }
    fn begin<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        class: Class,
        p: &names::Prepared,
    ) -> Result<usize, Error> {
        self.room(e)?;
        let name = self.bytes(e, p.name())?;
        let origin = match p.id {
            Some(id) => Origin::Id(id),
            None => Origin::Name(self.bytes(e, p.canonical())?),
        };
        let at = self.entries.len();
        self.entries.push(Slot::Building(Identity {
            class,
            name,
            origin,
        }));
        Ok(at)
    }
    fn lookup<L: LiquidLookup + ?Sized>(
        &self,
        e: &Extractor<'_, '_, L>,
        p: &names::Prepared,
        class: Option<Class>,
    ) -> Result<Option<CachedOutcome>, Error> {
        for slot in &self.entries {
            let (name, origin, actual_class, result, magic) = match slot {
                Slot::Vacant => return Err(Error::State),
                Slot::Building(i) => (i.name.as_slice(), &i.origin, i.class, None, None),
                Slot::Complete(h) => {
                    let m = e.models.get(h.index()).ok_or(Error::State)?;
                    (
                        m.name.as_slice(),
                        &m.origin,
                        m.class,
                        Some(Outcome::Present(*h)),
                        m.source.as_ref().ok_or(Error::State)?.first_magic,
                    )
                }
                Slot::Omitted {
                    identity: i,
                    reason,
                    magic,
                } => (
                    i.name.as_slice(),
                    &i.origin,
                    i.class,
                    Some(Outcome::Omitted(*reason)),
                    *magic,
                ),
            };
            if name != p.name() {
                continue;
            }
            let same = match (origin, p.id) {
                (Origin::Id(a), Some(b)) => *a == b,
                (Origin::Name(a), None) => a == p.canonical(),
                _ => false,
            };
            if result.is_none() {
                return Err(Error::Cycle);
            }
            if !same || class.is_some_and(|c| c != actual_class) {
                return Err(Error::Alias);
            }
            return result.map(|r| Some((r, magic))).ok_or(Error::Cycle);
        }
        Ok(None)
    }
    fn omit(
        &mut self,
        at: usize,
        reason: Omission,
        magic: Option<[u8; 4]>,
    ) -> Result<Outcome, Error> {
        // Move identity without removing, shifting or manufacturing a handle.
        let old = std::mem::replace(&mut self.entries[at], Slot::Vacant);
        let Slot::Building(identity) = old else {
            return Err(Error::State);
        };
        self.entries[at] = Slot::Omitted {
            identity,
            reason,
            magic,
        };
        Ok(Outcome::Omitted(reason))
    }
    fn commit<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        at: usize,
        converted: Converted,
    ) -> Result<Outcome, Error> {
        let Converted {
            status,
            source,
            raw,
            wmo,
            bindings,
            native_references,
        } = converted;
        let cap = e.vector_reservation_cap(&e.models, 1, true)?;
        reserve(&mut e.models, 1, &mut e.live, cap)?;
        let handle = ModelHandle(e.models.len());
        let old = std::mem::replace(&mut self.entries[at], Slot::Complete(handle));
        let Slot::Building(i) = old else {
            return Err(Error::State);
        };
        let bytes = add(
            i.name.capacity(),
            match &i.origin {
                Origin::Id(_) => 0,
                Origin::Name(n) => n.capacity(),
            },
        )?;
        // Identity ownership moves to the compact model without a residency
        // refund or allocation. Cache vector capacity remains externally owned.
        self.resident = self.resident.checked_sub(bytes).ok_or(Error::State)?;
        e.external = e.external.checked_sub(bytes).ok_or(Error::State)?;
        e.compatible_descriptors = e
            .compatible_descriptors
            .checked_sub(bytes)
            .ok_or(Error::State)?;
        e.models.push(Model {
            class: i.class,
            name: i.name,
            origin: i.origin,
            pending: false,
            status,
            raw,
            source: Some(source),
            wmo,
        });
        let compact = e.compact_descriptors()?;
        let repo_bytes = e.payloads.as_ref().ok_or(Error::State)?.resident_bytes();
        let budget = e
            .limits
            .max_memory_bytes
            .checked_sub(e.store.stats().resident_bytes)
            .and_then(|n| n.checked_sub(e.live))
            .and_then(|n| n.checked_sub(repo_bytes))
            .ok_or(Error::Limit)?;
        let repository = e.payloads.as_mut().ok_or(Error::State)?;
        repository
            .prepare(e.models.len(), compact, budget)
            .map_err(|_| Error::Aborted)?;
        let released = repository
            .store_compatible(
                handle,
                &mut e.models[handle.index()],
                compact,
                e.live,
                bindings.as_deref(),
                native_references.as_deref(),
            )
            .map_err(|_| Error::Aborted)?;
        e.release(released);
        if let Some(bindings) = bindings {
            e.release(capacity(&bindings)?);
            drop(bindings);
        }
        if let Some(references) = native_references {
            let bytes = capacity(&references)?;
            drop(references);
            e.release(bytes);
        }
        e.available_total()?;
        Ok(Outcome::Present(handle))
    }
    fn obtain<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        class: Class,
        input: InputRef<'_>,
        request_kind: crate::local_store::ProductDiagnosticRequestKind,
    ) -> Result<Outcome, Error> {
        let p = names::Prepared::new(input, class)?;
        e.store.verify_unchanged().map_err(Error::Store)?;
        if let Some((found, _)) = self.lookup(e, &p, Some(class))? {
            return Ok(found);
        }
        let at = self.begin(e, class, &p)?;
        let route =
            p.id.map_or(InputRef::LogicalName(p.source()), InputRef::FileDataId);
        let Some((bytes, source)) =
            e.read_capped_policy(route, e.limits.max_file_bytes, true, request_kind)?
        else {
            return self.omit(at, Omission::MissingRoot, None);
        };
        self.convert(e, at, class, bytes, source)
    }
    fn gameobject<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        id: u32,
    ) -> Result<GameObjectOutcome, Error> {
        let p = names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
        e.store.verify_unchanged().map_err(Error::Store)?;
        // Typed MissingRoot has no verified magic, so cannot classify a GODI
        // row. Re-read below, without relabelling that negative identity.
        if let Some((outcome, Some(magic))) = self.lookup(e, &p, None)? {
            classify(Some(magic))?;
            return Ok(match outcome {
                Outcome::Present(h) => GameObjectOutcome::Model(h),
                Outcome::Omitted(r) => GameObjectOutcome::Omitted(r),
            });
        }
        self.room(e)?;
        let Some((bytes, source)) = e.read_capped_policy(
            InputRef::FileDataId(id),
            e.limits.max_file_bytes,
            true,
            crate::local_store::ProductDiagnosticRequestKind::GameObjectModel,
        )?
        else {
            return Ok(GameObjectOutcome::Omitted(Omission::MissingRoot));
        };
        let Some(class) = classify(source.first_magic)? else {
            e.release(bytes.capacity());
            return Ok(GameObjectOutcome::Texture);
        };
        if let Some((outcome, _)) = self.lookup(e, &p, Some(class))? {
            e.release(bytes.capacity());
            return Ok(match outcome {
                Outcome::Present(h) => GameObjectOutcome::Model(h),
                Outcome::Omitted(r) => GameObjectOutcome::Omitted(r),
            });
        }
        let at = self.begin(e, class, &p)?;
        Ok(match self.convert(e, at, class, bytes, source)? {
            Outcome::Present(h) => GameObjectOutcome::Model(h),
            Outcome::Omitted(r) => GameObjectOutcome::Omitted(r),
        })
    }
    fn convert<L: LiquidLookup + ?Sized>(
        &mut self,
        e: &mut Extractor<'_, '_, L>,
        at: usize,
        class: Class,
        bytes: Vec<u8>,
        source: SourceFacts,
    ) -> Result<Outcome, Error> {
        if class == Class::M2 {
            let raw = e.convert_m2_bytes(&bytes)?;
            let status = if raw.is_some() {
                Status::Collision
            } else {
                Status::NoCollision
            };
            let capacity = bytes.capacity();
            drop(bytes);
            e.release(capacity);
            return self.commit(
                e,
                at,
                Converted {
                    status,
                    source,
                    raw,
                    wmo: None,
                    bindings: None,
                    native_references: None,
                },
            );
        }
        let root = wmo::Root::parse(&bytes, e.limits.wmo).map_err(Error::Wmo)?;
        let mut bindings = Vec::new();
        let mut valid = Vec::new();
        let mut omitted = Vec::new();
        for request in doodads::requests(&root) {
            if e.dependency_requests >= e.limits.max_dependency_requests {
                return Err(Error::Limit);
            }
            e.dependency_requests += 1;
            let input = match request.target {
                doodads::Target::FileDataId(id) => InputRef::FileDataId(id),
                doodads::Target::Name(n) => InputRef::LogicalName(n),
            };
            let outcome = self.obtain(
                e,
                Class::M2,
                input,
                crate::local_store::ProductDiagnosticRequestKind::Child,
            )?;
            let cap = e.vector_reservation_cap(&bindings, 1, false)?;
            reserve(&mut bindings, 1, &mut e.live, cap)?;
            let disposition = match outcome {
                Outcome::Present(h) => {
                    if e.models[h.index()].status == Status::Collision {
                        let cap = e.vector_reservation_cap(&valid, 1, false)?;
                        reserve(&mut valid, 1, &mut e.live, cap)?;
                        valid.push(request.name_index);
                    }
                    Disposition::Present(h)
                }
                Outcome::Omitted(_) => {
                    let cap = e.vector_reservation_cap(&omitted, 1, false)?;
                    reserve(&mut omitted, 1, &mut e.live, cap)?;
                    omitted.push(request.name_index);
                    Disposition::Missing
                }
            };
            bindings.push(Binding {
                name_index: request.name_index,
                disposition,
            });
        }
        let mut groups: Vec<Vec<u8>> = Vec::new();
        let mut facts = Vec::new();
        for id in root.detailed_group_ids() {
            // Apply input/sentinel validation before exact MissingFile handling.
            names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
            let Some((group, fact)) = e.read_capped_policy(
                InputRef::FileDataId(id),
                e.limits.max_file_bytes,
                true,
                crate::local_store::ProductDiagnosticRequestKind::WmoGroup,
            )?
            else {
                for g in &groups {
                    e.release(g.capacity());
                }
                e.release(capacity(&groups)?);
                e.release(capacity(&facts)?);
                e.release(capacity(&bindings)?);
                e.release(capacity(&valid)?);
                e.release(capacity(&omitted)?);
                e.release(bytes.capacity());
                return self.omit(at, Omission::MissingRequiredGroup, source.first_magic);
            };
            let cap = e.vector_reservation_cap(&groups, 1, false)?;
            reserve(&mut groups, 1, &mut e.live, cap)?;
            groups.push(group);
            let cap = e.vector_reservation_cap(&facts, 1, false)?;
            reserve(&mut facts, 1, &mut e.live, cap)?;
            facts.push(fact);
        }
        let (selection, raw, omitted_usable) =
            e.convert_wmo_geometry(&root, &groups, &facts, &valid, &omitted)?;
        // Strict Selection remains sorted. Only the explicitly selected v2
        // retains the separate GNU15 traversal while physical group bytes live.
        let native_references = if self.native_order {
            Some(native_references(e, &root, &groups, &facts, &selection)?)
        } else {
            None
        };
        e.release(capacity(&omitted)?);
        for g in &groups {
            e.release(g.capacity());
        }
        e.release(capacity(&groups)?);
        drop(groups);
        e.release(capacity(&valid)?);
        drop(valid);
        drop(omitted);
        let status = match selection.outcome(raw.is_some()) {
            doodads::Outcome::Ok => Ok(Status::Collision),
            doodads::Outcome::OkNoCollision => Ok(Status::DoodadOnly),
            doodads::Outcome::FailedNoCollision if omitted_usable => {
                Err(Omission::NoUsableCollisionAfterOmission)
            }
            doodads::Outcome::FailedNoCollision if self.native_order => {
                Err(Omission::CompleteNoCollision)
            }
            doodads::Outcome::FailedNoCollision => return Err(Error::NoCollision),
        };
        let status = match status {
            Ok(status) => status,
            Err(reason) => {
                let references_bytes = native_references
                    .as_ref()
                    .map(capacity)
                    .transpose()?
                    .unwrap_or(0);
                let selection_bytes = selection.resident_bytes();
                let facts_bytes = capacity(&facts)?;
                let bindings_bytes = capacity(&bindings)?;
                let root_bytes = bytes.capacity();
                // The root view only borrows bytes and has no owned heap.
                // Destroy every owned temporary before refunding its charge
                // and caching absence. Previously committed children survive.
                drop(native_references);
                drop(selection);
                drop(facts);
                drop(bindings);
                drop(raw);
                drop(bytes);
                e.release(references_bytes);
                e.release(selection_bytes);
                e.release(facts_bytes);
                e.release(bindings_bytes);
                e.release(root_bytes);
                return self.omit(at, reason, source.first_magic);
            }
        };
        let metadata = WmoMetadata {
            root_bytes: bytes,
            bindings: Vec::new(),
            selection,
            groups: facts,
            limits: e.limits.wmo,
        };
        self.commit(
            e,
            at,
            Converted {
                status,
                source,
                raw,
                wmo: Some(metadata),
                bindings: Some(bindings),
                native_references,
            },
        )
    }
}

fn native_references<L: LiquidLookup + ?Sized>(
    e: &mut Extractor<'_, '_, L>,
    root: &wmo::Root<'_>,
    group_bytes: &[Vec<u8>],
    group_facts: &[SourceFacts],
    selection: &doodads::Selection,
) -> Result<Vec<u16>, Error> {
    // This separate traversal shares the existing projection ceiling with the
    // retained sorted Selection. Each new actual capacity is subtracted before
    // the next allocation: Group + GroupInput + contexts + Order work. Order
    // work itself includes its output vector, so finish never double-charges it.
    let mut projection = e
        .limits
        .wmo
        .max_projection_bytes
        .checked_sub(selection.resident_bytes())
        .ok_or(Error::Limit)?;
    let mut groups = Vec::new();
    let cap = e
        .vector_reservation_cap(&groups, group_bytes.len(), false)?
        .min(add(e.live, projection)?);
    reserve(&mut groups, group_bytes.len(), &mut e.live, cap)?;
    let group_capacity = capacity(&groups)?;
    projection = projection.checked_sub(group_capacity).ok_or(Error::Limit)?;
    for bytes in group_bytes {
        groups.push(wmo::Group::parse(bytes, e.limits.wmo).map_err(Error::Wmo)?);
    }
    let mut inputs = Vec::new();
    let cap = e
        .vector_reservation_cap(&inputs, groups.len(), false)?
        .min(add(e.live, projection)?);
    reserve(&mut inputs, groups.len(), &mut e.live, cap)?;
    let input_capacity = capacity(&inputs)?;
    projection = projection.checked_sub(input_capacity).ok_or(Error::Limit)?;
    for (source, group) in group_facts.iter().zip(&groups) {
        inputs.push(wmo::GroupInput {
            file_data_id: source.id,
            group,
        });
    }
    let contexts = wmo::resolve_contexts(
        root,
        &inputs,
        wmo::Limits {
            max_projection_bytes: projection.min(e.free()?),
            ..e.limits.wmo
        },
    )
    .map_err(Error::Wmo)?;
    let context_capacity = capacity(&contexts)?;
    e.charge(context_capacity)?;
    let budget = projection
        .checked_sub(context_capacity)
        .ok_or(Error::Limit)?
        .min(e.free()?);
    let mut order = native_order::Order::new(selection.references().len(), budget)?;
    let work = order.work_bytes()?;
    e.charge(work)?;
    // Selection already applied exact spawn bounds, successful-collision names
    // and skipped contexts. Missing/NoCollision never affect insertion history.
    for context in &contexts {
        if context.skip_reason().is_some() {
            continue;
        }
        let group = context.group();
        for index in 0..group.doodad_reference_count() {
            let reference = group.doodad_reference(index).ok_or(Error::State)?;
            if selection.references().binary_search(&reference).is_ok() {
                order.insert(reference)?;
            }
        }
    }
    let references = order.finish()?;
    let retained = capacity(&references)?;
    e.release(work.checked_sub(retained).ok_or(Error::State)?);
    drop(contexts);
    e.release(context_capacity);
    drop(inputs);
    e.release(input_capacity);
    drop(groups);
    e.release(group_capacity);
    Ok(references)
}

pub struct Collector<'s, 'l, L: LiquidLookup + ?Sized> {
    pub(super) inner: payload::SpoolingExtractor<'s, 'l, L>,
    state: State,
    target: Target,
}
impl<'s, 'l, L: LiquidLookup + ?Sized> Collector<'s, 'l, L> {
    pub(super) fn new(
        store: &'s mut LocalStore,
        lookup: &'l L,
        target: Target,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new_with_native_order(store, lookup, target, limits, spool, false)
    }
    fn new_with_native_order(
        store: &'s mut LocalStore,
        lookup: &'l L,
        target: Target,
        limits: Limits,
        spool: Spool,
        native_order: bool,
    ) -> Result<Self, payload::CollectorError> {
        target.check(store)?;
        let mut inner = payload::SpoolingExtractor::new(store, lookup, limits, spool)?;
        let repository = inner.inner.payloads.as_mut().ok_or(Error::State)?;
        if native_order {
            repository.enable_compatible_native_order()?;
        } else {
            repository.enable_compatible()?;
        }
        Ok(Self {
            inner,
            state: State {
                native_order,
                ..State::default()
            },
            target,
        })
    }
    pub fn new_70374(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new(store, lookup, Target::Build70374, limits, spool)
    }
    pub fn new_70425(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new(store, lookup, Target::Build70425, limits, spool)
    }
    pub fn new_70450(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new(store, lookup, Target::Build70450, limits, spool)
    }
    /// Explicit GNU libstdc++15 doodad order; existing v1 constructors stay sorted.
    pub fn new_native_gnu15_70374(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new_with_native_order(store, lookup, Target::Build70374, limits, spool, true)
    }
    pub fn new_native_gnu15_70425(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new_with_native_order(store, lookup, Target::Build70425, limits, spool, true)
    }
    pub fn new_native_gnu15_70450(
        store: &'s mut LocalStore,
        lookup: &'l L,
        limits: Limits,
        spool: Spool,
    ) -> Result<Self, payload::CollectorError> {
        Self::new_with_native_order(store, lookup, Target::Build70450, limits, spool, true)
    }
    pub fn profile(&self) -> &'static str {
        model_profile_for(self.target, self.state.native_order)
    }
    pub fn extract(
        &mut self,
        request: ModelRequest<'_>,
    ) -> Result<Outcome, payload::CollectorError> {
        let result = (|| {
            self.inner.inner.begin_request()?;
            let (class, input) = match request {
                ModelRequest::M2(i) => (Class::M2, i),
                ModelRequest::Wmo(i) => (Class::Wmo, i),
            };
            let request_kind = match class {
                Class::M2 => crate::local_store::ProductDiagnosticRequestKind::RootM2,
                Class::Wmo => crate::local_store::ProductDiagnosticRequestKind::WmoRoot,
            };
            let result = self
                .state
                .obtain(&mut self.inner.inner, class, input, request_kind)?;
            if let Outcome::Present(h) = result {
                self.inner.inner.record_request(h)?;
            }
            Ok(result)
        })();
        // Missing is already an Ok typed disposition before sticky/guard closure.
        self.inner.checked(result)
    }
    pub fn extract_gameobject_fdid(
        &mut self,
        id: u32,
    ) -> Result<GameObjectOutcome, payload::CollectorError> {
        let result = (|| {
            self.inner.inner.begin_request()?;
            let outcome = self.state.gameobject(&mut self.inner.inner, id)?;
            if let GameObjectOutcome::Model(h) = outcome {
                self.inner.inner.record_request(h)?;
            }
            Ok(outcome)
        })();
        self.inner.checked(result)
    }
    /// Bounds-checked enumeration of genuine committed models only. This
    /// metadata access preserves sticky readiness; it performs no IO, allocation
    /// or acquisition/work charge. Use payload/verify/finish for guarded IO.
    pub fn model_handle(&self, ordinal: usize) -> Result<ModelHandle, payload::CollectorError> {
        let count = self.inner.facts()?.models;
        if ordinal >= count {
            return Err(Error::State.into());
        }
        let handle = ModelHandle(ordinal);
        if self.inner.descriptor(handle)?.source().is_none() {
            return Err(Error::State.into());
        }
        Ok(handle)
    }
    pub fn descriptor(
        &self,
        handle: ModelHandle,
    ) -> Result<payload::Descriptor<'_>, payload::CollectorError> {
        self.inner.descriptor(handle)
    }
    pub fn payload_shape(
        &self,
        handle: ModelHandle,
    ) -> Result<payload::PayloadShape, payload::CollectorError> {
        self.inner.payload_shape(handle)
    }
    pub fn with_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(payload::CompatiblePayload<'_>) -> Result<T, payload::Error>,
    ) -> Result<T, payload::CollectorError> {
        self.inner.with_compatible_payload(handle, call)
    }
    pub(in crate::collision) const fn target(&self) -> Target {
        self.target
    }
    pub(in crate::collision) fn set_product_diagnostic_phase(
        &mut self,
        phase: crate::local_store::ProductDiagnosticPhase,
    ) {
        self.inner.inner.set_product_diagnostic_phase(phase);
    }
    pub(in crate::collision) const fn native_order(&self) -> bool {
        self.state.native_order
    }
    pub(crate) fn read_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<(Vec<u8>, SourceFacts), payload::CollectorError> {
        self.inner.read_source(id, cap)
    }
    // Trusted world traversal only: exact typed MissingFile is absence. Keep
    // successful source capacities in the same source-release ledger; no facts
    // are manufactured for an omitted WDT/own object ADT.
    pub(in crate::collision) fn read_world_source(
        &mut self,
        input: InputRef<'_>,
        cap: usize,
        request_kind: crate::local_store::ProductDiagnosticRequestKind,
    ) -> Result<Option<(Vec<u8>, SourceFacts)>, payload::CollectorError> {
        let result = (|| {
            let e = &mut self.inner.inner;
            e.ready()?;
            if let InputRef::FileDataId(id) = input {
                names::Prepared::new(InputRef::FileDataId(id), Class::M2)?;
            }
            let value = e.read_capped_policy(input, cap, true, request_kind)?;
            if let Some((bytes, _)) = &value {
                e.external_sources = add(e.external_sources, bytes.capacity())?;
            }
            Ok(value)
        })();
        self.inner.checked(result)
    }
    pub(in crate::collision) fn read_available_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<crate::local_store::availability::AvailableSource, payload::CollectorError> {
        let result = self
            .inner
            .inner
            .read_available_source_target(id, cap, self.target);
        self.inner.checked(result)
    }
    pub(crate) fn release_source(&mut self, bytes: usize) -> Result<(), payload::CollectorError> {
        self.inner.release_source(bytes)
    }
    pub(crate) fn begin_shared_run(&mut self) -> Result<(), payload::CollectorError> {
        self.inner.begin_shared_run()
    }
    pub fn backend(&self) -> super::spool::BackendProfile {
        self.inner.backend()
    }
    pub(crate) fn memory_ceiling(&self) -> usize {
        self.inner.memory_ceiling()
    }
    // Closed placement stage owns this disjoint reservation for its full lifetime.
    // This forwards the existing checked clamp and cannot increase any ceiling.
    pub(in crate::collision) fn constrain_memory_for_stage(
        &mut self,
        ceiling: usize,
    ) -> Result<(), payload::CollectorError> {
        self.inner.constrain_memory_for_stage(ceiling)
    }
    pub(crate) fn retained_bytes(&self) -> Result<usize, payload::CollectorError> {
        self.inner.retained_bytes()
    }
    pub(crate) fn resources(&self) -> super::resources::Profile {
        self.inner.resources()
    }
    pub(crate) fn reserve_external(&mut self, bytes: usize) -> Result<(), payload::CollectorError> {
        self.inner.reserve_external(bytes)
    }
    pub(crate) fn release_external(&mut self, bytes: usize) -> Result<(), payload::CollectorError> {
        self.inner.release_external(bytes)
    }
    pub(crate) fn free_bytes(&self) -> Result<usize, payload::CollectorError> {
        self.inner.free_bytes()
    }
    pub(crate) fn admit_reloads(
        &mut self,
        count: usize,
        bytes: usize,
    ) -> Result<(), payload::CollectorError> {
        self.inner.admit_reloads(count, bytes)
    }
    pub(crate) fn finish_external_operation<T, E>(
        &mut self,
        result: Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.inner.finish_external_operation(result)
    }
    pub(crate) fn with_descriptors_result<T, E>(
        &mut self,
        call: impl FnOnce(payload::CompactView<'_>) -> Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.inner.with_descriptors_result(call)
    }
    pub(crate) fn with_compatible_payload_descriptors_result<T, E>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(payload::CompatiblePayload<'_>, payload::CompactView<'_>) -> Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.inner
            .with_compatible_payload_descriptors_result(handle, call)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), payload::CollectorError> {
        self.inner.verify_unchanged()
    }
    pub fn facts(&self) -> Result<Facts, payload::CollectorError> {
        self.inner.facts()
    }
    pub fn request_calls(&self) -> usize {
        self.inner.request_calls()
    }
    pub fn with_lookup<'n, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'n N,
    ) -> Result<Collector<'s, 'n, N>, payload::CollectorError> {
        Ok(Collector {
            inner: self.inner.with_lookup(lookup)?,
            state: self.state,
            target: self.target,
        })
    }
    // Recover the original owner on failed guard admission so owned user state
    // can drop before final guards; cache/handles/target/order move unchanged.
    pub(in crate::collision) fn try_with_lookup<'n, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'n N,
    ) -> payload::LookupTransition<Collector<'s, 'n, N>, Self> {
        let Self {
            inner,
            state,
            target,
        } = self;
        match inner.try_with_lookup(lookup) {
            payload::LookupTransition::Ready(inner) => {
                payload::LookupTransition::Ready(Collector {
                    inner,
                    state,
                    target,
                })
            }
            payload::LookupTransition::Failed(inner, error) => payload::LookupTransition::Failed(
                Collector {
                    inner,
                    state,
                    target,
                },
                error,
            ),
        }
    }
    pub fn finish(self) -> Result<Completed, payload::CollectorError> {
        self.finish_with_external(0)
    }
    pub(super) fn finish_with_external(
        self,
        retained: usize,
    ) -> Result<Completed, payload::CollectorError> {
        let extraction = self
            .inner
            .finish_with_retained_external(add(retained, self.state.resident)?)?;
        Ok(Completed {
            extraction,
            state: self.state,
            target: self.target,
        })
    }
}
pub struct Completed {
    extraction: payload::SpoolingExtraction,
    state: State,
    target: Target,
}
impl Completed {
    // Modified 2026-10-17: private genuine compatible VMO phase facade. No
    // mutable extraction, caller metadata or unvalidated RAW mode is exposed.
    pub(crate) fn phase_compact(
        &self,
    ) -> Result<payload::CompactView<'_>, payload::CollectorError> {
        self.extraction.phase_compact()
    }
    pub(crate) fn phase_resident_bytes(&self) -> Result<usize, payload::CollectorError> {
        self.extraction.phase_resident_bytes()
    }
    pub(crate) fn with_payload_phase<T, E: Copy>(
        &mut self,
        handle: ModelHandle,
        source_allowance: usize,
        callback_heap: usize,
        call: impl FnOnce(payload::CompatiblePayload<'_>) -> Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.extraction
            .with_compatible_payload_phase(handle, source_allowance, callback_heap, call)
    }

    pub(crate) fn verify_metadata_unchanged(&mut self) -> Result<(), payload::CollectorError> {
        self.extraction.metadata_guard()
    }
    pub(crate) fn with_metadata_phase<T, E: Copy>(
        &mut self,
        callback_heap: usize,
        call: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.extraction.with_metadata_phase(callback_heap, call)
    }

    /// Metadata readiness only; phase owners separately authenticate source/spool IO.
    pub(in crate::collision) fn metadata_ready(&self) -> Result<(), payload::CollectorError> {
        self.extraction.ready()
    }
    pub const fn backend(&self) -> super::spool::BackendProfile {
        self.extraction.backend()
    }
    pub const fn resources(&self) -> super::resources::Profile {
        self.extraction.resources()
    }
    pub fn payload_shape(
        &self,
        handle: ModelHandle,
    ) -> Result<payload::PayloadShape, payload::CollectorError> {
        self.extraction.phase_shape(handle)
    }
    pub fn facts(&self) -> Facts {
        self.extraction.facts()
    }
    pub fn profile(&self) -> &'static str {
        model_profile_for(self.target, self.state.native_order)
    }
    /// Enumerates the immutable completed collection, excluding negative/staged
    /// identities. Metadata readiness is not a claim of source/spool IO checks.
    pub fn model_handle(&self, ordinal: usize) -> Result<ModelHandle, payload::CollectorError> {
        self.extraction.ready()?;
        if ordinal >= self.extraction.facts().models {
            return Err(Error::State.into());
        }
        let handle = ModelHandle(ordinal);
        if self.descriptor(handle)?.source().is_none() {
            return Err(Error::State.into());
        }
        Ok(handle)
    }
    pub fn descriptor(
        &self,
        handle: ModelHandle,
    ) -> Result<payload::Descriptor<'_>, payload::CollectorError> {
        self.extraction.ready()?;
        self.extraction.descriptor(handle)
    }
    pub fn with_payload<T>(
        &mut self,
        handle: ModelHandle,
        call: impl FnOnce(payload::CompatiblePayload<'_>) -> Result<T, payload::Error>,
    ) -> Result<T, payload::CollectorError> {
        self.extraction.with_compatible_payload(handle, call)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), payload::CollectorError> {
        self.extraction.verify_unchanged()
    }
    pub fn cached_identity_count(&self) -> usize {
        self.state.entries.len()
    }
}
