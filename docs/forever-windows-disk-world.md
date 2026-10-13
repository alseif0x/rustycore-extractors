# Windows disk-world collision source candidate

[Issue #26](https://github.com/alseif0x/rustycore-extractors/issues/26) develops
the Windows counterpart of the existing Linux six-owner disk collision pipeline.
The branch starts from `4519a0d5ca4ca0374db1be145653f7a59ae91058`, the accepted
[resident-product merge](forever-windows-product.md#accepted-native-resident-product--2026-10-13).
Implementation and cross-compilation are not native Windows/NTFS acceptance.
The native disk-world gate is pending; no real client, runtime, distribution or
full-extractor completion is established by this source candidate.

Contracts:

- [Product v7](../contracts/forever-product-contract-v7.json).
- [Windows collision disk v1](../contracts/forever-windows-disk-world-contract-v1.json).

## Explicit plan and admission

Plan v7 requires `collision-storage\twindows-exclusive-append-disk-world-v1`
and all seven ordered downward budgets from the Linux disk plan. The first six
spools admit 32 bytes through 128 GiB each; the output budget admits 1 byte
through 64 GiB and bounds the **whole six-component output**, not just collision.
Storage is never selected automatically. Older versions retain their storage
literal and admission rules. A v7 plan must select collision and build 70170 or
70205. The implicit catalog is `pinned-metadata-v1`; explicit
`wowdbdefs-70205-v1` retains its exact build/schema gates. Build 70338 remains
DB2-only v5 and is not admitted here.

Optional `inline-f32-bits-v1` offmesh records retain the exact v6 scope, order,
duplicates, bit spelling and 1,024-record ceiling. They require navigation and
the pinned metadata catalog; v7 does not extend offmesh to the target catalog.
The Windows storage selects version 7 before catalog/offmesh version selection.
Its canonical plan hash binds the storage, seven budgets, catalog and every
optional connection bit. Paths and key material do not enter the public plan.

Client commands require Windows x86_64/64-bit/MSVC, the explicit
`windows-msvc-seal-v2` feature, a successful actual native AMD64 process query,
and the loaded sealed producer before key/client/scratch/output I/O. Unsupported
platforms reject `product_disk_unsupported` after plan parsing. Plan-only v7
inspection is portable; this does not relax older Linux disk admission. V7
verification checks supported target/feature and actual native AMD64 before
output I/O, without requiring a client, keys, scratch or loaded-producer seal.

## Six-owner pipeline and temporary archives

RAW, WDT, parent cache, placement, VMO and map owners are created before startup.
The pipeline preserves GameObject startup, authenticated world reads, placement
projection, selected VMO conversion and complete package assembly. Only the
completed package is visited and sent to `Run.put`; a dependency or assembly
failure cannot publish an available collision prefix. Geometry algorithms,
record envelopes, ordinal handles, serial IDs, immediate-parent cache replay
and first-present effective-ancestor binding retain their existing contracts.
Linux spool/archive behavior remains unchanged.

The Windows backend is `windows-exclusive-append-v1`. Each archive has a private
`GENERIC_READ|GENERIC_WRITE`, `SHARE0` handle, protected owner DACL, identity and
EOF checks. The API permits writes only in one pending new extent and never
modifies a committed prefix. This is not kernel-enforced append access,
`F_SETLEASE`, an oplock, an atomic snapshot or isolation from a process with the
same SID. Generic write access also permits the required `FlushFileBuffers`.

An append guards the archive and captures EOF, writes a provisional 64-byte
record header, streams the exact body and SHA-256, patches only that new header,
flushes, then checks exact EOF and rereads the complete header/body using the
same handle and bounded 8,192-byte hash scratch. Identity settlement precedes
ordinal commit. Failed, panicking or dropped uncommitted transactions poison
the archive permanently. The 32-byte container header is checked independently.
No per-record file-metadata inventory or growing all-record guard scan is added.

## Scratch, capacity and recovery

Extract/resume require explicit scratch. Every unfinished collision attempt
creates a fresh exclusive private namespace through retained directory handles;
existing failed scratch is neither adopted nor removed. Source/key/output
ancestor exclusions, reparse/hardlink rejection, exact names and cumulative
guard checks apply throughout the run.
The new append profile requires each candidate/protected drive-root handle to
report the actual NTFS volume root. A `SUBST` mapping inside a directory rejects
before mutation because its hidden ancestors cannot establish source exclusion.
Aliases of an entire volume retain the same identity checks. This restriction
does not change the older namespace/read profiles.

`windows-fixed-ntfs-spool-volumes-v1` preflight queries caller-available bytes
and allocation units by held directory handle. Reservations aggregate by actual
volume identity, including output and scratch on the same volume; drive letters
are not the identity. Inode availability is absent. Quota-aware available space
is an observation, not an atomic reservation or a promise of later capacity.
Subsequent write/flush/guard failures remain terminal.
The underlying native query profile is
`windows-native-amd64-ntfs-available-space-v1`. Preflight counts 20 scratch
objects (namespace and lock plus six owner directory/lock/archive triples),
with 64 MiB headroom per endpoint before same-volume aggregation.

The combined logical ceiling remains 2 GiB: 258 MiB for the common product Run,
16 MiB for external guards and 1,860,173,824 bytes for conversion. Retained
archive/protection capacities and old-plus-new vector growth are charged within
their declared owner budgets; caller store residency is charged separately.
The namespace constructor has a 4 MiB logical ceiling, and preflight has a
separate 4 MiB allowance within the existing 16 MiB external guard reservation.
These are logical capacity/work bounds, not RSS, exact allocator usage or all
CPU work. Per-file, ID, geometry, navigation, output and work limits remain
bounded; disk storage does not establish unlimited world size or throughput.

Publication reuses the existing Windows product namespace, internal state and
transaction v3 and public manifest v1. The existing plan/source/producer hashes
bind recovery; no state migration or new context field is introduced. Sealed
components are verified and skipped. An unfinished producer reruns using fresh
scratch and exact retained artifact receipts. The Linux hardlink publisher is
not ported, and no power-loss durability is claimed.

## Required native acceptance

Both debug and release must execute actual fixed local NTFS append/space and
collision tests. The product corpus must match complete independent Linux disk
inventories for all six nonempty components and exercise actual disk streaming
beyond the resident 4,096 model/request and 256 MiB cumulative source/RAW limits.
Larger constants or mocked I/O are insufficient evidence.

Interruption must occur **during collision**, followed by recovery in a fresh
namespace while the previous scratch remains unchanged. The old resident
DB2-seal interruption does not satisfy this gate. Exact context/replay, loaded
PE/source/key guards, whole-product output budgets, aliases/case/reparse/hardlink
failures, resource ceilings and callback/panic poison also require execution.
Existing Linux leased and native Windows resident suites must remain green.

Native CI uses authored synthetic assets only. Until separately measured,
network attempts are `not_measured`, not zero. This corpus does not establish
new original C++ execution, native malformed-input parity, real-client geometry,
runtime integration, reproducible Windows bundles or publisher authentication.
No release or client-data publication is authorized. The first full real target
remains 70205; its missing source inputs and all component/runtime gates remain
in [the completion checklist](forever-completion.md).

The planned native job has separate 20-minute debug/release matrix budgets and
closed compiled-test inventories, including actual streaming archives and the
4,100-model complete supplied-world path. Its 17 CLI methods use authored toy
CASC and existing build-bearing Linux artifact inventories. The interruption
fixture supplies two distinct 16 MiB M2 sources to expose a first-RAW/second-read
window; it polls directory metadata and uses `TerminateProcess`, requiring its
exact CPython exit code 1. Observed RAW size greater than the container header
proves an append extent started, not that its record committed. Neither a
source-level inventory nor a skipped local run counts as native execution.
One collision-only case requires a second actual fixed local NTFS volume,
checks its distinct native volume identity, and compares the complete manifest
after moving only scratch. It does not infer two-volume acceptance from the
pure capacity arithmetic.

## Native API references

Microsoft documents the write-access requirement for
[FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers).
Its [FILE_FS_FULL_SIZE_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddk/ns-ntddk-_file_fs_full_size_information)
documentation separates caller-available allocation units from actual free units
and permits querying through an open directory handle. Those fields explain the
quota-aware observation; they do not reserve space or prove a later write.
The [NtQueryInformationFile name rules](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntqueryinformationfile)
describe full names for handles opened by name and partial names for some ID-based
opens. This append profile infers volume-root admission only from the exact
full-root result on its named handle, and rejects other results. Native alias
tests remain necessary evidence for that policy.

## Local source acceptance — 2026-10-13

The frozen Rust/Cargo union passes 2,762 ordinary workspace tests across 42
targets in each Linux debug/release profile, with 26 historical ignored cases
per profile. All 449 Rust/Cargo inputs remain unchanged. Workspace Clippy and
full formatting pass. Windows GNU all-target workspace Clippy passes defaults
and the explicit MSVC-v2 feature; its 448-file Rust/Cargo inventory excludes
`rust-toolchain.toml`, and all common hashes match the Linux inventory.
This is compilation, without Wine or native Windows execution.

The final source checks pass 307 contracts and 162 distribution tests, with one
Windows-only skip. Packager checks admit the new optional document/helper
through the same exact selection/admission tuple; they do not build a new real
bundle. A prior full contract attempt fails one stale native-test inventory
while additional tests are still being authored; that attempt stays retained.
The first release wrapper fails before Cargo because `rg` is absent from its
mandated PATH. Its later accepted attempt uses a pathlib inventory and leaves
the earlier result unchanged.

Separate bounded Linux v3 extract/verify runs execute both 70205/enUS and
70170/esES worlds through all six disk producers. Each complete 35-file,
142,622-byte inventory exactly matches the existing build-bearing golden,
including each artifact hash and byte length. All four processes have empty
denied-network syscall traces. Those Linux observations are not Windows
network measurements. A further v3 collision-only probe admits the authored
two-16-MiB-M2 interruption fixture, yielding seven files / 1,148 bytes.
No original C++ or real client is executed by these regressions.

Retained evidence is under the external
`windows-disk-world-20261013-3jcuiuo8` laboratory:

- `local-validation-v1.json`:
  `5f4c3db3ac8daf7fedb93dba74c5aa7f7a89d7f9f235f914573693a7c8e41425`;
- `lab/casc/reports/workspace-debug-validation.json`:
  `f353e3026eae5355bc7200c80ab66d7160fac1ec602b76605c80d30b23ad5c8a`;
- `lab/product/workspace-release-v2/report.json`:
  `f0f7044a698003deeef1645298a1c67684262cb5af1eaaafd8a3e9668267d9e7`;
- `lab/platform/workspace-windows-gnu-clippy-report.json`:
  `c7e180d885a711a4b81c9ec3ea48bc9257f57cf1e4f6d3f472741cda8218d914`;
- `lab/root/linux-disk-goldens-v1/report.json`:
  `9f4d1fa1729faa6eb16fad82524cbbdbc5ff43eb4c23b88a21652568a1c75b9a`;
- `lab/product/linux-v3-padded-world-v1/report.json`:
  `40c6811130b535810beaf0650994baa442444f6ec2a76938bd0efcf6d00f8e63`.

Native Windows disk-world execution remains pending. These results do not
transfer the earlier Linux bundle acceptance to this binary, close native
Windows distribution or establish real geometry/runtime/full extraction.
