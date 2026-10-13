// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: externally retained synthetic MMap corpus only.
use super::*;
use std::{env, fs, path::PathBuf};

#[test]
#[ignore = "requires explicit external synthetic inputs/output directories"]
fn write_mmap_corpus() {
    let input = PathBuf::from(env::var_os("RCE_MMAP_INPUT").expect("input"));
    let root = PathBuf::from(env::var_os("RCE_MMAP_OUTPUT").expect("output"));
    fs::create_dir(&root).unwrap();
    let mut paths = fs::read_dir(input)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for p in paths {
        let data = fs::read(&p).unwrap();
        assert!(data.len() >= 40);
        // Authored request: map ID, seven param words, count, liquid flag;
        // source records, payload size, and a previous exact Detour payload.
        let count = word(&data, 32) as usize;
        let stop = 40 + 44 * count;
        let params = Params {
            origin: [float(&data, 4), float(&data, 8), float(&data, 12)],
            tile_width: float(&data, 16),
            tile_height: float(&data, 20),
            max_tiles: word(&data, 24) as i32,
            max_polys: word(&data, 28) as i32,
        };
        let records = data[40..stop]
            .as_chunks::<44>()
            .0
            .iter()
            .map(|b| source(b))
            .collect::<Vec<_>>();
        let size = word(&data, stop) as usize;
        assert_eq!(data.len(), stop + 4 + size);
        let dir = root.join(p.file_stem().unwrap());
        fs::create_dir(&dir).unwrap();
        let b = encode_global(word(&data, 0), params, &records, Limits::default()).unwrap();
        let v = GlobalView::parse(&b, word(&data, 0), Limits::default()).unwrap();
        assert_eq!(v.bytes(), b);
        fs::write(dir.join("global.bin"), b).unwrap();
        if size > 0 {
            let b =
                encode_payload(&data[stop + 4..], word(&data, 36) != 0, Limits::default()).unwrap();
            let v = TileView::parse(&b, Limits::default()).unwrap();
            assert_eq!(v.payload(), &data[stop + 4..]);
            fs::write(dir.join("tile.bin"), b).unwrap();
        }
    }
}
