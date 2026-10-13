"""GPL-3.0-or-later. Authored 2026-10-11: complete toy product input.

Independent literal CASC/WDT/ADT wire writer and the existing synthetic DB2
writer. No real client data, network, native libraries or production imports.
Contiguous indexes and Header50893V2 are explicit toy physical choices.
"""
from pathlib import Path
import hashlib
import json
import struct
import db2_oracle
from test_db2_oracle import Wdc5Builder

REPO = Path(__file__).resolve().parents[2]
SCHEMAS = db2_oracle.load_catalog()
CATALOG = json.loads((REPO / 'contracts/forever-metadata-v1.json').read_text())
IDS = {t['name']: t['file_data_id'] for t in CATALOG['tables']}
GT = json.loads((REPO / 'contracts/forever-gt-catalog-v1.json').read_text())['tables']

def words(*values):
    return struct.pack('<' + 'I' * len(values), *values)

def chunk(tag, payload):
    return tag.encode()[::-1] + words(len(payload)) + payload

# Bob Jenkins' original lookup3 byte algorithm is public domain. This authored
# Python adaptation reuses our 2026-10-11 synthetic startup fixture algorithm.
MASK = 0xffffffff
def rot(value, count):
    return ((value << count) | (value >> (32 - count))) & MASK

def hash2(data, primary=0, secondary=0):
    a = b = (0xdeadbeef + len(data) + primary) & MASK
    c = (a + secondary) & MASK
    while len(data) > 12:
        x, y, z = struct.unpack('<3I', data[:12])
        a = (a + x) & MASK; b = (b + y) & MASK; c = (c + z) & MASK
        a = ((a-c)&MASK)^rot(c,4); c=(c+b)&MASK
        b = ((b-a)&MASK)^rot(a,6); a=(a+c)&MASK
        c = ((c-b)&MASK)^rot(b,8); b=(b+a)&MASK
        a = ((a-c)&MASK)^rot(c,16); c=(c+b)&MASK
        b = ((b-a)&MASK)^rot(a,19); a=(a+c)&MASK
        c = ((c-b)&MASK)^rot(b,4); b=(b+a)&MASK
        data = data[12:]
    if not data:
        return c, b
    x, y, z = struct.unpack('<3I', data + bytes(12-len(data)))
    a=(a+x)&MASK; b=(b+y)&MASK; c=(c+z)&MASK
    c=((c^b)-rot(b,14))&MASK; a=((a^c)-rot(c,11))&MASK
    b=((b^a)-rot(a,25))&MASK; c=((c^b)-rot(b,16))&MASK
    a=((a^c)-rot(c,4))&MASK; b=((b^a)-rot(a,14))&MASK
    c=((c^b)-rot(b,24))&MASK
    return c, b

def table(name, rows=(), ids=()):
    return Wdc5Builder(SCHEMAS[name], rows, ids=ids).build()

def files(tile=False):
    result = {
        IDS['Map']: table('Map', [{0:'Synthetic', 1:'Synthetic map', 13:-1, 14:-1, 21:100}], [7]),
        IDS['LiquidType']: table('LiquidType'),
        IDS['LiquidMaterial']: table('LiquidMaterial'),
        IDS['LiquidObject']: table('LiquidObject'),
        IDS['GameObjectDisplayInfo']: table('GameObjectDisplayInfo'),
        IDS['CinematicCamera']: table('CinematicCamera', [{3:301}, {3:302}], [1,2]),
        301: b'authored opaque camera A', 302: b'authored opaque camera B',
    }
    header = bytearray(32); struct.pack_into('<I', header, 0, 0x200)
    main = bytearray(32768); maid = bytearray(131072)
    if tile:
        at = 32 * 64 + 32
        struct.pack_into('<I', main, at*8, 1)
        struct.pack_into('<II', maid, at*32, 201, 202)
        # Terrain input has one explicit flat MCNK. Collision's separate Obj0
        # input has no placements. Both remain complete supported files.
        cell = bytearray(128)
        struct.pack_into('<f', cell, 112, 1.)
        result[201] = chunk('MVER', words(18)) + chunk('MCNK', cell)
        result[202] = chunk('MVER', words(18))
    result[100] = chunk('MVER', words(18)) + chunk('MPHD',header) + chunk('MAIN',main) + chunk('MAID',maid)
    for entry in GT:
        columns = entry.get('value_columns')
        if columns is None:
            data = ('authored opaque ' + entry['name']).encode()
        else:
            data = ('id' + ''.join('\tc'+str(i) for i in range(columns)) + '\n1' + '\t1.25'*columns + '\n').encode()
        result[entry['file_id']] = data
    return result

