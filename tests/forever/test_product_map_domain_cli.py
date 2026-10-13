"""GPL-3.0-or-later. Explicit Linux selected-domain integration acceptance.

RCE_PRODUCT_BINARY and RCE_MAP_DOMAIN_CASES select a frozen executable and
fresh external corpus. Optional RCE_MAP_DOMAIN_NATIVE_BUILD selects a retained
original C++ build report; native executes only complete toy positives AFTER
Rust extraction and both independent verification commands succeed.
RCE_MAP_DOMAIN_GRID_SOURCE optionally selects an absolute external GridMap .rs
file; its expected SHA remains the retained terrain-reader report value.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import unittest

import product_map_domain_fixture as fixture
import product_fixture as plain
import product_disk_fixture as disk
import product_keys_fixture as encrypted
import product_offmesh_fixture as offmesh
import product_catalog_fixture as catalog
from test_product_links_cli import reconstruct

BINARY = os.environ.get('RCE_PRODUCT_BINARY')
CASES = os.environ.get('RCE_MAP_DOMAIN_CASES')
NATIVE_BUILD = os.environ.get('RCE_MAP_DOMAIN_NATIVE_BUILD')
ADT_BUILD = os.environ.get('RCE_MAP_DOMAIN_ADT_BUILD')
TERRAIN_REPORT = os.environ.get('RCE_MAP_DOMAIN_TERRAIN_REPORT')
GRID_SOURCE = os.environ.get('RCE_MAP_DOMAIN_GRID_SOURCE')
REPO = Path(__file__).resolve().parents[2]
PIN = '02245dcd245e7433e524577656177723d3e4992e'
sha = lambda data: hashlib.sha256(data).hexdigest()


def checked_grid_source(expected_sha256, explicit, repo=REPO):
    """Select one source dependency; explicit invalid input never falls back."""
    if not isinstance(expected_sha256, str) or not re.fullmatch(r'[0-9a-f]{64}', expected_sha256):
        raise AssertionError('grid_source_expected_hash')
    path = (Path(explicit) if explicit is not None else
            repo.parent / 'rustycore/crates/wow-map/src/grid_map.rs')
    if explicit is not None and not path.is_absolute():
        raise AssertionError('grid_source_absolute')
    try:
        if not path.is_file():
            raise AssertionError('grid_source_file')
        actual = sha(path.read_bytes())
    except OSError:
        raise AssertionError('grid_source_read') from None
    if actual != expected_sha256:
        raise AssertionError('grid_source_hash')
    return path


def hashes(root):
    return {p.relative_to(root).as_posix(): sha(p.read_bytes())
            for p in sorted(root.rglob('*')) if p.is_file()}


def source_hashes():
    result = {}
    for path in [REPO / 'Cargo.toml', REPO / 'Cargo.lock', REPO / 'rust-toolchain.toml']:
        result[path.relative_to(REPO).as_posix()] = sha(path.read_bytes())
    for folder in ('crates', 'contracts', 'tests'):
        for path in sorted((REPO / folder).rglob('*')):
            if path.is_file() and '__pycache__' not in path.parts:
                result[path.relative_to(REPO).as_posix()] = sha(path.read_bytes())
    return result


def native_guards(path):
    report = json.loads(Path(path).read_text())
    source = Path(report['source_root'])
    inventory_path = source / 'inventory.json'
    inventory = json.loads(inventory_path.read_text())
    assert report['reference'] == inventory['pin'] == PIN
    assert sha(inventory_path.read_bytes()) == report['original_inventory_sha256']
    checked = {str(inventory_path): sha(inventory_path.read_bytes())}
    for relative, expected in inventory['files'].items():
        p = source / relative
        assert sha(p.read_bytes()) == expected['sha256'], str(p)
        checked[str(p)] = expected['sha256']
    boost = Path(report['boost_root'])
    assert sha((boost / 'inventory.json').read_bytes()) == report['boost_inventory_sha256']
    checked[str(boost / 'inventory.json')] = report['boost_inventory_sha256']
    for relative, expected in report['boost_used_files'].items():
        p = boost / relative
        assert sha(p.read_bytes()) == expected, str(p)
        checked[str(p)] = expected
    assert report['depfile_closure']
    wrapper = REPO / 'tests/forever/native_collision_assembled.cpp'
    assert sha(wrapper.read_bytes()) == report['wrapper_sha256']
    binary = Path(report['binary'])
    assert sha(binary.read_bytes()) == report['binary_sha256']
    checked[str(binary)] = report['binary_sha256']
    checked[str(wrapper)] = report['wrapper_sha256']
    return report, checked


@unittest.skipUnless(BINARY and CASES, 'explicit external frozen Linux map-domain run required')
class ProductMapDomainCli(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.explicit_grid_source = None
        cls.explicit_grid_sha256 = None
        if GRID_SOURCE is not None:
            if not TERRAIN_REPORT:
                raise AssertionError('grid_source_report_required')
            retained = json.loads(Path(TERRAIN_REPORT).read_text())
            cls.explicit_grid_sha256 = retained['actual_grid_map_source_sha256']
            cls.explicit_grid_source = checked_grid_source(cls.explicit_grid_sha256, GRID_SOURCE)
        os.umask(0o077)
        cls.corpus = Path(CASES).resolve()
        cls.corpus.mkdir(mode=0o700, parents=True)
        if any((p / '.git').exists() for p in (cls.corpus, *cls.corpus.parents)):
            raise AssertionError('corpus must be outside Git')
        cls.binary_before = sha(Path(BINARY).read_bytes())
        cls.sources_before = source_hashes()
        cls.native_report, cls.native_sources = native_guards(NATIVE_BUILD) if NATIVE_BUILD else (None, {})
        if cls.explicit_grid_source is not None:
            cls.native_sources[str(cls.explicit_grid_source)] = cls.explicit_grid_sha256
        cls.base = cls.corpus / '_base'
        cls.base.mkdir(mode=0o700)
        cls.base_payloads = fixture.files()
        plain.store(cls.base / 'client', cls.base_payloads)
        cls.base_plan = fixture.plan()
        (cls.base / 'plan.tsv').write_text(cls.base_plan)
        (cls.base / 'scratch').mkdir(mode=0o700)
        cls.base_calls = []
        cls.command(cls.base, cls.base_calls, 'extract', ['--plan', str(cls.base / 'plan.tsv'),
                    '--client', str(cls.base / 'client'), '--output', str(cls.base / 'output'),
                    '--scratch', str(cls.base / 'scratch')])
        for name in ('verify', 'verify-links'):
            cls.command(cls.base, cls.base_calls, name,
                        ['--plan', str(cls.base / 'plan.tsv'), '--output', str(cls.base / 'output')])
        cls.base_hashes = hashes(cls.base)
        # One complete shared navigation package covers the root, ordinary
        # parent and cosmetic child. The child has no own geometry and must
        # use ancestor MMaps rather than inventing a child global/tile.
        cls.nav_base = cls.corpus / '_navigation-base'
        cls.nav_base.mkdir(mode=0o700)
        plain.store(cls.nav_base / 'client', cls.base_payloads)
        cls.nav_base_plan = fixture.plan(17, navigation=True)
        (cls.nav_base / 'plan.tsv').write_text(cls.nav_base_plan)
        (cls.nav_base / 'scratch').mkdir(mode=0o700)
        cls.command(cls.nav_base, cls.base_calls, 'extract', ['--plan', str(cls.nav_base / 'plan.tsv'),
                    '--client', str(cls.nav_base / 'client'), '--output', str(cls.nav_base / 'output'),
                    '--scratch', str(cls.nav_base / 'scratch')])
        for name in ('verify', 'verify-links'):
            cls.command(cls.nav_base, cls.base_calls, name,
                        ['--plan', str(cls.nav_base / 'plan.tsv'), '--output', str(cls.nav_base / 'output')])
        cls.nav_base_hashes = hashes(cls.nav_base)

    @classmethod
    def tearDownClass(cls):
        guards = {'binary_unchanged': cls.binary_before == sha(Path(BINARY).read_bytes()),
                  'sources_unchanged': cls.sources_before == source_hashes(),
                  'base_unchanged': cls.base_hashes == hashes(cls.base),
                  'navigation_base_unchanged': cls.nav_base_hashes == hashes(cls.nav_base),
                  'native_sources_unchanged': all(sha(Path(p).read_bytes()) == h
                                                  for p, h in cls.native_sources.items())}
        report = {'scope': 'authored_selected_domain_product_and_original_conversion_comparison',
                  'binary_sha256': cls.binary_before, 'guards': guards,
                  'base_commands': cls.base_calls, 'native_build_report': NATIVE_BUILD,
                  'native_source_hashes': cls.native_sources, 'full_extractor_complete': False,
                  'real_client_inputs': False, 'native_CASC_routing': False,
                  'native_map_selection_executed': False, 'native_malformed_execution': False}
        (cls.corpus / 'suite-report.json').write_text(json.dumps(report, indent=2) + '\n')
        (cls.corpus / 'source-hashes.json').write_text(json.dumps(cls.sources_before, indent=2) + '\n')
        if not all(guards.values()):
            raise AssertionError('frozen acceptance evidence changed')

    @classmethod
    def command(cls, root, calls, command, options, code=0, kill=None, executable=None):
        executable = executable or BINARY
        stem = root / f'{len(calls) + 1:03}-{command}'
        trace = Path(str(stem) + '.network')
        argv = ['strace', '-f', '-qq', '-e', 'trace=network,fsync' if kill else 'trace=network',
                '-e', 'inject=network:error=ENETUNREACH', '-o', str(trace)]
        if kill:
            argv += ['-e', f'inject=fsync:signal=SIGKILL:when={kill}']
        argv += [str(executable)] + ([command] if executable == BINARY else []) + list(map(str, options))
        plan_sha = None
        if '--plan' in options:
            selected_plan = Path(options[options.index('--plan') + 1])
            plan_bytes = selected_plan.read_bytes()
            plan_sha = sha(plan_bytes)
            Path(str(stem) + '.plan.tsv').write_bytes(plan_bytes)
        result = subprocess.run(argv, capture_output=True, text=True, timeout=360)
        Path(str(stem) + '.stdout').write_text(result.stdout)
        Path(str(stem) + '.stderr').write_text(result.stderr)
        facts = {'argv': argv, 'exit_code': result.returncode, 'expected_sigkill': bool(kill),
                 'stdout_sha256': sha(result.stdout.encode()), 'stderr_sha256': sha(result.stderr.encode()),
                 'trace_sha256': sha(trace.read_bytes()), 'trace_bytes': trace.stat().st_size,
                 'invoked_plan_sha256': plan_sha}
        calls.append(facts)
        if kill:
            calls_seen = set(re.findall(r'^\d+\s+(\w+)\(', trace.read_text(), re.M))
            if calls_seen - {'fsync'}:
                raise AssertionError('unexpected syscall in interruption trace')
            if result.returncode == 0 or 'SIGKILL' not in trace.read_text():
                raise AssertionError('actual bounded SIGKILL did not occur')
            return None
        if trace.read_bytes() != b'':
            raise AssertionError('network syscall observed')
        if result.returncode != code:
            raise AssertionError(f'expected {code}, got {result.returncode}: {result.stdout} {result.stderr}')
        return json.loads(result.stdout if code == 0 else result.stderr)

    def setUp(self):
        self.root = self.corpus / self._testMethodName
        self.root.mkdir(mode=0o700)
        self.client, self.plan, self.output = self.root / 'client', self.root / 'plan.tsv', self.root / 'output'
        self.scratch = self.root / 'scratch'
        self.calls, self.results = [], []
        self.inputs = {}
        self.keys = None

    def tearDown(self):
        unchanged = all(sha(Path(p).read_bytes()) == h for p, h in self.inputs.items())
        report = {'commands': self.calls, 'results': self.results,
                  'inputs_unchanged': unchanged, 'source_input_hashes': self.inputs,
                  'synthetic_only': True, 'native_malformed_execution': False,
                  'native_map_selection_executed': False, 'full_extractor_complete': False}
        (self.root / 'case-report.json').write_text(json.dumps(report, indent=2) + '\n')
        self.assertTrue(unchanged)

    def prepare(self, map_id=7, navigation=False, rows=fixture.ROWS, remove=(), supplied=False, empty_world=False, overlapping_parent=False):
        payloads = fixture.files(rows)
        if empty_world:
            for fid in (100, 101, 117):
                payloads[fid] = fixture.wdt()
        if overlapping_parent:
            payloads[212] = fixture.objects(position=(0., 0., 0.))
        for fid in remove:
            payloads.pop(fid)
        if supplied:
            encrypted.store(self.client, payloads)
            self.keys = self.root / 'keys.tsv'
            self.keys.write_text(encrypted.keyfile())
        else:
            plain.store(self.client, payloads)
        self.scratch.mkdir(mode=0o700)
        self.plan.write_text(fixture.plan(map_id, navigation, supplied=supplied))
        self.payloads, self.rows = payloads, rows
        self.inputs = {str(p): sha(p.read_bytes()) for p in self.client.rglob('*') if p.is_file()}
        if self.keys:
            self.inputs[str(self.keys)] = sha(self.keys.read_bytes())

    def options(self, command):
        result = ['--plan', str(self.plan)]
        if command in ('extract', 'resume'):
            result += ['--client', str(self.client), '--output', str(self.output), '--scratch', str(self.scratch)]
            if self.keys:
                result += ['--keys', str(self.keys)]
        elif command in ('verify', 'verify-links'):
            result += ['--output', str(self.output)]
        return result

    def invoke(self, command, code=0, kill=None, options=None):
        return self.command(self.root, self.calls, command,
                            self.options(command) if options is None else options, code, kill)

    def accept(self):
        produced = self.invoke('extract')
        self.assertEqual(produced['contract'], 'forever-product-v8')
        self.assertEqual(produced['map_domain'], fixture.PROFILE)
        self.assertEqual(produced['gameobject_scope'], fixture.GAMEOBJECT)
        self.assertEqual(produced['placement_ids'], fixture.PLACEMENT_IDS)
        self.assertFalse(produced['full_extractor_complete'])
        for command in ('verify', 'verify-links'):
            verified = self.invoke(command)
            self.assertEqual(produced['package'], verified['package'])
        return produced

    def native(self, map_id):
        if not self.native_report:
            self.skipTest('retained guarded original assembly build not selected')
        inventory = self.root / 'native-inventory.bin'
        raw = fixture.native_inventory(self.payloads, map_id, self.rows)
        inventory.write_bytes(raw)
        self.inputs[str(inventory)] = sha(raw)
        self.assertNotEqual(sha(self.payloads[plain.IDS['Map']]), sha(fixture.map_table(fixture.closure(map_id, self.rows))))
        stage = self.root / 'native-output'
        summary = self.command(self.root, self.calls, 'native-assembled', [inventory, stage],
                               executable=self.native_report['binary'])
        original, rust = hashes(stage / 'vmaps'), hashes(self.output / 'vmaps')
        self.assertEqual(original, rust)
        for name in original:
            self.assertEqual((stage / 'vmaps' / name).read_bytes(), (self.output / 'vmaps' / name).read_bytes())
        self.assertTrue(summary['original_ParsMapFiles'] and summary['original_convertWorld2'])
        self.results.append({'native_exact_files': len(original),
                             'native_exact_bytes': sum((stage / 'vmaps' / n).stat().st_size for n in original),
                             'full_map_sha256': sha(self.payloads[plain.IDS['Map']]),
                             'native_projected_map_sha256': sha(fixture.map_table(fixture.closure(map_id, self.rows))),
                             'native_closure': [r[0] for r in fixture.closure(map_id, self.rows)],
                             'native_summary': summary})

    def cached(self, navigation=False):
        source = self.nav_base if navigation else self.base
        self.plan.write_text(self.nav_base_plan if navigation else self.base_plan)
        shutil.copytree(source / 'output', self.output)

    def reference_navigation(self, map_id, selection='all'):
        payloads = dict(self.payloads)
        payloads[plain.IDS['Map']] = fixture.map_table(fixture.closure(map_id, self.rows))
        reference_client = self.root / 'reference-client'
        plain.store(reference_client, payloads)
        self.client = reference_client
        self.inputs.update({str(p): sha(p.read_bytes()) for p in reference_client.rglob('*') if p.is_file()})
        self.output = self.root / 'reference-output'
        self.scratch = self.root / 'reference-scratch'; self.scratch.mkdir(mode=0o700)
        text = disk.plan('db2,maps,vmaps,mmaps').replace('forever-product\t3\n', 'forever-product\t6\n')
        text = text.replace('navigation\tall\n', f'navigation\t{selection}\n')
        text = text.replace('non-steep-slope\t55\n', 'non-steep-slope\t55\noffmesh\tinline-f32-bits-v1\n')
        self.plan.write_text(text)

    def test_child_ignores_missing_unrelated_wdt_and_matches_original(self):
        self.prepare(); self.accept(); self.native(7)
        self.assertEqual({p.name for p in (self.output / 'vmaps').iterdir() if p.is_dir()}, {'0001', '0007'})
        self.assertEqual({p.name for p in (self.output / 'maps').iterdir()}, {'0001.tilelist', '0001_00_00.map', '0007.tilelist', '0007_32_32.map'})
        self.assertTrue((self.output / 'vmaps/FILE00000258.xxx.vmo').is_file())

    def test_root_domain_matches_original_and_retains_global_startup(self):
        self.prepare(1); self.accept(); self.native(1)
        self.assertEqual({p.name for p in (self.output / 'vmaps').iterdir() if p.is_dir()}, {'0001'})
        self.assertTrue((self.output / 'vmaps/FILE00000258.xxx.vmo').is_file())

    def test_cosmetic_chain_matches_original_and_has_empty_own_terrain(self):
        self.prepare(17); self.accept(); self.native(17)
        self.assertEqual({p.name for p in (self.output / 'vmaps').iterdir() if p.is_dir()}, {'0001', '0007', '0017'})
        self.assertEqual({p.name for p in (self.output / 'maps').iterdir()}, {'0001.tilelist', '0001_00_00.map', '0007.tilelist', '0007_32_32.map', '0017.tilelist'})

    def test_all_map_plan_still_rejects_the_same_missing_unrelated_wdt(self):
        self.prepare()
        self.plan.write_text(disk.plan('db2,maps,vmaps'))
        self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.invoke('verify', code=8)

    def test_selected_missing_wdt_remains_terminal(self):
        self.prepare(9); self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_unknown_selected_map_remains_terminal(self):
        self.prepare(25); self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_missing_required_ancestor_wdt_remains_terminal(self):
        self.prepare(remove=(100,)); self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_missing_selected_object_adt_remains_terminal(self):
        self.prepare(remove=(202,)); self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_global_startup_missing_model_is_not_pruned(self):
        self.prepare(1, remove=(600,)); self.invoke('extract', code=9)
        self.assertFalse((self.output / 'forever-package.json').exists())

    def test_supplied_keys_selected_child_matches_original(self):
        self.prepare(supplied=True); self.accept(); self.native(7)

    def test_empty_requested_terrain_navigation_keeps_complete_ancestors(self):
        self.prepare(17, navigation=True); self.accept(); self.native(17)
        selected = hashes(self.output / 'mmaps')
        self.assertEqual({n for n in selected if n.endswith('.mmap')}, {'0001.mmap', '0007.mmap'})
        self.assertFalse(any(n.startswith('0017') for n in selected))
        self.assertFalse(list((self.output / 'maps').glob('0017_*.map')))
        self.reference_navigation(17)
        self.invoke('extract'); self.invoke('verify'); self.invoke('verify-links')
        self.assertEqual(selected, hashes(self.output / 'mmaps'))
        self.results.append({'navigation_v6_exact_files': len(selected),
                             'reference_map_is_explicit_closure': True,
                             'own_empty_child_omitted': 17, 'ancestor_globals': [1, 7]})

    def test_whole_empty_navigation_domain_completes_without_fabricated_mmaps(self):
        self.prepare(17, navigation=True, empty_world=True)
        self.accept(); self.native(17)
        self.assertEqual(hashes(self.output / 'mmaps'), {})
        self.results.append({'authenticated_empty_domain_mmap_files': 0})
        self.reference_navigation(17, 'map:17')
        result = self.invoke('extract', code=9)
        self.assertEqual(result['code'], 'navigation_run_selection')
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.invoke('verify', code=8)

    def test_nearest_ancestor_mixed_payload_mismatch_remains_terminal(self):
        self.prepare(17, overlapping_parent=True)
        result = self.invoke('extract', code=9)
        self.assertEqual(result['code'], 'assembly_association')
        self.assertFalse((self.output / 'forever-package.json').exists())
        self.invoke('verify', code=8)

    def test_selected_navigation_matches_explicit_complete_v6_closure(self):
        self.prepare(navigation=True); self.accept()
        selected = hashes(self.output / 'mmaps')
        self.assertTrue(any(n.endswith('.mmtile') for n in selected))
        self.reference_navigation(7)
        self.invoke('extract'); self.invoke('verify'); self.invoke('verify-links')
        self.assertEqual(selected, hashes(self.output / 'mmaps'))
        self.results.append({'navigation_v6_exact_files': len(selected), 'reference_map_is_explicit_closure': True})

    def test_selected_terrain_matches_original_conversion_and_checked_reader(self):
        if not ADT_BUILD or not TERRAIN_REPORT:
            self.skipTest('retained guarded terrain converters/readers not selected')
        retained = json.loads(Path(TERRAIN_REPORT).read_text())
        grid_source = checked_grid_source(retained['actual_grid_map_source_sha256'], GRID_SOURCE)
        self.prepare()
        self.inputs[str(grid_source)] = retained['actual_grid_map_source_sha256']
        self.accept()
        build = json.loads(Path(ADT_BUILD).read_text())
        source = Path(build['source_root'])
        inventory_path = source / 'inventory.json'
        inventory = json.loads(inventory_path.read_text())
        self.assertEqual(build['reference_pin'], PIN)
        self.assertEqual(inventory['pin'], PIN)
        self.assertEqual(sha(inventory_path.read_bytes()), build['source_inventory_sha256'])
        self.inputs[str(inventory_path)] = sha(inventory_path.read_bytes())
        for relative, expected in inventory['files'].items():
            path = source / relative
            self.assertEqual(sha(path.read_bytes()), expected['sha256'])
            self.inputs[str(path)] = expected['sha256']
        for name, expected in build['adapter_sources'].items():
            self.assertEqual(sha(Path(name).read_bytes()), expected)
            self.inputs[name] = expected
        native = Path(build['binary'])
        self.assertEqual(sha(native.read_bytes()), build['binary_sha256'])
        self.inputs[str(native)] = build['binary_sha256']
        adt, lookup, context = (self.root / name for name in ('adt.bin', 'lookup.bin', 'context.bin'))
        adt.write_bytes(self.payloads[201])
        lookup.write_bytes(b'RCELIQ01' + struct.pack('<II', 0, 0))
        context.write_bytes(b'RCECONV1' + struct.pack('<IBBIBIII', 7, 32, 32, 70205, 1,
                                                   16 * 1024 * 1024, 4096, 202112))
        for path in (adt, lookup, context):
            self.inputs[str(path)] = sha(path.read_bytes())
        native_map = self.root / 'native.map'
        summary = self.command(self.root, self.calls, 'native-terrain',
                               [adt, lookup, context, native_map], executable=str(native))
        self.assertTrue(summary['original_parser'] and summary['original_converter'])
        published = self.output / 'maps/0007_32_32.map'
        self.assertEqual(native_map.read_bytes(), published.read_bytes())
        rust_reader = cpp_reader = None
        for name, expected in retained['binaries'].items():
            path = Path(name)
            self.assertEqual(sha(path.read_bytes()), expected)
            self.inputs[name] = expected
            if path.name == 'terrain-grid-reader':
                rust_reader = path
            elif path.name == 'terrain-grid-oracle':
                cpp_reader = path
        self.assertIsNotNone(rust_reader); self.assertIsNotNone(cpp_reader)
        query = self.root / 'queries.bin'
        points = [(float(x), float(y)) for y in (-533, -200, -1, 0, 1, 200, 533)
                  for x in (-533, -200, -1, 0, 1, 200, 533)]
        query.write_bytes(b'RCEQUERY' + struct.pack('<I', len(points)) +
                          b''.join(struct.pack('<ff', x, y) for x, y in points))
        self.inputs[str(query)] = sha(query.read_bytes())
        answers = []
        for name, reader in (('rust-grid', rust_reader), ('cpp-height-kernel', cpp_reader)):
            out = self.root / (name + '.bin')
            row = self.command(self.root, self.calls, name, [published, query, out], executable=str(reader))
            self.assertEqual(row['queries'], len(points))
            answers.append(out.read_bytes())
        self.assertEqual(answers[0], answers[1])
        self.results.append({'native_exact_maps': 1, 'native_exact_map_bytes': published.stat().st_size,
                             'exact_checked_grid_height_queries': len(points),
                             'cpp_reader_scope': 'independent pinned scalar height kernel; not full native GridMap'})

    def test_sigkill_resume_preserves_domain_and_exact_final_package(self):
        self.prepare()
        original = self.plan.read_text()
        self.invoke('extract', kill=15)
        self.assertTrue((self.output / '.forever-package-incomplete').exists())
        self.invoke('verify', code=8)
        self.plan.write_text(fixture.plan(1))
        self.invoke('resume', code=9)
        self.plan.write_text(original)
        self.invoke('resume'); self.invoke('verify'); self.invoke('verify-links')
        first = json.loads((self.output / 'forever-package.json').read_text())
        manifest = (self.output / 'forever-package.json').read_bytes()
        inventory = {a['path']: sha((self.output / a['path']).read_bytes()) for a in first['artifacts']}
        self.output = self.root / 'uninterrupted'
        self.scratch = self.root / 'uninterrupted-scratch'; self.scratch.mkdir(mode=0o700)
        self.accept()
        self.assertEqual(manifest, (self.output / 'forever-package.json').read_bytes())
        self.assertEqual(inventory, {a['path']: sha((self.output / a['path']).read_bytes()) for a in first['artifacts']})

    def test_rehashed_unrelated_vmap_tree_is_rejected(self):
        self.cached()
        parent = self.output / 'vmaps/0001'
        unrelated = self.output / 'vmaps/0009'
        parent.rename(unrelated)
        for path in tuple(unrelated.iterdir()):
            path.rename(path.with_name(path.name.replace('0001', '0009', 1)))
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_rehashed_missing_ancestor_package_is_rejected(self):
        self.cached(); shutil.rmtree(self.output / 'vmaps/0001'); reconstruct(self.output)
        for command in ('verify', 'verify-links'):
            result = self.invoke(command, code=9)
            self.assertEqual(result['code'], 'product_map_domain_collision')

    def test_rehashed_all_vmap_map_folders_removed_is_rejected(self):
        self.cached()
        for path in (self.output / 'vmaps').iterdir():
            if path.is_dir():
                shutil.rmtree(path)
        reconstruct(self.output)
        self.assertTrue((self.output / 'vmaps/GameObjectModels.dtree').is_file())
        for command in ('verify', 'verify-links'):
            result = self.invoke(command, code=9)
            self.assertEqual(result['code'], 'product_map_domain_collision')

    def test_rehashed_map_source_missing_ancestor_is_rejected(self):
        self.cached()
        rows = [r for r in fixture.ROWS if r[0] != 1]
        (self.output / 'dbc/enUS/Map.db2').write_bytes(fixture.map_table(rows))
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_rehashed_terrain_tile_set_mismatch_is_rejected_by_links(self):
        self.cached()
        path = self.output / 'maps/0007.tilelist'
        data = bytearray(path.read_bytes()); data[12 + 4095 - (32 * 64 + 32)] = ord('0'); path.write_bytes(data)
        reconstruct(self.output)
        for command in ('verify', 'verify-links'):
            result = self.invoke(command, code=9)
            self.assertEqual(result['code'], 'product_links_terrain_tile_set')

    def test_rehashed_foreign_terrain_map_is_rejected(self):
        self.cached()
        source = self.output / 'maps/0007_32_32.map'
        source.rename(self.output / 'maps/0009_32_32.map')
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_rehashed_missing_ancestor_navigation_global_is_rejected(self):
        self.cached(navigation=True)
        (self.output / 'mmaps/0001.mmap').unlink()
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_rehashed_all_ancestor_navigation_files_removed_is_rejected(self):
        self.cached(navigation=True)
        for path in (self.output / 'mmaps').glob('0001*'):
            path.unlink()
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_rehashed_invented_own_empty_child_global_is_rejected(self):
        self.cached(navigation=True)
        # Empty offmesh globals have no embedded map identity. This copied
        # complete header passes intrinsic framing but violates own discovery.
        shutil.copyfile(self.output / 'mmaps/0007.mmap', self.output / 'mmaps/0017.mmap')
        reconstruct(self.output)
        self.invoke('verify', code=9); self.invoke('verify-links', code=9)

    def test_plan_admission_rejects_wrong_domain_and_mismatched_terrain(self):
        self.plan.write_text(fixture.plan().replace(fixture.PROFILE, 'not-reviewed'))
        self.invoke('inspect', code=2)
        self.plan.write_text(fixture.plan().replace('terrain\tmap:7', 'terrain\tmap:1'))
        self.invoke('inspect', code=2)
        self.assertFalse(self.output.exists())

    def test_v1_v2_v3_v6_keep_retained_all_six_linux_golden_artifacts(self):
        for version in (1, 2, 3, 6):
            for build, locale, filename in (
                    (70205, 'enUS', 'windows-resident-product-golden-v1.json'),
                    (70170, 'esES', 'windows-resident-product-70170-golden-v1.json')):
                root = self.root / f'v{version}-{build}-{locale}'
                root.mkdir(mode=0o700)
                client, output, plan = root / 'client', root / 'output', root / 'plan.tsv'
                payloads = disk.files()
                keys = None
                if version == 2:
                    encrypted.store(client, payloads, build, locale)
                    keys = root / 'keys.tsv'; keys.write_text(encrypted.keyfile())
                    text = encrypted.plan(build=build, locale=locale)
                else:
                    plain.store(client, payloads, build, locale)
                    text = (disk.plan(build=build, locale=locale) if version == 3 else
                            offmesh.plan((), build, locale, 'db2,gt,cameras,maps,vmaps,mmaps')
                            if version == 6 else plain.plan(build=build, locale=locale))
                plan.write_text(text)
                self.inputs.update({str(p): sha(p.read_bytes()) for p in client.rglob('*') if p.is_file()})
                options = ['--plan', plan, '--client', client, '--output', output]
                if keys:
                    options += ['--keys', keys]; self.inputs[str(keys)] = sha(keys.read_bytes())
                if version == 3:
                    scratch = root / 'scratch'; scratch.mkdir(mode=0o700)
                    options += ['--scratch', scratch]
                self.command(root, self.calls, 'extract', options)
                for name in ('verify', 'verify-links'):
                    self.command(root, self.calls, name, ['--plan', plan, '--output', output])
                manifest = json.loads((output / 'forever-package.json').read_text())
                path = REPO / 'tests/forever/fixtures' / filename
                golden = json.loads(path.read_text())
                self.inputs[str(path)] = sha(path.read_bytes())
                self.assertEqual(manifest['artifacts'], golden['artifacts'])
                self.results.append({'legacy_version': version, 'build': build, 'locale': locale,
                                     'golden_files': len(manifest['artifacts']),
                                     'golden_bytes': manifest['total_bytes'], 'exact_artifact_golden': True})

    def test_v4_v5_keep_complete_multisection_catalog_source_bytes(self):
        for version, build, name in catalog.PROFILES:
            root = self.root / f'v{version}-{build}'
            root.mkdir(mode=0o700)
            client, output, plan = root / 'client', root / 'output', root / 'plan.tsv'
            payloads = catalog.files()
            catalog.store(client, payloads, build)
            plan.write_text(catalog.plan(version, build, name))
            self.inputs.update({str(p): sha(p.read_bytes()) for p in client.rglob('*') if p.is_file()})
            self.command(root, self.calls, 'extract', ['--plan', plan, '--client', client, '--output', output])
            self.command(root, self.calls, 'verify', ['--plan', plan, '--output', output])
            self.command(root, self.calls, 'verify-links', ['--plan', plan, '--output', output])
            for table, metadata in catalog.TABLES.items():
                self.assertEqual((output / f'dbc/enUS/{table}.db2').read_bytes(), payloads[metadata[0]])
            self.results.append({'legacy_version': version, 'build': build, 'exact_catalog_source_files': 2})

    def test_plan_only_inspect_reports_explicit_domain_without_client(self):
        self.plan.write_text(fixture.plan())
        result = self.invoke('inspect')
        self.assertEqual(result['map_domain'], fixture.PROFILE)
        self.assertEqual(result['requested_map'], 7)
        self.assertEqual(result['gameobject_scope'], fixture.GAMEOBJECT)
        self.assertEqual(result['placement_ids'], fixture.PLACEMENT_IDS)
        self.assertFalse(self.output.exists())


class GridSourceGuards(unittest.TestCase):
    """Authored path/hash checks; no product, native process or client input."""
    def setUp(self):
        import tempfile
        self.private = tempfile.TemporaryDirectory(prefix='forever-grid-source-')
        self.addCleanup(self.private.cleanup)
        self.root = Path(self.private.name).resolve()
        self.repo = self.root / 'moved/source'
        self.default = self.repo.parent / 'rustycore/crates/wow-map/src/grid_map.rs'
        self.default.parent.mkdir(parents=True)
        self.body = b'// authored default GridMap source fixture\n'
        self.default.write_bytes(self.body)
        self.expected = sha(self.body)

    def test_historical_default_path_and_hash_are_preserved(self):
        self.assertEqual(checked_grid_source(self.expected, None, self.repo), self.default)

    def test_absolute_external_source_supports_moved_bundle_without_sibling_checkout(self):
        external = self.root / 'external grid_map.rs'
        external.write_bytes(self.body)
        self.default.unlink()
        self.assertEqual(checked_grid_source(self.expected, str(external), self.repo), external)

    def test_explicit_source_is_selected_even_when_default_has_different_bytes(self):
        external = self.root / 'external.rs'
        external.write_bytes(b'// authored explicitly selected source\n')
        expected = sha(external.read_bytes())
        self.assertEqual(checked_grid_source(expected, str(external), self.repo), external)

    def test_empty_and_relative_explicit_paths_reject_without_valid_default_fallback(self):
        for explicit in ('', 'grid_map.rs', './grid_map.rs'):
            with self.subTest(explicit=explicit):
                with self.assertRaisesRegex(AssertionError, '^grid_source_absolute$'):
                    checked_grid_source(self.expected, explicit, self.repo)

    def test_missing_and_directory_explicit_paths_reject_without_default_fallback(self):
        for explicit in (self.root / 'missing.rs', self.root):
            with self.subTest(explicit=explicit):
                with self.assertRaisesRegex(AssertionError, '^grid_source_file$'):
                    checked_grid_source(self.expected, str(explicit), self.repo)

    def test_mismatched_explicit_hash_rejects_without_matching_default_fallback(self):
        external = self.root / 'changed.rs'
        external.write_bytes(b'// authored different source\n')
        with self.assertRaisesRegex(AssertionError, '^grid_source_hash$'):
            checked_grid_source(self.expected, str(external), self.repo)

    def test_retained_expected_hash_must_be_exact_lowercase_sha256(self):
        for expected in (None, '', '0' * 63, 'A' * 64):
            with self.subTest(expected=expected):
                with self.assertRaisesRegex(AssertionError, '^grid_source_expected_hash$'):
                    checked_grid_source(expected, None, self.repo)

    def test_explicit_invalid_dependency_rejects_before_any_product_or_native_command(self):
        from unittest.mock import patch
        report = self.root / 'authored-retained-report.json'
        report.write_text(json.dumps({'actual_grid_map_source_sha256': self.expected}))
        for selected_report, code in ((None, 'grid_source_report_required'),
                                      (str(report), 'grid_source_file')):
            with self.subTest(report=selected_report):
                with patch.dict(globals(), {'GRID_SOURCE': str(self.root / 'missing.rs'),
                                           'TERRAIN_REPORT': selected_report}):
                    with patch.object(ProductMapDomainCli, 'command') as command:
                        with self.assertRaisesRegex(AssertionError, '^' + code + '$'):
                            ProductMapDomainCli.setUpClass()
                        command.assert_not_called()


if __name__ == '__main__':
    unittest.main()
