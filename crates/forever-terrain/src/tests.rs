// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic framing/WDT fixtures only.
use crate::{
    wdt::{FILE_ID_REFERENCES, MAP_SIZE, TILE_COUNT},
    *,
};

fn frame(tag: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = tag.into_iter().rev().collect::<Vec<_>>();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn fixture(flags: Option<u32>, has_maid: bool) -> Vec<u8> {
    let mut bytes = frame(*b"MVER", &18u32.to_le_bytes());
    if let Some(flags) = flags {
        let mut mphd = vec![0; 32];
        for i in 0..8 {
            put(&mut mphd, i * 4, if i == 0 { flags } else { i as u32 });
        }
        bytes.extend(frame(*b"MPHD", &mphd));
    }
    bytes.extend(frame(*b"MAIN", &vec![0; TILE_COUNT * 8]));
    if has_maid {
        bytes.extend(frame(*b"MAID", &vec![0; TILE_COUNT * 32]));
    }
    bytes
}
fn offset(bytes: &[u8], tag: [u8; 4]) -> usize {
    Chunks::parse(bytes, Limits::default())
        .unwrap()
        .unique(tag)
        .unwrap()
        .unwrap()
        .offset()
        + 8
}
fn select(bytes: &mut [u8], x: usize, y: usize, flags: u32, data1: u32, ids: [u32; 8]) {
    let base = offset(bytes, *b"MAIN") + (y * MAP_SIZE + x) * 8;
    put(bytes, base, flags);
    put(bytes, base + 4, data1);
    let base = offset(bytes, *b"MAID") + (y * MAP_SIZE + x) * 32;
    for (i, id) in ids.into_iter().enumerate() {
        put(bytes, base + i * 4, id);
    }
}
fn parse(bytes: &[u8]) -> Result<Wdt<'_>, Error> {
    Wdt::parse(bytes, Limits::default())
}

#[test]
fn literal_mver_golden_and_fourcc_reversal_are_exact() {
    let golden = b"REVM\x04\x00\x00\x00\x12\x00\x00\x00";
    assert_eq!(frame(*b"MVER", &18u32.to_le_bytes()), golden);
    let chunks = Chunks::parse(golden, Limits::default()).unwrap();
    let chunk = chunks.unique(*b"MVER").unwrap().unwrap();
    assert_eq!(chunk.raw_fourcc(), *b"REVM");
    assert_eq!(chunk.fourcc(), *b"MVER");
    assert_eq!(chunk.offset(), 0);
    assert_eq!(chunk.range(), 0..12);
    assert_eq!(chunk.payload(), &18u32.to_le_bytes());
    assert_eq!(chunks.bytes().as_ptr(), golden.as_ptr());
    assert_eq!(chunk.payload().as_ptr(), golden[8..].as_ptr());
}

#[test]
fn chunks_preserve_unknown_nontext_tags_order_and_empty_frames() {
    let tag = [0, 0xff, b'x', b'z'];
    let mut bytes = frame(tag, b"opaque\0\xff");
    bytes.extend(frame(tag, b""));
    let original = bytes.clone();
    let chunks = Chunks::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(chunks.all().len(), 2);
    assert_eq!(chunks.all()[0].fourcc(), tag);
    assert_eq!(chunks.all()[0].payload(), b"opaque\0\xff");
    assert_eq!(chunks.all()[1].payload(), b"");
    assert_eq!(chunks.unique(tag).err(), Some(Error::DuplicateChunk));
    assert!(chunks.unique(*b"NONE").unwrap().is_none());
    assert_eq!(bytes, original);
}

