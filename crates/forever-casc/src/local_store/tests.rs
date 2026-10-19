// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original, complete synthetic on-disk CASC stores only.

use super::*;
use crate::blte::md5;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::local_index::jenkins;

#[cfg(target_os = "linux")]
#[path = "availability_tests.rs"]
mod availability_tests;
#[path = "encrypted_tests.rs"]
mod encrypted_tests;
#[path = "route_tests.rs"]
mod route_tests;

// Shared with collision and terrain integration tests. Existing store fixtures
// and tests retain their original behavior; no production API is exposed.
pub(crate) type CollisionFile<'a> = (u32, Option<&'a [u8]>, Vec<u8>);
pub(crate) struct CollisionFixture(Fixture);
impl CollisionFixture {
    pub(crate) fn new(files: &[CollisionFile<'_>]) -> Self {
        Self::for_build(files, 70205)
    }
    pub(crate) fn for_build(files: &[CollisionFile<'_>], build: u32) -> Self {
        Self(Self::builder(files, build).write())
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn with_probe_root_and_later_encoding(
        files: &[CollisionFile<'_>],
        build: u32,
        root: Vec<u8>,
        file_data_id: u32,
    ) -> Self {
        let mut builder = Self::builder(files, build);
        builder.root_override = Some(root);
        let index = files
            .iter()
            .position(|file| file.0 == file_data_id)
            .expect("probe fixture contains the requested FDID");
        let (content_key, content_size, _) = &builder.routes[index];
        let content_key = *content_key;
        let content_size = *content_size;
        let absent_ekey = [0x99; 16];
        let later_ekey = [0x88; 16];
        builder.routes[index].2 = vec![absent_ekey];
        builder
            .routes
            .push((content_key, content_size, vec![later_ekey]));
        builder
            .records
            .push((later_ekey, b'N', files[index].2.clone()));
        Self(builder.write())
    }
    fn builder(files: &[CollisionFile<'_>], build: u32) -> Builder {
        let mut builder = Builder::new(b"unused");
        builder.version = match build {
            70170 => "1.60.1.70170",
            70205 => "1.60.1.70205",
            70374 => "1.60.1.70374",
            70425 => "1.60.1.70425",
            70450 => "1.60.1.70450",
            _ => panic!("unsupported synthetic fixture build"),
        };
        builder.routes.clear();
        builder.records.clear();
        let mut root = b"TSFM".to_vec();
        for n in [
            20,
            2,
            files.len() as u32,
            files.iter().filter(|row| row.1.is_some()).count() as u32,
        ] {
            root.extend_from_slice(&n.to_le_bytes());
        }
        for (ordinal, (id, name, bytes)) in files.iter().enumerate() {
            let key = md5::digest(bytes);
            let ekey = [0x60 + ordinal as u8; 16];
            builder.routes.push((key, bytes.len() as u32, vec![ekey]));
            builder.records.push((ekey, b'N', bytes.clone()));
            for value in [1u32, 2, if name.is_some() { 0 } else { 0x1000_0000 }, 0] {
                root.extend_from_slice(&value.to_le_bytes());
            }
            root.push(0);
            root.extend_from_slice(&id.to_le_bytes());
            root.extend_from_slice(&key);
            if let Some(name) = name {
                root.extend_from_slice(&root::name::hash_name(name, 1024).unwrap().to_le_bytes());
            }
        }
        builder.root_override = Some(root);
        builder
    }
    /// Test-only framed N/E/N sources with real descriptor MD5 and full header
    /// EKey. Omitted plaintext exists only in the authored fixture declaration;
    /// no decoder decrypts or exposes the encrypted hole.
    #[cfg(target_os = "linux")]
    pub(crate) fn with_available_holes(
        files: &[CollisionFile<'_>],
        holes: &[(u32, std::ops::Range<usize>)],
    ) -> Self {
        Self::with_available_holes_for_build(files, holes, 70205)
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn with_available_holes_for_build(
        files: &[CollisionFile<'_>],
        holes: &[(u32, std::ops::Range<usize>)],
        build: u32,
    ) -> Self {
        let mut builder = Self::builder(files, build);
        let mut seen = std::collections::BTreeSet::new();
        for (id, range) in holes {
            assert!(seen.insert(*id));
            let at = files.iter().position(|f| f.0 == *id).unwrap();
            let bytes = &files[at].2;
            assert!(range.start < range.end && range.end <= bytes.len());
            let mut blocks = Vec::new();
            if range.start != 0 {
                let mut block = vec![b'N'];
                block.extend(&bytes[..range.start]);
                blocks.push((block, range.start));
            }
            let mut encrypted = vec![b'E', 8];
            encrypted.extend([0x11; 8]);
            encrypted.push(4);
            encrypted.extend([0; 4]);
            encrypted.extend([b'S', 0x22]);
            blocks.push((encrypted, range.end - range.start));
            if range.end < bytes.len() {
                let mut block = vec![b'N'];
                block.extend(&bytes[range.end..]);
                blocks.push((block, bytes.len() - range.end));
            }
            let header_size = 12 + blocks.len() * 24;
            let mut encoded = b"BLTE".to_vec();
            encoded.extend((header_size as u32).to_be_bytes());
            encoded.push(15);
            encoded.extend(&(blocks.len() as u32).to_be_bytes()[1..]);
            for (block, decoded) in &blocks {
                encoded.extend((block.len() as u32).to_be_bytes());
                encoded.extend((*decoded as u32).to_be_bytes());
                encoded.extend(md5::digest(block));
            }
            let ekey = md5::digest(&encoded);
            for (block, _) in blocks {
                encoded.extend(block);
            }
            builder.routes[at].2 = vec![ekey];
            builder.records[at] = (ekey, b'N', Vec::new());
            builder.framed_records.insert(ekey, encoded);
        }
        Self(builder.write())
    }
    pub(crate) fn open(&self, names: bool) -> LocalStore {
        let mut store = self.0.open();
        if names {
            store.enable_names(root::name::Limits::default()).unwrap();
        }
        store
    }
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.0.path
    }
    pub(crate) fn change_metadata(&self) {
        fs::write(&self.0.config_path, b"changed synthetic config").unwrap();
    }
    pub(crate) fn truncate_archive(&self) {
        fs::OpenOptions::new()
            .write(true)
            .open(self.0.path.join("Data/data/data.000"))
            .unwrap()
            .set_len(0)
            .unwrap();
    }
}

const CONTENT: &[u8] = b"synthetic complete local content";
const FILE_ID: u32 = 42;
const EKEY: [u8; 16] = [0x22; 16];
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
type Route = ([u8; 16], u32, Vec<[u8; 16]>);
type RootRow = (u32, u32, u32, [u8; 16]);
type Record = ([u8; 16], u8, Vec<u8>);

struct Builder {
    root: Vec<RootRow>,
    root_override: Option<Vec<u8>>,
    routes: Vec<Route>,
    records: Vec<Record>,
    framed_records: std::collections::BTreeMap<[u8; 16], Vec<u8>>,
    auxiliary: String,
    version: &'static str,
}
impl Builder {
    fn new(content: &[u8]) -> Self {
        let key = md5::digest(content);
        Self {
            root: vec![(FILE_ID, 0, 2, key)],
            root_override: None,
            routes: vec![(key, content.len() as u32, vec![EKEY])],
            records: vec![(EKEY, b'N', content.to_vec())],
            framed_records: std::collections::BTreeMap::new(),
            auxiliary: String::new(),
            version: "1.60.1.70205",
        }
    }
    fn write(self) -> Fixture {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rce-local-store-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let data = path.join("Data/data");
        fs::create_dir_all(&data).unwrap();
        let root = self.root_override.unwrap_or_else(|| root_file(&self.root));
        let root_key = md5::digest(&root);
        let mut routes = vec![(root_key, root.len() as u32, vec![ROOT_EKEY])];
        routes.extend(self.routes);
        let encoding = encoding_file(&routes);
        let encoding_key = md5::digest(&encoding);
        let config=format!("root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-{}\n{}",hex(root_key),hex(encoding_key),hex(ENCODING_EKEY),encoding.len(),encoding.len()+39,self.version,self.auxiliary).into_bytes();
        let config_key = md5::digest(&config);
        let config_path = io::config_path(&path, config_key);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, &config).unwrap();
        let build_info = build_info(config_key, self.version);
        fs::write(path.join(".build.info"), build_info).unwrap();
        let mut records = self.records;
        records.push((ROOT_EKEY, b'N', root));
        records.push((ENCODING_EKEY, b'N', encoding));
        let mut archive = Vec::new();
        let mut rows = Vec::new();
        for (key, mode, payload) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            if let Some(encoded) = self.framed_records.get(&key) {
                // Explicit test-only override: existing F/unsupported modes
                // retain their original headerless fixture behavior.
                archive.extend_from_slice(encoded);
            } else {
                archive.extend_from_slice(b"BLTE\0\0\0\0");
                archive.push(mode);
                archive.extend_from_slice(&payload);
            }
            rows.push(index_entry(
                key,
                offset,
                (archive.len() as u64 - offset) as u32,
            ));
        }
        fs::write(data.join("data.000"), archive).unwrap();
        for bucket in 0..16_u8 {
            let entries = if bucket == 0 {
                rows.clone()
            } else {
                vec![index_entry([0xe0 + bucket; 16], 0, 39)]
            };
            fs::write(
                data.join(format!("{bucket:02x}00000001.idx")),
                index_file(bucket, &entries),
            )
            .unwrap();
        }
        Fixture {
            path,
            config_path,
            rows,
        }
    }
}

struct Fixture {
    path: PathBuf,
    config_path: PathBuf,
    rows: Vec<[u8; 18]>,
}
impl Fixture {
    fn open(&self) -> LocalStore {
        LocalStore::open(&self.path, options()).unwrap()
    }
    fn index_path(&self, bucket: u8) -> PathBuf {
        self.path
            .join(format!("Data/data/{bucket:02x}00000001.idx"))
    }
    fn rewrite_rows(&self) {
        fs::write(self.index_path(0), index_file(0, &self.rows)).unwrap();
    }
    fn open_error(&self, opts: OpenOptions) -> Error {
        LocalStore::open(&self.path, opts)
            .err()
            .expect("expected rejection")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}

fn options() -> OpenOptions {
    OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: Limits::default(),
    }
}
fn hex(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn build_info(key: [u8; 16], version: &str) -> String {
    format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|{version}|{}|{}|enUS\n",
        hex(key),
        hex([0x55; 16])
    )
}
fn root_file(rows: &[RootRow]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    for value in [20, 2, rows.len() as u32, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for &(id, flags, locale, key) in rows {
        for value in [1, locale, flags | 0x10000000, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&key);
    }
    bytes
}
fn encoding_file(rows: &[Route]) -> Vec<u8> {
    let mut page = Vec::new();
    for (key, size, ekeys) in rows {
        page.extend_from_slice(&(ekeys.len() as u16).to_le_bytes());
        page.extend_from_slice(&size.to_be_bytes());
        page.extend_from_slice(key);
        for ekey in ekeys {
            page.extend_from_slice(ekey);
        }
    }
    assert!(page.len() < 1024);
    page.resize(1024, 0);
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(&rows[0].0);
    bytes.extend_from_slice(&md5::digest(&page));
    bytes.extend_from_slice(&page);
    bytes
}
fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
    let mut row = [0; 18];
    row[..9].copy_from_slice(&key[..9]);
    row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
    row[14..].copy_from_slice(&size.to_le_bytes());
    row
}
fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
    let header_hash = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&header_hash.to_le_bytes());
    bytes.extend_from_slice(&(rows.len() as u32 * 18).to_le_bytes());
    let mut checksum = (0, 0);
    for row in rows {
        checksum = jenkins::hashlittle2(row, checksum.0, checksum.1);
    }
    bytes.extend_from_slice(&checksum.0.to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(row);
    }
    bytes
}

#[test]
fn complete_store_bootstrap_and_read_return_owned_verified_bytes() {
    let fixture = Builder::new(CONTENT).write();
    let mut store = fixture.open();
    assert_eq!(store.build().build, 70205);
    assert_eq!(store.build().product, crate::PRODUCT);
    assert_eq!(
        store.build().config_build_uid.as_deref(),
        Some("wow_classic_beta")
    );
    assert_eq!(store.build().tags, "enUS");
    assert_eq!(store.build().cdn_key, [0x55; 16]);
    assert_eq!(store.stats().encoding_entries, 2);
    assert_eq!(store.stats().encoded_keys, 2);
    assert_eq!(store.stats().index_entries, 18);
    assert_eq!(store.stats().root_entries, 1);
    let file = store.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(file.bytes, CONTENT);
    assert_eq!(file.content_key, md5::digest(CONTENT));
    assert_eq!(file.file_data_id, FILE_ID);
    assert_eq!(file.candidate_attempts, 1);
    drop(store);
    assert_eq!(file.bytes, CONTENT);
}

#[test]
fn both_admitted_builds_work_and_unknown_build_rejects() {
    for version in ["1.60.1.70170", "1.60.1.70205"] {
        let mut builder = Builder::new(CONTENT);
        builder.version = version;
        let fixture = builder.write();
        assert_eq!(fixture.open().build().version, version);
    }
    let mut builder = Builder::new(CONTENT);
    builder.version = "1.60.1.99999";
    assert_eq!(
        builder.write().open_error(options()),
        Error::UnsupportedClient
    );
}

#[test]
fn duplicate_index_prefixes_and_complete_ekey_alternatives_survive() {
    let mut builder = Builder::new(CONTENT);
    builder.routes[0].2.insert(0, [0x66; 16]); // Missing encoding must not discard the available alternative.
    builder
        .records
        .insert(0, (EKEY, b'N', vec![b'x'; CONTENT.len()])); // Same nine-byte prefix, wrong full CKey.
    let fixture = builder.write();
    let mut store = fixture.open();
    let file = store.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(file.bytes, CONTENT);
    assert_eq!(file.candidate_attempts, 3);
    assert_eq!(store.stats().encoded_keys, 3);
}

#[test]
fn duplicate_encoding_rows_cannot_fill_the_first_unknown_size() {
    let mut builder = Builder::new(CONTENT);
    let key = md5::digest(CONTENT);
    builder.routes.insert(0, (key, u32::MAX, vec![[0x66; 16]]));
    let fixture = builder.write();
    let mut store = fixture.open();
    assert_eq!(
        store.read_file_data_id(FILE_ID).unwrap_err(),
        Error::UnknownSize
    );
    assert_eq!(store.stats().encoding_entries, 3);
}

#[test]
fn fully_declared_conflicting_sizes_keep_first_insertion_semantics() {
    let mut builder = Builder::new(CONTENT);
    let key = md5::digest(CONTENT);
    builder.routes.insert(0, (key, 1, vec![EKEY]));
    let fixture = builder.write();
    let mut store = fixture.open();
    assert_eq!(
        store.read_file_data_id(FILE_ID).unwrap_err(),
        Error::NoLocalEncoding(CandidateFailures {
            corrupt: 2,
            ..CandidateFailures::default()
        })
    );
}

#[test]
fn central_presence_selects_missing_root_once_without_availability_fallback() {
    let mut builder = Builder::new(CONTENT);
    let missing = [0x77; 16];
    builder.root.insert(0, (FILE_ID, 0, 2, missing));
    builder.routes.push((missing, 1, vec![[0x88; 16]]));
    let fixture = builder.write();
    let mut store = fixture.open();
    assert_eq!(
        store.read_file_data_id(FILE_ID).unwrap_err(),
        Error::NoLocalEncoding(CandidateFailures {
            missing: 1,
            ..CandidateFailures::default()
        })
    );
}

#[test]
fn auxiliary_config_ckey_presence_selects_missing_root_without_encoding_row() {
    let mut builder = Builder::new(CONTENT);
    let missing = [0x77; 16];
    builder.root.insert(0, (FILE_ID, 0, 2, missing));
    builder.auxiliary = format!("install = {}\ninstall-size = 1\n", hex(missing));
    let fixture = builder.write();
    let mut store = fixture.open();
    assert_eq!(
        store.read_file_data_id(FILE_ID).unwrap_err(),
        Error::NoLocalEncoding(CandidateFailures::default())
    );
}

#[test]
fn absent_central_ckey_is_skipped_before_selecting_root() {
    let mut builder = Builder::new(CONTENT);
    builder.root.insert(0, (FILE_ID, 0, 2, [0x77; 16]));
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
        CONTENT
    );
}

#[test]
fn root_audio_order_precedes_physical_order_but_locale_never_falls_back() {
    let mut builder = Builder::new(CONTENT);
    let missing = [0x77; 16];
    builder.root.insert(0, (FILE_ID, 0x80000000, 2, missing));
    builder.routes.push((missing, 1, vec![[0x88; 16]]));
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
        CONTENT
    );
    let mut opts = options();
    opts.locale = Locale::EsEs;
    assert_eq!(
        LocalStore::open(&fixture.path, opts)
            .unwrap()
            .read_file_data_id(FILE_ID)
            .unwrap_err(),
        Error::MissingFile
    );
}

#[test]
fn neutral_root_and_es_es_are_supported_without_relabeling_locale() {
    for mask in [0, 0x80] {
        let mut builder = Builder::new(CONTENT);
        builder.root[0].2 = mask;
        let fixture = builder.write();
        let mut opts = options();
        opts.locale = Locale::EsEs;
        assert_eq!(
            LocalStore::open(&fixture.path, opts)
                .unwrap()
                .read_file_data_id(FILE_ID)
                .unwrap()
                .bytes,
            CONTENT
        );
    }
}

#[test]
fn empty_content_requires_a_complete_local_blte_candidate() {
    let fixture = Builder::new(b"").write();
    assert!(
        fixture
            .open()
            .read_file_data_id(FILE_ID)
            .unwrap()
            .bytes
            .is_empty()
    );
    let mut builder = Builder::new(b"");
    builder.records.clear();
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap_err(),
        Error::NoLocalEncoding(CandidateFailures {
            missing: 1,
            ..CandidateFailures::default()
        })
    );
}

