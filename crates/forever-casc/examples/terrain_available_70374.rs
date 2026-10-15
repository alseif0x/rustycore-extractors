// SPDX-License-Identifier: GPL-3.0-or-later
//! terrain_available_70374 CLIENT NEW_PRIVATE_OUTPUT [--native-inputs]
//! Fixed esES disabled-key Map0 tiles(31,31)/(31,32), packing enabled.
//! All outputs provisional until whole run and publication checks close.
//! Private complete.json contains source coverage/identity; stdout aggregates only.
//! No product manifest, tilelist, resume or full-map/world/runtime claim.
#[cfg(target_os = "linux")]
mod qa {
    use forever_casc::{
        Locale, local_index, local_store, root,
        terrain::{available_70374 as selected, plan, run},
    };
    use forever_terrain::conversion;
    use std::fs::{self, DirBuilder, File, Metadata, OpenOptions};
    use std::io::{self, Read, Seek, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    use std::path::{Component, Path, PathBuf};
    const O_NONBLOCK: i32 = 0x800;
    const O_DIRECTORY: i32 = 0x10000;
    const O_NOFOLLOW: i32 = 0x20000;
    const MAX_BYTES: usize = 64 * 1024 * 1024;
    const MAX_FILES: usize = 2;
    const REQUESTS: [plan::TileRequest; 2] = [
        plan::TileRequest {
            map_id: 0,
            x: 31,
            y: 31,
        },
        plan::TileRequest {
            map_id: 0,
            x: 31,
            y: 32,
        },
    ];
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

    struct Stage {
        files: Vec<(String, Vec<u8>)>,
        memory: usize,
        native: bool,
        witnesses: usize,
        witness_sources: [Option<run::SourceFacts>; 2],
    }
    impl Stage {
        fn new(native: bool) -> Result<Self, &'static str> {
            let mut files = Vec::new();
            files
                .try_reserve_exact(if native { 7 } else { MAX_FILES })
                .map_err(|_| "qa_allocation")?;
            let memory = files.capacity() * size_of::<(String, Vec<u8>)>();
            Ok(Self {
                files,
                memory,
                native,
                witnesses: 0,
                witness_sources: [None; 2],
            })
        }
        fn push(&mut self, name: String, bytes: &[u8]) -> Result<(), run::SinkError> {
            if self.files.len() >= if self.native { 7 } else { MAX_FILES }
                || self.files.iter().any(|f| f.0 == name)
            {
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
    impl selected::Sink for Stage {
        fn native_witness(&mut self, w: selected::NativeWitness<'_>) -> Result<(), run::SinkError> {
            if !self.native {
                return Ok(());
            }
            if self.witnesses >= 2
                || w.adt.len() > 16 * 1024 * 1024
                || w.types.len() > 300000
                || w.materials.len() > 300000
            {
                return Err(run::SinkError);
            }
            let mut lookup = Vec::new();
            let size = 16 + w.types.len() * 6 + w.materials.len() * 9;
            if self
                .memory
                .checked_add(size.checked_mul(2).ok_or(run::SinkError)?)
                .ok_or(run::SinkError)?
                > MAX_BYTES
            {
                return Err(run::SinkError);
            }
            lookup.try_reserve_exact(size).map_err(|_| run::SinkError)?;
            lookup.extend(b"RCELIQ01");
            lookup.extend((w.types.len() as u32).to_le_bytes());
            lookup.extend((w.materials.len() as u32).to_le_bytes());
            for (id, v) in w.types {
                lookup.extend(id.to_le_bytes());
                lookup.extend([v.sound_bank, v.material_id]);
            }
            for (id, v) in w.materials {
                lookup.extend(id.to_le_bytes());
                lookup.extend(v.flags_bits.to_le_bytes());
                lookup.push(v.lvf as u8);
            }
            if self
                .memory
                .checked_add(lookup.capacity())
                .ok_or(run::SinkError)?
                > MAX_BYTES
            {
                return Err(run::SinkError);
            }
            if self.witnesses == 0 {
                self.push("native-inputs/lookup.bin".into(), &lookup)?;
            } else if self
                .files
                .iter()
                .find(|f| f.0 == "native-inputs/lookup.bin")
                .is_none_or(|f| f.1 != lookup)
            {
                return Err(run::SinkError);
            }
            drop(lookup);
            let mut context = b"RCECONV1".to_vec();
            context.extend(w.context.map_id.to_le_bytes());
            context.extend([w.context.tile_y, w.context.tile_x]);
            context.extend(w.context.build.to_le_bytes());
            context.push(u8::from(w.context.pack));
            for value in [16 * 1024 * 1024u32, 4096, 202112] {
                context.extend(value.to_le_bytes());
            }
            self.push(
                format!("native-inputs/context-{}.bin", self.witnesses),
                &context,
            )?;
            self.push(format!("native-inputs/adt-{}.bin", self.witnesses), w.adt)?;
            self.witness_sources[self.witnesses] = Some(w.source);
            self.witnesses += 1;
            Ok(())
        }

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
    }
    fn write_new(dir: &File, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
        if bytes.len() > MAX_BYTES {
            return Err("qa_output_limit");
        }
        if Path::new(name).components().count() != 1
            || !matches!(
                Path::new(name).components().next(),
                Some(Component::Normal(_))
            )
        {
            return Err("qa_artifact_name");
        }
        let mut out = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(anchored(dir).join(name))
            .map_err(|_| "qa_output_file")?;
        out.write_all(bytes)
            .and_then(|_| out.sync_all())
            .map_err(|_| "qa_output_write")?;
        let before = out.metadata().map_err(|_| "qa_output_readback")?;
        if before.nlink() != 1
            || before.mode() & 0o777 != 0o600
            || before.len() != bytes.len() as u64
        {
            return Err("qa_output_readback");
        }
        out.rewind().map_err(|_| "qa_output_readback")?;
        let mut readback = vec![0; bytes.len()];
        out.read_exact(&mut readback)
            .map_err(|_| "qa_output_readback")?;
        let mut end = [0];
        if readback != bytes || out.read(&mut end).map_err(|_| "qa_output_readback")? != 0 {
            return Err("qa_output_readback");
        }
        let after = out.metadata().map_err(|_| "qa_output_readback")?;
        let named =
            fs::symlink_metadata(anchored(dir).join(name)).map_err(|_| "qa_output_readback")?;
        if !same_directory(&before, &after)
            || !same_directory(&before, &named)
            || before.len() != after.len()
            || before.len() != named.len()
            || before.nlink() != after.nlink()
            || before.nlink() != named.nlink()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
            || before.ctime() != after.ctime()
            || before.ctime_nsec() != after.ctime_nsec()
        {
            return Err("qa_output_readback");
        }
        Ok(())
    }
    pub fn main() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() != 2 && !(args.len() == 3 && args[2] == "--native-inputs") {
            return Err("qa_usage");
        }
        let native = args.len() == 3;
        let client = Path::new(&args[0]);
        let destination = Destination::prepare(client, Path::new(&args[1]))?;
        let mut store = local_store::LocalStore::open_db2_70374(
            client,
            local_store::OpenOptions {
                locale: Locale::EsEs,
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: local_store::Limits::default(),
            },
        )
        .map_err(|e| e.code())?;
        let identity = store.content_identity();
        let mut stage = Stage::new(native)?;
        let defaults = selected::Limits::default();
        let limits = selected::Limits {
            resources: run::Limits {
                max_memory_bytes: defaults.resources.max_memory_bytes - MAX_BYTES,
                ..defaults.resources
            },
        };
        let complete = match selected::run_selected(&mut store, &REQUESTS, true, limits, &mut stage)
        {
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
        if stage.files.len() != if native { 7 } else { 2 }
            || (native && stage.witnesses != 2)
            || complete.stats().converted_tiles != 2
            || complete.plan().absent_tiles != 0
        {
            return Err("qa_selection_incomplete");
        }
        store.verify_unchanged().map_err(|e| e.code())?;
        let output = destination.create()?;
        let maps_path = anchored(&output).join("maps");
        DirBuilder::new()
            .mode(0o700)
            .create(&maps_path)
            .map_err(|_| "qa_output_directory")?;
        let maps = directory(&maps_path)?;
        let native_dir = if native {
            let p = anchored(&output).join("native-inputs");
            DirBuilder::new()
                .mode(0o700)
                .create(&p)
                .map_err(|_| "qa_output_directory")?;
            Some(directory(&p)?)
        } else {
            None
        };
        let native_sources = stage
            .witness_sources
            .iter()
            .flatten()
            .enumerate()
            .map(|(ordinal, f)| {
                let key = f
                    .content_key
                    .iter()
                    .map(|v| format!("{v:02x}"))
                    .collect::<String>();
                format!(
                    "{{\"ordinal\":{},\"file_data_id\":{},\"bytes\":{},\"content_key\":\"{}\"}}",
                    ordinal, f.file_data_id, f.bytes, key
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        for (name, bytes) in stage.files {
            destination.verify_created(&output)?;
            if let Some(leaf) = name.strip_prefix("maps/") {
                write_new(&maps, leaf, &bytes)?;
            } else {
                write_new(
                    native_dir.as_ref().ok_or("qa_artifact_name")?,
                    name.strip_prefix("native-inputs/")
                        .ok_or("qa_artifact_name")?,
                    &bytes,
                )?;
            }
        }
        if let Some(dir) = native_dir {
            dir.sync_all().map_err(|_| "qa_output_sync")?;
        }
        maps.sync_all().map_err(|_| "qa_output_sync")?;
        destination.verify_created(&output)?;
        let s = complete.stats();
        let p = complete.plan();
        let m = complete.map();
        let sections=m.sections.iter().flatten().map(|v|format!("{{\"ordinal\":{},\"declared_records\":{},\"declared_copies\":{},\"admitted\":{}}}",v.ordinal,v.declared_records,v.declared_copies,v.state==forever_db2::availability::SectionState::Admitted)).collect::<Vec<_>>().join(",");
        let identity = identity
            .iter()
            .map(|v| format!("{v:02x}"))
            .collect::<String>();
        let report = format!(
            "{{\"schema\":\"rce.terrain_selected_70374_qa.v1\",\"status\":\"complete_selected_tiles\",\"profile\":\"{}\",\"build\":70374,\"maps\":{},\"selected_coordinates\":{},\"converted_tiles\":{},\"tile_lists\":0,\"logical_source_bytes_including_declared_map\":{},\"output_bytes\":{},\"logical_content_identity\":\"{}\",\"map\":{{\"declared_bytes\":{},\"available_bytes\":{},\"full_ckey_verified\":{},\"encoded_key_authenticated\":{},\"direct_rows\":{},\"materialized_rows\":{},\"sections\":[{}]}},\"full_extractor_complete\":false,\"native_sources\":[{}],\"native_inputs\":{},\"product_publication\":false}}\n",
            selected::PROFILE,
            p.maps,
            p.selected_coordinates,
            s.converted_tiles,
            s.source_bytes,
            s.output_bytes,
            identity,
            m.declared_bytes,
            m.available_bytes,
            m.full_ckey_verified,
            m.encoded_key_authenticated,
            m.direct_rows,
            m.materialized_rows,
            sections,
            native_sources,
            native
        );
        if report.len() > 64 * 1024 {
            return Err("qa_report_limit");
        }
        write_new(&output, "complete.json", report.as_bytes())?;
        output.sync_all().map_err(|_| "qa_output_sync")?;
        destination.verify_created(&output)?;
        store.verify_unchanged().map_err(|e| e.code())?;
        println!(
            "{{\"status\":\"complete_selected_tiles\",\"converted_tiles\":2,\"tile_lists\":0}}"
        );
        Ok(())
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn fixed_selection_is_two_tiles_no_world_alias() {
            assert_eq!(REQUESTS.len(), 2);
            assert_eq!(
                (REQUESTS[0].map_id, REQUESTS[0].x, REQUESTS[0].y),
                (0, 31, 31)
            );
            assert_eq!(
                (REQUESTS[1].map_id, REQUESTS[1].x, REQUESTS[1].y),
                (0, 31, 32)
            );
            assert_eq!(MAX_FILES, 2);
        }
        #[test]
        fn stage_has_no_tilelist_path_and_rejects_aliases() {
            let mut stage = Stage::new(false).unwrap();
            stage
                .push("maps/0000_31_31.map".into(), b"authored")
                .unwrap();
            assert!(
                stage
                    .push("maps/0000_31_31.map".into(), b"duplicate")
                    .is_err()
            );
            stage
                .push("maps/0000_32_31.map".into(), b"authored")
                .unwrap();
            assert!(stage.push("third".into(), b"extra").is_err());
        }
        #[test]
        fn native_witness_is_optional_complete_bounded_and_exact_wire() {
            let adt = b"authored complete ADT";
            let types = [(
                22,
                forever_db2::terrain_sources::LiquidTypeFields {
                    sound_bank: 0,
                    material_id: 7,
                },
            )];
            let materials = [(
                7,
                forever_db2::terrain_sources::LiquidMaterialFields {
                    flags_bits: 2,
                    lvf: 0,
                },
            )];
            let w = || selected::NativeWitness {
                adt,
                types: &types,
                materials: &materials,
                context: conversion::Context {
                    map_id: 0,
                    tile_x: 31,
                    tile_y: 31,
                    build: 70374,
                    pack: true,
                },
                source: run::SourceFacts {
                    step: run::Step::Adt {
                        map: 0,
                        x: 31,
                        y: 31,
                    },
                    file_data_id: 201,
                    content_key: [0; 16],
                    bytes: adt.len(),
                },
            };
            let mut off = Stage::new(false).unwrap();
            selected::Sink::native_witness(&mut off, w()).unwrap();
            assert!(off.files.is_empty());
            let mut on = Stage::new(true).unwrap();
            on.push("maps/a.map".into(), b"authored MAPS").unwrap();
            selected::Sink::native_witness(&mut on, w()).unwrap();
            on.push("maps/b.map".into(), b"authored MAPS").unwrap();
            selected::Sink::native_witness(&mut on, w()).unwrap();
            assert_eq!(on.files.len(), 7);
            assert_eq!(on.witnesses, 2);
            let lookup = &on
                .files
                .iter()
                .find(|f| f.0 == "native-inputs/lookup.bin")
                .unwrap()
                .1;
            assert_eq!(&lookup[..8], b"RCELIQ01");
            assert_eq!(lookup.len(), 31);
            let ctx = &on
                .files
                .iter()
                .find(|f| f.0 == "native-inputs/context-0.bin")
                .unwrap()
                .1;
            assert_eq!(ctx.len(), 31);
            assert_eq!(&ctx[..8], b"RCECONV1");
            assert_eq!(u32::from_le_bytes(ctx[14..18].try_into().unwrap()), 70374);
            assert_eq!(
                &on.files
                    .iter()
                    .find(|f| f.0 == "native-inputs/adt-1.bin")
                    .unwrap()
                    .1,
                adt
            );
            assert!(selected::Sink::native_witness(&mut on, w()).is_err());
        }
        #[test]
        fn output_names_reject_escape_before_writing() {
            let dir = directory(std::path::Path::new("/tmp")).unwrap();
            assert!(write_new(&dir, "../invalid", b"authored").is_err());
            assert!(write_new(&dir, ".", b"authored").is_err());
        }
    }
}
#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    match qa::main() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            std::process::ExitCode::from(4)
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("qa_linux_only");
    std::process::ExitCode::from(2)
}
