"""GPL-3.0-or-later. Explicit Linux synthetic output-link CLI acceptance.

RCE_PRODUCT_BINARY and RCE_PRODUCT_LINKS_CASES select a frozen executable and a
new external private corpus. One complete all-six toy extraction is shared;
each case owns a copied package. Deliberately reconstructed integrity metadata
is synthetic verifier input, never producer/authenticity or client evidence.
All child commands deny network syscalls and retain complete command evidence.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import unittest

import product_fixture as plain
import product_offmesh_fixture as terrain
import product_keys_fixture as keys
import product_disk_fixture as disk
import product_catalog_fixture as catalog

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_PRODUCT_LINKS_CASES')
PROFILE = 'forever-terrain-navigation-links-v1'
SCOPE = 'artifact_integrity_intrinsic_formats_and_terrain_navigation_links'
SOURCE = Path(__file__).resolve().parents[2]
COMPONENTS = ('db2', 'gt', 'cameras', 'maps', 'vmaps', 'mmaps')
sha = lambda value: hashlib.sha256(value).hexdigest()
compact = lambda value: json.dumps(value, separators=(',', ':'))


def hashes(root):
    return {p.relative_to(root).as_posix(): sha(p.read_bytes())
            for p in sorted(root.rglob('*')) if p.is_file()}


def source_hashes():
    paths = [SOURCE / name for name in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml')]
    for name in ('crates', 'contracts', 'tests'):
        paths.extend(p for p in (SOURCE / name).rglob('*')
                     if p.is_file() and '__pycache__' not in p.parts)
    return {p.relative_to(SOURCE).as_posix(): sha(p.read_bytes()) for p in sorted(paths) if p.exists()}


def reconstruct(output, plan=None, components=None):
    """Author a fully consistent complete package after controlled toy edits.

    Rebuild receipts, component seals, inventory and context. This allows old
    intrinsic verification to pass before the new relationship check rejects.
    Rebinding plans is verifier-only input; no new extraction is inferred.
    """
    path = output / 'forever-package.json'
    context = json.loads(path.read_text())['context']
    if plan is not None:
        context['plan_sha256'] = sha(plan.encode())
    if components is not None:
        context['components'] = sum(1 << COMPONENTS.index(c) for c in components)
    selected = [c for i, c in enumerate(COMPONENTS) if context['components'] & (1 << i)]
    artifacts = []
    for component in COMPONENTS:
        folder = output / ('dbc' if component == 'db2' else component)
        if component not in selected:
            if folder.exists():
                shutil.rmtree(folder)
            continue
        if folder.exists():
            for p in folder.rglob('*'):
                if p.is_file():
                    data = p.read_bytes()
                    artifacts.append({'component': component, 'path': p.relative_to(output).as_posix(),
                                      'bytes': len(data), 'sha256': sha(data)})
    artifacts.sort(key=lambda a: a['path'].lower())
    for folder in ('files', 'sealed'):
        for p in (output / '.state' / folder).iterdir():
            p.unlink()
    context_text = compact(context)
    (output / '.state/context.json').write_text(context_text)
    protocol_path = output / '.state/protocol.json'
    if protocol_path.exists():
        protocol = json.loads(protocol_path.read_text())
        protocol['context_sha256'] = sha(context_text.encode())
        protocol_path.write_text(compact(protocol))
    for entry in artifacts:
        (output / '.state/files' / (sha(entry['path'].lower().encode()) + '.json')).write_text(compact(entry))
    for component in selected:
        subset = [a for a in artifacts if a['component'] == component]
        seal = {'component': component, 'context_sha256': sha(context_text.encode()),
                'files': len(subset), 'bytes': sum(a['bytes'] for a in subset),
                'inventory_sha256': sha(''.join(compact(a) + '\n' for a in subset).encode())}
        (output / '.state/sealed' / component).write_text(compact(seal))
    text = ('{"manifest_version":1,"scope":"artifact_integrity","status":"complete","context":' +
            context_text + ',"artifacts":[\n' + ',\n'.join(compact(a) for a in artifacts) + ('\n' if artifacts else '') +
            '],"file_count":' + str(len(artifacts)) + ',"total_bytes":' +
            str(sum(a['bytes'] for a in artifacts)) + ',"inventory_sha256":"' +
            sha(''.join(compact(a) + '\n' for a in artifacts).encode()) + '"}\n')
    path.write_text(text)


def tilelist(points=(), build=70205):
    data = bytearray(b'MAPS' + struct.pack('<II', 10, build) + b'0' * 4096)
    for x, y in points:
        data[12 + 4095 - (y * 64 + x)] = ord('1')
    return bytes(data)


@unittest.skipUnless(BINARY and CASES, 'explicit external Linux synthetic link run required')
class ProductLinksCli(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        os.umask(0o077)
        cls.corpus = Path(CASES).resolve()
        cls.corpus.mkdir(mode=0o700, parents=True)
        if any((p / '.git').exists() for p in (cls.corpus, *cls.corpus.parents)):
            raise AssertionError('corpus must be outside Git')
        cls.binary_before = sha(Path(BINARY).read_bytes())
        cls.source_before = source_hashes()
        cls.base = cls.corpus / '_base'
        cls.base.mkdir(mode=0o700)
        payloads = disk.files()
        payloads[201] = terrain.files()[201]
        plain.store(cls.base / 'client', payloads)
        cls.base_plan = plain.plan()
        (cls.base / 'plan.tsv').write_text(cls.base_plan)
        cls.inputs_before = hashes(cls.base / 'client')
        cls.base_calls = []
        cls.command(cls.base, cls.base_calls, 'extract', ['--plan', str(cls.base / 'plan.tsv'),
                    '--client', str(cls.base / 'client'), '--output', str(cls.base / 'output')])
        cls.command(cls.base, cls.base_calls, 'verify', ['--plan', str(cls.base / 'plan.tsv'),
                    '--output', str(cls.base / 'output')])
        if not list((cls.base / 'output/mmaps').glob('*.mmtile')):
            raise AssertionError('shared toy must contain a real synthetic navigation tile')
        cls.base_outputs = hashes(cls.base / 'output')

    @classmethod
    def tearDownClass(cls):
        report = {'binary_sha256': cls.binary_before,
                  'binary_unchanged': cls.binary_before == sha(Path(BINARY).read_bytes()),
                  'source_unchanged': cls.source_before == source_hashes(),
                  'inputs_unchanged': cls.inputs_before == hashes(cls.base / 'client'),
                  'base_package_unchanged': cls.base_outputs == hashes(cls.base / 'output'),
                  'commands': cls.base_calls, 'synthetic_only': True,
                  'real_client_acceptance': False, 'full_extractor_complete': False}
        (cls.corpus / 'suite-report.json').write_text(json.dumps(report, indent=2) + '\n')
        (cls.corpus / 'source-hashes.json').write_text(json.dumps(cls.source_before, indent=2) + '\n')
        (cls.corpus / 'input-hashes.json').write_text(json.dumps(cls.inputs_before, indent=2) + '\n')
        if not all(report[k] for k in ('binary_unchanged', 'source_unchanged', 'inputs_unchanged', 'base_package_unchanged')):
            raise AssertionError('frozen evidence changed')

    @classmethod
    def command(cls, root, calls, command, options, code=0, parse=True):
        stem = root / f'{len(calls) + 1:03}-{command.lstrip("-")}'
        trace = Path(str(stem) + '.network')
        argv = ['strace', '-f', '-qq', '-e', 'trace=network', '-e',
                'inject=network:error=ENETUNREACH', '-o', str(trace), BINARY, command, *options]
        result = subprocess.run(argv, capture_output=True, text=True, timeout=360)
        Path(str(stem) + '.stdout').write_text(result.stdout)
        Path(str(stem) + '.stderr').write_text(result.stderr)
        calls.append({'argv': argv, 'exit_code': result.returncode,
                      'stdout_sha256': sha(result.stdout.encode()), 'stderr_sha256': sha(result.stderr.encode()),
                      'trace_sha256': sha(trace.read_bytes()), 'trace_bytes': trace.stat().st_size})
        if trace.read_bytes() != b'':
            raise AssertionError('network syscall observed')
        if result.returncode != code:
            raise AssertionError(f'expected {code}, received {result.returncode}: {result.stderr}')
        return json.loads(result.stdout if code == 0 else result.stderr) if parse else result.stdout

    def setUp(self):
        self.root = self.corpus / self._testMethodName
        self.root.mkdir(mode=0o700)
        self.output = self.root / 'output'
        shutil.copytree(self.base / 'output', self.output)
        self.plan = self.root / 'plan.tsv'
        self.plan.write_text(self.base_plan)
        self.calls = []
        self.results = []

    def tearDown(self):
        (self.root / 'case-report.json').write_text(json.dumps({
            'commands': self.calls, 'results': self.results, 'synthetic_only': True,
            'producer_authenticity_claim': False, 'real_client_acceptance': False}, indent=2) + '\n')

    def invoke(self, command='verify-links', code=0, extra=(), options=None):
        if options is None:
            options = ['--plan', str(self.plan), '--output', str(self.output), *extra]
        result = self.command(self.root, self.calls, command, options, code)
        self.results.append({'command': command, 'summary': result})
        return result

    def positive(self):
        old = self.invoke('verify')
        linked = self.invoke()
        self.assertEqual(old['scope'], 'artifact_integrity_and_intrinsic_formats')
        self.assertNotIn('verification_profile', old)
        self.assertEqual(linked['verification_profile'], PROFILE)
        self.assertEqual(linked['scope'], SCOPE)
        self.assertFalse(linked['full_extractor_complete'])
        self.assertEqual(linked['package'], old['package'])
        return linked

    def negative(self, error, component='maps'):
        reconstruct(self.output)
        self.invoke('verify')
        result = self.invoke(code=9)
        self.assertEqual(result['code'], error)
        self.assertEqual(result['component'], component)
        self.assertFalse(result['full_extractor_complete'])

    def maps_only(self, selection='all'):
        text = plain.plan('maps').replace('terrain\tall', 'terrain\t' + selection)
        self.plan.write_text(text)
        reconstruct(self.output, text, ['maps'])

    def test_complete_six_component_package(self):
        before = hashes(self.output)
        self.positive()
        self.assertEqual(before, hashes(self.output))

    def test_maps_without_tilelist_reject(self):
        (self.output / 'maps/0007.tilelist').unlink()
        self.negative('product_links_terrain_missing_list')

    def test_nonempty_tilelist_without_maps_reject(self):
        for p in (self.output / 'maps').glob('*.map'):
            p.unlink()
        self.negative('product_links_terrain_tile_set')

    def test_list_omitting_published_maps_reject(self):
        (self.output / 'maps/0007.tilelist').write_bytes(tilelist())
        self.negative('product_links_terrain_tile_set')

    def test_list_claiming_unpublished_maps_reject(self):
        (self.output / 'maps/0007.tilelist').write_bytes(tilelist([(32, 32), (4, 19)]))
        self.negative('product_links_terrain_tile_set')

    def test_navigation_tile_without_global_reject(self):
        (self.output / 'mmaps/0007.mmap').unlink()
        self.negative('product_links_navigation_missing_global', 'mmaps')

    def test_empty_tilelist_without_maps_accepts(self):
        self.maps_only()
        for p in (self.output / 'maps').glob('*.map'):
            p.unlink()
        (self.output / 'maps/0007.tilelist').write_bytes(tilelist())
        reconstruct(self.output)
        self.positive()

    def test_global_without_navigation_tiles_accepts(self):
        for p in (self.output / 'mmaps').glob('*.mmtile'):
            p.unlink()
        reconstruct(self.output)
        self.positive()

    def test_vmap_relationship_completeness_is_outside_profile(self):
        # Deliberately omit a complete map's VMap artifacts while retaining the
        # intrinsically valid dtree. This profile establishes neither VMap
        # association completeness nor unobserved client-map completeness.
        shutil.rmtree(self.output / 'vmaps/0007')
        reconstruct(self.output)
        self.positive()

    def test_combined_map_metadata_ceiling_rejects_8193_ids(self):
        self.maps_only()
        shutil.rmtree(self.output / 'maps')
        (self.output / 'maps').mkdir(mode=0o700)
        empty = tilelist()
        for map_id in range(8193):
            (self.output / f'maps/{map_id:04}.tilelist').write_bytes(empty)
        self.negative('product_links_map_limit')

    def test_asymmetric_reversed_tile_bit_order(self):
        self.maps_only()
        old = self.output / 'maps/0007_32_32.map'
        data = old.read_bytes()
        old.unlink()
        points = [(2, 61), (47, 9), (0, 63), (63, 0)]
        for x, y in points:
            (self.output / f'maps/0007_{y:02}_{x:02}.map').write_bytes(data)
        path = self.output / 'maps/0007.tilelist'
        path.write_bytes(tilelist(points))
        reconstruct(self.output)
        self.positive()
        wrong = bytearray(tilelist())
        for x, y in points:
            wrong[12 + y * 64 + x] = ord('1')
        path.write_bytes(wrong)
        self.negative('product_links_terrain_tile_set')

    def test_map_selection_and_empty_unobserved_map_accept(self):
        self.maps_only('map:7')
        self.positive()
        self.maps_only('map:9')
        shutil.rmtree(self.output / 'maps')
        reconstruct(self.output)
        self.positive()

    def test_tile_selection_accepts_no_whole_map_list(self):
        self.maps_only('tile:7:32:32')
        (self.output / 'maps/0007.tilelist').unlink()
        reconstruct(self.output)
        self.positive()

    def test_tile_selection_list_rejects_intrinsically(self):
        self.maps_only('tile:7:32:32')
        old = self.invoke('verify', code=9)
        new = self.invoke(code=9)
        self.assertEqual(new['code'], old['code'])
        self.assertNotIn('product_links', new['code'])

    def test_changed_bytes_keep_integrity_failure_precedence(self):
        path = self.output / 'maps/0007.tilelist'
        data = bytearray(path.read_bytes()); data[12] ^= 1; path.write_bytes(data)
        old = self.invoke('verify', code=9)
        new = self.invoke(code=9)
        self.assertEqual(new['code'], old['code'])
        self.assertNotIn('product_links', new['code'])

    def test_intrinsic_failure_precedes_missing_relationship(self):
        (self.output / 'maps/0007.tilelist').unlink()
        path = self.output / 'maps/0007_32_32.map'
        data = bytearray(path.read_bytes()); data[:4] = b'NOPE'; path.write_bytes(data)
        reconstruct(self.output)
        old = self.invoke('verify', code=9)
        new = self.invoke(code=9)
        self.assertEqual(new['code'], old['code'])
        self.assertNotIn('product_links', new['code'])

    def test_context_match_and_mismatch(self):
        context = self.root / 'expected.json'
        context.write_bytes((self.output / '.state/context.json').read_bytes())
        self.invoke(extra=['--expect-context', str(context)])
        changed = json.loads(context.read_text()); changed['source_sha256'] = '01' * 32
        context.write_text(compact(changed))
        old = self.invoke('verify', 9, ['--expect-context', str(context)])
        new = self.invoke(code=9, extra=['--expect-context', str(context)])
        self.assertEqual(new['code'], old['code'])

    def test_v2_v3_v6_verifier_rebinding_needs_no_client_keys_scratch(self):
        plans = [keys.plan(), disk.plan(), terrain.plan(components=','.join(COMPONENTS))]
        for text in plans:
            with self.subTest(version=text.splitlines()[0]):
                self.plan.write_text(text)
                reconstruct(self.output, text)
                self.positive()
        self.assertFalse((self.root / 'client').exists())
        self.assertFalse((self.root / 'keys').exists())
        self.assertFalse((self.root / 'scratch').exists())

    def test_v4_v5_db2_only_have_no_relationship_requirement(self):
        for version, build, name in catalog.PROFILES:
            with self.subTest(version=version):
                client = self.root / f'client-v{version}'
                catalog.store(client, catalog.files(), build)
                before = hashes(client)
                self.plan.write_text(catalog.plan(version, build, name))
                self.output = self.root / f'output-v{version}'
                self.invoke('extract', options=['--plan', str(self.plan), '--client', str(client),
                                               '--output', str(self.output)])
                self.assertEqual(before, hashes(client))
                shutil.rmtree(client)
                self.positive()
                (self.root / f'input-hashes-v{version}.json').write_text(json.dumps(before, indent=2) + '\n')

    def test_closed_options_help_and_old_verify_output_stability(self):
        before = self.invoke('verify')
        for option in ('--client', '--keys', '--scratch', '--unknown'):
            with self.subTest(option=option):
                result = self.invoke(code=2, extra=[option, str(self.root / 'absent')])
                self.assertEqual(result['code'], 'product_usage')
        for options in ([], ['--plan', str(self.plan)], ['--plan', str(self.plan), '--output', str(self.output),
                         '--plan', str(self.plan)]):
            result = self.invoke(code=2, options=options)
            self.assertEqual(result['code'], 'product_usage')
        help_text = self.command(self.root, self.calls, '--help', [], parse=False)
        self.assertIn('verify-links --plan PLAN --output OUTPUT', help_text)
        self.invoke()
        after = self.invoke('verify')
        self.assertEqual(before, after)


if __name__ == '__main__':
    unittest.main()