#[test]
fn encrypted_recursive_and_unknown_modes_never_return_padding() {
    for (mode, encrypted, unsupported) in [(b'E', 1, 0), (b'F', 0, 1), (b'Q', 0, 1)] {
        let mut builder = Builder::new(CONTENT);
        builder.records[0].1 = mode;
        let fixture = builder.write();
        assert_eq!(
            fixture.open().read_file_data_id(FILE_ID).unwrap_err(),
            Error::NoLocalEncoding(CandidateFailures {
                encrypted,
                unsupported,
                ..CandidateFailures::default()
            })
        );
    }
}

#[test]
fn incomplete_missing_archive_and_full_ckey_corruption_are_distinct() {
    for scenario in 0..3 {
        let mut fixture = Builder::new(CONTENT).write();
        if scenario == 0 {
            fixture.rows[0][9..14].copy_from_slice(&0x3fff_ffff_u64.to_be_bytes()[3..]);
        } else if scenario == 1 {
            fixture.rows[0][9..14].copy_from_slice(&(1_u64 << 30).to_be_bytes()[3..]);
        } else {
            let mut bytes = fs::read(fixture.path.join("Data/data/data.000")).unwrap();
            bytes[39] ^= 1;
            fs::write(fixture.path.join("Data/data/data.000"), bytes).unwrap();
        }
        fixture.rewrite_rows();
        let error = fixture.open().read_file_data_id(FILE_ID).unwrap_err();
        assert_eq!(
            error,
            Error::NoLocalEncoding(CandidateFailures {
                incomplete: u32::from(scenario == 0),
                missing: u32::from(scenario == 1),
                corrupt: u32::from(scenario == 2),
                ..CandidateFailures::default()
            })
        );
    }
}

