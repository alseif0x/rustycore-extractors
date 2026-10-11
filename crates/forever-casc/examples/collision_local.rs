// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded local collision QA adapter.
//! Linux QA: collision_local <client> <plan> <new-output> [--names].
//! Plan: one `m2\tFDID` or `wmo\tFDID` per line, optional final newline.
//! This is not a product manifest or the original extractor startup sequence.
//! WMO plans load LiquidType/Material; GameObjectDisplayInfo is not loaded.
//! No output is created before the public collector successfully finishes.
//! Lookup construction has separate byte/row ceilings; retained capacities
//! reduce the collector's memory allowance. These are logical, not RSS limits.

#[cfg(target_os = "linux")]
mod qa {
    use std::collections::BTreeMap;
    use std::fs::{self, DirBuilder, File, Metadata, OpenOptions};
    use std::io::{self, BufWriter, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    use std::path::{Component, Path, PathBuf};

    use forever_casc::collision::{
        self, Class, Extraction, Extractor, InputRef, ModelRequest, Status,
    };
    use forever_casc::{Locale, local_index, local_store, root};
    use forever_db2::terrain_sources::{
        self, LiquidMaterialFields, LiquidTypeFields, SourceRecords,
    };
    use forever_vmap::wmo::liquid::LiquidLookup;

    const MAX_PLAN_BYTES: usize = 64 * 1024;
    const MAX_REQUESTS: usize = 256;
    const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
    const MAX_LOOKUP_ROWS: usize = 65_536;
    const MAX_LOOKUP_BYTES: usize = 2 * 1024 * 1024;
    const MAX_CORRELATION_BYTES: usize = 64 * 1024 * 1024;
    const O_NONBLOCK: i32 = 0x800;
    const O_DIRECTORY: i32 = 0x10000;
    const O_NOFOLLOW: i32 = 0x20000;

    fn same(a: &Metadata, b: &Metadata) -> bool {
        same_directory(a, b)
            && a.len() == b.len()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
    }

    fn same_directory(a: &Metadata, b: &Metadata) -> bool {
        a.dev() == b.dev() && a.ino() == b.ino() && a.mode() == b.mode()
    }

    struct Plan {
        file: File,
        path: PathBuf,
        identity: Metadata,
        requests: Vec<(Class, u32)>,
    }

    impl Plan {
        fn read(path: &Path) -> Result<Self, &'static str> {
            let identity = fs::symlink_metadata(path).map_err(|_| "qa_plan_metadata")?;
            if !identity.is_file() || identity.file_type().is_symlink() {
                return Err("qa_plan_not_regular");
            }
            let length = usize::try_from(identity.len()).map_err(|_| "qa_plan_limit")?;
            if length == 0 || length > MAX_PLAN_BYTES {
                return Err("qa_plan_limit");
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(O_NOFOLLOW | O_NONBLOCK)
                .open(path)
                .map_err(|_| "qa_plan_open")?;
            if !same(&identity, &file.metadata().map_err(|_| "qa_plan_metadata")?) {
                return Err("qa_plan_changed");
            }
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(length)
                .map_err(|_| "qa_allocation")?;
            bytes.resize(length, 0);
            let read_result = file.read_exact(&mut bytes);
            let mut tail = [0];
            let tail_result = file.read(&mut tail);
            let after = file.metadata().map_err(|_| "qa_plan_changed")?;
            let named = fs::symlink_metadata(path).map_err(|_| "qa_plan_changed")?;
            if !same(&identity, &after) || !same(&identity, &named) {
                return Err("qa_plan_changed");
            }
            read_result.map_err(|_| "qa_plan_read")?;
            if tail_result.map_err(|_| "qa_plan_read")? != 0 {
                return Err("qa_plan_changed");
            }
            let text = std::str::from_utf8(&bytes).map_err(|_| "qa_plan_shape")?;
            let mut requests = Vec::new();
            requests
                .try_reserve_exact(MAX_REQUESTS)
                .map_err(|_| "qa_allocation")?;
            for line in text.strip_suffix('\n').unwrap_or(text).split('\n') {
                if requests.len() == MAX_REQUESTS {
                    return Err("qa_plan_limit");
                }
                let (class, id) = line.split_once('\t').ok_or("qa_plan_shape")?;
                let class = match class {
                    "m2" => Class::M2,
                    "wmo" => Class::Wmo,
                    _ => return Err("qa_plan_shape"),
                };
                if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
                    return Err("qa_plan_shape");
                }
                let id: u32 = id.parse().map_err(|_| "qa_plan_id")?;
                if id == 0 || id == u32::MAX {
                    return Err("qa_plan_id");
                }
                requests.push((class, id));
            }
            Ok(Self {
                file,
                path: path.to_path_buf(),
                identity,
                requests,
            })
        }

        fn verify(&self) -> Result<(), &'static str> {
            let handle = self.file.metadata().map_err(|_| "qa_plan_changed")?;
            let named = fs::symlink_metadata(&self.path).map_err(|_| "qa_plan_changed")?;
            if !same(&self.identity, &handle) || !same(&self.identity, &named) {
                return Err("qa_plan_changed");
            }
            Ok(())
        }
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

