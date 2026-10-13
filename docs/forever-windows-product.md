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

The second native run, functional commit `159e2c0`, separately passes 2,333
Windows workspace tests / 21 ignored across 42 summaries, plus all 30 namespace
and 21 package cases in both debug and release with zero failures or ignored
focused cases. It remains an overall failure:
[run 38342630164 / Windows job 115116818646](https://github.com/alseif0x/rustycore-extractors/actions/runs/38342630164/job/115116818646)
stops at `pe_input_changed` in the Python sealer, before any product CLI case.
The failed job log SHA-256 is
`d5284d64bc29849491c396f47b865b14d773e5c206594a941fc291f5d7ba1651`.
Its 1,075-file before/after source guard is unchanged, with inventory SHA-256
`c315e6c1bdbc1a8d6305841d009388b09a18b71e0cd19ef6249c33b8bd9bcdc6`.
These positive filesystem checks do not establish whole-product acceptance.

The sealer correction uses complete Win32 identity snapshots for both the
pathname and the held data reader, before and after the bounded read. It keeps
file type, attributes, volume/index, size, creation/write times and link count.
This avoids CPython 3.11's filename-derived execute bits and sharing-conflict
pathname fallback with missing IDs, documented in its
[pathname implementation](https://github.com/python/cpython/blob/v3.11.9/Modules/posixmodule.c)
and [descriptor implementation](https://github.com/python/cpython/blob/v3.11.9/Python/fileutils.c).
Linux's existing reader and all PE layout/digest rules remain unchanged.
The local correction report SHA-256 is
`6b3e31d51c3802ced30e91061a08a145f02be25a22f12a75749532a80d768cc6`;
126 distribution/sealer cases pass and the additional actual Win32 read is
explicitly skipped on Linux. Native confirmation is a separate gate.

The subsequent native helper also executes the synthetic Python sealer suite.
Full synthetic CLI command reports are emitted to the job log, allowing their
stored JSON bytes to be reconstructed and checked against each logged digest.
The runner's temporary directories are ephemeral; no executable, complete
source inventory, fixture or output artifact is uploaded by this workflow.
Retained evidence must distinguish the downloaded job log and reconstructed
reports from those temporary files. Network observations remain unmeasured.

The third native run, functional commit `6d77c6a`, confirms the Win32 reader:
49 synthetic sealer tests pass, including the actual Windows `.exe` identity
test; one Unix-only test is excluded. All focused namespace/package inventories
again pass in both profiles. This run remains an overall failure:
[run 38343831466 / Windows job 115120582241](https://github.com/alseif0x/rustycore-extractors/actions/runs/38343831466/job/115120582241)
stops at `pe_unsupported_directory` while sealing the actual debug MSVC PE.
No full product CLI case executes. The log does not identify the rejected
directory, so this evidence does not establish that it is load-config or CFG.
The retained failed log SHA-256 is
`83ec8c170ef08aa72bd844c75ce1b94318af29999d74ff4068efe06bde54ba42`.
The 1,075-file source guard remains unchanged, with inventory SHA-256
`ceb7fcbaab2b00e27f31cb0c8f0004bd21f4efc58116743eb26b5e8068b76dc6`.

The subsequent CI plan builds both actual product profiles immediately after
the locked fetch and records bounded PE directory/section metadata and at most
512 bytes of mapped load-config data before calling the strict sealer. This
diagnostic neither modifies the input nor admits a new directory. Both profiles
are recorded before the first seal attempt; the separate preflight source guard
also runs on failure. The full workspace/product suite follows only after
successful PE admission. Local checks cover that ordering and synthetic
diagnostic bounds; native diagnosis and complete product acceptance remain
pending. No runner binary or client data is uploaded.

The fourth run, functional commit `48726f0`, fails at that preflight as intended:
[run 38344928358 / Windows job 115123998498](https://github.com/alseif0x/rustycore-extractors/actions/runs/38344928358/job/115123998498).
Both compiler profiles declare directory 10 with a 320-byte load-config record;
it is their only populated directory forbidden by v1. The record contains a
security-cookie pointer, guard slots and a volatile-metadata pointer. This is
compiler metadata evidence, not admission of those structures or execution of
their product commands. Windows workspace/product suites are skipped after the
failure; Linux debug/release pass. The complete downloaded job-log SHA-256 is
`5e6ebabf4a7d58e0b54036a7834c34b155e6f6f3bba8444d3b20d6a112635d5e`.
The reconstructed diagnostic report SHA-256 is
`fde5dd0609067d12cf5d0c4f819e8c4f119248c2988f7daec819e47da1fc733c`.
The unchanged 1,075-file preflight source inventory has SHA-256
`1919cfb7d5d28bb42f4a2f3dc71befc45f94e3fe3d58062e0b8083e033215ac3`.
The subsequent diagnostic additionally records the image base, DLL flags,
bounded pointed-to prefixes and intersecting relocations. Its purpose is to
review a separate explicit MSVC profile; it does not relax v1 or disable guard
features. Native execution of that extended diagnostic remains pending.

## Primary API references

The implementation uses handle-relative
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
exclusive
[FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information),
and explicit
[file security](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).
The narrower admitted profile and its checks are defined by this repository;
the operating-system API documentation does not prove this implementation.