#[test]
fn unknown_decoded_size_is_explicit_and_candidate_attempts_are_bounded() {
    let mut builder = Builder::new(CONTENT);
    builder.routes[0].1 = u32::MAX;
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap_err(),
        Error::UnknownSize
    );
    let mut builder = Builder::new(CONTENT);
    builder.routes[0].2.insert(0, [0x66; 16]);
    let fixture = builder.write();
    let mut opts = options();
    opts.limits.max_candidate_attempts = 1;
    assert_eq!(
        LocalStore::open(&fixture.path, opts)
            .unwrap()
            .read_file_data_id(FILE_ID)
            .unwrap_err(),
        Error::Limit
    );
}

#[test]
fn caller_read_limit_can_tighten_but_never_enlarge_store_policy() {
    let fixture = Builder::new(CONTENT).write();
    let mut store = fixture.open();
    assert_eq!(
        store.read_file_data_id_with_limit(FILE_ID, 1).unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, CONTENT.len() as u64)
            .unwrap()
            .bytes,
        CONTENT
    );
    let mut opts = options();
    opts.limits.max_decoded_file_bytes = 1;
    let mut store = LocalStore::open(&fixture.path, opts).unwrap();
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, u64::MAX)
            .unwrap_err(),
        Error::Limit
    );
}

