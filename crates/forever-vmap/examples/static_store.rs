// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic static-store lifecycle/query QA.
//! Linux: static_store <vmaps> <RCSTORE1-plan> <new-private-stage>.
//! The complete plan/topology is validated before loading. Every operation,
//! source verification and full plan reread succeeds before creating output.
//! Query budgets are shared across the entire plan; loads retain the Store's
//! bounded per-load budgets. Source metadata has its own bounded ledger.
//! The final source verification is metadata-only, not a full post-run rehash
//! or an atomic filesystem snapshot. No source I/O happens per ray.
//! Authored input/output guards are reused from collision_query.rs. Flush and
//! complete readback are checked without a disk crash-durability claim. This
//! private QA stage is not a product publication API; I/O errors can leave it.
#[cfg(target_os = "linux")]
mod qa {
    use forever_vmap::{
        query::{self, Budget},
        store::{
            self, INVALID_HEIGHT, MapParent, Options, SourceError, StaticStore, WORLD_MIDPOINT,
            fs::{FsLimits, RootedSource},
        },
    };
    use std::{
        fs::{self, DirBuilder, File, Metadata, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        os::{
            fd::AsRawFd,
            unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        },
        path::{Component, Path, PathBuf},
    };
    const O_NONBLOCK: i32 = 0x800;
    const O_DIRECTORY: i32 = 0x10000;
    const O_NOFOLLOW: i32 = 0x20000;
    const MAX_MAPS: usize = 64;
    const MAX_OPERATIONS: usize = 4096;
    const MAX_INPUT: usize = 24 + MAX_MAPS * 12 + MAX_OPERATIONS * 64;
    const MAX_OUTPUT: usize = 16 + MAX_OPERATIONS * 48;
    fn identity(a: &Metadata, b: &Metadata) -> bool {
        a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.mode() == b.mode()
            && a.uid() == b.uid()
            && a.gid() == b.gid()
    }
    fn same(a: &Metadata, b: &Metadata) -> bool {
        identity(a, b)
            && a.len() == b.len()
            && a.nlink() == b.nlink()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
    }
    fn read(path: &Path) -> Result<(Vec<u8>, Metadata), &'static str> {
        let named = fs::symlink_metadata(path).map_err(|_| "qa_input_metadata")?;
        if !named.is_file() || named.len() > MAX_INPUT as u64 {
            return Err("qa_input_limit");
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_input_open")?;
        let before = file.metadata().map_err(|_| "qa_input_metadata")?;
        if !same(&before, &named) {
            return Err("qa_input_changed");
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(before.len() as usize)
            .map_err(|_| "qa_allocation")?;
        if bytes.capacity() > MAX_INPUT {
            return Err("qa_input_limit");
        }
        bytes.resize(before.len() as usize, 0);
        file.read_exact(&mut bytes).map_err(|_| "qa_input_read")?;
        let mut tail = [0];
        if file.read(&mut tail).map_err(|_| "qa_input_read")? != 0 {
            return Err("qa_input_changed");
        }
        if !same(&before, &file.metadata().map_err(|_| "qa_input_changed")?)
            || !same(
                &before,
                &fs::symlink_metadata(path).map_err(|_| "qa_input_changed")?,
            )
        {
            return Err("qa_input_changed");
        }
        Ok((bytes, before))
    }
    fn word(bytes: &[u8], at: usize) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            bytes
                .get(at..at + 4)
                .ok_or("qa_input_framing")?
                .try_into()
                .map_err(|_| "qa_input_framing")?,
        ))
    }
    fn directory(path: &Path) -> Result<File, &'static str> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_output_directory")
    }
    fn anchor(file: &File) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
    }
    fn no_git(path: &Path) -> Result<(), &'static str> {
        for p in path.ancestors() {
            if p.join(".git").exists()
                || (p.join("HEAD").exists()
                    && p.join("objects").exists()
                    && p.join("refs").exists())
            {
                return Err("qa_output_in_git");
            }
        }
        Ok(())
    }
    fn write_stage(
        destination: &Path,
        bytes: &[u8],
        input_root: &Path,
    ) -> Result<(), &'static str> {
        if !matches!(
            destination.components().next_back(),
            Some(Component::Normal(_))
        ) {
            return Err("qa_output_boundary");
        }
        let name = destination.file_name().ok_or("qa_output_boundary")?;
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = fs::canonicalize(parent).map_err(|_| "qa_output_boundary")?;
        no_git(&parent)?;
        if parent.starts_with(input_root) {
            return Err("qa_output_in_source");
        }
        if bytes.len() > MAX_OUTPUT {
            return Err("qa_output_limit");
        }
        let parent_file = directory(&parent)?;
        let parent_meta = parent_file.metadata().map_err(|_| "qa_output_boundary")?;
        let stage_path = anchor(&parent_file).join(name);
        DirBuilder::new()
            .mode(0o700)
            .create(&stage_path)
            .map_err(|_| "qa_output_exists")?;
        let stage = directory(&stage_path)?;
        let stage_meta = stage.metadata().map_err(|_| "qa_output_metadata")?;
        if stage_meta.mode() & 0o777 != 0o700 {
            return Err("qa_output_mode");
        }
        let output = anchor(&stage).join("results.bin");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(&output)
            .map_err(|_| "qa_output_create")?;
        file.write_all(bytes)
            .and_then(|()| file.flush())
            .map_err(|_| "qa_output_write")?;
        let before = file.metadata().map_err(|_| "qa_output_metadata")?;
        if !before.is_file()
            || before.nlink() != 1
            || before.mode() & 0o777 != 0o600
            || before.len() != bytes.len() as u64
        {
            return Err("qa_output_metadata");
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|_| "qa_output_readback")?;
        let mut buffer = [0; 4096];
        for chunk in bytes.chunks(buffer.len()) {
            let got = &mut buffer[..chunk.len()];
            file.read_exact(got).map_err(|_| "qa_output_readback")?;
            if got != chunk {
                return Err("qa_output_readback");
            }
        }
        let mut tail = [0];
        if file.read(&mut tail).map_err(|_| "qa_output_readback")? != 0 {
            return Err("qa_output_readback");
        }
        if !same(&before, &file.metadata().map_err(|_| "qa_output_changed")?)
            || !same(
                &before,
                &fs::symlink_metadata(&output).map_err(|_| "qa_output_changed")?,
            )
            || !identity(
                &stage_meta,
                &fs::symlink_metadata(&stage_path).map_err(|_| "qa_output_changed")?,
            )
            || !identity(
                &stage_meta,
                &fs::symlink_metadata(parent.join(name)).map_err(|_| "qa_output_changed")?,
            )
            || !identity(
                &parent_meta,
                &fs::symlink_metadata(&parent).map_err(|_| "qa_output_changed")?,
            )
            || fs::canonicalize(anchor(&parent_file)).map_err(|_| "qa_output_changed")? != parent
        {
            return Err("qa_output_changed");
        }
        Ok(())
    }
    fn operation(bytes: &[u8]) -> Result<[u32; 16], &'static str> {
        let mut words = [0; 16];
        for (i, value) in words.iter_mut().enumerate() {
            *value = word(bytes, i * 4)?;
        }
        let w = words;
        if w[0] > 5 || w[12..].iter().any(|&v| v != 0) {
            return Err("qa_operation");
        }
        if w[0] <= 2 {
            if w[2] > 32767 || w[3] > 255 || (w[0] == 2 && (w[2] != 0 || w[3] != 0)) {
                return Err("qa_tile");
            }
            if w[4..12].iter().any(|&v| v != 0) {
                return Err("qa_reserved");
            }
        } else {
            if w[2] != 0 || w[3] != 0 || w[11] > 1 {
                return Err("qa_query_options");
            }
            if w[4..10].iter().any(|&v| !f32::from_bits(v).is_finite()) {
                return Err("qa_nonfinite");
            }
            let distance = f32::from_bits(w[10]);
            match w[0] {
                3 if w[10] != 0 => return Err("qa_reserved"),
                4 => {
                    if w[7..10].iter().any(|&v| v != 0)
                        || w[11] != 0
                        || !distance.is_finite()
                        || distance < 0.0
                    {
                        return Err("qa_height");
                    }
                }
                5 => {
                    if w[11] != 0 || !distance.is_finite() {
                        return Err("qa_hit");
                    }
                    let dx = (WORLD_MIDPOINT - f32::from_bits(w[7]))
                        - (WORLD_MIDPOINT - f32::from_bits(w[4]));
                    let dy = (WORLD_MIDPOINT - f32::from_bits(w[8]))
                        - (WORLD_MIDPOINT - f32::from_bits(w[5]));
                    let dz = f32::from_bits(w[9]) - f32::from_bits(w[6]);
                    let length = ((dx * dx + dy * dy) + dz * dz).sqrt();
                    if !length.is_finite() || length >= f32::MAX {
                        return Err("qa_hit_length");
                    }
                }
                _ => {}
            }
        }
        Ok(words)
    }
    fn find(parents: &[MapParent], id: u32) -> Result<&MapParent, &'static str> {
        parents
            .binary_search_by_key(&id, |row| row.map_id)
            .map(|i| &parents[i])
            .map_err(|_| "qa_unknown_map")
    }
    fn topology(parents: &[MapParent]) -> Result<(), &'static str> {
        for (i, row) in parents.iter().enumerate() {
            if (i != 0 && parents[i - 1].map_id >= row.map_id)
                || matches!((row.parent, row.cosmetic_parent), (Some(a), Some(b)) if a != b)
            {
                return Err("qa_topology");
            }
        }
        for row in parents {
            let mut parent = row.parent.or(row.cosmetic_parent);
            let mut depth = 0;
            while let Some(id) = parent {
                depth += 1;
                if id == row.map_id || depth > parents.len() {
                    return Err("qa_cycle");
                }
                let next = find(parents, id)?;
                parent = next.parent.or(next.cosmetic_parent);
            }
        }
        Ok(())
    }
    fn source_error(error: SourceError) -> &'static str {
        match error {
            SourceError::Io => "qa_source_io",
            SourceError::InputChanged => "qa_source_changed",
            SourceError::Limit => "qa_source_limit",
        }
    }
    fn push(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }
    fn count(value: usize) -> Result<u32, &'static str> {
        u32::try_from(value).map_err(|_| "qa_count_limit")
    }
    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().take(5).collect();
        if args.len() != 4 {
            return Err("qa_usage");
        }
        let root_path = Path::new(&args[1]);
        let plan_path = Path::new(&args[2]);
        let (bytes, input_identity) = read(plan_path)?;
        if bytes.get(..8) != Some(b"RCSTORE1") || word(&bytes, 20)? != 0 {
            return Err("qa_input_header");
        }
        let parent_count = word(&bytes, 8)? as usize;
        let operation_count = word(&bytes, 12)? as usize;
        let options = word(&bytes, 16)?;
        if parent_count == 0
            || parent_count > MAX_MAPS
            || operation_count > MAX_OPERATIONS
            || options > 7
        {
            return Err("qa_plan_limits");
        }
        let operations_at = 24 + parent_count * 12;
        if bytes.len() != operations_at + operation_count * 64 {
            return Err("qa_input_framing");
        }
        let mut parents = [MapParent {
            map_id: 0,
            parent: None,
            cosmetic_parent: None,
        }; MAX_MAPS];
        for (i, row) in parents[..parent_count].iter_mut().enumerate() {
            let at = 24 + i * 12;
            let parent = word(&bytes, at + 4)?;
            let cosmetic = word(&bytes, at + 8)?;
            *row = MapParent {
                map_id: word(&bytes, at)?,
                parent: (parent != u32::MAX).then_some(parent),
                cosmetic_parent: (cosmetic != u32::MAX).then_some(cosmetic),
            };
        }
        let parents = &parents[..parent_count];
        topology(parents)?;
        let operations = &bytes[operations_at..];
        for record in operations.as_chunks::<64>().0 {
            let op = operation(record)?;
            find(parents, op[1])?;
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(16 + operation_count * 48)
            .map_err(|_| "qa_allocation")?;
        if result.capacity() > MAX_OUTPUT {
            return Err("qa_output_limit");
        }
        result.extend_from_slice(b"RCSOUT01");
        push(&mut result, operation_count as u32);
        push(&mut result, 0);
        let input_root = fs::canonicalize(root_path).map_err(|_| "qa_source_directory")?;
        let source = RootedSource::new(root_path, FsLimits::default()).map_err(source_error)?;
        let mut store = StaticStore::new(
            source,
            parents,
            Options {
                load_path_only_models: options & 1 != 0,
                enable_line_of_sight: options & 2 != 0,
                enable_height: options & 4 != 0,
            },
            store::Limits::default(),
        )
        .map_err(|e| e.code())?;
        let mut budget = Budget::new(query::Limits::default()).map_err(|e| e.code())?;
        let (mut loads, mut unloads, mut queries) = (0, 0, 0);
        for record in operations.as_chunks::<64>().0 {
            let w = operation(record)?;
            let mut status = 0;
            let mut payload = [0; 4];
            let start = [w[4], w[5], w[6]].map(f32::from_bits);
            let end = [w[7], w[8], w[9]].map(f32::from_bits);
            match w[0] {
                0 => {
                    match store.load_tile(w[1], w[2] as u16, w[3] as u8) {
                        Ok(_) => {}
                        Err(store::Error::Disabled) => status = 4,
                        Err(error) => return Err(error.code()),
                    }
                    loads += 1;
                }
                1 => {
                    store
                        .unload_tile(w[1], w[2] as u16, w[3] as u8)
                        .map_err(|e| e.code())?;
                    unloads += 1;
                }
                2 => {
                    store.unload_map(w[1]).map_err(|e| e.code())?;
                    unloads += 1;
                }
                3 => {
                    payload[0] = u32::from(
                        store
                            .line_of_sight_world(w[1], start, end, w[11] != 0, &mut budget)
                            .map_err(|e| e.code())?,
                    );
                    queries += 1;
                }
                4 => {
                    payload[0] = store
                        .height_world(w[1], start, f32::from_bits(w[10]), &mut budget)
                        .map_err(|e| e.code())?
                        .unwrap_or(INVALID_HEIGHT)
                        .to_bits();
                    queries += 1;
                }
                5 => {
                    let hit = store
                        .object_hit_world(w[1], start, end, f32::from_bits(w[10]), &mut budget)
                        .map_err(|e| e.code())?;
                    payload[0] = u32::from(hit.hit());
                    payload[1..].copy_from_slice(&hit.position_bits());
                    queries += 1;
                }
                _ => return Err("qa_operation"),
            }
            let map = store.map_stats(w[1]);
            push(&mut result, w[0]);
            push(&mut result, status);
            push(&mut result, count(map.map_or(0, |m| m.primitive_slots))?);
            push(&mut result, count(map.map_or(0, |m| m.active_instances))?);
            push(&mut result, count(store.stats().models)?);
            push(&mut result, 0);
            for value in payload {
                push(&mut result, value);
            }
            push(&mut result, 0);
            push(&mut result, 0);
        }
        store.verify_source().map_err(|e| e.code())?;
        let (reread, current_identity) = read(plan_path)?;
        if !same(&input_identity, &current_identity) || bytes != reread {
            return Err("qa_input_changed");
        }
        drop(reread);
        store.verify_source().map_err(|e| e.code())?;
        write_stage(Path::new(&args[3]), &result, &input_root)?;
        let stats = budget.stats();
        println!(
            "{{\"status\":\"success\",\"operations\":{operation_count},\"loads\":{loads},\"unloads\":{unloads},\"queries\":{queries},\"node_visits\":{},\"triangle_tests\":{}}}",
            stats.node_visits, stats.triangle_tests
        );
        Ok(())
    }
}
#[cfg(target_os = "linux")]
fn main() {
    if let Err(code) = qa::run() {
        println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
        std::process::exit(if code == "qa_usage" { 2 } else { 4 });
    }
}
#[cfg(not(target_os = "linux"))]
fn main() {
    println!("{{\"status\":\"error\",\"code\":\"qa_platform_unsupported\"}}");
    std::process::exit(4);
}
