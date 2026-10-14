# SPDX-License-Identifier: GPL-3.0-or-later
"""Authored product-v9 toy CASC and private synthetic package helpers.
No real client inputs, historical oracle access, discovery or native consumers.
"""
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import signal
import stat
import struct
import subprocess
import time

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / 'contracts/forever-gt-catalog-v1.json'
SOURCE = 'dbc/esES/CinematicCamera.db2'
compact = lambda value: json.dumps(value, separators=(',', ':'), ensure_ascii=True)
sha = lambda data: hashlib.sha256(data).hexdigest()

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


def private_bytes(path, limit):
    fd = os.open(path, os.O_RDONLY|os.O_NOFOLLOW|os.O_NONBLOCK)
    with os.fdopen(fd, 'rb') as handle:
        before = os.fstat(handle.fileno())
        assert stat.S_ISREG(before.st_mode) and before.st_nlink == 1 and before.st_size <= limit
        data = handle.read(limit+1)
        after = os.fstat(handle.fileno())
        identity = lambda m: (m.st_dev,m.st_ino,m.st_mode,m.st_nlink,m.st_size,m.st_mtime_ns,m.st_ctime_ns)
        assert len(data) == before.st_size and identity(before) == identity(after)
        return data


def inventory(path):
    if not path.exists(): return {'exists': False}
    files, directories = {}, []
    for item in sorted(path.rglob('*')):
        meta = item.lstat(); assert not stat.S_ISLNK(meta.st_mode)
        name = item.relative_to(path).as_posix()
        if stat.S_ISDIR(meta.st_mode): directories.append(name)
        else:
            assert stat.S_ISREG(meta.st_mode) and meta.st_nlink == 1
            data = private_bytes(item, 128*1024*1024)
            files[name] = {'bytes': len(data), 'sha256': sha(data)}
    return {'exists': True, 'files': files, 'directories': directories}


def package_data(path):
    return {k:v for k,v in inventory(path)['files'].items() if k.startswith(('dbc/','gt/','cameras/'))}


def rewrite_metadata(path, changes):
    """Authored negative packages only. Preserve canonical existing wire contract."""
    manifest = path/'forever-package.json'
    if manifest.exists(): context = json.loads(manifest.read_text())['context']
    else: context = json.loads((path/'.state/context.json').read_text())
    facts = [json.loads(p.read_text()) for p in (path/'.state/files').glob('*.json')]
    facts = changes(facts)
    facts.sort(key=lambda f:f['path'].lower())
    rendered = [compact(f) for f in facts]
    for old in (path/'.state/files').glob('*.json'): old.unlink()
    for fact,line in zip(facts,rendered):
        (path/'.state/files'/f"{sha(fact['path'].lower().encode())}.json").write_text(line)
    for seal in (path/'.state/sealed').iterdir():
        selected = [(f,s) for f,s in zip(facts,rendered) if f['component']==seal.name]
        value = {'component':seal.name,'context_sha256':sha(compact(context).encode()),
                 'files':len(selected),'bytes':sum(f['bytes'] for f,_ in selected),
                 'inventory_sha256':sha(''.join(s+'\n' for _,s in selected).encode())}
        seal.write_text(compact(value))
    if manifest.exists():
        text = '{"manifest_version":1,"scope":"artifact_integrity","status":"complete","context":'+compact(context)+',"artifacts":[\n'
        text += ',\n'.join(rendered)+'\n'
        text += '],"file_count":'+str(len(facts))+',"total_bytes":'+str(sum(f['bytes'] for f in facts))+',"inventory_sha256":"'+sha(''.join(s+'\n' for s in rendered).encode())+'"}\n'
        manifest.write_text(text)


def fixture(parent, camera_ids=tuple(range(501,511)), changed_gt=False):
    catalog = json.loads(CATALOG.read_text())['tables']
    source_bytes = source(camera_ids)
    payloads = {1294214:source_bytes}; expected = {SOURCE:{'bytes':len(source_bytes),'sha256':sha(source_bytes)}}
    for entry in catalog:
        columns = entry.get('value_columns')
        data = (('id'+''.join('\tc'+str(i) for i in range(columns))+'\n1'+'\t1.25'*columns+'\n').encode()
                if columns is not None else ('authored opaque '+entry['name']).encode())
        if changed_gt and columns is None: data += b' changed input identity'
        payloads[entry['file_id']] = data
        expected[entry['relative_path']] = {'bytes':len(data),'sha256':sha(data)}
    for fid in camera_ids:
        data = f'authored opaque camera {fid}'.encode()
        payloads[fid]=data; expected[f'cameras/FILE{fid:08X}.xxx']={'bytes':len(data),'sha256':sha(data)}
    client=parent/'client'; store(client,payloads,70338,'esES')
    return client, expected


