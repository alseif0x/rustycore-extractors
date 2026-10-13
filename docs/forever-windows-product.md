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

The separate opt-in `windows-msvc-seal-v2` feature selects the observed-shape
`forever-windows-producer-seal-msvc-v2` payload and parser for MSVC. Its explicit
native-AMD64 gate, metadata/fixup rules and pending execution evidence are in
[the v2 producer profile](forever-windows-producer-msvc-v2.md). The resident
native acceptance helper now builds with that feature and seals with the
explicit v2 tool profile. V1 remains the feature-free default; no prior failed
MSVC run or old bundle is relabelled as successful v2 acceptance.

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

The fifth run, functional commit `2675e87`, completes that metadata collection
and remains failed at strict v1 sealing:
[run 38345728238 / Windows job 115126572127](https://github.com/alseif0x/rustycore-extractors/actions/runs/38345728238/job/115126572127).
Both profiles declare image base `0x140000000` and DLL flags `0x8160`. The
pointed-to volatile prefix begins with six u32 words: 24, `0x80038003`, an RVA,
40, another RVA, and 24. Its two pointed spans fit the captured prefix and
contain ten apparent code RVAs and three apparent code-range pairs. This is
observed compiler output, not an official ABI or established interpretation of
the second word. Six guard slots contain code VAs; the CastGuard slot is zero.
All nine load-config VA fields have exact DIR64 fixups in the retained metadata.
These observations are inputs to profile review, without admitting any new
structure, malformed native execution or full product CLI acceptance.

The complete downloaded job-log SHA-256 is
`f74360e54ca08cf2a2b9b990536f342dedf1263e7c02069d6f94bcb84c93fbb3`;
the reconstructed diagnostic report SHA-256 is
`9253cfe81e213fe1d02effb8a41e8f9f7f944ad73c71652c553e9a38147f23f8`.
Its unchanged 1,075-file source inventory has SHA-256
`15b5e71381cced67f2fe1364e33391ad4f0c255c3ee14cff5c7df21a8d0ac6de`.
The fourth/fifth compiler PE hashes differ despite unchanged Rust sources;
no reproducible MSVC artifact or distribution is established by these runs.

## Primary API references

The implementation uses handle-relative
[NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile),
exclusive
[FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information),
and explicit
[file security](https://learn.microsoft.com/en-us/windows/win32/fileio/file-security-and-access-rights).
The narrower admitted profile and its checks are defined by this repository;
the operating-system API documentation does not prove this implementation.


The subsequent explicit MSVC-v2 implementation has separate frozen local
acceptance: 2,743 Linux workspace tests per profile, 280 contracts and 158
passing distribution tests / one platform skip. The v1 failures above remain
failed; no native v2 CLI result is inferred. See the exact source/test report
hashes and independent review in
[the MSVC-v2 acceptance record](forever-windows-producer-msvc-v2.md).


## First explicit MSVC-v2 native attempt — retained failure

Functional commit `5440090015fc618ad39ff3326eed0b1c61a87a67`,
[Rust run 38348243944 / Windows job 115134527122](https://github.com/alseif0x/rustycore-extractors/actions/runs/38348243944/job/115134527122),
passes debug/release offline v2 PE sealing and explicit-feature native MSVC
Clippy. Windows workspace acceptance is 2,359 passed / 21 ignored across 42
targets. Each focused profile passes 30 namespace, 21 package and one actual
native-AMD64 machine-query test, with zero focused ignored cases. All 82 Python
sealer tests execute (81 pass and one Unix-only skip).

The product helper then fails importing `product_fixture.py:122` under Python
3.11.9: a backslash-bearing bytes literal occurs inside an f-string expression.
The local Python 3.14 parser accepted that newer syntax. No actual product CLI
case executes, and the release product-case loop is not reached. This run is a
retained failure, not complete v2 product acceptance. The correction only moves
the synthetic CDN-key expression outside the f-string; its bytes must remain
identical and imported-fixture syntax receives explicit regression coverage.

The full-product debug candidate was sealed as
`9b896341da677e5f9883a573ad52f129b887d3ab408d1ca8210d4f5fb144612b`
from unsealed
`141264a8a39b50db1422e787180ef04b5b64dff51fad27571ee7390d9dc32364`.
These runner binaries are observed, not downloaded or retained. The complete
1,078-file product source guard remains unchanged, digest
`748be14f8393d08b9f895f6dff2932c7156274362b437db24c1ec7a717a0ca1c`.
External retained files under the existing Windows product laboratory:

- `root-logs/native-sixth-windows-job-api.log`:
  `021c944c9b29f857647900517e129f0a43a0cad67ad25ca8fba183bebb448fea`.
- `root-logs/native-sixth-retained-failure.json`:
  `b5fe2e3ef21bf0f6564a0e2955989b93ac50f8119538c9f31b364f27ed9c06e7`.

The original five v1 failures remain retained. No actual loaded-v2 producer
binding, complete product extraction/recovery, Windows distribution, real-client
or runtime acceptance is inferred from this attempt.


The subsequent fixture correction has byte-for-byte before/after evidence:
eight stores, 156 files / 1,426,566 bytes, sixteen plans, four payload inventories
and the authored toy key file are unchanged. Three source regressions check the
original backslash failure, the exact eight-module local import closure and an
isolated import discovering all fifteen cases without executing them. Local
Python is 3.14.4; the explicit legacy f-string check supplements `ast`'s feature
version option, without claiming a local Python-3.11 execution. The retained
`fixture-python311-byte-comparison.json` report has SHA256
`2b55f373d5938c5e2b4444b0f391458a1741644f955e2a7ec35085bfda3d062a`.
Rust/Cargo, both producer parsers, payloads and the workflow are unchanged from
5440090 by this correction. Native import and product CLI acceptance remain
separate gates.

## Second explicit MSVC-v2 native attempt — partial debug evidence

Functional commit `3770dc89491d8b0299c3403b26ca7ac71595a30e`,
[run 38349096456 / Windows job 115137168501](https://github.com/alseif0x/rustycore-extractors/actions/runs/38349096456/job/115137168501),
passes the Python-3.11 fixture import, both offline seal preflights, native
feature Clippy, the Windows workspace and all focused native inventories.
Fourteen of fifteen actual debug product methods pass: both build/locale
goldens, all six components, supplied toy keys, independent verification,
fresh-output equality, interruption/resume and the admitted rejection cases.
These are authored synthetic inputs on native Windows/NTFS, without original
C++ execution, real-client assets or network tracing.

The remaining method first confirms unsealed-producer rejection, then errors
inside its Python test harness: `pe.locate(data)` selects default v1 for the
explicit v2 binary and raises `pe_seal_marker` before creating the modified
candidate. The correction selects `profile=pe.PROFILE_V2` explicitly and adds
source regressions against omitted/v1 profiles. The native product suite is
still failed; release cases were not reached. The passing debug methods do
not establish the missing modified-seal check or complete two-profile acceptance.

The debug suite takes 775.175 seconds; the whole job takes approximately
19 minutes 15 seconds without reaching release. The subsequent CI plan moves
the unchanged complete helper to a dedicated `windows-product` job, keeping
the separate workspace checks, explicit preflight/lint, offline execution,
both CLI profiles and each job's 20-minute ceiling. This provides separate
time budgets without reducing the test inventories. The helper also records
run/attempt/job/repository, Windows/X64 and actual checkout SHA explicitly;
PR merge checkout and head commit are kept distinct.

The observed full-product debug binary has sealed SHA256
`41475555d7c216f4f63c36ba0997e4d7a89d46592bcb94d6de367a015ccfc798`
and unsealed SHA256
`565abd63d8534b3d37fbc3dae2ce4b007da3fadc7253d195337d5de1eae0137f`.
They are observed in the log, not downloaded. All 1,078 tracked files remain
unchanged, source-inventory SHA256
`7037bdec740f69ec688cc97c5928fc937e70071534c418b912099b3cd67a0a9d`.
Fifteen complete case reports / 36 process records are reconstructed and their
logged report hashes verified, including the failing method's partial report.
External evidence in the existing Windows product laboratory:

- `root-logs/native-seventh-windows-job-api.log`:
  `f72983d6eb04018ce59fd43cc30c8a4d549ff562ab2e73866a92f5a3799f2fc8`.
- `native-seventh-retained-failure-v2/failure.json`:
  `22b9d10403307689afab26d1cac3200d03154c7e88158bd18428e09bd5714f2e`.

The run remains failed. Complete native product acceptance, reproducible
Windows distribution, real geometry/runtime and full extraction remain gates.
