# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-13: explicit synthetic native-Windows product acceptance.
$ErrorActionPreference = 'Stop'
$temporaryRoot = [IO.Path]::GetFullPath($env:RUNNER_TEMP)
if ($temporaryRoot -notmatch '^[A-Za-z]:\\') { throw 'native product requires a normal local drive path' }
$volume = Get-Volume -DriveLetter $temporaryRoot.Substring(0, 1)
if ($volume.FileSystem -ne 'NTFS' -or $volume.DriveType -ne 'Fixed') { throw 'native product requires fixed local NTFS' }
Write-Output ('Native resident product volume: fixed NTFS; OS=' + [Environment]::OSVersion.VersionString)
$env:TEMP = $temporaryRoot
$env:TMP = $temporaryRoot
$contract = Get-Content -Raw -Encoding UTF8 contracts/forever-windows-product-contract-v1.json | ConvertFrom-Json
$runRoot = Join-Path $temporaryRoot 'forever-windows-product'
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
Write-Output ('WINDOWS_PRODUCT_SOURCE ' + (@{ files = $sourceBefore.Count; sha256 = $sourceDigest; unchanged = $true } | ConvertTo-Json -Compress))
}
