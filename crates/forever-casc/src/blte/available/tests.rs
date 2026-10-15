use super::*;

const LIMITS: Limits = Limits {
    max_encoded_bytes: 4096,
    max_decoded_bytes: 4096,
    max_frames: 16,
};
fn encrypted() -> Vec<u8> {
    let mut f = vec![b'E', 8];
    f.extend([0x11; 8]); // Authored fake identifier, never consulted.
    f.push(4);
    f.extend([0; 4]);
    f.extend([b'S', 0x22]);
    f
}
fn framed(frames: &[(&[u8], u32)]) -> (Vec<u8>, [u8; 16], u64) {
    let header = 12 + frames.len() * 24;
    let mut b = b"BLTE".to_vec();
    b.extend((header as u32).to_be_bytes());
    b.extend([15, 0, 0, frames.len() as u8]);
    for (f, n) in frames {
        b.extend((f.len() as u32).to_be_bytes());
        b.extend(n.to_be_bytes());
        b.extend(md5::digest(f));
    }
    let ekey = md5::digest(&b);
    for (f, _) in frames {
        b.extend(*f);
    }
    (b, ekey, frames.iter().map(|(_, n)| u64::from(*n)).sum())
}
fn rejected(result: Result<AvailableBytes, Error>, expected: Error) {
    assert!(matches!(result, Err(e) if e == expected));
}

#[test]
fn available_ranges_never_expose_encrypted_zeros() {
    let e = encrypted();
    let (b, key, n) = framed(&[(b"Nabc", 3), (&e, 7), (b"Nxyz", 3)]);
    let a = decode(&b, n, key, [0; 16], LIMITS).unwrap();
    assert_eq!((a.declared_bytes(), a.available_bytes()), (13, 6));
    assert!(!a.full_ckey_verified());
    assert_eq!(a.range(0, 3), Ok(&b"abc"[..]));
    assert_eq!(a.range(10, 3), Ok(&b"xyz"[..]));
    assert_eq!(a.range(2, 9), Err(RangeError::Unavailable));
    assert_eq!(a.range(3, 0), Err(RangeError::Empty));
    assert_eq!(a.range(u64::MAX, 2), Err(RangeError::Bounds));
}
#[test]
fn adjacent_available_frames_allow_whole_cross_boundary_span() {
    let (b, k, n) = framed(&[(b"Nabc", 3), (b"Ndef", 3)]);
    let a = decode(&b, n, k, md5::digest(b"abcdef"), LIMITS).unwrap();
    assert!(a.full_ckey_verified());
    assert_eq!(a.range(2, 3), Ok(&b"cde"[..]));
}
#[test]
fn full_content_key_is_required_when_no_holes_remain() {
    let (b, k, n) = framed(&[(b"Nabc", 3)]);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Checksum);
}
#[test]
fn header_ekey_authentication_is_not_prefix_matching() {
    let (b, mut k, n) = framed(&[(b"Nabc", 3)]);
    k[15] ^= 1;
    rejected(
        decode(&b, n, k, md5::digest(b"abc"), LIMITS),
        Error::Checksum,
    );
}
#[test]
fn descriptor_tamper_rejects_before_using_changed_sizes() {
    let e = encrypted();
    let (mut b, k, n) = framed(&[(&e, 4)]);
    b[19] ^= 1;
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Checksum);
}
#[test]
fn encrypted_payload_tamper_is_not_hidden_by_omission() {
    let e = encrypted();
    let (mut b, k, n) = framed(&[(&e, 4)]);
    *b.last_mut().unwrap() ^= 1;
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Checksum);
}
#[test]
fn exact_eof_and_declared_decoded_sum_are_mandatory() {
    let e = encrypted();
    let (mut b, k, n) = framed(&[(&e, 4)]);
    rejected(decode(&b, n + 1, k, [0; 16], LIMITS), Error::Size);
    b.push(0);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Size);
}
#[test]
fn hole_bytes_are_charged_against_decoded_limit() {
    let e = encrypted();
    let (b, k, n) = framed(&[(&e, 4097)]);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Limit);
}
#[test]
fn frame_and_encoded_limits_are_not_relaxed() {
    let (b, k, n) = framed(&[(b"Nabc", 3)]);
    rejected(
        decode(
            &b,
            n,
            k,
            [0; 16],
            Limits {
                max_frames: 0,
                ..LIMITS
            },
        ),
        Error::Limit,
    );
    rejected(
        decode(
            &b,
            n,
            k,
            [0; 16],
            Limits {
                max_encoded_bytes: 8,
                ..LIMITS
            },
        ),
        Error::Limit,
    );
}
#[test]
fn malformed_encryption_and_unknown_modes_are_fatal() {
    let (b, k, n) = framed(&[(b"E", 4)]);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::EncryptionHeader);
    let (b, k, n) = framed(&[(b"Qabc", 3)]);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::UnknownMode);
    let (b, k, n) = framed(&[(b"Fabc", 3)]);
    rejected(
        decode(&b, n, k, [0; 16], LIMITS),
        Error::RecursiveUnsupported,
    );
}
#[test]
fn headerless_ekey_hashes_entire_object() {
    let mut b = b"BLTE\0\0\0\0Nabc".to_vec();
    let k = md5::digest(&b);
    let a = decode(&b, 3, k, md5::digest(b"abc"), LIMITS).unwrap();
    assert!(a.full_ckey_verified());
    *b.last_mut().unwrap() ^= 1;
    rejected(
        decode(&b, 3, k, md5::digest(b"abc"), LIMITS),
        Error::Checksum,
    );
}
#[test]
fn exact_zlib_decode_with_encrypted_sibling() {
    let mut z = vec![b'Z'];
    // Existing independently authored Python-zlib golden for b"abc";
    // no encoder feature or dependency is added to the production crate.
    z.extend([
        0x78, 0x9c, 0x4b, 0x4c, 0x4a, 0x06, 0x00, 0x02, 0x4d, 0x01, 0x27,
    ]);
    let e = encrypted();
    let (b, k, n) = framed(&[(&z, 3), (&e, 4)]);
    assert_eq!(
        decode(&b, n, k, [0; 16], LIMITS).unwrap().range(0, 3),
        Ok(&b"abc"[..])
    );
    let (b, k, n) = framed(&[(&z, 4), (&e, 4)]);
    rejected(decode(&b, n, k, [0; 16], LIMITS), Error::Size);
}
