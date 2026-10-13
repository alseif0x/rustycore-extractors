// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Only original synthetic keys and plaintext fixtures.
use super::{Error, Limits, decode, decode_with_keys, md5};
use crate::keys::{Keyring, xor};

const KEY: [u8; 16] = [0x42; 16];
const NAME: u64 = 0x0102030405060708;
const LIMITS: Limits = Limits {
    max_encoded_bytes: 4096,
    max_decoded_bytes: 4096,
    max_frames: 16,
};

fn keys(name: u64, key: [u8; 16]) -> Keyring {
    let hex = key
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Keyring::parse(format!("forever-tact-keys\t1\n{name:016x}\t{hex}\n").as_bytes()).unwrap()
}
fn single(frame: &[u8]) -> Vec<u8> {
    let mut encoded = b"BLTE\0\0\0\0".to_vec();
    encoded.extend_from_slice(frame);
    encoded
}
fn framed(frames: &[(&[u8], u32)]) -> Vec<u8> {
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(12_u32 + 24 * frames.len() as u32).to_be_bytes());
    encoded.push(0x0f);
    encoded.extend_from_slice(&(frames.len() as u32).to_be_bytes()[1..]);
    for (frame, size) in frames {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&size.to_be_bytes());
        encoded.extend_from_slice(&md5::digest(frame));
    }
    for (frame, _) in frames {
        encoded.extend_from_slice(frame);
    }
    encoded
}
fn encrypted(inner: &[u8], nonce_bytes: &[u8], index: u32, name: Option<u64>) -> Vec<u8> {
    let mut frame = vec![b'E', if name.is_some() { 8 } else { 0 }];
    if let Some(name) = name {
        frame.extend_from_slice(&name.to_le_bytes());
    }
    frame.push(nonce_bytes.len() as u8);
    frame.extend_from_slice(nonce_bytes);
    frame.push(b'S');
    let mut nonce = [0; 8];
    nonce[..nonce_bytes.len()].copy_from_slice(nonce_bytes);
    for (byte, index_byte) in nonce[..4].iter_mut().zip(index.to_le_bytes()) {
        *byte ^= index_byte;
    }
    let mut ciphertext = inner.to_vec();
    xor(&KEY, nonce, &mut ciphertext);
    frame.extend_from_slice(&ciphertext);
    frame
}
fn read(encoded: &[u8], output: &[u8]) -> Result<Vec<u8>, Error> {
    decode_with_keys(
        encoded,
        output.len() as u64,
        Some(md5::digest(output)),
        LIMITS,
        &keys(NAME, KEY),
    )
}
fn zabc() -> Vec<u8> {
    // Python stdlib zlib.compress(b"abc") independent zlib golden.
    vec![
        b'Z', 0x78, 0x9c, 0x4b, 0x4c, 0x4a, 0x06, 0x00, 0x02, 0x4d, 0x01, 0x27,
    ]
}

