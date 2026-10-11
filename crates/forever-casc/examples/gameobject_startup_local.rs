// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded offline startup QA with complete synthetic CASC.
//! Linux QA: gameobject_startup_local <client> <new-stage> --synthetic-casc
//! Fixed enUS/contiguous/header-50893-v2 profile; no native or name fallback.
//! No stage exists before public startup+finish succeeds. Stage files are private
//! QA artifacts, not a publication transaction. I/O failure can leave partial
//! staging, never successful output. Full readback and identity checks are made;
//! closing/flushing does not promise crash durability. No later liquid phase,
//! original producer, runtime or real-client acceptance is claimed here.

#[cfg(target_os = "linux")]
mod qa {
    use forever_casc::{
        Locale,
        collision::{self, gameobjects},
        local_index, local_store, root,
    };
    use std::{
        fs::{self, DirBuilder, File, Metadata, OpenOptions},
        io::{self, Read, Seek, SeekFrom, Write},
        os::{
            fd::AsRawFd,
            unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        },
        path::{Component, Path, PathBuf},
    };

    const O_NONBLOCK: i32 = 0x800;
    const O_DIRECTORY: i32 = 0x10000;
    const O_NOFOLLOW: i32 = 0x20000;
    const MAX_OUTPUT_BYTES: usize = 384 * 1024 * 1024;
    const MAX_OUTPUT_FILES: usize = 4098;

