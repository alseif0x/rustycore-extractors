// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored by RustyCore Extractors, 2026-10-17.
//! Offline complete declared inventory/association checks for explicit v14.
//! The common package first verifies every digest. A held no-follow reader then
//! replays each genuine ordered witness through the unchanged pure assembler.
//! v22 available-source coverage is producer-reported; v28-v41 re-derive
//! selected/all-available Map/VMap coverage from the sealed artifact without a
//! fresh client read. Source key flags remain acquisition facts, not offline
//! cryptographic authentication. v30 ROOT absence, v31 WMO-doodad M2, v33
//! vacated-WMO, v34 intrinsically empty-WMO, v35 unselected-WMO and v36-v41
//! material-filtered-WMO declarations are producer-reported: offline replay cannot
//! independently prove raw ROOT absence, original dependency requests or
//! conversion-time geometry absence/selection/material filtering. Vacating a parent
//! WMO does not justify excluding it when its actual Map placement is retained.
//! v32-v36 use RequestHeavyV1, v37 DependencyHeavyV1, v38 SourceHeavyV1, v39 ReadHeavyV1,
//! v40 ModelHeavyV1 and v41 SourceExpandedV1.
use crate::{
    Error,
    collision_coverage::{self as cov, Coverage, Entry},
    driver::{
        CompleteDb2Source,
        collision_geometry_v22::{self as geometry_v22, ArtifactFact},
    },
    plan::{CollisionStorage, Plan},
};
use forever_casc::{
    collision::plan::{MapSelection, Maps},
    integrity,
    package::{self, Artifact, Component, Context},
};
use forever_vmap::{assembly, gameobject_models, placement, vmo};
use std::{
    collections::BTreeMap,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedGeometryProfile {
    V22,
    V28,
    V29,
    V30,
    V31,
    V32,
    V33,
    V34,
    V35,
    V36,
    V37,
    V38,
    V39,
    V40,
    V41,
    V42,
    V43,
    V44,
    V45,
    V46,
    V47,
    V48,
    V49,
    V50,
}
impl SelectedGeometryProfile {
    const fn components(self) -> u8 {
        match self {
            Self::V22 => Component::Collision.bit(),
            Self::V28
            | Self::V29
            | Self::V30
            | Self::V31
            | Self::V32
            | Self::V33
            | Self::V34
            | Self::V35
            | Self::V36
            | Self::V37
            | Self::V38
            | Self::V39
            | Self::V40
            | Self::V41
            | Self::V42
            | Self::V43
            | Self::V44
            | Self::V45
            | Self::V46
            | Self::V47
            | Self::V48
            | Self::V49
            | Self::V50 => Component::Db2.bit() | Component::Collision.bit(),
        }
    }
    const fn has_authenticated_map(self) -> bool {
        matches!(
            self,
            Self::V28
                | Self::V29
                | Self::V30
                | Self::V31
                | Self::V32
                | Self::V33
                | Self::V34
                | Self::V35
                | Self::V36
                | Self::V37
                | Self::V38
                | Self::V39
                | Self::V40
                | Self::V41
                | Self::V42
                | Self::V43
                | Self::V44
                | Self::V45
                | Self::V46
                | Self::V47
                | Self::V48
                | Self::V49
                | Self::V50
        )
    }
    const fn has_authenticated_gameobjects(self) -> bool {
        matches!(self, Self::V47 | Self::V48 | Self::V49 | Self::V50)
    }
    const fn coverage_path(self) -> &'static str {
        match self {
            Self::V22 | Self::V28 => geometry_v22::COVERAGE_PATH,
            Self::V29 => geometry_v22::AVAILABLE_COVERAGE_PATH,
            Self::V30 => geometry_v22::ROOT_ABSENT_COVERAGE_PATH,
            Self::V31 => geometry_v22::MODEL_ROOT_ABSENT_COVERAGE_PATH,
            Self::V32 => geometry_v22::REQUEST_CAPACITY_COVERAGE_PATH,
            Self::V33 => geometry_v22::VACATED_WMO_COVERAGE_PATH,
            Self::V34 => geometry_v22::EMPTY_WMO_COVERAGE_PATH,
            Self::V35 => geometry_v22::UNSELECTED_WMO_COVERAGE_PATH,
            Self::V36 => geometry_v22::MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V37 => geometry_v22::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V38 => geometry_v22::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V39 => geometry_v22::READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V40 => geometry_v22::MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V41 => geometry_v22::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V42 => geometry_v22::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V43 => geometry_v22::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V44 => geometry_v22::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V45 => geometry_v22::PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V46 => geometry_v22::ASSEMBLY_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V47 => geometry_v22::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V48 => geometry_v22::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
            Self::V49 | Self::V50 => {
                geometry_v22::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PATH
            }
        }
    }
}

fn assembled_domain_limit(profile: SelectedGeometryProfile, plan_limit: u64) -> u64 {
    let profile_limit = if matches!(
        profile,
        SelectedGeometryProfile::V46
            | SelectedGeometryProfile::V47
            | SelectedGeometryProfile::V48
            | SelectedGeometryProfile::V49
            | SelectedGeometryProfile::V50
    ) {
        geometry_v22::MAX_ASSEMBLED_DOMAIN_BYTES_V46
    } else {
        geometry_v22::MAX_ASSEMBLED_DOMAIN_BYTES
    };
    plan_limit.min(profile_limit)
}

#[cfg(test)]
mod assembled_domain_limit_tests {
    use super::{SelectedGeometryProfile, assembled_domain_limit, geometry_v22};

    #[test]
    fn v45_v46_v47_and_plan_budget_keep_their_assembled_domain_caps() {
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V45, u64::MAX),
            128 * 1024 * 1024
        );
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V46, u64::MAX),
            256 * 1024 * 1024
        );
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V47, u64::MAX),
            256 * 1024 * 1024
        );
        let reduced_plan_budget = 96 * 1024 * 1024;
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V45, reduced_plan_budget),
            reduced_plan_budget
        );
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V46, reduced_plan_budget),
            reduced_plan_budget
        );
        assert_eq!(
            assembled_domain_limit(SelectedGeometryProfile::V47, reduced_plan_budget),
            reduced_plan_budget
        );
        assert_eq!(geometry_v22::MAX_ASSEMBLED_DOMAIN_BYTES, 128 * 1024 * 1024);
        assert_eq!(
            geometry_v22::MAX_ASSEMBLED_DOMAIN_BYTES_V46,
            256 * 1024 * 1024
        );
    }
}

struct V28MapAuthentication {
    declared_bytes: u64,
    direct_rows: usize,
    copy_records: usize,
    tact_payload_sizes: Vec<u64>,
    owner: forever_db2::availability::MaterializedVMapMaps,
}

fn authenticate_v28_map(bytes: &[u8]) -> Result<V28MapAuthentication, Error> {
    let structural =
        forever_db2::availability::read_structure_70450_for_table(&CompleteDb2Source(bytes), "Map")
            .map_err(|error| Error::from(error.code()))?;
    let available = forever_db2::availability::read_vmap_maps_70450(&CompleteDb2Source(bytes))
        .map_err(|error| Error::from(error.code()))?;
    let direct_rows = available.direct_records().len();
    let copy_records = available.copies().len();
    let owner = available
        .materialize()
        .map_err(|error| Error::from(error.code()))?;
    Ok(V28MapAuthentication {
        declared_bytes: bytes.len() as u64,
        direct_rows,
        copy_records,
        tact_payload_sizes: structural.tact_payload_sizes,
        owner,
    })
}

struct V47GameObjectAuthentication {
    declared_bytes: u64,
    direct_rows: usize,
    copy_records: usize,
    structural_sections: Vec<forever_db2::availability::SectionCoverage>,
    tact_payload_sizes: Vec<u64>,
    rows: Vec<(u32, u32)>,
}

fn authenticate_v47_gameobjects(bytes: &[u8]) -> Result<V47GameObjectAuthentication, Error> {
    let structural = forever_db2::availability::read_structure_70450_for_table(
        &CompleteDb2Source(bytes),
        "GameObjectDisplayInfo",
    )
    .map_err(|error| Error::from(error.code()))?;
    let available =
        forever_db2::availability::read_gameobject_display_info_70450(&CompleteDb2Source(bytes))
            .map_err(|error| Error::from(error.code()))?;
    let direct_rows = available.direct_records().len();
    let copy_records = available.copies().len();
    let mut rows = Vec::new();
    rows.try_reserve_exact(direct_rows)
        .map_err(|_| Error::from("product_allocation"))?;
    rows.extend(
        available
            .direct_records()
            .iter()
            .map(|row| (row.id, row.fields.file_data_id)),
    );
    Ok(V47GameObjectAuthentication {
        declared_bytes: bytes.len() as u64,
        direct_rows,
        copy_records,
        structural_sections: structural.sections,
        tact_payload_sizes: structural.tact_payload_sizes,
        rows,
    })
}

fn collision_artifacts(artifacts: &[Artifact]) -> Result<Vec<Artifact>, package::Error> {
    let count = artifacts
        .iter()
        .filter(|artifact| artifact.component == Component::Collision)
        .count();
    let mut retained = Vec::new();
    retained
        .try_reserve_exact(count)
        .map_err(|_| package::Error::Limit)?;
    retained.extend(
        artifacts
            .iter()
            .filter(|artifact| artifact.component == Component::Collision)
            .cloned(),
    );
    Ok(retained)
}

fn geometry_artifacts(
    artifacts: &[Artifact],
    evidence_path: Option<&str>,
) -> Result<Vec<Artifact>, package::Error> {
    let count = artifacts
        .iter()
        .filter(|artifact| {
            artifact.component == Component::Collision
                && Some(artifact.path.as_str()) != evidence_path
        })
        .count();
    let mut retained = Vec::new();
    retained
        .try_reserve_exact(count)
        .map_err(|_| package::Error::Limit)?;
    retained.extend(
        artifacts
            .iter()
            .filter(|artifact| {
                artifact.component == Component::Collision
                    && Some(artifact.path.as_str()) != evidence_path
            })
            .cloned(),
    );
    Ok(retained)
}

/// Build-70450 selected geometry has its own coverage identity and domain
/// verifier. This intentionally does not route through the v15/v18 six-owner
/// coverage schema or alter those verifiers.
pub(super) fn verify_selected_geometry_v22(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v22() || plan.components != Component::Collision.bit() {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V22)
}

pub(super) fn verify_selected_geometry_v28(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v28() || plan.components != Component::Db2.bit() | Component::Collision.bit() {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V28)
}

pub(super) fn verify_selected_geometry_v29(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v29() || plan.components != Component::Db2.bit() | Component::Collision.bit() {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V29)
}

pub(super) fn verify_selected_geometry_v30(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v30()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V30)
}

pub(super) fn verify_selected_geometry_v31(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v31()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V31)
}

pub(super) fn verify_selected_geometry_v32(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v32()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V32)
}

pub(super) fn verify_selected_geometry_v33(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v33()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V33)
}

pub(super) fn verify_selected_geometry_v34(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v34()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V34)
}

pub(super) fn verify_selected_geometry_v35(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v35()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V35)
}

pub(super) fn verify_selected_geometry_v36(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v36()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V36)
}

pub(super) fn verify_selected_geometry_v37(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v37()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V37)
}

pub(super) fn verify_selected_geometry_v38(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v38()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V38)
}

pub(super) fn verify_selected_geometry_v39(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v39()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V39)
}

pub(super) fn verify_selected_geometry_v40(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v40()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V40)
}

pub(super) fn verify_selected_geometry_v41(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v41()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V41)
}

pub(super) fn verify_selected_geometry_v42(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v42()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V42)
}

pub(super) fn verify_selected_geometry_v43(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v43()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V43)
}

pub(super) fn verify_selected_geometry_v44(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v44()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V44)
}

pub(super) fn verify_selected_geometry_v45(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v45()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V45)
}

pub(super) fn verify_selected_geometry_v46(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v46()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V46)
}

pub(super) fn verify_selected_geometry_v47(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v47()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V47)
}

pub(super) fn verify_selected_geometry_v48(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v48()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V48)
}

pub(super) fn verify_selected_geometry_v49(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !plan.is_v49()
        || plan.components != Component::Db2.bit() | Component::Collision.bit()
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V49)
}

