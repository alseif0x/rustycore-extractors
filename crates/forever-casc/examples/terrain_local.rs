// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded local terrain QA using the complete public runner.
//! terrain_local <client> <all|maps:ID,ID|tiles:ID,X,Y;ID,X,Y> <new-output>
//!               [--names] [--no-pack] [--locale=enUS|--locale=esES]
//! Offline Linux QA, not a versioned production manifest or full-client claim.
//! Complete source validation/conversion precedes any output creation. The new
//! destination retains failed writes; any I/O error is a failed invocation.
//! complete.json is a QA summary, not an atomic product commit or file verifier.
#[cfg(target_os = "linux")]
mod qa {
    use forever_casc::{
        Locale, local_index, local_store, root,
        terrain::{plan, run, tilelist},
    };
    use forever_terrain::conversion;
    use std::fs::{self, DirBuilder, File, Metadata, OpenOptions};
    use std::io::{self, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    use std::path::{Component, Path, PathBuf};
    const O_NONBLOCK: i32 = 0x800;
    const O_DIRECTORY: i32 = 0x10000;
    const O_NOFOLLOW: i32 = 0x20000;
    const MAX_BYTES: usize = 64 * 1024 * 1024;
    const MAX_FILES: usize = 1024;
    fn same_directory(a: &Metadata, b: &Metadata) -> bool {
        a.dev() == b.dev() && a.ino() == b.ino() && a.mode() == b.mode()
    }
    fn exists(path: &Path) -> Result<bool, &'static str> {
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(_) => Err("qa_output_boundary"),
        }
    }

    fn no_git_ancestor(path: &Path) -> Result<(), &'static str> {
        for ancestor in path.ancestors() {
            // Also reject bare Git directories and dangling .git symlinks.
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

    fn directory(path: &Path) -> Result<File, &'static str> {
        OpenOptions::new()
            .read(true)
            .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_output_directory")
    }

    fn anchored(file: &File) -> PathBuf {
        // Linux descriptor path keeps writes anchored to the opened directory,
        // including when its ordinary pathname is concurrently replaced.
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
            let value = Self {
                client,
                data,
                parent,
                path,
                parent_file,
                parent_identity,
            };
            value.verify()?;
            if exists(&value.path)? {
                return Err("qa_output_exists");
            }
            Ok(value)
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
            let result = directory(&path)?;
            self.verify_created(&result)?;
            Ok(result)
        }

        fn verify_created(&self, output: &File) -> Result<(), &'static str> {
            self.verify()?;
            let named = fs::symlink_metadata(&self.path).map_err(|_| "qa_output_changed")?;
            let handle = output.metadata().map_err(|_| "qa_output_changed")?;
            if !named.is_dir()
                || !same_directory(&named, &handle)
                || fs::canonicalize(anchored(output)).map_err(|_| "qa_output_changed")? != self.path
                || handle.mode() & 0o077 != 0
            {
                return Err("qa_output_changed");
            }
            no_git_ancestor(&self.path)
        }
    }

    enum Selection {
        All,
        Maps(Vec<u32>),
        Tiles(Vec<plan::TileRequest>),
    }
    impl Selection {
        fn parse(text: &str) -> Result<Self, &'static str> {
            if text.len() > 8192 {
                return Err("qa_selection_limit");
            }
            if text == "all" {
                return Ok(Self::All);
            }
            fn number(text: &str) -> Result<u32, &'static str> {
                if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("qa_selection_shape");
                }
                text.parse().map_err(|_| "qa_selection_number")
            }
            if let Some(text) = text.strip_prefix("maps:") {
                let rows = text.split(',').map(number).collect::<Result<Vec<_>, _>>()?;
                if rows.len() > 16 {
                    return Err("qa_selection_limit");
                }
                return Ok(Self::Maps(rows));
            }
            if let Some(text) = text.strip_prefix("tiles:") {
                let mut rows = Vec::new();
                for part in text.split(';') {
                    if rows.len() == 1024 {
                        return Err("qa_selection_limit");
                    }
                    let mut values = part.split(',');
                    let map_id = number(values.next().ok_or("qa_selection_shape")?)?;
                    let x = number(values.next().ok_or("qa_selection_shape")?)?;
                    let y = number(values.next().ok_or("qa_selection_shape")?)?;
                    if values.next().is_some() || x >= 64 || y >= 64 {
                        return Err("qa_selection_shape");
                    }
                    rows.push(plan::TileRequest {
                        map_id,
                        x: x as u8,
                        y: y as u8,
                    });
                }
                return Ok(Self::Tiles(rows));
            }
            Err("qa_selection_shape")
        }
        fn borrowed(&self) -> plan::Selection<'_> {
            match self {
                Self::All => plan::Selection::AllMaps,
                Self::Maps(v) => plan::Selection::MapIds(v),
                Self::Tiles(v) => plan::Selection::Tiles(v),
            }
        }
    }
    struct Stage {
        files: Vec<(String, Vec<u8>)>,
        memory: usize,
    }
    impl Stage {
        fn new() -> Result<Self, &'static str> {
            let mut files = Vec::new();
            files
                .try_reserve_exact(MAX_FILES)
                .map_err(|_| "qa_allocation")?;
            let memory = files.capacity() * size_of::<(String, Vec<u8>)>();
            Ok(Self { files, memory })
        }
        fn push(&mut self, name: String, bytes: &[u8]) -> Result<(), run::SinkError> {
            if self.files.len() >= MAX_FILES || self.files.iter().any(|f| f.0 == name) {
                return Err(run::SinkError);
            }
            let next = self
                .memory
                .checked_add(name.capacity())
                .and_then(|n| n.checked_add(bytes.len()))
                .ok_or(run::SinkError)?;
            if next > MAX_BYTES {
                return Err(run::SinkError);
            }
            let mut owned = Vec::new();
            owned
                .try_reserve_exact(bytes.len())
                .map_err(|_| run::SinkError)?;
            let actual = self
                .memory
                .checked_add(name.capacity())
                .and_then(|n| n.checked_add(owned.capacity()))
                .ok_or(run::SinkError)?;
            if actual > MAX_BYTES {
                return Err(run::SinkError);
            }
            owned.extend_from_slice(bytes);
            self.files.push((name, owned));
            self.memory = actual;
            Ok(())
        }
    }
    impl run::Sink for Stage {
        fn tile(
            &mut self,
            c: conversion::Context,
            _: run::SourceFacts,
            t: &conversion::ConvertedTile,
        ) -> Result<(), run::SinkError> {
            // Public planner defines the canonical decimal row-before-column identity.
            let path = plan::map_relative_path(c.map_id, c.tile_x, c.tile_y)
                .map_err(|_| run::SinkError)?;
            self.push(path, t.bytes())
        }
        fn tile_list(
            &mut self,
            map: u32,
            bytes: &[u8; tilelist::BYTES],
        ) -> Result<(), run::SinkError> {
            self.push(format!("maps/{map:04}.tilelist"), bytes)
        }
    }
    fn write_new(dir: &File, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        if Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(Component::Normal(_))
            )
        {
            return Err("qa_artifact_name");
        }
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(anchored(dir).join(name))
            .map_err(|_| "qa_output_file")?;
        out.write_all(bytes)
            .and_then(|_| out.sync_all())
            .map_err(|_| "qa_output_write")
    }
    pub fn main() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if !(3..=6).contains(&args.len()) {
            return Err("qa_usage");
        }
        let mut names = false;
        let mut pack = true;
        let mut locale = None;
        for arg in &args[3..] {
            if arg == "--names" && !names {
                names = true;
            } else if arg == "--no-pack" && pack {
                pack = false;
            } else if arg == "--locale=enUS" && locale.is_none() {
                locale = Some(Locale::EnUs);
            } else if arg == "--locale=esES" && locale.is_none() {
                locale = Some(Locale::EsEs);
            } else {
                return Err("qa_usage");
            }
        }
        let selection = Selection::parse(args[1].to_str().ok_or("qa_selection_shape")?)?;
        let client = Path::new(&args[0]);
        let destination = Destination::prepare(client, Path::new(&args[2]))?;
        let mut store = local_store::LocalStore::open(
            client,
            local_store::OpenOptions {
                locale: locale.unwrap_or(Locale::EsEs),
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: local_store::Limits::default(),
            },
        )
        .map_err(|e| e.code())?;
        if names {
            store
                .enable_names(root::name::Limits::default())
                .map_err(|e| e.code())?;
        }
        let mut stage = Stage::new()?;
        let defaults = run::Limits::default();
        let limits = run::Limits {
            max_reads: 4096,
            max_source_bytes: 256 * 1024 * 1024,
            max_output_bytes: MAX_BYTES as u64,
            max_memory_bytes: defaults.max_memory_bytes - MAX_BYTES,
            plan: plan::Limits {
                max_maps: 16,
                max_total_coordinates: 16 * 4096,
                max_explicit_tiles: 1024,
                ..defaults.plan
            },
            ..defaults
        };
        let complete = match run::run(&mut store, selection.borrowed(), pack, limits, &mut stage) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "{{\"status\":\"failed\",\"code\":\"{}\",\"read_attempts\":{},\"converted_provisional_tiles\":{}}}",
                    e.error.code(),
                    e.stats.read_attempts,
                    e.stats.converted_tiles
                );
                return Err("qa_terrain_run");
            }
        };
        let output = destination.create()?;
        let maps_path = anchored(&output).join("maps");
        DirBuilder::new()
            .mode(0o700)
            .create(&maps_path)
            .map_err(|_| "qa_output_directory")?;
        let maps = directory(&maps_path)?;
        for (name, bytes) in stage.files {
            destination.verify_created(&output)?;
            write_new(
                &maps,
                name.strip_prefix("maps/").ok_or("qa_artifact_name")?,
                &bytes,
            )?;
        }
        maps.sync_all().map_err(|_| "qa_output_sync")?;
        destination.verify_created(&output)?;
        let s = complete.stats();
        let p = complete.plan();
        let report = format!(
            "{{\"schema\":\"rce.terrain_qa.v1\",\"status\":\"complete\",\"build\":{},\"maps\":{},\"selected_coordinates\":{},\"converted_tiles\":{},\"tile_lists\":{},\"source_bytes\":{},\"output_bytes\":{},\"full_extractor_complete\":false}}\n",
            complete.build(),
            p.maps,
            p.selected_coordinates,
            s.converted_tiles,
            s.tile_lists,
            s.source_bytes,
            s.output_bytes
        );
        write_new(&output, "complete.json", report.as_bytes())?;
        output.sync_all().map_err(|_| "qa_output_sync")?;
        destination.verify_created(&output)?;
        print!("{report}");
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn selection_shapes_are_explicit_bounded_and_not_paths() {
            for good in ["all", "maps:0,4294967295", "tiles:7,3,2;7,1,4"] {
                assert!(Selection::parse(good).is_ok());
            }
            for bad in [
                "",
                "maps:",
                "maps:-1",
                "maps:4294967296",
                "tiles:7,64,0",
                "tiles:7,0",
                "tiles:7,0,1,2",
                "../maps",
                "all ",
            ] {
                assert!(Selection::parse(bad).is_err());
            }
        }
        #[test]
        fn duplicate_artifacts_never_overwrite() {
            let mut s = Stage::new().unwrap();
            s.push("maps/0007.tilelist".into(), b"one").unwrap();
            assert!(s.push("maps/0007.tilelist".into(), b"two").is_err());
            assert_eq!(s.files[0].1, b"one");
        }
    }
}
fn main() {
    #[cfg(target_os = "linux")]
    if let Err(code) = qa::main() {
        eprintln!("{{\"status\":\"failed\",\"code\":\"{code}\"}}");
        std::process::exit(4);
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("{{\"status\":\"failed\",\"code\":\"qa_linux_required\"}}");
        std::process::exit(4);
    }
}
