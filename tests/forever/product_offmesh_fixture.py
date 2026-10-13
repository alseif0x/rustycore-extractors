"""GPL-3.0-or-later. Authored 2026-10-13: toy product v6 offmesh input.

Two terrain islands, separated by explicit ADT holes. Full toy CASC is emitted
by product_fixture; no client files, network, Rust production imports or keys.
All connection coordinates are source/world XYZ, not Detour YZX coordinates.
"""
import struct
import product_fixture as plain

MAP, ROW, COLUMN = 7, 32, 32
FROM = (-283.3333, -183.3333, 3.125)
TO = (-283.3333, -383.3333, 3.125)


def bits(value):
    return f'{struct.unpack("<I", struct.pack("<f", value))[0]:08x}'


def connection(direction=0, start=FROM, end=TO, radius=4.0, area=1, flags=1,
               map_id=MAP, row=ROW, column=COLUMN):
    return [str(map_id), str(row), str(column), *(bits(v) for v in (*start, *end, radius)),
            str(direction), str(area), str(flags)]


def line(record):
    return 'offmesh-connection\t' + '\t'.join(record) + '\n'


def plan(connections=(), build=70205, locale='enUS', components='db2,maps,vmaps,mmaps'):
    text = plain.plan(components, build, locale)
    text = text.replace('forever-product\t1\n', 'forever-product\t6\n', 1)
    text = text.replace('root\theader-50893-v2\n',
                        'root\theader-50893-v2\ndecryption\tdisabled\n', 1)
    marker = 'non-steep-slope\t55\n'
    return text.replace(marker, marker + 'offmesh\tinline-f32-bits-v1\n' +
                        ''.join(line(record) for record in connections), 1)


def files():
    payloads = plain.files(tile=True)
    cells = []
    for y in range(16):
        for x in range(16):
            header = bytearray(128)
            struct.pack_into('<II', header, 4, x, y)
            # Continuous gentle slope on the visible islands as well as holes.
            # A perfectly flat mesh has a degenerate native BV vertical extent:
            # Recast raises vertices by one voxel beyond its zero-height bounds.
            struct.pack_into('<f', header, 112, 1.0 + y * 0.25)
            island = 7 <= y <= 9 and (4 <= x <= 6 or 10 <= x <= 12)
            struct.pack_into('<I', header, 60, 0 if island else 0xffff)
            heights = []
            for local_y in range(9):
                heights.extend([local_y / 32] * 9)
                if local_y < 8:
                    heights.extend([(local_y + 0.5) / 32] * 8)
            cells.append(plain.chunk('MCNK', header + plain.chunk(
                'MCVT', struct.pack('<145f', *heights))))
    payloads[201] = plain.chunk('MVER', plain.words(18)) + b''.join(cells)
    return payloads


def store(path, build=70205, locale='enUS'):
    plain.store(path, files(), build, locale)
