"""GPL-3.0-or-later. Explicit retained synthetic CLI acceptance, 2026-10-11.

Set RCE_PRODUCT_BINARY and RCE_PRODUCT_CASES outside Git. No real-client inputs.
Every CLI runs with network syscall denial and a retained trace. Outputs survive.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import unittest
import product_fixture as fixture

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_CASES')

@unittest.skipUnless(BINARY and CASES, 'explicit external synthetic product run required')
class ProductCli(unittest.TestCase):
    def setUp(self):
        self.root = Path(CASES)/self._testMethodName
        self.root.mkdir(parents=True, mode=0o700)
        self.assertFalse(any((p/'.git').exists() for p in self.root.resolve().parents))
        self.calls = 0

    def prepare(self, tile=False, components='db2,gt,cameras,maps,vmaps,mmaps', build=70205, locale='enUS', missing=None):
        payloads = fixture.files(tile)
        if missing is not None:
            payloads.pop(missing)
        fixture.store(self.root/'client',payloads,build,locale)
        self.plan = self.root/'plan.tsv'
        self.plan.write_text(fixture.plan(components,build,locale))
        self.client = self.root/'client'
        self.output = self.root/'output'

    def invoke(self, command, options=None, code=0, kill=None):
        self.calls += 1
        if options is None:
            options = ['--plan',str(self.plan),'--output',str(self.output)]
            if command in ('extract','resume'):
                options += ['--client',str(self.client)]
        trace = self.root/f'{self.calls:02}-{command}.network'
        args = ['strace','-f','-qq','-e','trace=network,fsync' if kill is not None else 'trace=network','-e','inject=network:error=ENETUNREACH','-o',str(trace)]
        if kill is not None:
            args += ['-e',f'inject=fsync:signal=SIGKILL:when={kill}']
        args += [BINARY,command,*options]
        result = subprocess.run(args,capture_output=True,text=True,timeout=240)
        (self.root/f'{self.calls:02}-{command}.stdout').write_text(result.stdout)
        (self.root/f'{self.calls:02}-{command}.stderr').write_text(result.stderr)
        if kill is not None:
            self.assertNotEqual(result.returncode,0)
            self.assertIn('SIGKILL',trace.read_text())
        else:
            self.assertEqual(result.returncode,code,result.stderr)
            self.assertEqual(trace.read_text(),'')
        text = result.stdout if code==0 else result.stderr
        return json.loads(text) if kill is None and command!='--help' else None

    def test_all_six_empty_geometry(self):
        self.prepare()
        result=self.invoke('extract')
        self.assertEqual(result['package']['context']['components'],63)
        self.assertFalse(result['full_extractor_complete'])
        verified=self.invoke('verify')
        self.assertEqual(result['package'],verified['package'])
        self.assertEqual(verified['package']['files'],28)

    def test_all_six_actual_navigation_tile(self):
        self.prepare(tile=True)
        self.invoke('extract'); self.invoke('verify')
        self.assertTrue(list((self.output/'maps').glob('*.map')))
        self.assertTrue(list((self.output/'mmaps').glob('*.mmtile')))

    def test_complete_empty_map_world(self):
        payloads=fixture.files();payloads[fixture.IDS['Map']]=fixture.table('Map')
        fixture.store(self.root/'client',payloads)
        self.plan=self.root/'plan.tsv';self.plan.write_text(fixture.plan())
        self.client=self.root/'client';self.output=self.root/'output'
        self.invoke('extract');self.invoke('verify')
        self.assertFalse((self.output/'maps').exists())
        self.assertFalse((self.output/'mmaps').exists())

    def test_70170_esES_subset(self):
        self.prepare(components='db2,gt,cameras',build=70170,locale='esES')
        self.invoke('extract'); report=self.invoke('verify')
        self.assertEqual(report['package']['context']['build'],70170)
        self.assertEqual(report['package']['context']['locale'],'esES')

    def test_inspect_opens_no_output(self):
        self.prepare()
        result=self.invoke('inspect',['--plan',str(self.plan)])
        self.assertEqual(result['status'],'planned')
        result=self.invoke('inspect',['--plan',str(self.plan),'--client',str(self.client)])
        self.assertEqual(result['status'],'admitted')
        self.assertFalse(result['availability_checked']); self.assertFalse(self.output.exists())

    def test_byte_determinism(self):
        self.prepare(components='db2,gt,cameras')
        self.invoke('extract')
        first=(self.output/'forever-package.json').read_bytes()
        self.output=self.root/'second-output'; self.invoke('extract')
        self.assertEqual(first,(self.output/'forever-package.json').read_bytes())

    def test_real_sigkill_resume(self):
        self.prepare(components='db2,gt,cameras')
        self.invoke('extract',kill=25)
        self.assertTrue((self.output/'.forever-package-incomplete').exists())
        self.invoke('verify',code=8)
        self.invoke('resume'); self.invoke('verify')
        recovered=(self.output/'forever-package.json').read_bytes()
        self.output=self.root/'uninterrupted'; self.invoke('extract')
        self.assertEqual(recovered,(self.output/'forever-package.json').read_bytes())

    def test_resume_wrong_plan_and_complete_output(self):
        self.prepare(components='db2,gt,cameras')
        self.invoke('extract',kill=25)
        original=self.plan.read_text(); self.plan.write_text(original.replace('pack\ttrue','pack\tfalse'))
        report=self.invoke('resume',code=9); self.assertEqual(report['code'],'package_context')
        self.plan.write_text(original);self.invoke('resume')
        report=self.invoke('resume',code=9);self.assertEqual(report['code'],'package_component_sealed')

    def test_resume_skips_verified_finished_db2(self):
        self.prepare(components='db2,gt,cameras')
        self.output=self.root/'baseline';self.invoke('extract')
        baseline=(self.output/'forever-package.json').read_bytes()
        self.output=self.root/'output';self.invoke('extract',kill=70)
        self.assertTrue((self.output/'.state/sealed/db2').exists())
        self.assertFalse((self.output/'.state/sealed/gt').exists())
        # Only our private toy archive is mutated. Logical ROOT/ENCODING keys
        # stay fixed. A rerun of the finished DB2 producer would now fail its
        # complete CKey check; GT/camera source regions remain unchanged.
        path=self.client/'Data/data/data.000';archive=bytearray(path.read_bytes())
        needle=fixture.files()[fixture.IDS['LiquidType']]
        self.assertEqual(archive.count(needle),1)
        at=archive.index(needle);archive[at]^=1;path.write_bytes(archive)
        self.invoke('resume');self.invoke('verify')
        self.assertEqual(baseline,(self.output/'forever-package.json').read_bytes())

    def test_missing_source_retains_failure(self):
        self.prepare(missing=fixture.IDS['LiquidType'])
        result=self.invoke('extract',code=9)
        self.assertEqual(result['component'],'db2')
        self.assertTrue((self.output/'.forever-package-incomplete').exists())
        self.invoke('verify',code=8)

    def test_changed_payload_rejected(self):
        self.prepare(components='db2,gt,cameras')
        self.invoke('extract')
        path=next((self.output/'cameras').iterdir());path.write_bytes(b'tampered')
        self.invoke('verify',code=9)

    def test_wrong_build_before_output(self):
        self.prepare();self.plan.write_text(self.plan.read_text().replace('build\t70205','build\t70170'))
        report=self.invoke('extract',code=9)
        self.assertEqual(report['code'],'product_build_mismatch');self.assertFalse(self.output.exists())

    def test_client_output_boundary(self):
        self.prepare();self.output=self.client/'forbidden-output'
        report=self.invoke('extract',code=9)
        self.assertEqual(report['code'],'package_path');self.assertFalse(self.output.exists())

    def test_bad_plan_and_unknown_options(self):
        self.prepare();self.plan.write_text(self.plan.read_text()+'network\ttrue\n')
        self.invoke('extract',code=2)
        self.invoke('inspect',['--plan',str(self.plan),'--network','true'],code=2)
        self.assertFalse(self.output.exists())

    def test_fifo_plan_rejects_without_blocking(self):
        self.plan=self.root/'fifo';os.mkfifo(self.plan)
        report=self.invoke('inspect',['--plan',str(self.plan)],code=9)
        self.assertEqual(report['code'],'product_input_type')

    def test_plan_symlink_rejects(self):
        self.prepare(); alias=self.root/'alias.tsv';alias.symlink_to(self.plan)
        self.invoke('inspect',['--plan',str(alias)],code=9)

    def test_expected_context_binding(self):
        self.prepare(components='db2,gt,cameras');self.invoke('extract')
        context=(self.output/'.state/context.json').read_text()
        expected=self.root/'context.json';expected.write_text(context)
        args=['--plan',str(self.plan),'--output',str(self.output),'--expect-context',str(expected)]
        self.invoke('verify',args)
        context=json.loads(context);context['source_sha256']='01'*32
        expected.write_text(json.dumps(context,separators=(',',':')))
        self.invoke('verify',args,code=9)

if __name__=='__main__':
    unittest.main()