    #[derive(Default)]
    struct Lookups {
        types: Vec<(u32, LiquidTypeFields)>,
        materials: Vec<(u32, LiquidMaterialFields)>,
        source_bytes: usize,
    }

    impl LiquidLookup for Lookups {
        fn material_id(&self, id: u32) -> Option<u8> {
            self.types
                .binary_search_by_key(&id, |row| row.0)
                .ok()
                .map(|i| self.types[i].1.material_id)
        }
        fn material_flags(&self, id: u8) -> Option<u32> {
            self.materials
                .binary_search_by_key(&u32::from(id), |row| row.0)
                .ok()
                .map(|i| self.materials[i].1.flags_bits)
        }
    }

    fn bound_rows<T>(rows: &SourceRecords<T>) -> Result<(), &'static str> {
        // Each copy may insert both an absent source and a destination.
        let upper = rows
            .copies()
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(rows.direct_records().len()))
            .ok_or("qa_lookup_limit")?;
        if upper > MAX_LOOKUP_ROWS {
            return Err("qa_lookup_limit");
        }
        Ok(())
    }

    fn sorted_lookup<T>(rows: BTreeMap<u32, T>) -> Result<Vec<(u32, T)>, &'static str> {
        let mut result = Vec::new();
        if rows.len() > MAX_LOOKUP_ROWS {
            return Err("qa_lookup_limit");
        }
        result
            .try_reserve_exact(rows.len())
            .map_err(|_| "qa_allocation")?;
        if result
            .capacity()
            .checked_mul(size_of::<(u32, T)>())
            .ok_or("qa_lookup_limit")?
            > MAX_LOOKUP_BYTES
        {
            return Err("qa_lookup_limit");
        }
        result.extend(rows);
        Ok(result)
    }

    impl Lookups {
        fn load(store: &mut local_store::LocalStore) -> Result<Self, &'static str> {
            let types = store
                .read_file_data_id_with_limit(1_371_380, MAX_FILE_BYTES as u64)
                .map_err(|e| e.code())?;
            let mut source_bytes = types.bytes.len();
            let rows = terrain_sources::read_liquid_types(&types.bytes).map_err(|e| e.code())?;
            bound_rows(&rows)?;
            let types_map = rows.materialize().map_err(|e| e.code())?;
            drop(types);
            let types = sorted_lookup(types_map)?;
            let materials = store
                .read_file_data_id_with_limit(1_132_538, MAX_FILE_BYTES as u64)
                .map_err(|e| e.code())?;
            source_bytes = source_bytes
                .checked_add(materials.bytes.len())
                .ok_or("qa_lookup_limit")?;
            let rows =
                terrain_sources::read_liquid_materials(&materials.bytes).map_err(|e| e.code())?;
            bound_rows(&rows)?;
            let materials_map = rows.materialize().map_err(|e| e.code())?;
            drop(materials);
            let value = Self {
                types,
                materials: sorted_lookup(materials_map)?,
                source_bytes,
            };
            if value.resident_bytes()? > MAX_LOOKUP_BYTES {
                return Err("qa_lookup_limit");
            }
            Ok(value)
        }

        fn resident_bytes(&self) -> Result<usize, &'static str> {
            self.types
                .capacity()
                .checked_mul(size_of::<(u32, LiquidTypeFields)>())
                .and_then(|n| {
                    self.materials
                        .capacity()
                        .checked_mul(size_of::<(u32, LiquidMaterialFields)>())
                        .and_then(|m| n.checked_add(m))
                })
                .ok_or("qa_lookup_limit")
        }
    }

    fn class_name(class: Class) -> &'static str {
        match class {
            Class::M2 => "m2",
            Class::Wmo => "wmo",
        }
    }
    fn status_name(status: Status) -> &'static str {
        match status {
            Status::Collision => "collision",
            Status::NoCollision => "no_collision",
            Status::DoodadOnly => "doodad_only",
        }
    }

    fn write_hex(output: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        for byte in bytes {
            output.write_all(&[
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ])?;
        }
        Ok(())
    }

    fn correlation(output: &mut impl Write, plan: &Plan, result: &Extraction) -> io::Result<()> {
        // This private QA index contains client identifiers; never echo it.
        writeln!(output, "collision-local-qa-v1")?;
        for (ordinal, ((class, id), handle)) in
            plan.requests.iter().zip(result.requests()).enumerate()
        {
            writeln!(
                output,
                "request\t{ordinal}\t{}\t{id}\t{}",
                class_name(*class),
                handle.index()
            )?;
        }
        for (ordinal, model) in result.models().iter().enumerate() {
            writeln!(
                output,
                "model\t{ordinal}\t{}\t{}\t{}",
                class_name(model.class()),
                status_name(model.status()),
                model.raw().map_or(0, <[u8]>::len)
            )?;
            write!(output, "name_hex\t{ordinal}\t")?;
            write_hex(output, model.output_basename())?;
            writeln!(output)?;
            if let Some(source) = model.source() {
                write!(
                    output,
                    "source\t{ordinal}\t{}\t{}\t",
                    source.file_data_id(),
                    source.byte_count()
                )?;
                write_hex(output, &source.content_key())?;
                writeln!(output)?;
            }
            if let Some(wmo) = model.wmo_metadata() {
                writeln!(output, "sets\t{ordinal}\t{}", wmo.selection().set_count())?;
                for binding in wmo.bindings() {
                    writeln!(
                        output,
                        "binding\t{ordinal}\t{}\t{}",
                        binding.name_index,
                        binding.model.index()
                    )?;
                }
                for reference in wmo.selection().references() {
                    writeln!(output, "reference\t{ordinal}\t{reference}")?;
                }
                for (group, source) in wmo.group_sources().iter().enumerate() {
                    write!(
                        output,
                        "group\t{ordinal}\t{group}\t{}\t{}\t",
                        source.file_data_id(),
                        source.byte_count()
                    )?;
                    write_hex(output, &source.content_key())?;
                    writeln!(output)?;
                }
            }
        }
        Ok(())
    }

    struct Counter(usize);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let next = self
                .0
                .checked_add(bytes.len())
                .filter(|n| *n <= MAX_CORRELATION_BYTES)
                .ok_or_else(|| io::Error::from(io::ErrorKind::FileTooLarge))?;
            self.0 = next;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn new_file(directory: &File, name: &str) -> Result<File, &'static str> {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(anchored(directory).join(name))
            .map_err(|_| "qa_output_file")
    }

    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().take(6).collect();
        if !(args.len() == 4 || (args.len() == 5 && args[4] == "--names")) {
            return Err("qa_usage");
        }
        let names = args.len() == 5;
        let plan = Plan::read(Path::new(&args[2]))?;
        let destination = Destination::prepare(Path::new(&args[1]), Path::new(&args[3]))?;
        let mut store = local_store::LocalStore::open(
            &destination.client,
            local_store::OpenOptions {
                locale: Locale::EsEs,
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: local_store::Limits {
                    max_decoded_file_bytes: MAX_FILE_BYTES as u64,
                    ..Default::default()
                },
            },
        )
        .map_err(|e| e.code())?;
        if names {
            store
                .enable_names(root::name::Limits::default())
                .map_err(|e| e.code())?;
        }
        let liquid_tables_loaded = plan.requests.iter().any(|(class, _)| *class == Class::Wmo);
        let lookups = if liquid_tables_loaded {
            Lookups::load(&mut store)?
        } else {
            Lookups::default()
        };
        let limits = collision::Limits::default();
        let mut extractor = Extractor::new(
            &mut store,
            &lookups,
            collision::Limits {
                max_file_bytes: MAX_FILE_BYTES,
                max_requests: MAX_REQUESTS,
                max_memory_bytes: limits
                    .max_memory_bytes
                    .checked_sub(lookups.resident_bytes()?)
                    .ok_or("qa_lookup_limit")?,
                ..limits
            },
        )
        .map_err(|e| e.code())?;
        for (class, id) in &plan.requests {
            let input = InputRef::FileDataId(*id);
            let request = match class {
                Class::M2 => ModelRequest::M2(input),
                Class::Wmo => ModelRequest::Wmo(input),
            };
            extractor.extract(request).map_err(|e| e.code())?;
        }
        let result = extractor.finish().map_err(|e| e.code())?;
        plan.verify()?;
        let mut counter = Counter(0);
        correlation(&mut counter, &plan, &result).map_err(|_| "qa_correlation_limit")?;
        let mut raw_files = 0usize;
        let mut raw_bytes = 0usize;
        let mut refs = 0usize;
        let mut m2 = 0usize;
        let mut wmo = 0usize;
        let mut collision = 0usize;
        let mut no_collision = 0usize;
        let mut doodad_only = 0usize;
        for model in result.models() {
            match model.class() {
                Class::M2 => m2 += 1,
                Class::Wmo => wmo += 1,
            }
            match model.status() {
                Status::Collision => collision += 1,
                Status::NoCollision => no_collision += 1,
                Status::DoodadOnly => doodad_only += 1,
            }
            if let Some(bytes) = model.raw() {
                raw_files += 1;
                raw_bytes = raw_bytes.checked_add(bytes.len()).ok_or("qa_count_limit")?;
            }
            if let Some(wmo) = model.wmo_metadata() {
                refs = refs
                    .checked_add(wmo.selection().references().len())
                    .ok_or("qa_count_limit")?;
            }
        }
        plan.verify()?;
        // From here onward an I/O failure may leave the exclusive private run
        // for inspection. It never prints success and never removes old data.
        let output = destination.create()?;
        for (ordinal, model) in result.models().iter().enumerate() {
            if let Some(bytes) = model.raw() {
                destination.verify_created(&output)?;
                let mut file = new_file(&output, &format!("model-{ordinal:06}.raw"))?;
                file.write_all(bytes)
                    .and_then(|()| file.sync_all())
                    .map_err(|_| "qa_output_write")?;
            }
        }
        destination.verify_created(&output)?;
        let mut index = BufWriter::with_capacity(64 * 1024, new_file(&output, "correlation.tsv")?);
        correlation(&mut index, &plan, &result)
            .and_then(|()| index.flush())
            .and_then(|()| index.get_ref().sync_all())
            .map_err(|_| "qa_output_write")?;
        destination.verify_created(&output)?;
        output
            .sync_all()
            .and_then(|()| destination.parent_file.sync_all())
            .map_err(|_| "qa_output_sync")?;
        let facts = result.facts();
        println!(
            "{{\"status\":\"ok\",\"requests\":{},\"models\":{},\"m2\":{m2},\"wmo\":{wmo},\"collision\":{collision},\"no_collision\":{no_collision},\"doodad_only\":{doodad_only},\"raw_files\":{raw_files},\"raw_bytes\":{raw_bytes},\"refs\":{refs},\"source_reads\":{},\"source_bytes\":{},\"dependency_requests\":{},\"names_enabled\":{names},\"liquid_tables_loaded\":{liquid_tables_loaded},\"lookup_source_reads\":{},\"lookup_source_bytes\":{},\"lookup_resident_bytes\":{}}}",
            plan.requests.len(),
            facts.models,
            facts.reads,
            facts.source_bytes,
            facts.dependency_requests,
            if liquid_tables_loaded { 2 } else { 0 },
            lookups.source_bytes,
            lookups.resident_bytes()?
        );
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(code) = qa::run() {
        println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
        std::process::exit(4);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("{{\"status\":\"error\",\"code\":\"qa_platform_unsupported\"}}");
    std::process::exit(4);
}
