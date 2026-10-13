// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original implementation of the MD5 algorithm for BLTE
// integrity checks. Algorithm constants and test vectors are from RFC 1321;
// no reference implementation is copied or translated. Not authentication.

const CONSTANTS: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];
const ROTATIONS: [[u32; 4]; 4] = [
    [7, 12, 17, 22],
    [5, 9, 14, 20],
    [4, 11, 16, 23],
    [6, 10, 15, 21],
];

pub(crate) fn digest(bytes: &[u8]) -> [u8; 16] {
    let mut state = [0x67452301_u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    let (blocks, remaining) = bytes.as_chunks::<64>();
    for block in blocks {
        compress(&mut state, block);
    }
    // MD5 stores the low 64 bits of the message length. Padding uses at most
    // two stack blocks; hashing does not duplicate or allocate the input.
    let mut tail = [0_u8; 128];
    tail[..remaining.len()].copy_from_slice(remaining);
    tail[remaining.len()] = 0x80;
    let tail_size = if remaining.len() < 56 { 64 } else { 128 };
    let bits = (bytes.len() as u64).wrapping_mul(8);
    tail[tail_size - 8..tail_size].copy_from_slice(&bits.to_le_bytes());
    for block in tail[..tail_size].as_chunks::<64>().0 {
        compress(&mut state, block);
    }
    let mut digest = [0_u8; 16];
    for (output, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        output.copy_from_slice(&word.to_le_bytes());
    }
    digest
}

fn compress(state: &mut [u32; 4], block: &[u8; 64]) {
    let mut words = [0_u32; 16];
    for (word, input) in words.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_le_bytes(*input);
    }
    let [mut a, mut b, mut c, mut d] = *state;
    for (step, constant) in CONSTANTS.into_iter().enumerate() {
        let (function, word_index) = match step / 16 {
            0 => ((b & c) | (!b & d), step),
            1 => ((b & d) | (c & !d), (5 * step + 1) % 16),
            2 => (b ^ c ^ d, (3 * step + 5) % 16),
            _ => (c ^ (b | !d), (7 * step) % 16),
        };
        let next = b.wrapping_add(
            a.wrapping_add(function)
                .wrapping_add(constant)
                .wrapping_add(words[word_index])
                .rotate_left(ROTATIONS[step / 16][step % 4]),
        );
        [a, b, c, d] = [d, next, b, c];
    }
    for (word, addend) in state.iter_mut().zip([a, b, c, d]) {
        *word = word.wrapping_add(addend);
    }
}

#[cfg(test)]
mod tests {
    use super::digest;

    fn hex(bytes: &[u8]) -> String {
        digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    #[test]
    fn rfc_1321_vectors() {
        for (input, expected) in [
            ("", "d41d8cd98f00b204e9800998ecf8427e"),
            ("a", "0cc175b9c0f1b6a831c399e269772661"),
            ("abc", "900150983cd24fb0d6963f7d28e17f72"),
            ("message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
            (
                "abcdefghijklmnopqrstuvwxyz",
                "c3fcd3d76192e4007dfb496cca67e13b",
            ),
            (
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
                "d174ab98d277d9f5a5611c2c9f419d9f",
            ),
            (
                "12345678901234567890123456789012345678901234567890123456789012345678901234567890",
                "57edf4a22be3c955ac49da2e2107b67a",
            ),
        ] {
            assert_eq!(hex(input.as_bytes()), expected);
        }
    }

    #[test]
    fn million_a_vector() {
        assert_eq!(
            hex(&vec![b'a'; 1_000_000]),
            "7707d6ae4e027c70eea2a935c2296f21"
        );
    }

    #[test]
    fn padding_boundaries_match_python_hashlib() {
        for (size, expected) in [
            (55, "6912ee65fff2d9f9ce2508cddf8bcda0"),
            (56, "51fdd1acda72405dfdfa03fcb85896d7"),
            (63, "48a6295221902e8e0938f773a7185e72"),
            (64, "b2d3f56bc197fd985d5965079b5e7148"),
            (65, "8bd7053801c768420faf816fadba971c"),
            (119, "1c772251899a7ff007400b888d6b2042"),
            (120, "b7ba1efc6022e9ed272f00b8831e26e6"),
            (127, "8402b21e7bc7906493bae0dac017f1f9"),
            (128, "37eff01866ba3f538421b30b7cbefcac"),
            (255, "11b7aaa64c413d2f0fccf893881c46a2"),
        ] {
            let input: Vec<u8> = (0..size).map(|value| value as u8).collect();
            assert_eq!(hex(&input), expected);
        }
    }
}
