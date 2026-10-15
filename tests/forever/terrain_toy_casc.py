# SPDX-License-Identifier: GPL-3.0-or-later
# Exact authored toyCASC writer functions plus literal camera WDC5 fixture, no clientbytes.
from pathlib import Path
import hashlib,struct
MASK=0xffffffff

def words(*values):
    return struct.pack('<' + 'I' * len(values), *values)

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

def source(ids,copies=()):
 b=bytearray(384);b[:4]=b'WDC5'
 def put(at,value):struct.pack_into('<I',b,at,value)
 for at,v in [(4,5),(136,len(ids)),(140,5),(144,28),(152,0xC2CEFDF9),(156,0x8BFD752B),(160,1),(164,max(1,len(ids),max((n for n,_ in copies),default=0))),(176,5),(188,120),(200,1),(212,384),(216,len(ids)),(228,len(ids)*4),(240,len(copies))]:put(at,v)
 struct.pack_into('<H',b,174,0xffff)
 for field,(offset,count) in enumerate([(0,3),(96,1),(128,1),(160,1),(192,1)]):
  at=264+24*field;struct.pack_into('<H',b,at,offset);put(at+16,32);put(at+20,count)
 for value in ids:
  at=len(b);b+=bytes(28);put(at+20,value)
 for rid in range(1,len(ids)+1):b+=words(rid)
 for new,old in copies:b+=words(new,old)
 return bytes(b)
