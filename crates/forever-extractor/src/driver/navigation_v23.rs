// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-20: add selected-input v23 navigation publication.
//! Linux-only selected navigation from sealed v20 terrain and v22 VMap packages.
//! Input coverage remains producer-reported; this profile adds no whole-world,
//! format-parity, runtime, or client-completeness claim.
#![cfg(target_os = "linux")]

use super::{Error, Plan, Selection};
use crate::{
    driver::{self, collision_geometry_v22, terrain_v20},
    plan::SchemaCatalog,
};
use forever_casc::{
    blte::available::{CoverageRange, RangeState},
    integrity,
    local_store::{LocalStore, OpenOptions as CascOpenOptions, availability::AvailableSource},
    package::{self, Artifact, Component, Context, Run, Summary},
    terrain::available_70450::WdtDisposition,
};
use forever_db2::{
    availability::{self, AvailableRecords},
    catalog, terrain_sources,
};
pub(crate) use forever_navigation::package_v23::COVERAGE_PATH;
use forever_navigation::{
    inventory::fs as navigation_input,
    mmap, models,
    package_v23::{
        self, ClosureMap, Coverage, LiquidSnapshot, MAP_LIMIT, MAP_RANGES_MAX, MAP_ROWS_MAX,
        MapSnapshot, PackageIdentity, SourceIdentity, TerrainMap, TileOutput, closure_depths,
        terrain_materialized_rows_match, vmap_materialized_rows_match,
    },
    tilebuilder::Coordinate,
};

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Component as PathComponent, Path, PathBuf},
};

const DB2_LIMIT: u64 = 16 * 1024 * 1024;
const MAP_DOMAIN: &[u8] = b"rustycore-v23-map-available-snapshot-v2\0";
const MAP_DECODED_FACTS_DOMAIN: &[u8] = b"rustycore-v23-map-decoded-facts-v2\0";

#[cfg(test)]
#[path = "navigation_v23/coverage_tests.rs"]
mod coverage_tests;
#[cfg(test)]
#[path = "navigation_v23/input_plan_tests.rs"]
mod input_plan_tests;

const INPUT_COVERAGE_VMAP: &str = "vmaps/selected-geometry-coverage-v1.bin";
const INPUT_COVERAGE_TERRAIN: &str = terrain_v20::COVERAGE_PATH;
const MAX_INPUT_FILES: usize = 262_144;
const MAX_INPUT_PACKAGE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_COVERAGE_VMAP: usize = 128 * 1024 * 1024;
const MAP_SNAPSHOT_MAX: usize = MAP_LIMIT as usize + MAP_RANGES_MAX * 32 + 4096;
fn offline_output_limits() -> package::Limits {
    package::Limits {
        total_bytes: 1024 * 1024 * 1024 * 1024,
        ..package::Limits::default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileIdentity {
    dev: u64,
    ino: u64,
    mode: u32,
    nlink: u64,
    uid: u32,
    gid: u32,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl FileIdentity {
    fn of(metadata: &fs::Metadata) -> Self {
        Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            mode: metadata.mode(),
            nlink: metadata.nlink(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            len: metadata.len(),
            mtime: metadata.mtime(),
            mtime_ns: metadata.mtime_nsec(),
            ctime: metadata.ctime(),
            ctime_ns: metadata.ctime_nsec(),
        }
    }
}

struct PlanGuard {
    file: File,
    path: PathBuf,
    identity: FileIdentity,
    plan: Plan,
    sha256: [u8; 32],
}
impl PlanGuard {
    fn open(path: &Path) -> Result<Self, Error> {
        let path = absolute_without_symlinks(path, true)?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(0x20000 | 0x800)
            .open(&path)
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let before = file
            .metadata()
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        if !before.is_file()
            || before.nlink() != 1
            || before.len() > crate::plan::MAX_PLAN_BYTES as u64
        {
            return Err("product_navigation_input_plan".into());
        }
        let size = usize::try_from(before.len())
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let capacity = size.checked_add(1).ok_or("product_navigation_input_plan")?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| Error::from("product_allocation"))?;
        if bytes.capacity() > capacity {
            return Err("product_navigation_input_plan".into());
        }
        bytes.resize(capacity, 0);
        file.read_exact(&mut bytes[..size])
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        if file
            .read(&mut bytes[size..])
            .map_err(|_| Error::from("product_navigation_input_plan"))?
            != 0
        {
            return Err("product_navigation_input_plan".into());
        }
        let after = file
            .metadata()
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let named = fs::symlink_metadata(&path)
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let id = FileIdentity::of(&before);
        if id != FileIdentity::of(&after) || id != FileIdentity::of(&named) {
            return Err("product_input_changed".into());
        }
        let text = std::str::from_utf8(&bytes[..size])
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let plan = Plan::parse(text).map_err(Error::from)?;
        if plan.render().as_bytes() != &bytes[..size] {
            return Err("product_navigation_input_plan".into());
        }
        let sha256 = integrity::digest(&bytes[..size]);
        let mut guard = Self {
            file,
            path,
            identity: id,
            plan,
            sha256,
        };
        guard.verify()?;
        Ok(guard)
    }

    fn verify(&mut self) -> Result<(), Error> {
        let fd = self
            .file
            .metadata()
            .map_err(|_| Error::from("product_input_changed"))?;
        let named =
            fs::symlink_metadata(&self.path).map_err(|_| Error::from("product_input_changed"))?;
        if FileIdentity::of(&fd) != self.identity || FileIdentity::of(&named) != self.identity {
            return Err("product_input_changed".into());
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::from("product_input_changed"))?;
        let n = usize::try_from(self.identity.len)
            .map_err(|_| Error::from("product_navigation_input_plan"))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(n.saturating_add(1))
            .map_err(|_| Error::from("product_allocation"))?;
        if bytes.capacity() > n.saturating_add(1) {
            return Err("product_navigation_input_plan".into());
        }
        bytes.resize(n.saturating_add(1), 0);
        self.file
            .read_exact(&mut bytes[..n])
            .map_err(|_| Error::from("product_input_changed"))?;
        if self
            .file
            .read(&mut bytes[n..])
            .map_err(|_| Error::from("product_input_changed"))?
            != 0
            || integrity::digest(&bytes[..n]) != self.sha256
            || FileIdentity::of(
                &self
                    .file
                    .metadata()
                    .map_err(|_| Error::from("product_input_changed"))?,
            ) != self.identity
            || FileIdentity::of(
                &fs::symlink_metadata(&self.path)
                    .map_err(|_| Error::from("product_input_changed"))?,
            ) != self.identity
        {
            return Err("product_input_changed".into());
        }
        Ok(())
    }
}

struct VerifiedTerrain {
    identity: PackageIdentity,
    summary: Summary,
    artifacts: Vec<Artifact>,
    coverage: terrain_v20::Coverage,
}
struct VerifiedVmap {
    identity: PackageIdentity,
    summary: Summary,
    artifacts: Vec<Artifact>,
    coverage: collision_geometry_v22::Coverage,
}
struct VerifiedPackagePayload {
    summary: Summary,
    artifacts: Vec<Artifact>,
    coverage_sha256: [u8; 32],
    coverage_bytes: Vec<u8>,
}

fn package_identity(
    summary: Summary,
    plan_sha256: [u8; 32],
    coverage_sha256: [u8; 32],
) -> PackageIdentity {
    PackageIdentity {
        context: summary.context,
        plan_sha256,
        files: summary.files as u64,
        bytes: summary.bytes,
        inventory_sha256: summary.inventory_sha256,
        coverage_sha256,
    }
}

fn verify_package_payload(
    root: &Path,
    context: Context,
    component: Component,
    coverage_path: &str,
    max_coverage: usize,
) -> Result<VerifiedPackagePayload, Error> {
    let mut found = None;
    let mut admitted = Vec::new();
    let mut coverage_bytes = Vec::new();
    let summary = package::verify_with_preflight(
        root,
        Some(context),
        package::Limits {
            total_bytes: MAX_INPUT_PACKAGE_BYTES,
            ..package::Limits::default()
        },
        |artifacts| {
            if artifacts.len() > MAX_INPUT_FILES || artifacts.is_empty() {
                return Err(package::Error::Limit);
            }
            admitted
                .try_reserve_exact(artifacts.len())
                .map_err(|_| package::Error::Allocation)?;
            if admitted.capacity() != artifacts.len() {
                return Err(package::Error::Limit);
            }
            let mut total = 0_u64;
            let mut matches = 0;
            for artifact in artifacts {
                if artifact.component != component {
                    return Err(package::Error::Component);
                }
                total = total
                    .checked_add(artifact.bytes)
                    .ok_or(package::Error::Limit)?;
                if artifact.path == coverage_path {
                    matches += 1;
                    if artifact.bytes == 0 || artifact.bytes > max_coverage as u64 {
                        return Err(package::Error::Limit);
                    }
                    found = Some(artifact.clone());
                }
                admitted.push(artifact.clone());
            }
            if matches != 1 || total > MAX_INPUT_PACKAGE_BYTES {
                return Err(package::Error::Inventory);
            }
            Ok(())
        },
        |artifact, file| {
            if artifact.path == coverage_path {
                let n = usize::try_from(artifact.bytes).map_err(|_| package::Error::Limit)?;
                coverage_bytes
                    .try_reserve_exact(n)
                    .map_err(|_| package::Error::Allocation)?;
                if coverage_bytes.capacity() > n {
                    return Err(package::Error::Limit);
                }
                coverage_bytes.resize(n, 0);
                file.read_exact(&mut coverage_bytes)
                    .map_err(|_| package::Error::Io)?;
            }
            Ok(())
        },
    )?;
    let artifact = found.ok_or("product_navigation_coverage")?;
    if coverage_bytes.len() as u64 != artifact.bytes {
        return Err("product_navigation_coverage".into());
    }
    Ok(VerifiedPackagePayload {
        summary,
        artifacts: admitted,
        coverage_sha256: artifact.sha256,
        coverage_bytes,
    })
}

fn prevalidate_terrain(root: &Path, plan: &PlanGuard) -> Result<VerifiedTerrain, Error> {
    if !plan.plan.is_v20() || plan.plan.build != 70450 {
        return Err("product_navigation_input_profile".into());
    }
    let semantic = crate::verify::verify_links(&plan.plan, root, None)?;
    let VerifiedPackagePayload {
        summary,
        artifacts,
        coverage_sha256,
        coverage_bytes,
    } = verify_package_payload(
        root,
        semantic.context,
        Component::Terrain,
        INPUT_COVERAGE_TERRAIN,
        terrain_v20::COVERAGE_BYTES,
    )?;
    if summary != semantic || summary.context.plan_sha256 != plan.sha256 {
        return Err("product_navigation_input_context".into());
    }
    let coverage = terrain_v20::Coverage::parse(&coverage_bytes)?;
    if coverage.context != summary.context {
        return Err("product_navigation_coverage".into());
    }
    Ok(VerifiedTerrain {
        identity: package_identity(summary, plan.sha256, coverage_sha256),
        summary,
        artifacts,
        coverage,
    })
}

fn prevalidate_vmap(root: &Path, plan: &PlanGuard) -> Result<VerifiedVmap, Error> {
    if !plan.plan.is_v22() || plan.plan.build != 70450 {
        return Err("product_navigation_input_profile".into());
    }
    let semantic = crate::verify::verify_links(&plan.plan, root, None)?;
    let VerifiedPackagePayload {
        summary,
        artifacts,
        coverage_sha256,
        coverage_bytes,
    } = verify_package_payload(
        root,
        semantic.context,
        Component::Collision,
        INPUT_COVERAGE_VMAP,
        MAX_COVERAGE_VMAP,
    )?;
    if summary != semantic || summary.context.plan_sha256 != plan.sha256 {
        return Err("product_navigation_input_context".into());
    }
    let coverage = collision_geometry_v22::Coverage::parse(&coverage_bytes)?;
    if coverage.context != summary.context {
        return Err("product_navigation_coverage".into());
    }
    Ok(VerifiedVmap {
        identity: package_identity(summary, plan.sha256, coverage_sha256),
        summary,
        artifacts,
        coverage,
    })
}

fn absolute_without_symlinks(path: &Path, regular_file: bool) -> Result<PathBuf, Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| Error::from("product_navigation_path"))?
            .join(path)
    };
    if absolute.as_os_str().len() > 4096
        || absolute
            .components()
            .any(|component| matches!(component, PathComponent::ParentDir))
    {
        return Err("product_navigation_path".into());
    }
    let mut walk = PathBuf::from("/");
    let mut components = absolute
        .components()
        .filter(|part| !matches!(part, PathComponent::RootDir | PathComponent::CurDir))
        .peekable();
    while let Some(component) = components.next() {
        walk.push(component);
        let metadata =
            fs::symlink_metadata(&walk).map_err(|_| Error::from("product_navigation_path"))?;
        if metadata.file_type().is_symlink() {
            return Err("product_navigation_path".into());
        }
        if components.peek().is_some() && !metadata.is_dir() {
            return Err("product_navigation_path".into());
        }
    }
    let metadata =
        fs::symlink_metadata(&walk).map_err(|_| Error::from("product_navigation_path"))?;
    if (regular_file && !metadata.is_file()) || (!regular_file && !metadata.is_dir()) {
        return Err("product_navigation_path".into());
    }
    fs::canonicalize(&absolute).map_err(|_| Error::from("product_navigation_path"))
}

