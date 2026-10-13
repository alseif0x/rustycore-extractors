# SPDX-License-Identifier: GPL-3.0-or-later
"""Coordinator-run independent synthetic regular-section native acceptance.

Usage: python3 regular_sections_acceptance.py --source-root SNAPSHOT
  --rust RUST_EXAMPLE --native ORIGINAL_WRAPPER --native-build-report JSON
  --original-inventory JSON --run-root NEW_PRIVATE_DIRECTORY --profile debug

The coordinator must first build the exact immutable source snapshot and original
closure. This runner neither builds nor fetches. The build report uses the
existing native-QA fields reference, binary, binary_sha256, wrapper_sha256,
original_inventory_sha256, source_root, original_sources_unchanged and commands.
A retained actual compiler dependency closure must be attested separately by
that report's nonempty depfile_closure. Original files/notices are hash-guarded.
Every complete positive first passes the independent whole-wire oracle and Rust;
only then does original loading/getters/materialization execute. Malformed and
policy negatives never enter native. strace records each complete process tree;
any network syscall, unexpected stderr/exit, changed input/source/binary or
projection mismatch is fatal. No real client, CASC, DB, hotfix or runtime calls.
"""
import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import resource
import shutil
import stat
import subprocess
import sys

if not __debug__:
    raise RuntimeError('qa_assertions_required')

PIN = '02245dcd245e7433e524577656177723d3e4992e'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def read_json(path):
    return json.loads(Path(path).read_text())