def fsync_ordinals(trace, output):
    entries=[]
    for line in trace.read_text().splitlines():
        match=re.fullmatch(r'(\d+) fsync\(\d+<([^<>\r\n]+)>\)\s+= 0',line)
        if match: entries.append((match[1],match[2]))
    assert entries and len({pid for pid,_ in entries})==1, 'calibration_single_writer_required'
    found={}
    for ordinal,(_,name) in enumerate(entries,1):
        for component in ('gt','cameras'):
            if name==str(output/component) and component not in found:found[component]=ordinal
    assert set(found)=={'gt','cameras'}
    return found



def rebind_checkpoint_producer(output):
    """Consistently author a different producer identity in a synthetic checkpoint.

    A supplied packaged binary has no different debug/release counterpart. This
    tests context rejection with authored state instead of altering an executable.
    """
    assert not (output / 'forever-package.json').exists()
    path = output / '.state/context.json'
    context = json.loads(path.read_text())
    value = context['producer_sha256']
    context['producer_sha256'] = ('1' if value[0] == '0' else '0') + value[1:]
    text = compact(context)
    path.write_text(text)
    digest = sha(text.encode())
    for name in ('protocol.json', 'transaction.json'):
        path = output / '.state' / name
        if path.exists():
            value = json.loads(path.read_text())
            value['context_sha256'] = digest
            path.write_text(compact(value))
    for path in (output / '.state/sealed').iterdir():
        value = json.loads(path.read_text())
        value['context_sha256'] = digest
        path.write_text(compact(value))


