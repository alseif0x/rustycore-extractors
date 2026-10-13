# Private shared spooled-model world runner

`linux-private-spooled-shared-world-run-v1` connects the accepted spooled
GameObject startup and serial placement Session to WDT/Obj0ADT acquisition and
immediate-parent cache replay. Bounded executed evidence is recorded below. This is an
isolated Linux library addition; the canonical product is unchanged.

## Lifecycle and ordering

`run::spooled::extract` borrows the same live startup Context and consumes its
owned provisional placement Sink. It admits only one shared run per collector,
including across lookup transitions. Source requests, model cache, handles,
startup Data, RAW, acquisition work and serial IDs are never reset. The models
remain in the authenticated payload repository; the placement Sink receives no
redundant model/RAW reload callbacks. Subsequent final assembly must explicitly
select and reload completed models from that same repository.

All WDT files preload in physical Map order before the first callback. Globals
follow that order. Tiles follow depth/physical Map order, then x/y. Every present
Obj0ADT must be read and parsed completely. Empty own ADTs block fallback. Only
successful own ADTs of parent maps populate caches; inherited tiles never become
child caches, and globals never inherit. Replay is from the immediate effective
parent and preserves IDs without allocator calls. An absent cache is not a
reason to acquire or search a different tile. Original projectors/codecs and
cache byte replay remain unchanged. This is serial ordering, without native
parallel/unordered scheduling equivalence.

## Ownership and accounting

The caller supplies separate explicit cache and user-Sink heap allowances.
The cache allowance only reduces the existing run cache ceiling and includes
map/tile descriptor capacities, complete Cached bytes, and old plus FULL new
vector growth. The user Sink must enforce its own retained-heap allowance,
including temporary allocations; no owned heap may escape its lifetime.
Both allowances plus the full configured ID ceiling and fixed record scratch
are precharged for the placement Session lifetime. Small active budgets require
explicitly reduced allowances/ID ceilings, never automatic ceiling increases.

WDT source capacities/descriptors, one parser allowance, one Obj0ADT source and
parser, current placement batch and temporary replay output additionally share
the same collector ledger. Parser allowances become exact retained capacities
after scratch destruction. The owned cache is charged throughout callbacks;
replay additionally reserves the complete transient output. No actual storage
survives an exact reservation release. Sink and cache ownership drop before
Session finish releases their full reservations. Startup Data remains charged
through the existing consuming finish path. Caller Map rows/strings and source
residency remain an explicit external contract. These are logical admission
counters, not RSS, all allocator overhead or all CPU work.

## Errors and scope

All errors are sticky and preserve typed Sink/bridge/run causes. Independent
source and payload-spool guards execute on normal and error exits; Changed has
diagnostic priority. Every callback is provisional, even after success. There
is no publication, prefix fallback, retry, returned Sink or implicit cleanup.
Reports distinguish actual placement callbacks from model acquisition facts;
they do not relabel compact descriptors as RAW callbacks. Output admission
continues to include all acquired unique RAW bytes plus staged placement bytes.

WDT and parent-cache inventories remain bounded resident components in this
increment. Disk-backed WDT/cache staging, scalable final VMO/map assembly and
product integration are still required. The supplied Map graph is checked but
caller-authored; authenticated Map/liquid DB2 startup requires the later world
wrapper. Existing model/request/ID/geometry/output ceilings are unchanged and
may reject a complete real selection. No full-world, native CASC, real client,
server activation, Windows or distribution acceptance is established here.

## Executed bounded evidence — 2026-10-12 (Europe/Madrid)

The first frozen source passes 361 collision tests per debug/release profile,
CASC library/tests Clippy with warnings denied, four owned Rust-file formatting
checks and 17 static product/package checks. The second source adds explicit
named Obj0ADT, lookup-transition/unchanged cached RAW, external reservation and
combined source/spool/typed-Sink failures; all 365 collision tests pass in each
profile, with the same remaining checks. Production Rust is identical between
those sources. The new reports do not claim fresh original C++ execution.

Private run: `shared-spooled-run-20261011T2310Z`. The source body of the old
runner is byte-exact except for its Linux module export. The strengthened
complete toy-CASC comparisons retain physical/depth/coordinate ordering, exact
context and placement bytes, ID statistics, all collector facts and acquisition
deltas against the unchanged resident shared runner. A ninth parent tile
exercises full old/new descriptor overlap. The later final source adds a
larger integrated model-inventory comparison; its own report governs that case.

Second report SHA256:
`daa666c5d117a226ae99e4fb35d8a6826ad49bb6c01fe790e61cbb86bdd36791`. Both source freezes, logs, strict traces and produced executables
are retained. This remains an isolated bounded library increment; authenticated
DB2 world startup, disk-backed WDT/cache inventories, final assembly/product
integration and complete real-client/runtime acceptance remain separate.

The final review found and corrected eager parent validation in the new runner.
The established contract checks parent availability only when a missing own
tile needs replay. The 4,096-own-tile regression does not claim successful full
map extraction: the unchanged 4,096 cumulative source-read ceiling includes
startup DB2 and WDT, so both runners reject on the 4,095th ADT read. They produce
the exact same 4,095 provisional callbacks before that limit, without consulting
the inactive parent. A partial-own child rejects at the first missing tile with
ParentUnavailable and the exact same provisional prefix. Attempt4's initial
full-own success assertion failed in the unchanged resident reference at that
read ceiling; its report remains retained. No ceiling was raised. Earlier
frozen code and reports remain retained; their narrower cases did not exercise
this edge and do not establish it.