#[test]
fn missing_invalid_ids_and_filtered_root_return_no_data() {
    let fixture = Builder::new(CONTENT).write();
    let mut store = fixture.open();
    for id in [0, u32::MAX, FILE_ID + 1] {
        assert_eq!(store.read_file_data_id(id).unwrap_err(), Error::MissingFile);
    }
    for flags in [0x80, 0x100] {
        let mut builder = Builder::new(CONTENT);
        builder.root[0].1 = flags;
        let fixture = builder.write();
        assert_eq!(
            fixture.open().read_file_data_id(FILE_ID).unwrap_err(),
            Error::MissingFile
        );
    }
}

#[test]
fn exact_build_info_guard_detects_same_length_and_mtime_rewrite() {
    let fixture = Builder::new(CONTENT).write();
    let mut store = fixture.open();
    let path = fixture.path.join(".build.info");
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let bytes = fs::read_to_string(&path)
        .unwrap()
        .replace("|enUS\n", "|esES\n");
    fs::write(&path, bytes).unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(
        store.read_file_data_id(FILE_ID).unwrap_err(),
        Error::InputChanged
    );
}

#[test]
fn metadata_index_and_directory_changes_invalidate_open_store() {
    for choice in 0..3 {
        let fixture = Builder::new(CONTENT).write();
        let mut store = fixture.open();
        let path = match choice {
            0 => fixture.config_path.clone(),
            1 => fixture.index_path(0),
            _ => fixture.path.join("Data/data/new-marker"),
        };
        fs::write(path, b"changed").unwrap();
        #[cfg(unix)]
        if choice == 2 {
            // This exercises metadata change detection. A newly created name
            // may share the captured directory timestamp on a fast filesystem;
            // force a distinct timestamp instead of depending on clock ticks.
            // The production guard remains metadata-only, not a name snapshot.
            let dir = fixture.path.join("Data/data");
            let old = fs::metadata(&dir).unwrap().modified().unwrap();
            fs::File::open(&dir)
                .unwrap()
                .set_times(
                    fs::FileTimes::new().set_modified(old + std::time::Duration::from_secs(1)),
                )
                .unwrap();
        }
        assert_eq!(
            store.read_file_data_id(FILE_ID).unwrap_err(),
            Error::InputChanged
        );
    }
}

