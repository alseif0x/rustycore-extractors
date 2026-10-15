# SPDX-License-Identifier: GPL-3.0-or-later
# Authored mathematical Salsa fixture only. Reuses exact accepted cipher/vector
# functions; narrow store adapter uses the independent literal CASC writer.
import hashlib
from pathlib import Path
import struct
import zlib
import terrain_toy_casc as plain

KEYS = {0x101: bytes(range(16)), 0x202: bytes([0x22]) * 16,
        0x303: bytes([0x33]) * 16, 0x404: bytes([0x44]) * 16}

MASK = (1 << 32) - 1

def salsa(key, nonce, message):
    """128-bit key, 64-bit nonce, block counter zero; complete-message XOR."""
    assert len(key) == 16 and len(nonce) == 8
    constants = struct.unpack('<4I', b'expand 16-byte k')
    kw = struct.unpack('<4I', key)
    nw = struct.unpack('<2I', nonce)
    output = bytearray()

    def rotate(value, count):
        value &= MASK
        return ((value << count) | (value >> (32 - count))) & MASK

    for counter, start in enumerate(range(0, len(message), 64)):
        initial = [constants[0], *kw, constants[1], *nw,
                   counter & MASK, counter >> 32, constants[2], *kw, constants[3]]
        state = initial.copy()

        def quarter(a, b, c, d):
            state[b] ^= rotate(state[a] + state[d], 7)
            state[c] ^= rotate(state[b] + state[a], 9)
            state[d] ^= rotate(state[c] + state[b], 13)
            state[a] ^= rotate(state[d] + state[c], 18)

        for _ in range(10):
            for indices in [(0, 4, 8, 12), (5, 9, 13, 1), (10, 14, 2, 6), (15, 3, 7, 11),
                            (0, 1, 2, 3), (5, 6, 7, 4), (10, 11, 8, 9), (15, 12, 13, 14)]:
                quarter(*indices)
        stream = struct.pack('<16I', *((a + b) & MASK for a, b in zip(state, initial)))
        output.extend(a ^ b for a, b in zip(message[start:start + 64], stream))
    return bytes(output)

def check_vector():
    # eSTREAM Salsa20/20 Set1 vector0, 128-bit. Published at
    # https://raw.githubusercontent.com/weidai11/cryptopp/master/TestVectors/salsa.txt
    expected = bytes.fromhex(
        '4DFA5E481DA23EA09A31022050859936DA52FCEE218005164F267CB65F5CFD7F2B'
        '4F97E0FF16924A52DF269515110A07F9E460BC65EF95DA58F740B7D1DBB0AA')
    assert salsa(b'\x80' + bytes(15), bytes(8), bytes(64)) == expected
    expected_later = bytes.fromhex(
        'B375703739DACED4DD4059FD71C3C47FC2F9939670FAD4A46066ADCC6A564578'
        '3308B90FFB72BE04A6B147CBE38CC0C3B9267C296A92A7C69873F9F263BE9703')
    assert salsa(b'\x80' + bytes(15), bytes(8), bytes(512))[448:512] == expected_later

def keyfile(entries=None):
    entries = KEYS if entries is None else entries
    return 'forever-tact-keys\t1\n' + ''.join(
        f'{name:016x}\t{value.hex()}\n' for name, value in sorted(entries.items()))

def encrypted(content, name, framed=False, iv_size=4):
    def frame(part, index, compressed):
        inner = b'Z' + zlib.compress(part) if compressed else b'N' + part
        iv = bytes(range(1, iv_size + 1))
        nonce = bytearray(iv + bytes(8 - len(iv)))
        for at, byte in enumerate(struct.pack('<I', index)):
            nonce[at] ^= byte
        return b'E\x08' + struct.pack('<Q', name) + bytes([iv_size]) + iv + b'S' + salsa(KEYS[name], nonce, inner)

    if not framed:
        return b'BLTE' + bytes(4) + frame(content, 0, False)
    middle = max(1, len(content) // 2)
    parts = [content[:middle], content[middle:]]
    frames = [frame(part, i, i % 2 == 0) for i, part in enumerate(parts)]
    size = 12 + 24 * len(frames)
    return b'BLTE' + struct.pack('>I', size) + b'\x0f' + len(frames).to_bytes(3, 'big') + b''.join(
        struct.pack('>II', len(encoded), len(part)) + hashlib.md5(encoded).digest()
        for encoded, part in zip(frames, parts)) + b''.join(frames)

def store(path, payloads, build=70205, locale='enUS', metadata=True):
    """Create plain authored store, rewrite only private toy archive/index/config.

    Complete CKeys remain the original content hashes. Ciphertext, indexes and
    encoding-size are independently rewritten. Return private payload offsets
    for deliberate synthetic corruption; never place these in product reports.
    """
    check_vector()
    path = Path(path)
    plain.store(path, payloads, build, locale)
    data = path / 'Data/data'
    archive = (data / 'data.000').read_bytes()
    index = (data / '0000000001.idx').read_bytes()
    rows = [index[40 + at:40 + at + 18] for at in range(0, len(index) - 40, 18)]
    content_names = {content: 0x303 for content in payloads.values()}
    if 1294214 in payloads:
        content_names[payloads[1294214]] = 0x101
    rebuilt = bytearray()
    new_rows = []
    offsets = {}
    encoding_size = None
    for position, row in enumerate(rows):
        old_offset = int.from_bytes(row[9:14], 'big')
        old_size = struct.unpack('<I', row[14:])[0]
        record = archive[old_offset:old_offset + old_size]
        assert record[30:39] == b'BLTE\0\0\0\0N'
        content = record[39:]
        is_metadata = position >= len(rows) - 2
        name = 0x101 if is_metadata else content_names[content]
        wire = encrypted(content, name, position % 2 == 1, 8 if position % 3 == 0 else 4)
        if is_metadata and not metadata:
            wire = record[30:]
        offset = len(rebuilt)
        new_record = record[:30] + wire
        rebuilt.extend(new_record)
        new_rows.append(row[:9] + offset.to_bytes(5, 'big') + plain.words(len(new_record)))
        offsets[hashlib.md5(content).digest()] = (offset, len(new_record))
        if position == len(rows) - 1:
            encoding_size = len(new_record)
    (data / 'data.000').write_bytes(rebuilt)
    header = bytearray(index[:32])
    checksum = (0, 0)
    for row in new_rows:
        checksum = plain.hash2(row, *checksum)
    (data / '0000000001.idx').write_bytes(header + plain.words(len(new_rows) * 18, checksum[0]) + b''.join(new_rows))
    buildinfo = path / '.build.info'
    old_key = buildinfo.read_text().splitlines()[1].split('|')[3]
    old_config = path / 'Data/config' / old_key[:2] / old_key[2:4] / old_key
    lines = old_config.read_text().splitlines()
    lines = [(' '.join(line.split()[:-1]) + f' {encoding_size}') if line.startswith('encoding-size = ') else line for line in lines]
    config = ('\n'.join(lines) + '\n').encode()
    new_key = hashlib.md5(config).hexdigest()
    destination = path / 'Data/config' / new_key[:2] / new_key[2:4] / new_key
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(config)
    buildinfo.write_text(buildinfo.read_text().replace(old_key, new_key))
    return offsets