    fn exists(path: &Path) -> Result<bool, &'static str> {
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(_) => Err("qa_output_boundary"),
        }
    }
    fn no_git_ancestor(path: &Path) -> Result<(), &'static str> {
        for ancestor in path.ancestors() {
            if exists(&ancestor.join(".git"))?
                || (exists(&ancestor.join("HEAD"))?
                    && exists(&ancestor.join("objects"))?
                    && exists(&ancestor.join("refs"))?)
            {
                return Err("qa_output_in_git");
            }
        }
        Ok(())
    }
    fn same_directory(a: &Metadata, b: &Metadata) -> bool {
        a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.mode() == b.mode()
            && a.uid() == b.uid()
            && a.gid() == b.gid()
    }
    fn same_file(a: &Metadata, b: &Metadata) -> bool {
        same_directory(a, b)
            && a.nlink() == b.nlink()
            && a.len() == b.len()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
    }
    fn directory(path: &Path) -> Result<File, &'static str> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_output_directory")
    }
    fn anchored(file: &File) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()))
    }
    struct Destination {
        client: PathBuf,
        data: PathBuf,
        parent: PathBuf,
        path: PathBuf,
        parent_file: File,
        parent_identity: Metadata,
    }
    impl Destination {
        fn prepare(client: &Path, destination: &Path) -> Result<Self, &'static str> {
            let client = fs::canonicalize(client).map_err(|_| "qa_client_path")?;
            let data = fs::canonicalize(client.join("Data")).map_err(|_| "qa_client_path")?;
            if !matches!(
                destination.components().next_back(),
                Some(Component::Normal(_))
            ) {
                return Err("qa_output_boundary");
            }
            let parent = destination
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .ok_or("qa_output_boundary")?;
            let parent = fs::canonicalize(parent).map_err(|_| "qa_output_boundary")?;
            let path = parent.join(destination.file_name().ok_or("qa_output_boundary")?);
            let parent_file = directory(&parent)?;
            let parent_identity = parent_file.metadata().map_err(|_| "qa_output_boundary")?;
            let result = Self {
                client,
                data,
                parent,
                path,
                parent_file,
                parent_identity,
            };
            result.verify()?;
            if exists(&result.path)? {
                return Err("qa_output_exists");
            }
            Ok(result)
        }
        fn verify(&self) -> Result<(), &'static str> {
            if fs::canonicalize(&self.parent).map_err(|_| "qa_output_changed")? != self.parent
                || fs::canonicalize(anchored(&self.parent_file)).map_err(|_| "qa_output_changed")?
                    != self.parent
                || fs::canonicalize(&self.client).map_err(|_| "qa_client_changed")? != self.client
                || fs::canonicalize(self.client.join("Data")).map_err(|_| "qa_client_changed")?
                    != self.data
            {
                return Err("qa_output_changed");
            }
            let named = fs::symlink_metadata(&self.parent).map_err(|_| "qa_output_changed")?;
            let handle = self
                .parent_file
                .metadata()
                .map_err(|_| "qa_output_changed")?;
            if !same_directory(&self.parent_identity, &named)
                || !same_directory(&self.parent_identity, &handle)
                || self.parent.starts_with(&self.client)
                || self.client.starts_with(&self.path)
                || self.parent.starts_with(&self.data)
                || self.data.starts_with(&self.path)
            {
                return Err("qa_output_boundary");
            }
            no_git_ancestor(&self.parent)
        }
        fn create(&self) -> Result<File, &'static str> {
            self.verify()?;
            let path = anchored(&self.parent_file)
                .join(self.path.file_name().ok_or("qa_output_boundary")?);
            DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .map_err(|_| "qa_output_create")?;
            let file = directory(&path)?;
            self.verify_created(&file)?;
            Ok(file)
        }
        fn verify_created(&self, file: &File) -> Result<(), &'static str> {
            self.verify()?;
            let named = fs::symlink_metadata(&self.path).map_err(|_| "qa_output_changed")?;
            let handle = file.metadata().map_err(|_| "qa_output_changed")?;
            if !named.is_dir()
                || !same_directory(&named, &handle)
                || handle.mode() & 0o777 != 0o700
                || fs::canonicalize(anchored(file)).map_err(|_| "qa_output_changed")? != self.path
            {
                return Err("qa_output_changed");
            }
            no_git_ancestor(&self.path)
        }
    }
    fn subdirectory(parent: &File, name: &str) -> Result<File, &'static str> {
        let path = anchored(parent).join(name);
        DirBuilder::new()
            .mode(0o700)
            .create(&path)
            .map_err(|_| "qa_output_create")?;
        let file = directory(&path)?;
        verify_directory(parent, name, &file)?;
        Ok(file)
    }
    fn verify_directory(parent: &File, name: &str, file: &File) -> Result<(), &'static str> {
        let named =
            fs::symlink_metadata(anchored(parent).join(name)).map_err(|_| "qa_output_changed")?;
        let handle = file.metadata().map_err(|_| "qa_output_changed")?;
        if !named.is_dir() || !same_directory(&named, &handle) || handle.mode() & 0o777 != 0o700 {
            return Err("qa_output_changed");
        }
        Ok(())
    }
    fn basename(name: &[u8]) -> Result<&str, &'static str> {
        if name.is_empty()
            || name.len() > 255
            || matches!(name, b"." | b".." | b"temp_gameobject_models")
            || name
                .iter()
                .any(|byte| !(32..=126).contains(byte) || b"/\\:".contains(byte))
        {
            return Err("qa_output_name");
        }
        std::str::from_utf8(name).map_err(|_| "qa_output_name")
    }
    fn write_file(parent: &File, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        let path = anchored(parent).join(name);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(&path)
            .map_err(|_| "qa_output_file")?;
        file.write_all(bytes).map_err(|_| "qa_output_write")?;
        file.flush().map_err(|_| "qa_output_flush")?;
        file.seek(SeekFrom::Start(0))
            .map_err(|_| "qa_output_readback")?;
        let before = file.metadata().map_err(|_| "qa_output_metadata")?;
        if !before.is_file()
            || before.nlink() != 1
            || before.mode() & 0o777 != 0o600
            || before.len() != bytes.len() as u64
        {
            return Err("qa_output_metadata");
        }
        let mut buffer = [0; 64 * 1024];
        for chunk in bytes.chunks(buffer.len()) {
            let read = &mut buffer[..chunk.len()];
            file.read_exact(read).map_err(|_| "qa_output_readback")?;
            if read != chunk {
                return Err("qa_output_readback");
            }
        }
        let mut tail = [0];
        if file.read(&mut tail).map_err(|_| "qa_output_readback")? != 0 {
            return Err("qa_output_readback");
        }
        let after = file.metadata().map_err(|_| "qa_output_changed")?;
        let named = fs::symlink_metadata(&path).map_err(|_| "qa_output_changed")?;
        if !same_file(&before, &after) || !same_file(&after, &named) {
            return Err("qa_output_changed");
        }
        Ok(())
    }

    pub fn run() -> Result<String, &'static str> {
        let args: Vec<_> = std::env::args_os().take(5).collect();
        if args.len() != 4 || args[3] != "--synthetic-casc" {
            return Err("qa_usage");
        }
        let destination = Destination::prepare(Path::new(&args[1]), Path::new(&args[2]))?;
        let mut store = local_store::LocalStore::open(
            &destination.client,
            local_store::OpenOptions {
                locale: Locale::EnUs,
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: local_store::Limits::default(),
            },
        )
        .map_err(|error| error.code())?;
        let completed =
            gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default())
                .map_err(|error| error.code())?
                .finish()
                .map_err(|error| error.code())?;
        let data = completed.data();
        let extraction = completed.extraction();
        let mut output_bytes = data
            .lists()
            .temp_bytes()
            .len()
            .checked_add(data.lists().dtree_bytes().len())
            .ok_or("qa_output_limit")?;
        let mut raw_files = 0usize;
        for model in extraction.models() {
            if let Some(raw) = model.raw() {
                if model.status() != collision::Status::Collision {
                    return Err("qa_output_state");
                }
                basename(model.output_basename())?;
                output_bytes = output_bytes
                    .checked_add(raw.len())
                    .ok_or("qa_output_limit")?;
                raw_files += 1;
            }
        }
        if output_bytes > MAX_OUTPUT_BYTES || raw_files + 2 > MAX_OUTPUT_FILES {
            return Err("qa_output_limit");
        }
        // All source work and every budget check above precede the first write.
        let stage = destination.create()?;
        let raw_dir = subdirectory(&stage, "raw")?;
        let native_dir = subdirectory(&stage, "native")?;
        for model in extraction.models() {
            if let Some(raw) = model.raw() {
                write_file(&raw_dir, basename(model.output_basename())?, raw)?;
            }
        }
        write_file(
            &raw_dir,
            "temp_gameobject_models",
            data.lists().temp_bytes(),
        )?;
        write_file(
            &native_dir,
            "GameObjectModels.dtree",
            data.lists().dtree_bytes(),
        )?;
        verify_directory(&stage, "raw", &raw_dir)?;
        verify_directory(&stage, "native", &native_dir)?;
        destination.verify_created(&stage)?;
        let stats = data.stats();
        let facts = extraction.facts();
        Ok(format!(
            concat!(
                "{{\"status\":\"success\",\"direct_rows\":{},\"copy_records\":{},",
                "\"candidate_requests\":{},\"zero_fdids\":{},\"textures\":{},\"m2_no_collision\":{},",
                "\"wmo_doodad_only\":{},\"collision_rows\":{},\"reads\":{},\"source_bytes\":{},",
                "\"models\":{},\"raw_files\":{},\"raw_bytes\":{},\"temp_bytes\":{},",
                "\"dtree_bytes\":{},\"output_bytes\":{}}}"
            ),
            stats.direct_rows,
            stats.copy_records,
            stats.candidate_requests,
            stats.zero_fdids,
            stats.textures,
            stats.m2_no_collision,
            stats.wmo_doodad_only,
            stats.collision_rows,
            facts.reads,
            facts.source_bytes,
            facts.models,
            raw_files,
            facts.raw_bytes,
            data.lists().temp_bytes().len(),
            data.lists().dtree_bytes().len(),
            output_bytes
        ))
    }
}

fn main() -> std::process::ExitCode {
    use std::io::Write;
    #[cfg(target_os = "linux")]
    let result = qa::run();
    #[cfg(not(target_os = "linux"))]
    let result: Result<String, &'static str> = Err("qa_linux_required");
    let (message, code) = match result {
        Ok(message) => (message, 0),
        Err(code) => (
            format!("{{\"status\":\"error\",\"code\":\"{code}\"}}"),
            if code == "qa_usage" { 2 } else { 4 },
        ),
    };
    if writeln!(std::io::stdout().lock(), "{message}").is_err() {
        return std::process::ExitCode::from(4);
    }
    std::process::ExitCode::from(code)
}
