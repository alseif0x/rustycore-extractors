# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-13: explicit synthetic native Windows disk-world acceptance.
param([Parameter(Mandatory=$true)][ValidateSet('debug', 'release')][string]$BuildProfile)
$ErrorActionPreference = 'Stop'
$temporaryRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
if ($temporaryRoot -notmatch '^[A-Za-z]:\\') { throw 'native disk product requires a normal local drive path' }
$volume = Get-Volume -DriveLetter $temporaryRoot.Substring(0, 1)
if ($volume.FileSystem -ne 'NTFS' -or $volume.DriveType -ne 'Fixed') { throw 'native disk product requires fixed local NTFS' }
$env:TEMP = $temporaryRoot
$env:TMP = $temporaryRoot
$contract = Get-Content -Raw -Encoding UTF8 contracts/forever-windows-disk-world-contract-v1.json | ConvertFrom-Json
$runRoot = Join-Path $temporaryRoot ('forever-windows-disk-world-' + $BuildProfile)
if (Test-Path -LiteralPath $runRoot) { throw 'native disk run root already exists' }
[IO.Directory]::CreateDirectory($runRoot) | Out-Null
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
foreach ($name in @('GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'GITHUB_JOB',
                    'GITHUB_REPOSITORY', 'GITHUB_SHA', 'RUNNER_OS', 'RUNNER_ARCH')) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
        throw ('missing native disk run metadata: ' + $name)
    }
}
if ($env:RUNNER_OS -cne 'Windows' -or $env:RUNNER_ARCH -cne 'X64') { throw 'native disk run requires Windows X64' }
if ($env:GITHUB_REPOSITORY -cne 'alseif0x/rustycore-extractors') { throw 'native disk run repository mismatch' }
if ($env:GITHUB_SHA -cnotmatch '^[0-9a-f]{40}$') { throw 'native disk GitHub SHA invalid' }
$checkoutSha = @(& git rev-parse HEAD)
if ($LASTEXITCODE -ne 0 -or $checkoutSha.Count -ne 1 -or $checkoutSha[0] -cnotmatch '^[0-9a-f]{40}$') {
    throw 'native disk checkout SHA invalid'
}
Write-Output ('WINDOWS_DISK_PRODUCT_RUN ' + ([ordered]@{
    type = 'synthetic'
    profile = $BuildProfile
    github_run_id = $env:GITHUB_RUN_ID
    github_run_attempt = $env:GITHUB_RUN_ATTEMPT
    github_job = $env:GITHUB_JOB
    github_repository = $env:GITHUB_REPOSITORY
    github_sha = $env:GITHUB_SHA
    runner_os = $env:RUNNER_OS
    runner_arch = $env:RUNNER_ARCH
    checkout_sha = $checkoutSha[0]
} | ConvertTo-Json -Compress))

function Test-ExactInventory($group) {
    $prefix = $group.prefix
    $expected = @($group.names | ForEach-Object { "${prefix}${_}: test" } | Sort-Object)
    if ($expected.Count -eq 0) { throw 'empty native disk test inventory' }
    $arguments = @('test', '--offline', '--locked', '-p', $group.package, '--lib', '--no-default-features')
    if ($BuildProfile -ceq 'release') { $arguments += '--release' }
    if ($group.features) { $arguments += @('--features', $group.features) }
    $listing = & cargo @arguments $prefix -- --list --color never
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $listed = @($listing | Where-Object { $_.StartsWith($prefix) -and $_.EndsWith(': test') } | Sort-Object)
    if ([string]::Join("`n", $listed) -cne [string]::Join("`n", $expected)) { throw 'native disk test inventory mismatch' }
    Write-Output "Selected exactly $($expected.Count) tests from $prefix; profile=$BuildProfile"
    & cargo @arguments $prefix -- --test-threads=1 --color never | Tee-Object -Variable results
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $summary = '^test result: ok\. ' + $expected.Count + ' passed; 0 failed; 0 ignored;'
    if (@($results | Where-Object { $_ -match $summary }).Count -ne 1) { throw 'native disk result count mismatch' }
}
$groups = @($contract.validation.native_test_groups)
if ($groups.Count -eq 0) { throw 'empty native disk test groups' }
foreach ($group in $groups) { Test-ExactInventory $group }