#[test]
fn chunk_empty_file_and_zero_sized_frames_make_bounded_progress() {
    assert!(
        Chunks::parse(
            &[],
            Limits {
                max_file_bytes: 0,
                max_chunks: 0
            }
        )
        .unwrap()
        .all()
        .is_empty()
    );
    let bytes = frame(*b"ZERO", &[]).repeat(3);
    let chunks = Chunks::parse(
        &bytes,
        Limits {
            max_file_bytes: 24,
            max_chunks: 3,
        },
    )
    .unwrap();
    assert_eq!(
        chunks.all().iter().map(|c| c.offset()).collect::<Vec<_>>(),
        [0, 8, 16]
    );
    assert_eq!(
        Chunks::parse(
            &bytes,
            Limits {
                max_file_bytes: 24,
                max_chunks: 2
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn chunk_truncation_and_trailing_bytes_never_return_a_prefix() {
    let bytes = frame(*b"TEST", b"abcdef");
    for length in 1..bytes.len() {
        assert_eq!(
            Chunks::parse(&bytes[..length], Limits::default()).err(),
            Some(Error::Truncated)
        );
    }
    for length in 1..8 {
        let mut trailing = bytes.clone();
        trailing.resize(trailing.len() + length, 0);
        assert_eq!(
            Chunks::parse(&trailing, Limits::default()).err(),
            Some(Error::Truncated)
        );
    }
}

#[test]
fn unknown_chunks_with_bad_extents_and_huge_lengths_are_rejected() {
    for size in [1u32, u32::MAX] {
        let mut bytes = frame(*b"UNKN", &[]);
        put(&mut bytes, 4, size);
        let error = Chunks::parse(&bytes, Limits::default()).err().unwrap();
        // The u32::MAX extent addition can be a policy overflow on 32-bit.
        assert!(matches!(error, Error::Truncated | Error::Limit));
    }
}

#[test]
fn chunk_exact_byte_and_count_limits_have_no_implicit_extra_room() {
    let bytes = frame(*b"DATA", b"abcd");
    assert!(
        Chunks::parse(
            &bytes,
            Limits {
                max_file_bytes: 12,
                max_chunks: 1
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_file_bytes: 11,
            max_chunks: 1,
        },
        Limits {
            max_file_bytes: 12,
            max_chunks: 0,
        },
    ] {
        assert_eq!(Chunks::parse(&bytes, limits).err(), Some(Error::Limit));
    }
    assert_eq!(
        Limits::default(),
        Limits {
            max_file_bytes: 16 * 1024 * 1024,
            max_chunks: 4096
        }
    );
    assert_eq!(
        Chunks::parse(&vec![0; 16 * 1024 * 1024 + 1], Limits::default()).err(),
        Some(Error::Limit)
    );
}

#[test]
fn complete_modern_fixture_has_exact_reviewed_lengths() {
    let bytes = fixture(Some(FILE_ID_REFERENCES), true);
    assert_eq!(bytes.len(), 163908);
    let wdt = parse(&bytes).unwrap();
    assert_eq!(
        wdt.chunks()
            .all()
            .iter()
            .map(|c| c.payload().len())
            .collect::<Vec<_>>(),
        [4, 32, 32768, 131072]
    );
    assert_eq!(wdt.tiles().len(), 4096);
    assert_eq!(wdt.modern_root_adts().unwrap().count(), 0);
    assert_eq!(
        wdt.mphd(),
        Some(Mphd {
            flags: 0x200,
            lgt_file_data_id: 1,
            occ_file_data_id: 2,
            fogs_file_data_id: 3,
            mpv_file_data_id: 4,
            tex_file_data_id: 5,
            wdl_file_data_id: 6,
            pd4_file_data_id: 7
        })
    );
    assert!(wdt.has_maid());
}

#[test]
fn selected_tiles_use_y_then_x_without_losing_any_fields() {
    let mut bytes = fixture(Some(FILE_ID_REFERENCES | 0x80000000), true);
    select(
        &mut bytes,
        7,
        2,
        0x80000001,
        0xdeadbeef,
        [11, 12, 13, 14, 15, 16, 17, 18],
    );
    select(&mut bytes, 2, 7, 1, 99, [21, 22, 23, 24, 25, 26, 27, 28]);
    let wdt = parse(&bytes).unwrap();
    let tile = wdt.tile(7, 2).unwrap();
    assert_eq!(
        (tile.x, tile.y, tile.flags, tile.data1),
        (7, 2, 0x80000001, 0xdeadbeef)
    );
    assert_eq!(
        tile.file_ids,
        Some(TileFileIds {
            root_adt: 11,
            obj0_adt: 12,
            obj1_adt: 13,
            tex0_adt: 14,
            lod_adt: 15,
            map_texture: 16,
            map_texture_n: 17,
            minimap_texture: 18
        })
    );
    assert_eq!(wdt.mphd().unwrap().flags, 0x80000200);
    assert_eq!(
        wdt.modern_root_adts().unwrap().collect::<Vec<_>>(),
        [
            TileReference {
                x: 7,
                y: 2,
                file_data_id: 11
            },
            TileReference {
                x: 2,
                y: 7,
                file_data_id: 21
            }
        ]
    );
}

#[test]
fn zero_duplicate_and_maximum_file_ids_are_not_filtered_or_deduplicated() {
    let mut bytes = fixture(Some(FILE_ID_REFERENCES), true);
    for (x, id) in [(0, 0), (1, 7), (2, 7), (3, u32::MAX)] {
        select(&mut bytes, x, 0, 1, 0, [id; 8]);
    }
    let wdt = parse(&bytes).unwrap();
    assert_eq!(
        wdt.modern_root_adts()
            .unwrap()
            .map(|t| t.file_data_id)
            .collect::<Vec<_>>(),
        [0, 7, 7, u32::MAX]
    );
}

#[test]
fn inactive_tiles_keep_opaque_words_and_ids_without_becoming_dependencies() {
    let mut bytes = fixture(Some(FILE_ID_REFERENCES), true);
    select(&mut bytes, 9, 10, 0xfffffffe, 777, [44; 8]);
    let wdt = parse(&bytes).unwrap();
    let tile = wdt.tile(9, 10).unwrap();
    assert!(!tile.present());
    assert_eq!(tile.data1, 777);
    assert_eq!(tile.file_ids.unwrap().root_adt, 44);
    assert_eq!(wdt.modern_root_adts().unwrap().count(), 0);
}

#[test]
fn coordinate_bounds_and_last_cell_are_exact() {
    let mut bytes = fixture(Some(FILE_ID_REFERENCES), true);
    select(&mut bytes, 63, 63, 1, 55, [99; 8]);
    let wdt = parse(&bytes).unwrap();
    assert_eq!(wdt.tile(63, 63).unwrap().file_ids.unwrap().root_adt, 99);
    for (x, y) in [(64, 0), (0, 64), (usize::MAX, 1), (1, usize::MAX)] {
        assert!(wdt.tile(x, y).is_none());
    }
    assert_eq!(wdt.tiles().last().unwrap().x, 63);
    assert_eq!(wdt.tiles().last().unwrap().y, 63);
}

#[test]
fn name_reference_mode_is_preserved_and_never_falls_back_to_maid() {
    for flags in [None, Some(0), Some(0x800001ff)] {
        let mut bytes = fixture(flags, true);
        select(&mut bytes, 4, 5, 1, 0, [123; 8]);
        let wdt = parse(&bytes).unwrap();
        assert!(wdt.tile(4, 5).unwrap().present());
        assert_eq!(
            wdt.modern_root_adts().err(),
            Some(Error::UnsupportedReferenceMode)
        );
    }
    let bytes = fixture(None, false);
    let wdt = parse(&bytes).unwrap();
    assert!(!wdt.has_maid());
    assert!(wdt.tile(0, 0).unwrap().file_ids.is_none());
}

#[test]
fn maid_is_required_exactly_when_a_modern_present_tile_needs_it() {
    let mut bytes = fixture(Some(FILE_ID_REFERENCES), false);
    assert_eq!(
        parse(&bytes).unwrap().modern_root_adts().unwrap().count(),
        0
    );
    let pos = offset(&bytes, *b"MAIN");
    put(&mut bytes, pos, 1);
    assert_eq!(parse(&bytes).err(), Some(Error::MissingTileFileIds));
    let pos = offset(&bytes, *b"MPHD");
    put(&mut bytes, pos, 0);
    let wdt = parse(&bytes).unwrap();
    assert!(wdt.tile(0, 0).unwrap().present());
    assert_eq!(
        wdt.modern_root_adts().err(),
        Some(Error::UnsupportedReferenceMode)
    );
}

#[test]
fn required_chunks_cannot_be_manufactured_by_absent_or_embedded_frames() {
    assert_eq!(parse(&[]).err(), Some(Error::MissingChunk));
    assert_eq!(
        parse(&frame(*b"MVER", &18u32.to_le_bytes())).err(),
        Some(Error::MissingChunk)
    );
    let bytes = frame(*b"UNKN", &fixture(Some(FILE_ID_REFERENCES), true));
    assert_eq!(parse(&bytes).err(), Some(Error::MissingChunk));
    let mut bytes = fixture(None, false);
    bytes[..4].copy_from_slice(b"MVER");
    assert_eq!(parse(&bytes).err(), Some(Error::MissingChunk));
}

#[test]
fn no_byte_resynchronization_accepts_prefix_garbage() {
    let mut bytes = vec![0xff];
    bytes.extend(fixture(Some(FILE_ID_REFERENCES), true));
    assert!(parse(&bytes).is_err());
}

#[test]
fn known_chunks_are_unique_including_unused_optional_ones() {
    for tag in [*b"MVER", *b"MAIN", *b"MPHD", *b"MAID"] {
        let mut bytes = fixture(Some(0), true);
        let extra = {
            let chunks = Chunks::parse(&bytes, Limits::default()).unwrap();
            frame(tag, chunks.unique(tag).unwrap().unwrap().payload())
        };
        bytes.extend(extra);
        assert_eq!(parse(&bytes).err(), Some(Error::DuplicateChunk));
    }
}

#[test]
fn known_payload_sizes_are_exact_even_when_the_chunk_is_unused() {
    for tag in [*b"MVER", *b"MAIN", *b"MPHD", *b"MAID"] {
        for delta in [-1isize, 1] {
            let original = fixture(Some(0), true);
            let chunks = Chunks::parse(&original, Limits::default()).unwrap();
            let mut bytes = Vec::new();
            for chunk in chunks.all() {
                let mut payload = chunk.payload().to_vec();
                if chunk.fourcc() == tag {
                    if delta < 0 {
                        payload.pop();
                    } else {
                        payload.push(0);
                    }
                }
                bytes.extend(frame(chunk.fourcc(), &payload));
            }
            assert_eq!(parse(&bytes).err(), Some(Error::InvalidChunkSize));
        }
    }
}

#[test]
fn unsupported_versions_are_not_reinterpreted_as_another_format() {
    for version in [0u32, 17, 19, u32::MAX] {
        let mut bytes = fixture(Some(0x200), true);
        put(&mut bytes, 8, version);
        assert_eq!(parse(&bytes).err(), Some(Error::UnsupportedVersion));
    }
}

#[test]
fn arbitrary_chunk_order_unknown_duplicates_and_raw_bytes_are_preserved() {
    let original = fixture(Some(0x200), true);
    let chunks = Chunks::parse(&original, Limits::default()).unwrap();
    let opaque = frame(*b"UNKN", &frame(*b"MAIN", b"fake"));
    let mut bytes = opaque.clone();
    for chunk in chunks.all().iter().rev() {
        bytes.extend(frame(chunk.fourcc(), chunk.payload()));
    }
    bytes.extend(opaque);
    let before = bytes.clone();
    let wdt = parse(&bytes).unwrap();
    assert_eq!(wdt.chunks().all().len(), 6);
    assert_eq!(wdt.chunks().all()[0].payload(), frame(*b"MAIN", b"fake"));
    assert_eq!(wdt.modern_root_adts().unwrap().count(), 0);
    assert_eq!(bytes, before);
}

#[test]
fn malformed_tail_prevents_any_wdt_projection() {
    let original = fixture(Some(0x200), true);
    for length in 1..8 {
        let mut bytes = original.clone();
        bytes.resize(bytes.len() + length, 0);
        assert_eq!(parse(&bytes).err(), Some(Error::Truncated));
    }
    let mut bytes = original;
    bytes.extend(frame(*b"UNKN", b"abcd"));
    bytes.pop();
    assert_eq!(parse(&bytes).err(), Some(Error::Truncated));
}

#[test]
fn wdt_exact_policy_boundaries_are_enforced_before_typed_projection() {
    let bytes = fixture(Some(0x200), true);
    assert!(
        Wdt::parse(
            &bytes,
            Limits {
                max_file_bytes: bytes.len(),
                max_chunks: 4
            }
        )
        .is_ok()
    );
    assert_eq!(
        Wdt::parse(
            &bytes,
            Limits {
                max_file_bytes: bytes.len() - 1,
                max_chunks: 4
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Wdt::parse(
            &bytes,
            Limits {
                max_file_bytes: bytes.len(),
                max_chunks: 3
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn every_error_uses_a_fixed_diagnostic_without_input_bytes() {
    for (error, code) in [
        (Error::Limit, "terrain_policy_limit"),
        (Error::Truncated, "terrain_chunk_truncated"),
        (Error::DuplicateChunk, "terrain_duplicate_chunk"),
        (Error::MissingChunk, "terrain_missing_chunk"),
        (Error::InvalidChunkSize, "terrain_invalid_chunk_size"),
        (Error::UnsupportedVersion, "terrain_unsupported_version"),
        (
            Error::UnsupportedReferenceMode,
            "terrain_unsupported_reference_mode",
        ),
        (Error::MissingTileFileIds, "terrain_missing_tile_file_ids"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