fn output_path(output: &Path) -> Result<PathBuf, Error> {
    if output.as_os_str().is_empty() || output.as_os_str().len() > 4096 {
        return Err("product_navigation_path".into());
    }
    let absolute = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|_| Error::from("product_navigation_path"))?
            .join(output)
    };
    if absolute
        .components()
        .any(|component| matches!(component, PathComponent::ParentDir))
    {
        return Err("product_navigation_path".into());
    }
    let name = absolute.file_name().ok_or("product_navigation_path")?;
    let parent = absolute.parent().ok_or("product_navigation_path")?;
    let parent = absolute_without_symlinks(parent, false)?;
    let candidate = parent.join(name);
    match fs::symlink_metadata(&candidate) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err("product_navigation_path".into());
            }
            absolute_without_symlinks(&candidate, false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(candidate),
        Err(_) => Err("product_navigation_path".into()),
    }
}

fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}

fn validate_input_output_separation(
    terrain_root: &Path,
    terrain_plan: &Path,
    vmap_root: &Path,
    vmap_plan: &Path,
    output: &Path,
) -> Result<(PathBuf, [PathBuf; 2]), Error> {
    let terrain_root = absolute_without_symlinks(terrain_root, false)?;
    let vmap_root = absolute_without_symlinks(vmap_root, false)?;
    let terrain_plan = absolute_without_symlinks(terrain_plan, true)?;
    let vmap_plan = absolute_without_symlinks(vmap_plan, true)?;
    let output = output_path(output)?;
    if terrain_root == vmap_root
        || overlaps(&terrain_root, &vmap_root)
        || overlaps(&terrain_root, &terrain_plan)
        || overlaps(&terrain_root, &vmap_plan)
        || overlaps(&vmap_root, &terrain_plan)
        || overlaps(&vmap_root, &vmap_plan)
        || terrain_plan == vmap_plan
        || [&terrain_root, &vmap_root, &terrain_plan, &vmap_plan]
            .iter()
            .any(|input| overlaps(&output, input))
    {
        return Err("product_navigation_path_overlap".into());
    }
    Ok((output, [terrain_root, vmap_root]))
}