$arguments = @('build', '--offline', '--locked', '-p', 'forever-extractor', '--bin', 'forever-extractor',
               '--no-default-features', '--features', 'windows-msvc-seal-v2')
if ($BuildProfile -ceq 'release') { $arguments += '--release' }
& cargo @arguments
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$unsealed = Join-Path (Join-Path $env:CARGO_TARGET_DIR $BuildProfile) 'forever-extractor.exe'
$sealed = Join-Path $runRoot 'forever-extractor.exe'
python tools/seal_forever_windows.py seal --input $unsealed --output $sealed --zero-checksum --profile forever-windows-producer-seal-msvc-v2
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Output ('WINDOWS_DISK_PRODUCT_BINARY ' + (@{
    profile = $BuildProfile
    producer_profile = 'forever-windows-producer-seal-msvc-v2'
    cargo_features = @('windows-msvc-seal-v2')
    sealed_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $sealed).Hash.ToLowerInvariant()
    unsealed_sha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $unsealed).Hash.ToLowerInvariant()
} | ConvertTo-Json -Compress))
$env:FOREVER_WINDOWS_PRODUCT_BINARY = $sealed
$secondary = @(Get-Volume | Where-Object {
    $_.FileSystem -eq 'NTFS' -and $_.DriveType -eq 'Fixed' -and $_.DriveLetter -and
    $_.DriveLetter -ne $temporaryRoot.Substring(0, 1)
} | Sort-Object DriveLetter | Select-Object -First 1)
if ($secondary.Count -ne 1) { throw 'explicit second fixed local NTFS fixture volume unavailable' }
$otherParent = Join-Path ($secondary[0].DriveLetter + ':\') (
    'forever-disk-world-' + $env:GITHUB_RUN_ID + '-' + $env:GITHUB_RUN_ATTEMPT + '-' + $BuildProfile)
if (Test-Path -LiteralPath $otherParent) { throw 'second-volume fixture parent already exists' }
[IO.Directory]::CreateDirectory($otherParent) | Out-Null
$env:FOREVER_WINDOWS_DISK_OTHER_VOLUME_PARENT = $otherParent
$env:FOREVER_WINDOWS_PRODUCT_UNSEALED = $unsealed
$env:FOREVER_WINDOWS_PRODUCT_CASES = Join-Path $runRoot 'cases'
# List independently before executing; skips are not native acceptance.
$listing = @'
import json, unittest
from pathlib import Path
suite = unittest.defaultTestLoader.discover('tests/forever', pattern='test_windows_disk_product_cli.py')
def flatten(node):
    for case in node:
        if isinstance(case, unittest.TestSuite):
            yield from flatten(case)
        else:
            yield case
cases = list(flatten(suite))
contract = json.loads(Path('contracts/forever-windows-disk-world-contract-v1.json').read_text())
expected = contract['validation']['cli_test_names']
if sorted(case._testMethodName for case in cases) != expected:
    raise RuntimeError('native disk CLI inventory mismatch')
if any(getattr(case.__class__, '__unittest_skip__', False) for case in cases):
    raise RuntimeError('native disk CLI would skip')
print('WINDOWS_DISK_PRODUCT_CLI_INVENTORY ' + json.dumps(expected))
'@
$listing | python -
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
python -m unittest discover -s tests/forever -p test_windows_disk_product_cli.py -v
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
$sourceAfterJson = Get-SourceInventory | ConvertTo-Json -Compress
[IO.File]::WriteAllText((Join-Path $runRoot 'source-after.json'), $sourceAfterJson)
if ($sourceBeforeJson -cne $sourceAfterJson) { throw 'tracked source changed during native disk acceptance' }
$sourceDigest = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($sourceBeforeJson))).ToLowerInvariant()
Write-Output ('WINDOWS_DISK_PRODUCT_SOURCE ' + (@{
    profile = $BuildProfile; files = $sourceBefore.Count; sha256 = $sourceDigest; unchanged = $true
} | ConvertTo-Json -Compress))
}
