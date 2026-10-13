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
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
# Diagnostic metadata only: this does not admit a PE or change any bytes. Dump
# both compiler profiles before asking the unchanged strict sealer to admit them.
$diagnostic = @'
import hashlib, json, os
from pathlib import Path
from tools import seal_forever_windows as seal
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
    load_config = None
    directory = directories[10]
    if directory['rva'] and directory['size']:
        matches = [s for s in sections if s['rva'] <= directory['rva'] and
                   directory['rva']+directory['size'] <= s['rva']+s['raw_size']]
        if len(matches) == 1:
            start = matches[0]['raw']+directory['rva']-matches[0]['rva']
            length = min(directory['size'], 512)
            if start+length <= len(data):
                load_config = {'prefix_hex': data[start:start+length].hex(),
                               'prefix_bytes': length, 'truncated': length != directory['size']}
    print('WINDOWS_PE_PREFLIGHT ' + json.dumps({'profile': profile, 'bytes': len(data),
          'sha256': hashlib.sha256(data).hexdigest(), 'directories': directories,
          'sections': sections, 'load_config': load_config,
          'diagnostic_only': True, 'admission_unchanged': True}, sort_keys=True), flush=True)
'@
$diagnostic | python -
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
foreach ($profile in @('debug', 'release')) {
    $unsealed = Join-Path (Join-Path $env:CARGO_TARGET_DIR $profile) 'forever-extractor.exe'
    $sealed = Join-Path $runRoot ($profile + '-forever-extractor.exe')
    python tools/seal_forever_windows.py seal --input $unsealed --output $sealed --zero-checksum
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
return
}

python -m unittest discover -s tests/distribution -p test_windows_seal.py -v
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

function Test-ExactInventory($package, $prefix, $expectedNames, $release) {
    $expected = @($expectedNames | ForEach-Object { "${prefix}${_}: test" } | Sort-Object)
    if ($expected.Count -eq 0) { throw 'empty native package test inventory' }
    $arguments = @('test', '--offline', '--locked', '-p', $package, '--lib', '--no-default-features')
    if ($release) { $arguments += '--release' }
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
}

cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --offline --locked -p forever-extractor --bin forever-extractor --no-default-features --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
foreach ($profile in @('debug', 'release')) {
    $profileRoot = Join-Path $runRoot $profile
    New-Item -ItemType Directory -Path $profileRoot | Out-Null
    $unsealed = Join-Path (Join-Path $env:CARGO_TARGET_DIR $profile) 'forever-extractor.exe'
    $sealed = Join-Path $profileRoot 'forever-extractor.exe'
    python tools/seal_forever_windows.py seal --input $unsealed --output $sealed --zero-checksum
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    Write-Output ('WINDOWS_PRODUCT_BINARY ' + (@{
        profile = $profile
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