fn append_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_map_range_snapshot<'a>(
    declared: u64,
    available: u64,
    full_ckey_verified: bool,
    encoded_key_authenticated: bool,
    ranges: &[CoverageRange],
    mut read_range: impl FnMut(u64, usize) -> Result<&'a [u8], Error>,
) -> Result<Vec<u8>, Error> {
    if declared == 0
        || declared > MAP_LIMIT
        || available > declared
        || ranges.is_empty()
        || ranges.len() > MAP_RANGES_MAX
        || (!full_ckey_verified && !encoded_key_authenticated)
        || (full_ckey_verified && available != declared)
    {
        return Err("product_navigation_map_source".into());
    }
    let capacity = usize::try_from(declared)
        .ok()
        .and_then(|n| n.checked_add(ranges.len().checked_mul(32)?))
        .and_then(|n| n.checked_add(64))
        .filter(|n| *n <= MAP_SNAPSHOT_MAX)
        .ok_or("product_navigation_map_source")?;
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(capacity)
        .map_err(|_| Error::from("product_allocation"))?;
    if snapshot.capacity() > capacity {
        return Err("product_navigation_map_source".into());
    }
    snapshot.extend_from_slice(MAP_DOMAIN);
    append_u64(&mut snapshot, declared);
    append_u64(&mut snapshot, available);
    snapshot.push(u8::from(full_ckey_verified));
    snapshot.push(u8::from(encoded_key_authenticated));
    append_u64(&mut snapshot, ranges.len() as u64);
    let mut offset = 0_u64;
    let mut available_total = 0_u64;
    for range in ranges {
        let end = range
            .offset
            .checked_add(range.bytes)
            .ok_or("product_navigation_map_source")?;
        if range.offset != offset || end > declared {
            return Err("product_navigation_map_source".into());
        }
        append_u64(&mut snapshot, range.offset);
        append_u64(&mut snapshot, range.bytes);
        snapshot.push(match range.state {
            RangeState::Available => 1,
            RangeState::EncryptedUnavailable => 2,
        });
        if range.state == RangeState::Available {
            if range.bytes != 0 {
                let n = usize::try_from(range.bytes)
                    .map_err(|_| Error::from("product_navigation_map_source"))?;
                let bytes = read_range(range.offset, n)?;
                if bytes.len() != n {
                    return Err("product_navigation_map_source".into());
                }
                snapshot.extend_from_slice(bytes);
            }
            available_total = available_total
                .checked_add(range.bytes)
                .ok_or("product_navigation_map_source")?;
        }
        // Zero-length frames remain ordered facts at this offset. They do not
        // request an empty capability read or move the decoded cursor.
        offset = end;
    }
    if offset != declared || available_total != available || snapshot.len() > MAP_SNAPSHOT_MAX {
        return Err("product_navigation_map_source".into());
    }
    Ok(snapshot)
}

fn map_decoded_facts_sha256(
    records: &AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    terrain_materialized_rows: usize,
    vmap_materialized_rows: usize,
) -> Result<[u8; 32], Error> {
    let section_count = records.sections().len();
    let direct_count = records.direct_records().len();
    let copy_count = records.copies().len();
    let record_capacity = section_count
        .checked_mul(13)
        .and_then(|sections| {
            MAP_DECODED_FACTS_DOMAIN
                .len()
                .checked_add(40)?
                .checked_add(sections)
        })
        .and_then(|n| n.checked_add(direct_count.checked_mul(14)?))
        .and_then(|n| n.checked_add(copy_count.checked_mul(8)?))
        .filter(|n| *n <= 4 * 1024 * 1024)
        .ok_or("product_navigation_map_source")?;
    if direct_count > MAP_ROWS_MAX
        || copy_count > MAP_ROWS_MAX
        || terrain_materialized_rows > MAP_ROWS_MAX
        || vmap_materialized_rows > MAP_ROWS_MAX
    {
        return Err("product_navigation_map_source".into());
    }
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(record_capacity)
        .map_err(|_| Error::from("product_allocation"))?;
    decoded.extend_from_slice(MAP_DECODED_FACTS_DOMAIN);
    append_u64(&mut decoded, terrain_materialized_rows as u64);
    append_u64(&mut decoded, vmap_materialized_rows as u64);
    append_u64(&mut decoded, records.sections().len() as u64);
    for section in records.sections() {
        append_u32(&mut decoded, section.ordinal);
        append_u32(&mut decoded, section.declared_records);
        append_u32(&mut decoded, section.declared_copies);
        decoded.push(match section.state {
            forever_db2::availability::SectionState::Admitted => 1,
            forever_db2::availability::SectionState::UnknownTact => 2,
        });
    }
    append_u64(&mut decoded, records.direct_records().len() as u64);
    for row in records.direct_records() {
        append_u32(&mut decoded, row.id);
        decoded.push(row.fields.map_type);
        decoded.push(row.fields.instance_type as u8);
        decoded.extend_from_slice(&row.fields.parent_map_id.to_le_bytes());
        decoded.extend_from_slice(&row.fields.cosmetic_parent_map_id.to_le_bytes());
        decoded.extend_from_slice(&row.fields.flags.to_le_bytes());
    }
    append_u64(&mut decoded, records.copies().len() as u64);
    for copy in records.copies() {
        append_u32(&mut decoded, copy.new_id);
        append_u32(&mut decoded, copy.source_id);
    }
    if decoded.len() != record_capacity {
        return Err("product_navigation_map_source".into());
    }
    Ok(integrity::digest(&decoded))
}

fn map_snapshot(
    source: &AvailableSource,
    records: &AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    terrain_materialized_rows: usize,
    vmap_materialized_rows: usize,
) -> Result<MapSnapshot, Error> {
    let declared = source.declared_bytes();
    let available = source.available_bytes();
    let ranges = source.coverage();
    if source.file_data_id() == 0
        || declared == 0
        || declared > MAP_LIMIT
        || available > declared
        || ranges.is_empty()
        || ranges.len() > MAP_RANGES_MAX
        || (!source.full_ckey_verified() && !source.encoded_key_authenticated())
        || (source.full_ckey_verified() && available != declared)
    {
        return Err("product_navigation_map_source".into());
    }
    let snapshot = append_map_range_snapshot(
        declared,
        available,
        source.full_ckey_verified(),
        source.encoded_key_authenticated(),
        ranges,
        |offset, bytes| {
            source
                .range(offset, bytes)
                .map_err(|_| Error::from("product_navigation_map_source"))
        },
    )?;
    let decoded_sha256 =
        map_decoded_facts_sha256(records, terrain_materialized_rows, vmap_materialized_rows)?;
    let declared_direct_rows = records
        .sections()
        .iter()
        .try_fold(0usize, |total, section| {
            total.checked_add(section.declared_records as usize)
        })
        .ok_or("product_navigation_map_source")?;
    let declared_copy_pairs = records
        .sections()
        .iter()
        .try_fold(0usize, |total, section| {
            total.checked_add(section.declared_copies as usize)
        })
        .ok_or("product_navigation_map_source")?;
    let materialized_max = records
        .direct_records()
        .len()
        .checked_add(records.copies().len())
        .ok_or("product_navigation_map_source")?;
    if declared_direct_rows > MAP_ROWS_MAX
        || declared_copy_pairs > MAP_ROWS_MAX
        || records.direct_records().len() > declared_direct_rows
        || records.copies().len() > declared_copy_pairs
        || vmap_materialized_rows < records.direct_records().len()
        || terrain_materialized_rows > materialized_max
        || vmap_materialized_rows > materialized_max
    {
        return Err("product_navigation_map_source".into());
    }
    Ok(MapSnapshot {
        file_data_id: source.file_data_id(),
        declared_bytes: declared,
        available_bytes: available,
        full_ckey_verified: source.full_ckey_verified(),
        encoded_key_authenticated: source.encoded_key_authenticated(),
        range_count: ranges.len(),
        section_count: records.sections().len(),
        declared_direct_rows,
        declared_copy_pairs,
        direct_rows: records.direct_records().len(),
        copy_pairs: records.copies().len(),
        terrain_materialized_rows,
        vmap_materialized_rows,
        decoded_sha256,
        snapshot_sha256: integrity::digest(&snapshot),
    })
}

