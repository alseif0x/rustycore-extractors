"""GPL-3.0-or-later. Authored complete toy map-domain sources.

Independent literal source writer composed with the existing authored WDC5,
CASC and M2/WMO fixtures. No production readers, real client data or network.
Native oracle Map projection is explicit and has a distinct source identity.
"""
import struct

import product_fixture as plain
import product_disk_fixture as disk

PROFILE = 'map-and-ancestors-v1'
GAMEOBJECT = 'global-startup-v1'
PLACEMENT_IDS = 'selected-closure-serial-ids-v1'
# Physical direct-row order deliberately differs from parent order. Map7's
# cosmetic9 is ignored because its ordinary parent1 takes precedence.
ROWS = ((9, 999, -1, -1), (17, 117, -1, 7), (7, 101, 1, 9), (1, 100, -1, -1))


def map_table(rows=ROWS):
    return plain.table('Map', [{0: 'Synthetic', 1: 'Synthetic map',
                               13: parent, 14: cosmetic, 21: wdt}
                              for ident, wdt, parent, cosmetic in rows],
                       [ident for ident, *_ in rows])


def closure(map_id, rows=ROWS):
    """Independent authored graph walk, preserving physical row order."""
    by_id = {r[0]: r for r in rows}
    included = set()
    current = map_id
    while current >= 0:
        if current not in by_id or current in included:
            raise ValueError('missing or cyclic authored ancestor')
        included.add(current)
        _, _, parent, cosmetic = by_id[current]
        current = parent if parent >= 0 else cosmetic
    return tuple(r for r in rows if r[0] in included)


def wdt(tiles=(), globals=()):
    header = bytearray(32)
    struct.pack_into('<I', header, 0, 0x200)
    main, maid = bytearray(32768), bytearray(131072)
    for x, y, root, objects in tiles:
        at = y * 64 + x
        struct.pack_into('<I', main, at * 8, 1)
        struct.pack_into('<II', maid, at * 32, root, objects)
    result = (plain.chunk('MVER', plain.words(18)) + plain.chunk('MPHD', header)
              + plain.chunk('MAIN', main) + plain.chunk('MAID', maid))
    if globals:
        records = []
        for fid, unique in globals:
            record = bytearray(64)
            struct.pack_into('<II', record, 0, fid, unique)
            struct.pack_into('<H', record, 56, 8)
            records.append(record)
        result += plain.chunk('MODF', b''.join(records))
    return result


def objects(fid=400, unique=1000, position=(600., 0., 600.)):
    record = bytearray(36)
    struct.pack_into('<II', record, 0, fid, unique)
    struct.pack_into('<3f', record, 8, *position)
    struct.pack_into('<HH', record, 32, 1024, 0x40)
    return plain.chunk('MVER', plain.words(18)) + plain.chunk('MDDF', record)


def files(rows=ROWS, complete_unrelated=False):
    result = disk.files()
    result[plain.IDS['Map']] = map_table(rows)
    result[100] = wdt([(0, 0, 211, 212)], [(501, 3000)])
    result[101] = wdt([(32, 32, 201, 202)])
    result[117] = wdt()
    result[211] = result[201]
    result[212] = objects()
    # A startup-only model must survive the global list/model union despite
    # having no selected-map placement. Distinct complete source bytes, same
    # collision geometry, avoid deriving selection from acquired models.
    result[600] = disk.m2() + b'authored startup-only tail'
    result[plain.IDS['GameObjectDisplayInfo']] = plain.table(
        'GameObjectDisplayInfo', [{1: 400}, {1: 400}, {1: 501}, {1: 0}, {1: 600}],
        [1, 2, 3, 4, 5])
    if complete_unrelated:
        result[999] = wdt()
    return result


def plan(map_id=7, navigation=False, build=70205, locale='enUS', supplied=False,
         components=None, names=None):
    if components is None:
        components = 'db2,maps,vmaps,mmaps' if navigation else 'db2,maps,vmaps'
    if names is None:
        names = ['LiquidType', 'Map'] if navigation else ['Map']
    text = disk.plan(components, build, locale, supplied)
    text = text.replace('forever-product\t3\n', 'forever-product\t8\n', 1)
    marker = f'collision-output-bytes\t{disk.BUDGET}\n'
    text = text.replace(marker, marker + f'map-domain\t{PROFILE}\nmap-id\t{map_id}\n'
                        f'gameobject-scope\t{GAMEOBJECT}\nplacement-ids\t{PLACEMENT_IDS}\n', 1)
    text = text.replace('terrain\tall\n', f'terrain\tmap:{map_id}\n', 1)
    if navigation:
        text = text.replace('navigation\tall\n', f'navigation\tmap:{map_id}\n', 1)
        marker = 'non-steep-slope\t55\n'
        text = text.replace(marker, marker + 'offmesh\tinline-f32-bits-v1\n', 1)
    text = ''.join(line + '\n' for line in text.splitlines() if not line.startswith('db2\t'))
    if 'db2' in components.split(','):
        text += ''.join(f'db2\t{name}\t{plain.IDS[name]}\n' for name in sorted(names))
    return text


def native_inventory(payloads, map_id, rows=ROWS):
    """Original wrapper inventory with independently projected Map source.

    All global startup sources remain identical. The differing Map bytes are
    recorded by QA; this is conversion parity, not native selection parity.
    """
    projected = dict(payloads)
    projected[plain.IDS['Map']] = map_table(closure(map_id, rows))
    entries = sorted(projected.items())
    envelope = b'RCCASM01' + plain.words(len(entries), 0, 1)
    return envelope + b''.join(plain.words(fid, len(content), 0) + content
                                for fid, content in entries)
