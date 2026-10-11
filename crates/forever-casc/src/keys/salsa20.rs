// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Salsa20/20 implementation from algorithm facts.
//! Algorithm: D. J. Bernstein, Salsa20 specification (2005-04-27),
//! https://cr.yp.to/snuffle/spec.pdf, sections 3..10; no source implementation
//! is imported. Only the defined 16-byte-key expansion is exposed here.

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn quarter(words: &mut [u32; 16], positions: [usize; 4]) {
    let [a, b, c, d] = positions;
    words[b] ^= words[a].wrapping_add(words[d]).rotate_left(7);
    words[c] ^= words[b].wrapping_add(words[a]).rotate_left(9);
    words[d] ^= words[c].wrapping_add(words[b]).rotate_left(13);
    words[a] ^= words[d].wrapping_add(words[c]).rotate_left(18);
}
fn block(key: &[u8; 16], nonce: [u8; 8], counter: u64) -> [u8; 64] {
    let constants = b"expand 16-byte k";
    let mut initial = [0u32; 16];
    for (index, position) in [0, 5, 10, 15].into_iter().enumerate() {
        initial[position] = word(&constants[index * 4..]);
    }
    for index in 0..4 {
        let key_word = word(&key[index * 4..]);
        initial[index + 1] = key_word;
        initial[index + 11] = key_word;
    }
    initial[6] = word(&nonce);
    initial[7] = word(&nonce[4..]);
    initial[8] = counter as u32;
    initial[9] = (counter >> 32) as u32;
    let mut mixed = initial;
    // Four disjoint columns, then four disjoint cyclically rotated rows.
    const POSITIONS: [[usize; 4]; 8] = [
        [0, 4, 8, 12],
        [5, 9, 13, 1],
        [10, 14, 2, 6],
        [15, 3, 7, 11],
        [0, 1, 2, 3],
        [5, 6, 7, 4],
        [10, 11, 8, 9],
        [15, 12, 13, 14],
    ];
    for _ in 0..10 {
        for positions in POSITIONS {
            quarter(&mut mixed, positions);
        }
    }
    let mut output = [0; 64];
    for index in 0..16 {
        output[index * 4..index * 4 + 4]
            .copy_from_slice(&mixed[index].wrapping_add(initial[index]).to_le_bytes());
    }
    output
}
pub(super) fn xor(key: &[u8; 16], nonce: [u8; 8], bytes: &mut [u8]) {
    // A Rust slice cannot approach 2^64 blocks, so the enumerated counter cannot
    // wrap. Counter words are serialized separately by block, including carry.
    for (counter, chunk) in bytes.chunks_mut(64).enumerate() {
        let stream = block(key, nonce, counter as u64);
        for (byte, mask) in chunk.iter_mut().zip(stream) {
            *byte ^= mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_128_bit_expansion_with_nonzero_nonce_and_both_counter_words() {
        // Specification section9: k=(1..16), n=(101..116). The latter
        // splits into the eight-byte nonce and little-endian 64-bit counter.
        let key = std::array::from_fn(|index| (index + 1) as u8);
        let nonce = std::array::from_fn(|index| (index + 101) as u8);
        let counter = u64::from_le_bytes([109, 110, 111, 112, 113, 114, 115, 116]);
        let expected = [
            39, 173, 46, 248, 30, 200, 82, 17, 48, 67, 254, 239, 37, 18, 13, 247, 241, 200, 61,
            144, 10, 55, 50, 185, 6, 47, 246, 253, 143, 86, 187, 225, 134, 85, 110, 246, 161, 163,
            43, 235, 231, 94, 171, 51, 145, 214, 112, 29, 14, 232, 5, 16, 151, 140, 183, 141, 171,
            9, 122, 181, 104, 182, 177, 193,
        ];
        assert_eq!(block(&key, nonce, counter), expected);
    }
    #[test]
    fn published_quarterround_examples() {
        // Bernstein specification section3, public synthetic input/output facts.
        for (input, expected) in [
            ([0, 0, 0, 0], [0, 0, 0, 0]),
            ([1, 0, 0, 0], [0x08008145, 0x80, 0x10200, 0x20500000]),
            ([0, 1, 0, 0], [0x88000100, 1, 0x200, 0x00402000]),
            ([0, 0, 1, 0], [0x80040000, 0, 1, 0x2000]),
            ([0, 0, 0, 1], [0x00048044, 0x80, 0x10000, 0x20100001]),
            (
                [0xe7e8c006, 0xc4f9417d, 0x6479b4b2, 0x68c67137],
                [0xe876d72b, 0x9361dfd5, 0xf1460244, 0x948541a3],
            ),
            (
                [0xd3917c5b, 0x55f1c407, 0x52a58a7a, 0x8f887a3b],
                [0x3e2f308c, 0xd90a8f36, 0x6ab2a923, 0x2883524c],
            ),
        ] {
            let mut words = [0; 16];
            words[..4].copy_from_slice(&input);
            quarter(&mut words, [0, 1, 2, 3]);
            assert_eq!(words[..4], expected);
            assert_eq!(words[4..], [0; 12]);
        }
    }
    #[test]
    fn counter_high_word_is_effective_and_not_a_repeated_low_word() {
        let key = [0; 16];
        let nonce = [0; 8];
        assert_ne!(block(&key, nonce, 0), block(&key, nonce, 1u64 << 32));
        assert_ne!(
            block(&key, nonce, u32::MAX as u64),
            block(&key, nonce, 1u64 << 32)
        );
        assert_ne!(block(&key, nonce, 1), block(&key, nonce, 1u64 << 32));
    }
}