struct MapInputs {
    navigation: AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    vmap_wdt_ids: BTreeMap<u32, u32>,
    global_parent_ids: BTreeSet<u32>,
    snapshot: MapSnapshot,
}

struct MaterializedMapProjections {
    terrain_rows: usize,
    vmap_rows: usize,
    global_parent_ids: BTreeSet<u32>,
}

fn validate_map_reader_parity(
    navigation: &AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    terrain: &AvailableRecords<forever_db2::terrain_sources::MapFields>,
    vmap: &AvailableRecords<forever_db2::terrain_sources::VMapFields>,
) -> Result<(), Error> {
    if navigation.sections() != terrain.sections()
        || navigation.sections() != vmap.sections()
        || navigation.direct_records().len() != terrain.direct_records().len()
        || navigation.direct_records().len() != vmap.direct_records().len()
        || navigation.copies().len() != terrain.copies().len()
        || navigation.copies().len() != vmap.copies().len()
        || navigation
            .direct_records()
            .iter()
            .zip(terrain.direct_records())
            .any(|(left, right)| left.id != right.id)
        || navigation
            .direct_records()
            .iter()
            .zip(vmap.direct_records())
            .any(|(left, right)| left.id != right.id)
        || navigation
            .copies()
            .iter()
            .zip(terrain.copies())
            .any(|(left, right)| left.new_id != right.new_id || left.source_id != right.source_id)
        || navigation
            .copies()
            .iter()
            .zip(vmap.copies())
            .any(|(left, right)| left.new_id != right.new_id || left.source_id != right.source_id)
    {
        return Err("product_navigation_map_source".into());
    }
    Ok(())
}

fn materialize_map_projections(
    terrain: AvailableRecords<forever_db2::terrain_sources::MapFields>,
    vmap: AvailableRecords<forever_db2::terrain_sources::VMapFields>,
) -> Result<MaterializedMapProjections, Error> {
    let materialized_vmap = vmap
        .materialize()
        .map_err(|error| Error::from(error.code()))?;
    let mut global_parent_ids = BTreeSet::new();
    for row in materialized_vmap.rows() {
        if row.fields.parent_map_id >= 0 {
            global_parent_ids.insert(row.fields.parent_map_id as u32);
        }
    }
    let vmap_rows = materialized_vmap.rows().len();
    let terrain_materialized = terrain
        .materialize()
        .map_err(|error| Error::from(error.code()))?;
    let terrain_rows = terrain_materialized.rows().len();
    drop(terrain_materialized);
    drop(materialized_vmap);
    Ok(MaterializedMapProjections {
        terrain_rows,
        vmap_rows,
        global_parent_ids,
    })
}

fn load_map_inputs(store: &mut LocalStore) -> Result<MapInputs, Error> {
    let map_id = catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), "Map")
        .map_err(|_| Error::from("product_navigation_identity"))?
        .ok_or("product_navigation_identity")?
        .metadata()
        .file_data_id;
    let source = store
        .read_file_data_id_available_70450_with_limit(map_id, MAP_LIMIT)
        .map_err(|error| Error::from(error.code()))?;
    if source.file_data_id() != map_id {
        return Err("product_navigation_map_source".into());
    }
    let navigation = availability::read_navigation_maps_70450(&source)
        .map_err(|error| Error::from(error.code()))?;
    let terrain =
        availability::read_map_70450(&source).map_err(|error| Error::from(error.code()))?;
    let vmap =
        availability::read_vmap_maps_70450(&source).map_err(|error| Error::from(error.code()))?;
    validate_map_reader_parity(&navigation, &terrain, &vmap)?;
    let mut vmap_wdt_ids = BTreeMap::new();
    for row in vmap.direct_records() {
        if vmap_wdt_ids
            .insert(row.id, row.fields.wdt_file_data_id as u32)
            .is_some()
        {
            return Err("product_navigation_map_source".into());
        }
    }
    let materialized = materialize_map_projections(terrain, vmap)?;
    let snapshot = map_snapshot(
        &source,
        &navigation,
        materialized.terrain_rows,
        materialized.vmap_rows,
    )?;
    Ok(MapInputs {
        navigation,
        vmap_wdt_ids,
        global_parent_ids: materialized.global_parent_ids,
        snapshot,
    })
}

fn liquid_snapshot(
    store: &mut LocalStore,
) -> Result<(LiquidSnapshot, models::liquid::Table), Error> {
    let file_data_id = catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), "LiquidType")
        .map_err(|_| Error::from("product_navigation_identity"))?
        .ok_or("product_navigation_identity")?
        .metadata()
        .file_data_id;
    let file = store
        .read_file_data_id_with_limit(file_data_id, DB2_LIMIT)
        .map_err(|error| Error::from(error.code()))?;
    if file.bytes.is_empty() || file.bytes.len() as u64 > DB2_LIMIT {
        return Err("product_navigation_liquid_type".into());
    }
    let source = terrain_sources::read_liquid_types_70450(&file.bytes)
        .map_err(|error| Error::from(error.code()))?;
    if source.direct_records().len() > MAP_ROWS_MAX || source.copies().len() > MAP_ROWS_MAX {
        return Err("product_navigation_liquid_type".into());
    }
    let table = models::liquid::Table::from_source(&source, models::liquid::Limits::default())
        .map_err(|error| Error::from(error.code()))?;
    let snapshot = LiquidSnapshot {
        file_data_id,
        bytes: file.bytes.len() as u64,
        direct_rows: source.direct_records().len(),
        copy_pairs: source.copies().len(),
        sha256: integrity::digest(&file.bytes),
    };
    Ok((snapshot, table))
}

fn closure_maps(
    owner: &navigation_input::available::AvailableSelectedPackage,
    records: &AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    vmap_wdt_ids: &BTreeMap<u32, u32>,
    global_parent_ids: &BTreeSet<u32>,
    selected_map: u32,
) -> Result<Vec<ClosureMap>, Error> {
    let maps = owner.plan().maps();
    if maps.is_empty() || maps.len() > 4096 {
        return Err("product_navigation_closure".into());
    }
    let mut fields = BTreeMap::new();
    for row in records.direct_records() {
        fields.insert(row.id, row.fields);
    }
    if fields.len() > MAP_ROWS_MAX {
        return Err("product_navigation_closure".into());
    }
    let mut closure = Vec::new();
    closure
        .try_reserve_exact(maps.len())
        .map_err(|_| Error::from("product_allocation"))?;
    let mut output_maps = 0usize;
    for map in maps {
        let source = fields
            .get(&map.map_id())
            .ok_or("product_navigation_closure")?;
        let raw_parent = (source.parent_map_id >= 0).then_some(source.parent_map_id as u32);
        let cosmetic_parent =
            (source.cosmetic_parent_map_id >= 0).then_some(source.cosmetic_parent_map_id as u32);
        let wdt_file_data_id = *vmap_wdt_ids
            .get(&map.map_id())
            .ok_or("product_navigation_closure")?;
        let output = map.is_output();
        output_maps += usize::from(output);
        closure.push(ClosureMap {
            id: map.map_id(),
            root: map.root_map_id(),
            parent: raw_parent,
            cosmetic_parent,
            wdt_file_data_id,
            output,
            is_parent: global_parent_ids.contains(&map.map_id()),
            tiles: map.tile_count(),
        });
    }
    closure.sort_unstable_by_key(|entry| entry.id);
    if output_maps != 1
        || !closure
            .iter()
            .any(|entry| entry.id == selected_map && entry.output)
        || closure.iter().any(|entry| entry.root != closure[0].root)
        || closure
            .windows(2)
            .any(|entries| entries[0].id == entries[1].id)
    {
        return Err("product_navigation_closure".into());
    }
    Ok(closure)
}

