# Resident Windows product and package v1

Issue [#24](https://github.com/alseif0x/rustycore-extractors/issues/24) joins the
six existing resident producers to a separate Windows NTFS package backend.
DB2, GT, cameras, terrain, collision and navigation retain their plan, target,
format and resource gates. Build 70338 remains the separately pinned DB2-only
v5 target. Navigation v6 remains limited to builds 70170/70205.

The implementation is a source candidate until native acceptance is recorded
below. Cross-compilation is not native Windows/NTFS acceptance. Neither this
backend nor its synthetic tests completes real-client extraction, RustyCore
runtime integration, full-world resource acceptance or a Windows distribution.
No release or client-data publication is authorized.

## Filesystem and producer profiles

`forever-windows-package-namespace-v1` keeps the root and its ancestors, a
package lock, and temporary leases for the currently accessed path. Its handle
budget does not grow with the number of artifacts. Each relative open checks
the admitted volume, ordinary object, exact case-preserved spelling, identity
and protected process-owner DACL. Files with multiple hardlink names, reparse
points, case-sensitive directories and aliases reject. A detected integrity
change poisons the owner. These checks are observations, without an atomic
snapshot or protection from an administrator or the same SID.

Names admit printable ASCII components up to 255 bytes, complete relative paths
up to 512 bytes and depth 16. Win32 invalid characters, reserved DOS stems,
trailing spaces/dots and `.git` reject. This admits actual product names such as
`dbc/enUS/Map.db2`, `GameObjectModels.dtree` and 69-byte receipt basenames.
The older lowercase 64-byte private-output and membership v1 profiles remain
separate and unchanged.

The public manifest stays version 1. Internal state is version 3, identified by
`forever-windows-product-ntfs-v1`; Linux's state version 2 is not migrated or
adopted. Windows uses exclusive same-volume rename and identity-checked removal.
It does not use the Linux recovery protocol's two-hardlink publication window.
The intent binds the expected context and complete artifact fact before writing
the temporary payload, final payload, receipt and component seal. Recovery
classifies the complete observed inventory and verifies existing facts before
mutation. Unsealed facts require exact replay; sealed components may be skipped.
Complete packages reject a writer and admit shared read verification.

Common package ceilings remain 262,144 artifacts and directories, 128 MiB per
artifact, 1 TiB total payload, 256 MiB metadata and 96 MiB manifest. The native
namespace separately bounds protocol entries, logical work and live handles.
Its ceilings include 786,464 entries, four billion charged logical visits and
96 simultaneous handles. These independent ceilings do not promise that all
maximum-size dimensions can be admitted together.
Filesystem callbacks use the held reader/writer lease; reentrant public root
I/O returns `Busy` before opening another path. The callback gate releases on
return, error or unwind.
Producer limits may be smaller: the resident collision path still has its
existing 1 GiB logical-memory, 512 MiB output and 65,536-file ceilings. These
counters are not RSS, free-space reservation or all CPU work.

Windows client commands require `producer::loaded_identity()` against the
actual loaded `forever-windows-producer-seal-v1` PE. An unsealed executable or
modified digest rejects before client/output acquisition. Plan-only inspection
and offline package verification do not require a sealed producer. The seal
binds artifact identity; it does not authenticate a publisher or measure every
loaded code page. Provided-key leases survive acquisition and producers, and
their identities participate in output/source exclusions.

The Linux leased collision disk profile rejects on Windows before client,
output, producer, key or scratch I/O. Its plan syntax and Linux implementation
are unchanged. No Windows disk staging support is implied by resident support.

## Native acceptance plan and retained evidence

CI runs `tools/check_forever_windows_product.ps1` on Windows Server 2022; the
script requires a fixed local NTFS temporary volume. Debug and release list then execute
the exact namespace/package test inventories from the versioned contract.
They build the actual product, seal it with the unchanged strict PE sealer,
and run `tests/forever/test_windows_product_cli.py` with explicit new paths.
No client inputs, real keys, native C++ reference or external runtime corpus
are used. The harness does not trace network syscalls; it makes no claim that
the hosted runner's global network traffic was measured or denied.

The product corpus has nonempty authored M2/WMO geometry, placements and a
terrain/navigation tile. Its 35 final artifacts / 142,622 bytes have a frozen
Linux regression inventory at
`tests/forever/fixtures/windows-resident-product-golden-v1.json`; the separate
`windows-resident-product-70170-golden-v1.json` retains the build-70170/esES
version with its different build-bearing MAPS bytes.
Both inventories were generated with the accepted `6f1a6fa` Linux binary
`a318428a0ef70eb3fa36ab6a34325a2ba9bf495f137a6797e4cde8f3b4d9fdbf`;
extract and verify succeeded under network syscall denial with empty traces.
The comparison is a cross-platform Rust artifact regression, not a new
TrinityCore comparison or real-client/runtime acceptance.

Native CLI cases cover all six producers, enUS/esES, builds 70170/70205, v6
empty offmesh, synthetic supplied Salsa20 keys, independently reopened
verification, fresh determinism, actual `TerminateProcess` followed by resume,
wrong-context replay, complete-output exclusion, missing dependencies,
artifact/inventory corruption, loaded-seal rejection and platform admission.
The termination boundary is an observed DB2 seal before the final manifest;
it is not an injected power-loss or storage-durability experiment. Separate
package tests exercise the individual journal and publication transitions.

Native execution and CI report identities are pending. Failed runs must remain
failed and be retained alongside any corrected acceptance.

Local source acceptance passes 2,722 workspace tests / 26 ignored in each Linux
profile, Linux and Windows-GNU all-target Clippy, full formatting, 269 contracts
and 118 distribution/sealer cases. Windows evidence here is cross-compilation
only: 29 namespace and 20 package tests are authored without ignored cases.
The rebuilt Linux debug/release binaries each reproduce all 35 nonempty corpus
artifacts / 142,622 bytes, with two empty network-denial traces per profile and
unchanged client inputs/binaries. The 1,075-file source guard remains unchanged
during that final validation. Its external `root-logs/frozen-acceptance.json`
SHA-256 is `21b2f3053a7ab04d87034d42de630f2915cfebeda48b7f39361ec20882087ed6`.
Later documentation is not a rebuilt binary. Two earlier contract runs failed
only because test inventories changed during implementation; they remain
retained as failures, followed by the passing frozen-inventory run.

The first native run of functional commit `467325f` is retained as failed:
[run 38342056314 / Windows job 115115024198](https://github.com/alseif0x/rustycore-extractors/actions/runs/38342056314/job/115115024198).
Its CASC workspace target reports 741 passed, ten failed and one ignored test.
All ten failures occur when publishing a one-character artifact leaf; longer
product names pass. The rename FFI supplied 22 bytes for that case, below the
x64 structure's minimum size. The correction changes only the native request
framing and adds namespace/package short-name regressions; it does not relax
identity, locking, recovery or namespace admission. This first run did not
reach native PE sealing or the product CLI suite. The complete failed Windows
log SHA-256 is
`6cfcb2d448c5703f882f46eb8fbf3ac020c219be811da1988d095748a37cb25d`.
The original local frozen acceptance remains associated with its preceding
source, rather than this later correction.

## Primary API references

The implementation uses handle-relative
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
exclusive
[FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information),
and explicit
[file security](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).
The narrower admitted profile and its checks are defined by this repository;
the operating-system API documentation does not prove this implementation.
