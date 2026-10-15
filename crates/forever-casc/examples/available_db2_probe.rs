// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15: fixed two-source availability diagnostic, no publication.
//! available_db2_probe <client> <new-private-output>
//! Linux70374/esES, disabled keys, Contiguous across sixteen buckets/header50893v2. Exactly Map and
//! GameObjectDisplayInfo; no row/raw source export, native backend or retries.
//! A partial source/readable table is not complete product/runtime acceptance.
#[cfg(target_os = "linux")]
mod qa {
    use forever_casc::{
        Locale, integrity, local_index,
        local_store::{
            self,
            availability::{AvailableSource, RangeState},
        },
        root,
    };
    use forever_db2::{
        CharacterTable,
        availability::{self as db2, SectionCoverage, SectionState},
        catalog,
    };
    use std::{
        ffi::OsString,
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
    const FILE_CAP: u64 = 16 * 1024 * 1024;
    const REPORT_CAP: usize = 64 * 1024;
    const REQUESTS: [&str; 2] = ["Map", "GameObjectDisplayInfo"];

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

    fn arguments(args: &[OsString]) -> Result<(PathBuf, PathBuf), &'static str> {
        if args.len() != 3 {
            return Err("qa_usage");
        }
        Ok((PathBuf::from(&args[1]), PathBuf::from(&args[2])))
    }
    #[derive(Default)]
    struct Counts {
        declared: u64,
        available: u64,
        unavailable: u64,
        ranges: usize,
        encrypted_ranges: usize,
    }
    fn coverage(source: &AvailableSource) -> Result<Counts, &'static str> {
        let mut out = Counts {
            declared: source.declared_bytes(),
            available: source.available_bytes(),
            ..Counts::default()
        };
        if out.declared > FILE_CAP || source.coverage().len() > 65536 {
            return Err("qa_source_limit");
        }
        let (mut at, mut known) = (0u64, 0u64);
        for r in source.coverage() {
            if r.offset != at || r.bytes == 0 {
                return Err("qa_coverage_shape");
            }
            at = at.checked_add(r.bytes).ok_or("qa_source_limit")?;
            match r.state {
                RangeState::Available => {
                    known = known.checked_add(r.bytes).ok_or("qa_source_limit")?
                }
                RangeState::EncryptedUnavailable => out.encrypted_ranges += 1,
            }
        }
        if at != out.declared || known != out.available {
            return Err("qa_coverage_shape");
        }
        out.unavailable = out
            .declared
            .checked_sub(out.available)
            .ok_or("qa_coverage_shape")?;
        out.ranges = source.coverage().len();
        if !source.full_ckey_verified() && !source.encoded_key_authenticated() {
            return Err("qa_source_authentication");
        }
        Ok(out)
    }
    struct Projection {
        direct: usize,
        copies: usize,
        materialized: Option<usize>,
        text: usize,
        sections: Vec<SectionCoverage>,
    }
    fn project(input: &impl db2::Source, name: &str) -> Result<Projection, &'static str> {
        match name {
            "Map" => {
                let rows = db2::read_map_70374(input).map_err(|e| e.code())?;
                let (direct, copies, text) = (
                    rows.direct_records().len(),
                    rows.copies().len(),
                    rows.decoded_utf8_bytes(),
                );
                let rows = rows.materialize().map_err(|e| e.code())?;
                Ok(Projection {
                    direct,
                    copies,
                    materialized: Some(rows.rows().len()),
                    text,
                    sections: rows.sections().to_vec(),
                })
            }
            "GameObjectDisplayInfo" => {
                let rows = db2::read_gameobject_display_info_70374(input).map_err(|e| e.code())?;
                Ok(Projection {
                    direct: rows.direct_records().len(),
                    copies: rows.copies().len(),
                    materialized: None,
                    text: rows.decoded_utf8_bytes(),
                    sections: rows.sections().to_vec(),
                })
            }
            _ => Err("qa_request_scope"),
        }
    }
    fn projection_budget(
        store: &local_store::LocalStore,
        source: &AvailableSource,
    ) -> Result<(), &'static str> {
        let declared =
            usize::try_from(source.declared_bytes()).map_err(|_| "qa_projection_limit")?;
        let allowance = declared
            .checked_mul(32)
            .and_then(|n| n.checked_add(66 * 1024 * 1024))
            .ok_or("qa_projection_limit")?;
        let peak = store
            .stats()
            .resident_bytes
            .checked_add(source.resident_bytes().map_err(|e| e.code())?)
            .and_then(|n| n.checked_add(allowance))
            .ok_or("qa_projection_limit")?;
        if peak > local_store::Limits::default().max_memory_bytes {
            return Err("qa_projection_limit");
        }
        Ok(())
    }
    struct Table {
        name: &'static str,
        error: Option<&'static str>,
        source: Option<SourceFacts>,
        projection: Option<Projection>,
    }
    struct SourceFacts {
        counts: Counts,
        full_ckey: bool,
        encoded_key: bool,
        strict_attempts: usize,
        partial_attempts: usize,
        partial_encoded_bytes: u64,
    }
    impl Table {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                error: None,
                source: None,
                projection: None,
            }
        }
        fn json(&self) -> String {
            let error = self
                .error
                .map_or_else(|| "null".to_owned(), |e| format!("\"{e}\""));
            let source=self.source.as_ref().map_or_else(||"null".to_owned(),|s|format!(
                "{{\"declared_bytes\":{},\"available_bytes\":{},\"encrypted_unavailable_bytes\":{},\"coverage_ranges\":{},\"encrypted_ranges\":{},\"full_ckey_verified\":{},\"encoded_key_authenticated\":{},\"strict_candidate_attempts\":{},\"partial_candidate_attempts\":{},\"partial_encoded_bytes_read\":{}}}",
                s.counts.declared,s.counts.available,s.counts.unavailable,s.counts.ranges,s.counts.encrypted_ranges,s.full_ckey,s.encoded_key,s.strict_attempts,s.partial_attempts,s.partial_encoded_bytes));
            let projection=self.projection.as_ref().map_or_else(||"null".to_owned(),|p|{
                let sections=p.sections.iter().map(|s|format!("{{\"ordinal\":{},\"declared_records\":{},\"declared_copies\":{},\"state\":\"{}\"}}",s.ordinal,s.declared_records,s.declared_copies,if s.state==SectionState::Admitted{"admitted"}else{"unknown_tact"})).collect::<Vec<_>>().join(",");
                let admitted=p.sections.iter().filter(|s|s.state==SectionState::Admitted).count();
                format!("{{\"direct_records\":{},\"copy_pairs\":{},\"materialized_records\":{},\"decoded_utf8_bytes\":{},\"admitted_sections\":{},\"omitted_sections\":{},\"sections\":[{}]}}",p.direct,p.copies,p.materialized.map_or_else(||"null".to_owned(),|n|n.to_string()),p.text,admitted,p.sections.len()-admitted,sections)
            });
            format!(
                "{{\"table\":\"{}\",\"status\":\"{}\",\"error\":{},\"source\":{},\"projection\":{}}}",
                self.name,
                if self.error.is_none() && self.projection.is_some() {
                    "projected-available-rows"
                } else {
                    "failed"
                },
                error,
                source,
                projection
            )
        }
    }
    struct Report {
        identity: Option<[u8; 32]>,
        tables: Vec<Table>,
        attempts: usize,
        error: Option<&'static str>,
        closing: Vec<&'static str>,
    }
    impl Report {
        fn passed(&self) -> bool {
            self.error.is_none()
                && self.closing.is_empty()
                && self.attempts == 2
                && self.tables.len() == 2
                && self
                    .tables
                    .iter()
                    .all(|t| t.error.is_none() && t.projection.is_some())
        }
        fn json(&self) -> Result<String, &'static str> {
            let tables = self
                .tables
                .iter()
                .map(Table::json)
                .collect::<Vec<_>>()
                .join(",");
            let errors = self
                .closing
                .iter()
                .map(|e| format!("\"{e}\""))
                .collect::<Vec<_>>()
                .join(",");
            let identity = self.identity.as_ref().map_or_else(
                || "null".to_owned(),
                |i| format!("\"{}\"", integrity::hex(i)),
            );
            let error = self
                .error
                .map_or_else(|| "null".to_owned(), |e| format!("\"{e}\""));
            let value = format!(
                "{{\"schema\":\"linux70374-map-godi-available-probe-v1\",\"status\":\"{}\",\"build\":70374,\"locale\":\"esES\",\"keys\":\"disabled\",\"source_profile\":\"{}\",\"db2_profile\":\"{}\",\"logical_content_identity\":{},\"requests_observed\":{},\"tables\":[{}],\"error\":{},\"closing_errors\":[{}],\"plaintext_exports\":0,\"extraction_accepted\":false,\"runtime_accepted\":false}}\n",
                if self.passed() { "passed" } else { "failed" },
                local_store::availability::PROFILE,
                db2::PROFILE,
                identity,
                self.attempts,
                tables,
                error,
                errors
            );
            if value.len() > REPORT_CAP {
                return Err("qa_report_limit");
            }
            Ok(value)
        }
    }
    fn execute(client: &Path) -> Report {
        let mut report = Report {
            identity: None,
            tables: Vec::new(),
            attempts: 0,
            error: None,
            closing: Vec::new(),
        };
        let options = local_store::OpenOptions {
            locale: Locale::EsEs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: local_store::Limits::default(),
        };
        let mut store = match local_store::LocalStore::open_db2_70374(client, options) {
            Ok(s) => s,
            Err(e) => {
                report.error = Some(e.code());
                return report;
            }
        };
        report.identity = Some(store.content_identity());
        for name in REQUESTS {
            if let Err(e) = store.verify_unchanged() {
                report.closing.push(e.code());
                break;
            }
            let mut table = Table::new(name);
            let id =
                match CharacterTable::from_catalog_id(catalog::WOWDBDEFS_70374_CATALOG_ID, name) {
                    Ok(Some(t)) => t.file_id(),
                    _ => {
                        report.error = Some("qa_catalog_scope");
                        break;
                    }
                };
            report.attempts += 1;
            let source = store.read_file_data_id_available_with_limit(id, FILE_CAP);
            // Independent close even when acquisition fails. A changed source
            // invalidates the entire diagnostic; no later read is attempted.
            let read_close = store.verify_unchanged();
            if let Err(e) = read_close {
                table.error = Some(e.code());
                report.tables.push(table);
                report.closing.push(e.code());
                break;
            }
            match source {
                Err(e) => table.error = Some(e.code()),
                Ok(source) => {
                    let result = (|| {
                        let counts = coverage(&source)?;
                        let work = source.work();
                        table.source = Some(SourceFacts {
                            counts,
                            full_ckey: source.full_ckey_verified(),
                            encoded_key: source.encoded_key_authenticated(),
                            strict_attempts: work.strict_candidate_attempts,
                            partial_attempts: work.partial_candidate_attempts,
                            partial_encoded_bytes: work.partial_encoded_bytes_read,
                        });
                        projection_budget(&store, &source)?;
                        table.projection = Some(project(&source, name)?);
                        Ok(())
                    })();
                    if let Err(code) = result {
                        table.error = Some(code);
                    }
                    // Source and typed buffers drop before the next request.
                }
            }
            let projection_close = store.verify_unchanged();
            report.tables.push(table);
            if let Err(e) = projection_close {
                report.closing.push(e.code());
            }
            if !report.closing.is_empty() {
                break;
            }
        }
        if let Err(e) = store.verify_unchanged() {
            report.closing.push(e.code());
        }
        report
    }
    pub fn run() -> Result<(bool, String), &'static str> {
        let args: Vec<_> = std::env::args_os().take(4).collect();
        let (client, output) = arguments(&args)?;
        let dest = Destination::prepare(&client, &output)?;
        let directory = dest.create()?;
        let report = execute(&client);
        let bytes = report.json()?;
        dest.verify_created(&directory)?;
        write_file(&directory, "report.json", bytes.as_bytes())?;
        dest.verify_created(&directory)?;
        let projected = report
            .tables
            .iter()
            .filter(|t| t.error.is_none() && t.projection.is_some())
            .count();
        Ok((
            report.passed(),
            format!(
                "{{\"status\":\"{}\",\"requests\":{},\"projected_tables\":{},\"report_sha256\":\"{}\"}}",
                if report.passed() { "passed" } else { "failed" },
                report.attempts,
                projected,
                integrity::hex(&integrity::digest(bytes.as_bytes()))
            ),
        ))
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            os::unix::fs::symlink,
            sync::atomic::{AtomicU64, Ordering},
        };
        struct Fixture {
            data: Vec<u8>,
            hole: bool,
        }
        impl db2::Source for Fixture {
            fn declared_bytes(&self) -> u64 {
                self.data.len() as u64
            }
            fn range(&self, offset: u64, len: usize) -> Result<&[u8], db2::Unavailable> {
                if self.hole || len == 0 {
                    return Err(db2::Unavailable);
                }
                let start = usize::try_from(offset).map_err(|_| db2::Unavailable)?;
                self.data
                    .get(start..start.checked_add(len).ok_or(db2::Unavailable)?)
                    .ok_or(db2::Unavailable)
            }
        }
        fn put32(data: &mut [u8], at: usize, value: u32) {
            data[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        fn empty(name: &str) -> Fixture {
            let table = catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
                .unwrap()
                .unwrap()
                .metadata();
            let count = table.file_field_count;
            let mut data = vec![0; 204 + count * 28];
            data[..4].copy_from_slice(b"WDC5");
            put32(&mut data, 4, 5);
            put32(&mut data, 140, count as u32);
            put32(&mut data, 144, 1);
            put32(&mut data, 152, table.table_hash.unwrap());
            put32(&mut data, 156, table.layout_hash);
            put32(&mut data, 176, count as u32);
            put32(&mut data, 188, (count * 24) as u32);
            Fixture { data, hole: false }
        }
        #[test]
        fn exact_two_positional_arguments_are_required_before_io() {
            for count in [0, 1, 2, 4] {
                assert!(arguments(&vec![OsString::from("x"); count]).is_err());
            }
            assert!(arguments(&["probe".into(), "client".into(), "new".into()]).is_ok());
        }
        #[test]
        fn fixed_requests_resolve_exact_current_catalog_fdids() {
            for (name, id) in REQUESTS.into_iter().zip([1349477, 1266277]) {
                assert_eq!(
                    CharacterTable::from_catalog_id(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
                        .unwrap()
                        .unwrap()
                        .file_id(),
                    id
                );
            }
        }
        #[test]
        fn authored_empty_map_projection_is_count_only_and_covered() {
            let p = project(&empty("Map"), "Map").unwrap();
            assert_eq!(
                (p.direct, p.copies, p.materialized, p.text),
                (0, 0, Some(0), 0)
            );
            assert!(p.sections.is_empty());
        }
        #[test]
        fn authored_empty_godi_never_materializes_copy_rows() {
            let p = project(&empty("GameObjectDisplayInfo"), "GameObjectDisplayInfo").unwrap();
            assert_eq!((p.direct, p.copies, p.materialized), (0, 0, None));
        }
        #[test]
        fn unavailable_authored_metadata_is_fatal_not_a_projection() {
            let mut f = empty("Map");
            f.hole = true;
            assert_eq!(project(&f, "Map").err(), Some("record_data_bounds"));
            assert_eq!(project(&f, "unrequested").err(), Some("qa_request_scope"));
        }
        #[test]
        fn closing_failure_invalidates_preceding_projection_counts() {
            let mut tables = Vec::new();
            for name in REQUESTS {
                let mut t = Table::new(name);
                t.projection = Some(project(&empty(name), name).unwrap());
                tables.push(t);
            }
            let mut r = Report {
                identity: Some([1; 32]),
                tables,
                attempts: 2,
                error: None,
                closing: Vec::new(),
            };
            assert!(r.passed());
            r.closing.push("local_store_input_changed");
            assert!(!r.passed());
            let s = r.json().unwrap();
            assert!(s.contains("\"status\":\"failed\""));
            assert!(s.contains("\"extraction_accepted\":false"));
            assert!(!s.contains("file_data_id"));
            assert!(!s.contains("key_name"));
        }
        struct Temp(PathBuf);
        impl Temp {
            fn new() -> Self {
                static NEXT: AtomicU64 = AtomicU64::new(0);
                let p = std::env::temp_dir().join(format!(
                    "rce-available-probe-authored-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                fs::create_dir(&p).unwrap();
                fs::create_dir_all(p.join("client/Data")).unwrap();
                Self(p)
            }
        }
        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        #[test]
        fn private_report_only_is_exclusive_and_readback_verified() {
            let t = Temp::new();
            let dest = Destination::prepare(&t.0.join("client"), &t.0.join("new")).unwrap();
            let dir = dest.create().unwrap();
            write_file(&dir, "report.json", b"{}\n").unwrap();
            dest.verify_created(&dir).unwrap();
            assert!(write_file(&dir, "report.json", b"{}\n").is_err());
            assert_eq!(
                fs::symlink_metadata(t.0.join("new")).unwrap().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::symlink_metadata(t.0.join("new/report.json"))
                    .unwrap()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(fs::read_dir(t.0.join("new")).unwrap().count(), 1);
        }
        #[test]
        fn output_inside_client_and_existing_or_symlink_destinations_reject() {
            let t = Temp::new();
            assert!(Destination::prepare(&t.0.join("client"), &t.0.join("client/new")).is_err());
            fs::create_dir(t.0.join("existing")).unwrap();
            assert!(Destination::prepare(&t.0.join("client"), &t.0.join("existing")).is_err());
            symlink(t.0.join("client"), t.0.join("alias")).unwrap();
            assert!(Destination::prepare(&t.0.join("client"), &t.0.join("alias")).is_err());
        }
    }
}
#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    std::panic::set_hook(Box::new(|_| eprintln!("qa_internal_failure")));
    match qa::run() {
        Ok((passed, summary)) => {
            println!("{summary}");
            if passed {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
        Err(code) => {
            eprintln!("{code}");
            std::process::ExitCode::FAILURE
        }
    }
}
#[cfg(not(target_os = "linux"))]
fn main() -> std::process::ExitCode {
    eprintln!("qa_platform_unsupported");
    std::process::ExitCode::from(2)
}
