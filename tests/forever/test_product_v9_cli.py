# SPDX-License-Identifier: GPL-3.0-or-later
"""Shipped product-v9 synthetic CLI acceptance against an explicit executable.

Set RCE_PRODUCT_BINARY and a fresh absolute RCE_PRODUCT_V9_CASES outside Git.
One integrated method performs39 invocations, including2 calibrated SIGKILLs.
Running once for shipped and once for moved-rebuilt binaries gives78 invocations;
these are not debug/release profiles or78 unittest methods. Inputs are authored.
No client assets, private oracles, original consumers or native builds are used.
"""
import json
import os
from pathlib import Path
import re
import shutil
import time
import unittest
import product_v9_fixture as fixture

REPO = Path(__file__).resolve().parents[2]
PLAN = REPO / 'tests/contracts/fixtures/forever-product-v9/disabled.tsv'
SOURCE = fixture.SOURCE
HASH = re.compile('[0-9a-f]{64}')
BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_V9_CASES')
sha = fixture.sha
private_bytes = fixture.private_bytes
inventory = fixture.inventory
package_data = fixture.package_data
rewrite_metadata = fixture.rewrite_metadata
fsync_ordinals = fixture.fsync_ordinals


class Run:
    def __init__(self, root, binary):
        self.root = root
        self.binary = str(binary)
        self.binary_sha256 = sha(private_bytes(binary, 128*1024*1024))
        self.deadline = time.monotonic() + 3600
        self.env = fixture.environment(root)
        self.plan = root / 'plan.tsv'
        self.plan.write_bytes(private_bytes(PLAN, 262144))
        self.call_count = 0
        self.owned = []
        self.retained = {}
        self.calibration = {}
        self.source_before = fixture.source_guards()
        self.guards = dict(self.source_before)
        self.guards[str(binary)] = self.binary_sha256
        self.guards[str(self.plan)] = sha(self.plan.read_bytes())
        self.aliases = {}
        self.client = None
        self.build_metadata = None

    def guard_attempts(self):
        errors=[]
        try:assert {str(path) for path in fixture.source_paths()}==set(self.source_before)
        except Exception as error:errors.append({'path':'source_path_set','error':type(error).__name__})
        for alias,target in self.aliases.items():
            try:assert str(Path(alias).resolve(strict=True))==target
            except Exception as error:errors.append({'path':alias,'error':type(error).__name__})
        for path,wanted in self.guards.items():
            try:
                actual=sha(private_bytes(Path(path),262144)) if self.client is not None and Path(path)==self.client/'.build.info' else sha(private_bytes(Path(path),128*1024*1024))
                assert actual==wanted
            except Exception as error:errors.append({'path':path,'error':type(error).__name__})
        if self.client is not None and self.build_metadata is not None:
            try:assert sha(private_bytes(self.client/'.build.info',262144))==self.build_metadata
            except Exception as error:errors.append({'path':'bounded_build_metadata','error':type(error).__name__})
        return errors
    def invoke(self,profile,command,output=None,plan=None,client=None,expected=0,calibrate=False,kill=None):
        assert not self.guard_attempts(), 'guards_before_command'
        remaining=self.deadline-time.monotonic();assert remaining>0,'phase_cutoff'
        self.call_count+=1;label=f'{self.call_count:03}-{profile}-{command}'
        argv=[self.binary,command,'--plan',str(plan or self.plan)]
        if output is not None:argv+=['--output',str(output)]
        if command in ('extract','resume') or (command=='inspect' and client is not None):argv+=['--client',str(client or self.client)]
        fixture.command(argv,self.root,label,self.env,expected=expected,trace_fsync=calibrate or kill is not None,kill_fsync=kill,timeout_seconds=min(660,remaining))
        result=None
        if kill is None:
            stream=self.root/(label+('.stdout' if expected==0 else '.stderr'))
            result=json.loads(private_bytes(stream,1024*1024))
        return result,self.root/(label+'.network.trace')
    def verify(self,profile,output):
        for command in ('verify','verify-links'):self.invoke(profile,command,output)
    def retained_stage(self,path): self.retained[str(path)]=inventory(path)
    def copy_stage(self,source,name):
        target=self.root/name;shutil.copytree(source,target);self.owned.append(target);return target
    def negatives(self,profile,baseline,pending,other_client):
        wrong_plan=self.root/(profile+'-wrong-plan.tsv');wrong_plan.write_bytes(self.plan.read_bytes().replace(b'pack\ttrue',b'pack\tfalse'))
        self.guards[str(wrong_plan)]=sha(wrong_plan.read_bytes())
        self.invoke(profile,'inspect',plan=wrong_plan)
        for name,changes in [('plan',{'plan':wrong_plan}),('producer',{}),('source',{'client':other_client})]:
            output=self.copy_stage(pending,f'{profile}-wrong-{name}')
            if name=='producer':fixture.rebind_checkpoint_producer(output)
            before=inventory(output)
            options={'profile':profile,'command':'resume','output':output,'expected':9,**changes}
            result,_=self.invoke(**options);assert result['code']=='package_context'
            assert inventory(output)==before;self.retained_stage(output)
        output=self.copy_stage(baseline,profile+'-sealed-source-mismatch')
        (output/'forever-package.json').unlink()
        (output/'.forever-package-incomplete').write_bytes(b'incomplete\n')
        (output/'.state/sealed/cameras').unlink()
        for camera in (output/'cameras').iterdir():camera.unlink()
        assert not (output/'.state/transaction.json').exists()
        payload=output/SOURCE;data=bytearray(payload.read_bytes());data[0]^=1;payload.write_bytes(data)
        def changed_source(facts):
            facts=[f for f in facts if f['component']!='cameras']
            for fact in facts:
                if fact['path']==SOURCE:fact.update(bytes=len(data),sha256=sha(data))
            return facts
        rewrite_metadata(output,changed_source);before=inventory(output)
        result,_=self.invoke(profile,'resume',output,expected=9)
        assert result['code']=='package_inventory' and result['component']=='cameras' and inventory(output)==before
        self.retained_stage(output)
        result,_=self.invoke(profile,'resume',baseline,expected=9);assert result['code']=='package_component_sealed'
        baseline_before=inventory(baseline)
        result,_=self.invoke(profile,'extract',baseline,expected=9)
        assert result['code']=='package_exists' and inventory(baseline)==baseline_before
        for kind in ('camera_file_cap','source_file_cap','gt_file_cap','camera_total_cap','empty','missing_gt','intrinsic_source','tampered_payload'):
            output=self.copy_stage(baseline,profile+'-negative-'+kind)
            def change(facts):
                if kind=='missing_gt':return [f for f in facts if f['path']!=next(f['path'] for f in facts if f['component']=='gt')]
                if kind=='camera_total_cap':
                    facts=[f for f in facts if f['component']!='cameras']
                    facts += [{'component':'cameras','path':f'cameras/FILE{0xf0000000+i:08X}.xxx','bytes':16*1024*1024,'sha256':'0'*64} for i in range(16)]
                    return facts
                for fact in facts:
                    chosen=(kind.startswith('camera_') and fact['component']=='cameras') or (kind=='source_file_cap' and fact['path']==SOURCE) or (kind=='gt_file_cap' and fact['component']=='gt') or (kind=='empty' and fact['component']=='cameras')
                    if chosen:
                        fact['bytes']=0 if kind=='empty' else ((4 if kind=='gt_file_cap' else 16)*1024*1024+1);break
                return facts
            if kind=='intrinsic_source':
                payload=output/SOURCE;data=bytearray(payload.read_bytes());data[152]^=1;payload.write_bytes(data)
                def change(facts):
                    for f in facts:
                        if f['path']==SOURCE:f.update(bytes=len(data),sha256=sha(data))
                    return facts
            if kind=='tampered_payload':next((output/'cameras').iterdir()).write_bytes(b'tampered')
            else:rewrite_metadata(output,change)
            for command in ('verify','verify-links'):
                result,_=self.invoke(profile,command,output,expected=9)
                if kind.endswith('file_cap'):assert result['code']=='product_file_limit'
                if kind=='camera_total_cap':assert result['code']=='camera_policy_limit'
                if kind=='empty':assert result['code']=='product_empty_artifact'
                if kind=='missing_gt':assert result['code']=='product_inventory'
                if kind=='intrinsic_source':assert result['code']=='unsupported_table_schema'
                if kind=='tampered_payload':assert result['code']=='package_digest'
            self.retained_stage(output)

    def execute(self):
        self.client, expected = fixture.fixture(self.root)
        other = self.root / 'other-source'
        other.mkdir(mode=0o700)
        other_client, _ = fixture.fixture(other, changed_gt=True)
        assert len(expected) == 31
        self.build_metadata = sha(private_bytes(self.client / '.build.info', 262144))
        for client in (self.client, other_client):
            for name, fact in inventory(client)['files'].items():
                self.guards[str(client / name)] = fact['sha256']
        profile = 'packaged'
        self.invoke(profile, 'inspect')
        self.invoke(profile, 'inspect', client=self.client)
        baseline = self.root / 'clean'
        self.owned.append(baseline)
        result, trace = self.invoke(profile, 'extract', baseline, calibrate=True)
        assert result['package']['context']['producer_sha256'] == self.binary_sha256
        assert package_data(baseline) == expected
        self.verify(profile, baseline)
        self.calibration[profile] = fsync_ordinals(trace, baseline)
        self.retained_stage(baseline)
        for component in ('gt', 'cameras'):
            output = self.root / ('interrupt-' + component)
            self.owned.append(output)
            self.invoke(profile, 'extract', output, kill=self.calibration[profile][component])
            assert (output / '.forever-package-incomplete').exists()
            assert (output / '.state/sealed/db2').exists()
            assert not (output / '.state/sealed' / component).exists()
            if component == 'cameras':
                assert (output / '.state/sealed/gt').exists()
            db2_seal = (output / '.state/sealed/db2').read_bytes()
            if component == 'gt':
                self.negatives(profile, baseline, output, other_client)
            self.invoke(profile, 'resume', output)
            self.verify(profile, output)
            assert (output / '.state/sealed/db2').read_bytes() == db2_seal
            assert package_data(output) == expected
            assert (output / 'forever-package.json').read_bytes() == (baseline / 'forever-package.json').read_bytes()
            self.retained_stage(output)
        empty = self.root / 'empty-source'
        empty.mkdir(mode=0o700)
        client, expected_empty = fixture.fixture(empty, ())
        for name, fact in inventory(client)['files'].items():
            self.guards[str(client / name)] = fact['sha256']
        output = empty / 'output'
        self.owned.append(output)
        self.invoke(profile, 'extract', output, client=client)
        self.verify(profile, output)
        assert package_data(output) == expected_empty and len(expected_empty) == 21
        assert not (output / 'cameras').exists()
        self.retained_stage(output)
        assert self.call_count == 39, 'authored_command_count'
        return expected


