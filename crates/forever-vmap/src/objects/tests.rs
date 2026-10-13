// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic object ADT/WDT streams only.
use super::*;
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b: Vec<_> = tag.iter().rev().copied().collect();
    b.extend_from_slice(&(body.len() as u32).to_le_bytes());
    b.extend_from_slice(body);
    b
}
fn mddf(id: u32, fdid: bool) -> [u8; 36] {
    let mut b = [0; 36];
    b[..4].copy_from_slice(&id.to_le_bytes());
    b[4..8].copy_from_slice(&17u32.to_le_bytes());
    b[32..34].copy_from_slice(&1024u16.to_le_bytes());
    b[34..36].copy_from_slice(&(if fdid { 0x40u16 } else { 0 }).to_le_bytes());
    b
}
fn modf(id: u32, fdid: bool) -> [u8; 64] {
    let mut b = [0; 64];
    b[..4].copy_from_slice(&id.to_le_bytes());
    b[4..8].copy_from_slice(&23u32.to_le_bytes());
    b[56..58].copy_from_slice(&(if fdid { 8u16 } else { 0 }).to_le_bytes());
    b
}
fn name<'a>(request: &ModelPlacement<'a>) -> &'a [u8] {
    match request.target() {
        Target::Name(n) => n,
        _ => panic!("expected name"),
    }
}
fn id(request: &ModelPlacement<'_>) -> u32 {
    match request.target() {
        Target::FileDataId(id) => id,
        _ => panic!("expected FDID"),
    }
}
#[test]
fn physical_names_append_and_ordinals_ignore_mmid_offsets() {
    let mut b = chunk(b"MMDX", b"A.m2\0");
    b.extend(chunk(b"MMDX", b"B.m2\0C.m2\0"));
    b.extend(chunk(b"MMID", &[0xff; 3]));
    let mut records = mddf(2, false).to_vec();
    records.extend(mddf(0, false));
    records.extend(mddf(1, false));
    b.extend(chunk(b"MDDF", &records));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(
        o.requests().iter().map(name).collect::<Vec<_>>(),
        [b"C.m2", b"A.m2", b"B.m2"]
    );
    assert_eq!(o.stats().names, 3);
    assert_eq!(o.stats().records, 3);
}
#[test]
fn requests_preserve_interleaved_source_order_and_borrowed_values() {
    let mut b = chunk(b"MDDF", &mddf(9, true));
    b.extend(chunk(b"MODF", &modf(3, true)));
    b.extend(chunk(b"MDDF", &mddf(7, true)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(o.requests().iter().map(id).collect::<Vec<_>>(), [9, 3, 7]);
    assert_eq!(
        o.requests()
            .iter()
            .map(ModelPlacement::kind)
            .collect::<Vec<_>>(),
        [Kind::M2, Kind::Wmo, Kind::M2]
    );
    match o.requests()[0].source() {
        Source::M2(s) => assert_eq!(s.unique_id(), 17),
        _ => panic!("kind"),
    };
    assert!(o.requests().iter().all(|r| !r.global_wmo()));
}
#[test]
fn fdid_zero_and_maximum_are_source_values_not_acquisition_decisions() {
    let mut b = chunk(b"MDDF", &mddf(0, true));
    b.extend(chunk(b"MODF", &modf(u32::MAX, true)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(id(&o.requests()[0]), 0);
    assert_eq!(id(&o.requests()[1]), u32::MAX);
}
#[test]
fn adt_nonempty_record_chunk_clears_only_its_own_names() {
    let mut b = chunk(b"MMDX", b"A.m2\0");
    b.extend(chunk(b"MWMO", b"B.wmo\0"));
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    b.extend(chunk(b"MODF", &modf(0, false)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(name(&o.requests()[1]), b"B.wmo");
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    assert_eq!(
        Objects::parse_adt(&b, Limits::default()).err(),
        Some(Error::NameIndex)
    );
}
#[test]
fn adt_fdid_only_records_also_clear_prior_names() {
    for (names, records) in [(b"MMDX", b"MDDF"), (b"MWMO", b"MODF")] {
        let mut b = chunk(names, b"unused\0");
        let direct = if records == b"MDDF" {
            mddf(2, true).to_vec()
        } else {
            modf(2, true).to_vec()
        };
        let indexed = if records == b"MDDF" {
            mddf(0, false).to_vec()
        } else {
            modf(0, false).to_vec()
        };
        b.extend(chunk(records, &direct));
        b.extend(chunk(records, &indexed));
        assert_eq!(
            Objects::parse_adt(&b, Limits::default()).err(),
            Some(Error::NameIndex)
        );
    }
}
#[test]
fn empty_record_chunks_do_not_clear_names() {
    let mut b = chunk(b"MMDX", b"A\0");
    b.extend(chunk(b"MDDF", &[]));
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(name(&o.requests()[0]), b"A");
    let mut b = chunk(b"MWMO", b"B\0");
    b.extend(chunk(b"MODF", &[]));
    b.extend(chunk(b"MODF", &modf(0, false)));
    assert!(Objects::parse_adt(&b, Limits::default()).is_ok());
}
#[test]
fn fresh_names_after_clear_start_at_ordinal_zero_and_prior_borrows_survive() {
    let mut b = chunk(b"MMDX", b"old\0");
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    b.extend(chunk(b"MMDX", b"new\0"));
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(name(&o.requests()[0]), b"old");
    assert_eq!(name(&o.requests()[1]), b"new");
}
#[test]
fn wdt_names_persist_across_modf_chunks_and_all_placements_are_global() {
    let mut b = chunk(b"MWMO", b"A.wmo\0");
    b.extend(chunk(b"MODF", &modf(0, false)));
    b.extend(chunk(b"MWMO", b"B.wmo\0"));
    let mut rows = modf(1, false).to_vec();
    rows.extend(modf(0, false));
    b.extend(chunk(b"MODF", &rows));
    let o = Objects::parse_wdt(&b, Limits::default()).unwrap();
    assert_eq!(
        o.requests().iter().map(name).collect::<Vec<_>>(),
        [b"A.wmo", b"B.wmo", b"A.wmo"]
    );
    assert!(o.requests().iter().all(|r| r.global_wmo()));
}
#[test]
fn wdt_ignores_mmdx_mddf_and_unknown_payloads_without_interpreting_them() {
    let mut b = chunk(b"MMDX", &[0xff]);
    b.extend(chunk(b"MDDF", &[0xff; 7]));
    b.extend(chunk(b"UNKN", &[0xff; 5]));
    b.extend(chunk(b"MODF", &modf(7, true)));
    let o = Objects::parse_wdt(&b, Limits::default()).unwrap();
    assert_eq!(o.requests().len(), 1);
    assert_eq!(id(&o.requests()[0]), 7);
}
#[test]
fn wdt_metadata_is_optional_exact_and_unique_without_tile_planning() {
    let mut b = chunk(b"MPHD", &[0; 32]);
    b.extend(chunk(b"MAIN", &[0xff; 32768]));
    b.extend(chunk(b"MAID", &[0xff; 131072]));
    b.extend(chunk(b"MODF", &modf(1, true)));
    assert_eq!(
        Objects::parse_wdt(&b, Limits::default())
            .unwrap()
            .requests()
            .len(),
        1
    );
    for (tag, size) in [(b"MPHD", 32), (b"MAIN", 32768), (b"MAID", 131072)] {
        let mut b = chunk(tag, &vec![0; size]);
        b.extend(chunk(tag, &vec![0; size]));
        assert_eq!(
            Objects::parse_wdt(&b, Limits::default()).err(),
            Some(Error::DuplicateMetadata)
        );
        assert_eq!(
            Objects::parse_wdt(&chunk(tag, &vec![0; size - 1]), Limits::default()).err(),
            Some(Error::Framing)
        );
    }
}
#[test]
fn empty_file_rejected_but_framed_empty_or_opaque_chunks_are_valid() {
    assert_eq!(
        Objects::parse_adt(&[], Limits::default()).err(),
        Some(Error::Framing)
    );
    for tag in [b"MDDF", b"MODF", b"MWMO", b"UNKN"] {
        assert!(
            Objects::parse_adt(&chunk(tag, &[]), Limits::default())
                .unwrap()
                .requests()
                .is_empty()
        );
    }
}
#[test]
fn every_final_frame_truncation_is_rejected_even_after_valid_records() {
    let prefix = chunk(b"MDDF", &mddf(7, true));
    let tail = chunk(b"MODF", &modf(9, true));
    for n in 1..tail.len() {
        let mut b = prefix.clone();
        b.extend(&tail[..n]);
        assert!(Objects::parse_adt(&b, Limits::default()).is_err());
    }
    let mut b = prefix;
    b.extend_from_slice(b"X");
    assert_eq!(
        Objects::parse_adt(&b, Limits::default()).err(),
        Some(Error::Framing)
    );
}
#[test]
fn record_remainders_are_rejected_including_source_floor_zero_cases() {
    for (tag, size) in [(b"MDDF", 36), (b"MODF", 64)] {
        for n in [1, size - 1, size + 1] {
            assert_eq!(
                Objects::parse_adt(&chunk(tag, &vec![0; n]), Limits::default()).err(),
                Some(Error::Framing)
            );
        }
    }
}
#[test]
fn invalid_or_future_name_ordinals_cannot_fall_back_to_offsets_or_fdid() {
    for id in [1, 5, u32::MAX] {
        let mut b = chunk(b"MMDX", b"A.m2\0");
        b.extend(chunk(b"MDDF", &mddf(id, false)));
        assert_eq!(
            Objects::parse_adt(&b, Limits::default()).err(),
            Some(Error::NameIndex)
        );
    }
    let mut b = chunk(b"MODF", &modf(0, false));
    b.extend(chunk(b"MWMO", b"later\0"));
    assert_eq!(
        Objects::parse_wdt(&b, Limits::default()).err(),
        Some(Error::NameIndex)
    );
}
#[test]
fn invalid_names_are_rejected_even_when_unused() {
    for names in [b"unterminated".as_slice(), b"\0", b"A\0\0", b"\xff\0"] {
        assert_eq!(
            Objects::parse_adt(&chunk(b"MMDX", names), Limits::default()).err(),
            Some(Error::Name)
        );
    }
    let mut longest = vec![b'A'; 1024];
    longest.push(0);
    assert!(Objects::parse_adt(&chunk(b"MMDX", &longest), Limits::default()).is_ok());
    longest.insert(0, b'A');
    assert_eq!(
        Objects::parse_adt(&chunk(b"MMDX", &longest), Limits::default()).err(),
        Some(Error::Limit)
    );
}
#[test]
fn full_source_float_validation_precedes_exposing_any_request() {
    let mut record = modf(8, true);
    record[20..24].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    let mut b = chunk(b"MDDF", &mddf(3, true));
    b.extend(chunk(b"MODF", &record));
    assert_eq!(
        Objects::parse_adt(&b, Limits::default()).err(),
        Some(Error::Source(placement::Error::NonFinite))
    );
}
#[test]
fn names_limit_is_cumulative_across_adt_clears() {
    let mut b = chunk(b"MMDX", b"A\0");
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    b.extend(chunk(b"MMDX", b"B\0"));
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    assert_eq!(
        Objects::parse_adt(
            &b,
            Limits {
                max_names: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(
        Objects::parse_adt(
            &b,
            Limits {
                max_names: 2,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}
#[test]
fn every_policy_can_only_reduce_and_zero_caps_do_not_allocate() {
    let b = chunk(b"MDDF", &mddf(1, true));
    for limits in [
        Limits {
            max_input_bytes: b.len() - 1,
            ..Limits::default()
        },
        Limits {
            max_chunks: 0,
            ..Limits::default()
        },
        Limits {
            max_records: 0,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(Objects::parse_adt(&b, limits).err(), Some(Error::Limit));
    }
    assert!(
        Objects::parse_adt(
            &b,
            Limits {
                max_input_bytes: b.len(),
                max_chunks: 1,
                max_records: 1,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_input_bytes: MAX_INPUT_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_chunks: MAX_CHUNKS + 1,
            ..Limits::default()
        },
        Limits {
            max_records: MAX_RECORDS + 1,
            ..Limits::default()
        },
        Limits {
            max_names: MAX_NAMES + 1,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: MAX_NAME_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            Objects::parse_adt(&[], limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}
#[test]
fn exact_memory_peak_covers_requests_and_scratch_names_then_releases_scratch() {
    let mut b = chunk(b"MMDX", b"A\0");
    b.extend(chunk(b"MDDF", &mddf(0, false)));
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    let stats = o.stats();
    assert_eq!(stats.resident_bytes, capacity(&o.requests).unwrap());
    assert!(stats.peak_bytes > stats.resident_bytes);
    assert!(
        Objects::parse_adt(
            &b,
            Limits {
                max_memory_bytes: stats.peak_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        Objects::parse_adt(
            &b,
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn geometric_capacity_growth_preserves_large_ordered_record_batch() {
    let mut b = Vec::new();
    for id in 0..64 {
        b.extend(chunk(b"MDDF", &mddf(id, true)));
    }
    let o = Objects::parse_adt(&b, Limits::default()).unwrap();
    assert_eq!(
        o.requests().iter().map(id).collect::<Vec<_>>(),
        (0..64).collect::<Vec<_>>()
    );
    assert_eq!(o.stats().records, 64);
    assert!(o.stats().peak_bytes <= MAX_MEMORY_BYTES);
}