struct InputCoverage<'a> {
    selected_map: u32,
    map: &'a MapSnapshot,
    records: &'a AvailableRecords<forever_db2::terrain_sources::NavigationMapFields>,
    vmap_wdt_ids: &'a BTreeMap<u32, u32>,
    global_parent_ids: &'a BTreeSet<u32>,
    closure: &'a [ClosureMap],
    terrain_plan: &'a Plan,
    terrain: &'a terrain_v20::Coverage,
    vmap_plan: &'a Plan,
    vmap: &'a collision_geometry_v22::Coverage,
}

fn validate_input_coverage(input: InputCoverage<'_>) -> Result<Vec<TerrainMap>, Error> {
    let InputCoverage {
        selected_map,
        map,
        records,
        vmap_wdt_ids,
        global_parent_ids,
        closure,
        terrain_plan,
        terrain,
        vmap_plan,
        vmap,
    } = input;
    if vmap.requested_map != selected_map
        || vmap_plan.map_domain != Some(selected_map)
        || terrain.context.build != 70450
        || vmap.context.build != 70450
        || terrain.context.locale != vmap.context.locale
        || terrain.context.source_sha256 != vmap.context.source_sha256
        || terrain_plan.build != 70450
        || vmap_plan.build != 70450
    {
        return Err("product_navigation_input_context".into());
    }
    match terrain.selection {
        Selection::Map(id) if id == selected_map && closure.len() == 1 => {}
        Selection::All => {}
        _ => return Err("product_navigation_input_terrain_coverage".into()),
    }
    let mut terrain_maps = Vec::new();
    terrain_maps
        .try_reserve_exact(closure.len())
        .map_err(|_| Error::from("product_allocation"))?;
    for entry in closure {
        let outcome = terrain
            .maps
            .iter()
            .find(|outcome| outcome.map_id == entry.id)
            .ok_or("product_navigation_input_terrain_coverage")?;
        if outcome.wdt != WdtDisposition::Loaded
            || outcome.known_present_tiles.is_none()
            || outcome.converted_tiles != outcome.known_present_tiles.unwrap_or(usize::MAX)
            || outcome.missing_root_adts != 0
        {
            return Err("product_navigation_input_terrain_coverage".into());
        }
        let known_present = outcome
            .known_present_tiles
            .filter(|count| *count <= 4096)
            .ok_or("product_navigation_input_terrain_coverage")?;
        terrain_maps.push(TerrainMap {
            id: entry.id,
            loaded: true,
            known_present,
            converted: outcome.converted_tiles,
            missing_root_adts: outcome.missing_root_adts,
        });
    }
    if terrain_maps
        .windows(2)
        .any(|entries| entries[0].id >= entries[1].id)
    {
        return Err("product_navigation_input_terrain_coverage".into());
    }
    let terrain_section_count = terrain.map.sections.iter().flatten().count();
    if terrain.map.declared_bytes != map.declared_bytes
        || terrain.map.available_bytes != map.available_bytes
        || terrain.map.full_ckey_verified != map.full_ckey_verified
        || terrain.map.encoded_key_authenticated != map.encoded_key_authenticated
        || terrain.map.direct_rows != map.direct_rows
        || terrain.map.copy_records != map.copy_pairs
        || !terrain_materialized_rows_match(map, terrain.map.materialized_rows)
        || terrain_section_count != records.sections().len()
    {
        return Err("product_navigation_input_terrain_coverage".into());
    }
    for (actual, expected) in terrain
        .map
        .sections
        .iter()
        .flatten()
        .zip(records.sections())
    {
        if actual.ordinal != expected.ordinal
            || actual.declared_records != expected.declared_records
            || actual.declared_copies != expected.declared_copies
            || actual.state != expected.state
        {
            return Err("product_navigation_input_terrain_coverage".into());
        }
    }
    let depths = closure_depths(closure).map_err(|error| Error::from(error.code()))?;
    if vmap.maps.len() != closure.len() {
        return Err("product_navigation_input_vmap_coverage".into());
    }
    for (actual, expected) in vmap.maps.iter().zip(closure) {
        if actual.id != expected.id
            || actual.parent != expected.parent.or(expected.cosmetic_parent)
            || actual.wdt_file_data_id != expected.wdt_file_data_id
            || Some(actual.wdt_file_data_id) != vmap_wdt_ids.get(&expected.id).copied()
            || actual.depth
                != *depths
                    .get(&expected.id)
                    .ok_or("product_navigation_closure")?
            || actual.is_parent != expected.is_parent
            || actual.is_parent != global_parent_ids.contains(&expected.id)
        {
            return Err("product_navigation_input_vmap_coverage".into());
        }
    }
    if vmap.map.file_data_id != map.file_data_id
        || vmap.map.declared_bytes != map.declared_bytes
        || vmap.map.available_bytes != map.available_bytes
        || vmap.map.full_ckey_verified != map.full_ckey_verified
        || vmap.map.encoded_key_authenticated != map.encoded_key_authenticated
        || vmap.map.declared_direct_rows as usize != map.declared_direct_rows
        || vmap.map.declared_copy_records as usize != map.declared_copy_pairs
        || vmap.map.direct_rows as usize != map.direct_rows
        || vmap.map.copy_records as usize != map.copy_pairs
        || !vmap_materialized_rows_match(map, vmap.map.materialized_rows as usize)
        || vmap.map.sections.len() != records.sections().len()
    {
        return Err("product_navigation_input_vmap_coverage".into());
    }
    for (actual, expected) in vmap.map.sections.iter().zip(records.sections()) {
        if actual.ordinal != expected.ordinal
            || actual.records != expected.declared_records
            || actual.copies != expected.declared_copies
            || actual.admitted
                != (expected.state == forever_db2::availability::SectionState::Admitted)
        {
            return Err("product_navigation_input_vmap_coverage".into());
        }
    }
    Ok(terrain_maps)
}

fn selected_map(plan: &Plan) -> Result<u32, Error> {
    driver::admit(plan)?;
    if !plan.is_v23() || plan.build != 70450 {
        return Err("product_navigation_profile".into());
    }
    match plan.navigation {
        Selection::Map(id) => Ok(id),
        _ => Err("product_navigation_profile".into()),
    }
}

fn checked_input_plans(
    product: &Plan,
    inputs: &driver::SelectedNavigationInputs<'_>,
) -> Result<(PlanGuard, PlanGuard), Error> {
    let terrain = PlanGuard::open(inputs.terrain_plan)?;
    let vmap = PlanGuard::open(inputs.vmap_plan)?;
    // The validated v20/v22 product plans intentionally publish no DB2
    // outputs. Navigation source Map/LiquidType tables are resolved through
    // the pinned 70450 catalog by their readers, not added to those plans.
    if !terrain.plan.is_v20()
        || !vmap.plan.is_v22()
        || terrain.plan.build != 70450
        || vmap.plan.build != 70450
        || terrain.plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1
        || vmap.plan.schema_catalog != SchemaCatalog::WowDbDefs70450V1
        || terrain.plan.locale != product.locale
        || vmap.plan.locale != product.locale
    {
        return Err("product_navigation_input_profile".into());
    }
    Ok((terrain, vmap))
}

fn open_store(plan: &Plan, client: &Path) -> Result<LocalStore, Error> {
    let options = CascOpenOptions {
        locale: plan.locale,
        index_layouts: [plan.index_layout; 16],
        root_layout: plan.root_layout,
        limits: Default::default(),
    };
    let store =
        LocalStore::open_db2_70450(client, options).map_err(|error| Error::from(error.code()))?;
    if store.build().build != 70450 {
        return Err("product_build_mismatch".into());
    }
    Ok(store)
}

fn navigation_limits() -> navigation_input::Limits {
    navigation_input::Limits::default()
}

