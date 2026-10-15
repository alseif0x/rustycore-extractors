# SPDX-License-Identifier: GPL-3.0-or-later
"""Literal authored V12 terrain inputs. No I/O or execution at import time."""
import hashlib
import os
from pathlib import Path
import struct
import zlib
import terrain_toy_casc as toy_casc
import terrain_toy_encrypted as toy_encrypted

BUILD = 70374
MAP_ID, MATERIAL_ID, OBJECT_ID, TYPE_ID = 1349477, 1132538, 1308058, 1371380
MAP_FIELDS = ((32,1),(32,1),(32,1),(32,1),(32,1),(32,1),(32,2),
              (8,1),(8,1),(8,1),(16,1),(16,1),(16,1),(16,1),(16,1),
              (8,1),(32,1),(16,1),(8,1),(16,1),(32,1),(32,1),(32,1),
              (32,1),(32,1),(32,3))
TYPE_FIELDS = ((32,1),(32,6),(32,1),(8,1),(32,1),(32,1),(32,1),(32,1),
               (32,1),(32,1),(16,1),(32,1),(8,1),(8,1),(8,1),(32,1),
               (8,6),(32,3),(32,38),(32,4),(32,4))
# Current public catalog table hashes; independently pinned by source provenance.
TABLE_HASHES = {MAP_ID: 3179597154, MATERIAL_ID: 2352452393,
                OBJECT_ID: 4230614527, TYPE_ID: 1712570067}


def md5(data):
    return hashlib.md5(data).digest()


def words(*values):
    return struct.pack('<' + 'I' * len(values), *values)


def put(data, at, value):
    struct.pack_into('<I', data, at, value)