#[test]
fn inventory_counts_unknown_files_toward_directory_limit() {
    let fixture = Builder::new(CONTENT).write();
    let mut opts = options();
    opts.limits.max_directory_entries = 17; // 16 indices + archive.
    assert!(LocalStore::open(&fixture.path, opts).is_ok());
    fs::write(fixture.path.join("Data/data/unrelated"), b"").unwrap();
    assert_eq!(fixture.open_error(opts), Error::Limit);
}

#[test]
fn incomplete_index_set_duplicate_alias_and_newest_corruption_reject() {
    let fixture = Builder::new(CONTENT).write();
    fs::remove_file(fixture.index_path(15)).unwrap();
    assert_eq!(fixture.open_error(options()), Error::IndexSet);
    // A case-sensitive filesystem can contain two physical entries whose
    // parsed bucket/generation aliases. Default Windows NTFS resolves these
    // spellings to one entry instead; the pure selector tests reject duplicate
    // parsed candidates independently of host filesystem behavior.
    #[cfg(unix)]
    {
        let fixture = Builder::new(CONTENT).write();
        fs::copy(
            fixture.index_path(0),
            fixture.path.join("Data/data/0000000001.IDX"),
        )
        .unwrap();
        assert!(matches!(fixture.open_error(options()), Error::Index(_)));
    }
    #[cfg(windows)]
    {
        use forever_platform::{
            path::Limits,
            windows::{LinkPolicy, ReadFile},
        };
        let fixture = Builder::new(CONTENT).write();
        let directory = fixture.path.join("Data").join("data");
        let lower = directory.join("0000000001.idx");
        let upper = directory.join("0000000001.IDX");
        let identity = |path: &std::path::Path| {
            ReadFile::open(path, Limits::default(), LinkPolicy::Allow)
                .unwrap()
                .identity()
        };
        assert_eq!(identity(&lower), identity(&upper));
        assert_eq!(fs::read(&lower).unwrap(), fs::read(&upper).unwrap());
        let aliases = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| {
                name.to_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case("0000000001.idx"))
            })
            .count();
        assert_eq!(aliases, 1);
        assert_eq!(
            fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
            CONTENT
        );
    }
    let fixture = Builder::new(CONTENT).write();
    fs::write(
        fixture.path.join("Data/data/0000000002.idx"),
        b"bad newer generation",
    )
    .unwrap();
    assert!(matches!(fixture.open_error(options()), Error::Index(_)));
}

