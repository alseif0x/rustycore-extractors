# SPDX-License-Identifier: GPL-3.0-or-later
"""Independent authored numeric WDC5 multi-section fixture writer and oracle.

Pinned 02245dcd DB2FileLoader.cpp and original SpellDurationLoadInfo metadata:
1137828 / A931BD2B, three signed32 scalars, external ID, no parent or strings.
No product decoder/catalog/client bytes or helper imports. Direct values are
specified independently of encoding and independently decoded from the whole
wire inventory. Native callers must additionally gate these positive cases
through Rust first. Malformed cases are Rust-only; self-copy is never native.
Nothing executes at import. Python assertions must remain enabled.
"""
import hashlib
import json
from pathlib import Path
import struct

if not __debug__:
    raise RuntimeError('qa_assertions_required')

PIN = '02245dcd245e7433e524577656177723d3e4992e'
LAYOUT = 0xA931BD2B
FDID = 1137828
MAX_BYTES = 16 * 1024 * 1024
MAX_DIRECT = 4096
MAX_COPIES = 4096
MAX_ID = 4095
MASK = 0xFFFFFFFF


def put(data, at, value):
    struct.pack_into('<I', data, at, value)


def take(data, at, size):
    assert 0 <= at <= len(data) and 0 <= size <= len(data) - at, 'qa_extent'
    return data[at:at + size]


def word(data, at):
    return struct.unpack('<I', take(data, at, 4))[0]


def expected(direct, copies=()):
    """Direct public projection plus deferred ordered native index semantics."""
    physical = [[identifier, [v & MASK for v in values]] for identifier, values in direct]
    assert len({row[0] for row in physical}) == len(physical), 'qa_duplicate_id'
    assert all(0 <= row[0] <= MAX_ID and len(row[1]) == 3 for row in physical)
    pairs = [list(pair) for pair in copies]
    extent = 1 + max([0] + [row[0] for row in physical] + [pair[0] for pair in pairs])
    indexed = {identifier: values[:] for identifier, values in physical}
    for new, old in pairs:
        assert 0 <= new <= MAX_ID and 0 <= old <= MASK
        if old == 0 or old >= extent or old not in indexed:
            continue
        assert old != new, 'qa_native_self_copy'
        indexed[new] = indexed[old][:]
    return {'direct': physical, 'copies': pairs,
            'rows': [[key, indexed[key]] for key in sorted(indexed)]}