pub(super) fn verify_selected_geometry_v50(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<package::Summary, Error> {
    if !(plan.is_v50() || plan.is_v51())
        || plan.components != if plan.is_v51() { 25 } else { 17 }
        || plan.map_domain.is_some()
    {
        return Err("product_plan_component_scope".into());
    }
    verify_selected_geometry(plan, output, expected, SelectedGeometryProfile::V50)
}

fn verify_selected_geometry(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
    profile: SelectedGeometryProfile,
) -> Result<package::Summary, Error> {
    let held = Held::open(output)?;
    let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
        return Err(BAD.into());
    };
    let limits = match profile {
        SelectedGeometryProfile::V22 => package::Limits {
            total_bytes: budget.output_bytes,
            ..Default::default()
        },
        SelectedGeometryProfile::V28
        | SelectedGeometryProfile::V29
        | SelectedGeometryProfile::V30
        | SelectedGeometryProfile::V31
        | SelectedGeometryProfile::V32
        | SelectedGeometryProfile::V33
        | SelectedGeometryProfile::V34
        | SelectedGeometryProfile::V35
        | SelectedGeometryProfile::V36
        | SelectedGeometryProfile::V37
        | SelectedGeometryProfile::V38
        | SelectedGeometryProfile::V39
        | SelectedGeometryProfile::V40
        | SelectedGeometryProfile::V41
        | SelectedGeometryProfile::V42
        | SelectedGeometryProfile::V43
        | SelectedGeometryProfile::V44
        | SelectedGeometryProfile::V45
        | SelectedGeometryProfile::V46
        | SelectedGeometryProfile::V47
        | SelectedGeometryProfile::V48
        | SelectedGeometryProfile::V49
        | SelectedGeometryProfile::V50 => geometry_v22::package_limits_v28_with_extra_files(
            budget,
            usize::from(profile == SelectedGeometryProfile::V50),
        ),
    };
    let limits = if plan.is_v51() {
        crate::driver::terrain_v20::package_limits_v51(budget)
    } else {
        limits
    };
    let coverage_path = if plan.is_v51() {
        geometry_v22::DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PATH
    } else {
        profile.coverage_path()
    };
    let mut terrain = crate::driver::terrain_v20::VerifyFacts::new();
    let mut terrain_map = None;
    let liquid_object_path = (profile == SelectedGeometryProfile::V50)
        .then(|| format!("dbc/{}/LiquidObject.db2", plan.locale.name()));
    let liquid_object_evidence_path =
        (profile == SelectedGeometryProfile::V50).then_some(crate::driver::liquid_object_v23::PATH);
    let map_path = profile
        .has_authenticated_map()
        .then(|| format!("dbc/{}/Map.db2", plan.locale.name()));
    let gameobject_path = profile
        .has_authenticated_gameobjects()
        .then(|| format!("dbc/{}/GameObjectDisplayInfo.db2", plan.locale.name()));

    let mut package_artifacts: Option<Vec<Artifact>> = None;
    let mut evidence_artifact = None;
    let mut coverage_bytes = None;
    let mut liquid_object_bytes = None;
    let mut liquid_object_evidence = None;
    let mut liquid_facts = [None; 2];
    let mut map_authentication = None;
    let mut gameobject_authentication = None;
    let mut global_declarations = Vec::new();
    let mut table_declarations = Vec::new();
    let mut failure = None;
    let preflight = |artifacts: &[Artifact]| {
        let db2_count = artifacts
            .iter()
            .filter(|artifact| artifact.component == Component::Db2)
            .count();
        let expected_db2 = usize::from(profile.has_authenticated_map())
            + usize::from(profile.has_authenticated_gameobjects())
            + usize::from(profile == SelectedGeometryProfile::V50)
            + 2 * usize::from(plan.is_v51());
        let valid_components = artifacts.iter().all(|artifact| match artifact.component {
            Component::Collision => artifact.path.starts_with("vmaps/"),
            Component::Terrain => plan.is_v51(),
            Component::Db2 => {
                Some(artifact.path.as_str()) == map_path.as_deref()
                    || Some(artifact.path.as_str()) == gameobject_path.as_deref()
                    || Some(artifact.path.as_str()) == liquid_object_path.as_deref()
                    || (plan.is_v51()
                        && ["LiquidMaterial", "LiquidType"].iter().any(|name| {
                            artifact.path == format!("dbc/{}/{name}.db2", plan.locale.name())
                        }))
            }
            _ => false,
        }) && artifacts.iter().all(|artifact| {
            (Some(artifact.path.as_str()) != liquid_object_evidence_path
                || artifact.component == Component::Collision)
                && (Some(artifact.path.as_str()) != liquid_object_path.as_deref()
                    || artifact.component == Component::Db2)
        });
        if plan.is_v51() {
            let terrain_artifacts: Vec<_> = artifacts
                .iter()
                .filter(|artifact| artifact.component != Component::Collision)
                .cloned()
                .collect();
            crate::driver::terrain_v20::admit_metadata_v24(plan, &terrain_artifacts)
                .map_err(|_| package::Error::Inventory)?;
        }
        let expected_files = if plan.is_v51() {
            limits.files
        } else {
            geometry_v22::PACKAGE_MAX_FILES + usize::from(profile == SelectedGeometryProfile::V50)
        };
        let evidence_count = artifacts
            .iter()
            .filter(|artifact| Some(artifact.path.as_str()) == liquid_object_evidence_path)
            .count();
        if artifacts.len() > expected_files
            || evidence_count != usize::from(profile == SelectedGeometryProfile::V50)
            || db2_count != expected_db2
            || !valid_components
            || artifacts
                .iter()
                .filter(|artifact| artifact.path == coverage_path)
                .count()
                != 1
        {
            return Err(package::Error::Inventory);
        }
        evidence_artifact = artifacts
            .iter()
            .find(|artifact| Some(artifact.path.as_str()) == liquid_object_evidence_path)
            .cloned();
        let geometry = if profile == SelectedGeometryProfile::V50 {
            geometry_artifacts(artifacts, liquid_object_evidence_path)?
        } else {
            collision_artifacts(artifacts)?
        };
        if plan.is_v51() {
            let bytes = geometry
                .iter()
                .try_fold(0u64, |total, artifact| total.checked_add(artifact.bytes))
                .ok_or(package::Error::Limit)?;
            if geometry.len() > geometry_v22::PACKAGE_MAX_FILES || bytes > budget.output_bytes {
                return Err(package::Error::Limit);
            }
        }
        package_artifacts = Some(geometry);
        Ok(())
    };
    let inspect = |artifact: &Artifact, file: &mut File| {
        let is_coverage = artifact.path == coverage_path;
        let is_map = map_path.as_deref() == Some(artifact.path.as_str());
        let is_gameobjects = gameobject_path.as_deref() == Some(artifact.path.as_str());
        let is_liquid_object = liquid_object_path.as_deref() == Some(artifact.path.as_str());
        let is_liquid_object_evidence = Some(artifact.path.as_str()) == liquid_object_evidence_path;
        let is_terrain = plan.is_v51() && artifact.component == Component::Terrain;
        let is_liquid_table = plan.is_v51()
            && artifact.component == Component::Db2
            && (artifact.path.ends_with("/LiquidMaterial.db2")
                || artifact.path.ends_with("/LiquidType.db2"));
        if !is_coverage
            && !is_map
            && !is_gameobjects
            && !is_liquid_object
            && !is_liquid_object_evidence
            && !is_terrain
            && !is_liquid_table
        {
            return Ok(());
        }
        let result = (|| {
            let maximum = if is_map || is_gameobjects || is_liquid_object || is_liquid_table {
                16 * 1024 * 1024
            } else if is_liquid_object_evidence {
                112
            } else {
                cov::MAX_BYTES
            };
            let length = usize::try_from(artifact.bytes).map_err(|_| LIMIT)?;
            if length > maximum {
                return Err(LIMIT.into());
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| Error::from("product_allocation"))?;
            if bytes.capacity() > maximum {
                return Err(LIMIT.into());
            }
            bytes.resize(length, 0);
            file.read_exact(&mut bytes).map_err(|_| BAD)?;
            if is_terrain {
                terrain.inspect(artifact, &bytes)?;
            } else if is_liquid_table {
                let name = if artifact.path.ends_with("/LiquidMaterial.db2") {
                    "LiquidMaterial"
                } else {
                    "LiquidType"
                };
                crate::driver::validate_db2(plan, name, &bytes)?;
                let structural = forever_db2::availability::read_structure_70450_for_table(
                    &crate::driver::CompleteDb2Source(&bytes),
                    name,
                )
                .map_err(|_| Error::from("product_liquid_source"))?;
                if structural.sections.iter().any(|section| {
                    section.state != forever_db2::availability::SectionState::Admitted
                }) {
                    return Err("product_liquid_source_partial".into());
                }
                let (index, file_data_id, direct_rows, copy_records, materialized_rows) = if name
                    == "LiquidMaterial"
                {
                    let source = forever_db2::terrain_sources::read_liquid_materials_70450(&bytes)
                        .map_err(|_| Error::from("product_liquid_source"))?;
                    let counts = (source.direct_records().len(), source.copies().len());
                    let rows = source
                        .materialize()
                        .map_err(|_| Error::from("product_liquid_source"))?
                        .len();
                    (0, 1_132_538, counts.0, counts.1, rows)
                } else {
                    let source = forever_db2::terrain_sources::read_liquid_types_70450(&bytes)
                        .map_err(|_| Error::from("product_liquid_source"))?;
                    let counts = (source.direct_records().len(), source.copies().len());
                    let rows = source
                        .materialize()
                        .map_err(|_| Error::from("product_liquid_source"))?
                        .len();
                    (1, 1_371_380, counts.0, counts.1, rows)
                };
                liquid_facts[index] = Some(geometry_v22::LiquidFact {
                    file_data_id,
                    bytes: bytes.len() as u64,
                    magic: bytes[..4].try_into().map_err(|_| BAD)?,
                    direct_rows: direct_rows as u64,
                    copy_records: copy_records as u64,
                    materialized_rows: materialized_rows as u64,
                });
            } else if is_map {
                map_authentication = Some(authenticate_v28_map(&bytes)?);
                if plan.is_v51() {
                    terrain_map = Some(crate::driver::terrain_v20::authenticate_map(&bytes)?);
                }
            } else if is_gameobjects {
                gameobject_authentication = Some(authenticate_v47_gameobjects(&bytes)?);
            } else if is_liquid_object {
                liquid_object_bytes = Some(bytes);
            } else if is_liquid_object_evidence {
                liquid_object_evidence = Some(bytes);
            } else {
                coverage_bytes = Some(bytes);
            }
            Ok::<_, Error>(())
        })();
        result.map_err(|error| {
            failure = Some(error);
            package::Error::Schema
        })
    };
    let verified = match profile {
        SelectedGeometryProfile::V22 => {
            package::verify_with_preflight(output, expected, limits, preflight, inspect)
        }
        SelectedGeometryProfile::V28
        | SelectedGeometryProfile::V29
        | SelectedGeometryProfile::V30
        | SelectedGeometryProfile::V31
        | SelectedGeometryProfile::V32
        | SelectedGeometryProfile::V33
        | SelectedGeometryProfile::V34
        | SelectedGeometryProfile::V35
        | SelectedGeometryProfile::V36
        | SelectedGeometryProfile::V37
        | SelectedGeometryProfile::V38
        | SelectedGeometryProfile::V39
        | SelectedGeometryProfile::V40
        | SelectedGeometryProfile::V41
        | SelectedGeometryProfile::V42
        | SelectedGeometryProfile::V43
        | SelectedGeometryProfile::V44
        | SelectedGeometryProfile::V45
        | SelectedGeometryProfile::V46
        | SelectedGeometryProfile::V47
        | SelectedGeometryProfile::V48
        | SelectedGeometryProfile::V49
        | SelectedGeometryProfile::V50 => package::verify_with_preflight_and_partial_declarations(
            output,
            expected,
            limits,
            |sections| {
                global_declarations
                    .try_reserve_exact(sections.len())
                    .map_err(|_| package::Error::Allocation)?;
                global_declarations.extend_from_slice(sections);
                Ok(())
            },
            |tables| {
                table_declarations
                    .try_reserve_exact(tables.len())
                    .map_err(|_| package::Error::Allocation)?;
                table_declarations.extend_from_slice(tables);
                Ok(())
            },
            preflight,
            inspect,
        ),
    };
    let summary = verified.map_err(|error| failure.unwrap_or_else(|| error.into()))?;
    let bytes = coverage_bytes.ok_or(BAD)?;
    let coverage = match profile {
        SelectedGeometryProfile::V22 => geometry_v22::Coverage::parse(&bytes)?,
        SelectedGeometryProfile::V28 => geometry_v22::Coverage::parse_v28(&bytes)?,
        SelectedGeometryProfile::V29 => geometry_v22::Coverage::parse_v29(&bytes)?,
        SelectedGeometryProfile::V30 => geometry_v22::Coverage::parse_v30(&bytes)?,
        SelectedGeometryProfile::V31 => geometry_v22::Coverage::parse_v31(&bytes)?,
        SelectedGeometryProfile::V32 => geometry_v22::Coverage::parse_v32(&bytes)?,
        SelectedGeometryProfile::V33 => geometry_v22::Coverage::parse_v33(&bytes)?,
        SelectedGeometryProfile::V34 => geometry_v22::Coverage::parse_v34(&bytes)?,
        SelectedGeometryProfile::V35 => geometry_v22::Coverage::parse_v35(&bytes)?,
        SelectedGeometryProfile::V36 => geometry_v22::Coverage::parse_v36(&bytes)?,
        SelectedGeometryProfile::V37 => geometry_v22::Coverage::parse_v37(&bytes)?,
        SelectedGeometryProfile::V38 => geometry_v22::Coverage::parse_v38(&bytes)?,
        SelectedGeometryProfile::V39 => geometry_v22::Coverage::parse_v39(&bytes)?,
        SelectedGeometryProfile::V40 => geometry_v22::Coverage::parse_v40(&bytes)?,
        SelectedGeometryProfile::V41 => geometry_v22::Coverage::parse_v41(&bytes)?,
        SelectedGeometryProfile::V42 => geometry_v22::Coverage::parse_v42(&bytes)?,
        SelectedGeometryProfile::V43 => geometry_v22::Coverage::parse_v43(&bytes)?,
        SelectedGeometryProfile::V44 => geometry_v22::Coverage::parse_v44(&bytes)?,
        SelectedGeometryProfile::V45 => geometry_v22::Coverage::parse_v45(&bytes)?,
        SelectedGeometryProfile::V46 => geometry_v22::Coverage::parse_v46(&bytes)?,
        SelectedGeometryProfile::V47 => geometry_v22::Coverage::parse_v47(&bytes)?,
        SelectedGeometryProfile::V48 => geometry_v22::Coverage::parse_v48(&bytes)?,
        SelectedGeometryProfile::V49 | SelectedGeometryProfile::V50 => {
            if plan.is_v51() {
                geometry_v22::Coverage::parse_v51(&bytes)?
            } else {
                geometry_v22::Coverage::parse_v49(&bytes)?
            }
        }
    };
    let expected_selection = match profile {
        SelectedGeometryProfile::V22 | SelectedGeometryProfile::V28 => {
            MapSelection::MapAndAncestors(plan.map_domain.ok_or("product_plan_map_domain")?)
        }
        SelectedGeometryProfile::V29
        | SelectedGeometryProfile::V30
        | SelectedGeometryProfile::V31
        | SelectedGeometryProfile::V32
        | SelectedGeometryProfile::V33
        | SelectedGeometryProfile::V34
        | SelectedGeometryProfile::V35
        | SelectedGeometryProfile::V36
        | SelectedGeometryProfile::V37
        | SelectedGeometryProfile::V38
        | SelectedGeometryProfile::V39
        | SelectedGeometryProfile::V40
        | SelectedGeometryProfile::V41
        | SelectedGeometryProfile::V42
        | SelectedGeometryProfile::V43
        | SelectedGeometryProfile::V44
        | SelectedGeometryProfile::V45
        | SelectedGeometryProfile::V46
        | SelectedGeometryProfile::V47
        | SelectedGeometryProfile::V48
        | SelectedGeometryProfile::V49
        | SelectedGeometryProfile::V50 => MapSelection::All,
    };
    if coverage.context != summary.context
        || coverage.context.build != 70450
        || coverage.context.locale != plan.locale
        || coverage.context.components
            != if plan.is_v51() {
                25
            } else {
                profile.components()
            }
        || coverage.context.plan_sha256 != integrity::digest(plan.render().as_bytes())
        || coverage.selection != expected_selection
        || coverage.map.file_data_id != forever_casc::collision::world::MAP_FILE_DATA_ID
    {
        return Err("product_plan_mismatch".into());
    }
    if plan.is_v51() {
        terrain.finish_authenticated(plan, summary.context, terrain_map.as_ref().ok_or(BAD)?)?;
        if !global_declarations.is_empty()
            || table_declarations.iter().any(|table| {
                !["Map", "GameObjectDisplayInfo"]
                    .iter()
                    .any(|name| table.path == format!("dbc/{}/{name}.db2", plan.locale.name()))
            })
        {
            return Err("product_partial_manifest_scope".into());
        }
        if liquid_facts != coverage.liquids.map(Some) {
            return Err("product_liquid_source_facts".into());
        }
    }
    match profile {
        SelectedGeometryProfile::V22 => verify_geometry_domain(&coverage)?,
        SelectedGeometryProfile::V28
        | SelectedGeometryProfile::V29
        | SelectedGeometryProfile::V30
        | SelectedGeometryProfile::V31
        | SelectedGeometryProfile::V32
        | SelectedGeometryProfile::V33
        | SelectedGeometryProfile::V34
        | SelectedGeometryProfile::V35
        | SelectedGeometryProfile::V36
        | SelectedGeometryProfile::V37
        | SelectedGeometryProfile::V38
        | SelectedGeometryProfile::V39
        | SelectedGeometryProfile::V40
        | SelectedGeometryProfile::V41
        | SelectedGeometryProfile::V42
        | SelectedGeometryProfile::V43
        | SelectedGeometryProfile::V44
        | SelectedGeometryProfile::V45
        | SelectedGeometryProfile::V46 => verify_v28_coverage(
            &coverage,
            map_authentication.ok_or(BAD)?,
            &global_declarations,
            &table_declarations,
        )?,
        SelectedGeometryProfile::V47
        | SelectedGeometryProfile::V48
        | SelectedGeometryProfile::V49
        | SelectedGeometryProfile::V50 => {
            verify_gameobject_coverage(
                &coverage,
                map_authentication.ok_or(BAD)?,
                gameobject_authentication.ok_or(BAD)?,
                matches!(
                    profile,
                    SelectedGeometryProfile::V48
                        | SelectedGeometryProfile::V49
                        | SelectedGeometryProfile::V50
                ),
                matches!(
                    profile,
                    SelectedGeometryProfile::V49 | SelectedGeometryProfile::V50
                ),
                &global_declarations,
                &table_declarations,
            )?;
        }
    }
    let mut facts = GeometryInventory::new(
        &coverage,
        matches!(
            profile,
            SelectedGeometryProfile::V47
                | SelectedGeometryProfile::V48
                | SelectedGeometryProfile::V49
                | SelectedGeometryProfile::V50
        ),
    )?;
    compare_geometry_inventory(
        package_artifacts.as_deref().ok_or(BAD)?,
        &coverage,
        coverage_path,
    )?;
    if profile == SelectedGeometryProfile::V50 {
        let source = liquid_object_bytes.as_deref().ok_or(BAD)?;
        let evidence_bytes = liquid_object_evidence.as_deref().ok_or(BAD)?;
        let evidence_artifact = evidence_artifact.as_ref().ok_or(BAD)?;
        if evidence_artifact.component != Component::Collision
            || evidence_artifact.path != crate::driver::liquid_object_v23::PATH
            || evidence_artifact.bytes != evidence_bytes.len() as u64
            || evidence_artifact.sha256 != integrity::digest(evidence_bytes)
        {
            return Err("product_liquid_object_evidence".into());
        }
        let evidence = crate::driver::liquid_object_v23::Evidence::from_source(source)?;
        if !evidence.matches_source(source)? || evidence.encode()?.as_slice() != evidence_bytes {
            return Err("product_liquid_object_evidence".into());
        }
        if coverage.liquid_object_scope.as_str() != "out_of_scope"
            || coverage.liquid_object_request_made
            || !global_declarations.is_empty()
            || table_declarations
                .iter()
                .any(|table| table.path == format!("dbc/{}/LiquidObject.db2", plan.locale.name()))
        {
            return Err("product_liquid_object_evidence".into());
        }
    }
    if profile == SelectedGeometryProfile::V50 {
        let liquid_object_path = format!("dbc/{}/LiquidObject.db2", plan.locale.name());
        if !global_declarations.is_empty()
            || table_declarations
                .iter()
                .any(|declaration| declaration.path == liquid_object_path)
        {
            return Err("product_liquid_object_source_partial".into());
        }
    }
    for map in coverage.maps.iter().filter(|map| map.root_absent) {
        if facts.contains_map_artifacts(map.id) {
            return Err(BAD.into());
        }
    }
    let mut missing_models = std::collections::BTreeSet::new();
    for &id in &coverage.missing_m2 {
        let name = geometry_v22::m2_file_name(id);
        let path = format!("vmaps/{name}.vmo");
        if !missing_models.insert(name) || facts.optional_fact(&path).is_some() {
            return Err(BAD.into());
        }
    }
    for &id in coverage
        .missing_wmo
        .iter()
        .chain(&coverage.empty_wmo)
        .chain(&coverage.unselected_wmo)
        .chain(&coverage.material_filtered_wmo)
    {
        let name = geometry_v22::m2_file_name(id);
        let path = format!("vmaps/{name}.vmo");
        if !missing_models.insert(name) || facts.optional_fact(&path).is_some() {
            return Err(BAD.into());
        }
    }
    for &id in coverage
        .gameobject_missing_root
        .iter()
        .chain(&coverage.gameobject_encrypted_unavailable)
    {
        let name = geometry_v22::m2_file_name(id);
        let path = format!("vmaps/{name}.vmo");
        if !missing_models.insert(name) || facts.optional_fact(&path).is_some() {
            return Err(BAD.into());
        }
    }
    let mut model_names = std::collections::BTreeSet::new();
    for fact in &coverage.artifacts {
        if fact.path == coverage_path {
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
    let map_total_limit = assembled_domain_limit(profile, budget.map_bytes);
    for map in &coverage.maps {
        if map.root_absent {
            continue;
        }
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
            if missing_models.contains(name) {
                return Err(BAD.into());
            }
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
                    let parent_map = maps.get(&parent_id).ok_or(BAD)?;
                    if parent_map.root_absent || parent_map.wdt_file_data_id == 0 {
                        return Err(BAD.into());
                    }
                    let parent_path = geometry_tile_path(parent_id, tile.x(), tile.y(), false);
                    if let Some(parent_fact) = facts.optional_fact(&parent_path) {
                        ancestor_payload = Some(held.read(&entry(parent_fact))?);
                        break;
                    }
                    parent = parent_map.parent;
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
    if map_total > map_total_limit
        || (!matches!(
            profile,
            SelectedGeometryProfile::V47
                | SelectedGeometryProfile::V48
                | SelectedGeometryProfile::V49
                | SelectedGeometryProfile::V50
        ) && used_models != model_names)
    {
        return Err("product_selected_geometry_inventory".into());
    }
    if matches!(
        profile,
        SelectedGeometryProfile::V47
            | SelectedGeometryProfile::V48
            | SelectedGeometryProfile::V49
            | SelectedGeometryProfile::V50
    ) {
        verify_v47_gameobject_artifacts(&held, &mut facts, &coverage, &model_names, &used_models)?;
    }
    if !facts.all_used() {
        return Err("product_selected_geometry_inventory".into());
    }
    if profile == SelectedGeometryProfile::V50 {
        let liquid_object_path = format!("dbc/{}/LiquidObject.db2", plan.locale.name());
        if !global_declarations.is_empty()
            || table_declarations
                .iter()
                .any(|declaration| declaration.path == liquid_object_path)
        {
            return Err("product_liquid_object_source_partial".into());
        }
    }

    let mut metadata_error = None;
    let last = package::verify_with_preflight(
        output,
        Some(coverage.context),
        limits,
        |artifacts| {
            let collision = match if profile == SelectedGeometryProfile::V50 {
                geometry_artifacts(artifacts, liquid_object_evidence_path)
            } else {
                collision_artifacts(artifacts)
            } {
                Ok(collision) => collision,
                Err(error) => {
                    metadata_error = Some(error.into());
                    return Err(error);
                }
            };
            compare_geometry_inventory(&collision, &coverage, coverage_path).map_err(|error| {
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

fn verify_gameobject_coverage(
    coverage: &geometry_v22::Coverage,
    map_authentication: V28MapAuthentication,
    gameobject_authentication: V47GameObjectAuthentication,
    allow_missing_root: bool,
    allow_encrypted_unavailable: bool,
    global_sections: &[package::PartialEncryptedSection],
    tables: &[package::PartialEncryptedTable],
) -> Result<(), Error> {
    if allow_encrypted_unavailable && !allow_missing_root {
        return Err(BAD.into());
    }
    let map_path = format!("dbc/{}/Map.db2", coverage.context.locale.name());
    let gameobject_path = format!(
        "dbc/{}/GameObjectDisplayInfo.db2",
        coverage.context.locale.name()
    );
    let liquid_object_path = format!("dbc/{}/LiquidObject.db2", coverage.context.locale.name());
    if allow_encrypted_unavailable
        && (coverage.liquid_object_scope.as_str() != "out_of_scope"
            || coverage.liquid_object_request_made
            || !global_sections.is_empty()
            || tables.iter().any(|table| table.path == liquid_object_path))
    {
        return Err("product_liquid_object_source_partial".into());
    }
    let map_tact_payload_sizes = map_authentication.tact_payload_sizes.clone();
    let mut map_declarations = Vec::new();
    let map_declaration_count = tables.iter().filter(|table| table.path == map_path).count();
    map_declarations
        .try_reserve_exact(map_declaration_count)
        .map_err(|_| Error::from("product_allocation"))?;
    map_declarations.extend(
        tables
            .iter()
            .filter(|table| table.path == map_path)
            .cloned(),
    );
    verify_v28_coverage(
        coverage,
        map_authentication,
        global_sections,
        &map_declarations,
    )?;

    let structural = BTreeMap::from([
        (map_path, map_tact_payload_sizes),
        (
            gameobject_path.clone(),
            gameobject_authentication.tact_payload_sizes.clone(),
        ),
    ]);
    super::verify_v26_partial_declarations(&structural, global_sections, tables)?;

    let unavailable_bytes = tables
        .iter()
        .filter(|table| table.path == gameobject_path)
        .flat_map(|table| &table.sections)
        .try_fold(0_u64, |total, section| {
            total.checked_add(section.decoded_size).ok_or(LIMIT)
        })?;
    let available_bytes = gameobject_authentication
        .declared_bytes
        .checked_sub(unavailable_bytes)
        .ok_or(BAD)?;
    let gameobjects = coverage.gameobjects.as_ref().ok_or(BAD)?;
    if gameobject_authentication.rows.len() != gameobjects.outcomes.len()
        || gameobjects.declared_bytes != gameobject_authentication.declared_bytes
        || gameobjects.available_bytes != available_bytes
        || (gameobjects.full_ckey_verified
            && available_bytes != gameobject_authentication.declared_bytes)
        || (!gameobjects.full_ckey_verified && !gameobjects.encoded_key_authenticated)
        || gameobjects.direct_rows != gameobject_authentication.direct_rows as u64
        || gameobjects.copy_records != gameobject_authentication.copy_records as u64
        || gameobjects.materialized_rows != gameobject_authentication.direct_rows as u64
        || gameobjects.direct_outcomes != gameobject_authentication.rows.len() as u64
    {
        return Err(BAD.into());
    }

    let mut declared_direct_rows = 0_u64;
    let mut declared_copy_records = 0_u64;
    let mut candidate_requests = 0_u64;
    let mut zero_fdids = 0_u64;
    let mut textures = 0_u64;
    let mut collision_rows = 0_u64;
    let mut m2_no_collision = 0_u64;
    let mut wmo_doodad_only = 0_u64;
    let mut missing_root_rows = 0_u64;
    let mut missing_root_fdids = std::collections::BTreeSet::new();
    let mut encrypted_unavailable_rows = 0_u64;
    let mut encrypted_unavailable_fdids = std::collections::BTreeSet::new();
    if gameobjects.sections.len() != gameobject_authentication.structural_sections.len() {
        return Err(BAD.into());
    }
    for (fact, section) in gameobjects
        .sections
        .iter()
        .zip(&gameobject_authentication.structural_sections)
    {
        if fact.ordinal != section.ordinal
            || fact.records != section.declared_records
            || fact.copies != section.declared_copies
            || fact.admitted != (section.state == forever_db2::availability::SectionState::Admitted)
        {
            return Err(BAD.into());
        }
        declared_direct_rows = declared_direct_rows
            .checked_add(u64::from(section.declared_records))
            .ok_or(LIMIT)?;
        declared_copy_records = declared_copy_records
            .checked_add(u64::from(section.declared_copies))
            .ok_or(LIMIT)?;
    }
    for (ordinal, (&(display_id, file_data_id), fact)) in gameobject_authentication
        .rows
        .iter()
        .zip(&gameobjects.outcomes)
        .enumerate()
    {
        if fact.ordinal != ordinal as u64
            || fact.display_id != display_id
            || fact.file_data_id != file_data_id
            || (file_data_id == 0
                && fact.disposition != geometry_v22::GameObjectDisposition::ZeroFileDataId)
            || (file_data_id != 0
                && fact.disposition == geometry_v22::GameObjectDisposition::ZeroFileDataId)
        {
            return Err(BAD.into());
        }
        candidate_requests = candidate_requests
            .checked_add(u64::from(file_data_id != 0))
            .ok_or(LIMIT)?;
        if allow_missing_root
            && candidate_requests
                > forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS as u64
        {
            return Err(LIMIT.into());
        }
        match fact.disposition {
            geometry_v22::GameObjectDisposition::ZeroFileDataId => zero_fdids += 1,
            geometry_v22::GameObjectDisposition::Texture => textures += 1,
            geometry_v22::GameObjectDisposition::Collision => collision_rows += 1,
            geometry_v22::GameObjectDisposition::M2NoCollision => m2_no_collision += 1,
            geometry_v22::GameObjectDisposition::WmoDoodadOnly => wmo_doodad_only += 1,
            geometry_v22::GameObjectDisposition::MissingRoot => {
                if !allow_missing_root || file_data_id == 0 {
                    return Err(BAD.into());
                }
                missing_root_rows += 1;
                missing_root_fdids.insert(file_data_id);
            }
            geometry_v22::GameObjectDisposition::EncryptedUnavailable => {
                if !allow_encrypted_unavailable || file_data_id == 0 {
                    return Err(BAD.into());
                }
                encrypted_unavailable_rows += 1;
                encrypted_unavailable_fdids.insert(file_data_id);
            }
        }
    }
    let listed_gameobject_rows = gameobjects
        .list_m2_rows
        .checked_add(gameobjects.list_wmo_rows)
        .ok_or(LIMIT)?;
    if gameobjects.declared_direct_rows != declared_direct_rows
        || gameobjects.declared_copy_records != declared_copy_records
        || gameobjects.copy_records > declared_copy_records
        || candidate_requests != gameobjects.candidate_requests
        || zero_fdids != gameobjects.zero_fdids
        || textures != gameobjects.textures
        || collision_rows != gameobjects.collision_rows
        || m2_no_collision != gameobjects.m2_no_collision
        || wmo_doodad_only != gameobjects.wmo_doodad_only
        || (allow_missing_root
            && (gameobjects.missing_root_rows != missing_root_rows
                || missing_root_fdids.iter().copied().collect::<Vec<_>>()
                    != coverage.gameobject_missing_root))
        || (!allow_missing_root
            && (gameobjects.missing_root_rows != 0
                || !gameobjects.missing_root_fdids.is_empty()
                || !coverage.gameobject_missing_root.is_empty()))
        || (allow_encrypted_unavailable
            && (gameobjects.encrypted_unavailable_rows != encrypted_unavailable_rows
                || encrypted_unavailable_fdids
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    != coverage.gameobject_encrypted_unavailable))
        || (!allow_encrypted_unavailable
            && (gameobjects.encrypted_unavailable_rows != 0
                || !gameobjects.encrypted_unavailable_fdids.is_empty()
                || !coverage.gameobject_encrypted_unavailable.is_empty()))
        || missing_root_fdids
            .iter()
            .any(|id| encrypted_unavailable_fdids.contains(id))
        || gameobjects.list_rows != collision_rows
        || listed_gameobject_rows != collision_rows
    {
        return Err(BAD.into());
    }
    Ok(())
}

fn verify_v28_coverage(
    coverage: &geometry_v22::Coverage,
    authentication: V28MapAuthentication,
    global_sections: &[package::PartialEncryptedSection],
    tables: &[package::PartialEncryptedTable],
) -> Result<(), Error> {
    let V28MapAuthentication {
        declared_bytes,
        direct_rows,
        copy_records,
        tact_payload_sizes,
        owner,
    } = authentication;
    geometry_v22::validate_missing_m2(&coverage.missing_m2, coverage.missing_m2_policy)
        .map_err(|_| BAD)?;
    let map_path = format!("dbc/{}/Map.db2", coverage.context.locale.name());
    let structural = std::collections::BTreeMap::from([(map_path, tact_payload_sizes)]);
    super::verify_v26_partial_declarations(&structural, global_sections, tables)?;

    let mut unavailable_bytes = 0_u64;
    for table in tables {
        for section in &table.sections {
            unavailable_bytes = unavailable_bytes
                .checked_add(section.decoded_size)
                .ok_or(LIMIT)?;
        }
    }
    let available_bytes = declared_bytes.checked_sub(unavailable_bytes).ok_or(BAD)?;
    if (coverage.map.full_ckey_verified && available_bytes != declared_bytes)
        || (!coverage.map.full_ckey_verified && !coverage.map.encoded_key_authenticated)
    {
        return Err(BAD.into());
    }
    if coverage.map.declared_bytes != declared_bytes
        || coverage.map.available_bytes != available_bytes
    {
        return Err(BAD.into());
    }

    let maps = Maps::from_materialized_selection(
        owner.rows(),
        owner.direct_records().len(),
        coverage.selection,
        Default::default(),
    )
    .map_err(|error| Error::from(error.code()))?;
    let mut declared_direct_rows = 0_u64;
    let mut declared_copy_records = 0_u64;
    if coverage.map.sections.len() != owner.sections().len() {
        return Err(BAD.into());
    }
    for (fact, section) in coverage.map.sections.iter().zip(owner.sections()) {
        if fact.ordinal != section.ordinal
            || fact.records != section.declared_records
            || fact.copies != section.declared_copies
            || fact.admitted != (section.state == forever_db2::availability::SectionState::Admitted)
        {
            return Err(BAD.into());
        }
        declared_direct_rows = declared_direct_rows
            .checked_add(u64::from(section.declared_records))
            .ok_or(LIMIT)?;
        declared_copy_records = declared_copy_records
            .checked_add(u64::from(section.declared_copies))
            .ok_or(LIMIT)?;
    }
    if coverage.map.declared_direct_rows != declared_direct_rows
        || coverage.map.declared_copy_records != declared_copy_records
        || coverage.map.direct_rows != direct_rows as u64
        || coverage.map.copy_records != copy_records as u64
        || coverage.map.materialized_rows != maps.stats().rows as u64
        || coverage.maps.len() != maps.stats().active_maps
    {
        return Err(BAD.into());
    }
    for fact in &coverage.maps {
        if !maps.contains_selected(fact.id)
            || (fact.root_absent
                && coverage.missing_wdt_policy
                    != forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent)
        {
            return Err(BAD.into());
        }
        let entry = maps.get(fact.id).ok_or(BAD)?;
        if entry.id() != fact.id
            || entry.parent_map_id() != fact.parent
            || entry.wdt_file_data_id() != fact.wdt_file_data_id
            || u32::try_from(entry.depth()).map_err(|_| LIMIT)? != fact.depth
            || entry.is_parent() != fact.is_parent
        {
            return Err(BAD.into());
        }
    }
    Ok(())
}

struct GeometryInventory<'a> {
    entries: std::collections::BTreeMap<&'a str, &'a ArtifactFact>,
    used: std::collections::BTreeSet<&'a str>,
}
impl<'a> GeometryInventory<'a> {
    fn new(coverage: &'a geometry_v22::Coverage, v47: bool) -> Result<Self, Error> {
        let mut entries = std::collections::BTreeMap::new();
        for fact in &coverage.artifacts {
            if !(valid_geometry_path(&fact.path)
                || (v47 && fact.path == geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH))
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
    fn contains_map_artifacts(&self, id: u32) -> bool {
        let (tree_path, proof_path) = geometry_paths(id);
        if self.entries.contains_key(tree_path.as_str())
            || self.entries.contains_key(proof_path.as_str())
        {
            return true;
        }
        let prefix = format!("vmaps/{id:04}/");
        self.entries
            .range(prefix.as_str()..)
            .take_while(|(path, _)| path.starts_with(&prefix))
            .next()
            .is_some()
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
    coverage_path: &str,
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
        if artifact.path == coverage_path {
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

fn verify_v47_gameobject_artifacts(
    held: &Held,
    facts: &mut GeometryInventory<'_>,
    coverage: &geometry_v22::Coverage,
    model_names: &std::collections::BTreeSet<String>,
    placement_models: &std::collections::BTreeSet<String>,
) -> Result<(), Error> {
    let gameobjects = coverage.gameobjects.as_ref().ok_or(BAD)?;
    let tree_fact = facts.fact(geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH)?;
    if tree_fact.bytes != gameobjects.dtree_bytes || tree_fact.sha != gameobjects.dtree_sha {
        return Err(BAD.into());
    }
    let tree_bytes = held.read(&entry(tree_fact))?;
    let tree = gameobject_models::Dtree::parse(&tree_bytes, gameobject_models::Limits::default())
        .map_err(|error| Error::from(error.code()))?;
    if tree.row_count() as u64 != gameobjects.collision_rows {
        return Err(BAD.into());
    }

    let temp_size = as_usize(gameobjects.temp_bytes)?;
    if temp_size > gameobject_models::MAX_LIST_BYTES {
        return Err(LIMIT.into());
    }
    let mut reconstructed_temp = Vec::new();
    reconstructed_temp
        .try_reserve_exact(temp_size)
        .map_err(|_| Error::from("product_allocation"))?;
    reconstructed_temp.extend_from_slice(gameobject_models::TEMP_MAGIC);
    let mut outcomes = gameobjects
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == geometry_v22::GameObjectDisposition::Collision);
    let mut gameobject_models_used = std::collections::BTreeSet::new();
    for row in tree.rows() {
        let outcome = outcomes.next().ok_or(BAD)?;
        let expected_name = geometry_v22::m2_file_name(outcome.file_data_id);
        if row.display_id() != outcome.display_id || row.name() != expected_name.as_bytes() {
            return Err(BAD.into());
        }
        reconstructed_temp.extend_from_slice(&row.display_id().to_le_bytes());
        reconstructed_temp.extend_from_slice(
            &u32::try_from(row.name().len())
                .map_err(|_| LIMIT)?
                .to_le_bytes(),
        );
        reconstructed_temp.extend_from_slice(row.name());

        let model_path = format!("vmaps/{expected_name}.vmo");
        facts.fact(&model_path)?;
        gameobject_models_used.insert(expected_name);
    }
    let mut expected_models = placement_models.clone();
    expected_models.extend(gameobject_models_used.iter().cloned());
    let missing_gameobject_models: std::collections::BTreeSet<_> = coverage
        .gameobject_missing_root
        .iter()
        .chain(&coverage.gameobject_encrypted_unavailable)
        .map(|&id| geometry_v22::m2_file_name(id))
        .collect();
    if outcomes.next().is_some()
        || reconstructed_temp.len() != temp_size
        || integrity::digest(&reconstructed_temp) != gameobjects.temp_sha
        || model_names != &expected_models
        || missing_gameobject_models
            .iter()
            .any(|name| model_names.contains(name) || placement_models.contains(name))
    {
        return Err(BAD.into());
    }
    facts.mark(tree_fact)?;
    Ok(())
}

fn verify_geometry_domain(coverage: &geometry_v22::Coverage) -> Result<(), Error> {
    let MapSelection::MapAndAncestors(requested_map) = coverage.selection else {
        return Err(BAD.into());
    };
    let mut by_id = std::collections::BTreeMap::new();
    for map in &coverage.maps {
        if by_id.insert(map.id, map).is_some() || map.wdt_file_data_id == 0 {
            return Err(BAD.into());
        }
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut current = Some(requested_map);
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
    if path == geometry_v22::COVERAGE_PATH
        || path == geometry_v22::AVAILABLE_COVERAGE_PATH
        || path == geometry_v22::ROOT_ABSENT_COVERAGE_PATH
        || path == geometry_v22::MODEL_ROOT_ABSENT_COVERAGE_PATH
        || path == geometry_v22::REQUEST_CAPACITY_COVERAGE_PATH
        || path == geometry_v22::VACATED_WMO_COVERAGE_PATH
        || path == geometry_v22::EMPTY_WMO_COVERAGE_PATH
        || path == geometry_v22::UNSELECTED_WMO_COVERAGE_PATH
        || path == geometry_v22::MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
        || path == geometry_v22::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH
    {
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
    if path == geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH {
        return false;
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
            selection: MapSelection::MapAndAncestors(2),
            missing_wdt_policy: forever_casc::collision::run::MissingWdtPolicy::Reject,
            missing_m2_policy: forever_casc::collision::run::MissingModelPolicy::Reject,
            missing_wmo_policy: forever_casc::collision::run::MissingWmoPolicy::Reject,
            resource_profile: forever_casc::collision::resources::Profile::BoundedV1,
            missing_m2: Vec::new(),
            missing_wmo: Vec::new(),
            empty_wmo: Vec::new(),
            unselected_wmo: Vec::new(),
            material_filtered_wmo: Vec::new(),
            gameobject_missing_root: Vec::new(),
            gameobject_encrypted_unavailable: Vec::new(),
            gameobject_scope: forever_casc::collision::geometry::GameObjectScope::Excluded,
            gameobject_file_data_id: forever_casc::collision::gameobjects::TABLE_FILE_DATA_ID,
            gameobject_block_reason:
                forever_casc::collision::geometry::GameObjectBlockReason::ExcludedByProfile,
            gameobject_request_made: false,
            gameobject_startup_reads: 0,
            gameobjects: None,
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
                root_absent: false,
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

    mod v28 {
        use super::*;

        const V28: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v28/map2.tsv");
        const V37: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v37/all.tsv");
        const V38: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v38/all.tsv");
        const MAP_FIELDS: &[(u16, u16)] = &[
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 2),
            (8, 1),
            (8, 1),
            (8, 1),
            (16, 1),
            (16, 1),
            (16, 1),
            (16, 1),
            (16, 1),
            (8, 1),
            (32, 1),
            (16, 1),
            (8, 1),
            (16, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 1),
            (32, 3),
        ];

        struct V28Fixture {
            plan: Plan,
            coverage: geometry_v22::Coverage,
            files: Vec<(String, Vec<u8>)>,
            map: Vec<u8>,
            declaration: package::PartialEncryptedTable,
        }
        impl V28Fixture {
            fn authenticate(&self) -> Result<(), Error> {
                verify_v28_coverage(
                    &self.coverage,
                    authenticate_v28_map(&self.map)?,
                    &[],
                    std::slice::from_ref(&self.declaration),
                )
            }

            fn publish(&self, lab: &Lab, name: &str) -> PathBuf {
                self.publish_with_declarations(
                    lab,
                    name,
                    &[],
                    std::slice::from_ref(&self.declaration),
                )
            }

            fn publish_with_declarations(
                &self,
                lab: &Lab,
                name: &str,
                global: &[package::PartialEncryptedSection],
                tables: &[package::PartialEncryptedTable],
            ) -> PathBuf {
                let output = lab.0.join(name);
                let CollisionStorage::LeasedDiskV1(budget) = self.plan.collision_storage else {
                    panic!("v28 fixture requires a disk budget");
                };
                let mut run = package::Run::create(
                    &output,
                    self.coverage.context,
                    &[],
                    geometry_v22::package_limits_v28_with_extra_files(budget, 0),
                )
                .unwrap();
                let map_path = format!("dbc/{}/Map.db2", self.plan.locale.name());
                run.put(Component::Db2, &map_path, &self.map).unwrap();
                for (path, bytes) in &self.files {
                    run.put(Component::Collision, path, bytes).unwrap();
                }
                let coverage_path = match (
                    self.coverage.selection,
                    self.coverage.missing_wdt_policy,
                    self.coverage.missing_m2_policy,
                    self.coverage.resource_profile,
                ) {
                    (
                        MapSelection::MapAndAncestors(_),
                        forever_casc::collision::run::MissingWdtPolicy::Reject,
                        forever_casc::collision::run::MissingModelPolicy::Reject,
                        forever_casc::collision::resources::Profile::BoundedV1,
                    ) => geometry_v22::COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::Reject,
                        forever_casc::collision::run::MissingModelPolicy::Reject,
                        forever_casc::collision::resources::Profile::BoundedV1,
                    ) => geometry_v22::AVAILABLE_COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent,
                        forever_casc::collision::run::MissingModelPolicy::Reject,
                        forever_casc::collision::resources::Profile::BoundedV1,
                    ) => geometry_v22::ROOT_ABSENT_COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent,
                        forever_casc::collision::run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                        forever_casc::collision::resources::Profile::BoundedV1,
                    ) => geometry_v22::MODEL_ROOT_ABSENT_COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent,
                        forever_casc::collision::run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                        forever_casc::collision::resources::Profile::RequestHeavyV1,
                    ) => geometry_v22::REQUEST_CAPACITY_COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent,
                        forever_casc::collision::run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                        forever_casc::collision::resources::Profile::DependencyHeavyV1,
                    ) => geometry_v22::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
                    (
                        MapSelection::All,
                        forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent,
                        forever_casc::collision::run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                        forever_casc::collision::resources::Profile::SourceHeavyV1,
                    ) => geometry_v22::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH,
                    _ => panic!("invalid coverage selection and missing-WDT/model policies"),
                };
                run.put(
                    Component::Collision,
                    coverage_path,
                    &self.coverage.encode().unwrap(),
                )
                .unwrap();
                if !global.is_empty() {
                    run.set_partial_encrypted_sections(global).unwrap();
                }
                for table in tables {
                    run.set_partial_encrypted_table(&table.path, &table.sections)
                        .unwrap();
                }
                run.finish_component(Component::Db2).unwrap();
                run.finish_component(Component::Collision).unwrap();
                run.finish().unwrap();
                output
            }
        }

        fn put32(bytes: &mut [u8], at: usize, value: u32) {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        fn field_offset(field: usize) -> usize {
            MAP_FIELDS[..field]
                .iter()
                .map(|(width, count)| usize::from(width * count / 8))
                .sum()
        }
        fn partial_map() -> Vec<u8> {
            let rows: [(u32, u32, i16, i16); 4] = [
                (1, 101, -1, -1),
                (2, 102, 1, -1),
                (3, 103, 2, -1),
                (4, 0, -1, -1),
            ];
            let stride = field_offset(MAP_FIELDS.len());
            let section_headers = 2 * 40;
            let metadata = 204 + section_headers + MAP_FIELDS.len() * 28;
            let global = metadata + 4;
            let mut bytes = vec![0; global];
            bytes[..4].copy_from_slice(b"WDC5");
            let mut records = Vec::new();
            let mut texts = Vec::new();
            for (ordinal, &(_, wdt, parent, cosmetic)) in rows.iter().enumerate() {
                let mut row = vec![0; stride];
                for (field, text) in [(0, b"Synthetic".as_slice()), (1, b"Map".as_slice())] {
                    let offset = field_offset(field);
                    let relative = (5 - ordinal) * stride + texts.len() - offset;
                    put32(&mut row, offset, relative as u32);
                    texts.extend_from_slice(text);
                    texts.push(0);
                }
                put32(&mut row, field_offset(21), wdt);
                for (field, value) in [(13, parent), (14, cosmetic)] {
                    let at = field_offset(field);
                    row[at..at + 2].copy_from_slice(&value.to_le_bytes());
                }
                records.extend(row);
            }
            for (at, value) in [
                (4, 5),
                (136, 5),
                (140, MAP_FIELDS.len() as u32),
                (144, stride as u32),
                (148, texts.len() as u32),
                (152, 0xBD84_CD62),
                (156, 0xD43A_FAC3),
                (164, u32::MAX),
                (176, MAP_FIELDS.len() as u32),
                (180, stride as u32),
                (188, (MAP_FIELDS.len() * 24) as u32),
                (200, 2),
            ] {
                put32(&mut bytes, at, value);
            }
            bytes[172..174].copy_from_slice(&4_u16.to_le_bytes());
            bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
            for (field, &(width, count)) in MAP_FIELDS.iter().enumerate() {
                let primitive = 204 + section_headers + field * 4;
                bytes[primitive..primitive + 2].copy_from_slice(&(32 - width).to_le_bytes());
                bytes[primitive + 2..primitive + 4]
                    .copy_from_slice(&(field_offset(field) as u16).to_le_bytes());
                let column = 204 + section_headers + MAP_FIELDS.len() * 4 + field * 24;
                bytes[column..column + 2]
                    .copy_from_slice(&((field_offset(field) * 8) as u16).to_le_bytes());
                bytes[column + 2..column + 4].copy_from_slice(&(width * count).to_le_bytes());
            }
            put32(&mut bytes, 212, global as u32);
            put32(&mut bytes, 216, 4);
            put32(&mut bytes, 220, texts.len() as u32);
            put32(&mut bytes, 228, 16);
            put32(&mut bytes, 240, 1);
            bytes.extend(records);
            bytes.extend(texts);
            for (id, _, _, _) in rows {
                bytes.extend_from_slice(&id.to_le_bytes());
            }
            bytes.extend_from_slice(&5_u32.to_le_bytes());
            bytes.extend_from_slice(&4_u32.to_le_bytes());

            bytes[244] = 1;
            let omitted_offset = bytes.len() as u32;
            put32(&mut bytes, 252, omitted_offset);
            put32(&mut bytes, 256, 1);
            put32(&mut bytes, 268, 4);
            put32(&mut bytes, 280, 1);
            bytes.resize(bytes.len() + stride + 4 + 8, 0);
            bytes
        }

        fn section(frame_ordinal: u32, decoded_size: u64) -> package::PartialEncryptedSection {
            package::PartialEncryptedSection {
                lookup: if frame_ordinal == 0 {
                    "0123456789abcdef".into()
                } else {
                    "fedcba9876543210".into()
                },
                frame_ordinal,
                decoded_size,
            }
        }

        fn fixture() -> V28Fixture {
            let plan = Plan::parse(V28).unwrap();
            let (_, mut coverage, _) = empty_geometry_fixture();
            coverage.context = Context {
                build: 70450,
                locale: plan.locale,
                components: Component::Db2.bit() | Component::Collision.bit(),
                source_sha256: [0x41; 32],
                plan_sha256: integrity::digest(plan.render().as_bytes()),
                producer_sha256: [0x42; 32],
            };
            let map = partial_map();
            let structural = forever_db2::availability::read_structure_70450_for_table(
                &CompleteDb2Source(&map),
                "Map",
            )
            .unwrap();
            assert_eq!(structural.tact_payload_sizes.len(), 1);
            let unavailable = structural.tact_payload_sizes[0];
            let first = unavailable / 2;
            let declaration = package::PartialEncryptedTable {
                path: format!("dbc/{}/Map.db2", plan.locale.name()),
                sections: vec![section(0, first), section(1, unavailable - first)],
            };
            coverage.map = geometry_v22::AvailableMapFact {
                file_data_id: forever_casc::collision::world::MAP_FILE_DATA_ID,
                declared_bytes: map.len() as u64,
                available_bytes: map.len() as u64 - unavailable,
                full_ckey_verified: false,
                encoded_key_authenticated: true,
                declared_direct_rows: 5,
                declared_copy_records: 2,
                direct_rows: 4,
                copy_records: 1,
                materialized_rows: 5,
                sections: vec![
                    geometry_v22::SectionFact {
                        ordinal: 0,
                        records: 4,
                        copies: 1,
                        admitted: true,
                    },
                    geometry_v22::SectionFact {
                        ordinal: 1,
                        records: 1,
                        copies: 1,
                        admitted: false,
                    },
                ],
            };
            coverage.maps = vec![
                geometry_v22::DomainMapFact {
                    id: 1,
                    parent: None,
                    wdt_file_data_id: 101,
                    depth: 0,
                    is_parent: true,
                    root_absent: false,
                },
                geometry_v22::DomainMapFact {
                    id: 2,
                    parent: Some(1),
                    wdt_file_data_id: 102,
                    depth: 1,
                    is_parent: true,
                    root_absent: false,
                },
            ];
            let mut files = Vec::new();
            for id in [1, 2] {
                let assembled = assembly::build(id, &[], Default::default()).unwrap();
                let (tree_path, proof_path) = geometry_paths(id);
                files.push((proof_path, assembled.ordered_dir_bin().to_vec()));
                files.push((tree_path, assembled.tree_bytes().to_vec()));
            }
            coverage.artifacts = artifact_facts(&files);
            V28Fixture {
                plan,
                coverage,
                files,
                map,
                declaration,
            }
        }

        fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
            match verify_selected_geometry_v28(&fixture.plan, output, None) {
                Ok(_) => panic!("mutated v28 package unexpectedly passed"),
                Err(error) => error,
            }
        }

        #[test]
        fn partial_map_fixture_authenticates_full_graph_and_multi_frame_coverage() {
            let fixture = fixture();
            fixture.authenticate().unwrap();
            assert_eq!(fixture.coverage.maps.len(), 2);
            assert!(fixture.coverage.maps[1].is_parent);
            assert_eq!(fixture.coverage.map.materialized_rows, 5);
        }

        #[test]
        fn sealed_partial_map_and_selected_geometry_verify() {
            let lab = Lab::new();
            let fixture = fixture();
            let output = fixture.publish(&lab, "package");
            let summary = verify_selected_geometry_v28(&fixture.plan, &output, None).unwrap();
            assert_eq!(summary.context, fixture.coverage.context);
            assert_eq!(summary.files, 6);
        }

        #[test]
        fn rejects_a_selection_not_bound_to_the_plan_map() {
            let lab = Lab::new();
            let mut fixture = fixture();
            fixture.coverage.selection = MapSelection::MapAndAncestors(1);
            let output = fixture.publish(&lab, "wrong-selection");
            assert_eq!(rejection(&fixture, &output).code, "product_plan_mismatch");
        }

        #[test]
        fn rejects_schema_valid_but_false_map_facts() {
            #[derive(Clone, Copy)]
            enum Mutation {
                Parent,
                Wdt,
                Depth,
                IsParent,
                Section,
                MaterializedRows,
            }
            let lab = Lab::new();
            for (name, mutation) in [
                ("parent", Mutation::Parent),
                ("wdt", Mutation::Wdt),
                ("depth", Mutation::Depth),
                ("is-parent", Mutation::IsParent),
                ("section", Mutation::Section),
                ("materialized", Mutation::MaterializedRows),
            ] {
                let mut fixture = fixture();
                match mutation {
                    Mutation::Parent => fixture.coverage.maps[1].parent = None,
                    Mutation::Wdt => fixture.coverage.maps[1].wdt_file_data_id += 1,
                    Mutation::Depth => fixture.coverage.maps[1].depth = 2,
                    Mutation::IsParent => fixture.coverage.maps[1].is_parent = false,
                    Mutation::Section => {
                        fixture.coverage.map.sections[1].records += 1;
                        fixture.coverage.map.declared_direct_rows += 1;
                    }
                    Mutation::MaterializedRows => fixture.coverage.map.materialized_rows = 4,
                }
                let encoded = fixture.coverage.encode().unwrap();
                geometry_v22::Coverage::parse_v28(&encoded).unwrap();
                let output = fixture.publish(&lab, name);
                assert_eq!(rejection(&fixture, &output).code, BAD, "{name}");
            }
        }

        #[test]
        fn rejects_inconsistent_source_authentication_flags() {
            for (full_ckey, encoded_key) in [(true, true), (false, false)] {
                let mut fixture = fixture();
                fixture.coverage.map.full_ckey_verified = full_ckey;
                fixture.coverage.map.encoded_key_authenticated = encoded_key;
                assert_eq!(fixture.authenticate().unwrap_err().code, BAD);
            }
        }

        #[test]
        fn rejects_missing_wrong_and_extra_per_table_declarations() {
            let lab = Lab::new();
            let missing = fixture();
            let output = missing.publish_with_declarations(&lab, "missing", &[], &[]);
            assert_eq!(
                rejection(&missing, &output).code,
                "product_partial_undeclared"
            );

            let mut wrong = fixture();
            wrong.declaration.sections[0].decoded_size += 1;
            let output = wrong.publish(&lab, "wrong");
            assert_eq!(
                rejection(&wrong, &output).code,
                "product_partial_section_count"
            );

            let extra = fixture();
            let tables = vec![
                extra.declaration.clone(),
                package::PartialEncryptedTable {
                    path: format!("dbc/{}/MapExtra.db2", extra.plan.locale.name()),
                    sections: vec![section(2, 1)],
                },
            ];
            let output = extra.publish_with_declarations(&lab, "extra", &[], &tables);
            assert_eq!(
                rejection(&extra, &output).code,
                "product_partial_declared_file_missing"
            );
        }

        #[test]
        fn rejects_global_partial_declarations() {
            let fixture = fixture();
            let error = verify_v28_coverage(
                &fixture.coverage,
                authenticate_v28_map(&fixture.map).unwrap(),
                &[section(0, 1)],
                std::slice::from_ref(&fixture.declaration),
            )
            .unwrap_err();
            assert_eq!(error.code, "product_partial_manifest_scope");
        }

        #[test]
        fn rejects_nonzero_fill_in_the_omitted_payload() {
            let lab = Lab::new();
            let mut fixture = fixture();
            let structural = forever_db2::availability::read_structure_70450_for_table(
                &CompleteDb2Source(&fixture.map),
                "Map",
            )
            .unwrap();
            let offset = structural.tact_payload_offsets[0] as usize;
            fixture.map[offset] = 1;
            let output = fixture.publish(&lab, "nonzero-fill");
            rejection(&fixture, &output);
        }

        #[test]
        fn wrappers_reject_the_other_profile_before_io() {
            let fixture = fixture();
            let (v22, _, _) = empty_geometry_fixture();
            let missing = Path::new("/nonexistent-selected-geometry-v28");
            assert_eq!(
                verify_selected_geometry_v28(&v22, missing, None)
                    .unwrap_err()
                    .code,
                "product_plan_component_scope"
            );
            assert_eq!(
                verify_selected_geometry_v22(&fixture.plan, missing, None)
                    .unwrap_err()
                    .code,
                "product_plan_component_scope"
            );
        }

        mod v29 {
            use super::*;

            const V29: &str =
                include_str!("../../../../tests/contracts/fixtures/forever-product-v29/all.tsv");

            fn push_map(files: &mut Vec<(String, Vec<u8>)>, id: u32, proof: &[u8]) {
                let parsed = placement::DirBin::parse(proof, placement::Limits::default()).unwrap();
                let records: Vec<_> = parsed.records().collect();
                let assembled = assembly::build(id, &records, Default::default()).unwrap();
                assert_eq!(assembled.ordered_dir_bin(), proof);
                let (tree_path, proof_path) = geometry_paths(id);
                files.push((proof_path, assembled.ordered_dir_bin().to_vec()));
                files.push((tree_path, assembled.tree_bytes().to_vec()));
                for tile in assembled.tiles() {
                    files.push((
                        geometry_tile_path(id, tile.x(), tile.y(), true),
                        tile.index_bytes().to_vec(),
                    ));
                    if let Some(payload) = tile.payload() {
                        files.push((
                            geometry_tile_path(id, tile.x(), tile.y(), false),
                            payload.to_vec(),
                        ));
                    }
                }
            }

            fn owned_placement() -> Vec<u8> {
                let name = b"synthetic";
                let mut bytes = vec![1, 7];
                for value in [
                    17,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    1.0_f32.to_bits(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    name.len() as u32,
                ] {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
                bytes.extend_from_slice(name);
                bytes
            }

            fn synthetic_vmo() -> Vec<u8> {
                let mut bytes = b"VMAP_4.EWMOD".to_vec();
                bytes.extend_from_slice(&8_u32.to_le_bytes());
                bytes.extend_from_slice(&0_u32.to_le_bytes());
                bytes.extend_from_slice(&0_u32.to_le_bytes());
                assert!(vmo::View::parse(&bytes, Default::default()).is_ok());
                bytes
            }

            fn available_fixture() -> V28Fixture {
                let mut fixture = super::fixture();
                fixture.plan = Plan::parse(V29).unwrap();
                fixture.declaration.path = format!("dbc/{}/Map.db2", fixture.plan.locale.name());
                fixture.coverage.context = Context {
                    build: 70450,
                    locale: fixture.plan.locale,
                    components: Component::Db2.bit() | Component::Collision.bit(),
                    source_sha256: [0x41; 32],
                    plan_sha256: integrity::digest(fixture.plan.render().as_bytes()),
                    producer_sha256: [0x42; 32],
                };
                fixture.coverage.selection = MapSelection::All;
                fixture.coverage.maps = vec![
                    geometry_v22::DomainMapFact {
                        id: 1,
                        parent: None,
                        wdt_file_data_id: 101,
                        depth: 0,
                        is_parent: true,
                        root_absent: false,
                    },
                    geometry_v22::DomainMapFact {
                        id: 2,
                        parent: Some(1),
                        wdt_file_data_id: 102,
                        depth: 1,
                        is_parent: true,
                        root_absent: false,
                    },
                    geometry_v22::DomainMapFact {
                        id: 3,
                        parent: Some(2),
                        wdt_file_data_id: 103,
                        depth: 2,
                        is_parent: false,
                        root_absent: false,
                    },
                ];
                fixture.files.clear();
                push_map(&mut fixture.files, 1, &owned_placement());
                push_map(&mut fixture.files, 2, &[]);
                push_map(&mut fixture.files, 3, &[]);
                fixture
                    .files
                    .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                fixture
            }

            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                match verify_selected_geometry_v29(&fixture.plan, output, None) {
                    Ok(_) => panic!("mutated v29 package unexpectedly passed"),
                    Err(error) => error,
                }
            }

            #[test]
            fn all_selection_uses_the_complete_materialized_graph_but_only_active_maps() {
                let fixture = available_fixture();
                assert_eq!(fixture.plan.map_domain, None);
                assert_eq!(fixture.coverage.context.components, 17);
                let authentication = authenticate_v28_map(&fixture.map).unwrap();
                let (rows, active_maps, inactive_rows, inactive_selected, omitted_unknown) = {
                    let maps = Maps::from_materialized_selection(
                        authentication.owner.rows(),
                        authentication.owner.direct_records().len(),
                        fixture.coverage.selection,
                        Default::default(),
                    )
                    .unwrap();
                    (
                        maps.stats().rows,
                        maps.stats().active_maps,
                        maps.all().filter(|entry| !entry.active()).count(),
                        maps.contains_selected(4) || maps.contains_selected(5),
                        maps.get(6).is_none(),
                    )
                };
                assert_eq!((rows, active_maps, inactive_rows), (5, 3, 2));
                assert!(!inactive_selected);
                assert!(omitted_unknown);
                verify_v28_coverage(
                    &fixture.coverage,
                    authentication,
                    &[],
                    std::slice::from_ref(&fixture.declaration),
                )
                .unwrap();
                assert_eq!(fixture.coverage.map.materialized_rows, 5);
                assert_eq!(
                    fixture
                        .coverage
                        .maps
                        .iter()
                        .map(|map| map.id)
                        .collect::<Vec<_>>(),
                    [1, 2, 3]
                );
                for id in [1, 2, 3] {
                    let (tree, proof) = geometry_paths(id);
                    assert!(fixture.files.iter().any(|(path, _)| path == &tree));
                    assert!(fixture.files.iter().any(|(path, _)| path == &proof));
                }
                for id in [4, 5] {
                    let prefix = format!("vmaps/{id:04}/");
                    assert!(
                        !fixture
                            .files
                            .iter()
                            .any(|(path, _)| path.starts_with(&prefix))
                    );
                }
            }

            #[test]
            fn sealed_all_available_geometry_replays_populated_and_empty_maps() {
                let lab = Lab::new();
                let fixture = available_fixture();
                assert_eq!(fixture.declaration.sections.len(), 2);
                assert!(
                    fixture
                        .files
                        .iter()
                        .any(|(path, _)| path.ends_with(".vmtile"))
                );
                let output = fixture.publish(&lab, "package");
                let summary = verify_selected_geometry_v29(&fixture.plan, &output, None).unwrap();
                assert_eq!(summary.context, fixture.coverage.context);
                assert_eq!(summary.files, 11);
            }

            #[test]
            fn rejects_a_missing_proof_for_an_empty_active_map() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                let (_, proof) = geometry_paths(2);
                fixture.files.retain(|(path, _)| path != &proof);
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                let output = fixture.publish(&lab, "missing-empty-proof");
                assert_eq!(rejection(&fixture, &output).code, BAD);
            }

            #[test]
            fn rejects_an_id_unknown_because_its_section_was_omitted() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.coverage.maps.push(geometry_v22::DomainMapFact {
                    id: 6,
                    parent: None,
                    wdt_file_data_id: 106,
                    depth: 0,
                    is_parent: false,
                    root_absent: false,
                });
                push_map(&mut fixture.files, 6, &[]);
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                assert_eq!(fixture.authenticate().unwrap_err().code, BAD);
                let output = fixture.publish(&lab, "omitted-id");
                assert_eq!(rejection(&fixture, &output).code, BAD);
            }

            #[test]
            fn rejects_reduced_materialized_rows() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.coverage.map.materialized_rows = 4;
                let output = fixture.publish(&lab, "reduced-materialized-rows");
                assert_eq!(rejection(&fixture, &output).code, BAD);
            }

            #[test]
            fn rejects_a_wdt_zero_row_presented_as_an_empty_active_map() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.coverage.maps.push(geometry_v22::DomainMapFact {
                    id: 4,
                    parent: None,
                    wdt_file_data_id: 0,
                    depth: 0,
                    is_parent: false,
                    root_absent: false,
                });
                push_map(&mut fixture.files, 4, &[]);
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                assert_eq!(fixture.authenticate().unwrap_err().code, BAD);
                let output = fixture.publish(&lab, "wdt-zero-empty-map");
                assert_eq!(
                    rejection(&fixture, &output).code,
                    "product_selected_geometry_coverage_schema"
                );
            }

            #[test]
            fn accepts_a_parent_payload_shared_by_multiple_inherited_tiles() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.files.clear();
                push_map(&mut fixture.files, 1, &owned_placement());
                let mut inherited = owned_placement();
                inherited[0] |= 2;
                push_map(&mut fixture.files, 2, &inherited);
                push_map(&mut fixture.files, 3, &inherited);
                fixture
                    .files
                    .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                fixture.authenticate().unwrap();
                let output = fixture.publish(&lab, "shared-active-parent-payload");
                assert_eq!(
                    verify_selected_geometry_v29(&fixture.plan, &output, None)
                        .unwrap()
                        .files,
                    13
                );
            }

            #[test]
            fn rejects_an_inherited_tile_bound_to_an_inactive_parent() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.coverage.maps[1].is_parent = false;
                fixture.coverage.maps[2].parent = Some(4);
                fixture.coverage.maps[2].depth = 1;
                let data_start =
                    u32::from_le_bytes(fixture.map[212..216].try_into().unwrap()) as usize;
                let stride = u32::from_le_bytes(fixture.map[144..148].try_into().unwrap()) as usize;
                let parent_at = data_start + 2 * stride + field_offset(13);
                fixture.map[parent_at..parent_at + 2].copy_from_slice(&4_i16.to_le_bytes());
                fixture.files.clear();
                push_map(&mut fixture.files, 1, &owned_placement());
                push_map(&mut fixture.files, 2, &[]);
                push_map(&mut fixture.files, 4, &owned_placement());
                let (_, inactive_proof) = geometry_paths(4);
                fixture.files.retain(|(path, _)| {
                    path.ends_with(".vmtile")
                        || (!path.starts_with("vmaps/0004/") && path != &inactive_proof)
                });
                let mut inherited = owned_placement();
                inherited[0] |= 2;
                push_map(&mut fixture.files, 3, &inherited);
                fixture
                    .files
                    .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                fixture.coverage.artifacts = artifact_facts(&fixture.files);
                fixture.authenticate().unwrap();
                let owned = owned_placement();
                let parent_records: Vec<_> =
                    placement::DirBin::parse(&owned, placement::Limits::default())
                        .unwrap()
                        .records()
                        .collect();
                let parent = assembly::build(4, &parent_records, Default::default()).unwrap();
                let child_records: Vec<_> =
                    placement::DirBin::parse(&inherited, placement::Limits::default())
                        .unwrap()
                        .records()
                        .collect();
                let child = assembly::build(3, &child_records, Default::default()).unwrap();
                assembly::verify_inherited_associations(
                    parent.tiles()[0].payload().unwrap(),
                    child.tiles()[0].index_bytes(),
                    child.ordered_dir_bin(),
                    Default::default(),
                )
                .unwrap();
                let output = fixture.publish(&lab, "inactive-parent-payload");
                assert_eq!(rejection(&fixture, &output).code, BAD);
            }

            #[test]
            fn rejects_map_db2_as_a_collision_coverage_artifact() {
                let lab = Lab::new();
                let mut fixture = available_fixture();
                fixture.coverage.artifacts.push(ArtifactFact {
                    path: format!("dbc/{}/Map.db2", fixture.plan.locale.name()),
                    bytes: fixture.map.len() as u64,
                    sha: integrity::digest(&fixture.map),
                });
                fixture
                    .coverage
                    .artifacts
                    .sort_unstable_by(|left, right| left.path.cmp(&right.path));
                let output = fixture.publish(&lab, "map-in-collision-inventory");
                assert_eq!(
                    rejection(&fixture, &output).code,
                    "product_selected_geometry_coverage_schema"
                );
            }

            #[test]
            fn rejects_missing_and_global_partial_declarations() {
                let lab = Lab::new();
                let missing = available_fixture();
                let output = missing.publish_with_declarations(&lab, "missing", &[], &[]);
                assert_eq!(
                    rejection(&missing, &output).code,
                    "product_partial_undeclared"
                );

                let global = available_fixture();
                let error = verify_v28_coverage(
                    &global.coverage,
                    authenticate_v28_map(&global.map).unwrap(),
                    &[section(0, 1)],
                    std::slice::from_ref(&global.declaration),
                )
                .unwrap_err();
                assert_eq!(error.code, "product_partial_manifest_scope");
            }

            #[test]
            fn rejects_crossed_versions_masks_and_profiles() {
                let lab = Lab::new();
                let selected = super::fixture();
                let available = available_fixture();
                assert_eq!(
                    geometry_v22::Coverage::parse_v29(&selected.coverage.encode().unwrap())
                        .err()
                        .unwrap()
                        .code,
                    "product_selected_geometry_coverage_schema"
                );
                assert_eq!(
                    geometry_v22::Coverage::parse_v28(&available.coverage.encode().unwrap())
                        .err()
                        .unwrap()
                        .code,
                    "product_selected_geometry_coverage_schema"
                );

                let mut wrong_mask = available_fixture();
                wrong_mask.coverage.context.components = Component::Collision.bit();
                assert_eq!(
                    geometry_v22::Coverage::parse_v29(&wrong_mask.coverage.encode().unwrap())
                        .err()
                        .unwrap()
                        .code,
                    "product_selected_geometry_coverage_schema"
                );

                let (v22, _, _) = empty_geometry_fixture();
                let missing = Path::new("/nonexistent-available-geometry-v29");
                for error in [
                    verify_selected_geometry_v29(&selected.plan, missing, None).unwrap_err(),
                    verify_selected_geometry_v29(&v22, missing, None).unwrap_err(),
                    verify_selected_geometry_v28(&available.plan, missing, None).unwrap_err(),
                    verify_selected_geometry_v22(&available.plan, missing, None).unwrap_err(),
                ] {
                    assert_eq!(error.code, "product_plan_component_scope");
                }
                let mut wrong_components = available.plan.clone();
                wrong_components.components = Component::Collision.bit();
                assert_eq!(
                    verify_selected_geometry_v29(&wrong_components, missing, None)
                        .unwrap_err()
                        .code,
                    "product_plan_component_scope"
                );

                let mut v1_for_v29 = available_fixture();
                v1_for_v29.coverage.selection = MapSelection::MapAndAncestors(2);
                let output = v1_for_v29.publish(&lab, "v1-for-v29");
                rejection(&v1_for_v29, &output);

                let mut v2_for_v28 = super::fixture();
                v2_for_v28.coverage.selection = MapSelection::All;
                let output = v2_for_v28.publish(&lab, "v2-for-v28");
                super::rejection(&v2_for_v28, &output);
            }

            mod v30 {
                use super::*;
                use forever_casc::collision::run::MissingWdtPolicy;

                const V30: &str = include_str!(
                    "../../../../tests/contracts/fixtures/forever-product-v30/all.tsv"
                );

                fn root_absent_fixture() -> V28Fixture {
                    let mut fixture = available_fixture();
                    fixture.plan = Plan::parse(V30).unwrap();
                    fixture.declaration.path =
                        format!("dbc/{}/Map.db2", fixture.plan.locale.name());
                    fixture.coverage.context = Context {
                        build: 70450,
                        locale: fixture.plan.locale,
                        components: Component::Db2.bit() | Component::Collision.bit(),
                        source_sha256: [0x41; 32],
                        plan_sha256: integrity::digest(fixture.plan.render().as_bytes()),
                        producer_sha256: [0x42; 32],
                    };
                    fixture.coverage.missing_wdt_policy = MissingWdtPolicy::DeclareRootAbsent;

                    let data_start =
                        u32::from_le_bytes(fixture.map[212..216].try_into().unwrap()) as usize;
                    let stride =
                        u32::from_le_bytes(fixture.map[144..148].try_into().unwrap()) as usize;
                    let string_bytes =
                        u32::from_le_bytes(fixture.map[148..152].try_into().unwrap()) as usize;
                    let copy_source_at = data_start + 4 * stride + string_bytes + 16 + 4;
                    fixture.map[copy_source_at..copy_source_at + 4]
                        .copy_from_slice(&3_u32.to_le_bytes());

                    fixture.coverage.maps = vec![
                        geometry_v22::DomainMapFact {
                            id: 1,
                            parent: None,
                            wdt_file_data_id: 101,
                            depth: 0,
                            is_parent: true,
                            root_absent: false,
                        },
                        geometry_v22::DomainMapFact {
                            id: 2,
                            parent: Some(1),
                            wdt_file_data_id: 102,
                            depth: 1,
                            is_parent: true,
                            root_absent: true,
                        },
                        geometry_v22::DomainMapFact {
                            id: 3,
                            parent: Some(2),
                            wdt_file_data_id: 103,
                            depth: 2,
                            is_parent: false,
                            root_absent: false,
                        },
                        geometry_v22::DomainMapFact {
                            id: 5,
                            parent: Some(2),
                            wdt_file_data_id: 103,
                            depth: 2,
                            is_parent: false,
                            root_absent: true,
                        },
                    ];
                    fixture.files.clear();
                    push_map(&mut fixture.files, 1, &owned_placement());
                    push_map(&mut fixture.files, 3, &[]);
                    fixture
                        .files
                        .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                    fixture.coverage.artifacts = artifact_facts(&fixture.files);
                    fixture
                }

                fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                    match verify_selected_geometry_v30(&fixture.plan, output, None) {
                        Ok(_) => panic!("mutated v30 package unexpectedly passed"),
                        Err(error) => error,
                    }
                }

                fn assert_no_map_files(fixture: &V28Fixture, id: u32) {
                    let (tree, proof) = geometry_paths(id);
                    let prefix = format!("vmaps/{id:04}/");
                    assert!(
                        !fixture
                            .files
                            .iter()
                            .any(|(path, _)| { path == &proof || path.starts_with(&prefix) }),
                        "ROOT-absent map {id} unexpectedly retained {tree}"
                    );
                }

                #[test]
                fn sealed_root_absence_replays_only_present_maps_and_keeps_full_active_domain() {
                    let lab = Lab::new();
                    let fixture = root_absent_fixture();
                    assert_eq!(fixture.plan.map_domain, None);
                    let authentication = authenticate_v28_map(&fixture.map).unwrap();
                    let (rows, active_maps, inactive_rows, unknown_omitted_id) = {
                        let maps = Maps::from_materialized_selection(
                            authentication.owner.rows(),
                            authentication.owner.direct_records().len(),
                            MapSelection::All,
                            Default::default(),
                        )
                        .unwrap();
                        (
                            maps.stats().rows,
                            maps.stats().active_maps,
                            maps.all().filter(|entry| !entry.active()).count(),
                            maps.get(6).is_none(),
                        )
                    };
                    assert_eq!((rows, active_maps, inactive_rows), (5, 4, 1));
                    assert!(unknown_omitted_id);
                    verify_v28_coverage(
                        &fixture.coverage,
                        authentication,
                        &[],
                        std::slice::from_ref(&fixture.declaration),
                    )
                    .unwrap();
                    assert_eq!(fixture.coverage.maps.len(), active_maps);
                    assert_eq!(
                        fixture
                            .coverage
                            .maps
                            .iter()
                            .filter(|map| map.root_absent)
                            .map(|map| map.id)
                            .collect::<Vec<_>>(),
                        [2, 5]
                    );
                    for id in [2, 5] {
                        assert_no_map_files(&fixture, id);
                    }
                    for id in [1, 3] {
                        let (tree, proof) = geometry_paths(id);
                        assert!(fixture.files.iter().any(|(path, _)| path == &tree));
                        assert!(fixture.files.iter().any(|(path, _)| path == &proof));
                    }

                    let output = fixture.publish(&lab, "package");
                    let summary =
                        verify_selected_geometry_v30(&fixture.plan, &output, None).unwrap();
                    assert_eq!(summary.context, fixture.coverage.context);
                    assert_eq!(summary.files, 9);
                    assert!(
                        output
                            .join(geometry_v22::ROOT_ABSENT_COVERAGE_PATH)
                            .is_file()
                    );
                    assert!(!output.join(geometry_v22::AVAILABLE_COVERAGE_PATH).exists());
                }

                #[test]
                fn rejects_missing_and_extra_root_omissions() {
                    let lab = Lab::new();
                    let mut missing = root_absent_fixture();
                    missing.coverage.maps[1].root_absent = false;
                    let output = missing.publish(&lab, "missing-omission");
                    assert_eq!(rejection(&missing, &output).code, BAD);

                    let mut extra = root_absent_fixture();
                    extra.coverage.maps.push(geometry_v22::DomainMapFact {
                        id: 6,
                        parent: None,
                        wdt_file_data_id: 106,
                        depth: 0,
                        is_parent: false,
                        root_absent: true,
                    });
                    assert_eq!(extra.authenticate().unwrap_err().code, BAD);
                    let output = extra.publish(&lab, "extra-omission");
                    assert_eq!(rejection(&extra, &output).code, BAD);
                }

                #[test]
                fn rejects_wrong_fdid_and_overlapping_root_omissions() {
                    let lab = Lab::new();
                    let mut wrong_fdid = root_absent_fixture();
                    wrong_fdid.coverage.maps[1].wdt_file_data_id += 1;
                    assert_eq!(wrong_fdid.authenticate().unwrap_err().code, BAD);
                    let output = wrong_fdid.publish(&lab, "wrong-fdid");
                    assert_eq!(rejection(&wrong_fdid, &output).code, BAD);

                    let mut overlapping = root_absent_fixture();
                    let omitted = overlapping.coverage.maps[1];
                    overlapping.coverage.maps.insert(1, omitted);
                    let output = overlapping.publish(&lab, "overlapping-omission");
                    assert_eq!(
                        rejection(&overlapping, &output).code,
                        "product_selected_geometry_coverage_schema"
                    );
                }

                #[test]
                fn rejects_false_omission_and_any_artifacts_for_an_omitted_map() {
                    let lab = Lab::new();
                    let mut false_omission = root_absent_fixture();
                    false_omission.coverage.maps[0].root_absent = true;
                    let output = false_omission.publish(&lab, "false-omission");
                    assert_eq!(rejection(&false_omission, &output).code, BAD);

                    let mut artifacts = root_absent_fixture();
                    push_map(&mut artifacts.files, 2, &owned_placement());
                    artifacts.coverage.artifacts = artifact_facts(&artifacts.files);
                    artifacts.authenticate().unwrap();
                    let output = artifacts.publish(&lab, "omitted-artifacts");
                    assert_eq!(rejection(&artifacts, &output).code, BAD);
                }

                #[test]
                fn rejects_a_present_empty_map_without_proof() {
                    let lab = Lab::new();
                    let mut fixture = root_absent_fixture();
                    let (_, proof) = geometry_paths(3);
                    fixture.files.retain(|(path, _)| path != &proof);
                    fixture.coverage.artifacts = artifact_facts(&fixture.files);
                    let output = fixture.publish(&lab, "present-empty-no-proof");
                    assert_eq!(rejection(&fixture, &output).code, BAD);
                }

                #[test]
                fn accepts_valid_inheritance_from_a_present_parent() {
                    let lab = Lab::new();
                    let mut fixture = root_absent_fixture();
                    fixture.coverage.maps[1].root_absent = false;
                    fixture.files.clear();
                    push_map(&mut fixture.files, 1, &[]);
                    push_map(&mut fixture.files, 2, &owned_placement());
                    let mut inherited = owned_placement();
                    inherited[0] |= 2;
                    push_map(&mut fixture.files, 3, &inherited);
                    fixture
                        .files
                        .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                    fixture.coverage.artifacts = artifact_facts(&fixture.files);
                    fixture.authenticate().unwrap();
                    let output = fixture.publish(&lab, "present-parent-inheritance");
                    assert_eq!(
                        verify_selected_geometry_v30(&fixture.plan, &output, None)
                            .unwrap()
                            .files,
                        12
                    );
                }

                #[test]
                fn rejects_inheritance_through_root_absent_parent_even_with_grandparent_payload() {
                    let lab = Lab::new();
                    let mut fixture = root_absent_fixture();
                    fixture.files.clear();
                    let owned = owned_placement();
                    push_map(&mut fixture.files, 1, &owned);
                    let mut inherited = owned_placement();
                    inherited[0] |= 2;
                    push_map(&mut fixture.files, 3, &inherited);
                    fixture
                        .files
                        .push(("vmaps/synthetic.vmo".into(), synthetic_vmo()));
                    fixture.coverage.artifacts = artifact_facts(&fixture.files);
                    fixture.authenticate().unwrap();

                    let parent_records: Vec<_> =
                        placement::DirBin::parse(&owned, placement::Limits::default())
                            .unwrap()
                            .records()
                            .collect();
                    let parent = assembly::build(1, &parent_records, Default::default()).unwrap();
                    let child_records: Vec<_> =
                        placement::DirBin::parse(&inherited, placement::Limits::default())
                            .unwrap()
                            .records()
                            .collect();
                    let child = assembly::build(3, &child_records, Default::default()).unwrap();
                    assert!(child.tiles()[0].payload().is_none());
                    assembly::verify_inherited_associations(
                        parent.tiles()[0].payload().unwrap(),
                        child.tiles()[0].index_bytes(),
                        child.ordered_dir_bin(),
                        Default::default(),
                    )
                    .unwrap();

                    let output = fixture.publish(&lab, "root-absent-parent-inheritance");
                    assert_eq!(rejection(&fixture, &output).code, BAD);
                }

                #[test]
                fn rejects_crossed_v29_v30_versions_masks_and_profiles() {
                    let lab = Lab::new();
                    let available = available_fixture();
                    let root_absent = root_absent_fixture();
                    assert_eq!(
                        geometry_v22::Coverage::parse_v30(&available.coverage.encode().unwrap())
                            .err()
                            .unwrap()
                            .code,
                        "product_selected_geometry_coverage_schema"
                    );
                    assert_eq!(
                        geometry_v22::Coverage::parse_v29(&root_absent.coverage.encode().unwrap())
                            .err()
                            .unwrap()
                            .code,
                        "product_selected_geometry_coverage_schema"
                    );

                    let mut wrong_mask = root_absent_fixture();
                    wrong_mask.coverage.context.components = Component::Collision.bit();
                    assert_eq!(
                        geometry_v22::Coverage::parse_v30(&wrong_mask.coverage.encode().unwrap())
                            .err()
                            .unwrap()
                            .code,
                        "product_selected_geometry_coverage_schema"
                    );

                    let missing = Path::new("/nonexistent-root-absent-geometry-v30");
                    for error in [
                        verify_selected_geometry_v30(&available.plan, missing, None).unwrap_err(),
                        verify_selected_geometry_v29(&root_absent.plan, missing, None).unwrap_err(),
                    ] {
                        assert_eq!(error.code, "product_plan_component_scope");
                    }
                    let mut wrong_components = root_absent.plan.clone();
                    wrong_components.components = Component::Collision.bit();
                    assert_eq!(
                        verify_selected_geometry_v30(&wrong_components, missing, None)
                            .unwrap_err()
                            .code,
                        "product_plan_component_scope"
                    );
                    let mut selected = root_absent.plan.clone();
                    selected.map_domain = Some(2);
                    assert_eq!(
                        verify_selected_geometry_v30(&selected, missing, None)
                            .unwrap_err()
                            .code,
                        "product_plan_component_scope"
                    );

                    let mut v2_for_v30 = root_absent_fixture();
                    v2_for_v30.coverage.missing_wdt_policy = MissingWdtPolicy::Reject;
                    for map in &mut v2_for_v30.coverage.maps {
                        map.root_absent = false;
                    }
                    let output = v2_for_v30.publish(&lab, "v2-for-v30");
                    rejection(&v2_for_v30, &output);

                    let mut v3_for_v29 = available_fixture();
                    v3_for_v29.coverage.missing_wdt_policy = MissingWdtPolicy::DeclareRootAbsent;
                    let output = v3_for_v29.publish(&lab, "v3-for-v29");
                    super::rejection(&v3_for_v29, &output);
                }

                mod v31 {
                    use super::*;
                    use forever_casc::collision::run::MissingModelPolicy;

                    const V31: &str = include_str!(
                        "../../../../tests/contracts/fixtures/forever-product-v31/all.tsv"
                    );
                    const OMITTED: u32 = 0x1122_3344;
                    const PRESENT: u32 = 0x5566_7788;

                    fn named_placement(id: u32, name: &[u8]) -> Vec<u8> {
                        let mut bytes = vec![1, 7];
                        for value in [
                            id,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            1.0_f32.to_bits(),
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            name.len() as u32,
                        ] {
                            bytes.extend_from_slice(&value.to_le_bytes());
                        }
                        bytes.extend_from_slice(name);
                        bytes
                    }

                    fn present_name() -> String {
                        geometry_v22::m2_file_name(PRESENT)
                    }

                    fn omitted_name() -> String {
                        geometry_v22::m2_file_name(OMITTED)
                    }

                    fn present_proof() -> Vec<u8> {
                        [
                            named_placement(17, b"wmo-parent"),
                            named_placement(18, present_name().as_bytes()),
                        ]
                        .concat()
                    }

                    fn inherited_proof() -> Vec<u8> {
                        let mut parent = named_placement(17, b"wmo-parent");
                        parent[0] |= 2;
                        let mut present = named_placement(18, present_name().as_bytes());
                        present[0] |= 2;
                        [parent, present].concat()
                    }

                    fn model_vmo(name: &str) -> (String, Vec<u8>) {
                        (format!("vmaps/{name}.vmo"), synthetic_vmo())
                    }

                    fn push_present_models(files: &mut Vec<(String, Vec<u8>)>) {
                        files.push(model_vmo("wmo-parent"));
                        files.push(model_vmo(&present_name()));
                    }

                    fn model_fixture() -> V28Fixture {
                        let mut fixture = root_absent_fixture();
                        fixture.plan = Plan::parse(V31).unwrap();
                        fixture.declaration.path =
                            format!("dbc/{}/Map.db2", fixture.plan.locale.name());
                        fixture.coverage.context = Context {
                            build: 70450,
                            locale: fixture.plan.locale,
                            components: Component::Db2.bit() | Component::Collision.bit(),
                            source_sha256: [0x41; 32],
                            plan_sha256: integrity::digest(fixture.plan.render().as_bytes()),
                            producer_sha256: [0x42; 32],
                        };
                        fixture.coverage.missing_m2_policy =
                            MissingModelPolicy::DeclareRootAbsentWmoDoodadM2;
                        fixture.coverage.missing_m2 = vec![OMITTED];
                        fixture.files.clear();
                        push_map(&mut fixture.files, 1, &present_proof());
                        push_map(&mut fixture.files, 3, &[]);
                        push_present_models(&mut fixture.files);
                        fixture.coverage.artifacts = artifact_facts(&fixture.files);
                        fixture
                    }

                    fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                        match verify_selected_geometry_v31(&fixture.plan, output, None) {
                            Ok(_) => panic!("mutated v31 package unexpectedly passed"),
                            Err(error) => error,
                        }
                    }

                    #[test]
                    fn sealed_wmo_doodad_omission_preserves_parent_and_present_models() {
                        let lab = Lab::new();
                        let fixture = model_fixture();
                        assert_eq!(fixture.plan.map_domain, None);
                        assert_eq!(fixture.coverage.missing_m2, [OMITTED]);
                        fixture.authenticate().unwrap();
                        let encoded = fixture.coverage.encode().unwrap();
                        let parsed = geometry_v22::Coverage::parse_v31(&encoded).unwrap();
                        assert_eq!(
                            parsed.missing_m2_policy,
                            MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
                        );
                        assert_eq!(parsed.missing_m2, [OMITTED]);

                        let (_, proof) = geometry_paths(1);
                        let proof_bytes = fixture
                            .files
                            .iter()
                            .find(|(path, _)| path == &proof)
                            .unwrap()
                            .1
                            .as_slice();
                        let placements =
                            placement::DirBin::parse(proof_bytes, Default::default()).unwrap();
                        let names: Vec<_> = placements
                            .records()
                            .map(|record| record.name().to_vec())
                            .collect();
                        assert!(names.contains(&b"wmo-parent".to_vec()));
                        assert!(names.contains(&present_name().into_bytes()));
                        assert!(!names.contains(&omitted_name().into_bytes()));
                        assert!(
                            !fixture.files.iter().any(|(path, _)| {
                                path == &format!("vmaps/{}.vmo", omitted_name())
                            })
                        );

                        let output = fixture.publish(&lab, "package");
                        let summary =
                            verify_selected_geometry_v31(&fixture.plan, &output, None).unwrap();
                        assert_eq!(summary.context, fixture.coverage.context);
                        assert_eq!(summary.files, 10);
                        assert!(
                            output
                                .join(geometry_v22::MODEL_ROOT_ABSENT_COVERAGE_PATH)
                                .is_file()
                        );
                        assert!(
                            !output
                                .join(geometry_v22::ROOT_ABSENT_COVERAGE_PATH)
                                .exists()
                        );
                    }

                    #[test]
                    fn rejects_omitted_doodad_vmo_and_any_derived_placement() {
                        let lab = Lab::new();
                        let mut vmo = model_fixture();
                        vmo.files.push(model_vmo(&omitted_name()));
                        vmo.coverage.artifacts = artifact_facts(&vmo.files);
                        let output = vmo.publish(&lab, "omitted-vmo");
                        assert_eq!(rejection(&vmo, &output).code, BAD);

                        let mut placement = model_fixture();
                        placement.files.clear();
                        let proof = [
                            present_proof(),
                            named_placement(19, omitted_name().as_bytes()),
                        ]
                        .concat();
                        push_map(&mut placement.files, 1, &proof);
                        push_map(&mut placement.files, 3, &[]);
                        push_present_models(&mut placement.files);
                        placement.coverage.artifacts = artifact_facts(&placement.files);
                        placement.authenticate().unwrap();
                        let output = placement.publish(&lab, "omitted-placement");
                        assert_eq!(rejection(&placement, &output).code, BAD);
                    }

                    #[test]
                    fn rejects_zero_duplicate_unordered_and_overlapping_missing_fdids() {
                        for missing in [vec![0], vec![7, 7], vec![8, 7]] {
                            let mut fixture = model_fixture();
                            fixture.coverage.missing_m2 = missing;
                            assert_eq!(fixture.authenticate().unwrap_err().code, BAD);
                            assert!(
                                geometry_v22::Coverage::parse_v31(
                                    &fixture.coverage.encode().unwrap()
                                )
                                .is_err()
                            );
                        }
                    }

                    #[test]
                    fn rejects_omitted_map_artifacts_and_present_empty_map_without_proof() {
                        let lab = Lab::new();
                        let mut omitted = model_fixture();
                        push_map(&mut omitted.files, 2, &present_proof());
                        omitted.coverage.artifacts = artifact_facts(&omitted.files);
                        omitted.authenticate().unwrap();
                        let output = omitted.publish(&lab, "omitted-map-artifacts");
                        assert_eq!(rejection(&omitted, &output).code, BAD);

                        let mut empty = model_fixture();
                        let (_, proof) = geometry_paths(3);
                        empty.files.retain(|(path, _)| path != &proof);
                        empty.coverage.artifacts = artifact_facts(&empty.files);
                        let output = empty.publish(&lab, "present-empty-no-proof");
                        assert_eq!(rejection(&empty, &output).code, BAD);
                    }

                    #[test]
                    fn accepts_valid_inheritance_while_preserving_wmo_parent() {
                        let lab = Lab::new();
                        let mut fixture = model_fixture();
                        fixture.coverage.maps[1].root_absent = false;
                        fixture.files.clear();
                        push_map(&mut fixture.files, 1, &[]);
                        push_map(&mut fixture.files, 2, &present_proof());
                        push_map(&mut fixture.files, 3, &inherited_proof());
                        push_present_models(&mut fixture.files);
                        fixture.coverage.artifacts = artifact_facts(&fixture.files);
                        fixture.authenticate().unwrap();
                        let output = fixture.publish(&lab, "valid-wmo-inheritance");
                        assert_eq!(
                            verify_selected_geometry_v31(&fixture.plan, &output, None)
                                .unwrap()
                                .files,
                            13
                        );
                    }

                    #[test]
                    fn rejects_inheritance_through_root_absent_parent_with_grandparent_payload() {
                        let lab = Lab::new();
                        let mut fixture = model_fixture();
                        fixture.files.clear();
                        let parent_proof = present_proof();
                        push_map(&mut fixture.files, 1, &parent_proof);
                        let child_proof = inherited_proof();
                        push_map(&mut fixture.files, 3, &child_proof);
                        push_present_models(&mut fixture.files);
                        fixture.coverage.artifacts = artifact_facts(&fixture.files);
                        fixture.authenticate().unwrap();

                        let parent_records: Vec<_> =
                            placement::DirBin::parse(&parent_proof, placement::Limits::default())
                                .unwrap()
                                .records()
                                .collect();
                        let parent =
                            assembly::build(1, &parent_records, Default::default()).unwrap();
                        let child_records: Vec<_> =
                            placement::DirBin::parse(&child_proof, placement::Limits::default())
                                .unwrap()
                                .records()
                                .collect();
                        let child = assembly::build(3, &child_records, Default::default()).unwrap();
                        assert!(child.tiles()[0].payload().is_none());
                        assembly::verify_inherited_associations(
                            parent.tiles()[0].payload().unwrap(),
                            child.tiles()[0].index_bytes(),
                            child.ordered_dir_bin(),
                            Default::default(),
                        )
                        .unwrap();

                        let output = fixture.publish(&lab, "root-absent-wmo-parent");
                        assert_eq!(rejection(&fixture, &output).code, BAD);
                    }

                    #[test]
                    fn rejects_crossed_v30_v31_versions_masks_and_profiles() {
                        let lab = Lab::new();
                        let root_absent = root_absent_fixture();
                        let model_root_absent = model_fixture();
                        assert_eq!(
                            geometry_v22::Coverage::parse_v31(
                                &root_absent.coverage.encode().unwrap()
                            )
                            .err()
                            .unwrap()
                            .code,
                            "product_selected_geometry_coverage_schema"
                        );
                        assert_eq!(
                            geometry_v22::Coverage::parse_v30(
                                &model_root_absent.coverage.encode().unwrap()
                            )
                            .err()
                            .unwrap()
                            .code,
                            "product_selected_geometry_coverage_schema"
                        );

                        let mut wrong_mask = model_fixture();
                        wrong_mask.coverage.context.components = Component::Collision.bit();
                        assert_eq!(
                            geometry_v22::Coverage::parse_v31(
                                &wrong_mask.coverage.encode().unwrap()
                            )
                            .err()
                            .unwrap()
                            .code,
                            "product_selected_geometry_coverage_schema"
                        );

                        let missing = Path::new("/nonexistent-model-root-geometry-v31");
                        for error in [
                            verify_selected_geometry_v31(&root_absent.plan, missing, None)
                                .unwrap_err(),
                            verify_selected_geometry_v30(&model_root_absent.plan, missing, None)
                                .unwrap_err(),
                        ] {
                            assert_eq!(error.code, "product_plan_component_scope");
                        }
                        let mut wrong_components = model_root_absent.plan.clone();
                        wrong_components.components = Component::Collision.bit();
                        assert_eq!(
                            verify_selected_geometry_v31(&wrong_components, missing, None)
                                .unwrap_err()
                                .code,
                            "product_plan_component_scope"
                        );
                        let mut selected = model_root_absent.plan.clone();
                        selected.map_domain = Some(2);
                        assert_eq!(
                            verify_selected_geometry_v31(&selected, missing, None)
                                .unwrap_err()
                                .code,
                            "product_plan_component_scope"
                        );

                        let mut v3_for_v31 = model_fixture();
                        v3_for_v31.coverage.missing_m2_policy = MissingModelPolicy::Reject;
                        v3_for_v31.coverage.missing_m2.clear();
                        let output = v3_for_v31.publish(&lab, "v3-for-v31");
                        rejection(&v3_for_v31, &output);

                        let mut v4_for_v30 = root_absent_fixture();
                        v4_for_v30.coverage.missing_m2_policy =
                            MissingModelPolicy::DeclareRootAbsentWmoDoodadM2;
                        let output = v4_for_v30.publish(&lab, "v4-for-v30");
                        super::rejection(&v4_for_v30, &output);
                    }

                    mod v32 {
                        use super::*;
                        use forever_casc::collision::resources;

                        const V32: &str = include_str!(
                            "../../../../tests/contracts/fixtures/forever-product-v32/all.tsv"
                        );

                        fn request_fixture() -> V28Fixture {
                            let mut fixture = model_fixture();
                            fixture.plan = Plan::parse(V32).unwrap();
                            fixture.declaration.path =
                                format!("dbc/{}/Map.db2", fixture.plan.locale.name());
                            fixture.coverage.context = Context {
                                build: 70450,
                                locale: fixture.plan.locale,
                                components: Component::Db2.bit() | Component::Collision.bit(),
                                source_sha256: [0x41; 32],
                                plan_sha256: integrity::digest(fixture.plan.render().as_bytes()),
                                producer_sha256: [0x42; 32],
                            };
                            fixture.coverage.resource_profile = resources::Profile::RequestHeavyV1;
                            fixture
                        }

                        fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                            match verify_selected_geometry_v32(&fixture.plan, output, None) {
                                Ok(_) => panic!("mutated v32 package unexpectedly passed"),
                                Err(error) => error,
                            }
                        }

                        fn replace_text(bytes: &[u8], text: &str, replacement: &str) -> Vec<u8> {
                            let needle = text.as_bytes();
                            let at = bytes
                                .windows(needle.len())
                                .position(|window| window == needle)
                                .expect("coverage text");
                            let mut mutated = bytes[..at - 4].to_vec();
                            mutated.extend_from_slice(&(replacement.len() as u32).to_le_bytes());
                            mutated.extend_from_slice(replacement.as_bytes());
                            mutated.extend_from_slice(&bytes[at + needle.len()..]);
                            mutated
                        }

                        #[test]
                        fn sealed_request_capacity_preserves_wmo_parent_and_present_doodad() {
                            let lab = Lab::new();
                            let fixture = request_fixture();
                            assert_eq!(fixture.plan.map_domain, None);
                            assert_eq!(
                                fixture.coverage.resource_profile,
                                resources::Profile::RequestHeavyV1
                            );
                            fixture.authenticate().unwrap();
                            let parsed = geometry_v22::Coverage::parse_v32(
                                &fixture.coverage.encode().unwrap(),
                            )
                            .unwrap();
                            assert_eq!(parsed.resource_profile, resources::Profile::RequestHeavyV1);
                            assert_eq!(parsed.missing_m2, [OMITTED]);
                            let output = fixture.publish(&lab, "package");
                            let summary =
                                verify_selected_geometry_v32(&fixture.plan, &output, None).unwrap();
                            assert_eq!(summary.context, fixture.coverage.context);
                            assert_eq!(summary.files, 10);
                            assert!(
                                output
                                    .join(geometry_v22::REQUEST_CAPACITY_COVERAGE_PATH)
                                    .is_file()
                            );
                            assert!(
                                !output
                                    .join(geometry_v22::MODEL_ROOT_ABSENT_COVERAGE_PATH)
                                    .exists()
                            );
                        }

                        #[test]
                        fn rejects_omitted_doodad_vmo_and_derived_placement() {
                            let lab = Lab::new();
                            let mut vmo = request_fixture();
                            vmo.files.push(model_vmo(&omitted_name()));
                            vmo.coverage.artifacts = artifact_facts(&vmo.files);
                            let output = vmo.publish(&lab, "omitted-vmo");
                            assert_eq!(rejection(&vmo, &output).code, BAD);

                            let mut placement = request_fixture();
                            placement.files.clear();
                            let proof = [
                                present_proof(),
                                named_placement(19, omitted_name().as_bytes()),
                            ]
                            .concat();
                            push_map(&mut placement.files, 1, &proof);
                            push_map(&mut placement.files, 3, &[]);
                            push_present_models(&mut placement.files);
                            placement.coverage.artifacts = artifact_facts(&placement.files);
                            placement.authenticate().unwrap();
                            let output = placement.publish(&lab, "omitted-placement");
                            assert_eq!(rejection(&placement, &output).code, BAD);
                        }

                        #[test]
                        fn accepts_valid_inheritance_and_rejects_root_absent_parent() {
                            let lab = Lab::new();
                            let mut valid = request_fixture();
                            valid.coverage.maps[1].root_absent = false;
                            valid.files.clear();
                            push_map(&mut valid.files, 1, &[]);
                            push_map(&mut valid.files, 2, &present_proof());
                            push_map(&mut valid.files, 3, &inherited_proof());
                            push_present_models(&mut valid.files);
                            valid.coverage.artifacts = artifact_facts(&valid.files);
                            valid.authenticate().unwrap();
                            let output = valid.publish(&lab, "valid-inheritance");
                            assert_eq!(
                                verify_selected_geometry_v32(&valid.plan, &output, None)
                                    .unwrap()
                                    .files,
                                13
                            );

                            let mut invalid = request_fixture();
                            invalid.files.clear();
                            let parent_proof = present_proof();
                            push_map(&mut invalid.files, 1, &parent_proof);
                            let child_proof = inherited_proof();
                            push_map(&mut invalid.files, 3, &child_proof);
                            push_present_models(&mut invalid.files);
                            invalid.coverage.artifacts = artifact_facts(&invalid.files);
                            invalid.authenticate().unwrap();
                            let parent_records: Vec<_> = placement::DirBin::parse(
                                &parent_proof,
                                placement::Limits::default(),
                            )
                            .unwrap()
                            .records()
                            .collect();
                            let parent =
                                assembly::build(1, &parent_records, Default::default()).unwrap();
                            let child_records: Vec<_> = placement::DirBin::parse(
                                &child_proof,
                                placement::Limits::default(),
                            )
                            .unwrap()
                            .records()
                            .collect();
                            let child =
                                assembly::build(3, &child_records, Default::default()).unwrap();
                            assembly::verify_inherited_associations(
                                parent.tiles()[0].payload().unwrap(),
                                child.tiles()[0].index_bytes(),
                                child.ordered_dir_bin(),
                                Default::default(),
                            )
                            .unwrap();
                            let output = invalid.publish(&lab, "root-absent-parent");
                            assert_eq!(rejection(&invalid, &output).code, BAD);
                        }

                        #[test]
                        fn rejects_crossed_resources_versions_masks_and_profiles() {
                            let lab = Lab::new();
                            let model_root_absent = model_fixture();
                            let request_capacity = request_fixture();
                            assert_eq!(
                                geometry_v22::Coverage::parse_v32(
                                    &model_root_absent.coverage.encode().unwrap()
                                )
                                .err()
                                .unwrap()
                                .code,
                                "product_selected_geometry_coverage_schema"
                            );
                            assert_eq!(
                                geometry_v22::Coverage::parse_v31(
                                    &request_capacity.coverage.encode().unwrap()
                                )
                                .err()
                                .unwrap()
                                .code,
                                "product_selected_geometry_coverage_schema"
                            );

                            let bytes = request_capacity.coverage.encode().unwrap();
                            for profile in [
                                resources::Profile::BoundedV1,
                                resources::Profile::DiskWorldV1,
                            ] {
                                let mutated = replace_text(
                                    &bytes,
                                    resources::Profile::RequestHeavyV1.id(),
                                    profile.id(),
                                );
                                assert_eq!(
                                    geometry_v22::Coverage::parse_v32(&mutated)
                                        .err()
                                        .unwrap()
                                        .code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }

                            let mut wrong_mask = request_fixture();
                            wrong_mask.coverage.context.components = Component::Collision.bit();
                            assert_eq!(
                                geometry_v22::Coverage::parse_v32(
                                    &wrong_mask.coverage.encode().unwrap()
                                )
                                .err()
                                .unwrap()
                                .code,
                                "product_selected_geometry_coverage_schema"
                            );

                            let missing = Path::new("/nonexistent-request-capacity-geometry-v32");
                            for error in [
                                verify_selected_geometry_v32(
                                    &model_root_absent.plan,
                                    missing,
                                    None,
                                )
                                .unwrap_err(),
                                verify_selected_geometry_v31(&request_capacity.plan, missing, None)
                                    .unwrap_err(),
                            ] {
                                assert_eq!(error.code, "product_plan_component_scope");
                            }
                            let mut wrong_components = request_capacity.plan.clone();
                            wrong_components.components = Component::Collision.bit();
                            assert_eq!(
                                verify_selected_geometry_v32(&wrong_components, missing, None)
                                    .unwrap_err()
                                    .code,
                                "product_plan_component_scope"
                            );
                            let mut selected = request_capacity.plan.clone();
                            selected.map_domain = Some(2);
                            assert_eq!(
                                verify_selected_geometry_v32(&selected, missing, None)
                                    .unwrap_err()
                                    .code,
                                "product_plan_component_scope"
                            );

                            let mut v4_for_v32 = request_fixture();
                            v4_for_v32.coverage.resource_profile = resources::Profile::BoundedV1;
                            let output = v4_for_v32.publish(&lab, "v4-for-v32");
                            rejection(&v4_for_v32, &output);

                            let mut v5_for_v31 = model_fixture();
                            v5_for_v31.coverage.resource_profile =
                                resources::Profile::RequestHeavyV1;
                            let output = v5_for_v31.publish(&lab, "v5-for-v31");
                            super::rejection(&v5_for_v31, &output);
                        }

                        mod v33 {
                            use super::*;
                            use forever_casc::collision::{resources, run::MissingWmoPolicy};

                            const VACATED: u32 = 0xAABB_CCDD;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan.publication_profile =
                                    crate::plan::PublicationProfile::Build70450AvailableVacatedWmoGeometryV1;
                                fixture.coverage.context.plan_sha256 =
                                    integrity::digest(fixture.plan.render().as_bytes());
                                fixture.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2;
                                fixture.coverage.missing_wmo = vec![VACATED];
                                fixture
                            }

                            fn publish(
                                fixture: &V28Fixture,
                                lab: &Lab,
                                name: &str,
                                coverage: &[u8],
                            ) -> PathBuf {
                                let output = lab.0.join(name);
                                let CollisionStorage::LeasedDiskV1(budget) =
                                    fixture.plan.collision_storage
                                else {
                                    panic!("v33 fixture requires a disk budget");
                                };
                                let mut run = package::Run::create(
                                    &output,
                                    fixture.coverage.context,
                                    &[],
                                    geometry_v22::package_limits_v28_with_extra_files(budget, 0),
                                )
                                .unwrap();
                                run.put(Component::Db2, &fixture.declaration.path, &fixture.map)
                                    .unwrap();
                                for (path, bytes) in &fixture.files {
                                    run.put(Component::Collision, path, bytes).unwrap();
                                }
                                run.put(
                                    Component::Collision,
                                    geometry_v22::VACATED_WMO_COVERAGE_PATH,
                                    coverage,
                                )
                                .unwrap();
                                run.set_partial_encrypted_table(
                                    &fixture.declaration.path,
                                    &fixture.declaration.sections,
                                )
                                .unwrap();
                                run.finish_component(Component::Db2).unwrap();
                                run.finish_component(Component::Collision).unwrap();
                                run.finish().unwrap();
                                output
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v33(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v33 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            fn replace_text(
                                bytes: &[u8],
                                text: &str,
                                replacement: &str,
                            ) -> Vec<u8> {
                                let needle = text.as_bytes();
                                let at = bytes
                                    .windows(needle.len())
                                    .position(|window| window == needle)
                                    .expect("coverage text");
                                let mut mutated = bytes[..at - 4].to_vec();
                                mutated
                                    .extend_from_slice(&(replacement.len() as u32).to_le_bytes());
                                mutated.extend_from_slice(replacement.as_bytes());
                                mutated.extend_from_slice(&bytes[at + needle.len()..]);
                                mutated
                            }

                            #[test]
                            fn sealed_v33_vacated_wmo_preserves_parent_and_present_doodad() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v33(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert_eq!(parsed.missing_wmo, [VACATED]);
                                assert!(
                                    fixture
                                        .files
                                        .iter()
                                        .any(|(path, _)| path == "vmaps/wmo-parent.vmo")
                                );
                                let output = publish(&fixture, &lab, "package", &encoded);
                                let summary =
                                    verify_selected_geometry_v33(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output
                                        .join(geometry_v22::VACATED_WMO_COVERAGE_PATH)
                                        .is_file()
                                );
                            }

                            #[test]
                            fn rejects_a_vmo_for_a_vacated_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture
                                    .files
                                    .push(super::model_vmo(&geometry_v22::m2_file_name(VACATED)));
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output = publish(&fixture, &lab, "vacated-vmo", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_a_placement_derived_from_a_vacated_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture.files.clear();
                                let proof = [
                                    super::present_proof(),
                                    super::named_placement(
                                        19,
                                        geometry_v22::m2_file_name(VACATED).as_bytes(),
                                    ),
                                ]
                                .concat();
                                super::push_map(&mut fixture.files, 1, &proof);
                                super::push_map(&mut fixture.files, 3, &[]);
                                super::push_present_models(&mut fixture.files);
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output = publish(&fixture, &lab, "vacated-placement", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_a_crossed_v33_resource_profile() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let encoded = replace_text(
                                    &fixture.coverage.encode().unwrap(),
                                    resources::Profile::RequestHeavyV1.id(),
                                    resources::Profile::BoundedV1.id(),
                                );
                                let output = publish(&fixture, &lab, "bounded-v33", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }

                            #[test]
                            fn rejects_v32_coverage_at_the_v33_verifier() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let encoded = super::request_fixture().coverage.encode().unwrap();
                                assert_eq!(
                                    u32::from_le_bytes(encoded[8..12].try_into().unwrap()),
                                    5
                                );
                                let output = publish(&fixture, &lab, "v32-coverage", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }

                            #[test]
                            fn rejects_a_false_v33_vacated_wmo_policy_bit() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut encoded = fixture.coverage.encode().unwrap();
                                let profile = resources::Profile::RequestHeavyV1.id().as_bytes();
                                let at = encoded
                                    .windows(profile.len())
                                    .position(|window| window == profile)
                                    .expect("resource profile");
                                let policy_at = at + profile.len();
                                assert_eq!(encoded[policy_at], 1);
                                encoded[policy_at] = 0;
                                let output =
                                    publish(&fixture, &lab, "reject-vacated-policy", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }
                        }

                        mod v34 {
                            use super::*;
                            use forever_casc::collision::{resources, run::MissingWmoPolicy};

                            const VACATED: u32 = 0xAABB_CCDD;
                            const EMPTY: u32 = 0xDDEE_FF00;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan.publication_profile =
                                    crate::plan::PublicationProfile::Build70450AvailableEmptyWmoGeometryV1;
                                fixture.coverage.context.plan_sha256 =
                                    integrity::digest(fixture.plan.render().as_bytes());
                                fixture.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty;
                                fixture.coverage.missing_wmo = vec![VACATED];
                                fixture.coverage.empty_wmo = vec![EMPTY];
                                fixture
                            }

                            fn publish(
                                fixture: &V28Fixture,
                                lab: &Lab,
                                name: &str,
                                coverage: &[u8],
                            ) -> PathBuf {
                                let output = lab.0.join(name);
                                let CollisionStorage::LeasedDiskV1(budget) =
                                    fixture.plan.collision_storage
                                else {
                                    panic!("v34 fixture requires a disk budget");
                                };
                                let mut run = package::Run::create(
                                    &output,
                                    fixture.coverage.context,
                                    &[],
                                    geometry_v22::package_limits_v28_with_extra_files(budget, 0),
                                )
                                .unwrap();
                                run.put(Component::Db2, &fixture.declaration.path, &fixture.map)
                                    .unwrap();
                                for (path, bytes) in &fixture.files {
                                    run.put(Component::Collision, path, bytes).unwrap();
                                }
                                run.put(
                                    Component::Collision,
                                    geometry_v22::EMPTY_WMO_COVERAGE_PATH,
                                    coverage,
                                )
                                .unwrap();
                                run.set_partial_encrypted_table(
                                    &fixture.declaration.path,
                                    &fixture.declaration.sections,
                                )
                                .unwrap();
                                run.finish_component(Component::Db2).unwrap();
                                run.finish_component(Component::Collision).unwrap();
                                run.finish().unwrap();
                                output
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v34(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v34 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            fn replace_text(
                                bytes: &[u8],
                                text: &str,
                                replacement: &str,
                            ) -> Vec<u8> {
                                let needle = text.as_bytes();
                                let at = bytes
                                    .windows(needle.len())
                                    .position(|window| window == needle)
                                    .expect("coverage text");
                                let mut mutated = bytes[..at - 4].to_vec();
                                mutated
                                    .extend_from_slice(&(replacement.len() as u32).to_le_bytes());
                                mutated.extend_from_slice(replacement.as_bytes());
                                mutated.extend_from_slice(&bytes[at + needle.len()..]);
                                mutated
                            }

                            #[test]
                            fn sealed_v34_separates_vacated_and_intrinsically_empty_wmos() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v34(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert_eq!(parsed.missing_wmo, [VACATED]);
                                assert_eq!(parsed.empty_wmo, [EMPTY]);
                                let output = publish(&fixture, &lab, "package", &encoded);
                                let summary =
                                    verify_selected_geometry_v34(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output.join(geometry_v22::EMPTY_WMO_COVERAGE_PATH).is_file()
                                );
                            }

                            #[test]
                            fn rejects_a_vmo_for_an_intrinsically_empty_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture
                                    .files
                                    .push(super::model_vmo(&geometry_v22::m2_file_name(EMPTY)));
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output = publish(&fixture, &lab, "empty-vmo", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_a_placement_derived_from_an_intrinsically_empty_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture.files.clear();
                                let proof = [
                                    super::present_proof(),
                                    super::named_placement(
                                        19,
                                        geometry_v22::m2_file_name(EMPTY).as_bytes(),
                                    ),
                                ]
                                .concat();
                                super::push_map(&mut fixture.files, 1, &proof);
                                super::push_map(&mut fixture.files, 3, &[]);
                                super::push_present_models(&mut fixture.files);
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output = publish(&fixture, &lab, "empty-placement", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_a_crossed_v34_resource_profile() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let encoded = replace_text(
                                    &fixture.coverage.encode().unwrap(),
                                    resources::Profile::RequestHeavyV1.id(),
                                    resources::Profile::BoundedV1.id(),
                                );
                                let output = publish(&fixture, &lab, "bounded-v34", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }

                            #[test]
                            fn rejects_v33_coverage_at_the_v34_verifier() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut previous = super::request_fixture();
                                previous.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2;
                                previous.coverage.missing_wmo = vec![VACATED];
                                let encoded = previous.coverage.encode().unwrap();
                                assert_eq!(
                                    u32::from_le_bytes(encoded[8..12].try_into().unwrap()),
                                    6
                                );
                                let output = publish(&fixture, &lab, "v33-coverage", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }

                            #[test]
                            fn rejects_a_false_v34_wmo_policy_bit() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut encoded = fixture.coverage.encode().unwrap();
                                let profile = resources::Profile::RequestHeavyV1.id().as_bytes();
                                let at = encoded
                                    .windows(profile.len())
                                    .position(|window| window == profile)
                                    .expect("resource profile");
                                let policy_at = at + profile.len();
                                assert_eq!(encoded[policy_at], 1);
                                encoded[policy_at] = 0;
                                let output =
                                    publish(&fixture, &lab, "reject-empty-policy", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }
                        }

                        mod v35 {
                            use super::*;
                            use forever_casc::collision::run::MissingWmoPolicy;

                            const UNSELECTED: u32 = 0xDDEE_FF00;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan.publication_profile =
                                    crate::plan::PublicationProfile::Build70450AvailableUnselectedWmoGeometryV1;
                                fixture.coverage.context.plan_sha256 =
                                    integrity::digest(fixture.plan.render().as_bytes());
                                fixture.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected;
                                fixture.coverage.unselected_wmo = vec![UNSELECTED];
                                fixture
                            }

                            fn publish(
                                fixture: &V28Fixture,
                                lab: &Lab,
                                name: &str,
                                coverage: &[u8],
                            ) -> PathBuf {
                                let output = lab.0.join(name);
                                let CollisionStorage::LeasedDiskV1(budget) =
                                    fixture.plan.collision_storage
                                else {
                                    panic!("v35 fixture requires a disk budget");
                                };
                                let mut run = package::Run::create(
                                    &output,
                                    fixture.coverage.context,
                                    &[],
                                    geometry_v22::package_limits_v28_with_extra_files(budget, 0),
                                )
                                .unwrap();
                                run.put(Component::Db2, &fixture.declaration.path, &fixture.map)
                                    .unwrap();
                                for (path, bytes) in &fixture.files {
                                    run.put(Component::Collision, path, bytes).unwrap();
                                }
                                run.put(
                                    Component::Collision,
                                    geometry_v22::UNSELECTED_WMO_COVERAGE_PATH,
                                    coverage,
                                )
                                .unwrap();
                                run.set_partial_encrypted_table(
                                    &fixture.declaration.path,
                                    &fixture.declaration.sections,
                                )
                                .unwrap();
                                run.finish_component(Component::Db2).unwrap();
                                run.finish_component(Component::Collision).unwrap();
                                run.finish().unwrap();
                                output
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v35(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v35 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            #[test]
                            fn sealed_v35_declares_unselected_wmo_without_artifacts() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v35(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert!(parsed.missing_wmo.is_empty());
                                assert!(parsed.empty_wmo.is_empty());
                                assert_eq!(parsed.unselected_wmo, [UNSELECTED]);
                                let output = publish(&fixture, &lab, "package", &encoded);
                                let summary =
                                    verify_selected_geometry_v35(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output
                                        .join(geometry_v22::UNSELECTED_WMO_COVERAGE_PATH)
                                        .is_file()
                                );
                            }

                            #[test]
                            fn rejects_a_vmo_for_an_unselected_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture
                                    .files
                                    .push(super::model_vmo(&geometry_v22::m2_file_name(
                                        UNSELECTED,
                                    )));
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output = publish(&fixture, &lab, "unselected-vmo", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_a_placement_derived_from_an_unselected_wmo() {
                                let lab = Lab::new();
                                let mut fixture = fixture();
                                fixture.files.clear();
                                let proof = [
                                    super::present_proof(),
                                    super::named_placement(
                                        19,
                                        geometry_v22::m2_file_name(UNSELECTED).as_bytes(),
                                    ),
                                ]
                                .concat();
                                super::push_map(&mut fixture.files, 1, &proof);
                                super::push_map(&mut fixture.files, 3, &[]);
                                super::push_present_models(&mut fixture.files);
                                fixture.coverage.artifacts = super::artifact_facts(&fixture.files);
                                let encoded = fixture.coverage.encode().unwrap();
                                let output =
                                    publish(&fixture, &lab, "unselected-placement", &encoded);
                                assert_eq!(rejection(&fixture, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_v34_coverage_at_the_v35_verifier() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut previous = super::request_fixture();
                                previous.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty;
                                previous.coverage.empty_wmo = vec![UNSELECTED];
                                let encoded = previous.coverage.encode().unwrap();
                                assert_eq!(
                                    u32::from_le_bytes(encoded[8..12].try_into().unwrap()),
                                    7
                                );
                                let output = publish(&fixture, &lab, "v34-coverage", &encoded);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );
                            }
                        }

                        mod v36 {
                            use super::*;
                            use forever_casc::collision::{resources, run::MissingWmoPolicy};

                            const MATERIAL_FILTERED: u32 = 0x0102_0305;
                            const UNSELECTED: u32 = 0x0203_0405;
                            const VACATED: u32 = 0xAABB_CCDD;
                            const EMPTY: u32 = 0xDDEE_FF00;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan.publication_profile = crate::plan::PublicationProfile::
                                    Build70450MaterialFilteredWmoGeometryV1;
                                fixture.coverage.context.plan_sha256 =
                                    integrity::digest(fixture.plan.render().as_bytes());
                                fixture.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                fixture.coverage.missing_wmo = vec![VACATED];
                                fixture.coverage.empty_wmo = vec![EMPTY];
                                fixture.coverage.unselected_wmo = vec![UNSELECTED];
                                fixture.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                fixture
                            }

                            fn publish(
                                fixture: &V28Fixture,
                                lab: &Lab,
                                name: &str,
                                coverage: &[u8],
                            ) -> PathBuf {
                                let output = lab.0.join(name);
                                let CollisionStorage::LeasedDiskV1(budget) =
                                    fixture.plan.collision_storage
                                else {
                                    panic!("v36 fixture requires a disk budget");
                                };
                                let mut run = package::Run::create(
                                    &output,
                                    fixture.coverage.context,
                                    &[],
                                    geometry_v22::package_limits_v28_with_extra_files(budget, 0),
                                )
                                .unwrap();
                                run.put(Component::Db2, &fixture.declaration.path, &fixture.map)
                                    .unwrap();
                                for (path, bytes) in &fixture.files {
                                    run.put(Component::Collision, path, bytes).unwrap();
                                }
                                run.put(
                                    Component::Collision,
                                    geometry_v22::MATERIAL_FILTERED_WMO_COVERAGE_PATH,
                                    coverage,
                                )
                                .unwrap();
                                run.set_partial_encrypted_table(
                                    &fixture.declaration.path,
                                    &fixture.declaration.sections,
                                )
                                .unwrap();
                                run.finish_component(Component::Db2).unwrap();
                                run.finish_component(Component::Collision).unwrap();
                                run.finish().unwrap();
                                output
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v36(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v36 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            #[test]
                            fn sealed_v36_declares_material_filtered_wmo_as_producer_reported() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v36(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert_eq!(parsed.missing_wmo, [VACATED]);
                                assert_eq!(parsed.empty_wmo, [EMPTY]);
                                assert_eq!(parsed.unselected_wmo, [UNSELECTED]);
                                assert_eq!(parsed.material_filtered_wmo, [MATERIAL_FILTERED]);
                                let mut duplicate = self::fixture().coverage;
                                duplicate.material_filtered_wmo.push(UNSELECTED);
                                assert!(duplicate.encode().is_err());
                                let output = publish(&fixture, &lab, "package", &encoded);
                                let summary =
                                    verify_selected_geometry_v36(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output
                                        .join(geometry_v22::MATERIAL_FILTERED_WMO_COVERAGE_PATH)
                                        .is_file()
                                );
                                assert!(
                                    !output
                                        .join(geometry_v22::UNSELECTED_WMO_COVERAGE_PATH)
                                        .exists()
                                );
                            }

                            #[test]
                            fn rejects_a_vmo_or_placement_for_a_material_filtered_wmo() {
                                let lab = Lab::new();
                                let mut vmo = fixture();
                                vmo.files.push(super::model_vmo(&geometry_v22::m2_file_name(
                                    MATERIAL_FILTERED,
                                )));
                                vmo.coverage.artifacts = super::artifact_facts(&vmo.files);
                                let encoded = vmo.coverage.encode().unwrap();
                                let output = publish(&vmo, &lab, "material-filtered-vmo", &encoded);
                                assert_eq!(rejection(&vmo, &output).code, BAD);

                                let mut placement = fixture();
                                placement.files.clear();
                                let proof = [
                                    super::present_proof(),
                                    super::named_placement(
                                        19,
                                        geometry_v22::m2_file_name(MATERIAL_FILTERED).as_bytes(),
                                    ),
                                ]
                                .concat();
                                super::push_map(&mut placement.files, 1, &proof);
                                super::push_map(&mut placement.files, 3, &[]);
                                super::push_present_models(&mut placement.files);
                                placement.coverage.artifacts =
                                    super::artifact_facts(&placement.files);
                                let encoded = placement.coverage.encode().unwrap();
                                let output = publish(
                                    &placement,
                                    &lab,
                                    "material-filtered-placement",
                                    &encoded,
                                );
                                assert_eq!(rejection(&placement, &output).code, BAD);
                            }

                            #[test]
                            fn rejects_crossed_v35_coverage_profiles_and_wrapper_plans() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut previous = super::request_fixture();
                                previous.plan.publication_profile =
                                    crate::plan::PublicationProfile::
                                        Build70450AvailableUnselectedWmoGeometryV1;
                                previous.coverage.context.plan_sha256 =
                                    integrity::digest(previous.plan.render().as_bytes());
                                previous.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselected;
                                previous.coverage.unselected_wmo = vec![UNSELECTED];
                                let previous_bytes = previous.coverage.encode().unwrap();
                                assert_eq!(
                                    u32::from_le_bytes(previous_bytes[8..12].try_into().unwrap()),
                                    8
                                );
                                let output =
                                    publish(&fixture, &lab, "v35-coverage", &previous_bytes);
                                assert_eq!(
                                    rejection(&fixture, &output).code,
                                    "product_selected_geometry_coverage_schema"
                                );

                                let bytes = fixture.coverage.encode().unwrap();
                                let crossed_resource = replace_text(
                                    &bytes,
                                    resources::Profile::RequestHeavyV1.id(),
                                    resources::Profile::BoundedV1.id(),
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v36(&crossed_resource).is_err()
                                );
                                let mut wrong_mask = self::fixture().coverage;
                                wrong_mask.context.components = Component::Collision.bit();
                                assert!(
                                    geometry_v22::Coverage::parse_v36(
                                        &wrong_mask.encode().unwrap()
                                    )
                                    .is_err()
                                );

                                let missing =
                                    Path::new("/nonexistent-material-filtered-geometry-v36");
                                let mut wrong_wrapper = fixture.plan.clone();
                                wrong_wrapper.publication_profile = crate::plan::PublicationProfile::
                                    Build70450AvailableUnselectedWmoGeometryV1;
                                assert_eq!(
                                    verify_selected_geometry_v36(&wrong_wrapper, missing, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                                assert_eq!(
                                    verify_selected_geometry_v35(&fixture.plan, missing, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                            }
                        }

                        mod v37 {
                            use super::*;
                            use forever_casc::collision::{resources, run::MissingWmoPolicy};

                            const OMITTED: u32 = 0x0102_0301;
                            const VACATED: u32 = 0x0102_0303;
                            const EMPTY: u32 = 0x0102_0305;
                            const UNSELECTED: u32 = 0x0102_0307;
                            const MATERIAL_FILTERED: u32 = 0x0102_0309;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan = Plan::parse(V37).unwrap();
                                fixture.plan.publication_profile = crate::plan::PublicationProfile::
                                    Build70450DependencyHeavyMaterialFilteredWmoGeometryV1;
                                fixture.coverage.context = Context {
                                    build: 70450,
                                    locale: fixture.plan.locale,
                                    components: Component::Db2.bit() | Component::Collision.bit(),
                                    source_sha256: [0x41; 32],
                                    plan_sha256: integrity::digest(
                                        fixture.plan.render().as_bytes(),
                                    ),
                                    producer_sha256: [0x42; 32],
                                };
                                fixture.coverage.resource_profile =
                                    resources::Profile::DependencyHeavyV1;
                                fixture.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                fixture.coverage.missing_m2 = vec![OMITTED];
                                fixture.coverage.missing_wmo = vec![VACATED];
                                fixture.coverage.empty_wmo = vec![EMPTY];
                                fixture.coverage.unselected_wmo = vec![UNSELECTED];
                                fixture.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                fixture
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v37(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v37 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            #[test]
                            fn sealed_dependency_heavy_coverage_binds_five_producer_reported_lists()
                            {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v37(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert_eq!(parsed.missing_wmo, [VACATED]);
                                assert_eq!(parsed.empty_wmo, [EMPTY]);
                                assert_eq!(parsed.unselected_wmo, [UNSELECTED]);
                                assert_eq!(parsed.material_filtered_wmo, [MATERIAL_FILTERED]);
                                assert_eq!(
                                    parsed.resource_profile,
                                    resources::Profile::DependencyHeavyV1
                                );
                                let output = fixture.publish(&lab, "package");
                                let summary =
                                    verify_selected_geometry_v37(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output
                                        .join(
                                            geometry_v22::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
                                        )
                                        .is_file()
                                );
                                assert!(
                                    !output
                                        .join(geometry_v22::MATERIAL_FILTERED_WMO_COVERAGE_PATH)
                                        .exists()
                                );
                            }

                            #[test]
                            fn rejects_vmos_and_placements_for_all_five_dependency_heavy_omissions()
                            {
                                let lab = Lab::new();
                                for (label, id) in [
                                    ("missing-m2", OMITTED),
                                    ("vacated-wmo", VACATED),
                                    ("empty-wmo", EMPTY),
                                    ("unselected-wmo", UNSELECTED),
                                    ("material-filtered-wmo", MATERIAL_FILTERED),
                                ] {
                                    let mut vmo_fixture = fixture();
                                    vmo_fixture
                                        .files
                                        .push(super::model_vmo(&geometry_v22::m2_file_name(id)));
                                    vmo_fixture.coverage.artifacts =
                                        super::artifact_facts(&vmo_fixture.files);
                                    let output = vmo_fixture.publish(&lab, &format!("{label}-vmo"));
                                    assert_eq!(
                                        rejection(&vmo_fixture, &output).code,
                                        BAD,
                                        "{label}"
                                    );

                                    let mut placement_fixture = fixture();
                                    placement_fixture.files.clear();
                                    let proof = [
                                        super::present_proof(),
                                        super::named_placement(
                                            19,
                                            geometry_v22::m2_file_name(id).as_bytes(),
                                        ),
                                    ]
                                    .concat();
                                    super::push_map(&mut placement_fixture.files, 1, &proof);
                                    super::push_map(&mut placement_fixture.files, 3, &[]);
                                    super::push_present_models(&mut placement_fixture.files);
                                    placement_fixture.coverage.artifacts =
                                        super::artifact_facts(&placement_fixture.files);
                                    let output = placement_fixture
                                        .publish(&lab, &format!("{label}-placement"));
                                    assert_eq!(
                                        rejection(&placement_fixture, &output).code,
                                        BAD,
                                        "{label}"
                                    );
                                }
                            }

                            #[test]
                            fn rejects_crossed_layout_resource_mask_plan_and_verifier_wrappers() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let previous = super::request_fixture();
                                let mut v9 = super::request_fixture();
                                v9.plan.publication_profile =
                                    crate::plan::PublicationProfile::Build70450MaterialFilteredWmoGeometryV1;
                                v9.coverage.resource_profile = resources::Profile::RequestHeavyV1;
                                v9.coverage.context.plan_sha256 =
                                    integrity::digest(v9.plan.render().as_bytes());
                                v9.coverage.missing_wmo_policy =
                                    MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                v9.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                assert!(
                                    geometry_v22::Coverage::parse_v37(
                                        &previous.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v36(
                                        &fixture.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v37(
                                        &v9.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );

                                let bytes = fixture.coverage.encode().unwrap();
                                let crossed_resource = super::v32::replace_text(
                                    &bytes,
                                    resources::Profile::DependencyHeavyV1.id(),
                                    resources::Profile::RequestHeavyV1.id(),
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v37(&crossed_resource).is_err()
                                );
                                let mut wrong_mask = self::fixture().coverage;
                                wrong_mask.context.components = Component::Collision.bit();
                                assert!(
                                    geometry_v22::Coverage::parse_v37(
                                        &wrong_mask.encode().unwrap()
                                    )
                                    .is_err()
                                );

                                let output = fixture.publish(&lab, "package");
                                let mut wrong_wrapper = fixture.plan.clone();
                                wrong_wrapper.publication_profile =
                                    crate::plan::PublicationProfile::Build70450MaterialFilteredWmoGeometryV1;
                                assert_eq!(
                                    verify_selected_geometry_v37(&wrong_wrapper, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                                assert_eq!(
                                    verify_selected_geometry_v36(&fixture.plan, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                                let mut map_domain = fixture.plan.clone();
                                map_domain.map_domain = Some(2);
                                assert_eq!(
                                    verify_selected_geometry_v37(&map_domain, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                            }
                        }

                        mod v38 {
                            use super::*;
                            use forever_casc::collision::{resources, run::MissingWmoPolicy};

                            const OMITTED: u32 = 0x0102_0301;
                            const VACATED: u32 = 0x0102_0303;
                            const EMPTY: u32 = 0x0102_0305;
                            const UNSELECTED: u32 = 0x0102_0307;
                            const MATERIAL_FILTERED: u32 = 0x0102_0309;

                            fn fixture() -> V28Fixture {
                                let mut fixture = super::request_fixture();
                                fixture.plan = Plan::parse(V38).unwrap();
                                fixture.plan.publication_profile = crate::plan::PublicationProfile::
                                    Build70450SourceHeavyMaterialFilteredWmoGeometryV1;
                                fixture.coverage.context = Context {
                                    build: 70450,
                                    locale: fixture.plan.locale,
                                    components: Component::Db2.bit() | Component::Collision.bit(),
                                    source_sha256: [0x41; 32],
                                    plan_sha256: integrity::digest(
                                        fixture.plan.render().as_bytes(),
                                    ),
                                    producer_sha256: [0x42; 32],
                                };
                                fixture.coverage.resource_profile =
                                    resources::Profile::SourceHeavyV1;
                                fixture.coverage.missing_wmo_policy = MissingWmoPolicy::
                                    DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                fixture.coverage.missing_m2 = vec![OMITTED];
                                fixture.coverage.missing_wmo = vec![VACATED];
                                fixture.coverage.empty_wmo = vec![EMPTY];
                                fixture.coverage.unselected_wmo = vec![UNSELECTED];
                                fixture.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                fixture
                            }

                            fn rejection(fixture: &V28Fixture, output: &Path) -> Error {
                                match verify_selected_geometry_v38(&fixture.plan, output, None) {
                                    Ok(_) => panic!("mutated v38 package unexpectedly passed"),
                                    Err(error) => error,
                                }
                            }

                            #[test]
                            fn sealed_source_heavy_coverage_binds_five_producer_reported_lists() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                fixture.authenticate().unwrap();
                                let encoded = fixture.coverage.encode().unwrap();
                                let parsed = geometry_v22::Coverage::parse_v38(&encoded).unwrap();
                                assert_eq!(parsed.missing_m2, [OMITTED]);
                                assert_eq!(parsed.missing_wmo, [VACATED]);
                                assert_eq!(parsed.empty_wmo, [EMPTY]);
                                assert_eq!(parsed.unselected_wmo, [UNSELECTED]);
                                assert_eq!(parsed.material_filtered_wmo, [MATERIAL_FILTERED]);
                                assert_eq!(
                                    parsed.resource_profile,
                                    resources::Profile::SourceHeavyV1
                                );
                                let output = fixture.publish(&lab, "package");
                                let summary =
                                    verify_selected_geometry_v38(&fixture.plan, &output, None)
                                        .unwrap();
                                assert_eq!(summary.context, fixture.coverage.context);
                                assert_eq!(summary.files, 10);
                                assert!(
                                    output
                                        .join(
                                            geometry_v22::SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
                                        )
                                        .is_file()
                                );
                                assert!(
                                    !output
                                        .join(
                                            geometry_v22::DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PATH
                                        )
                                        .exists()
                                );
                            }

                            #[test]
                            fn rejects_vmos_and_placements_for_all_five_source_heavy_omissions() {
                                let lab = Lab::new();
                                for (label, id) in [
                                    ("missing-m2", OMITTED),
                                    ("vacated-wmo", VACATED),
                                    ("empty-wmo", EMPTY),
                                    ("unselected-wmo", UNSELECTED),
                                    ("material-filtered-wmo", MATERIAL_FILTERED),
                                ] {
                                    let mut vmo_fixture = fixture();
                                    vmo_fixture
                                        .files
                                        .push(super::model_vmo(&geometry_v22::m2_file_name(id)));
                                    vmo_fixture.coverage.artifacts =
                                        super::artifact_facts(&vmo_fixture.files);
                                    let output = vmo_fixture.publish(&lab, &format!("{label}-vmo"));
                                    assert_eq!(
                                        rejection(&vmo_fixture, &output).code,
                                        BAD,
                                        "{label}"
                                    );

                                    let mut placement_fixture = fixture();
                                    placement_fixture.files.clear();
                                    let proof = [
                                        super::present_proof(),
                                        super::named_placement(
                                            19,
                                            geometry_v22::m2_file_name(id).as_bytes(),
                                        ),
                                    ]
                                    .concat();
                                    super::push_map(&mut placement_fixture.files, 1, &proof);
                                    super::push_map(&mut placement_fixture.files, 3, &[]);
                                    super::push_present_models(&mut placement_fixture.files);
                                    placement_fixture.coverage.artifacts =
                                        super::artifact_facts(&placement_fixture.files);
                                    let output = placement_fixture
                                        .publish(&lab, &format!("{label}-placement"));
                                    assert_eq!(
                                        rejection(&placement_fixture, &output).code,
                                        BAD,
                                        "{label}"
                                    );
                                }
                            }

                            #[test]
                            fn rejects_crossed_v37_v9_layout_resources_and_verifier_wrappers() {
                                let lab = Lab::new();
                                let fixture = fixture();
                                let mut previous = super::request_fixture();
                                previous.plan = Plan::parse(V37).unwrap();
                                previous.plan.publication_profile = crate::plan::PublicationProfile::
                                    Build70450DependencyHeavyMaterialFilteredWmoGeometryV1;
                                previous.coverage.context.plan_sha256 =
                                    integrity::digest(previous.plan.render().as_bytes());
                                previous.coverage.resource_profile =
                                    resources::Profile::DependencyHeavyV1;
                                previous.coverage.missing_wmo_policy = MissingWmoPolicy::
                                    DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                previous.coverage.missing_m2 = vec![OMITTED];
                                previous.coverage.missing_wmo = vec![VACATED];
                                previous.coverage.empty_wmo = vec![EMPTY];
                                previous.coverage.unselected_wmo = vec![UNSELECTED];
                                previous.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                let mut v9 = super::request_fixture();
                                v9.plan.publication_profile = crate::plan::PublicationProfile::
                                    Build70450MaterialFilteredWmoGeometryV1;
                                v9.coverage.resource_profile = resources::Profile::RequestHeavyV1;
                                v9.coverage.context.plan_sha256 =
                                    integrity::digest(v9.plan.render().as_bytes());
                                v9.coverage.missing_wmo_policy = MissingWmoPolicy::
                                    DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
                                v9.coverage.material_filtered_wmo = vec![MATERIAL_FILTERED];
                                assert!(
                                    geometry_v22::Coverage::parse_v38(
                                        &previous.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v37(
                                        &fixture.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );
                                assert!(
                                    geometry_v22::Coverage::parse_v38(
                                        &v9.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );

                                let encoded = fixture.coverage.encode().unwrap();
                                for profile in [
                                    resources::Profile::DependencyHeavyV1,
                                    resources::Profile::RequestHeavyV1,
                                ] {
                                    let crossed = super::v32::replace_text(
                                        &encoded,
                                        resources::Profile::SourceHeavyV1.id(),
                                        profile.id(),
                                    );
                                    assert!(geometry_v22::Coverage::parse_v38(&crossed).is_err());
                                }
                                let mut wrong_mask = self::fixture();
                                wrong_mask.coverage.context.components = Component::Collision.bit();
                                assert!(
                                    geometry_v22::Coverage::parse_v38(
                                        &wrong_mask.coverage.encode().unwrap()
                                    )
                                    .is_err()
                                );

                                let output = fixture.publish(&lab, "package");
                                let mut wrong_wrapper = fixture.plan.clone();
                                wrong_wrapper.publication_profile = crate::plan::PublicationProfile::
                                    Build70450MaterialFilteredWmoGeometryV1;
                                assert_eq!(
                                    verify_selected_geometry_v38(&wrong_wrapper, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                                assert_eq!(
                                    verify_selected_geometry_v37(&fixture.plan, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                                let mut selected_map = fixture.plan.clone();
                                selected_map.map_domain = Some(2);
                                assert_eq!(
                                    verify_selected_geometry_v38(&selected_map, &output, None)
                                        .unwrap_err()
                                        .code,
                                    "product_plan_component_scope"
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod v47_tests {
        use super::*;
        use forever_casc::package::Component;
        use std::sync::atomic::{AtomicU64, Ordering};

        const V47: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
        const V48: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
        const V49: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v49/all.tsv");
        const V50: &str =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v50/all.tsv");
        static NEXT: AtomicU64 = AtomicU64::new(0);

        struct Lab(PathBuf);
        impl Lab {
            fn new() -> Self {
                let root = std::env::temp_dir().join(format!(
                    "rce-v47-verify-{}-{}",
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

        fn put16(bytes: &mut [u8], at: usize, value: u16) {
            bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
        }

        fn put32(bytes: &mut [u8], at: usize, value: u32) {
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }

        fn field_offset(fields: &[forever_db2::catalog::FieldMetadata], index: usize) -> usize {
            fields[..index]
                .iter()
                .map(|field| usize::from(field_width(field.kind) * u16::from(field.array_size)) / 8)
                .sum()
        }

        fn field_width(kind: forever_db2::catalog::FieldType) -> u16 {
            match kind {
                forever_db2::catalog::FieldType::String
                | forever_db2::catalog::FieldType::NonLocalizedString
                | forever_db2::catalog::FieldType::Int
                | forever_db2::catalog::FieldType::Float => 32,
                forever_db2::catalog::FieldType::Byte => 8,
                forever_db2::catalog::FieldType::Short => 16,
                forever_db2::catalog::FieldType::Long => 64,
            }
        }

        fn db2(name: &str, row: Option<(u32, u32)>) -> Vec<u8> {
            let rows = row.into_iter().collect::<Vec<_>>();
            db2_rows(name, &rows)
        }

        fn liquid_object_db2() -> Vec<u8> {
            let metadata = forever_db2::catalog::resolve_for(
                forever_db2::catalog::WOWDBDEFS_70450_CATALOG_ID,
                "LiquidObject",
            )
            .unwrap()
            .unwrap()
            .metadata();
            let fields = &metadata.fields[..metadata.file_field_count];
            let record_size = fields
                .iter()
                .map(|field| usize::from(field_width(field.kind) * u16::from(field.array_size)) / 8)
                .sum::<usize>();
            let record_offset = 204 + 40 + fields.len() * 4 + fields.len() * 24;
            let mut bytes = vec![0; record_offset + record_size + 4];
            bytes[..4].copy_from_slice(b"WDC5");
            for (at, value) in [
                (4, 5),
                (136, 1),
                (140, fields.len() as u32),
                (144, record_size as u32),
                (152, metadata.table_hash.unwrap()),
                (156, metadata.layout_hash),
                (160, 7),
                (164, 7),
                (176, metadata.field_count as u32),
                (188, (fields.len() * 24) as u32),
                (200, 1),
                (212, record_offset as u32),
                (216, 1),
                (228, 4),
            ] {
                put32(&mut bytes, at, value);
            }
            bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
            let mut column_offset = 0_u16;
            for (index, field) in fields.iter().enumerate() {
                let width = field_width(field.kind);
                let column = 244 + fields.len() * 4 + index * 24;
                bytes[column..column + 2].copy_from_slice(&column_offset.to_le_bytes());
                put32(&mut bytes, column + 16, u32::from(width));
                put32(&mut bytes, column + 20, u32::from(field.array_size));
                column_offset += width * u16::from(field.array_size);
            }
            put32(&mut bytes, 212, record_offset as u32);
            put32(&mut bytes, 164, 7);
            bytes[record_offset..record_offset + record_size]
                [field_offset(fields, 2)..field_offset(fields, 2) + 2]
                .copy_from_slice(&513_u16.to_le_bytes());
            bytes[record_offset + record_size..].copy_from_slice(&7_u32.to_le_bytes());
            bytes
        }

        fn db2_rows(name: &str, rows: &[(u32, u32)]) -> Vec<u8> {
            let metadata = forever_db2::catalog::resolve_for(
                forever_db2::catalog::WOWDBDEFS_70450_CATALOG_ID,
                name,
            )
            .unwrap()
            .unwrap()
            .metadata();
            assert_eq!(metadata.field_count, metadata.file_field_count);
            let fields = &metadata.fields[..metadata.file_field_count];
            let record_size = fields
                .iter()
                .map(|field| usize::from(field_width(field.kind) * u16::from(field.array_size)) / 8)
                .sum::<usize>();
            let header_and_section = 204 + 40;
            let columns = fields.len() * 4 + fields.len() * 24;
            let record_offset = header_and_section + columns;
            let row_count = rows.len();
            let mut bytes = vec![0; record_offset];
            bytes[..4].copy_from_slice(b"WDC5");
            for (at, value) in [
                (4, 5),
                (136, row_count as u32),
                (140, fields.len() as u32),
                (144, record_size as u32),
                (152, metadata.table_hash.unwrap_or(0)),
                (156, metadata.layout_hash),
                (160, rows.first().map_or(0, |(id, _)| *id)),
                (164, rows.last().map_or(0, |(id, _)| *id)),
                (176, metadata.field_count as u32),
                (188, (fields.len() * 24) as u32),
                (200, 1),
                (212, record_offset as u32),
                (216, row_count as u32),
                (228, (row_count * 4) as u32),
            ] {
                put32(&mut bytes, at, value);
            }
            bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
            let mut column_offset = 0_u16;
            for (index, field) in fields.iter().enumerate() {
                let width = field_width(field.kind);
                let column = header_and_section + fields.len() * 4 + index * 24;
                bytes[column..column + 2].copy_from_slice(&column_offset.to_le_bytes());
                put32(&mut bytes, column + 16, u32::from(width));
                put32(&mut bytes, column + 20, u32::from(field.array_size));
                column_offset += width * u16::from(field.array_size);
            }
            put32(&mut bytes, 204 + 8, record_offset as u32);
            for &(_, value) in rows {
                let mut record = vec![0; record_size];
                if name == "GameObjectDisplayInfo" {
                    put32(&mut record, field_offset(fields, 1), value);
                }
                bytes.extend_from_slice(&record);
            }
            for &(id, _) in rows {
                bytes.extend_from_slice(&id.to_le_bytes());
            }
            bytes
        }

        fn active_map_db2() -> Vec<u8> {
            let metadata = forever_db2::catalog::resolve_for(
                forever_db2::catalog::WOWDBDEFS_70450_CATALOG_ID,
                "Map",
            )
            .unwrap()
            .unwrap()
            .metadata();
            let fields = &metadata.fields[..metadata.file_field_count];
            let mut bytes = db2_rows("Map", &[(1, 0)]);
            let record_offset = u32::from_le_bytes(bytes[212..216].try_into().unwrap()) as usize;
            let record_size = u32::from_le_bytes(bytes[144..148].try_into().unwrap()) as usize;
            let strings = b"map\0dir\0";
            let insert_at = record_offset + record_size;
            bytes.splice(insert_at..insert_at, strings.iter().copied());
            put32(&mut bytes, 148, strings.len() as u32);
            put32(&mut bytes, 220, strings.len() as u32);
            for (field, logical) in [(0, 0), (1, 4)] {
                let offset = field_offset(fields, field);
                put32(
                    &mut bytes,
                    record_offset + offset,
                    u32::try_from(record_size + logical - offset).unwrap(),
                );
            }
            put32(&mut bytes, record_offset + field_offset(fields, 21), 100);
            put16(
                &mut bytes,
                record_offset + field_offset(fields, 13),
                u16::MAX,
            );
            put16(
                &mut bytes,
                record_offset + field_offset(fields, 14),
                u16::MAX,
            );
            bytes
        }

        fn named_placement(id: u32, name: &[u8]) -> Vec<u8> {
            let mut bytes = vec![1, 7];
            for value in [
                id,
                0,
                0,
                0,
                0,
                0,
                0,
                1.0_f32.to_bits(),
                0,
                0,
                0,
                0,
                0,
                0,
                name.len() as u32,
            ] {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            bytes.extend_from_slice(name);
            bytes
        }

        fn empty_geometry_files(map_id: u32) -> Vec<(String, Vec<u8>)> {
            let assembled = assembly::build(map_id, &[], Default::default()).unwrap();
            let (tree_path, proof_path) = geometry_paths(map_id);
            vec![
                (proof_path, assembled.ordered_dir_bin().to_vec()),
                (tree_path, assembled.tree_bytes().to_vec()),
            ]
        }

        fn synthetic_vmo() -> Vec<u8> {
            let mut bytes = b"VMAP_4.EWMOD".to_vec();
            bytes.extend_from_slice(&8_u32.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            bytes.extend_from_slice(&0_u32.to_le_bytes());
            assert!(vmo::View::parse(&bytes, Default::default()).is_ok());
            bytes
        }

        fn active_map_fixture() -> V47Fixture {
            active_map_fixture_for_plan(V48)
        }

        fn active_map_fixture_for_plan(plan_text: &str) -> V47Fixture {
            let (plan, mut coverage, mut files, _, godi) = fixture_for_plan(plan_text);
            let map = active_map_db2();
            coverage.map.declared_bytes = map.len() as u64;
            coverage.map.available_bytes = map.len() as u64;
            coverage.map.declared_direct_rows = 1;
            coverage.map.declared_copy_records = 0;
            coverage.map.direct_rows = 1;
            coverage.map.copy_records = 0;
            coverage.map.materialized_rows = 1;
            coverage.map.sections = vec![geometry_v22::SectionFact {
                ordinal: 0,
                records: 1,
                copies: 0,
                admitted: true,
            }];
            coverage.maps = vec![geometry_v22::DomainMapFact {
                id: 1,
                parent: None,
                wdt_file_data_id: 100,
                depth: 0,
                is_parent: false,
                root_absent: false,
            }];
            files.extend(empty_geometry_files(1));
            coverage.artifacts = artifact_facts(&files);
            (plan, coverage, files, map, godi)
        }

        fn placement_files(map_id: u32, name: &[u8]) -> Vec<(String, Vec<u8>)> {
            let proof = named_placement(19, name);
            let parsed = placement::DirBin::parse(&proof, Default::default()).unwrap();
            let records = parsed.records().collect::<Vec<_>>();
            let assembled = assembly::build(map_id, &records, Default::default()).unwrap();
            let (tree_path, proof_path) = geometry_paths(map_id);
            let mut files = vec![
                (proof_path, assembled.ordered_dir_bin().to_vec()),
                (tree_path, assembled.tree_bytes().to_vec()),
            ];
            for tile in assembled.tiles() {
                files.push((
                    geometry_tile_path(map_id, tile.x(), tile.y(), true),
                    tile.index_bytes().to_vec(),
                ));
                if let Some(payload) = tile.payload() {
                    files.push((
                        geometry_tile_path(map_id, tile.x(), tile.y(), false),
                        payload.to_vec(),
                    ));
                }
            }
            files
        }

        type V47Fixture = (
            Plan,
            geometry_v22::Coverage,
            Vec<(String, Vec<u8>)>,
            Vec<u8>,
            Vec<u8>,
        );

        fn fixture() -> V47Fixture {
            fixture_for_plan(V47)
        }

        fn fixture_for_plan(plan_text: &str) -> V47Fixture {
            let plan = Plan::parse(plan_text).unwrap();
            let (_, mut coverage, _) = empty_geometry_fixture();
            coverage.context = Context {
                build: 70450,
                locale: plan.locale,
                components: Component::Db2.bit() | Component::Collision.bit(),
                source_sha256: [0x41; 32],
                plan_sha256: integrity::digest(plan.render().as_bytes()),
                producer_sha256: [0x42; 32],
            };
            coverage.selection = MapSelection::All;
            coverage.missing_wdt_policy =
                forever_casc::collision::run::MissingWdtPolicy::DeclareRootAbsent;
            coverage.missing_m2_policy =
                forever_casc::collision::run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2;
            coverage.missing_wmo_policy = forever_casc::collision::run::MissingWmoPolicy::
                DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
            let v48 = plan.is_v48();
            let v49 = plan.is_v49();
            let v50 = plan.is_v50();
            let allows_root_outcomes = v48 || v49 || v50;
            coverage.liquid_object_scope =
                forever_casc::collision::geometry::LiquidObjectScope::OutOfScope;
            coverage.liquid_object_request_made = false;
            coverage.resource_profile = if v49 || v50 {
                forever_casc::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
            } else if v48 {
                forever_casc::collision::resources::Profile::GameObjectMissingRootExpandedV1
            } else {
                forever_casc::collision::resources::Profile::GameObjectExpandedV1
            };
            coverage.gameobject_scope = if v49 || v50 {
                forever_casc::collision::geometry::GameObjectScope::AvailableFdidDirectRootAbsentOrEncryptedUnavailableV1
            } else if v48 {
                forever_casc::collision::geometry::GameObjectScope::AvailableFdidDirectMissingRootV1
            } else {
                forever_casc::collision::geometry::GameObjectScope::AvailableFdidDirectV1
            };
            coverage.gameobject_block_reason =
                forever_casc::collision::geometry::GameObjectBlockReason::None;
            coverage.gameobject_request_made = true;
            coverage.gameobject_startup_reads = 1;
            coverage.model_scope = if v49 || v50 {
                geometry_v22::ModelScope::PlacementAndGameObjectRootAndEncryptedUnavailable
            } else if v48 {
                geometry_v22::ModelScope::PlacementAndGameObjectMissingRoot
            } else {
                geometry_v22::ModelScope::PlacementAndGameObject
            };
            coverage.map = geometry_v22::AvailableMapFact {
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
            };
            coverage.liquids = [
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
            ];
            coverage.maps.clear();
            coverage.artifacts.clear();
            let map = db2("Map", None);
            coverage.map.declared_bytes = map.len() as u64;
            coverage.map.available_bytes = map.len() as u64;
            let missing_root_id = if allows_root_outcomes { 50 } else { 0 };
            let encrypted_unavailable_id = if v49 || v50 { 51 } else { 0 };
            coverage.gameobject_missing_root = if allows_root_outcomes {
                vec![missing_root_id]
            } else {
                Vec::new()
            };
            coverage.gameobject_encrypted_unavailable = if v49 || v50 {
                vec![encrypted_unavailable_id]
            } else {
                Vec::new()
            };
            let godi_rows = if v49 || v50 {
                vec![(7, missing_root_id), (8, encrypted_unavailable_id)]
            } else {
                vec![(7, missing_root_id)]
            };
            let godi = db2_rows("GameObjectDisplayInfo", &godi_rows);
            let tree = gameobject_models::DTREE_MAGIC.to_vec();
            let files = vec![(
                geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH.into(),
                tree.clone(),
            )];
            let row_count = godi_rows.len() as u64;
            let gameobjects = geometry_v22::GameObjectFacts {
                declared_bytes: godi.len() as u64,
                available_bytes: godi.len() as u64,
                full_ckey_verified: true,
                encoded_key_authenticated: true,
                declared_direct_rows: row_count,
                declared_copy_records: 0,
                direct_rows: row_count,
                copy_records: 0,
                materialized_rows: row_count,
                candidate_requests: u64::from(v48) + 2 * u64::from(v49 || v50),
                direct_outcomes: row_count,
                zero_fdids: u64::from(!allows_root_outcomes),
                textures: 0,
                m2_no_collision: 0,
                wmo_doodad_only: 0,
                missing_root_rows: u64::from(allows_root_outcomes),
                missing_root_fdids: if allows_root_outcomes {
                    vec![missing_root_id]
                } else {
                    Vec::new()
                },
                encrypted_unavailable_rows: u64::from(v49 || v50),
                encrypted_unavailable_fdids: if v49 || v50 {
                    vec![encrypted_unavailable_id]
                } else {
                    Vec::new()
                },
                collision_rows: 0,
                list_rows: 0,
                list_m2_rows: 0,
                list_wmo_rows: 0,
                list_vertex_visits: 0,
                list_raw_visit_bytes: 0,
                temp_bytes: gameobject_models::TEMP_MAGIC.len() as u64,
                dtree_bytes: tree.len() as u64,
                temp_sha: integrity::digest(gameobject_models::TEMP_MAGIC),
                dtree_sha: integrity::digest(&tree),
                work_strict_candidates: 0,
                work_partial_candidates: 0,
                work_partial_bytes: 0,
                sections: vec![geometry_v22::GameObjectSectionFact {
                    ordinal: 0,
                    records: row_count as u32,
                    copies: 0,
                    admitted: true,
                }],
                outcomes: godi_rows
                    .iter()
                    .enumerate()
                    .map(|(ordinal, &(display_id, file_data_id))| {
                        geometry_v22::GameObjectOutcomeFact {
                            ordinal: ordinal as u64,
                            display_id,
                            file_data_id,
                            disposition: if file_data_id == 0 {
                                geometry_v22::GameObjectDisposition::ZeroFileDataId
                            } else if (v49 || v50) && file_data_id != missing_root_id {
                                geometry_v22::GameObjectDisposition::EncryptedUnavailable
                            } else {
                                geometry_v22::GameObjectDisposition::MissingRoot
                            },
                        }
                    })
                    .collect(),
            };
            coverage.gameobjects = Some(gameobjects);
            coverage.artifacts = artifact_facts(&files);
            (plan, coverage, files, map, godi)
        }

        fn publish(
            lab: &Lab,
            name: &str,
            plan: &Plan,
            coverage: &geometry_v22::Coverage,
            files: &[(String, Vec<u8>)],
            map: &[u8],
            godi: &[u8],
        ) -> PathBuf {
            let encoded = coverage.encode().unwrap();
            publish_encoded(
                lab,
                name,
                plan,
                coverage.context,
                files,
                map,
                godi,
                &encoded,
                None,
                false,
            )
        }

        #[allow(clippy::too_many_arguments)]
        fn publish_encoded(
            lab: &Lab,
            name: &str,
            plan: &Plan,
            context: Context,
            files: &[(String, Vec<u8>)],
            map: &[u8],
            godi: &[u8],
            coverage_bytes: &[u8],
            liquid_object: Option<&[u8]>,
            partial_liquid_object: bool,
        ) -> PathBuf {
            let CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
                panic!("gameobject fixture requires a disk budget");
            };
            let output = lab.0.join(name);
            let mut run = package::Run::create(
                &output,
                context,
                &[],
                geometry_v22::package_limits_v28_with_extra_files(
                    budget,
                    usize::from(plan.is_v50()),
                ),
            )
            .unwrap();
            run.put(
                Component::Db2,
                &format!("dbc/{}/Map.db2", plan.locale.name()),
                map,
            )
            .unwrap();
            run.put(
                Component::Db2,
                &format!("dbc/{}/GameObjectDisplayInfo.db2", plan.locale.name()),
                godi,
            )
            .unwrap();
            if plan.is_v50() {
                run.put(
                    Component::Db2,
                    &format!("dbc/{}/LiquidObject.db2", plan.locale.name()),
                    liquid_object.expect("v50 fixture requires LiquidObject"),
                )
                .unwrap();
            }
            for (path, bytes) in files {
                run.put(Component::Collision, path, bytes).unwrap();
            }
            let coverage_path = if plan.is_v49() || plan.is_v50() {
                geometry_v22::GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PATH
            } else if plan.is_v48() {
                geometry_v22::GAMEOBJECT_MISSING_ROOT_MATERIAL_FILTERED_WMO_COVERAGE_PATH
            } else {
                geometry_v22::GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PATH
            };
            run.put(Component::Collision, coverage_path, coverage_bytes)
                .unwrap();
            if partial_liquid_object {
                run.set_partial_encrypted_table(
                    &format!("dbc/{}/LiquidObject.db2", plan.locale.name()),
                    &[package::PartialEncryptedSection {
                        lookup: "0123456789abcdef".into(),
                        frame_ordinal: 0,
                        decoded_size: 1,
                    }],
                )
                .unwrap();
            }
            run.finish_component(Component::Db2).unwrap();
            run.finish_component(Component::Collision).unwrap();
            run.finish().unwrap();
            output
        }

        #[test]
        fn sealed_synthetic_v47_package_verifies_through_public_dispatcher() {
            let lab = Lab::new();
            let (plan, coverage, files, map, godi) = fixture();
            let output = publish(&lab, "valid", &plan, &coverage, &files, &map, &godi);
            let summary = crate::verify::verify(&plan, &output, None).unwrap();
            assert_eq!(summary.context, coverage.context);
            assert_eq!(summary.files, 4);

            let (plan, mut rows, files, map, godi) = fixture();
            rows.gameobjects.as_mut().unwrap().outcomes[0].display_id += 1;
            let output = publish(&lab, "rows", &plan, &rows, &files, &map, &godi);
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (plan, mut sections, files, map, godi) = fixture();
            sections.gameobjects.as_mut().unwrap().sections.push(
                geometry_v22::GameObjectSectionFact {
                    ordinal: 1,
                    records: 0,
                    copies: 0,
                    admitted: true,
                },
            );
            let output = publish(&lab, "sections", &plan, &sections, &files, &map, &godi);
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (plan, mut dtree, files, map, godi) = fixture();
            dtree.gameobjects.as_mut().unwrap().dtree_sha[0] ^= 1;
            let output = publish(&lab, "dtree", &plan, &dtree, &files, &map, &godi);
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );
        }

        fn gameobject_missing_root_fixture() -> V47Fixture {
            fixture_for_plan(V48)
        }

        fn mutate_v48_missing_root_count(bytes: &mut [u8], count: u64) {
            let pattern = [
                1_u64.to_le_bytes().as_slice(),
                1_u32.to_le_bytes().as_slice(),
                50_u32.to_le_bytes().as_slice(),
            ]
            .concat();
            let matches: Vec<_> = bytes
                .windows(pattern.len())
                .enumerate()
                .filter_map(|(offset, window)| (window == pattern).then_some(offset))
                .collect();
            assert_eq!(matches.len(), 1);
            bytes[matches[0]..matches[0] + 8].copy_from_slice(&count.to_le_bytes());
        }

        #[test]
        fn sealed_synthetic_v50_checks_liquid_object_evidence_and_partial_sources() {
            let lab = Lab::new();
            let (plan, coverage, files, map, godi) = fixture_for_plan(V50);
            let liquid_object = liquid_object_db2();
            let evidence = crate::driver::liquid_object_v23::Evidence::from_source(&liquid_object)
                .unwrap()
                .encode()
                .unwrap();
            let mut partial_source = liquid_object.clone();
            partial_source[204] = 1;
            let payload_offset =
                u32::from_le_bytes(partial_source[212..216].try_into().unwrap()) as usize;
            partial_source[payload_offset..].fill(0);
            partial_source.splice(payload_offset..payload_offset, [0; 4]);
            partial_source[212..216]
                .copy_from_slice(&u32::try_from(payload_offset + 4).unwrap().to_le_bytes());
            assert_eq!(
                crate::driver::liquid_object_v23::Evidence::from_source(&partial_source)
                    .unwrap_err()
                    .code,
                "product_liquid_object_source_partial"
            );
            let mut packaged_files = files.clone();
            packaged_files.push((
                crate::driver::liquid_object_v23::PATH.to_owned(),
                evidence.clone(),
            ));
            let output = publish_encoded(
                &lab,
                "v50-valid",
                &plan,
                coverage.context,
                &packaged_files,
                &map,
                &godi,
                &coverage.encode().unwrap(),
                Some(&liquid_object),
                false,
            );
            let summary = crate::verify::verify(&plan, &output, None).unwrap();
            assert_eq!(summary.context, coverage.context);
            assert_eq!(summary.files, 6);

            let mut wrong_evidence = evidence;
            wrong_evidence[24] ^= 1;
            let mut wrong_files = files.clone();
            wrong_files.push((
                crate::driver::liquid_object_v23::PATH.to_owned(),
                wrong_evidence,
            ));
            let output = publish_encoded(
                &lab,
                "v50-wrong-evidence",
                &plan,
                coverage.context,
                &wrong_files,
                &map,
                &godi,
                &coverage.encode().unwrap(),
                Some(&liquid_object),
                false,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                "product_liquid_object_evidence"
            );

            let (plan, mut partial_request, files, map, godi) = fixture_for_plan(V50);
            partial_request.liquid_object_request_made = true;
            let mut partial_files = files.clone();
            partial_files.push((
                crate::driver::liquid_object_v23::PATH.to_owned(),
                crate::driver::liquid_object_v23::Evidence::from_source(&liquid_object)
                    .unwrap()
                    .encode()
                    .unwrap(),
            ));
            let output = publish_encoded(
                &lab,
                "v50-partial-request",
                &plan,
                partial_request.context,
                &partial_files,
                &map,
                &godi,
                &partial_request.encode().unwrap(),
                Some(&liquid_object),
                false,
            );
            assert!(crate::verify::verify(&plan, &output, None).is_err());

            let mut partial_manifest_files = files;
            partial_manifest_files.push((
                crate::driver::liquid_object_v23::PATH.to_owned(),
                crate::driver::liquid_object_v23::Evidence::from_source(&liquid_object)
                    .unwrap()
                    .encode()
                    .unwrap(),
            ));
            let output = publish_encoded(
                &lab,
                "v50-partial-manifest",
                &plan,
                coverage.context,
                &partial_manifest_files,
                &map,
                &godi,
                &coverage.encode().unwrap(),
                Some(&liquid_object),
                true,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                "product_liquid_object_source_partial"
            );
        }

        #[test]
        fn sealed_synthetic_v48_missing_root_package_verifies_and_crosses_are_rejected() {
            let lab = Lab::new();
            let (plan, coverage, files, map, godi) = gameobject_missing_root_fixture();
            let output = publish(&lab, "v48-valid", &plan, &coverage, &files, &map, &godi);
            let summary = crate::verify::verify(&plan, &output, None).unwrap();
            assert_eq!(summary.context, coverage.context);
            assert_eq!(summary.files, 4);
            assert!(crate::verify::verify_links(&plan, &output, None).is_ok());
            assert!(verify_selected_geometry_v47(&plan, &output, None).is_err());

            let (plan, mut wrong_id, files, map, godi) = gameobject_missing_root_fixture();
            wrong_id.gameobject_missing_root = vec![51];
            let facts = wrong_id.gameobjects.as_mut().unwrap();
            facts.missing_root_fdids = vec![51];
            facts.outcomes[0].file_data_id = 51;
            let output = publish(
                &lab,
                "v48-wrong-missing-id",
                &plan,
                &wrong_id,
                &files,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (plan, coverage, files, map, godi) = gameobject_missing_root_fixture();
            let mut encoded = coverage.encode().unwrap();
            mutate_v48_missing_root_count(&mut encoded, 2);
            let output = publish_encoded(
                &lab,
                "v48-wrong-missing-count",
                &plan,
                coverage.context,
                &files,
                &map,
                &godi,
                &encoded,
                None,
                false,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                "product_selected_geometry_coverage_schema"
            );

            let (_, mut v47_coverage, _, _, _) = fixture();
            let v47_facts = v47_coverage.gameobjects.as_mut().unwrap();
            v47_facts.candidate_requests = 1;
            v47_facts.zero_fdids = 0;
            v47_facts.missing_root_rows = 1;
            v47_facts.missing_root_fdids = vec![50];
            v47_facts.outcomes[0].file_data_id = 50;
            v47_facts.outcomes[0].disposition = geometry_v22::GameObjectDisposition::MissingRoot;
            v47_coverage.gameobject_missing_root = vec![50];
            assert!(v47_coverage.encode().is_err());
        }

        #[test]
        fn sealed_synthetic_v49_keeps_missing_and_encrypted_unavailable_fdids_distinct() {
            let lab = Lab::new();
            let (plan, coverage, files, map, godi) = fixture_for_plan(V49);
            assert_eq!(coverage.gameobject_missing_root, [50]);
            assert_eq!(coverage.gameobject_encrypted_unavailable, [51]);
            let output = publish(&lab, "v49-valid", &plan, &coverage, &files, &map, &godi);
            let summary = crate::verify::verify(&plan, &output, None).unwrap();
            assert_eq!(summary.context, coverage.context);
            assert_eq!(summary.files, 4);
            assert!(crate::verify::verify_links(&plan, &output, None).is_ok());
            assert!(verify_selected_geometry_v48(&plan, &output, None).is_err());

            let (plan, mut wrong_id, files, map, godi) = fixture_for_plan(V49);
            let facts = wrong_id.gameobjects.as_mut().unwrap();
            wrong_id.gameobject_encrypted_unavailable = vec![52];
            facts.encrypted_unavailable_fdids = vec![52];
            facts.outcomes[1].file_data_id = 52;
            let output = publish(
                &lab,
                "v49-wrong-encrypted-unavailable-id",
                &plan,
                &wrong_id,
                &files,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (_plan, mut mismatched_list, _, map, godi) = fixture_for_plan(V49);
            mismatched_list.gameobject_encrypted_unavailable = vec![52];
            assert_eq!(
                verify_gameobject_coverage(
                    &mismatched_list,
                    authenticate_v28_map(&map).unwrap(),
                    authenticate_v47_gameobjects(&godi).unwrap(),
                    true,
                    true,
                    &[],
                    &[],
                )
                .unwrap_err()
                .code,
                BAD
            );

            let (plan, mut v48, _, _, _) = fixture_for_plan(V48);
            let v48_facts = v48.gameobjects.as_mut().unwrap();
            v48_facts.zero_fdids = 0;
            v48_facts.missing_root_rows = 0;
            v48_facts.missing_root_fdids.clear();
            v48_facts.encrypted_unavailable_rows = 1;
            v48_facts.encrypted_unavailable_fdids = vec![51];
            v48_facts.outcomes[0].file_data_id = 51;
            v48_facts.outcomes[0].disposition =
                geometry_v22::GameObjectDisposition::EncryptedUnavailable;
            v48.gameobject_missing_root.clear();
            v48.gameobject_encrypted_unavailable = vec![51];
            assert!(plan.is_v48());
            assert!(v48.encode().is_err());
        }

        #[test]
        fn v49_rejects_encrypted_unavailable_models_in_artifacts_or_placements() {
            let lab = Lab::new();
            let (plan, mut artifact_coverage, files, map, godi) = fixture_for_plan(V49);
            let id = artifact_coverage.gameobject_encrypted_unavailable[0];
            let mut with_model = files;
            with_model.push((
                format!("vmaps/{}.vmo", geometry_v22::m2_file_name(id)),
                synthetic_vmo(),
            ));
            artifact_coverage.artifacts = artifact_facts(&with_model);
            let output = publish(
                &lab,
                "v49-encrypted-model-artifact",
                &plan,
                &artifact_coverage,
                &with_model,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (plan, mut placement_coverage, files, map, godi) = active_map_fixture_for_plan(V49);
            let id = placement_coverage.gameobject_encrypted_unavailable[0];
            let unavailable_name = geometry_v22::m2_file_name(id);
            let mut with_placement = placement_files(1, unavailable_name.as_bytes());
            with_placement.extend(
                files
                    .into_iter()
                    .filter(|(path, _)| path == geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH),
            );
            with_placement.push(("vmaps/present.vmo".into(), synthetic_vmo()));
            placement_coverage.artifacts = artifact_facts(&with_placement);
            let output = publish(
                &lab,
                "v49-encrypted-model-placement",
                &plan,
                &placement_coverage,
                &with_placement,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );
        }

        #[test]
        fn v48_replay_caps_missing_root_direct_rows_with_a_deduplicated_fdid_list() {
            let lab = Lab::new();
            let (plan, mut coverage, files, map, _) = gameobject_missing_root_fixture();
            let rows: Vec<_> = (0..=forever_casc::collision::gameobjects::MAX_CANDIDATE_REQUESTS)
                .map(|index| (7 + index as u32, 50))
                .collect();
            let godi = db2_rows("GameObjectDisplayInfo", &rows);
            let facts = coverage.gameobjects.as_mut().unwrap();
            let row_count = rows.len() as u64;
            facts.declared_bytes = godi.len() as u64;
            facts.available_bytes = godi.len() as u64;
            facts.declared_direct_rows = row_count;
            facts.direct_rows = row_count;
            facts.materialized_rows = row_count;
            facts.candidate_requests = row_count;
            facts.direct_outcomes = row_count;
            facts.zero_fdids = 0;
            facts.missing_root_rows = row_count;
            facts.missing_root_fdids = vec![50];
            facts.sections[0].records = rows.len() as u32;
            facts.outcomes = rows
                .iter()
                .enumerate()
                .map(
                    |(ordinal, &(display_id, file_data_id))| geometry_v22::GameObjectOutcomeFact {
                        ordinal: ordinal as u64,
                        display_id,
                        file_data_id,
                        disposition: geometry_v22::GameObjectDisposition::MissingRoot,
                    },
                )
                .collect();

            let output = publish(
                &lab,
                "v48-duplicate-missing-root-over-limit",
                &plan,
                &coverage,
                &files,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                LIMIT
            );
        }

        #[test]
        fn v48_missing_root_rejects_an_unavailable_model_in_artifacts_or_placements() {
            let lab = Lab::new();
            let (plan, mut artifact_coverage, files, map, godi) = gameobject_missing_root_fixture();
            let id = artifact_coverage.gameobject_missing_root[0];
            let mut with_model = files;
            with_model.push((
                format!("vmaps/{}.vmo", geometry_v22::m2_file_name(id)),
                synthetic_vmo(),
            ));
            artifact_coverage.artifacts = artifact_facts(&with_model);
            let output = publish(
                &lab,
                "v48-missing-model-artifact",
                &plan,
                &artifact_coverage,
                &with_model,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );

            let (plan, mut placement_coverage, files, map, godi) = active_map_fixture();
            let id = placement_coverage.gameobject_missing_root[0];
            let missing_name = geometry_v22::m2_file_name(id);
            let mut with_placement = placement_files(1, missing_name.as_bytes());
            with_placement.extend(
                files
                    .into_iter()
                    .filter(|(path, _)| path == geometry_v22::GAMEOBJECT_MODELS_DTREE_PATH),
            );
            with_placement.push(("vmaps/present.vmo".into(), synthetic_vmo()));
            placement_coverage.artifacts = artifact_facts(&with_placement);
            let output = publish(
                &lab,
                "v48-missing-model-placement",
                &plan,
                &placement_coverage,
                &with_placement,
                &map,
                &godi,
            );
            assert_eq!(
                crate::verify::verify(&plan, &output, None)
                    .unwrap_err()
                    .code,
                BAD
            );
        }

        #[test]
        fn gameobject_list_verification_accepts_m2_and_wmo_rows() {
            for (list_m2_rows, list_wmo_rows) in [(1, 0), (0, 1)] {
                let (plan, mut coverage, _, map, _) = fixture();
                let godi = db2("GameObjectDisplayInfo", Some((7, 100)));
                let gameobjects = coverage.gameobjects.as_mut().unwrap();
                gameobjects.declared_bytes = godi.len() as u64;
                gameobjects.available_bytes = godi.len() as u64;
                gameobjects.candidate_requests = 1;
                gameobjects.zero_fdids = 0;
                gameobjects.collision_rows = 1;
                gameobjects.list_rows = 1;
                gameobjects.list_m2_rows = list_m2_rows;
                gameobjects.list_wmo_rows = list_wmo_rows;
                gameobjects.outcomes[0].file_data_id = 100;
                gameobjects.outcomes[0].disposition =
                    geometry_v22::GameObjectDisposition::Collision;

                verify_gameobject_coverage(
                    &coverage,
                    authenticate_v28_map(&map).unwrap(),
                    authenticate_v47_gameobjects(&godi).unwrap(),
                    false,
                    false,
                    &[],
                    &[],
                )
                .unwrap();
                assert!(plan.is_v47());
            }

            let (plan, mut mismatched, _, map, _) = fixture();
            let godi = db2("GameObjectDisplayInfo", Some((7, 100)));
            let gameobjects = mismatched.gameobjects.as_mut().unwrap();
            gameobjects.declared_bytes = godi.len() as u64;
            gameobjects.available_bytes = godi.len() as u64;
            gameobjects.candidate_requests = 1;
            gameobjects.zero_fdids = 0;
            gameobjects.collision_rows = 1;
            gameobjects.list_rows = 1;
            gameobjects.list_m2_rows = 1;
            gameobjects.list_wmo_rows = 1;
            gameobjects.outcomes[0].file_data_id = 100;
            gameobjects.outcomes[0].disposition = geometry_v22::GameObjectDisposition::Collision;

            assert_eq!(
                verify_gameobject_coverage(
                    &mismatched,
                    authenticate_v28_map(&map).unwrap(),
                    authenticate_v47_gameobjects(&godi).unwrap(),
                    false,
                    false,
                    &[],
                    &[],
                )
                .unwrap_err()
                .code,
                BAD
            );
            assert!(plan.is_v47());
        }
    }
}