#[test]
fn explicit_index_and_root_layout_mismatch_never_autodetects() {
    let fixture = Builder::new(CONTENT).write();
    let mut opts = options();
    opts.index_layouts[0] = local_index::Layout::Paged;
    assert!(matches!(fixture.open_error(opts), Error::Index(_)));
    let mut opts = options();
    opts.root_layout = root::Layout::Header50893V1;
    assert!(matches!(fixture.open_error(opts), Error::Root(_)));
}

#[test]
fn policy_limits_reject_before_large_metadata_allocations() {
    let fixture = Builder::new(CONTENT).write();
    for variant in 0..8 {
        let mut opts = options();
        match variant {
            0 => opts.limits.max_resident_bytes = 0,
            1 => opts.limits.max_scratch_bytes = 1,
            2 => opts.limits.max_memory_bytes = 1,
            3 => opts.limits.max_build_info_bytes = 1,
            4 => opts.limits.max_index_files = 15,
            5 => opts.limits.index.max_entries = 17,
            6 => opts.limits.max_path_bytes = 1,
            _ => opts.limits.max_archive_span_bytes = 1,
        }
        let error = fixture.open_error(opts);
        assert!(
            matches!(
                error,
                Error::Limit | Error::Index(local_index::Error::Limit)
            ),
            "{variant}: {error:?}"
        );
    }
}