def build(sections, *, modes=None, palettes=None, common=None, min_id=0,
          max_id=MAX_ID, flags=4):
    """sections=[{'rows':[(stored_id,[v0,v1,v2])], 'copies':[(new,old)]}].

    For packed fields a row scalar is the stored index/bit payload. Compression
    parameters are mode, packed bit offset, width, scalar pallet-array count.
    Complete initialized eight-byte windows are part of every packed record.
    """
    assert 1 <= len(sections) <= 17
    modes, palettes, common = modes or {}, palettes or {}, common or {}
    count = sum(len(s.get('rows', ())) for s in sections)
    nc = sum(len(s.get('copies', ())) for s in sections)
    assert count <= MAX_DIRECT and nc <= MAX_COPIES
    cols, extra, slack = [], bytearray(), 0
    for field in range(3):
        mode, a, b, c = modes.get(field, (0, 0, 0, 0))
        size = 0
        if mode in (1, 3, 4, 5):
            assert 1 <= b <= 32
            slack = max(slack, a // 8 + 8)
        if mode in (3, 4):
            size = len(palettes[field]) * 4
        elif mode == 2:
            a, entries = common.get(field, (0, ()))
            size = len(entries) * 8
        cols.append((field * 32, 32, size, mode, a & MASK, b, c))
    for mode in (3, 4, 2):
        for field, col in enumerate(cols):
            if col[3] != mode:
                continue
            if mode == 2:
                for key, value in common.get(field, (0, ()))[1]:
                    extra += struct.pack('<II', key, value & MASK)
            else:
                for value in palettes[field]:
                    extra += struct.pack('<I', value & MASK)
    record_size = 12 + slack
    data = bytearray(204 + len(sections) * 40)
    data[:4] = b'WDC5'
    for at, value in ((4, 5), (136, count), (140, 3), (144, record_size),
                      (156, LAYOUT), (160, min_id), (164, max_id), (176, 3),
                      (180, 12), (188, 72), (192, sum(c[2] for c in cols if c[3] == 2)),
                      (196, sum(c[2] for c in cols if c[3] in (3, 4))), (200, len(sections))):
        put(data, at, value)
    struct.pack_into('<Hh', data, 172, flags, -1)
    for field in range(3):
        data += struct.pack('<hH', 0, field * 4)
    for col in cols:
        data += struct.pack('<HHIIIII', *col)
    data += extra
    for index, section in enumerate(sections):
        rows, copies = section.get('rows', ()), section.get('copies', ())
        sh = 204 + index * 40
        for at, value in ((8, len(data)), (12, len(rows)), (24, len(rows) * 4), (36, len(copies))):
            put(data, sh + at, value)
        for stored, values in rows:
            assert 0 <= stored <= MASK and len(values) == 3
            record = bytearray(record_size)
            for field, value in enumerate(values):
                _, _, _, mode, a, b, _ = cols[field]
                if mode == 0:
                    put(record, field * 4, value & MASK)
                elif mode in (1, 3, 4, 5):
                    payload = value & ((1 << b) - 1)
                    for bit in range(b):
                        at = 12 * 8 + a + bit
                        if payload & (1 << bit):
                            record[at // 8] |= 1 << (at % 8)
            data += record
        for stored, _ in rows:
            data += struct.pack('<I', stored)
        for new, old in copies:
            data += struct.pack('<II', new, old)
    assert len(data) <= MAX_BYTES
    return bytes(data)


def inspect(data):
    """Whole-input independent framing and all scalar-width bits oracle."""
    assert type(data) is bytes and 204 <= len(data) <= MAX_BYTES, 'qa_input_limit'
    assert data[:4] == b'WDC5' and word(data, 4) == 5, 'qa_version'
    assert word(data, 140) == word(data, 176) == 3 and word(data, 156) == LAYOUT, 'qa_schema'
    assert not data[172] & 1 and word(data, 184) == word(data, 148) == 0, 'qa_profile'
    sections, n, size = word(data, 200), word(data, 136), word(data, 144)
    minimum, maximum = word(data, 160), word(data, 164)
    assert 2 <= sections <= 16 and n <= MAX_DIRECT and size >= 12, 'qa_counts'
    assert minimum <= maximum <= MAX_ID, 'qa_id_range'
    assert word(data, 188) == 72, 'qa_columns'
    base = 216 + 40 * sections
    columns = [struct.unpack('<HHIIIII', take(data, base + f * 24, 24)) for f in range(3)]
    for field in range(3):
        assert struct.unpack('<hH', take(data, 204 + 40 * sections + 4 * field, 4)) == (0, field * 4), 'qa_field_metadata'
    palettes, commons, at = {}, {}, base + 72
    for mode in (3, 4, 2):
        start = at
        for field, (_, _, extra, compression, _, _, _) in enumerate(columns):
            assert compression <= 5, 'qa_compression'
            if compression in (0, 1, 5):
                assert extra == 0, 'qa_extra'
            if compression != mode:
                continue
            stride = 8 if mode == 2 else 4
            assert extra % stride == 0, 'qa_extra'
            payload = take(data, at, extra)
            at += extra
            if mode == 2:
                commons[field] = {key: value for key, value in struct.iter_unpack('<II', payload)}
            else:
                palettes[field] = [x[0] for x in struct.iter_unpack('<I', payload)]
        if mode == 4:
            assert at - (base + 72) == word(data, 196), 'qa_palette_extent'
        elif mode == 2:
            assert at - start == word(data, 192), 'qa_common_extent'
    direct, copies, global_ordinal = [], [], 0
    for index in range(sections):
        sh = 204 + 40 * index
        assert take(data, sh, 8) == bytes(8), 'qa_encrypted_section'
        assert word(data, sh + 8) == at, 'qa_section_offset'
        count, nc = word(data, sh + 12), word(data, sh + 36)
        assert count <= MAX_DIRECT - global_ordinal and nc <= MAX_COPIES - len(copies), 'qa_counts'
        assert all(word(data, sh + off) == 0 for off in (16, 20, 28, 32)), 'qa_auxiliary'
        assert word(data, sh + 24) == count * 4, 'qa_id_size'
        ids_at = at + count * size
        copies_at = ids_at + count * 4
        take(data, at, count * size + count * 4 + nc * 8)
        for row in range(count):
            stored = word(data, ids_at + row * 4)
            identifier = minimum + global_ordinal if stored <= minimum else stored
            assert identifier <= maximum, 'qa_id_range'
            record = take(data, at + row * size, size)
            values = []
            for field, (offset, _, _, mode, a, width, array_count) in enumerate(columns):
                if mode == 0:
                    assert offset % 8 == 0, 'qa_column_alignment'
                    value = struct.unpack('<I', take(record, offset // 8, 4))[0]
                elif mode == 2:
                    value = commons[field].get(identifier, a)
                else:
                    assert 1 <= width <= 32, 'qa_packed_width'
                    location = word(data, 180) + a // 8
                    packed = int.from_bytes(take(record, location, 8), 'little')
                    value = (packed >> (a % 8)) & ((1 << width) - 1)
                    if mode == 5:
                        value = (value ^ (1 << (width - 1))) - (1 << (width - 1))
                    elif mode in (3, 4):
                        assert mode != 4 or array_count == 1, 'qa_pallet_array'
                        assert value < len(palettes[field]), 'qa_palette_index'
                        value = palettes[field][value]
                values.append(value & MASK)
            direct.append((identifier, values))
            global_ordinal += 1
        for copy in range(nc):
            pair = struct.unpack('<II', take(data, copies_at + copy * 8, 8))
            assert pair[0] <= maximum, 'qa_copy_destination'
            copies.append(pair)
        at = copies_at + nc * 8
    assert global_ordinal == n and at == len(data), 'qa_whole_extent'
    return expected(direct, copies)


def cases():
    out = []
    def add(label, sections, direct, copies=(), **options):
        data = build(sections, **options)
        wanted = expected(direct, copies)
        assert inspect(data) == wanted, label
        out.append((label, data, wanted))
    def section(rows=(), copies=()):
        return {'rows': rows, 'copies': copies}
    for n in range(2, 17):
        rows = [(i + 1, [i, -i, i * 19]) for i in range(n)]
        add(f'section-count-{n}', [section([row]) for row in rows], rows)
    for empty_position in range(4):
        rows = [(1, [0, MASK, 0x80000000]), (7, [1, 2, 3]), (2, [4, 5, 6])]
        inventory = [section([r]) for r in rows]
        inventory.insert(empty_position, section())
        add(f'empty-section-{empty_position}', inventory, rows)
    add('all-empty-two', [section(), section()], [])
    add('all-empty-sixteen', [section() for _ in range(16)], [])
    pairs = [(4, 2), (5, 4), (7, 5)]
    rows = [(2, [MASK, 0x80000000, 7]), (3, [11, 12, 13])]
    add('cross-section-backward-chain', [section([rows[0]], pairs[:1]), section([rows[1]], pairs[1:])], rows, pairs)
    pairs = [(6, 3), (7, 6)]
    add('copy-source-in-later-section', [section([rows[0]], pairs), section([rows[1]])], rows, pairs)
    pairs = [(8, 7), (7, 2), (9, 8)]
    add('forward-reference-not-retried', [section([rows[0]], pairs[:1]), section([rows[1]], pairs[1:])], rows, pairs)
    pairs = [(2, 3), (4, 2), (4, 3), (5, 4)]
    add('overwrites-then-chains', [section([rows[0]], pairs[:2]), section([rows[1]], pairs[2:])], rows, pairs)
    pairs = [(5, 0), (6, MASK), (7, 999), (8, 4), (4, 2), (0, 3)]
    add('skipped-sources-and-destination-zero', [section([rows[0]], pairs[:3]), section([rows[1]], pairs[3:])], rows, pairs)
    pairs = [(9, 2), (10, 9)]
    add('copies-in-empty-sections', [section((), pairs[:1]), section(rows), section((), pairs[1:])], rows, pairs)
    stored = [(0, [1, 2, 3]), (10, [4, 5, 6]), (9, [7, 8, 9]), (50, [10, 11, 12])]
    repaired = [(10, stored[0][1]), (11, stored[1][1]), (12, stored[2][1]), stored[3]]
    add('repair-global-ordinal', [section(stored[:1]), section(), section(stored[1:3]), section(stored[3:])], repaired, min_id=10)
    add('id-zero-direct-source-zero-skips', [section([(0, [1, 2, 3])], [(4, 0)]), section([(2, [4, 5, 6])])], [(0, [1, 2, 3]), (2, [4, 5, 6])], [(4, 0)])
    # Every scalar is compared as 32 exact bits, including the signed extrema.
    for field in range(3):
        values = [0, 1, 0x7FFFFFFF, 0x80000000, MASK]
        direct = [(i + 1, [value if f == field else (17 + f) for f in range(3)]) for i, value in enumerate(values)]
        add(f'direct-field-{field}-extrema', [section(direct[:2]), section(direct[2:])], direct)
    for mode in (1, 5):
        for width in (1, 2, 5, 8, 16, 31, 32):
            mask = (1 << width) - 1
            payloads = [0, 1, mask, 1 << (width - 1)]
            for offset in (0, 3, 17):
                encoded = [(i + 1, [x, 77, 88]) for i, x in enumerate(payloads)]
                decoded = [(identifier, [((x ^ (1 << (width - 1))) - (1 << (width - 1))) if mode == 5 else x, 77, 88]) for (identifier, _), x in zip(encoded, payloads)]
                pairs = [(7, 1), (8, 7)]
                add(f'packed-mode-{mode}-width-{width}-offset-{offset}', [section(encoded[:2], pairs[:1]), section(encoded[2:], pairs[1:])], decoded, pairs, modes={0: (mode, offset, width, 0)})
    palette = [MASK, 0, 0x80000000, 0x7FFFFFFF]
    for mode in (3, 4):
        for field in range(3):
            encoded = [(i + 1, [i if f == field else 17 + f for f in range(3)]) for i in range(4)]
            decoded = [(i + 1, [palette[i] if f == field else 17 + f for f in range(3)]) for i in range(4)]
            add(f'palette-mode-{mode}-field-{field}', [section(encoded[:2]), section(encoded[2:])], decoded, modes={field: (mode, 11, 2, 1)}, palettes={field: palette})
    entries = [(10, MASK), (11, 0x80000000), (11, 7), (13, 0)]
    encoded = [(0, [99, 11, 22]), (0, [99, 33, 44]), (0, [99, 55, 66]), (0, [99, 77, 88])]
    decoded = [(10, [MASK, 11, 22]), (11, [7, 33, 44]), (12, [123, 55, 66]), (13, [0, 77, 88])]
    add('common-global-repair-duplicate-entry-default', [section(encoded[:1]), section(encoded[1:3]), section(encoded[3:])], decoded, min_id=10, modes={0: (2, 0, 0, 0)}, common={0: (123, entries)})
    encoded = [(2, [7, 0, 99]), (3, [15, 1, 99])]
    decoded = [(2, [7, MASK, 0x80000000]), (3, [15, 7, 5])]
    add('mixed-immediate-palette-common', [section(encoded[:1], [(5, 3)]), section(encoded[1:], [(6, 5)])], decoded, [(5, 3), (6, 5)], modes={0: (1, 3, 4, 0), 1: (3, 19, 1, 0), 2: (2, 0, 0, 0)}, palettes={1: [MASK, 7]}, common={2: (5, [(2, 0x80000000)])})
    rows = [(1 + i, [(i * 0x9E3779B1) & MASK, (-i) & MASK, i * 17]) for i in range(256)]
    add('sixteen-sections-256-rows', [section(rows[i:i + 16]) for i in range(0, 256, 16)], rows)
    assert len({label for label, _, _ in out}) == len(out)
    return out


def negative_cases():
    """Malformed/policy inputs; execute only the Rust example."""
    base = build([{'rows': [(1, [1, 2, 3])]}, {'rows': [(2, [4, 5, 6])]}])
    out = [('magic', b'XXXX' + base[4:]), ('short-header', base[:203]),
           ('truncated', base[:-1]), ('tail', base + b'\0')]
    def mutate(label, at, value):
        copy = bytearray(base); put(copy, at, value); out.append((label, bytes(copy)))
    for label, at, value in (
        ('version', 4, 4), ('layout', 156, 0), ('field-count', 140, 2),
        ('total-field-count', 176, 2), ('strings', 148, 1), ('sparse', 172, 5),
        ('parent', 184, 1), ('header-count', 136, 3), ('zero-size', 144, 0),
        ('min-over-max', 160, MAX_ID + 1), ('column-bytes', 188, 0),
        ('encrypted', 204, 1), ('first-offset', 212, 1),
        ('first-id-size', 228, 0), ('section-strings', 220, 1),
        ('section-catalog-offset', 224, 1), ('section-parent', 232, 12),
        ('section-catalog-count', 236, 1), ('second-offset', 252, 1),
        ('second-encrypted', 244, 1), ('second-count', 256, 3),
        ('column-mode', 296 + 8, 6), ('column-offset', 296, 65535),
        ('column-extra', 296 + 4, 4),
    ):
        mutate(label, at, value)
    out += [('one-section', build([{'rows': [(1, [1, 2, 3])]}])),
            ('seventeen-sections', build([{} for _ in range(17)])),
            ('duplicate-global-id', build([{'rows': [(2, [1, 2, 3])]}, {'rows': [(2, [4, 5, 6])]}])),
            ('self-copy', build([{'rows': [(2, [1, 2, 3])], 'copies': [(2, 2)]}, {}]))]
    packed = build([{'rows': [(1, [1, 2, 3])]}, {}], modes={0: (1, 0, 1, 0)})
    for label, at, value in (('packed-width-zero', 296 + 16, 0), ('packed-width-33', 296 + 16, 33), ('packed-outside', 296 + 12, 65535)):
        copy = bytearray(packed); put(copy, at, value); out.append((label, bytes(copy)))
    pallet = build([{'rows': [(1, [1, 2, 3])]}, {}], modes={0: (3, 0, 1, 0)}, palettes={0: [7]})
    out.append(('pallet-index', pallet))
    assert len({label for label, _ in out}) == len(out)
    return out


def verify_result(data, output, *, native):
    actual = json.loads(output) if isinstance(output, (str, bytes)) else output
    wanted = inspect(data)
    if not native:
        wanted = {'rows': wanted['rows']}
    assert type(actual) is dict and set(actual) == set(wanted), 'qa_output_shape'
    assert actual == wanted, 'qa_exact_scalar_mismatch'
    for key in ('rows', 'direct') if native else ('rows',):
        assert type(actual[key]) is list
        for row in actual[key]:
            assert type(row) is list and len(row) == 2 and type(row[0]) is int
            assert type(row[1]) is list and len(row[1]) == 3
            assert all(type(v) is int and 0 <= v <= MASK for v in row[1])
    if native:
        assert type(actual['copies']) is list
        for pair in actual['copies']:
            assert type(pair) is list and len(pair) == 2
            assert all(type(v) is int and 0 <= v <= MASK for v in pair)
    return {'materialized_rows': len(wanted['rows']),
            'scalar_bits_compared': 3 * len(wanted['rows'])}


def provenance():
    path = Path(__file__)
    data = path.read_bytes()
    return {'reference': PIN, 'table': 'SpellDuration', 'file_data_id': FDID,
            'layout_hash': f'{LAYOUT:08X}', 'physical_fields': 3, 'scalar_fields': 3,
            'materialized_record_bytes': 16, 'corpus_sha256': hashlib.sha256(data).hexdigest(),
            'product_decoder_imported': False, 'generated_catalog_imported': False,
            'real_client_assets': False, 'assertions_enabled': __debug__,
            'no_cases_at_import': True}
