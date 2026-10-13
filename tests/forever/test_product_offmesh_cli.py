"""GPL-3.0-or-later. Explicit retained synthetic product v6 acceptance.

RCE_PRODUCT_BINARY, RCE_PRODUCT_OFFMESH_CASES, RCE_PRODUCT_OFFMESH_NATIVE
must select external frozen executables and a new private corpus. Every child
runs under network syscall denial. Complete toy positives reach native only
AFTER product extract and separate verify pass. No real client inputs.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import unittest
import product_offmesh_fixture as fixture
import product_disk_fixture as disk
import product_keys_fixture as encrypted

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_OFFMESH_CASES')
NATIVE = os.environ.get('RCE_PRODUCT_OFFMESH_NATIVE')
PIPELINE = os.environ.get('RCE_PRODUCT_OFFMESH_PIPELINE')
sha = lambda data: hashlib.sha256(data).hexdigest()
compact = lambda value: json.dumps(value, separators=(',', ':'))


def repair_integrity(output):
    """Rebuild authored package integrity facts to reach semantic inspection.

    Does not change context, plan, locks or protocol. This deliberately forges
    integrity metadata for a tampered SYNTHETIC artifact; no authenticity claim.
    """
    path = output / 'forever-package.json'
    old = json.loads(path.read_text())
    artifacts = old['artifacts']
    for entry in artifacts:
        data = (output / entry['path']).read_bytes()
        entry['bytes'], entry['sha256'] = len(data), sha(data)
        receipt = output / '.state/files' / (sha(entry['path'].lower().encode()) + '.json')
        receipt.write_text(compact(entry))
    context = compact(old['context'])
    for seal in (output / '.state/sealed').iterdir():
        selected = [a for a in artifacts if a['component'] == seal.name]
        seal.write_text(compact({'component': seal.name, 'context_sha256': sha(context.encode()),
                                 'files': len(selected), 'bytes': sum(a['bytes'] for a in selected),
                                 'inventory_sha256': sha(''.join(compact(a)+'\n' for a in selected).encode())}))
    digest = sha(''.join(compact(a)+'\n' for a in artifacts).encode())
    text = ('{"manifest_version":1,"scope":"artifact_integrity","status":"complete","context":' +
            context + ',"artifacts":[\n' + ',\n'.join(compact(a) for a in artifacts) + '\n' +
            '],"file_count":' + str(len(artifacts)) + ',"total_bytes":' +
            str(sum(a['bytes'] for a in artifacts)) + ',"inventory_sha256":"' + digest + '"}\n')
    path.write_text(text)


@unittest.skipUnless(BINARY and CASES and NATIVE, 'explicit external synthetic offmesh run required')
class ProductOffmeshCli(unittest.TestCase):
    def setUp(self):
        os.umask(0o077)
        self.root = Path(CASES) / self._testMethodName
        self.root.mkdir(parents=True, mode=0o700)
        self.assertFalse(any((p / '.git').exists() for p in self.root.resolve().parents))
        self.calls = []
        self.results = []
        self.pipeline_parity = False
        self.binary_before = sha(Path(BINARY).read_bytes())
        self.native_before = sha(Path(NATIVE).read_bytes())
        self.client = self.root / 'client'
        self.plan = self.root / 'plan.tsv'
        self.output = self.root / 'output'
        self.keys = None
        self.scratch = None

    def tearDown(self):
        report = {'binary_sha256': self.binary_before, 'native_sha256': self.native_before,
                  'binary_unchanged': self.binary_before == sha(Path(BINARY).read_bytes()),
                  'native_unchanged': self.native_before == sha(Path(NATIVE).read_bytes()),
                  'commands': self.calls, 'results': self.results,
                  'synthetic_only': True, 'native_malformed_execution': False,
                  'original_tilebuilder_byte_parity': self.pipeline_parity, 'real_client_acceptance': False}
        (self.root / 'case-report.json').write_text(json.dumps(report, indent=2) + '\n')
        self.assertTrue(report['binary_unchanged'] and report['native_unchanged'])

    def prepare(self, connections=(), build=70205, locale='enUS'):
        fixture.store(self.client, build, locale)
        self.plan.write_text(fixture.plan(connections, build, locale))
        self.source_hashes = {str(p.relative_to(self.client)): sha(p.read_bytes())
                              for p in self.client.rglob('*') if p.is_file()}
        (self.root / 'synthetic-input-hashes.json').write_text(json.dumps(self.source_hashes, indent=2)+'\n')

    def invoke(self, command, code=0, kill=None, options=None, executable=BINARY):
        ordinal = len(self.calls) + 1
        stem = self.root / f'{ordinal:03}-{command}'
        trace = Path(str(stem) + '.network')
        argv = ['strace', '-f', '-qq', '-e', 'trace=network,fsync' if kill else 'trace=network',
                '-e', 'inject=network:error=ENETUNREACH', '-o', str(trace)]
        if kill:
            argv += ['-e', f'inject=fsync:signal=SIGKILL:when={kill}']
        if options is None:
            options = ['--plan', str(self.plan)]
            if command in ('extract', 'resume'):
                options += ['--client', str(self.client), '--output', str(self.output)]
                if self.keys:
                    options += ['--keys', str(self.keys)]
                if self.scratch:
                    options += ['--scratch', str(self.scratch)]
            elif command == 'verify':
                options += ['--output', str(self.output)]
        argv += [executable] + ([command] if executable == BINARY else []) + options
        if self.plan.exists():
            Path(str(stem) + '.plan.tsv').write_bytes(self.plan.read_bytes())
        result = subprocess.run(argv, capture_output=True, text=True, timeout=360)
        Path(str(stem)+'.stdout').write_text(result.stdout)
        Path(str(stem)+'.stderr').write_text(result.stderr)
        self.calls.append({'argv': argv, 'exit_code': result.returncode,
                           'stdout_sha256': sha(result.stdout.encode()), 'stderr_sha256': sha(result.stderr.encode()),
                           'trace_sha256': sha(trace.read_bytes()), 'trace_bytes': trace.stat().st_size,
                           'expected_sigkill': bool(kill)})
        if kill:
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('SIGKILL', trace.read_text())
            self.assertNotIn('ENETUNREACH', trace.read_text())
            return None
        self.assertEqual(trace.read_text(), '')
        if code is None:
            self.assertNotEqual(result.returncode, 0)
        else:
            self.assertEqual(result.returncode, code, result.stderr)
        data = json.loads(result.stdout if result.returncode == 0 else result.stderr)
        self.results.append({'command': command, 'summary': data})
        return data

    def complete(self):
        extracted = self.invoke('extract')
        verified = self.invoke('verify')
        self.assertEqual(extracted['package'], verified['package'])
        self.assertEqual(extracted['contract'], 'forever-product-v6')
        self.assertEqual(extracted['offmesh'], 'inline-f32-bits-v1')
        self.assertEqual(extracted['package']['context']['producer_sha256'], self.binary_before)
        self.assertEqual(self.source_hashes, {str(p.relative_to(self.client)): sha(p.read_bytes())
                         for p in self.client.rglob('*') if p.is_file()})
        return extracted

    def global_bytes(self):
        return (self.output / f'mmaps/{fixture.MAP:04}.mmap').read_bytes()

    def assert_global_records(self, records):
        encoded = []
        for c in records:
            encoded.append(struct.pack('<3I7I2BH', *(int(x) for x in c[:3]),
                           *(int(x,16) for x in c[3:10]), *(int(x) for x in c[10:])))
        data = self.global_bytes()
        self.assertEqual(struct.unpack_from('<I', data, 36)[0], len(records))
        self.assertEqual(data[40:], b''.join(encoded))

    def query(self, forward, reverse, count):
        q = self.root / 'query-source-xyz.bin'
        q.write_bytes(struct.pack('<6f', *fixture.FROM, *fixture.TO))
        result = self.invoke('native-query', executable=NATIVE, options=[
            str(self.output / f'mmaps/{fixture.MAP:04}.mmap'),
            str(self.output / f'mmaps/{fixture.MAP:04}_{fixture.ROW:02}_{fixture.COLUMN:02}.mmtile'), str(q)])
        self.assertEqual(result['stored_connections'], count)
        for name, expected in [('forward', forward), ('reverse', reverse)]:
            self.assertEqual(result[name]['reached'], expected)
            self.assertEqual(result[name]['offmesh_polys'], 1 if expected else 0)

    def test_empty_oneway_bidirectional_original_detour(self):
        self.prepare()
        for name, records, forward, reverse in [
                ('empty', [], False, False),
                ('oneway', [fixture.connection(0)], True, False),
                ('bidirectional', [fixture.connection(1)], True, True)]:
            with self.subTest(name=name):
                self.plan.write_text(fixture.plan(records))
                self.output = self.root / name
                self.complete(); self.assert_global_records(records)
                self.query(forward, reverse, len(records))

    def test_70170_esES_direction_and_duplicates_order(self):
        records = [fixture.connection(0), fixture.connection(0, radius=5.0), fixture.connection(0)]
        self.prepare(records, build=70170, locale='esES')
        self.complete(); self.assert_global_records(records)
        self.query(True, False, 3)

    def test_real_sigkill_resume_exact_and_changed_context(self):
        records = [fixture.connection(0), fixture.connection(0, radius=5.0)]
        self.prepare(records)
        original = self.plan.read_text()
        self.invoke('extract', kill=25)
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.invoke('verify', code=8)
        changes = []
        changed = [c.copy() for c in records]; changed[0][3] = fixture.bits(-283.3334)
        changes.append(changed)
        changed = [c.copy() for c in records]; changed[0][10] = '1'
        changes.append(changed)
        changes.append(list(reversed(records)))
        for changed in changes:
            self.plan.write_text(fixture.plan(changed))
            result = self.invoke('resume', code=9)
            self.assertEqual(result['code'], 'package_context')
        self.plan.write_text(original)
        self.invoke('resume'); self.invoke('verify')
        first = (self.output / 'forever-package.json').read_bytes()
        inventory = json.loads(first)['artifacts']
        self.output = self.root / 'uninterrupted'
        self.complete()
        self.assertEqual(first, (self.output / 'forever-package.json').read_bytes())
        for fact in inventory:
            self.assertEqual(sha((self.output/fact['path']).read_bytes()), fact['sha256'])

    def test_reconstructed_global_tamper_reaches_semantics(self):
        records = [fixture.connection(0), fixture.connection(0, radius=5.0)]
        self.prepare(records); self.complete()
        original_output = self.output
        for name, offset, value in [('direction',80,1), ('radius',76,None), ('order',None,None)]:
            self.output = self.root / ('tampered-' + name)
            shutil.copytree(original_output, self.output)
            path = self.output / f'mmaps/{fixture.MAP:04}.mmap'
            data = bytearray(path.read_bytes())
            if name == 'order':
                data[40:84], data[84:128] = data[84:128], data[40:84]
            elif name == 'radius':
                struct.pack_into('<f',data,offset,6.0)
            else:
                data[offset] = value
            path.write_bytes(data); repair_integrity(self.output)
            result = self.invoke('verify', code=9)
            self.assertEqual(result['code'], 'product_offmesh_mismatch')

    def test_provided_keys_and_leased_disk_with_resume(self):
        records = [fixture.connection(0)]
        for name, supplied, leased in [('resident-key', True, False),
                                        ('disk-disabled', False, True),
                                        ('disk-key', True, True)]:
            with self.subTest(name=name):
                self.client = self.root / (name + '-client')
                self.output = self.root / (name + '-output')
                self.keys = self.scratch = None
                if supplied:
                    encrypted.store(self.client, fixture.files())
                    self.keys = self.root / (name + '-demo-keys.tsv')
                    self.keys.write_text(encrypted.keyfile())
                else:
                    fixture.store(self.client)
                text = fixture.plan(records)
                if supplied:
                    text = text.replace('decryption\tdisabled', 'decryption\tprovided-salsa20-v1')
                if leased:
                    self.scratch = self.root / (name + '-scratch')
                    self.scratch.mkdir(mode=0o700)
                    marker = 'decryption\t' + ('provided-salsa20-v1' if supplied else 'disabled') + '\n'
                    text = text.replace(marker, marker +
                        'collision-storage\tlinux-leased-append-disk-world-v1\n' +
                        ''.join(f'collision-{field}-bytes\t{disk.BUDGET}\n' for field in disk.FIELDS))
                self.plan.write_text(text)
                self.source_hashes = {str(p.relative_to(self.client)): sha(p.read_bytes())
                                      for p in self.client.rglob('*') if p.is_file()}
                (self.root / (name + '-source-hashes.json')).write_text(
                    json.dumps(self.source_hashes, indent=2) + '\n')
                self.complete(); self.assert_global_records(records); self.query(True, False, 1)
                if supplied and leased:
                    baseline = (self.output / 'forever-package.json').read_bytes()
                    self.output = self.root / (name + '-resumed')
                    self.invoke('extract', kill=25)
                    self.assertTrue((self.output / '.forever-package-incomplete').exists())
                    self.invoke('verify', code=8)
                    self.invoke('resume'); self.invoke('verify')
                    self.assertEqual(baseline, (self.output / 'forever-package.json').read_bytes())
                    for entry in json.loads(baseline)['artifacts']:
                        self.assertEqual(sha((self.output / entry['path']).read_bytes()), entry['sha256'])
                    self.query(True, False, 1)

    @unittest.skipUnless(PIPELINE, 'explicit original TileBuilder executable required')
    def test_original_tilebuilder_exact_oneway(self):
        records = [fixture.connection(0)]
        self.prepare(records); self.complete(); self.assert_global_records(records)
        source = self.root / 'native-input'
        source.mkdir(mode=0o700)
        for name in ('maps', 'vmaps'):
            shutil.copytree(self.output / name, source / name)
        floats = [struct.unpack('<f', struct.pack('<I', int(v,16)))[0] for v in records[0][3:10]]
        row = [*records[0][:3], *(format(v, '.9g') for v in floats), *records[0][10:]]
        (source / 'plan.txt').write_text('1 7 32 32 0 70205 0 55 55 0 1\n'
                                        '7 4294967295 4294967295 3\n' + ' '.join(row) + '\n')
        guarded = [Path(PIPELINE), self.output / 'mmaps/0007.mmap',
                   self.output / 'mmaps/0007_32_32.mmtile'] + sorted(
                       p for p in source.rglob('*') if p.is_file())
        before = {str(p): sha(p.read_bytes()) for p in guarded}
        (self.root / 'tilebuilder-input-guards.json').write_text(json.dumps(before, indent=2) + '\n')
        trace = self.root / 'native-tilebuilder.network'
        argv = ['strace','-f','-qq','-e','trace=network','-e',
                'inject=network:error=ENETUNREACH','-o',str(trace),PIPELINE,str(source),
                str(self.output / 'mmaps/0007.mmap'),str(self.root / 'native-output')]
        result = subprocess.run(argv,capture_output=True,timeout=360)
        (self.root / 'native-tilebuilder.stdout').write_bytes(result.stdout)
        (self.root / 'native-tilebuilder.stderr').write_bytes(result.stderr)
        self.calls.append({'argv':argv,'exit_code':result.returncode,
                           'stdout_sha256':sha(result.stdout),'stderr_sha256':sha(result.stderr),
                           'trace_sha256':sha(trace.read_bytes()),'trace_bytes':trace.stat().st_size,
                           'expected_sigkill':False,'pipeline_binary_sha256':sha(Path(PIPELINE).read_bytes())})
        self.assertEqual(result.returncode,0,result.stderr.decode(errors='replace'))
        self.assertEqual(trace.read_text(),'')
        self.assertEqual(before, {str(p): sha(p.read_bytes()) for p in guarded})
        native = self.root / 'native-output/mmaps/0007_32_32.mmtile'
        self.assertEqual(native.read_bytes(),(self.output / 'mmaps/0007_32_32.mmtile').read_bytes())
        self.pipeline_parity = True

    def test_plan_maximum_and_db2_tail(self):
        self.prepare()
        record = fixture.connection(start=(-0.0, fixture.FROM[1], fixture.FROM[2]))
        records = [record] * 1024
        self.plan.write_text(fixture.plan(records, components='db2,maps,vmaps,mmaps'))
        self.assertLessEqual(self.plan.stat().st_size, 128*1024)
        result = self.invoke('inspect')
        self.assertEqual(result['contract'], 'forever-product-v6')
        self.assertEqual(result['offmesh_connections'], 1024)
        self.assertEqual(result['status'], 'planned')
        self.assertEqual(result['plan_sha256'], sha(self.plan.read_bytes()))
        self.assertFalse(self.output.exists())

    def test_malformed_admission_before_output(self):
        self.prepare([fixture.connection()])
        good = fixture.connection()
        mutations = [('map-leading-zero',0,'07'), ('row',1,'64'), ('column',2,'64'),
                     ('bits-short',3,'0000000'), ('bits-uppercase',3,'BF800000'),
                     ('nan',3,'7fc00001'), ('infinity',6,'7f800000'),
                     ('zero-radius',9,'00000000'), ('negative-radius',9,'bf800000'),
                     ('direction',10,'2'), ('area',11,'64'), ('flags',12,'65536')]
        for name, index, value in mutations:
            with self.subTest(name=name):
                record = good.copy();record[index]=value
                self.plan.write_text(fixture.plan([record]));self.output=self.root/('bad-'+name)
                self.invoke('extract',code=2);self.assertFalse(self.output.exists())
        for name,text in [('missing-header',fixture.plan([good]).replace('offmesh\tinline-f32-bits-v1\n','')),
                          ('wrong-header',fixture.plan().replace('inline-f32-bits-v1','unknown')),
                          ('bad-build',fixture.plan().replace('build\t70205','build\t70338')),
                          ('no-navigation',fixture.plan(components='maps,vmaps')),
                          ('too-many',fixture.plan([good]*1025))]:
            with self.subTest(name=name):
                self.plan.write_text(text);self.output=self.root/('bad-'+name)
                self.invoke('extract',code=None);self.assertFalse(self.output.exists())


if __name__ == '__main__':
    unittest.main()