@unittest.skipUnless(BINARY and CASES, 'explicit external synthetic product v9 run required')
class ProductV9Cli(unittest.TestCase):
    def test_complete_extract_resume_intrinsic_verification(self):
        parent = Path(CASES)
        binary = Path(BINARY)
        self.assertTrue(parent.is_absolute() and binary.is_absolute())
        self.assertEqual(parent.resolve(), parent)
        self.assertEqual(binary.resolve(strict=True), binary)
        self.assertFalse(any((path / '.git').exists() for path in (parent, *parent.parents)))
        root = parent / self._testMethodName
        root.mkdir(parents=True, mode=0o700)
        run = Run(root, binary)
        report = {'schema': 'product-v9-packaged-synthetic-v1', 'status': 'running',
                  'unittest_methods': 1, 'binary_sha256': run.binary_sha256}
        error = None
        try:
            expected = run.execute()
            report.update(status='passed', files=len(expected),
                          bytes=sum(fact['bytes'] for fact in expected.values()),
                          calibration=run.calibration, complete_positive_packages=4,
                          retained_negative_copies=12, calibrated_sigkill_cuts=2)
        except BaseException as failure:
            error = failure
            report.update(status='failed', error_type=type(failure).__name__)
        finally:
            report['guard_errors'] = run.guard_attempts()
            report['stage_errors'] = []
            report['partial_output_inventories'] = {}
            for name, wanted in run.retained.items():
                try:
                    assert inventory(Path(name)) == wanted
                except Exception as failure:
                    report['stage_errors'].append({'path': name, 'error_type': type(failure).__name__})
            for output in run.owned:
                try:
                    report['partial_output_inventories'][str(output)] = inventory(output)
                except Exception as failure:
                    report['stage_errors'].append({'path': str(output), 'error_type': type(failure).__name__})
            if report['guard_errors'] or report['stage_errors']:
                report['status'] = 'failed'
            report.update(commands_observed=run.call_count, original_consumers_run=0,
                          traffic_scope='CLI calls; not CASC requests, OS I/O or RSS',
                          producer_negative='authored internally rebound checkpoint; no second executable',
                          runtime_closure_scope='source, authored inputs and executable guarded; external acceptance owns system closure')
            fixture.save(root / 'acceptance.json', report)
        if error is not None:
            raise error
        self.assertEqual(report['status'], 'passed', report)


if __name__ == '__main__':
    unittest.main()
