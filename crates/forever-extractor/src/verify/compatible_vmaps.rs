// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored by RustyCore Extractors, 2026-10-17.
//! Offline complete declared inventory/association checks for explicit v14.
//! The common package first verifies every digest. A held no-follow reader then
//! replays each genuine ordered witness through the unchanged pure assembler.
//! Available-source coverage is producer-reported, not a fresh client read.
use crate::{
    Error,
    collision_coverage::{self as cov, Coverage, Entry},
    driver::collision_geometry_v22::{self as geometry_v22, ArtifactFact},
    plan::{CollisionStorage, Plan},
};
use forever_casc::{
    integrity,
    package::{self, Artifact, Component, Context},
};
use forever_vmap::{assembly, gameobject_models, placement, vmo};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
const BAD: &str = "product_collision_association";
const LIMIT: &str = "product_collision_verify_limit";
const DIRECTORY: i32 = 0x10000;
const NOFOLLOW: i32 = 0x20000;
const NONBLOCK: i32 = 0x800;
fn nine(m: &Metadata) -> [i128; 11] {
    [
        m.dev() as i128,
        m.ino() as i128,
        m.mode() as i128,
        m.nlink() as i128,
        m.uid() as i128,
        m.gid() as i128,
        m.len() as i128,
        m.mtime() as i128,
        m.mtime_nsec() as i128,
        m.ctime() as i128,
        m.ctime_nsec() as i128,
    ]
}
fn anchor(f: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", f.as_raw_fd()))
}
fn directory(p: &Path) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .custom_flags(DIRECTORY | NOFOLLOW | NONBLOCK)
        .open(p)
        .map_err(|_| "product_input_changed".into())
}
/// Root is held throughout both integrity and semantic traversal. Every opened
/// component is no-follow; rereads require the declared manifest digest, full
/// regular-file identity stability and a still-identical root chain.
struct Held {
    file: File,
    path: PathBuf,
    identity: [i128; 6],
}
impl Held {
    fn open(p: &Path) -> Result<Self, Error> {
        if !p.is_absolute() || p.as_os_str().len() > 4096 {
            return Err(BAD.into());
        }
        for a in p.ancestors() {
            let m = std::fs::symlink_metadata(a).map_err(|_| BAD)?;
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err(BAD.into());
            }
        }
        let f = directory(p)?;
        let m = f.metadata().map_err(|_| BAD)?;
        Ok(Self {
            file: f,
            path: p.to_path_buf(),
            identity: [
                m.dev() as i128,
                m.ino() as i128,
                m.mode() as i128,
                m.nlink() as i128,
                m.uid() as i128,
                m.gid() as i128,
            ],
        })
    }
    fn guard(&self) -> Result<(), Error> {
        for p in self.path.ancestors() {
            let m = std::fs::symlink_metadata(p).map_err(|_| BAD)?;
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err(BAD.into());
            }
        }
        for m in [
            self.file.metadata().map_err(|_| BAD)?,
            std::fs::symlink_metadata(&self.path).map_err(|_| BAD)?,
        ] {
            if [
                m.dev() as i128,
                m.ino() as i128,
                m.mode() as i128,
                m.nlink() as i128,
                m.uid() as i128,
                m.gid() as i128,
            ] != self.identity
            {
                return Err("product_input_changed".into());
            }
        }
        Ok(())
    }
    fn read(&self, e: &Entry<'_>) -> Result<Vec<u8>, Error> {
        self.guard()?;
        let parts = e.path.split('/').collect::<Vec<_>>();
        if parts.len() > 16
            || parts
                .iter()
                .any(|p| p.is_empty() || *p == "." || *p == "..")
        {
            return Err(BAD.into());
        }
        let mut fd = self.file.try_clone().map_err(|_| BAD)?;
        for part in &parts[..parts.len() - 1] {
            fd = directory(&anchor(&fd).join(part))?;
        }
        let path = anchor(&fd).join(parts[parts.len() - 1]);
        let mut f = OpenOptions::new()
            .read(true)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(&path)
            .map_err(|_| BAD)?;
        let m = f.metadata().map_err(|_| BAD)?;
        if !m.is_file() || m.nlink() != 1 || m.len() != e.bytes || e.bytes > cov::MAX_BYTES as u64 {
            return Err(BAD.into());
        }
        let mut b = Vec::new();
        let n = usize::try_from(e.bytes).map_err(|_| LIMIT)?;
        b.try_reserve_exact(n).map_err(|_| "product_allocation")?;
        if b.capacity() > n {
            return Err(LIMIT.into());
        }
        b.resize(n, 0);
        f.read_exact(&mut b).map_err(|_| BAD)?;
        let mut extra = [0; 1];
        if f.read(&mut extra).map_err(|_| BAD)? != 0
            || nine(&m) != nine(&f.metadata().map_err(|_| BAD)?)
            || nine(&m) != nine(&std::fs::symlink_metadata(&path).map_err(|_| BAD)?)
            || integrity::digest(&b) != e.sha
        {
            return Err("product_input_changed".into());
        }
        self.guard()?;
        Ok(b)
    }
}
struct Facts<'a> {
    entries: Vec<Entry<'a>>,
    used: Vec<u8>,
}
impl<'a> Facts<'a> {
    fn new(c: &Coverage<'a>) -> Result<Self, Error> {
        let n = c.inventory().count();
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(n)
            .map_err(|_| "product_allocation")?;
        if entries.capacity() != n {
            return Err(LIMIT.into());
        }
        entries.extend(c.inventory());
        entries.sort_unstable_by_key(|e| e.path);
        if entries.windows(2).any(|p| p[0].path == p[1].path) {
            return Err(BAD.into());
        }
        let mut used = Vec::new();
        used.try_reserve_exact(n)
            .map_err(|_| "product_allocation")?;
        if used.capacity() > n {
            return Err(LIMIT.into());
        }
        used.resize(n, 0);
        Ok(Self { entries, used })
    }
    fn index(&self, path: &str) -> Result<usize, Error> {
        self.entries
            .binary_search_by_key(&path, |e| e.path)
            .map_err(|_| BAD.into())
    }
    fn mark(&mut self, path: &str, bytes: &[u8]) -> Result<(), Error> {
        let at = self.index(path)?;
        let e = &self.entries[at];
        if self.used[at] != 0 || e.bytes != bytes.len() as u64 || e.sha != integrity::digest(bytes)
        {
            return Err(BAD.into());
        }
        self.used[at] = 1;
        Ok(())
    }
    fn model(&mut self, name: &[u8]) -> Result<(), Error> {
        let name = std::str::from_utf8(name).map_err(|_| BAD)?;
        if name.len() > placement::MAX_NAME_BYTES || name.contains(['/', '\\']) || name.is_empty() {
            return Err(BAD.into());
        }
        let path = format!("vmaps/{name}.vmo");
        let at = self.index(&path)?;
        self.used[at] = 1;
        Ok(())
    }
}
fn tree_path(id: u32) -> String {
    format!("vmaps/{id:04}/{id:04}.vmtree")
}
fn tile_path(id: u32, x: u32, y: u32, index: bool) -> String {
    format!(
        "vmaps/{id:04}/{id:04}_{x:02}_{y:02}.{}",
        if index { "vmtileidx" } else { "vmtile" }
    )
}
fn intrinsic(path: &str, b: &[u8]) -> Result<(), Error> {
    if path.ends_with(".vmo")
        && path
            .strip_prefix("vmaps/")
            .is_some_and(|p| !p.contains('/'))
    {
        let mut limits = vmo::Limits::default();
        limits.max_memory_bytes = limits.max_memory_bytes.min(512 * 1024 * 1024);
        vmo::View::parse(b, limits).map_err(|e| Error::from(e.code()))?;
    } else if path == "vmaps/GameObjectModels.dtree" {
        gameobject_models::Dtree::parse(b, Default::default())
            .map_err(|e| Error::from(e.code()))?;
    }
    Ok(())
}
pub(super) fn verify(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    let held = Held::open(output)?;
    let mut limits = package::Limits::default();
    if let CollisionStorage::LeasedDiskV1(b) = plan.collision_storage {
        limits.total_bytes = b.output_bytes;
    } else {
        return Err(BAD.into());
    }
    // No extra cap: coverage128MiB, package metadata256MiB, all entry descriptors
    // <=~20MiB, proof64MiB, caller record Vec charged below, assembler <=1GiB and
    // one inherited payload64MiB stay within the original2GiB logical ceiling.
    let mut coverage = None;
    let mut failure = None;
    let summary = package::verify_with_preflight(
        output,
        expected,
        limits,
        |arts| {
            if arts.iter().filter(|a| a.path == cov::PATH).count() != 1
                || arts.iter().any(|a| a.component != Component::Collision)
            {
                return Err(package::Error::Inventory);
            }
            Ok(())
        },
        |a, f| {
            let result = (|| {
                let n = usize::try_from(a.bytes).map_err(|_| LIMIT)?;
                if n > cov::MAX_BYTES {
                    return Err(LIMIT.into());
                }
                let mut b = Vec::new();
                b.try_reserve_exact(n).map_err(|_| "product_allocation")?;
                if b.capacity() > n {
                    return Err(LIMIT.into());
                }
                b.resize(n, 0);
                f.read_exact(&mut b).map_err(|_| BAD)?;
                if a.path == cov::PATH {
                    coverage = Some(b);
                } else {
                    intrinsic(&a.path, &b)?;
                }
                Ok::<_, Error>(())
            })();
            result.map_err(|e| {
                failure = Some(e);
                package::Error::Schema
            })
        },
    )
    .map_err(|e| failure.unwrap_or_else(|| e.into()))?;
    let bytes = coverage.ok_or(BAD)?;
    let c = Coverage::parse(&bytes)?;
    if c.context != summary.context
        || c.selected != plan.map_domain
        || c.context.plan_sha256 != integrity::digest(plan.render().as_bytes())
        || c.context.build != plan.build
        || c.context.locale != plan.locale
    {
        return Err("product_plan_mismatch".into());
    }
    // The manifest is checked again after semantic replay. This also binds the
    // coverage-declared inventory to every authenticated package artifact.
    let mut facts = Facts::new(&c)?;
    let dtree_at = facts.index("vmaps/GameObjectModels.dtree")?;
    let dtree = held.read(&facts.entries[dtree_at])?;
    let mut startup = c.startup().filter(|r| r.disposition == 2);
    for row in gameobject_models::Dtree::parse(&dtree, Default::default())
        .map_err(|e| Error::from(e.code()))?
        .rows()
    {
        let expected = startup.next().ok_or(BAD)?;
        if row.display_id() != expected.display_id
            || row.name() != format!("FILE{:08X}.xxx", expected.fdid).as_bytes()
        {
            return Err(BAD.into());
        }
        facts.model(row.name())?;
    }
    if startup.next().is_some() {
        return Err(BAD.into());
    }
    facts.mark("vmaps/GameObjectModels.dtree", &dtree)?;
    drop(dtree);
    let mut ancestor_steps = 0usize;
    let mut proof_records = 0u64;
    for map in c.maps() {
        if !map.present {
            if facts.index(&tree_path(map.id)).is_ok()
                || facts.index(&cov::proof_path(map.id)).is_ok()
            {
                return Err(BAD.into());
            }
            continue;
        }
        let proof_path = cov::proof_path(map.id);
        let p = facts.index(&proof_path)?;
        let pb = held.read(&facts.entries[p])?;
        let proof = cov::proof(&pb, map.id)?;
        let parsed = placement::DirBin::parse(proof, placement::Limits::default())
            .map_err(|e| Error::from(e.code()))?;
        let count = parsed.record_count();
        proof_records = proof_records.checked_add(count as u64).ok_or(LIMIT)?;
        let has_records = map.global != 0 || map.cells.iter().any(|b| b & 3 == 3 || b >> 2 == 2);
        if (count != 0) != has_records || proof_records > c.records() {
            return Err(BAD.into());
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(count)
            .map_err(|_| "product_allocation")?;
        if records.capacity() > count {
            return Err(LIMIT.into());
        }
        let charge = records
            .capacity()
            .checked_mul(size_of::<placement::Placement<'_>>())
            .ok_or(LIMIT)?;
        let retained = bytes
            .capacity()
            .checked_add(c.resident_bytes())
            .and_then(|n| n.checked_add(facts.entries.capacity() * size_of::<Entry<'_>>()))
            .and_then(|n| n.checked_add(facts.used.capacity()))
            .and_then(|n| n.checked_add(limits.metadata_bytes))
            .and_then(|n| n.checked_add(pb.capacity()))
            .and_then(|n| n.checked_add(charge))
            .and_then(|n| n.checked_add(cov::MAX_BYTES + 16 * 1024 * 1024))
            .ok_or(LIMIT)?;
        let mut build_limits = assembly::Limits::default();
        build_limits.max_memory_bytes = build_limits.max_memory_bytes.min(
            (2usize * 1024 * 1024 * 1024)
                .checked_sub(retained)
                .ok_or(LIMIT)?,
        );
        for r in parsed.records() {
            facts.model(r.name())?;
            records.push(r);
        }
        let rebuilt =
            assembly::build(map.id, &records, build_limits).map_err(|e| Error::from(e.code()))?;
        if rebuilt.ordered_dir_bin() != proof {
            return Err(BAD.into());
        }
        facts.mark(&proof_path, &pb)?;
        facts.mark(&tree_path(map.id), rebuilt.tree_bytes())?;
        for t in rebuilt.tiles() {
            let index_path = tile_path(map.id, t.x(), t.y(), true);
            facts.mark(&index_path, t.index_bytes())?;
            if let Some(payload) = t.payload() {
                facts.mark(&tile_path(map.id, t.x(), t.y(), false), payload)?;
            } else {
                let mut ancestor = map.parent;
                let mut actual = None;
                let mut steps = 0;
                while let Some(id) = ancestor {
                    steps += 1;
                    ancestor_steps = ancestor_steps.checked_add(1).ok_or(LIMIT)?;
                    if steps > cov::MAX_MAPS || ancestor_steps > 256_000_000 {
                        return Err(BAD.into());
                    }
                    let path = tile_path(id, t.x(), t.y(), false);
                    if let Ok(at) = facts.index(&path) {
                        actual = Some(held.read(&facts.entries[at])?);
                        break;
                    }
                    // Input ADT cells are not spatial output tile coordinates. The complete
                    // canonical child witness determines parent-only output; resolve the first
                    // actual ancestor payload exactly as the genuine package owner does.
                    ancestor = c.parent(id)?;
                }
                assembly::verify_inherited_associations(
                    actual.as_deref().ok_or(BAD)?,
                    t.index_bytes(),
                    proof,
                    build_limits,
                )
                .map_err(|e| Error::from(e.code()))?;
            }
        }
    }
    if facts.used.contains(&0) {
        return Err("product_collision_inventory".into());
    }
    let mut metadata_error = None;
    let last = package::verify_with_preflight(
        output,
        Some(c.context),
        limits,
        |arts: &[Artifact]| {
            c.matches(arts).map_err(|e| {
                metadata_error = Some(e);
                package::Error::Schema
            })
        },
        |_, _| Ok(()),
    )
    .map_err(|e| metadata_error.unwrap_or_else(|| e.into()))?;
    held.guard()?;
    if last != summary {
        return Err("product_input_changed".into());
    }
    Ok(last)
}

/// Build-70450 selected geometry has its own coverage identity and domain
/// verifier. This intentionally does not route through the v15/v18 six-owner
/// coverage schema or alter those verifiers.
pub(super) fn verify_selected_geometry_v22(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    let held = Held::open(output)?;
    let mut limits = package::Limits::default();
    let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
        return Err(BAD.into());
    };
    limits.total_bytes = budget.output_bytes;

    let mut package_artifacts: Option<Vec<Artifact>> = None;
    let mut coverage_bytes = None;
    let mut failure = None;
    let summary = package::verify_with_preflight(
        output,
        expected,
        limits,
        |artifacts| {
            if artifacts.len() > geometry_v22::COVERAGE_MAX_ARTIFACTS + 1
                || artifacts.iter().any(|artifact| {
                    artifact.component != Component::Collision
                        || !artifact.path.starts_with("vmaps/")
                })
                || artifacts
                    .iter()
                    .filter(|artifact| artifact.path == geometry_v22::COVERAGE_PATH)
                    .count()
                    != 1
            {
                return Err(package::Error::Inventory);
            }
            let mut retained = Vec::new();
            retained
                .try_reserve_exact(artifacts.len())
                .map_err(|_| package::Error::Limit)?;
            retained.extend(artifacts.iter().cloned());
            package_artifacts = Some(retained);
            Ok(())
        },
        |artifact, file| {
            if artifact.path != geometry_v22::COVERAGE_PATH {
                return Ok(());
            }
            let result = (|| {
                let length = usize::try_from(artifact.bytes).map_err(|_| LIMIT)?;
                if length > cov::MAX_BYTES {
                    return Err(LIMIT.into());
                }
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(length)
                    .map_err(|_| Error::from("product_allocation"))?;
                if bytes.capacity() > cov::MAX_BYTES {
                    return Err(LIMIT.into());
                }
                bytes.resize(length, 0);
                file.read_exact(&mut bytes).map_err(|_| BAD)?;
                coverage_bytes = Some(bytes);
                Ok::<_, Error>(())
            })();
            result.map_err(|error| {
                failure = Some(error);
                package::Error::Schema
            })
        },
    )
    .map_err(|error| failure.unwrap_or_else(|| error.into()))?;
    let bytes = coverage_bytes.ok_or(BAD)?;
    let coverage = geometry_v22::Coverage::parse(&bytes)?;
    if coverage.context != summary.context
        || coverage.context.build != 70450
        || coverage.context.locale != plan.locale
        || coverage.context.components != Component::Collision.bit()
        || coverage.context.plan_sha256 != integrity::digest(plan.render().as_bytes())
        || coverage.requested_map != plan.map_domain.ok_or("product_plan_map_domain")?
        || coverage.map.file_data_id != forever_casc::collision::world::MAP_FILE_DATA_ID
    {
        return Err("product_plan_mismatch".into());
    }
    verify_geometry_domain(&coverage)?;
    let mut facts = GeometryInventory::new(&coverage)?;
    compare_geometry_inventory(package_artifacts.as_deref().ok_or(BAD)?, &coverage)?;

    let mut model_names = std::collections::BTreeSet::new();
    for fact in &coverage.artifacts {
        if fact.path == geometry_v22::COVERAGE_PATH {
            return Err(BAD.into());
        }
        if let Some(name) = vmo_name(&fact.path)? {
            let bytes = held.read(&entry(fact))?;
            vmo::View::parse(&bytes, Default::default())
                .map_err(|error| Error::from(error.code()))?;
            facts.mark(fact)?;
            model_names.insert(name.to_owned());
        }
    }

    let mut used_models = std::collections::BTreeSet::new();
    let mut maps = std::collections::BTreeMap::new();
    for map in &coverage.maps {
        maps.insert(map.id, *map);
    }
    let mut ancestor_steps = 0usize;
    let mut map_total = 0_u64;
    let map_total_limit = budget
        .map_bytes
        .min(geometry_v22::MAX_ASSEMBLED_DOMAIN_BYTES);
    for map in &coverage.maps {
        let (tree_path, proof_path) = geometry_paths(map.id);
        let proof_fact = facts.fact(&proof_path)?;
        map_total = map_total.checked_add(proof_fact.bytes).ok_or(LIMIT)?;
        let proof_bytes = held.read(&entry(proof_fact))?;
        let placements = placement::DirBin::parse(&proof_bytes, placement::Limits::default())
            .map_err(|error| Error::from(error.code()))?;
        if placements.record_count() > geometry_v22::MAX_PLACEMENT_RECORDS {
            return Err(LIMIT.into());
        }
        let mut records = Vec::new();
        records
            .try_reserve_exact(placements.record_count())
            .map_err(|_| Error::from("product_allocation"))?;
        for record in placements.records() {
            let name = str::from_utf8(record.name()).map_err(|_| BAD)?;
            if !model_names.contains(name) {
                return Err("product_selected_geometry_model_binding".into());
            }
            used_models.insert(name.to_owned());
            records.push(record);
        }
        let mut assembly_limits = assembly::Limits::default();
        assembly_limits.max_memory_bytes = assembly_limits
            .max_memory_bytes
            .min(geometry_v22::MAX_ASSEMBLY_MEMORY_BYTES);
        assembly_limits.max_input_bytes = assembly_limits.max_input_bytes.min(placement::MAX_BYTES);
        assembly_limits.max_spawns = assembly_limits
            .max_spawns
            .min(geometry_v22::MAX_PLACEMENT_RECORDS);
        assembly_limits.max_output_bytes = assembly_limits
            .max_output_bytes
            .min(geometry_v22::MAX_ASSEMBLY_OUTPUT_BYTES)
            .min(as_usize(budget.map_bytes)?);
        let assembled = assembly::build(map.id, &records, assembly_limits)
            .map_err(|error| Error::from(error.code()))?;
        if assembled.ordered_dir_bin() != proof_bytes {
            return Err(BAD.into());
        }
        let tree_fact = facts.fact(&tree_path)?;
        map_total = map_total.checked_add(tree_fact.bytes).ok_or(LIMIT)?;
        let tree_bytes = held.read(&entry(tree_fact))?;
        if tree_bytes != assembled.tree_bytes() {
            return Err(BAD.into());
        }
        facts.mark(proof_fact)?;
        facts.mark(tree_fact)?;

        for tile in assembled.tiles() {
            let index_path = geometry_tile_path(map.id, tile.x(), tile.y(), true);
            let index_fact = facts.fact(&index_path)?;
            map_total = map_total.checked_add(index_fact.bytes).ok_or(LIMIT)?;
            let index_bytes = held.read(&entry(index_fact))?;
            if index_bytes != tile.index_bytes() {
                return Err(BAD.into());
            }
            facts.mark(index_fact)?;
            if let Some(payload) = tile.payload() {
                let payload_path = geometry_tile_path(map.id, tile.x(), tile.y(), false);
                let payload_fact = facts.fact(&payload_path)?;
                map_total = map_total.checked_add(payload_fact.bytes).ok_or(LIMIT)?;
                let payload_bytes = held.read(&entry(payload_fact))?;
                if payload_bytes != payload {
                    return Err(BAD.into());
                }
                assembly::verify_associations(
                    &payload_bytes,
                    &index_bytes,
                    proof_bytes.as_slice(),
                    assembly_limits,
                )
                .map_err(|error| Error::from(error.code()))?;
                facts.mark(payload_fact)?;
            } else {
                let payload_path = geometry_tile_path(map.id, tile.x(), tile.y(), false);
                if facts.contains(&payload_path) {
                    return Err(BAD.into());
                }
                let mut parent = map.parent;
                let mut ancestor_payload = None;
                while let Some(parent_id) = parent {
                    ancestor_steps = ancestor_steps.checked_add(1).ok_or(LIMIT)?;
                    if ancestor_steps
                        > geometry_v22::COVERAGE_MAX_MAPS
                            .saturating_mul(geometry_v22::COVERAGE_MAX_MAPS)
                    {
                        return Err(LIMIT.into());
                    }
                    let parent_path = geometry_tile_path(parent_id, tile.x(), tile.y(), false);
                    if let Some(parent_fact) = facts.optional_fact(&parent_path) {
                        ancestor_payload = Some(held.read(&entry(parent_fact))?);
                        break;
                    }
                    parent = maps.get(&parent_id).ok_or(BAD)?.parent;
                }
                assembly::verify_inherited_associations(
                    ancestor_payload.as_deref().ok_or(BAD)?,
                    &index_bytes,
                    proof_bytes.as_slice(),
                    assembly_limits,
                )
                .map_err(|error| Error::from(error.code()))?;
            }
        }
    }
    if map_total > map_total_limit || used_models != model_names || !facts.all_used() {
        return Err("product_selected_geometry_inventory".into());
    }

    let mut metadata_error = None;
    let last = package::verify_with_preflight(
        output,
        Some(coverage.context),
        limits,
        |artifacts| {
            compare_geometry_inventory(artifacts, &coverage).map_err(|error| {
                metadata_error = Some(error);
                package::Error::Schema
            })
        },
        |_, _| Ok(()),
    )
    .map_err(|error| metadata_error.unwrap_or_else(|| error.into()))?;
    held.guard()?;
    if last != summary {
        return Err("product_input_changed".into());
    }
    Ok(last)
}

struct GeometryInventory<'a> {
    entries: std::collections::BTreeMap<&'a str, &'a ArtifactFact>,
    used: std::collections::BTreeSet<&'a str>,
}
impl<'a> GeometryInventory<'a> {
    fn new(coverage: &'a geometry_v22::Coverage) -> Result<Self, Error> {
        let mut entries = std::collections::BTreeMap::new();
        for fact in &coverage.artifacts {
            if !valid_geometry_path(&fact.path)
                || entries.insert(fact.path.as_str(), fact).is_some()
            {
                return Err(BAD.into());
            }
        }
        Ok(Self {
            entries,
            used: std::collections::BTreeSet::new(),
        })
    }
    fn fact(&self, path: &str) -> Result<&'a ArtifactFact, Error> {
        self.entries.get(path).copied().ok_or_else(|| BAD.into())
    }
    fn optional_fact(&self, path: &str) -> Option<&'a ArtifactFact> {
        self.entries.get(path).copied()
    }
    fn contains(&self, path: &str) -> bool {
        self.entries.contains_key(path)
    }
    fn mark(&mut self, fact: &'a ArtifactFact) -> Result<(), Error> {
        if !self.used.insert(fact.path.as_str()) {
            return Err(BAD.into());
        }
        Ok(())
    }
    fn all_used(&self) -> bool {
        self.used.len() == self.entries.len()
    }
}

