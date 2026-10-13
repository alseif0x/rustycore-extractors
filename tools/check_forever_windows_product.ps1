# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-13: explicit synthetic native-Windows product acceptance.
param([switch]$PePreflight)
$ErrorActionPreference = 'Stop'
$temporaryRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
if ($temporaryRoot -notmatch '^[A-Za-z]:\\') { throw 'native product requires a normal local drive path' }
$volume = Get-Volume -DriveLetter $temporaryRoot.Substring(0, 1)
if ($volume.FileSystem -ne 'NTFS' -or $volume.DriveType -ne 'Fixed') { throw 'native product requires fixed local NTFS' }
Write-Output ('Native resident product volume: fixed NTFS; OS=' + [Environment]::OSVersion.VersionString)
$env:TEMP = $temporaryRoot
$env:TMP = $temporaryRoot
$contract = Get-Content -Raw -Encoding UTF8 contracts/forever-windows-product-contract-v1.json | ConvertFrom-Json
$runName = if ($PePreflight) { 'forever-windows-pe-preflight' } else { 'forever-windows-product' }
$runRoot = Join-Path $temporaryRoot $runName
if (Test-Path -LiteralPath $runRoot) { throw 'native product run root already exists' }
New-Item -ItemType Directory -Path $runRoot | Out-Null
$env:PYTHONPYCACHEPREFIX = Join-Path $runRoot 'python-cache'

function Get-SourceInventory {
    $names = & git ls-files
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $inventory = [ordered]@{}
    foreach ($name in $names) {
        $inventory[$name] = (Get-FileHash -Algorithm SHA256 -LiteralPath $name).Hash.ToLowerInvariant()
    }
    return $inventory
}
$sourceBefore = Get-SourceInventory
$sourceBeforeJson = $sourceBefore | ConvertTo-Json -Compress
[IO.File]::WriteAllText((Join-Path $runRoot 'source-before.json'), $sourceBeforeJson)