def plan(components='db2,gt,cameras,maps,vmaps,mmaps', build=70205, locale='enUS', names=None):
    names = names if names is not None else ['CinematicCamera','LiquidType','Map']
    selected = components.split(',')
    text = (f'forever-product\t1\nbuild\t{build}\nlocale\t{locale}\nindex\tcontiguous\n'
        'root\theader-50893-v2\n' + f'components\t{components}\n'
        + ('terrain\tall\n' if 'maps' in selected else 'terrain\tnone\n')
        + 'pack\ttrue\n' + ('navigation\tall\n' if 'mmaps' in selected else 'navigation\tnone\n')
        + 'big-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n')
    if 'db2' in selected:
        text += ''.join(f'db2\t{name}\t{IDS[name]}\n' for name in sorted(names))
    return text

def store(path, payloads, build=70205, locale='enUS'):
    path = Path(path); path.mkdir(mode=0o700)
    datadir = path/'Data/data'; datadir.mkdir(parents=True)
    root = b'TSFM' + words(20,2,len(payloads),0)
    for fid, content in payloads.items():
        root += words(1,2 if locale=='enUS' else 0x80,0x10000000,0) + b'\0' + words(fid) + hashlib.md5(content).digest()
    rootkey = hashlib.md5(root).digest()
    contents = list(dict.fromkeys(payloads.values()))
    records = [(hashlib.sha256(b'toy-ekey'+words(i)).digest()[:16], content) for i,content in enumerate(contents)]
    root_ekey=b'\x33'*16; encoding_ekey=b'\x44'*16
    entries=[(rootkey,len(root),root_ekey)] + [(hashlib.md5(content).digest(),len(content),key) for key,content in records]
    page=b''.join(struct.pack('<H',1)+struct.pack('>I',size)+key+ekey for key,size,ekey in entries)
    assert len(page)<4096
    page+=bytes(4096-len(page))
    encoding=b'EN\x01\x10\x10'+struct.pack('>HHII',4,1,1,0)+b'\0'+struct.pack('>I',0)+rootkey+hashlib.md5(page).digest()+page
    config=(f'root = {rootkey.hex()}\nencoding = {hashlib.md5(encoding).hexdigest()} {encoding_ekey.hex()}\n'
        f'encoding-size = {len(encoding)} {len(encoding)+39}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.{build}\n').encode()
    config_key=hashlib.md5(config).hexdigest()
    cp=path/'Data/config'/config_key[:2]/config_key[2:4]/config_key;cp.parent.mkdir(parents=True);cp.write_bytes(config)
    cdn_key = (b'\x55' * 16).hex()
    (path/'.build.info').write_text('Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\n'
        f'wow_classic_beta|1|1.60.1.{build}|{config_key}|{cdn_key}|{locale}\n')
    records += [(root_ekey,root),(encoding_ekey,encoding)]
    archive=bytearray();rows=[]
    for key,content in records:
        offset=len(archive);record=b'\xa5'*30+b'BLTE\0\0\0\0N'+content;archive+=record
        rows.append(key[:9]+offset.to_bytes(5,'big')+words(len(record)))
    (datadir/'data.000').write_bytes(archive)
    for bucket in range(16):
        entries=rows if bucket==0 else [bytes([0xe0+bucket])*9+bytes(5)+words(39)]
        data=bytearray(32);struct.pack_into('<I',data,0,16);data[8:10]=struct.pack('<H',7);data[10]=bucket
        data[12:16]=bytes([4,5,9,30]);struct.pack_into('<I',data,4,hash2(data[8:24])[0]);checksum=(0,0)
        for row in entries:
            checksum=hash2(row,*checksum)
        data+=words(len(entries)*18,checksum[0])+b''.join(entries)
        (datadir/f'{bucket:02x}00000001.idx').write_bytes(data)