fn entry(fact: &ArtifactFact) -> Entry<'_> {
    Entry {
        path: &fact.path,
        bytes: fact.bytes,
        sha: fact.sha,
    }
}
fn as_usize(value: u64) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::from(LIMIT))
}
fn compare_geometry_inventory(
    actual: &[Artifact],
    coverage: &geometry_v22::Coverage,
) -> Result<(), Error> {
    if actual.len() != coverage.artifacts.len() + 1 {
        return Err(BAD.into());
    }
    let mut expected: Vec<_> = coverage.artifacts.iter().collect();
    expected.sort_unstable_by_key(|fact| fact.path.as_str());
    let mut seen_coverage = false;
    let mut got: Vec<_> = actual.iter().collect();
    got.sort_unstable_by_key(|artifact| artifact.path.as_str());
    let mut at = 0;
    for artifact in got {
        if artifact.component != Component::Collision || !artifact.path.starts_with("vmaps/") {
            return Err(BAD.into());
        }
        if artifact.path == geometry_v22::COVERAGE_PATH {
            if seen_coverage {
                return Err(BAD.into());
            }
            seen_coverage = true;
            continue;
        }
        let fact = expected.get(at).ok_or(BAD)?;
        if artifact.path != fact.path || artifact.bytes != fact.bytes || artifact.sha256 != fact.sha
        {
            return Err(BAD.into());
        }
        at += 1;
    }
    if !seen_coverage || at != expected.len() {
        return Err(BAD.into());
    }
    Ok(())
}

