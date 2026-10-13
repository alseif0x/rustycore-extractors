// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: complete streaming synthetic CASC with ENCODING pages.
// This test-only writer emits the same reviewed local formats as the authored
// LocalStore fixture. It keeps one supplied body active and never wraps EKeys.
use crate::{Locale, blte::md5, local_index, local_store, root};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const PAGE_BYTES: usize = 1024;
const ROUTE_BYTES: usize = 38;
const ROOT_EKEY: [u8; 16] = [0x31; 16];
const ENCODING_EKEY: [u8; 16] = [0x32; 16];
type Route = ([u8; 16], u32, [u8; 16]);
pub(super) struct SyntheticFixture {
    path: PathBuf,
    pub source_bytes: usize,
    pub encoding_pages: usize,
}
impl SyntheticFixture {
    pub fn new(files: impl IntoIterator<Item = (u32, Vec<u8>)>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-disk-world-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let data = path.join("Data/data");
        fs::create_dir_all(&data).unwrap();
        let mut archive = fs::File::create(data.join("data.000")).unwrap();
        let mut offset = 0u64;
        let mut rows = Vec::new();
        let mut routes = Vec::new();
        let mut root_rows = Vec::new();
        let mut source_bytes = 0;
        for (ordinal, (id, body)) in files.into_iter().enumerate() {
            let mut ekey = [0x71; 16];
            ekey[..8].copy_from_slice(&(ordinal as u64).to_be_bytes());
            let key = md5::digest(&body);
            routes.push((key, u32::try_from(body.len()).unwrap(), ekey));
            root_rows.push((id, key));
            append(&mut archive, &mut offset, &mut rows, ekey, &body);
            source_bytes += body.len();
        }
        let mut root_bytes = b"TSFM".to_vec();
        for value in [20, 2, root_rows.len() as u32, 0] {
            root_bytes.extend(value.to_le_bytes());
        }
        for (id, key) in root_rows {
            for value in [1u32, 2, 0x1000_0000, 0] {
                root_bytes.extend(value.to_le_bytes());
            }
            root_bytes.push(0);
            root_bytes.extend(id.to_le_bytes());
            root_bytes.extend(key);
        }
        let root_key = md5::digest(&root_bytes);
        routes.push((root_key, root_bytes.len() as u32, ROOT_EKEY));
        append(&mut archive, &mut offset, &mut rows, ROOT_EKEY, &root_bytes);
        let encoding_pages = routes.len().div_ceil(PAGE_BYTES / ROUTE_BYTES);
        let encoding = encoding(&routes);
        let encoding_key = md5::digest(&encoding);
        append(
            &mut archive,
            &mut offset,
            &mut rows,
            ENCODING_EKEY,
            &encoding,
        );
        archive.sync_all().unwrap();
        drop(archive);
        let config = format!(
            "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70205\n",
            hex(root_key),
            hex(encoding_key),
            hex(ENCODING_EKEY),
            encoding.len(),
            encoding.len() + 39
        );
        let config_key = md5::digest(config.as_bytes());
        let key = hex(config_key);
        let config_path = path
            .join("Data/config")
            .join(&key[..2])
            .join(&key[2..4])
            .join(&key);
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(config_path, config).unwrap();
        fs::write(path.join(".build.info"), format!(
            "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70205|{key}|{}|enUS\n", hex([0x55; 16]))).unwrap();
        for bucket in 0..16u8 {
            let dummy = index_entry([0xe0 + bucket; 16], 0, 39);
            let entries = if bucket == 0 {
                rows.as_slice()
            } else {
                std::slice::from_ref(&dummy)
            };
            fs::write(
                data.join(format!("{bucket:02x}00000001.idx")),
                index(bucket, entries),
            )
            .unwrap();
        }
        Self {
            path,
            source_bytes,
            encoding_pages,
        }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn open(&self) -> local_store::LocalStore {
        local_store::LocalStore::open(
            &self.path,
            local_store::OpenOptions {
                locale: Locale::EnUs,
                index_layouts: [local_index::Layout::Contiguous; 16],
                root_layout: root::Layout::Header50893V2,
                limits: local_store::Limits::default(),
            },
        )
        .unwrap()
    }
}
impl Drop for SyntheticFixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }
}
fn hex(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn append(
    file: &mut fs::File,
    offset: &mut u64,
    rows: &mut Vec<[u8; 18]>,
    ekey: [u8; 16],
    body: &[u8],
) {
    file.write_all(&[0xa5; 30]).unwrap();
    file.write_all(b"BLTE\0\0\0\0N").unwrap();
    file.write_all(body).unwrap();
    let span = u32::try_from(body.len() + 39).unwrap();
    rows.push(index_entry(ekey, *offset, span));
    *offset += u64::from(span);
    assert!(*offset < 1 << 30);
}
fn encoding(routes: &[Route]) -> Vec<u8> {
    let mut pages = Vec::new();
    for routes in routes.chunks(PAGE_BYTES / ROUTE_BYTES) {
        let mut page = [0; PAGE_BYTES];
        for (ordinal, (key, size, ekey)) in routes.iter().enumerate() {
            let at = ordinal * ROUTE_BYTES;
            page[at..at + 2].copy_from_slice(&1u16.to_le_bytes());
            page[at + 2..at + 6].copy_from_slice(&size.to_be_bytes());
            page[at + 6..at + 22].copy_from_slice(key);
            page[at + 22..at + 38].copy_from_slice(ekey);
        }
        pages.push(page);
    }
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend(1u16.to_be_bytes());
    bytes.extend(1u16.to_be_bytes());
    bytes.extend((pages.len() as u32).to_be_bytes());
    bytes.extend(0u32.to_be_bytes());
    bytes.push(0);
    bytes.extend(0u32.to_be_bytes());
    for page in &pages {
        bytes.extend_from_slice(&page[6..22]);
        bytes.extend(md5::digest(page));
    }
    for page in pages {
        bytes.extend(page);
    }
    bytes
}
fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
    let mut row = [0; 18];
    row[..9].copy_from_slice(&key[..9]);
    row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
    row[14..].copy_from_slice(&size.to_le_bytes());
    row
}
fn index(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
    let hash = local_index::jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&hash.to_le_bytes());
    bytes.extend((rows.len() as u32 * 18).to_le_bytes());
    let mut checksum = (0, 0);
    for row in rows {
        checksum = local_index::jenkins::hashlittle2(row, checksum.0, checksum.1);
    }
    bytes.extend(checksum.0.to_le_bytes());
    for row in rows {
        bytes.extend(row);
    }
    bytes
}

#[test]
fn streaming_fixture_authenticates_multiple_encoding_pages_and_unique_full_ekeys() {
    let f = SyntheticFixture::new((1u32..=60).map(|id| (id, id.to_le_bytes().to_vec())));
    assert_eq!(f.encoding_pages, 3);
    assert_eq!(f.source_bytes, 240);
    assert!(f.path().is_absolute());
    let mut store = f.open();
    assert_eq!(store.stats().encoding_entries, 61);
    assert_eq!(store.stats().root_entries, 60);
    for id in 1u32..=60 {
        assert_eq!(store.read_file_data_id(id).unwrap().bytes, id.to_le_bytes());
    }
}