try {
if ($PePreflight) {
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --features windows-msvc-seal-v2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --features windows-msvc-seal-v2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Diagnostic metadata only: this does not admit a PE or change any bytes. Dump
# both compiler profiles before asking the explicitly selected v2 sealer to admit them.
$diagnostic = @'
import hashlib, json, os
from pathlib import Path
from tools import seal_forever_windows as seal

POINTER_FIELDS = ((88, 'SecurityCookie'), (112, 'GuardCFCheckFunctionPointer'),
                  (120, 'GuardCFDispatchFunctionPointer'), (256, 'VolatileMetadataPointer'),
                  (280, 'GuardXFGCheckFunctionPointer'), (288, 'GuardXFGDispatchFunctionPointer'),
                  (296, 'GuardXFGTableDispatchFunctionPointer'), (304, 'CastGuardOsDeterminedFailureMode'),
                  (312, 'GuardMemcpyFunctionPointer'))

def mapped_span(data, sections, rva, length):
    if rva < 0 or length <= 0 or rva+length > 1 << 32:
        return None
    matches = []
    for section in sections:
        extent = min(section['raw_size'], section['virtual_size'])
        if section['rva']+extent > 1 << 32:
            return None
        if rva < section['rva']+extent and section['rva'] < rva+length:
            if not (section['rva'] <= rva and rva+length <= section['rva']+extent):
                return None
        if section['rva'] <= rva and rva+length <= section['rva']+extent:
            raw = section['raw']+rva-section['rva']
            if raw+length <= len(data):
                matches.append({'section_name_hex': section['name_hex'],
                                'mapped_rva_start': section['rva'],
                                'mapped_rva_end': section['rva']+extent,
                                'mapped_raw_start': section['raw'],
                                'mapped_raw_end': section['raw']+extent,
                                'rva_start': rva, 'rva_end': rva+length,
                                'raw_start': raw, 'raw_end': raw+length})
    return matches[0] if len(matches) == 1 else None

def pointer_peek(data, sections, image_base, offset, name, va):
    record = {'name': name, 'load_config_offset': offset, 'va': va, 'rva': None,
              'bounds': None, 'prefix_hex': '', 'prefix_bytes': 0, 'initial_qword': None}
    if va is None or va == 0 or va < image_base:
        return record
    rva = va-image_base
    record['rva'] = rva
    first = mapped_span(data, sections, rva, 1)
    if first is None:
        return record
    length = min(512, first['mapped_rva_end']-rva)
    bounds = mapped_span(data, sections, rva, length)
    if bounds is not None:
        prefix = data[bounds['raw_start']:bounds['raw_end']]
        record.update(bounds=bounds, prefix_hex=prefix.hex(), prefix_bytes=length,
                      initial_qword=int.from_bytes(prefix[:8], 'little') if length >= 8 else None)
    return record

def matching_relocations(data, sections, directory, image_base, watched):
    report = {'entries': 0, 'matches': []}
    rva, size = directory['rva'], directory['size']
    if rva == 0 and size == 0:
        return report
    seal.require(rva != 0 and 8 <= size <= 1024*1024, 'diagnostic_relocation_size')
    bounds = mapped_span(data, sections, rva, size)
    seal.require(bounds is not None, 'diagnostic_relocation_bounds')
    cursor, end = bounds['raw_start'], bounds['raw_end']
    while cursor < end:
        seal.require(cursor+8 <= end, 'diagnostic_relocation_block')
        page = seal.integer(data, cursor, 4)
        block = seal.integer(data, cursor+4, 4)
        seal.require(page % 4096 == 0 and block >= 8 and block % 2 == 0 and cursor+block <= end,
                     'diagnostic_relocation_block')
        count = (block-8)//2
        seal.require(report['entries']+count <= 65536, 'diagnostic_relocation_count')
        for index in range(count):
            value = seal.integer(data, cursor+8+index*2, 2)
            kind, target = value >> 12, page+(value & 4095)
            entry_index = report['entries']+index
            if kind != 10:
                continue
            seal.require(target+8 <= 1 << 32, 'diagnostic_relocation_target')
            overlaps = [name for name, start, stop in watched if target < stop and start < target+8]
            if not overlaps:
                continue
            target_bounds = mapped_span(data, sections, target, 8)
            initial = (seal.integer(data, target_bounds['raw_start'], 8)
                       if target_bounds is not None else None)
            report['matches'].append({'kind': kind, 'target_rva': target,
                                      'target_va': image_base+target, 'entry_index': entry_index,
                                      'block_rva': rva+cursor-bounds['raw_start'],
                                      'overlaps': overlaps, 'bounds': target_bounds,
                                      'initial_qword': initial})
        report['entries'] += count
        cursor += block
    return report

for profile in ('debug', 'release'):
    data = seal._read_file(Path(os.environ['CARGO_TARGET_DIR']) / profile / 'forever-extractor.exe')
    seal.require(data[:2] == b'MZ', 'diagnostic_dos')
    pe = seal.integer(data, 0x3c, 4)
    seal.require(pe >= 64 and data[pe:pe+4] == b'PE\0\0', 'diagnostic_pe')
    seal.require(seal.integer(data, pe+4, 2) == 0x8664, 'diagnostic_machine')
    count = seal.integer(data, pe+6, 2)
    seal.require(1 <= count <= 96 and seal.integer(data, pe+20, 2) == 240, 'diagnostic_sections')
    optional = pe+24
    seal.require(seal.integer(data, optional, 2) == 0x20b and
                 seal.integer(data, optional+108, 4) == 16, 'diagnostic_optional')
    image_base = seal.integer(data, optional+24, 8)
    dll_characteristics = seal.integer(data, optional+70, 2)
    table = optional+240
    seal.require(table+count*40 <= len(data), 'diagnostic_table')
    directories = [{'index': i, 'rva': seal.integer(data, optional+112+i*8, 4),
                    'size': seal.integer(data, optional+116+i*8, 4)} for i in range(16)]
    sections = []
    for i in range(count):
        p = table+i*40
        sections.append({'name_hex': data[p:p+8].hex(), 'virtual_size': seal.integer(data, p+8, 4),
                         'rva': seal.integer(data, p+12, 4), 'raw_size': seal.integer(data, p+16, 4),
                         'raw': seal.integer(data, p+20, 4), 'flags': seal.integer(data, p+36, 4)})
    load_config, pointers, watched = None, [], []
    directory = directories[10]
    if directory['rva'] and directory['size']:
        bounds = mapped_span(data, sections, directory['rva'], directory['size'])
        if bounds is not None:
            start = bounds['raw_start']
            length = min(directory['size'], 512)
            load_config = {'prefix_hex': data[start:start+length].hex(),
                           'prefix_bytes': length, 'truncated': length != directory['size'],
                           'bounds': bounds}
            watched.append(('load_config', directory['rva'], directory['rva']+directory['size']))
            for offset, name in POINTER_FIELDS:
                va = seal.integer(data, start+offset, 8) if offset+8 <= directory['size'] else None
                record = pointer_peek(data, sections, image_base, offset, name, va)
                pointers.append(record)
                if record['bounds'] is not None:
                    watched.append((name, record['rva'], record['rva']+record['prefix_bytes']))
    relocations = matching_relocations(data, sections, directories[5], image_base, watched)
    print('WINDOWS_PE_PREFLIGHT ' + json.dumps({'profile': profile, 'bytes': len(data),
          'sha256': hashlib.sha256(data).hexdigest(), 'directories': directories,
          'sections': sections, 'image_base': image_base, 'dll_characteristics': dll_characteristics,
          'load_config': load_config, 'load_config_pointers': pointers, 'relocations': relocations,
          'diagnostic_limits': {'peek_bytes': 512, 'relocation_entries': 65536,
                                'relocation_directory_bytes': 1024*1024},
          'diagnostic_only': True, 'admission_unchanged': True}, sort_keys=True), flush=True)
'@
$diagnostic | python -
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
foreach ($profile in @('debug', 'release')) {
    $unsealed = Join-Path (Join-Path $env:CARGO_TARGET_DIR $profile) 'forever-extractor.exe'
    $sealed = Join-Path $runRoot ($profile + '-forever-extractor.exe')
    python tools/seal_forever_windows.py seal --input $unsealed --output $sealed --zero-checksum --profile forever-windows-producer-seal-msvc-v2
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
return
}

# Bind this synthetic acceptance log to its explicit CI run and actual checkout.
# GITHUB_SHA may be a PR merge commit; retain the separately queried HEAD too.
foreach ($name in @('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB',
                    'GITHUB_REPOSITORY', 'GITHUB_SHA', 'RUNNER_OS', 'RUNNER_ARCH')) {
    $value = [Environment]::GetEnvironmentVariable($name)
    if ([string]::IsNullOrWhiteSpace($value)) { throw ('missing native product run metadata: ' + $name) }
}
if ($env:RUNNER_OS -cne 'Windows' -or $env:RUNNER_ARCH -cne 'X64') { throw 'native product run requires Windows X64 metadata' }
if ($env:GITHUB_REPOSITORY -cne 'alseif0x/rustycore-extractors') { throw 'native product run repository mismatch' }
if ($env:GITHUB_SHA -cnotmatch '^[0-9a-f]{40}$') { throw 'native product run GitHub SHA invalid' }
$checkoutSha = @(& git rev-parse HEAD)
if ($LASTEXITCODE -ne 0 -or $checkoutSha.Count -ne 1 -or $checkoutSha[0] -cnotmatch '^[0-9a-f]{40}$') { throw 'native product run checkout SHA invalid' }
$runIdentity = [ordered]@{
    type = 'synthetic'
    github_run_id = $env:GITHUB_RUN_ID
    github_run_attempt = $env:GITHUB_RUN_ATTEMPT
    github_job = $env:GITHUB_JOB
    github_repository = $env:GITHUB_REPOSITORY
    github_sha = $env:GITHUB_SHA
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    checkout_sha = $checkoutSha[0]
}
Write-Output ('WINDOWS_PRODUCT_RUN ' + ($runIdentity | ConvertTo-Json -Compress))

python -m unittest discover -s tests/distribution -p test_windows_seal.py -v
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

function Test-ExactInventory($package, $prefix, $expectedNames, $release, $features = $null) {
    $expected = @($expectedNames | ForEach-Object { "${prefix}${_}: test" } | Sort-Object)
    if ($expected.Count -eq 0) { throw 'empty native package test inventory' }
    $arguments = @('test', '--offline', '--locked', '-p', $package, '--lib', '--no-default-features')
    if ($release) { $arguments += '--release' }
    if ($features) { $arguments += @('--features', $features) }
    $listing = & cargo @arguments $prefix -- --list --color never
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $listed = @($listing | Where-Object { $_.StartsWith($prefix) -and $_.EndsWith(': test') } | Sort-Object)
    if ([string]::Join("`n", $listed) -cne [string]::Join("`n", $expected)) { throw 'native package test inventory mismatch' }
    Write-Output "Selected exactly $($expected.Count) tests from $prefix; release=$release"
    $results = & cargo @arguments $prefix -- --test-threads=1 --color never
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $results | Write-Output
    $summary = '^test result: ok\. ' + $expected.Count + ' passed; 0 failed; 0 ignored;'
    if (@($results | Where-Object { $_ -match $summary }).Count -ne 1) { throw 'native package result count mismatch' }
}

foreach ($release in @($false, $true)) {
    Test-ExactInventory 'forever-platform' 'windows::package::tests::' $contract.validation.native_test_names.namespace $release
    Test-ExactInventory 'forever-casc' 'package::windows::tests::' $contract.validation.native_test_names.package $release
    foreach ($name in $contract.validation.msvc_producer_native_test_names) {
        Test-ExactInventory 'forever-platform' ("windows::producer::tests::" + $name) @('') $release 'producer-msvc-v2'
    }
}

cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --features windows-msvc-seal-v2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --features windows-msvc-seal-v2 --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
foreach ($profile in @('debug', 'release')) {
    $profileRoot = Join-Path $runRoot $profile
    New-Item -ItemType Directory -Path $profileRoot | Out-Null
    $unsealed = Join-Path (Join-Path $env:CARGO_TARGET_DIR $profile) 'forever-extractor.exe'
    $sealed = Join-Path $profileRoot 'forever-extractor.exe'
    python tools/seal_forever_windows.py seal --input $unsealed --output $sealed --zero-checksum --profile forever-windows-producer-seal-msvc-v2
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Output ('WINDOWS_PRODUCT_BINARY ' + (@{
        profile = $profile
        producer_profile = 'forever-windows-producer-seal-msvc-v2'
        cargo_features = @('windows-msvc-seal-v2')
        sealed_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $sealed).Hash.ToLowerInvariant()
        unsealed_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $unsealed).Hash.ToLowerInvariant()
    } | ConvertTo-Json -Compress))
    $env:FOREVER_WINDOWS_PRODUCT_BINARY = $sealed
    $env:FOREVER_WINDOWS_PRODUCT_UNSEALED = $unsealed
    $env:FOREVER_WINDOWS_PRODUCT_CASES = Join-Path $profileRoot 'cases'
    python -m unittest discover -s tests/forever -p test_windows_product_cli.py -v
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
} finally {
$sourceAfterJson = Get-SourceInventory | ConvertTo-Json -Compress
[IO.File]::WriteAllText((Join-Path $runRoot 'source-after.json'), $sourceAfterJson)
if ($sourceBeforeJson -cne $sourceAfterJson) { throw 'tracked source changed during native acceptance' }
$sourceDigest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($sourceBeforeJson))).ToLowerInvariant()
$sourceLabel = if ($PePreflight) { 'WINDOWS_PE_PREFLIGHT_SOURCE ' } else { 'WINDOWS_PRODUCT_SOURCE ' }
Write-Output ($sourceLabel + (@{ files = $sourceBefore.Count; sha256 = $sourceDigest; unchanged = $true } | ConvertTo-Json -Compress))
}
