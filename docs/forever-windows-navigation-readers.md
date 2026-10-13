# Windows navigation read adapters

SPDX-License-Identifier: GPL-3.0-or-later

Authored 2026-10-11. This optional library profile adds Windows readers to
`forever-navigation`; it does not enable the product CLI or publish outputs.
Actual NTFS, real geometry, server integration and full extraction remain gates.

## APIs and admission

`terrain::load::windows::{RootedSource, FsLimits, FsStats}` implements the existing
portable terrain `Source` API. Requests use exact `{map:04}_{row:02}_{column:02}.map`
names, full u32 map IDs and coordinates 0..63. Every present read transfers the
complete bounded artifact, probes actual EOF and hashes all bytes. Repeated reads
must match both the prior full identity and SHA-256. Genuine absence is retained;
a later appearance, replacement or observed mutation permanently poisons the source.
Explicit `verify_unchanged` is metadata-only, not an atomic snapshot.

`inventory::windows::{Package, Limits, Statistics}` discovers canonical four-digit
map IDs (0..9999), complete tilelists and VMap map/tile descriptors. It retains full
root/final-directory and file identities, including observable unused entries.
The same portable graph, filters and tilelist parser run before planning. Excluded
map directories are observed but their children are not traversed. Tilelists are
read completely, actual EOF checked and hashed before their bytes are dropped.

`inventory::windows::run::{execute, Sink, Selection}` executes All/Map/Tile in
serial order using fresh guarded Windows terrain and VMap readers for each tile.
Callbacks are provisional; only `Completed` admits the full selection. Earlier
callbacks can occur before a later source/sink error. A caller-owned memory sink
is supported; there is no filesystem output adapter or publication protocol here.
Source/model reads remain outside queries; the existing checked query code is unchanged.

This is an explicitly narrower filesystem admission profile than Linux. Roots must
already be normal absolute drive paths. Relative, UNC, verbatim, dot/dotdot,
reparse and device aliases are rejected without canonicalization, case folding or
prefix stripping. Platform path/component ceilings apply (including 255 UTF16
units per component). Inventory entry names additionally must be printable ASCII
components of 1..255 bytes without Windows ambiguous punctuation or trailing dot/
space. Nonregular entries, reparses and unobservable names reject even when they
would otherwise be irrelevant to the plan. Observable irrelevant files/directories
remain in the guard inventory. This does not claim equal failure admission to Linux.

## Identity, handles and errors

The public `forever-platform` read leases check held/named file identity and ancestor
object identity. Navigation uses full snapshots for each final root/directory/file.
Directory enumeration holds at most one directory lease plus its iterator; that
iterator is dropped before the lease's final check. Child identities are acquired
serially after the directory lease is dropped. File leases are dropped before
independent root checks. No handles survive a public operation, and the maximum
simultaneous handle count is 65. There is no weak metadata fallback.

Complete reads perform a final lease check on success and read/allocation/size
failure. An already observed mutation/read error takes priority over a later guard
budget refusal. Known artifacts that cannot be reacquired are changes, not missing
fallbacks. Package and rooted-source change observations are sticky. Metadata
verification does not provide an atomic filesystem snapshot or authenticate content.
Static errors contain no paths or OS error strings. Discovery exposes only the
existing public plan/statistics and complete tilelist SHA identities.

## Downward resource ceilings

Terrain source ceilings remain 128 MiB metadata capacity, 65,536 retained/visited
entries, 4,096 root bytes and 262,144 conservative verification units per operation.
Present file bytes never exceed `forever_terrain::maps::MAX_MAP_BYTES`; transfer
uses at most 16 KiB chunks (below the platform 64 MiB per-call ceiling).
Caller `ReadBuffer` memory is accounted separately. A 256 KiB transient single-lease/
path/enumeration allowance and actual chunk/vector capacities, including growth
overlap, are charged. These are logical bounds, not allocator rounding or RSS.

Inventory ceilings remain 1,000,000 observed entries/descriptors, 4,096 tilelists,
4,096 root bytes, 256 MiB logical capacity and 8,000,000 work units per discovery/
verification window. Tilelist payloads are at most 4,108 bytes. The fixed transient
allowance is 256 KiB; actual names/entry/vector capacities and old/new growth
coexistence are charged. Plan, caller DB2s and fresh pipeline/provider/store memory
retain their separate downward limits. Enumeration and failed name acquisition
consume work; platform operations reserve `32 * (path_components + 2)` units each
before I/O, including the final checks needed after an operation failure. Terrain
read cleanup uses a separate pre-admitted work allowance that body reads cannot
spend. Directory enumeration reserves its final check before acquiring the lease.
Reserved units remain charged even when a missing/error path uses fewer checks.
Already observed pipeline changes retain their original error and permanently
poison the package even when the final inventory verification refuses on limits. This is
conservative work admission and can reject earlier than the Linux syscall counter.

Read statistics count only successful complete chunk returns, including reads from
operations that subsequently fail; failed primitive prefixes and EOF probe bytes
are not included. No counter claims all kernel bytes, CPU work or heap/RSS usage.

## Evidence scope

This source increment is intended for offline synthetic Linux regression and Windows
GNU compilation, then a separately authorized bounded Wine run of these new modules.
Wine evidence is not actual NTFS acceptance. No real-client data, credentials,
network acquisition, native assets or new registry dependencies are needed. The
only lockfile change is the local Windows navigation-to-platform dependency edge.

## Narrow remaining failure-boundary qualification

The NAV-1/NAV-2 correction covers protected terrain lease/enumeration final checks
and changed pipeline-result priority in `Package::run_tile`. It does not strengthen
the inherited high-level `open_rows`/source-setup early-error boundary or change
the sequential binding policy. A planning/admission/parser failure can return
before a whole-inventory final verification. No successful package or partial mesh
is returned from such a failure, but that error is not evidence that a concurrent
mutation was excluded. These adapters retain metadata-only verification, not an
atomic snapshot, and no broader error-admission equivalence is claimed.
