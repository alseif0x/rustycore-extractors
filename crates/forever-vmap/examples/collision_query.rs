// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic model/instance ray-query QA adapter.
//! Linux: collision_query <RCQUERY1-inventory> <new-private-stage>.
//! Exact complete VMO, optional one bounded dir_bin spawn and full ray plan.
//! No acquisition/network. All validation and queries finish before creating
//! output; I/O failures may leave private staging, never an existing overwrite.
//! Flush and complete readback are checked, with no crash-durability claim.
#[cfg(target_os = "linux")]
mod qa {
    use forever_vmap::{
        placement,
        query::{self, Budget, Mode, Ray, model},
        vmo,
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
    const MAX_QUERIES: usize = 65_536;
    const MAX_INPUT: usize = 24 + vmo::MAX_BYTES + 1024 + 36 * MAX_QUERIES;
    const MAX_OUTPUT: usize = 16 + 8 * MAX_QUERIES;
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
    fn vector(bytes: &[u8], at: usize) -> Result<[u32; 3], &'static str> {
        Ok([word(bytes, at)?, word(bytes, at + 4)?, word(bytes, at + 8)?])
    }
    fn ray(bytes: &[u8]) -> Result<(Ray, f32, Mode, bool), &'static str> {
        let mode = match word(bytes, 0)? {
            0 => Mode::Nearest,
            1 => Mode::Any,
            _ => return Err("qa_mode"),
        };
        let ignore = match word(bytes, 4)? {
            0 => false,
            1 => true,
            _ => return Err("qa_ignore"),
        };
        let ray = Ray::from_bits(vector(bytes, 8)?, vector(bytes, 20)?).map_err(|e| e.code())?;
        let distance = f32::from_bits(word(bytes, 32)?);
        if distance.is_nan() || distance < 0.0 {
            return Err("qa_distance");
        }
        Ok((ray, distance, mode, ignore))
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
    fn write_stage(destination: &Path, bytes: &[u8]) -> Result<(), &'static str> {
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
    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().take(4).collect();
        if args.len() != 3 {
            return Err("qa_usage");
        }
        let input = Path::new(&args[1]);
        let (bytes, identity) = read(input)?;
        if bytes.get(..8) != Some(b"RCQUERY1") || word(&bytes, 20)? != 0 {
            return Err("qa_input_header");
        }
        let model_size = word(&bytes, 8)? as usize;
        let spawn_size = word(&bytes, 12)? as usize;
        let count = word(&bytes, 16)? as usize;
        if model_size > vmo::MAX_BYTES || spawn_size > 1024 || count > MAX_QUERIES {
            return Err("qa_input_limit");
        }
        let spawn_at = 24 + model_size;
        let rays_at = spawn_at + spawn_size;
        if rays_at + 36 * count != bytes.len() {
            return Err("qa_input_framing");
        }
        let model =
            vmo::View::parse(&bytes[24..spawn_at], vmo::Limits::default()).map_err(|e| e.code())?;
        let instance = if spawn_size == 0 {
            None
        } else {
            let view =
                placement::DirBin::parse(&bytes[spawn_at..rays_at], placement::Limits::default())
                    .map_err(|e| e.code())?;
            if view.record_count() != 1 {
                return Err("qa_spawn_count");
            }
            Some(
                model::Instance::new(&view.records().next().ok_or("qa_spawn_count")?)
                    .map_err(|e| e.code())?,
            )
        };
        let rays = &bytes[rays_at..];
        for plan in rays.as_chunks::<36>().0 {
            ray(plan)?;
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(16 + 8 * count)
            .map_err(|_| "qa_allocation")?;
        if output.capacity() > MAX_OUTPUT {
            return Err("qa_output_limit");
        }
        output.extend_from_slice(b"RCRSLT01");
        output.extend_from_slice(&(count as u32).to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes());
        let mut budget = Budget::new(query::Limits::default()).map_err(|e| e.code())?;
        let mut hits = 0;
        for plan in rays.as_chunks::<36>().0 {
            let (ray, distance, mode, ignore) = ray(plan)?;
            let result = if let Some(instance) = &instance {
                instance.intersect(&model, &ray, distance, mode, ignore, &mut budget)
            } else {
                model::intersect(&model, &ray, distance, mode, ignore, &mut budget)
            }
            .map_err(|e| e.code())?;
            let hit = u32::from(result.is_some());
            hits += hit as usize;
            output.extend_from_slice(&hit.to_le_bytes());
            output.extend_from_slice(
                &result
                    .map_or(distance.to_bits(), |hit| hit.distance_bits())
                    .to_le_bytes(),
            );
        }
        if !same(
            &identity,
            &fs::symlink_metadata(input).map_err(|_| "qa_input_changed")?,
        ) {
            return Err("qa_input_changed");
        }
        write_stage(Path::new(&args[2]), &output)?;
        let stats = budget.stats();
        println!(
            "{{\"status\":\"success\",\"queries\":{count},\"hits\":{hits},\"misses\":{},\"node_visits\":{},\"triangle_tests\":{}}}",
            count - hits,
            stats.node_visits,
            stats.triangle_tests
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