#[test]
fn encrypted_n_and_z_headerless_and_framed() {
    for inner in [b"Nabc".to_vec(), zabc()] {
        for nonce in [&[1, 2, 3, 4][..], &[1, 2, 3, 4, 5, 6, 7, 8][..]] {
            let frame = encrypted(&inner, nonce, 0, Some(NAME));
            for encoded in [single(&frame), framed(&[(&frame, 3)])] {
                assert_eq!(read(&encoded, b"abc").unwrap(), b"abc");
                assert_eq!(
                    decode(&encoded, 3, Some(md5::digest(b"abc")), LIMITS),
                    Err(Error::EncryptedUnsupported)
                );
            }
        }
    }
}
#[test]
fn empty_plaintext_and_block_boundaries() {
    for len in [0, 1, 63, 64, 65, 127, 128, 129, 1024] {
        let payload = (0..len).map(|i| i as u8).collect::<Vec<_>>();
        let mut inner = vec![b'N'];
        inner.extend_from_slice(&payload);
        assert_eq!(
            read(
                &single(&encrypted(&inner, &[0; 8], 0, Some(NAME))),
                &payload
            )
            .unwrap(),
            payload
        );
    }
}
#[test]
fn frame_index_xor_and_mixed_plain_frames() {
    let a = encrypted(b"Nabc", &[1, 2, 3, 4], 1, Some(NAME));
    let b = encrypted(&zabc(), &[5, 6, 7, 8, 9, 10, 11, 12], 3, Some(NAME));
    assert_eq!(
        read(
            &framed(&[(b"Nx", 1), (&a, 3), (b"N", 0), (&b, 3)]),
            b"xabcabc"
        )
        .unwrap(),
        b"xabcabc"
    );
    let wrong_index = encrypted(b"Nabc", &[1, 2, 3, 4], 0, Some(NAME));
    assert!(read(&framed(&[(b"Nx", 1), (&wrong_index, 3)]), b"xabc").is_err());
    // Exercise all four index bytes, beyond the compact complete-object fixture.
    let frame = encrypted(b"Nabc", &[1, 2, 3, 4, 5, 6, 7, 8], 0x01020304, Some(NAME));
    assert_eq!(
        super::decrypt_frame(&frame, 0x01020304, &keys(NAME, KEY)).unwrap(),
        b"Nabc"
    );
}
#[test]
fn zero_length_key_name_means_key_zero() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, None);
    assert_eq!(
        decode_with_keys(
            &single(&frame),
            3,
            Some(md5::digest(b"abc")),
            LIMITS,
            &keys(0, KEY)
        )
        .unwrap(),
        b"abc"
    );
    assert_eq!(read(&single(&frame), b"abc"), Err(Error::MissingKey));
}
#[test]
fn content_key_required_only_for_encrypted_frames() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, Some(NAME));
    assert_eq!(
        decode_with_keys(&single(&frame), 3, None, LIMITS, &keys(NAME, KEY)),
        Err(Error::UnverifiedEncryption)
    );
    assert_eq!(
        decode_with_keys(&single(b"Nabc"), 3, None, LIMITS, &keys(NAME, KEY)).unwrap(),
        b"abc"
    );
    assert_eq!(
        decode_with_keys(&single(&frame), 3, Some([0; 16]), LIMITS, &keys(NAME, KEY)),
        Err(Error::Checksum)
    );
}
#[test]
fn missing_and_wrong_keys_never_return_content() {
    let encoded = single(&encrypted(b"Nabc", &[0; 4], 0, Some(NAME)));
    assert_eq!(
        decode_with_keys(
            &encoded,
            3,
            Some(md5::digest(b"abc")),
            LIMITS,
            &keys(NAME + 1, KEY)
        ),
        Err(Error::MissingKey)
    );
    assert!(
        decode_with_keys(
            &encoded,
            3,
            Some(md5::digest(b"abc")),
            LIMITS,
            &keys(NAME, [0; 16])
        )
        .is_err()
    );
}
#[test]
fn ciphertext_hash_precedes_key_lookup_and_header_checks() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, Some(NAME));
    let mut encoded = framed(&[(&frame, 3)]);
    *encoded.last_mut().unwrap() ^= 1;
    assert_eq!(
        decode_with_keys(
            &encoded,
            3,
            Some(md5::digest(b"abc")),
            LIMITS,
            &keys(NAME + 1, KEY)
        ),
        Err(Error::Checksum)
    );
    assert_eq!(
        decode_with_keys(&encoded, 3, None, LIMITS, &keys(NAME, KEY)),
        Err(Error::Checksum)
    );
}
#[test]
fn all_truncated_encryption_headers_reject() {
    for nonce in [&[0; 4][..], &[0; 8][..]] {
        let frame = encrypted(b"Nabc", nonce, 0, Some(NAME));
        let header_len = 1 + 1 + 8 + 1 + nonce.len() + 1;
        for len in 1..=header_len {
            assert_eq!(
                read(&single(&frame[..len]), b"abc"),
                Err(Error::EncryptionHeader)
            );
        }
    }
}
#[test]
fn encryption_name_and_nonce_lengths_are_closed() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, Some(NAME));
    for length in 0..=255_u8 {
        if length != 0 && length != 8 {
            let mut malformed = frame.clone();
            malformed[1] = length;
            assert_eq!(
                read(&single(&malformed), b"abc"),
                Err(Error::EncryptionHeader)
            );
        }
        if length != 4 && length != 8 {
            let mut malformed = frame.clone();
            malformed[10] = length;
            assert_eq!(
                read(&single(&malformed), b"abc"),
                Err(Error::EncryptionHeader)
            );
        }
    }
}
#[test]
fn unsupported_cipher_is_explicit() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, Some(NAME));
    for cipher in 0..=255_u8 {
        if cipher != b'S' {
            let mut malformed = frame.clone();
            malformed[15] = cipher;
            assert_eq!(
                read(&single(&malformed), b"abc"),
                Err(Error::EncryptionUnsupported)
            );
        }
    }
}
#[test]
fn nested_wrappers_recursive_and_unknown_inner_modes_reject() {
    for (inner, error) in [
        (b"Eabc".as_slice(), Error::EncryptedUnsupported),
        (b"Fabc", Error::RecursiveUnsupported),
        (b"?abc", Error::UnknownMode),
    ] {
        assert_eq!(
            read(&single(&encrypted(inner, &[0; 4], 0, Some(NAME))), b"abc"),
            Err(error)
        );
    }
}
#[test]
fn encrypted_z_retains_exact_stream_checks() {
    let original = zabc();
    for len in 1..original.len() {
        assert!(
            read(
                &single(&encrypted(&original[..len], &[0; 4], 0, Some(NAME))),
                b"abc"
            )
            .is_err()
        );
    }
    let mut tail = original.clone();
    tail.push(0);
    assert_eq!(
        read(&single(&encrypted(&tail, &[0; 4], 0, Some(NAME))), b"abc"),
        Err(Error::Size)
    );
    let mut bad_adler = original;
    *bad_adler.last_mut().unwrap() ^= 1;
    assert_eq!(
        read(
            &single(&encrypted(&bad_adler, &[0; 4], 0, Some(NAME))),
            b"abc"
        ),
        Err(Error::Checksum)
    );
}
#[test]
fn declared_size_and_resource_limits_apply_before_decryption_output() {
    let frame = encrypted(b"Nabc", &[0; 4], 0, Some(NAME));
    let encoded = single(&frame);
    for size in [0, 2, 4] {
        assert_eq!(
            decode_with_keys(
                &encoded,
                size,
                Some(md5::digest(b"abc")),
                LIMITS,
                &keys(NAME, KEY)
            ),
            Err(Error::Size)
        );
    }
    for limits in [
        Limits {
            max_encoded_bytes: encoded.len() as u64 - 1,
            ..LIMITS
        },
        Limits {
            max_decoded_bytes: 2,
            ..LIMITS
        },
        Limits {
            max_frames: 0,
            ..LIMITS
        },
    ] {
        assert_eq!(
            decode_with_keys(
                &encoded,
                3,
                Some(md5::digest(b"abc")),
                limits,
                &keys(NAME, KEY)
            ),
            Err(Error::Limit)
        );
    }
    let framed = framed(&[(&frame, 3), (b"Nx", 1)]);
    assert_eq!(
        decode_with_keys(
            &framed,
            4,
            Some(md5::digest(b"abcx")),
            Limits {
                max_frames: 1,
                ..LIMITS
            },
            &keys(NAME, KEY)
        ),
        Err(Error::Limit)
    );
}