def save(path, value):
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, 'w') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def source_paths():
    paths = [REPO / name for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')]
    for directory in ('crates', 'contracts', 'schemas', 'tests'):
        paths.extend(path for path in (REPO / directory).rglob('*')
                     if path.is_file() and '__pycache__' not in path.parts)
    return sorted(paths)


def source_guards():
    return {str(path): sha(private_bytes(path, 32 * 1024 * 1024)) for path in source_paths()}


def environment(root):
    tmp = root / 'tmp'
    tmp.mkdir(mode=0o700)
    return {'PATH': '/usr/bin:/bin', 'HOME': str(root), 'LANG': 'C',
            'LC_ALL': 'C', 'TMPDIR': str(tmp), 'PYTHONDONTWRITEBYTECODE': '1'}


def stop_group(process):
    actions = []
    def absent():
        process.poll()
        try:
            os.killpg(process.pid, 0)
            return False
        except ProcessLookupError:
            return True
    if absent():
        return {'group_absent': True, 'signals': actions}
    for signum in (signal.SIGTERM, signal.SIGKILL):
        try:
            os.killpg(process.pid, signum)
            actions.append(signum.name)
        except ProcessLookupError:
            return {'group_absent': True, 'signals': actions}
        for _ in range(30):
            if absent():
                return {'group_absent': True, 'signals': actions}
            time.sleep(0.1)
    raise AssertionError('owned_group_cleanup_unverified')


def child_limits(proof):
    # Same downward process envelope and kernel denial as accepted issue48 control2.
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CPU, (600, 600))
    resource.setrlimit(resource.RLIMIT_AS, (4 * 1024**3, 4 * 1024**3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (256 * 1024**2, 256 * 1024**2))
    class Filter(ctypes.Structure):
        _fields_ = [('code', ctypes.c_ushort), ('jt', ctypes.c_ubyte),
                    ('jf', ctypes.c_ubyte), ('k', ctypes.c_uint)]
    class Program(ctypes.Structure):
        _fields_ = [('len', ctypes.c_ushort), ('filter', ctypes.POINTER(Filter))]
    rules = [(0x20,0,0,4),(0x15,1,0,0xc000003e),(0x06,0,0,0x80000000),
             (0x20,0,0,0),(0x45,0,1,0x40000000),(0x06,0,0,0x50000|errno.EPERM),
             (0x15,2,0,41),(0x15,1,0,53),(0x06,0,0,0x7fff0000),
             (0x20,0,0,16),(0x15,1,0,1),(0x06,0,0,0x50000|errno.EPERM),
             (0x06,0,0,0x7fff0000)]
    code = (Filter * len(rules))(*(Filter(*rule) for rule in rules))
    program = Program(len(rules), code)
    libc = ctypes.CDLL(None, use_errno=True)
    assert libc.prctl(38, 1, 0, 0, 0) == 0, 'no_new_privs_failed'
    assert libc.prctl(22, 2, ctypes.byref(program), 0, 0) == 0, 'seccomp_failed'
    assert libc.prctl(21, 0, 0, 0, 0) == 2, 'seccomp_not_active'
    for domain in (2, 10):
        ctypes.set_errno(0)
        assert libc.syscall(41, domain, 1, 0) == -1 and ctypes.get_errno() == errno.EPERM
    with open(proof, 'x') as stream:
        stream.write('seccomp-mode2;native-x86_64;AF_INET/AF_INET6/all-non-AF_UNIX-socket-domains-denied\n')


def command(argv, root, label, env, expected=0, trace_fsync=False,
            kill_fsync=None, timeout_seconds=660):
    assert 0 < timeout_seconds <= 660
    assert kill_fsync is None or (trace_fsync and isinstance(kill_fsync, int) and kill_fsync > 0)
    trace = root / (label + '.network.trace')
    wrapped = ['/usr/bin/strace', '-f', '-qq', '-yy', '-s', '128', '-e',
               'trace=network,fsync' if trace_fsync else 'trace=network',
               '-e', 'inject=network:error=EPERM', '-o', str(trace)]
    if kill_fsync is not None:
        wrapped += ['-e', f'inject=fsync:signal=SIGKILL:when={kill_fsync}']
    wrapped += list(map(str, argv))
    with (root / (label + '.stdout')).open('xb') as out, (root / (label + '.stderr')).open('xb') as err:
        process = subprocess.Popen(wrapped, stdout=out, stderr=err, env=env,
            stdin=subprocess.DEVNULL, close_fds=True, start_new_session=True,
            preexec_fn=lambda: child_limits(root / (label + '.seccomp-proof')))
        save(root / (label + '.launch.json'), {'argv': list(map(str, argv)),
             'pid': process.pid, 'process_group': process.pid, 'timeout_seconds': timeout_seconds})
        wait_error = None
        try:
            process.wait(timeout=timeout_seconds)
        except BaseException as error:
            wait_error = type(error).__name__
        finally:
            cleanup = stop_group(process)
            save(root / (label + '.cleanup.json'), cleanup)
        code = process.returncode
        save(root / (label + '.result.json'), {'exit': code, 'wait_error': wait_error,
             'stdout_sha256': sha((root / (label + '.stdout')).read_bytes()),
             'stderr_sha256': sha((root / (label + '.stderr')).read_bytes()),
             'trace_sha256': sha(trace.read_bytes()), 'group_absent': cleanup['group_absent']})
    lines = trace.read_text().splitlines()
    for line in lines:
        signal_line = re.fullmatch(r'\d+ --- SIGCHLD \{[^\r\n}]*\} ---', line)
        fsync_line = trace_fsync and re.fullmatch(r'\d+ fsync\(\d+<[^<>\r\n]+>\)\s+= (?:0|\?)(?: \(INJECTED\))?', line)
        kill_line = kill_fsync is not None and (re.fullmatch(r'\d+ \+\+\+ killed by SIGKILL \+\+\+', line)
                    or re.fullmatch(r'\d+ --- SIGKILL \{[^\r\n}]*\} ---', line))
        assert signal_line or fsync_line or kill_line, 'runtime_network_or_unknown_event'
    save(root / (label + '.audit.json'), {'trace_sha256': sha(trace.read_bytes()),
         'actual_network_events': 0, 'classified_lines': len(lines),
         'scope': 'traced consumer only; child-preexec denial probes are separate'})
    assert wait_error is None, 'command_wait_' + str(wait_error)
    if kill_fsync is not None:
        assert code != 0 and any(re.fullmatch(r'\d+ \+\+\+ killed by SIGKILL \+\+\+', line) for line in lines)
    else:
        assert code == expected, 'unexpected_exit_' + label