#[test]
fn unique_typed_build_key_active_selection_and_rows_are_required() {
    let text = build_info([0x11; 16], "1.60.1.70205");
    assert_eq!(build_info::parse(text.as_bytes(), 0), Err(Error::Limit));
    for altered in [
        text.replace("Build Key!HEX:16", "Build Key!HEX:16|Build Key!HEX:16"),
        text.replace("Build Key!HEX:16", "Build Key!STRING:0"),
        text.replace("|1|", "|2|"),
        text.replace(&hex([0x11; 16]), "invalid"),
        text.replace("enUS", "xx"),
    ] {
        assert_eq!(
            build_info::parse(altered.as_bytes(), 10),
            Err(Error::BuildMetadata)
        );
    }
    let row = text.lines().nth(1).unwrap();
    let duplicated = format!("{text}{row}\n");
    assert_eq!(
        build_info::parse(duplicated.as_bytes(), 10),
        Err(Error::AmbiguousClient)
    );
}

#[test]
fn zlib_blte_content_is_read_through_the_same_verified_archive_path() {
    // Independent Python stdlib zlib.compress(CONTENT), not generated by decoder.
    let compressed = [
        120, 156, 43, 174, 204, 43, 201, 72, 45, 201, 76, 86, 72, 206, 207, 45, 200, 73, 45, 73,
        85, 200, 201, 79, 78, 204, 1, 114, 243, 74, 82, 243, 74, 0, 210, 91, 12, 155,
    ];
    let mut builder = Builder::new(CONTENT);
    builder.records[0] = (EKEY, b'Z', compressed.to_vec());
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
        CONTENT
    );
}

#[test]
fn all_config_central_reference_tags_affect_root_presence() {
    for tag in [
        "download",
        "install",
        "patch",
        "size",
        "vfs-root",
        "vfs-1",
        "vfs-4294967295",
    ] {
        let mut builder = Builder::new(CONTENT);
        let missing = [0x77; 16];
        builder.root.insert(0, (FILE_ID, 0, 2, missing));
        builder.auxiliary = format!(
            "{tag} = {} {}\n{tag}-size = 1\n",
            hex(missing),
            hex([0x88; 16])
        );
        let fixture = builder.write();
        assert_eq!(
            fixture.open().read_file_data_id(FILE_ID).unwrap_err(),
            Error::NoLocalEncoding(CandidateFailures {
                missing: 1,
                ..CandidateFailures::default()
            }),
            "{tag}"
        );
    }
}

