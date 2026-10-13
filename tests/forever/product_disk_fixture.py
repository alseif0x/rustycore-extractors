"""GPL-3.0-or-later. Authored 2026-10-12: nonempty toy collision world.

Independent literal M2/WMO/placement bytes, composed with the authored CASC,
terrain and WDC5 fixture writers. No client data or production reader imports.
"""
import struct
import product_fixture as plain

FIELDS = ('raw', 'wdt', 'cache', 'placement', 'vmo', 'map', 'output')
BUDGET = 8 * 1024 * 1024


def plan(components='db2,gt,cameras,maps,vmaps,mmaps', build=70205,
         locale='enUS', supplied=False, budgets=None):
    budgets = {name: BUDGET for name in FIELDS} if budgets is None else budgets
    storage = ('decryption\t' + ('provided-salsa20-v1' if supplied else 'disabled')
               + '\ncollision-storage\tlinux-leased-append-disk-world-v1\n'
               + ''.join(f'collision-{name}-bytes\t{budgets[name]}\n' for name in FIELDS))
    return plain.plan(components, build, locale).replace(
        'forever-product\t1\n', 'forever-product\t3\n').replace(
        'root\theader-50893-v2\n', 'root\theader-50893-v2\n' + storage)


def m2():
    data = bytearray(304)
    data[:4] = b'MD20'
    for offset, value in ((216, 3), (220, 304), (224, 3), (228, 310)):
        struct.pack_into('<I', data, offset, value)
    return bytes(data) + struct.pack('<3H9f', 0, 1, 2, -0., 0., 0., 1., 0., 0., 0., 1., 0.)


def wmo_root():
    header = bytearray(64)
    struct.pack_into('<I', header, 4, 1)
    struct.pack_into('<I', header, 32, 777)
    header[60] = 4
    return (plain.chunk('MVER', plain.words(17)) + plain.chunk('MOHD', header)
            + plain.chunk('GFID', plain.words(502)))


def wmo_group():
    header = bytearray(68)
    struct.pack_into('<I', header, 56, 99)
    body = (bytes(header) + plain.chunk('MOPY', bytes((8, 0)))
            + plain.chunk('MOVI', struct.pack('<3H', 0, 1, 2))
            + plain.chunk('MOVT', struct.pack('<9f', 0., 0., 0., 1., 0., 0., 0., 1., 0.)))
    return plain.chunk('MOGP', body)


def objects():
    mddf = bytearray(36)
    struct.pack_into('<II', mddf, 0, 400, 123)
    struct.pack_into('<HH', mddf, 32, 1024, 0x40)
    modf = bytearray(64)
    struct.pack_into('<II', modf, 0, 501, 456)
    struct.pack_into('<H', modf, 56, 8)
    return (plain.chunk('MVER', plain.words(18)) + plain.chunk('MDDF', mddf)
            + plain.chunk('MODF', modf))


def files():
    payloads = plain.files(tile=True)
    payloads[plain.IDS['GameObjectDisplayInfo']] = plain.table(
        'GameObjectDisplayInfo', [{1: 400}, {1: 400}, {1: 501}, {1: 0}], [1, 2, 3, 4])
    payloads[202] = objects()
    payloads[400] = m2()
    payloads[501] = wmo_root()
    payloads[502] = wmo_group()
    return payloads