def process_limits():
    for kind, value in ((resource.RLIMIT_AS, 768 * 1024**2),
                        (resource.RLIMIT_CPU, 30),
                        (resource.RLIMIT_FSIZE, 32 * 1024**2)):
        resource.setrlimit(kind, (value, value))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for field in ('source-root', 'rust', 'native', 'native-build-report',
                  'original-inventory', 'run-root'):
        parser.add_argument('--' + field, required=True, type=Path)
    parser.add_argument('--profile', choices=('debug', 'release'), required=True)
    args = parser.parse_args()
    os.umask(0o077)
    root = args.source_root.resolve(strict=True)
    run = args.run_root.absolute()
    assert not any((p / '.git').exists() for p in [run, *run.parents]), 'qa_private_outside_git'
    assert not run.exists(), 'qa_new_run_only'
    run.mkdir(mode=0o700)
    assert stat.S_IMODE(run.stat().st_mode) == 0o700
    paths = {'driver': Path(__file__).resolve(),
             'corpus': root / 'tests/forever/regular_sections_fixture.py',
             'wrapper': root / 'tests/forever/native_regular_sections.cpp',
             'example': root / 'crates/forever-db2/examples/regular_sections.rs',
             'inventory': args.original_inventory.resolve(strict=True),
             'build': args.native_build_report.resolve(strict=True)}
    assert paths['driver'] == root / 'tests/forever/regular_sections_acceptance.py', 'qa_exact_snapshot_driver'
    build, inventory = read_json(paths['build']), read_json(paths['inventory'])
    assert build['reference'] == inventory['pin'] == PIN
    source = Path(inventory['root']).resolve(strict=True)
    assert Path(build['source_root']).resolve(strict=True) == source
    assert sha(paths['inventory']) == build['original_inventory_sha256']
    assert build['original_sources_unchanged'] and build['depfile_closure']
    assert build['wrapper_sha256'] == sha(paths['wrapper'])
    assert any(str(paths['wrapper']) in command and '-c' in command for command in build['commands'])
    assert any(rel.endswith('/DB2LoadInfo.h') for rel in inventory['files'])
    assert any(rel.endswith('/DB2Metadata.h') for rel in inventory['files'])
    assert any(rel.endswith('/DB2DatabaseLoader.h') for rel in inventory['files'])
    assert any(rel.endswith('/HotfixDatabase.h') for rel in inventory['files'])
    assert 'AUTHORS' in inventory['files'] and 'COPYING' in inventory['files']
    binaries = {'rust': args.rust.resolve(strict=True), 'native': args.native.resolve(strict=True)}
    assert Path(build['binary']).resolve(strict=True) == binaries['native']
    assert sha(binaries['native']) == build['binary_sha256']
    watched = sorted((root / 'crates/forever-db2').rglob('*.rs')) + sorted(paths.values())
    before = {str(p): sha(p) for p in watched}
    binary_before = {key: sha(p) for key, p in binaries.items()}

    def guard():
        assert {str(p): sha(p) for p in watched} == before, 'qa_source_changed'
        assert {key: sha(p) for key, p in binaries.items()} == binary_before, 'qa_binary_changed'
        for rel, entry in inventory['files'].items():
            assert sha(source / rel) == entry['sha256'], 'qa_original_changed'
        assert read_json(paths['build']) == build and read_json(paths['inventory']) == inventory

    guard()
    artifacts = {}

    def track(path):
        state = path.lstat()
        assert stat.S_ISREG(state.st_mode) and state.st_nlink == 1
        value = {'bytes': state.st_size, 'sha256': sha(path),
                 'mode': stat.S_IMODE(state.st_mode)}
        key = path.name
        assert key not in artifacts or artifacts[key] == value, 'qa_artifact_changed'
        artifacts[key] = value
        return value

    archives = {}
    for key, path in paths.items():
        destination = run / ('archive-' + key + path.suffix)
        with destination.open('xb') as stream:
            stream.write(path.read_bytes())
        archives[key] = track(destination)['sha256']
    copied = {}
    for key, path in binaries.items():
        destination = run / key
        shutil.copyfile(path, destination)
        destination.chmod(0o500)
        assert sha(destination) == binary_before[key]
        copied[key] = destination
        track(destination)
    spec = importlib.util.spec_from_file_location('regular_sections_fixture', paths['corpus'])
    corpus = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(corpus)
    provenance = corpus.provenance()
    assert provenance['reference'] == PIN and provenance['table'] == 'SpellDuration'
    assert not provenance['product_decoder_imported'] and not provenance['generated_catalog_imported']
    assert not provenance['real_client_assets'] and provenance['assertions_enabled']
    exits, traces = collections.Counter(), []

    def save(path, data):
        with path.open('xb') as stream:
            stream.write(data)
        assert stat.S_IMODE(path.stat().st_mode) == 0o600
        track(path)

    def execute(kind, wire, destination, label, ceiling=16):
        trace = run / (label + '.network.trace')
        command = ['strace', '-f', '-qq', '-s', '256', '-e', 'trace=network',
                   '-o', str(trace), '--', str(copied[kind]), str(wire), str(destination)]
        if kind == 'rust':
            command += ['SpellDuration', str(ceiling)]
        result = subprocess.run(command, capture_output=True, timeout=45,
                                preexec_fn=process_limits, check=False)
        exits[result.returncode] += 1
        save(run / (label + '.stdout'), result.stdout)
        save(run / (label + '.stderr'), result.stderr)
        assert trace.is_file() and trace.read_bytes() == b'', 'qa_network_attempt'
        track(trace)
        traces.append(trace.name)
        assert result.stderr == b'', 'qa_unexpected_stderr'
        return result

    positives, negatives, reductions, exclusive = [], [], [], []
    for ordinal, (label, data, wanted) in enumerate(corpus.cases()):
        stem = f'positive-{ordinal:03}'
        wire = run / (stem + '.db2')
        save(wire, data)
        assert corpus.inspect(data) == wanted
        native = None
        for kind in ('rust', 'native'):
            destination = run / (stem + '.' + kind + '.json')
            result = execute(kind, wire, destination, stem + '.' + kind)
            assert result.returncode == 0, (label, kind, result.returncode)
            summary = json.loads(result.stdout)
            assert summary['status'] == 'success'
            assert summary['rows'] == len(wanted['rows'])
            assert stat.S_IMODE(destination.stat().st_mode) == 0o600
            track(destination)
            actual = read_json(destination)
            corpus.verify_result(data, actual, native=kind == 'native')
            if kind == 'native':
                assert summary['direct_records'] == len(wanted['direct'])
                assert summary['copy_records'] == len(wanted['copies'])
                assert summary['get_record_calls'] == len(wanted['direct'])
                assert summary['load_calls'] == summary['auto_produce_data_calls'] == summary['auto_produce_copy_calls'] == 1
                assert summary['bytes'] == destination.stat().st_size
                native = actual
        assert native == wanted
        positives.append({'case': label, 'input_bytes': len(data),
                          'input_sha256': sha(wire), 'sections': int.from_bytes(data[200:204], 'little'),
                          'direct_records': len(wanted['direct']), 'copy_records': len(wanted['copies']),
                          'materialized_rows': len(wanted['rows']),
                          'direct_scalar_comparisons': 3 * len(wanted['direct']),
                          'materialized_scalar_comparisons': 3 * len(wanted['rows'])})
        assert wire.read_bytes() == data
    for ordinal, (label, data) in enumerate(corpus.negative_cases()):
        try:
            corpus.inspect(data)
        except AssertionError:
            pass
        else:
            raise AssertionError('qa_negative_oracle_admitted:' + label)
        stem = f'negative-{ordinal:03}'
        wire, destination = run / (stem + '.db2'), run / (stem + '.json')
        save(wire, data)
        result = execute('rust', wire, destination, stem)
        assert result.returncode == 4 and not destination.exists(), (label, 'qa_rust_negative')
        summary = json.loads(result.stdout)
        assert summary['status'] == 'error' and type(summary['code']) is str
        negatives.append({'case': label, 'code': summary['code'], 'native_executed': False})
        assert wire.read_bytes() == data
    _, data, _ = corpus.cases()[0]
    wire = run / 'ceiling.db2'
    save(wire, data)
    for ceiling in (0, 1, 17, 0xFFFFFFFF):
        label = f'ceiling-{ceiling}'
        destination = run / (label + '.json')
        result = execute('rust', wire, destination, label, ceiling)
        assert result.returncode == 4 and not destination.exists()
        assert json.loads(result.stdout) == {'status': 'error', 'code': 'record_policy_limit'}
        reductions.append({'ceiling': ceiling, 'native_executed': False})
    data = corpus.build([{'rows': [(i + 1, [i, i, i])]} for i in range(3)])
    wire = run / 'reduced-three-sections.db2'
    save(wire, data)
    destination = run / 'reduced-three-sections.json'
    result = execute('rust', wire, destination, 'reduced-three-sections', 2)
    assert result.returncode == 4 and not destination.exists()
    assert json.loads(result.stdout) == {'status': 'error', 'code': 'record_policy_limit'}
    reductions.append({'ceiling': 2, 'sections': 3, 'native_executed': False})
    # Both tools preserve an existing output exactly. Native guard returns before
    # Load; Rust may decode its safe positive input before exclusive open fails.
    _, data, _ = corpus.cases()[0]
    wire = run / 'exclusive.db2'
    save(wire, data)
    for kind in ('rust', 'native'):
        destination = run / (kind + '-existing.json')
        sentinel = b'authored-existing-output\n'
        save(destination, sentinel)
        result = execute(kind, wire, destination, kind + '-existing')
        assert result.returncode == 4 and destination.read_bytes() == sentinel
        assert json.loads(result.stdout)['status'] == 'error'
        exclusive.append({'tool': kind, 'preserved': True})
    guard()
    assert {key: sha(p) for key, p in copied.items()} == binary_before
    inventory_out = {}
    for path in sorted(run.iterdir()):
        state = path.lstat()
        assert stat.S_ISREG(state.st_mode) and state.st_nlink == 1
        assert stat.S_IMODE(state.st_mode) == (0o500 if path.name in copied else 0o600)
        inventory_out[path.name] = track(path)
    assert set(inventory_out) == set(artifacts), 'qa_artifact_inventory'
    report = {'status': 'accepted', 'profile': args.profile, 'reference': PIN,
              'provenance': provenance, 'source_root': str(root),
              'binary_sha256': binary_before, 'guarded_source_sha256': before,
              'original_inventory_sha256': sha(paths['inventory']),
              'native_build_report_sha256': sha(paths['build']), 'archives': archives,
              'positive_cases': positives, 'rust_only_negatives': negatives,
              'section_ceiling_negatives': reductions, 'exclusive_guards': exclusive,
              'totals': {key: sum(case[key] for case in positives) for key in
                         ('input_bytes', 'direct_records', 'copy_records', 'materialized_rows',
                          'direct_scalar_comparisons', 'materialized_scalar_comparisons')},
              'native_load_calls': len(positives), 'native_AutoProduceData_calls': len(positives),
              'native_AutoProduceRecordCopies_calls': len(positives),
              'network_traces': traces, 'all_network_traces_empty': True,
              'process_exits': dict(exits), 'original_sources_unchanged': True,
              'authored_sources_unchanged': True, 'binaries_unchanged': True,
              'native_malformed_executed': False, 'native_self_copy_executed': False,
              'database_execution': False, 'CASC_execution': False,
              'client_acceptance': False, 'general_compatibility': False,
              'artifacts': inventory_out}
    save(run / 'report.json', (json.dumps(report, indent=2, sort_keys=True) + '\n').encode())
    print(json.dumps({'status': 'accepted', 'profile': args.profile,
                      'positive_cases': len(positives), 'rust_only_negatives': len(negatives),
                      'report_sha256': sha(run / 'report.json')}, sort_keys=True))


if __name__ == '__main__':
    main()
