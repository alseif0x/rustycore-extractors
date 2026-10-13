// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-11: safe Rust port of the byte-oriented lookup3 algorithm;
// no alignment tricks, pointer casts, out-of-bounds reads or C dependency.
// Source: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// dep/CascLib/src/jenkins/lookup3.c. Existing issue #1 native source closure.
//
// Original notice retained:
// lookup3.c, by Bob Jenkins, May 2006, Public Domain.
// These are functions for producing 32-bit hashes for hash table lookup.
// You can use this free for any purpose. It's in the public domain.
// It has no warranty.

pub(crate) fn hashlittle(bytes: &[u8], seed: u32) -> u32 {
    hashlittle2(bytes, seed, 0).0
}

/// Return (pc, pb), in the same order as the reference's two output pointers.
pub(crate) fn hashlittle2(bytes: &[u8], primary: u32, secondary: u32) -> (u32, u32) {
    let initial = 0xdeadbeef_u32
        .wrapping_add(bytes.len() as u32)
        .wrapping_add(primary);
    let [mut a, mut b, mut c] = [initial, initial, initial.wrapping_add(secondary)];
    let mut remaining = bytes;
    while remaining.len() > 12 {
        a = a.wrapping_add(word(&remaining[..4]));
        b = b.wrapping_add(word(&remaining[4..8]));
        c = c.wrapping_add(word(&remaining[8..12]));
        a = a.wrapping_sub(c) ^ c.rotate_left(4);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a) ^ a.rotate_left(6);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b) ^ b.rotate_left(8);
        b = b.wrapping_add(a);
        a = a.wrapping_sub(c) ^ c.rotate_left(16);
        c = c.wrapping_add(b);
        b = b.wrapping_sub(a) ^ a.rotate_left(19);
        a = a.wrapping_add(c);
        c = c.wrapping_sub(b) ^ b.rotate_left(4);
        b = b.wrapping_add(a);
        remaining = &remaining[12..];
    }
    if remaining.is_empty() {
        return (c, b);
    }
    let mut tail = [0_u8; 12];
    tail[..remaining.len()].copy_from_slice(remaining);
    a = a.wrapping_add(word(&tail[..4]));
    b = b.wrapping_add(word(&tail[4..8]));
    c = c.wrapping_add(word(&tail[8..12]));
    c = (c ^ b).wrapping_sub(b.rotate_left(14));
    a = (a ^ c).wrapping_sub(c.rotate_left(11));
    b = (b ^ a).wrapping_sub(a.rotate_left(25));
    c = (c ^ b).wrapping_sub(b.rotate_left(16));
    a = (a ^ c).wrapping_sub(c.rotate_left(4));
    b = (b ^ a).wrapping_sub(a.rotate_left(14));
    c = (c ^ b).wrapping_sub(b.rotate_left(24));
    (c, b)
}

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests {
    use super::{hashlittle, hashlittle2};

    #[test]
    fn pinned_lookup3_driver5_vectors() {
        for (input, primary, secondary, expected) in [
            (&b""[..], 0, 0, (0xdeadbeef, 0xdeadbeef)),
            (&b""[..], 0, 0xdeadbeef, (0xbd5b7dde, 0xdeadbeef)),
            (&b""[..], 0xdeadbeef, 0xdeadbeef, (0x9c093ccd, 0xbd5b7dde)),
            (
                &b"Four score and seven years ago"[..],
                0,
                0,
                (0x17770551, 0xce7226e6),
            ),
            (
                &b"Four score and seven years ago"[..],
                0,
                1,
                (0xe3607cae, 0xbd371de4),
            ),
            (
                &b"Four score and seven years ago"[..],
                1,
                0,
                (0xcd628161, 0x6cbea4b3),
            ),
        ] {
            assert_eq!(hashlittle2(input, primary, secondary), expected);
        }
        assert_eq!(hashlittle(b"Four score and seven years ago", 0), 0x17770551);
        assert_eq!(hashlittle(b"Four score and seven years ago", 1), 0xcd628161);
    }

    #[test]
    fn arbitrary_alignment_and_exact_slices_do_not_change_hashes() {
        let mut bytes = [0_u8; 300];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = index as u8;
        }
        for size in 0..=255 {
            for offset in 0..4 {
                let mut aligned = vec![0; offset];
                aligned.extend_from_slice(&bytes[..size]);
                assert_eq!(
                    hashlittle2(&aligned[offset..], 31, 47),
                    hashlittle2(&bytes[..size], 31, 47)
                );
            }
        }
    }
}
