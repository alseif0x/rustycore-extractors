"""GPL-3.0-or-later. Authored toy v4/v5 DB2 product inputs.

WDC5 bytes below are an independent literal wire writer, with no production
decoder or existing DB2 writer imports. Only the complete toy CASC container
writer is reused. These fixtures establish no real-client or row-loader parity.
"""
import struct

import product_fixture


PROFILES = ((4, 70205, 'wowdbdefs-70205-v1'),
            (5, 70338, 'wowdbdefs-70338-v1'))
# Independently pinned literal metadata for the same two declarations in both
# reviewed catalogs. The CLI tests check these against each source catalog.
TABLES = {
    'AnimKitBoneSet': (1375433, 2827780344, 1139241839, (32, 32, 8, 8, 32)),
    'SpellDuration': (1137828, 3022256762, 2838609195, (32, 32, 32)),
}
HEADER = 204
SECTION = 40


def put32(data, offset, value):
    struct.pack_into('<I', data, offset, value)


def column_offset(name, field):
    return HEADER + 2 * SECTION + len(TABLES[name][3]) * 4 + field * 24


def table(name, empty=False, text_compression=0):
    """Two complete regular sections, signed scalars and cross-section copies.

    Text pointers address logical concatenated records/string pools, whereas
    physical sections interleave records, strings, IDs and copies. Include
    non-ASCII UTF-8 and an explicit null pointer, independently of host locale.
    Empty fixtures still carry complete field and section metadata.
    """
    _, table_hash, layout_hash, widths = TABLES[name]
    is_text = name == 'AnimKitBoneSet'
    record_size = sum(widths) // 8
    if empty:
        parts = [((), b'', ()), ((), b'', ())]
    elif is_text:
        # (stored ID, section-local string offset or None)
        parts = [(((20, 0), (21, None)), b'alpha\0', ((40, 30),)),
                 (((30, 0),), 'café\0'.encode('utf-8'), ((50, 40),))]
    else:
        parts = [(((20, (-7, 8, -9)),), b'', ((40, 30),)),
                 (((30, (100, -200, 300)),), b'', ((50, 40),))]
    count = sum(len(rows) for rows, _, _ in parts)
    data = bytearray(HEADER + 2 * SECTION + len(widths) * 28)
    data[:4] = b'WDC5'
    for offset, value in ((4, 5), (136, count), (140, len(widths)),
                          (144, record_size), (148, sum(len(s) for _, s, _ in parts)),
                          (152, table_hash), (156, layout_hash), (160, 10),
                          (164, 100), (176, len(widths)), (188, len(widths) * 24),
                          (200, 2)):
        put32(data, offset, value)
    struct.pack_into('<h', data, 174, -1)
    bit = 0
    for field, width in enumerate(widths):
        struct.pack_into('<H', data, column_offset(name, field), bit)
        bit += width
    if is_text:
        put32(data, column_offset(name, 0) + 8, text_compression)
    ordinal = prior_strings = 0
    for section, (rows, strings, copies) in enumerate(parts):
        start = HEADER + section * SECTION
        for offset, value in ((8, len(data)), (12, len(rows)), (16, len(strings)),
                              (24, len(rows) * 4), (36, len(copies))):
            put32(data, start + offset, value)
        for _, values in rows:
            if is_text:
                pointer = (count * record_size + prior_strings + values
                           - ordinal * record_size) if values is not None else 0
                data.extend(struct.pack('<Iibbi', pointer, -23, -5, 7, 1001))
            else:
                data.extend(struct.pack('<iii', *values))
            ordinal += 1
        data.extend(strings)
        for stored_id, _ in rows:
            data.extend(struct.pack('<I', stored_id))
        for new_id, source_id in copies:
            data.extend(struct.pack('<II', new_id, source_id))
        prior_strings += len(strings)
    return bytes(data)


def files(empty_text=False, text_compression=0, bad_hash=None):
    payloads = {values[0]: table(name, empty_text if name == 'AnimKitBoneSet' else False,
                               text_compression)
                for name, values in TABLES.items()}
    if bad_hash is not None:
        # The alphabetical first table fails before any producer artifact write.
        fid = TABLES['AnimKitBoneSet'][0]
        changed = bytearray(payloads[fid])
        offset = {'table': 152, 'layout': 156}[bad_hash]
        put32(changed, offset, struct.unpack_from('<I', changed, offset)[0] ^ 1)
        payloads[fid] = bytes(changed)
    return payloads


def plan(version, build, catalog):
    text = (f'forever-product\t{version}\nbuild\t{build}\nlocale\tenUS\n'
            'index\tcontiguous\nroot\theader-50893-v2\n'
            f'schema-catalog\t{catalog}\ndecryption\tdisabled\ncomponents\tdb2\n'
            'terrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\n'
            'skip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n')
    return text + ''.join(f'db2\t{name}\t{TABLES[name][0]}\n' for name in sorted(TABLES))


def store(path, payloads, build):
    product_fixture.store(path, payloads, build, 'enUS')
