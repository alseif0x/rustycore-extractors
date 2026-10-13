<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Private spooled GameObject startup and row-wise lists

The explicit Linux `collision::gameobjects::spooled::extract_startup_fdid`
consumer uses `linux-private-spooled-gameobject-startup-v1`. The pure
`forever_vmap::gameobject_models::streaming` builder uses
`gameobject-model-lists-rowwise-v1`. The original resident startup, list builder,
Error enums and Limits remain unchanged; their modules only export these new
children. No product, placement/world or assembly adapter is enabled.

## Complete startup, same collector

The consumer reads the complete GameObjectDisplayInfo table through guarded
source/external-reservation wrappers on the same `SpoolingExtractor`. The existing
fixed numeric DB2 decoder validates all direct fields and copies. Copies are
never replayed. Physical direct order, duplicate IDs/FDIDs, repaired IDs, zero
FDIDs, textures, M2 NoCollision and WMO DoodadOnly follow the resident startup.
The same EmptyLookup, handles, cached magic, recursive children and cumulative
counters survive `Context::with_lookup`; only new models use the later lookup.

At each conversion completion RAW and all large WMO metadata remain externalized
by the accepted private spool. Mapping retains only class and optional RAW/full
record lengths in addition to its handle/ordinal. Actual Mapping capacities are
charged. The RCEPLD1 disk layout is unchanged. Ready immutable descriptors and
sizes provide list preflight without a preliminary RAW/root reload. Every actual
callback checks complete record length, RAW presence/length and WMO class shape
against that metadata, in addition to retained identities, SHA-256 and EOF.

## Whole metadata preflight, one record per physical row

Before list-stage RAW parsing/allocation, the pure Plan admits every name,
model presence/class, per-row RAW size, exact list lengths and cumulative repeated
RAW-byte visits. Projection vertex counts are unavailable from sizes: their
unchanged cumulative quota is enforced during strict RAW parsing before each
group's vertex projection. No invented vertex-count or CPU/RSS claim is made.

A temporary metadata vector borrows compact names only. Both complete list
buffers and a small per-row class/length/bound-offset plan are allocated once.
IDs/names are encoded directly into those private provisional buffers; bounds
are patched only after their physical row succeeds. There is no all-RAW/root,
borrowed geometry or all-row bound cache. The metadata vector drops before any
mutable reload. Exactly one full-record reload is attempted per listed physical
row, including duplicates. WMO records include root/bindings/selection/group
facts even when only RAW is projected. This extra repeated metadata I/O remains
charged to the existing spool reload ceilings.

The pure builder retains original newest-equal-extrema semantics, including
signed-zero bits, by using the existing merge operation. It exposes no partial
Lists or output-byte accessor; errors are sticky. Complete unchanged list readers
validate both outputs at finish. The lists themselves remain complete resident
buffers, with bounded O(rows) outcomes/plans: this is not constant-memory output.

## Global, active and retained-output accounting

Store residency + repository/compact descriptors + external source/decoder/
snapshot/outcomes/plans/lists + current payload share the existing collision
memory ceiling. The current spool profile conservatively also shares its active
allowance between external work and current conversion/reload; no cap increases
or second uncharged pool are introduced. Vectors reserve full requested new
capacity before allocation while existing live capacities remain charged, and
actual capacities are checked afterward. The decoder's 2MiB+32*input allowance
remains conservative admission, not instrumented BTreeMap allocation or RSS.

After outcomes/snapshot accounting, the consumer computes the largest COMPLETE
selected record R and admits the sum of repeated record lengths/counts against
remaining reload quotas before the first reload. This admission does not consume
counters; each actual attempt consumes them exactly once, with no refund.
Builder reservation is `min(list memory cap, free bytes - R)`, leaving a whole
largest record feasible instead of reserving all free memory for lists. Empty
selection uses R=0 and still requires both eight-byte magic headers. After the
plan drops, unused reservation is released and actual returned lists/outcomes
remain charged in Context.

A private exact-charge finish transfer retains Data's capacity in the returned
SpoolingExtraction. Subsequent reload subtracts it from global and active
budgets, even after `Completed::into_parts` or the caller drops Data. There is no
public undercharge or implicit refund helper. Original `SpoolingExtractor::finish`
still requires zero outstanding external/source capacity.

## Errors and guard boundaries

All source/reservation/reload operations and final startup/finish boundaries run
independent LocalStore and spool guards on success and error. Pure DB2/list/
allocation failures also poison the collector. New Error retains the original
Cause plus an optional final guard; its code gives InputChanged/Changed priority.
The generic payload callback bridge preserves the original operation error,
without collapsing a list/DB2 failure into a policy code. Context/Completed data
access checks readiness after observed failures. Caller-owned copies already
returned cannot be revoked; verification is not an atomic filesystem snapshot.

The private spool's explicit protected-input declaration, procfs/same-mount,
nofollow/identity/hash/EOF and no cleanup/publication/resume policy remain in
force. Completed files/partial failures are preserved; this is synthetic offline
library work without client or key reads, network, native or Windows acceptance.

## Remaining full-world gates

This increment removes global RAW/WMO-root retention from this startup consumer
and reloads row-by-row. Existing 4096 model/request and cumulative RAW/source caps
remain deliberate limits and full-world blockers. Placement Session WMO roots,
WDT/parent caches, one serial ID session, all-map dir_bin, selected VMOs/assembled
outputs and product provisional-to-sealed publication remain separate work.
Navigation, real inputs and runtime acceptance are unchanged. This is not a full
extractor or full-world scalability claim.
