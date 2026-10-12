# Private spooled serial placement consumer

`linux-private-spooled-serial-placements-v1` is a separate Linux library route.
Its bounded synthetic acceptance is recorded below. It does not replace the resident bridge or
connect world discovery, parent-cache replay, assembly or the product runner.

## Contract

`placements::spooled::Session` exclusively borrows the original
`payload::SpoolingExtractor`, owns one serial ID allocator and owns a provisional
`Sink`. Startup Data, compact handles, acquisition caches and lookup context
remain with that same collector. Lookup transitions occur before Session creation.
A projection receives an explicit request window and borrowed ordered `Batch`es.
Every top-level count, range, canonical identity, class, ready status and immutable
payload shape is preflighted before any ID assignment or payload reload.

M2 placements use compact names without a reload. Each physical WMO request,
including a repeated handle, reloads one complete authenticated private record.
The callback validates every binding in physical order against compact child
identities and statuses; fixed-width binary lookup resolves selected doodads.
No child reload, root cache, cloned name inventory or retained RAW is introduced.
DoodadOnly needs nonempty set and reference tables. Set zero precedes a valid
nonzero requested set; overlaps retain repeated references. Doodad ordinals
advance before selected-child checks and IDs. Existing projectors and one-record
`placement::encode` provide the exact established wire format.

## Accounting and failure

The full configured ID memory ceiling, caller-declared sink retained-heap
allowance and 562-byte record scratch are reserved for the Session lifetime.
A per-call vector bucket is additionally reserved under both the component and
collector budgets, leaving the full largest WMO record reload tail. Vector
growth admits the old plus FULL new capacity; actual capacities are rechecked.
Only one batch byte vector is retained and dropped after its guarded Sink call.
The sink must enforce its declared allowance, including growth overlap and
auxiliary state; borrowed Objects and source buffers are separately caller-owned
and must be charged by future orchestration. These are not allocator/RSS bounds.

Requests, emitted records and output bytes are cumulative, downward-only from
the old one-million / one-million / 64-MiB component ceilings. Metadata work
is additionally limited to one million: bindings, complete root name bytes or
MODI slots, twice the reference count, sets and group descriptors. This bounds
complete binding validation and the two possible reference scans; it is not a
counter for every binary-search comparison or every CPU operation. Existing
acquisition, descriptor, source, RAW, spool and reload ceilings stay in force.
There is no work refund, ID reset, partial success, retry or prefix fallback.

Original projection/collector/Sink errors survive independent final source and
spool guards. Changed takes diagnostic priority. All failures and early Session
Drop poison the collector and conservatively retain reservations. Successful
finish drops the actual owned IDs and Sink, then releases their exact allowance
and runs both guards before returning stats. It returns no Sink or output owner.
Previous Sink callbacks are provisional, including when finish succeeds; only a
later separately guarded whole-selection orchestrator may publish those files.

## Required acceptance

Scoped tests compare multi-call/multi-map wire bytes and serial ID statistics
with the unchanged resident Session, exercise M2 and WMO status/overlap cases,
prove exact reload counts, exercise large aggregate WMO inputs with a bounded
active record, retain startup Data accounting, and reject range/identity/budget/
Sink/source/spool failures. Full real-client and original native exporter
acceptance, WDT parent replay, all-world placement staging, VMO/final-package
assembly, product integration and production-scale discovery ceilings remain
separate gates. Synthetic evidence cannot establish full-world completion.

## Executed bounded evidence — 2026-10-12 (Europe/Madrid)

The fourth frozen attempt passes 345 collision tests per Linux debug/release
profile, CASC library/tests Clippy with warnings denied, four owned Rust-file
format checks and 17 existing product/package contract checks. Twenty-three
new tests cover the compact callbacks and new consumer; older tests remain.
The large-root comparison emits 48 placements from twelve WMO requests, with
exact resident-reference bytes and ID statistics under a 3-MiB collector/active
ceiling; the old resident collector fails at that same global ceiling. There
is one complete record reload per WMO request and zero M2/child reloads.
Multi-call, explicit-name, global-WDT and cross-class ID cases compare exact
bytes/statistics against the unchanged resident consumer. Startup Data remains
charged through Session finish. No new native or real-client execution occurs.

All six commands exit zero. Strict network traces contain only five AF_UNIX
socketpair/recvfrom pairs; three traces are empty, with no opaque records.
The source remains unchanged through those checks. Failed setup and attempts
one through three are retained. Private run: `placement-spooled-consumer-20261011T214916Z`.
Fourth-attempt report SHA256:
`a6edcce57bc79fb8a936dffba3ce67a795e1cf9b001f184de01e64b74e4dceaf`.
That evidence is for this library increment, which remains isolated from the
canonical product. The following documentation closeout reuses exact Rust
source bytes and reruns the same scoped checks; its report is retained privately.