def offset(fields, index):
    return sum(width * count // 8 for width, count in fields[:index])


def table(layout, fields, rows, strings=b'', copies=()):
    size = offset(fields, len(fields))
    start = 244 + 28 * len(fields)
    data = bytearray(start)
    data[:4] = b'WDC5'
    for at, value in ((4,5),(136,len(rows)),(140,len(fields)),(144,size),
                      (148,len(strings)),(156,layout),(164,0xffffffff),
                      (176,len(fields)),(180,size),(188,24*len(fields)),
                      (200,1),(212,start),(216,len(rows)),(220,len(strings)),
                      (228,4*len(rows)),(240,len(copies))):
        put(data, at, value)
    struct.pack_into('<HH', data, 172, 4, 0xffff)
    for i, (width, count) in enumerate(fields):
        struct.pack_into('<HH', data, 244+4*i, 32-width, offset(fields,i))
        struct.pack_into('<HH', data, 244+4*len(fields)+24*i,
                         8*offset(fields,i), width*count)
    for _, row in rows:
        if len(row) != size:
            raise ValueError('authored_row_size')
        data.extend(row)
    data.extend(strings)
    for rid, _ in rows:
        data.extend(words(rid))
    for new, old in copies:
        data.extend(words(new, old))
    return data


def map_table(partial):
    size = offset(MAP_FIELDS, len(MAP_FIELDS))
    rows, strings = [], bytearray()
    for ordinal, (rid, wdt) in enumerate(((0,100),(9,101))):
        row = bytearray(size)
        for field, text in ((0,b'Synthetic'),(1,b'Name')):
            relative = (2-ordinal)*size + len(strings) - offset(MAP_FIELDS,field)
            put(row, offset(MAP_FIELDS,field), relative)
            strings.extend(text+b'\0')
        put(row, offset(MAP_FIELDS,21), wdt)
        for field in (13,14):
            struct.pack_into('<h', row, offset(MAP_FIELDS,field), -1)
        rows.append((rid,row))
    data = table(0xD43AFAC3, MAP_FIELDS, rows, strings)
    put(data,152,TABLE_HASHES[MAP_ID])
    if not partial:
        return bytes(data), None
    # Add an omitted physical section, preserving virtual string addresses.
    old_start = struct.unpack_from('<I',data,212)[0]
    for ordinal in range(2):
        for field in (0,1):
            at = old_start + ordinal*size + offset(MAP_FIELDS,field)
            put(data,at,struct.unpack_from('<I',data,at)[0]+size)
    out = data[:244] + bytes(40) + data[244:old_start] + words(0) + data[old_start:]
    put(out,136,3)
    put(out,200,2)
    put(out,212,old_start+44)
    hole_start = len(out)
    struct.pack_into('<Q',out,244,1)  # nonzero unknown TACT key name
    put(out,252,hole_start)
    put(out,256,1)
    put(out,268,4)
    out.extend(bytes([0xa5])*size + words(99))
    return bytes(out), hole_start


def liquids():
    material = table(0x98E5D7AA, ((32,1),(8,1)), [(7,words(0)+b'\0')])
    obj = bytearray(12)
    struct.pack_into('<H',obj,8,65535)
    objects = table(0xCB0D39E8, ((32,1),(32,1),(16,1),(8,1),(8,1)),
                    [(9,obj)], copies=((10,9),(11,10),(0,0),(12,99)))
    row = bytearray(offset(TYPE_FIELDS,len(TYPE_FIELDS)))
    row[offset(TYPE_FIELDS,14)] = 7
    types = table(0xD1ECEEC9,TYPE_FIELDS,[(22,row)])
    result = {MATERIAL_ID:material, OBJECT_ID:objects, TYPE_ID:types}
    for fid, data in result.items():
        put(data,152,TABLE_HASHES[fid])
    return {fid:bytes(data) for fid,data in result.items()}


def chunk(tag, data):
    return tag[::-1] + words(len(data)) + data


def wdt(tiles):
    mphd = bytearray(32)
    put(mphd,0,0x200)
    main, maid = bytearray(32768), bytearray(131072)
    for x,y,fid in tiles:
        at = 64*y+x
        put(main,8*at,1)
        put(main,8*at+4,1234)
        put(maid,32*at,fid)
        put(maid,32*at+4,201)  # Obj0 fallback must not substitute for root ADT.
    return (chunk(b'MVER',words(18)) + chunk(b'MPHD',mphd)
            + chunk(b'MAIN',main) + chunk(b'MAID',maid))


def adt(height, liquid=False):
    cell = bytearray(128)
    put(cell,52,42)
    cell[112:116] = struct.pack('<f',height)
    out = chunk(b'MVER',words(18)) + chunk(b'MCNK',cell)
    if liquid:
        mh2o = bytearray(3096)
        put(mh2o,0,3072)
        put(mh2o,4,1)
        struct.pack_into('<H',mh2o,3072,22)
        mh2o[3076:3084] = struct.pack('<ff',5,5)
        mh2o[3086:3088] = bytes((8,8))
        out += chunk(b'MH2O',mh2o)
    return out


def payloads(*, partial=True, missing_present=False):
    if type(partial) is not bool or type(missing_present) is not bool:
        raise ValueError('fixture_options')
    map_bytes,hole = map_table(partial)
    result = {MAP_ID:map_bytes, **liquids(), 100:wdt(((3,2,201),(1,4,202))),
              101:wdt(()), 201:adt(1,True)}
    if not missing_present:
        result[202] = adt(2)
    return result,hole


def plan(selection='map:0'):
    if selection not in ('map:0','all-available-maps'):
        raise ValueError('fixture_selection')
    return ('forever-product\t12\nbuild\t70374\nlocale\tesES\nindex\tcontiguous\n'
            'root\theader-50893-v2\nschema-catalog\twowdbdefs-70374-v1\n'
            'decryption\tdisabled\ncomponents\tmaps\nterrain\t'+selection+'\n'
            'pack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\n'
            'skip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n')


def map_wire(content, hole):
    if not isinstance(content,bytes) or not isinstance(hole,int) or not 0 < hole < len(content):
        raise ValueError('map_hole')
    # Z known prefix, then an actual Salsa-encrypted N frame. Index1 nonce matters.
    clear = b'Z'+zlib.compress(content[:hole])
    iv = bytes((1,2,3,4))
    nonce = bytearray(iv+bytes(4))
    nonce[0] ^= 1
    encrypted = (b'E\x08'+struct.pack('<Q',0x303)+b'\x04'+iv+b'S'
                 + toy_encrypted.salsa(toy_encrypted.KEYS[0x303],nonce,b'N'+content[hole:]))
    frames = (clear,encrypted)
    sizes = (hole,len(content)-hole)
    header_size = 12+24*len(frames)
    header = (b'BLTE'+struct.pack('>I',header_size)+b'\x0f\0\0\x02'
              + b''.join(struct.pack('>II',len(frame),size)+md5(frame)
                         for frame,size in zip(frames,sizes)))
    wire = header+b''.join(frames)
    return wire,md5(header)


def rewrite_map(client, payload, hole):
    """Rewrite only authored Map wire and dependent ENCODING/index/config facts."""
    client = Path(client)
    data = client/'Data/data'
    archive = (data/'data.000').read_bytes()
    index_path = data/'0000000001.idx'
    index = index_path.read_bytes()
    entries = [index[at:at+18] for at in range(40,len(index),18)]
    records = []
    for entry in entries:
        at = int.from_bytes(entry[9:14],'big')
        n = struct.unpack_from('<I',entry,14)[0]
        record = archive[at:at+n]
        if record[30:39] != b'BLTE\0\0\0\0N':
            raise ValueError('strict_authored_archive')
        records.append((entry[:9],record[39:]))
    if records[0][1] != payload:
        raise ValueError('map_first_record')
    wire,ekey = map_wire(payload,hole)
    encoding = bytearray(records[-1][1])
    page = bytearray(encoding[54:])
    matches = 0
    for at in range(0,len(page)-37,38):
        if struct.unpack_from('<H',page,at)[0] == 0:
            break
        if page[at+6:at+22] == md5(payload):
            page[at+22:at+38] = ekey
            matches += 1
    if matches != 1:
        raise ValueError('one_map_encoding_route')
    encoding[38:54],encoding[54:] = md5(page),page
    rebuilt,new_entries = bytearray(),[]
    for ordinal,(prefix,content) in enumerate(records):
        if ordinal == 0:
            encoded = wire
            prefix = ekey[:9]
        elif ordinal == len(records)-1:
            encoded = b'BLTE'+bytes(4)+b'N'+encoding
        else:
            encoded = b'BLTE'+bytes(4)+b'N'+content
        at = len(rebuilt)
        record = bytes([0xa5])*30+encoded
        rebuilt.extend(record)
        new_entries.append(prefix+at.to_bytes(5,'big')+words(len(record)))
    (data/'data.000').write_bytes(rebuilt)
    checksum = (0,0)
    for entry in new_entries:
        checksum = toy_casc.hash2(entry,*checksum)
    index_path.write_bytes(index[:32]+words(18*len(new_entries),checksum[0])+b''.join(new_entries))
    build_info = client/'.build.info'
    build_text = build_info.read_text()
    old_key = build_text.splitlines()[1].split('|')[3]
    old_config = client/'Data/config'/old_key[:2]/old_key[2:4]/old_key
    lines = old_config.read_text().splitlines()
    lines = [f'encoding = {md5(encoding).hex()} '+('44'*16) if line.startswith('encoding = ') else
             f'encoding-size = {len(encoding)} {len(encoding)+39}' if line.startswith('encoding-size = ') else line
             for line in lines]
    config = ('\n'.join(lines)+'\n').encode()
    key = md5(config).hex()
    destination = client/'Data/config'/key[:2]/key[2:4]/key
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_bytes(config)
    build_info.write_text(build_text.replace(old_key,key))


def store(parent, *, partial=True, missing_present=False):
    """Create a new authored client only; caller owns and freezes its inventory."""
    parent = Path(parent)
    if not parent.is_absolute() or parent.resolve(strict=True) != parent or not parent.is_dir():
        raise ValueError('fixture_parent')
    client = parent/'client'
    if os.path.lexists(client):
        raise ValueError('new_fixture_client')
    data,hole = payloads(partial=partial,missing_present=missing_present)
    toy_casc.store(client,data,build=BUILD,locale='esES')
    if partial:
        rewrite_map(client,data[MAP_ID],hole)
    # Old writer's intermediate directories use the caller's umask. Seal owned fixture.
    for base,dirs,files in os.walk(client):
        Path(base).chmod(0o700)
        for name in files:
            (Path(base)/name).chmod(0o600)
    return client,expected(partial=partial,missing_present=missing_present)


def expected(*, partial=True, missing_present=False):
    data,hole = payloads(partial=partial,missing_present=missing_present)
    return {'build':BUILD,'locale':'esES','components':8,'decryption':'disabled',
            'source_files':len(data),'source_bytes':sum(map(len,data.values())),
            'map_declared_bytes':len(data[MAP_ID]),
            'map_available_bytes':hole if partial else len(data[MAP_ID]),
            'map_full_ckey_verified':not partial,'map_encoded_key_authenticated':partial,
            'map_declared_direct_rows':3 if partial else 2,
            'map_admitted_direct_rows':2,'map_omitted_direct_rows':1 if partial else 0,
            'map0_present':2,'map0_absent':4094,'map9_present':0,'map9_absent':4096,
            'known_present_missing':missing_present,'selected_map0_output_files':4,
            'all_output_files':5,'product_oracle_maps_bytes':None}


def product_test_store(parent, mode):
    """Authored fixture for Rust CLI integration only; no actual client inputs."""
    if mode not in ('complete', 'missing_adt', 'missing_wdt', 'malformed_adt'):
        raise ValueError('fixture_mode')
    parent = Path(parent)
    data, hole = payloads(partial=True, missing_present=mode == 'missing_adt')
    if mode == 'missing_wdt':
        del data[100]
    if mode == 'malformed_adt':
        data[201] = chunk(b'MVER', words(18)) + chunk(b'MCNK', bytes(4))
    client = parent / 'client'
    if os.path.lexists(client):
        raise ValueError('fixture_exists')
    toy_casc.store(client, data, build=BUILD, locale='esES')
    rewrite_map(client, data[MAP_ID], hole)
    for base, dirs, files in os.walk(client):
        Path(base).chmod(0o700)
        for name in files:
            (Path(base) / name).chmod(0o600)
    return client


if __name__ == '__main__':
    import sys
    if len(sys.argv) != 3:
        raise SystemExit('authored fixture expects parent and mode')
    product_test_store(Path(sys.argv[1]), sys.argv[2])