fn verify_geometry_domain(coverage: &geometry_v22::Coverage) -> Result<(), Error> {
    let mut by_id = std::collections::BTreeMap::new();
    for map in &coverage.maps {
        if by_id.insert(map.id, map).is_some() || map.wdt_file_data_id == 0 {
            return Err(BAD.into());
        }
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut current = Some(coverage.requested_map);
    while let Some(id) = current {
        if !visited.insert(id) || visited.len() > coverage.maps.len() {
            return Err(BAD.into());
        }
        let map = by_id.get(&id).ok_or(BAD)?;
        current = map.parent;
        if let Some(parent) = current {
            let parent_map = by_id.get(&parent).ok_or(BAD)?;
            if parent_map.depth.checked_add(1) != Some(map.depth) {
                return Err(BAD.into());
            }
        } else if map.depth != 0 {
            return Err(BAD.into());
        }
    }
    if visited.len() != coverage.maps.len() {
        return Err(BAD.into());
    }
    for map in &coverage.maps {
        let has_child = coverage
            .maps
            .iter()
            .any(|child| child.parent == Some(map.id));
        if map.is_parent != has_child {
            return Err(BAD.into());
        }
    }
    Ok(())
}

fn valid_geometry_path(path: &str) -> bool {
    if path == geometry_v22::COVERAGE_PATH {
        return false;
    }
    match vmo_name(path) {
        Ok(Some(name)) => return !name.is_empty() && name.len() <= placement::MAX_NAME_BYTES,
        Err(_) => return false,
        Ok(None) => {}
    }
    if let Some(rest) = path.strip_prefix("vmaps/coverage/") {
        return rest.ends_with(".placements") && rest.len() <= 32;
    }
    let Some(rest) = path.strip_prefix("vmaps/") else {
        return false;
    };
    let Some((map, file)) = rest.split_once('/') else {
        return false;
    };
    let Ok(id) = map.parse::<u32>() else {
        return false;
    };
    map == format!("{id:04}")
        && [".vmtree", ".vmtile", ".vmtileidx"]
            .iter()
            .any(|extension| file.ends_with(extension))
}
fn vmo_name(path: &str) -> Result<Option<&str>, Error> {
    let Some(name) = path
        .strip_prefix("vmaps/")
        .and_then(|value| value.strip_suffix(".vmo"))
    else {
        return Ok(None);
    };
    if name.is_empty() || name.contains('/') || name.contains('\\') || !name.is_ascii() {
        return Err(BAD.into());
    }
    Ok(Some(name))
}
fn geometry_paths(id: u32) -> (String, String) {
    (
        format!("vmaps/{id:04}/{id:04}.vmtree"),
        format!("vmaps/coverage/{id:04}.placements"),
    )
}
fn geometry_tile_path(map: u32, x: u32, y: u32, index: bool) -> String {
    format!(
        "vmaps/{map:04}/{map:04}_{x:02}_{y:02}.{}",
        if index { "vmtileidx" } else { "vmtile" }
    )
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod selected_geometry_v22_tests {
    use super::*;
    use forever_casc::Locale;
    use std::{
        fs,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
    };

    const V22: &str =
        include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Lab(PathBuf);
    impl Lab {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rce-v22-verify-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn empty_geometry_fixture() -> (Plan, geometry_v22::Coverage, Vec<(String, Vec<u8>)>) {
        let plan = Plan::parse(V22).unwrap();
        let context = Context {
            build: 70450,
            locale: Locale::EnUs,
            components: Component::Collision.bit(),
            source_sha256: [0x41; 32],
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            producer_sha256: [0x42; 32],
        };
        let assembled = assembly::build(2, &[], Default::default()).unwrap();
        let (tree_path, proof_path) = geometry_paths(2);
        let files = vec![
            (proof_path, assembled.ordered_dir_bin().to_vec()),
            (tree_path, assembled.tree_bytes().to_vec()),
        ];
        let coverage = geometry_v22::Coverage {
            context,
            requested_map: 2,
            gameobject_scope: forever_casc::collision::geometry::GameObjectScope::Excluded,
            gameobject_file_data_id: forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID,
            gameobject_block_reason:
                forever_casc::collision::geometry::GameObjectBlockReason::ExcludedByProfile,
            gameobject_request_made: false,
            gameobject_startup_reads: 0,
            model_scope: geometry_v22::ModelScope::PlacementOnly,
            liquid_object_scope: forever_casc::collision::geometry::LiquidObjectScope::OutOfScope,
            liquid_object_request_made: false,
            map: geometry_v22::AvailableMapFact {
                file_data_id: forever_casc::collision::world::MAP_FILE_DATA_ID,
                declared_bytes: 204,
                available_bytes: 204,
                full_ckey_verified: true,
                encoded_key_authenticated: true,
                declared_direct_rows: 0,
                declared_copy_records: 0,
                direct_rows: 0,
                copy_records: 0,
                materialized_rows: 0,
                sections: vec![geometry_v22::SectionFact {
                    ordinal: 0,
                    records: 0,
                    copies: 0,
                    admitted: true,
                }],
            },
            liquids: [
                geometry_v22::LiquidFact {
                    file_data_id: forever_casc::collision::world::MATERIAL_FILE_DATA_ID,
                    bytes: 4,
                    magic: *b"MATL",
                    direct_rows: 0,
                    copy_records: 0,
                    materialized_rows: 0,
                },
                geometry_v22::LiquidFact {
                    file_data_id: forever_casc::collision::world::TYPE_FILE_DATA_ID,
                    bytes: 4,
                    magic: *b"TYPE",
                    direct_rows: 0,
                    copy_records: 0,
                    materialized_rows: 0,
                },
            ],
            maps: vec![geometry_v22::DomainMapFact {
                id: 2,
                parent: None,
                wdt_file_data_id: 100,
                depth: 0,
                is_parent: false,
            }],
            artifacts: Vec::new(),
        };
        (plan, coverage, files)
    }

    fn artifact_facts(files: &[(String, Vec<u8>)]) -> Vec<ArtifactFact> {
        let mut facts: Vec<_> = files
            .iter()
            .map(|(path, bytes)| ArtifactFact {
                path: path.clone(),
                bytes: bytes.len() as u64,
                sha: integrity::digest(bytes),
            })
            .collect();
        facts.sort_unstable_by(|left, right| left.path.cmp(&right.path));
        facts
    }

    fn publish_fixture(
        lab: &Lab,
        name: &str,
        coverage: &geometry_v22::Coverage,
        files: &[(String, Vec<u8>)],
    ) -> PathBuf {
        let output = lab.0.join(name);
        let mut run =
            package::Run::create(&output, coverage.context, &[], Default::default()).unwrap();
        for (path, bytes) in files {
            run.put(Component::Collision, path, bytes).unwrap();
        }
        run.put(
            Component::Collision,
            geometry_v22::COVERAGE_PATH,
            &coverage.encode().unwrap(),
        )
        .unwrap();
        run.finish_component(Component::Collision).unwrap();
        run.finish().unwrap();
        output
    }

    fn verify_rejection(plan: &Plan, output: &Path, expected_code: &str) {
        let error = match verify_selected_geometry_v22(plan, output, None) {
            Ok(_) => panic!("mutated selected-geometry package unexpectedly passed"),
            Err(error) => error,
        };
        assert_eq!(error.code, expected_code);
    }

    #[test]
    fn v22_empty_selected_geometry_package_verifies_without_source_io() {
        let lab = Lab::new();
        let (plan, mut coverage, files) = empty_geometry_fixture();
        coverage.artifacts = artifact_facts(&files);
        let output = publish_fixture(&lab, "package", &coverage, &files);

        let verified = verify_selected_geometry_v22(&plan, &output, None).unwrap();
        assert_eq!(verified.context, coverage.context);
        assert_eq!(verified.files, 3);
    }

    #[test]
    fn v22_rejects_an_extra_vmo_not_referenced_by_any_placement_proof() {
        let lab = Lab::new();
        let (plan, mut coverage, mut files) = empty_geometry_fixture();
        let mut vmo = b"VMAP_4.EWMOD".to_vec();
        vmo.extend_from_slice(&8_u32.to_le_bytes());
        vmo.extend_from_slice(&0_u32.to_le_bytes());
        vmo.extend_from_slice(&0_u32.to_le_bytes());
        assert!(vmo::View::parse(&vmo, Default::default()).is_ok());
        files.push(("vmaps/unreferenced.vmo".into(), vmo));
        coverage.artifacts = artifact_facts(&files);
        let output = publish_fixture(&lab, "extra-vmo", &coverage, &files);
        verify_rejection(&plan, &output, "product_selected_geometry_inventory");
    }

    #[test]
    fn v22_rejects_a_placement_proof_changed_without_matching_coverage() {
        let lab = Lab::new();
        let (plan, mut coverage, mut files) = empty_geometry_fixture();
        coverage.artifacts = artifact_facts(&files);
        files
            .iter_mut()
            .find(|(path, _)| path.ends_with(".placements"))
            .unwrap()
            .1
            .push(0);
        let output = publish_fixture(&lab, "changed-proof", &coverage, &files);
        verify_rejection(&plan, &output, BAD);
    }

    #[test]
    fn v22_rejects_an_out_of_range_unadmitted_section_in_coverage() {
        let lab = Lab::new();
        let (plan, mut coverage, files) = empty_geometry_fixture();
        coverage.artifacts = artifact_facts(&files);
        coverage.map.sections.push(geometry_v22::SectionFact {
            ordinal: 999,
            records: 0,
            copies: 0,
            admitted: false,
        });
        let output = publish_fixture(&lab, "unadmitted-section", &coverage, &files);
        verify_rejection(&plan, &output, "product_selected_geometry_coverage_schema");
    }

    #[test]
    fn v22_rejects_a_false_parent_fact() {
        let lab = Lab::new();
        let (plan, mut coverage, files) = empty_geometry_fixture();
        coverage.artifacts = artifact_facts(&files);
        coverage.maps[0].is_parent = true;
        let output = publish_fixture(&lab, "false-parent", &coverage, &files);
        verify_rejection(&plan, &output, BAD);
    }

    #[test]
    fn v22_treats_well_formed_unadmitted_map_sections_as_producer_reported() {
        let lab = Lab::new();
        let (plan, mut coverage, files) = empty_geometry_fixture();
        coverage.artifacts = artifact_facts(&files);
        coverage.map.sections.push(geometry_v22::SectionFact {
            ordinal: 1,
            records: 0,
            copies: 0,
            admitted: false,
        });
        let output = publish_fixture(&lab, "reported-omission", &coverage, &files);
        assert!(verify_selected_geometry_v22(&plan, &output, None).is_ok());
    }
}