fn verify_input_binding(
    product: &Plan,
    identity: &SourceIdentity,
    terrain: &VerifiedTerrain,
    terrain_plan: &PlanGuard,
    vmap: &VerifiedVmap,
    vmap_plan: &PlanGuard,
) -> Result<(), Error> {
    let selected = selected_map(product)?;
    if identity.selected_map != selected
        || identity.terrain != terrain.identity
        || identity.vmap != vmap.identity
        || identity.terrain.context.source_sha256 != identity.current_source_sha256
        || identity.vmap.context.source_sha256 != identity.current_source_sha256
        || identity.terrain.context.locale != product.locale
        || identity.vmap.context.locale != product.locale
        || identity.terrain.plan_sha256 != terrain_plan.sha256
        || identity.vmap.plan_sha256 != vmap_plan.sha256
        || terrain.identity.plan_sha256 != terrain_plan.sha256
        || vmap.identity.plan_sha256 != vmap_plan.sha256
        || terrain.coverage.context != terrain.identity.context
        || vmap.coverage.context != vmap.identity.context
        || vmap.coverage.requested_map != selected
        || vmap.coverage.map.file_data_id != identity.map.file_data_id
        || vmap.coverage.map.declared_bytes != identity.map.declared_bytes
        || vmap.coverage.map.available_bytes != identity.map.available_bytes
        || vmap.coverage.map.full_ckey_verified != identity.map.full_ckey_verified
        || vmap.coverage.map.encoded_key_authenticated != identity.map.encoded_key_authenticated
        || vmap.coverage.map.declared_direct_rows as usize != identity.map.declared_direct_rows
        || vmap.coverage.map.declared_copy_records as usize != identity.map.declared_copy_pairs
        || vmap.coverage.map.direct_rows as usize != identity.map.direct_rows
        || vmap.coverage.map.copy_records as usize != identity.map.copy_pairs
        || !vmap_materialized_rows_match(
            &identity.map,
            vmap.coverage.map.materialized_rows as usize,
        )
        || vmap.coverage.map.sections.len() != identity.map.section_count
        || identity.closure.len() != vmap.coverage.maps.len()
        || terrain.coverage.map.declared_bytes != identity.map.declared_bytes
        || terrain.coverage.map.available_bytes != identity.map.available_bytes
        || terrain.coverage.map.full_ckey_verified != identity.map.full_ckey_verified
        || terrain.coverage.map.encoded_key_authenticated != identity.map.encoded_key_authenticated
        || terrain.coverage.map.direct_rows != identity.map.direct_rows
        || terrain.coverage.map.copy_records != identity.map.copy_pairs
        || !terrain_materialized_rows_match(&identity.map, terrain.coverage.map.materialized_rows)
        || terrain.coverage.map.sections.iter().flatten().count() != identity.map.section_count
    {
        return Err("product_navigation_input_context".into());
    }
    let depths = closure_depths(&identity.closure).map_err(|error| Error::from(error.code()))?;
    for (fact, section) in vmap
        .coverage
        .map
        .sections
        .iter()
        .zip(terrain.coverage.map.sections.iter().flatten())
    {
        if fact.ordinal != section.ordinal
            || fact.records != section.declared_records
            || fact.copies != section.declared_copies
            || fact.admitted != (section.state == forever_db2::availability::SectionState::Admitted)
        {
            return Err("product_navigation_input_map_coverage".into());
        }
    }
    for (fact, map) in vmap.coverage.maps.iter().zip(&identity.closure) {
        if fact.id != map.id
            || fact.parent != map.parent.or(map.cosmetic_parent)
            || fact.wdt_file_data_id != map.wdt_file_data_id
            || fact.depth != *depths.get(&map.id).ok_or("product_navigation_closure")?
            || fact.is_parent != map.is_parent
        {
            return Err("product_navigation_input_vmap_coverage".into());
        }
    }
    for map in &identity.terrain_maps {
        let fact = terrain
            .coverage
            .maps
            .iter()
            .find(|fact| fact.map_id == map.id)
            .ok_or("product_navigation_input_terrain_coverage")?;
        if fact.wdt != WdtDisposition::Loaded
            || fact.known_present_tiles != Some(map.known_present)
            || fact.converted_tiles != map.converted
            || fact.missing_root_adts != map.missing_root_adts
        {
            return Err("product_navigation_input_terrain_coverage".into());
        }
    }
    let liquid_type = vmap
        .coverage
        .liquids
        .iter()
        .find(|fact| fact.file_data_id == identity.liquid.file_data_id)
        .ok_or("product_navigation_liquid_type")?;
    if liquid_type.bytes != identity.liquid.bytes
        || liquid_type.direct_rows as usize != identity.liquid.direct_rows
        || liquid_type.copy_records as usize != identity.liquid.copy_pairs
    {
        return Err("product_navigation_liquid_type".into());
    }
    Ok(())
}

fn locked_input_check(
    root: &Path,
    verified: &VerifiedTerrain,
    callback: impl FnOnce() -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_locked_package(
        LockedPackage {
            root,
            expected_summary: verified.summary,
            expected_artifacts: &verified.artifacts,
            coverage_path: INPUT_COVERAGE_TERRAIN,
            coverage_sha256: verified.identity.coverage_sha256,
            max_coverage: terrain_v20::COVERAGE_BYTES,
            limits: terrain_v20::package_limits(),
        },
        callback,
    )
}

fn locked_vmap_check(
    root: &Path,
    plan: &Plan,
    verified: &VerifiedVmap,
    callback: impl FnOnce() -> Result<(), Error>,
) -> Result<Summary, Error> {
    let mut limits = package::Limits::default();
    let crate::plan::CollisionStorage::LeasedDiskV1(budget) = plan.collision_storage else {
        return Err("product_navigation_input_profile".into());
    };
    limits.total_bytes = budget.output_bytes;
    verify_locked_package(
        LockedPackage {
            root,
            expected_summary: verified.summary,
            expected_artifacts: &verified.artifacts,
            coverage_path: INPUT_COVERAGE_VMAP,
            coverage_sha256: verified.identity.coverage_sha256,
            max_coverage: MAX_COVERAGE_VMAP,
            limits,
        },
        callback,
    )
}

struct LockedPackage<'a> {
    root: &'a Path,
    expected_summary: Summary,
    expected_artifacts: &'a [Artifact],
    coverage_path: &'a str,
    coverage_sha256: [u8; 32],
    max_coverage: usize,
    limits: package::Limits,
}

fn verify_locked_package(
    locked: LockedPackage<'_>,
    callback: impl FnOnce() -> Result<(), Error>,
) -> Result<Summary, Error> {
    let LockedPackage {
        root,
        expected_summary,
        expected_artifacts,
        coverage_path,
        coverage_sha256,
        max_coverage,
        limits,
    } = locked;
    let mut callback = Some(callback);
    let mut failure = None;
    let summary = package::verify_with_preflight(
        root,
        Some(expected_summary.context),
        limits,
        |actual| {
            if actual != expected_artifacts {
                return Err(package::Error::Inventory);
            }
            Ok(())
        },
        |artifact, file| {
            if artifact.path != coverage_path {
                return Ok(());
            }
            let result = (|| {
                if artifact.sha256 != coverage_sha256
                    || artifact.bytes == 0
                    || artifact.bytes > max_coverage as u64
                {
                    return Err("product_navigation_coverage".into());
                }
                let bytes = read_bounded(file, artifact.bytes, max_coverage)?;
                if integrity::digest(&bytes) != coverage_sha256 {
                    return Err("product_input_changed".into());
                }
                callback.take().ok_or("product_navigation_input_context")?()
            })();
            result.map_err(|error| {
                failure = Some(error);
                package::Error::Schema
            })
        },
    )
    .map_err(|error| failure.unwrap_or_else(|| error.into()))?;
    if summary != expected_summary || callback.is_some() {
        return Err("product_input_changed".into());
    }
    Ok(summary)
}

