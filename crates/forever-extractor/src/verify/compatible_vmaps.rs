// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Offline complete declared inventory/association checks for explicit v14.
//! The common package first verifies every digest. A held no-follow reader then
//! replays each genuine ordered witness through the unchanged pure assembler.
//! Available-source coverage is producer-reported, not a fresh client read.
use crate::{
    Error,
    collision_coverage::{self as cov, Coverage, Entry},
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
#[cfg(test)]
mod tests;