#[test]
fn patch_single_ekey_does_not_create_a_false_central_ckey() {
    let mut builder = Builder::new(CONTENT);
    let missing = [0x77; 16];
    builder.root.insert(0, (FILE_ID, 0, 2, missing));
    builder.auxiliary = format!("patch = {}\n", hex(missing));
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
        CONTENT
    );
}

#[test]
fn only_build_references_can_fill_first_encoding_unknown_size() {
    let mut builder = Builder::new(CONTENT);
    let key = md5::digest(CONTENT);
    builder.routes.insert(0, (key, u32::MAX, vec![[0x66; 16]]));
    builder.routes[1].1 = 1; // Later ENCODING size is deliberately wrong and ignored.
    builder.auxiliary = format!(
        "download = {}\ndownload-size = {}\n",
        hex(key),
        CONTENT.len()
    );
    let fixture = builder.write();
    let mut store = fixture.open();
    let file = store.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(file.bytes, CONTENT);
    assert_eq!(file.candidate_attempts, 2);
}

#[test]
fn build_references_cannot_replace_a_known_first_encoding_size() {
    let mut builder = Builder::new(CONTENT);
    let key = md5::digest(CONTENT);
    builder.auxiliary = format!("download = {}\ndownload-size = 1\n", hex(key));
    let fixture = builder.write();
    assert_eq!(
        fixture.open().read_file_data_id(FILE_ID).unwrap().bytes,
        CONTENT
    );
}

#[path = "tests_name.rs"]
mod names;

#[path = "framing_tests.rs"]
mod framing_tests;

#[cfg(target_os = "linux")]
#[test]
fn scoped70374_bootstrap_keeps_legacy_admission_closed() {
    let fixture = Builder {
        version: "1.60.1.70374",
        ..Builder::new(CONTENT)
    }
    .write();
    assert_eq!(fixture.open_error(options()), Error::UnsupportedClient);
    assert_eq!(
        crate::admit(&fixture.path),
        Err(crate::Error::UnsupportedClient)
    );
    assert_eq!(crate::admit_db2_70374(&fixture.path), Ok(70374));
    let mut store = LocalStore::open_db2_70374(&fixture.path, options()).unwrap();
    assert_eq!(store.build().build, 70374);
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, 1000)
            .unwrap()
            .bytes,
        CONTENT
    );
    let keys = crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap();
    let store = LocalStore::open_db2_70374_with_keys(&fixture.path, options(), keys).unwrap();
    assert_eq!(store.build().build, 70374);
}
#[cfg(target_os = "linux")]
#[test]
fn scoped70374_rejects_old_builds_before_config_io() {
    for version in [
        "1.60.1.70170",
        "1.60.1.70205",
        "1.60.1.70338",
        "1.60.1.70375",
    ] {
        let fixture = Builder {
            version,
            ..Builder::new(CONTENT)
        }
        .write();
        fs::write(&fixture.config_path, b"poison config not to read").unwrap();
        assert_eq!(
            LocalStore::open_db2_70374(&fixture.path, options())
                .err()
                .unwrap(),
            Error::UnsupportedClient
        );
        assert_eq!(
            crate::admit_db2_70374(&fixture.path),
            Err(crate::Error::UnsupportedClient)
        );
    }
}
#[cfg(target_os = "linux")]
#[test]
fn scoped70374_keeps_bootstrap_source_identity_guards() {
    let fixture = Builder {
        version: "1.60.1.70374",
        ..Builder::new(CONTENT)
    }
    .write();
    let mut store = LocalStore::open_db2_70374(&fixture.path, options()).unwrap();
    fs::write(
        fixture.path.join(".build.info"),
        b"authored metadata changed",
    )
    .unwrap();
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, 1000)
            .err()
            .unwrap(),
        Error::InputChanged
    );
}

#[cfg(target_os = "linux")]
#[path = "bootstrap_70425_tests.rs"]
mod bootstrap_70425_tests;