fn read_bounded(file: &mut File, bytes: u64, max: usize) -> Result<Vec<u8>, Error> {
    let length =
        usize::try_from(bytes).map_err(|_| Error::from("product_navigation_input_limit"))?;
    if length > max {
        return Err("product_navigation_input_limit".into());
    }
    let capacity = length
        .checked_add(1)
        .ok_or("product_navigation_input_limit")?;
    let mut data = Vec::new();
    data.try_reserve_exact(capacity)
        .map_err(|_| Error::from("product_allocation"))?;
    if data.capacity() > capacity {
        return Err("product_navigation_input_limit".into());
    }
    data.resize(capacity, 0);
    file.seek(SeekFrom::Start(0))
        .map_err(|_| Error::from("product_input_io"))?;
    file.read_exact(&mut data[..length])
        .map_err(|_| Error::from("product_input_io"))?;
    if file
        .read(&mut data[length..])
        .map_err(|_| Error::from("product_input_io"))?
        != 0
    {
        return Err("product_input_changed".into());
    }
    data.truncate(length);
    Ok(data)
}

struct NavigationSink<'a> {
    run: &'a mut Run,
    selected_map: u32,
    global: Option<TileOutput>,
    tiles: Vec<TileOutput>,
}
impl navigation_input::run::Sink for NavigationSink<'_> {
    type Error = Error;

    fn emit(&mut self, event: navigation_input::run::Event<'_>) -> Result<(), Self::Error> {
        match event {
            navigation_input::run::Event::Global { map_id, bytes } => {
                if self.global.is_some() || !self.tiles.is_empty() || map_id != self.selected_map {
                    return Err("product_navigation_output".into());
                }
                let view =
                    mmap::GlobalView::parse(bytes, self.selected_map, mmap::Limits::default())
                        .map_err(|error| Error::from(error.code()))?;
                if view.sources().len() != 0 {
                    return Err("product_navigation_offmesh".into());
                }
                let path = mmap::global_path(self.selected_map);
                self.run
                    .put(Component::Navigation, &path, bytes)
                    .map_err(Error::from)?;
                self.global = Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: bytes.len() as u64,
                    sha256: integrity::digest(bytes),
                });
            }
            navigation_input::run::Event::Tile {
                map_id,
                tile,
                outcome,
            } => {
                if self.global.is_none() || map_id != self.selected_map || self.tiles.len() >= 4096
                {
                    return Err("product_navigation_output".into());
                }
                match outcome {
                    forever_navigation::pipeline::Outcome::Empty { .. } => {
                        self.tiles.push(TileOutput {
                            tile,
                            written: false,
                            bytes: 0,
                            sha256: [0; 32],
                        });
                    }
                    forever_navigation::pipeline::Outcome::Tile { bytes, .. } => {
                        mmap::TileView::parse(bytes, mmap::Limits::default())
                            .map_err(|error| Error::from(error.code()))?;
                        let path = mmap::tile_path(self.selected_map, tile.row, tile.column)
                            .map_err(|error| Error::from(error.code()))?;
                        self.run
                            .put(Component::Navigation, &path, bytes)
                            .map_err(Error::from)?;
                        self.tiles.push(TileOutput {
                            tile,
                            written: true,
                            bytes: bytes.len() as u64,
                            sha256: integrity::digest(bytes),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

fn run_selected(
    owner: &mut navigation_input::available::AvailableSelectedPackage,
    selected_map: u32,
    table: &models::liquid::Table,
    run: &mut Run,
) -> Result<
    (
        TileOutput,
        Vec<TileOutput>,
        navigation_input::run::AvailableCompleted,
    ),
    Error,
> {
    let mut tiles = Vec::new();
    tiles
        .try_reserve_exact(4096)
        .map_err(|_| Error::from("product_allocation"))?;
    if tiles.capacity() > 4096 {
        return Err("product_navigation_limit".into());
    }
    let mut sink = NavigationSink {
        run,
        selected_map,
        global: None,
        tiles,
    };
    let completed = navigation_input::run::execute_available_selected(
        owner,
        table.types(),
        &[],
        navigation_input::run::Options {
            selection: navigation_input::run::Selection::Map(selected_map),
            ..Default::default()
        },
        navigation_input::run::AvailableLimits::default(),
        &mut sink,
    )
    .map_err(|failure| match failure.cause {
        navigation_input::run::Cause::Run(error) => Error::from(error.code()),
        navigation_input::run::Cause::Sink(error) => error,
    })?;
    let global = sink.global.ok_or("product_navigation_output")?;
    if completed.completion.progress.maps != 1
        || completed.completion.progress.tiles != sink.tiles.len()
        || completed.completion.progress.tiles
            != completed
                .completion
                .progress
                .written_tiles
                .checked_add(completed.completion.progress.empty_tiles)
                .ok_or("product_navigation_output")?
        || completed.completion.progress.written_tiles
            != sink.tiles.iter().filter(|tile| tile.written).count()
        || completed.completion.progress.empty_tiles
            != sink.tiles.iter().filter(|tile| !tile.written).count()
        || completed.completion.progress.tiles > 4096
    {
        return Err("product_navigation_output".into());
    }
    Ok((global, sink.tiles, completed))
}

struct PendingNavigation {
    owner: navigation_input::available::AvailableSelectedPackage,
    run: Run,
    identity: SourceIdentity,
    context: Context,
    global: TileOutput,
    tiles: Vec<TileOutput>,
    completed: navigation_input::run::AvailableCompleted,
}

pub(super) fn extract(
    product: &Plan,
    client: &Path,
    output: &Path,
    producer: [u8; 32],
    resume: bool,
    inputs: &driver::SelectedNavigationInputs<'_>,
) -> Result<Summary, Error> {
    if !product.is_v23() {
        return Err("product_navigation_profile".into());
    }
    let selected = selected_map(product)?;
    let (mut terrain_plan, mut vmap_plan) = checked_input_plans(product, inputs)?;
    let (output, [terrain_root, vmap_root]) = validate_input_output_separation(
        inputs.terrain_package,
        &terrain_plan.path,
        inputs.vmap_package,
        &vmap_plan.path,
        output,
    )?;
    let terrain = prevalidate_terrain(&terrain_root, &terrain_plan)?;
    let vmap = prevalidate_vmap(&vmap_root, &vmap_plan)?;
    let mut store = open_store(product, client)?;
    let current_source_sha256 = store.content_identity();
    if terrain.identity.context.source_sha256 != current_source_sha256
        || vmap.identity.context.source_sha256 != current_source_sha256
        || terrain.identity.context.locale != product.locale
        || vmap.identity.context.locale != product.locale
    {
        return Err("product_navigation_input_context".into());
    }
    let map_inputs = load_map_inputs(&mut store)?;
    let (liquid, liquid_table) = liquid_snapshot(&mut store)?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    terrain_plan.verify()?;
    vmap_plan.verify()?;

    let data = client.join("Data");
    let archives = data.join("data");
    let configs = data.join("config");
    let protected_client = client.to_path_buf();
    let protected_data = data.clone();
    let protected_archives = archives.clone();
    let protected_configs = configs.clone();
    let protected_terrain_root = terrain_root.clone();
    let protected_vmap_root = vmap_root.clone();
    let protected_terrain_plan = terrain_plan.path.clone();
    let protected_vmap_plan = vmap_plan.path.clone();
    let protected = [
        protected_client.as_path(),
        protected_data.as_path(),
        protected_archives.as_path(),
        protected_configs.as_path(),
        protected_terrain_root.as_path(),
        protected_vmap_root.as_path(),
        protected_terrain_plan.as_path(),
        protected_vmap_plan.as_path(),
    ];
    let vmap_lock_plan = vmap_plan.plan.clone();
    let mut runner_calls = 0usize;
    let mut execution: Option<PendingNavigation> = None;
    let terrain_summary = locked_input_check(&terrain_root, &terrain, || {
        let vmap_summary = locked_vmap_check(&vmap_root, &vmap_lock_plan, &vmap, || {
            runner_calls = runner_calls
                .checked_add(1)
                .ok_or("product_navigation_input_context")?;
            if runner_calls != 1 {
                return Err("product_navigation_input_context".into());
            }
            // The roots used by the actual owner are opened only after both
            // sealed-package readers hold their locks and revalidate the exact
            // admitted inventories and coverage bytes.
            let mut owner = navigation_input::available::AvailableSelectedPackage::open_70450(
                &terrain_root.join("maps"),
                &vmap_root.join("vmaps"),
                &map_inputs.navigation,
                selected,
                navigation_limits(),
            )
            .map_err(|error| Error::from(error.code()))?;
            let closure = closure_maps(
                &owner,
                &map_inputs.navigation,
                &map_inputs.vmap_wdt_ids,
                &map_inputs.global_parent_ids,
                selected,
            )?;
            let terrain_maps = validate_input_coverage(InputCoverage {
                selected_map: selected,
                map: &map_inputs.snapshot,
                records: &map_inputs.navigation,
                vmap_wdt_ids: &map_inputs.vmap_wdt_ids,
                global_parent_ids: &map_inputs.global_parent_ids,
                closure: &closure,
                terrain_plan: &terrain_plan.plan,
                terrain: &terrain.coverage,
                vmap_plan: &vmap_plan.plan,
                vmap: &vmap.coverage,
            })?;
            let plan_sha256 = integrity::digest(product.render().as_bytes());
            let identity = SourceIdentity {
                selected_map: selected,
                build: product.build,
                locale: product.locale.name().to_owned(),
                plan_sha256,
                producer_sha256: producer,
                current_source_sha256,
                map: map_inputs.snapshot,
                closure,
                liquid,
                terrain: terrain.identity.clone(),
                terrain_maps,
                vmap: vmap.identity.clone(),
            };
            let context = Context {
                build: product.build,
                locale: product.locale,
                components: Component::Navigation.bit(),
                source_sha256: integrity::digest(
                    identity
                        .render()
                        .map_err(|error| Error::from(error.code()))?
                        .as_bytes(),
                ),
                plan_sha256,
                producer_sha256: producer,
            };
            verify_input_binding(
                product,
                &identity,
                &terrain,
                &terrain_plan,
                &vmap,
                &vmap_plan,
            )?;
            context.validate()?;
            store
                .verify_unchanged()
                .map_err(|error| Error::from(error.code()))?;
            terrain_plan.verify()?;
            vmap_plan.verify()?;
            let mut run = if resume {
                Run::resume(&output, context, &protected, offline_output_limits())?
            } else {
                Run::create(&output, context, &protected, offline_output_limits())?
            };
            let (global, tiles, completed) =
                run_selected(&mut owner, selected, &liquid_table, &mut run)?;
            if completed.completion.progress.maps != 1
                || completed.completion.progress.tiles
                    != owner
                        .plan()
                        .map(selected)
                        .ok_or("product_navigation_closure")?
                        .tile_count()
                || completed.completion.progress.tiles != tiles.len()
            {
                return Err("product_navigation_output".into());
            }
            execution = Some(PendingNavigation {
                owner,
                run,
                identity,
                context,
                global,
                tiles,
                completed,
            });
            Ok(())
        })?;
        if vmap_summary != vmap.summary {
            return Err("product_input_changed".into());
        }
        Ok(())
    })?;
    if runner_calls != 1 || terrain_summary != terrain.summary {
        return Err("product_input_changed".into());
    }
    let mut execution = execution.ok_or("product_navigation_output")?;
    drop(map_inputs);
    drop(liquid_table);

    // LocalStore's unchanged guard covers the captured source roots and
    // metadata. Re-read the exact consumed DB2 sources as well, so replacing a
    // same-sized archive member cannot be hidden by a stable directory entry.
    let refreshed_map = load_map_inputs(&mut store)?;
    if refreshed_map.snapshot != execution.identity.map {
        return Err("product_input_changed".into());
    }
    let refreshed_closure = closure_maps(
        &execution.owner,
        &refreshed_map.navigation,
        &refreshed_map.vmap_wdt_ids,
        &refreshed_map.global_parent_ids,
        selected,
    )?;
    if refreshed_closure != execution.identity.closure {
        return Err("product_input_changed".into());
    }
    drop(refreshed_map);
    let (refreshed_liquid, refreshed_table) = liquid_snapshot(&mut store)?;
    drop(refreshed_table);
    if refreshed_liquid != execution.identity.liquid
        || store.content_identity() != execution.identity.current_source_sha256
    {
        return Err("product_input_changed".into());
    }
    execution
        .owner
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    drop(execution.owner);
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    terrain_plan.verify()?;
    vmap_plan.verify()?;
    let coverage = Coverage {
        context: execution.context,
        identity: execution.identity,
        global_bytes: execution.global.bytes,
        global_sha256: execution.global.sha256,
        tiles: execution.tiles,
        maps: execution.completed.completion.progress.maps,
        written_tiles: execution.completed.completion.progress.written_tiles,
        empty_tiles: execution.completed.completion.progress.empty_tiles,
        output_bytes: execution.completed.completion.progress.output_bytes,
        builder_work: execution.completed.builder_work,
        save_work: execution.completed.save_work,
        metadata_calls: execution.completed.completion.metadata_calls,
    };
    let coverage_bytes = coverage
        .render()
        .map_err(|error| Error::from(error.code()))?;
    execution
        .run
        .put(Component::Navigation, COVERAGE_PATH, &coverage_bytes)?;
    execution.run.finish_component(Component::Navigation)?;
    execution.run.finish().map_err(Error::from)
}

pub(crate) fn verify_intrinsic(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<Summary, Error> {
    if !plan.is_v23() {
        return Err("product_navigation_profile".into());
    }
    selected_map(plan)?;
    verify_output_package(plan, output, expected).map(|(summary, _)| summary)
}

pub(crate) fn verify_links(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
    inputs: &driver::SelectedNavigationInputs<'_>,
) -> Result<Summary, Error> {
    if !plan.is_v23() {
        return Err("product_navigation_profile".into());
    }
    selected_map(plan)?;
    let (mut terrain_plan, mut vmap_plan) = checked_input_plans(plan, inputs)?;
    let (output, [terrain_root, vmap_root]) = validate_input_output_separation(
        inputs.terrain_package,
        &terrain_plan.path,
        inputs.vmap_package,
        &vmap_plan.path,
        output,
    )?;
    let terrain = prevalidate_terrain(&terrain_root, &terrain_plan)?;
    let vmap = prevalidate_vmap(&vmap_root, &vmap_plan)?;
    if terrain.identity.context.source_sha256 != vmap.identity.context.source_sha256
        || terrain.identity.context.locale != plan.locale
        || vmap.identity.context.locale != plan.locale
    {
        return Err("product_navigation_input_context".into());
    }
    let mut result = None;
    let terrain_summary = locked_input_check(&terrain_root, &terrain, || {
        let vmap_summary = locked_vmap_check(&vmap_root, &vmap_plan.plan, &vmap, || {
            let (summary, coverage) = verify_output_package(plan, &output, expected)?;
            verify_input_binding(
                plan,
                &coverage.identity,
                &terrain,
                &terrain_plan,
                &vmap,
                &vmap_plan,
            )?;
            result = Some(summary);
            Ok(())
        })?;
        if vmap_summary != vmap.summary {
            return Err("product_input_changed".into());
        }
        Ok(())
    })?;
    if terrain_summary != terrain.summary {
        return Err("product_input_changed".into());
    }
    terrain_plan.verify()?;
    vmap_plan.verify()?;
    result.ok_or_else(|| Error::from("product_navigation_input_context"))
}

fn verify_output_package(
    plan: &Plan,
    output: &Path,
    expected: Option<Context>,
) -> Result<(Summary, Coverage), Error> {
    let result = package_v23::verify(
        output,
        package_v23::Expected {
            selected_map: selected_map(plan)?,
            locale: plan.locale,
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            context: expected,
        },
    )
    .map_err(|error| Error::from(error.code()))?;
    Ok(result.into_parts())
}
