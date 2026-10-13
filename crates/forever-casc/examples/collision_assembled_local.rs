// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete offline collision assembly QA, synthetic CASC.
//! Linux QA: collision_assembled_local <client> <new-stage> --synthetic-casc
//! Complete toy CASC with explicit enUS/contiguous/header-50893-v2 and names.
//! The library owns the placement stage and enforces its own combined memory
//! ceilings. This caller uses fixed name/readback buffers and the library's
//! borrowed artifact iterator; it never clones model or artifact bytes. Its
//! separate directory-identity ledger is capped at4096 entries and1MiB.
//! Preflight every artifact before creating output. Only complete final files
//! are written under stage/vmaps; no RAW/intermediate inventory is published.
//! Private0700/0600 paths, exclusive creation, anchors and readback checks do
//! not constitute a product publication transaction or crash durability. An
//! output I/O failure can leave private partial staging. No runtime/live claim.

#[cfg(target_os = "linux")]
mod qa {
    use forever_casc::{
        Locale,
        collision::{assemble, gameobjects},
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
    const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;
    const MAX_OUTPUT_FILES: usize = 65_536;
    const MAX_DIRECTORIES: usize = 4096;
    const MAX_DIRECTORY_BYTES: usize = 1024 * 1024;

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

    struct NameBuffer {
        bytes: [u8; 1024],
        len: usize,
    }
    impl std::fmt::Write for NameBuffer {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            let end = self
                .len
                .checked_add(text.len())
                .filter(|&n| n <= self.bytes.len())
                .ok_or(std::fmt::Error)?;
            self.bytes[self.len..end].copy_from_slice(text.as_bytes());
            self.len = end;
            Ok(())
        }
    }
    impl NameBuffer {
        fn render(value: impl std::fmt::Display) -> Result<Self, &'static str> {
            let mut result = Self {
                bytes: [0; 1024],
                len: 0,
            };
            std::fmt::write(&mut result, format_args!("{value}")).map_err(|_| "qa_output_name")?;
            Ok(result)
        }
        fn as_str(&self) -> Result<&str, &'static str> {
            std::str::from_utf8(&self.bytes[..self.len]).map_err(|_| "qa_output_name")
        }
    }
    fn component(value: &str) -> Result<(), &'static str> {
        if value.is_empty()
            || value.len() > 255
            || value == "."
            || value == ".."
            || value
                .bytes()
                .any(|b| !(32..=126).contains(&b) || b"/\\:".contains(&b))
        {
            return Err("qa_output_name");
        }
        Ok(())
    }
    fn relative(value: &str) -> Result<(Option<&str>, &str), &'static str> {
        let mut parts = value.split('/');
        let first = parts.next().ok_or("qa_output_name")?;
        component(first)?;
        if let Some(second) = parts.next() {
            component(second)?;
            if parts.next().is_some()
                || first.len() < 4
                || first.len() > 10
                || !first.bytes().all(|b| b.is_ascii_digit())
            {
                return Err("qa_output_name");
            }
            Ok((Some(first), second))
        } else {
            Ok((None, first))
        }
    }
    #[derive(Clone, Copy)]
    struct Identity {
        dev: u64,
        ino: u64,
        uid: u32,
        gid: u32,
        mode: u32,
    }
    impl Identity {
        fn of(value: &Metadata) -> Self {
            Self {
                dev: value.dev(),
                ino: value.ino(),
                uid: value.uid(),
                gid: value.gid(),
                mode: value.mode(),
            }
        }
        fn matches(self, value: &Metadata) -> bool {
            self.dev == value.dev()
                && self.ino == value.ino()
                && self.uid == value.uid()
                && self.gid == value.gid()
                && self.mode == value.mode()
        }
    }
    struct DirStamp {
        name: [u8; 10],
        len: usize,
        identity: Option<Identity>,
    }
    impl DirStamp {
        fn name(&self) -> &str {
            std::str::from_utf8(&self.name[..self.len]).expect("validated ASCII directory")
        }
    }
    struct Directories {
        rows: Vec<DirStamp>,
    }
    impl Directories {
        fn new() -> Result<Self, &'static str> {
            let requested = MAX_DIRECTORIES
                .checked_mul(std::mem::size_of::<DirStamp>())
                .ok_or("qa_directory_limit")?;
            if requested > MAX_DIRECTORY_BYTES {
                return Err("qa_directory_limit");
            }
            let mut rows = Vec::new();
            rows.try_reserve_exact(MAX_DIRECTORIES)
                .map_err(|_| "qa_directory_allocation")?;
            if rows
                .capacity()
                .checked_mul(std::mem::size_of::<DirStamp>())
                .is_none_or(|n| n > MAX_DIRECTORY_BYTES)
            {
                return Err("qa_directory_limit");
            }
            Ok(Self { rows })
        }
        fn include(&mut self, name: &str) -> Result<(), &'static str> {
            match self.rows.binary_search_by(|row| row.name().cmp(name)) {
                Ok(_) => Ok(()),
                Err(at) => {
                    if self.rows.len() >= MAX_DIRECTORIES {
                        return Err("qa_directory_limit");
                    }
                    let mut bytes = [0; 10];
                    bytes[..name.len()].copy_from_slice(name.as_bytes());
                    self.rows.insert(
                        at,
                        DirStamp {
                            name: bytes,
                            len: name.len(),
                            identity: None,
                        },
                    );
                    Ok(())
                }
            }
        }
        fn create(&mut self, parent: &File) -> Result<(), &'static str> {
            for row in &mut self.rows {
                let file = subdirectory(parent, row.name())?;
                row.identity = Some(Identity::of(
                    &file.metadata().map_err(|_| "qa_output_changed")?,
                ));
            }
            Ok(())
        }
        fn open(&self, parent: &File, name: &str) -> Result<File, &'static str> {
            let at = self
                .rows
                .binary_search_by(|row| row.name().cmp(name))
                .map_err(|_| "qa_output_state")?;
            let row = &self.rows[at];
            let file = directory(&anchored(parent).join(row.name()))?;
            verify_directory(parent, row.name(), &file)?;
            if !row
                .identity
                .ok_or("qa_output_state")?
                .matches(&file.metadata().map_err(|_| "qa_output_changed")?)
            {
                return Err("qa_output_changed");
            }
            Ok(file)
        }
        fn verify(&self, parent: &File) -> Result<(), &'static str> {
            for row in &self.rows {
                self.open(parent, row.name())?;
            }
            Ok(())
        }
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
        store
            .enable_names(root::name::Limits::default())
            .map_err(|error| error.code())?;
        let startup = gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default())
            .map_err(|error| error.code())?;
        let completed = assemble::extract(startup, assemble::Limits::default())
            .map_err(|error| error.code())?;
        let mut directories = Directories::new()?;
        let mut count = 0usize;
        let mut bytes = 0usize;
        for artifact in completed.artifacts() {
            let name = NameBuffer::render(artifact.name())?;
            let (directory, _) = relative(name.as_str()?)?;
            if let Some(directory) = directory {
                directories.include(directory)?;
            }
            count = count
                .checked_add(1)
                .filter(|&n| n <= MAX_OUTPUT_FILES)
                .ok_or("qa_output_limit")?;
            bytes = bytes
                .checked_add(artifact.bytes().len())
                .filter(|&n| n <= MAX_OUTPUT_BYTES)
                .ok_or("qa_output_limit")?;
        }
        // Complete library success and all writer-policy checks precede output.
        let stage = destination.create()?;
        let vmaps = subdirectory(&stage, "vmaps")?;
        directories.create(&vmaps)?;
        let mut written = 0usize;
        let mut written_bytes = 0usize;
        for artifact in completed.artifacts() {
            let name = NameBuffer::render(artifact.name())?;
            let (directory, basename) = relative(name.as_str()?)?;
            if let Some(directory) = directory {
                let anchor = directories.open(&vmaps, directory)?;
                write_file(&anchor, basename, artifact.bytes())?;
                verify_directory(&vmaps, directory, &anchor)?;
            } else {
                write_file(&vmaps, basename, artifact.bytes())?;
            }
            written = written.checked_add(1).ok_or("qa_output_state")?;
            written_bytes = written_bytes
                .checked_add(artifact.bytes().len())
                .ok_or("qa_output_state")?;
        }
        if written != count || written_bytes != bytes {
            return Err("qa_output_state");
        }
        directories.verify(&vmaps)?;
        verify_directory(&stage, "vmaps", &vmaps)?;
        destination.verify_created(&stage)?;
        let world = completed.world();
        let facts = world.startup().extraction().facts();
        Ok(format!(
            "{{\"status\":\"success\",\"assembled_files\":{count},\"assembled_bytes\":{bytes},\"map_rows\":{},\"active_maps\":{},\"models\":{},\"reads\":{},\"source_bytes\":{},\"world_resident_bytes\":{},\"store_resident_bytes\":{}}}",
            world.topology().all().len(),
            world.map_stats().active_maps,
            facts.models,
            facts.reads,
            facts.source_bytes,
            world.resident_bytes(),
            world.store_resident_bytes()
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
