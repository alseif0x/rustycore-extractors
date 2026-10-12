# AGENTS.md

Shared operating guide for AI agents and human contributors working in
`rustycore-extractors`. Keep it factual and update it when the repository contract changes.

## Project and source of truth

`rustycore-extractors` produces standalone, versioned client-data extraction tools for
RustyCore. The first delivery isolates the proven C++ tools; the second replaces them
incrementally with Rust behind the same compatibility contract.

- Repository checkout: `/home/server/rustycore-extractors`
- Public remote: `https://github.com/alseif0x/rustycore-extractors.git`
- Integration repository: `/home/server/rustycore`
- C++ reference repository: `/home/server/woltk-trinity-legacy`
- Fixed initial upstream: `TrinityCoreLegacy/TrinityCore`, branch `3.4.3`, commit
  `92796557f9b0ba3d2d1c7c770f535153154cf83e`
- Plan/index: `https://github.com/alseif0x/rustycore-extractors/issues/13`
- Default development branch: `forever`; legacy version line: `3.4.3`

### Forever branch scope

The operator made this repository public, renamed `main` to `3.4.3`, and selected
`forever` as default on 2026-10-03. Forever's development target is modern WoW
`1.60.1` Beta x64. The operator additionally authorized build 70205 alongside
70170 for the bounded CASC read prototype. The complete extraction suite remains
the C++ 3.4.3 implementation. The Forever development branch also contains a
Rust WDC5 inspector, an optional Rust/CascLib local-read prototype, and a bounded
Rust DB2 decoder with 56 reviewed schemas (2026-10-11), an explicit batch CLI
and a v2 output verifier. Empty tables and supported unencrypted single-section
records have separate synthetic/local evidence in `docs/forever-rust-foundation.md`.
This is not full extraction: neither the full Rust-native replacement nor full
build-70170/70205 extraction compatibility is established.
The 2026-10-11 build-70205/esES acceptance validated 54 available tables against
the Python oracle/output verifier (141,594 decoded rows), four tables through
RustyCore's typed names loader, seven through its typed appearance loader, and
three through its item specialization loader. Eight tables are empty in that
client, including `ItemSpec` and `GemProperties`; their populated coverage is synthetic.
The five Birth schemas have synthetic coverage, including the complete typed
loader. Only four have complete local artifacts: `SkillLineAbility` fails with
`local_file_incomplete`. Its incomplete batch is rejected by the verifier;
the real Birth probe stops before invoking the typed loader. Full real Birth
acceptance remains blocked, and no available-prefix fallback is admitted.
All seven initialization schemas also have synthetic complete-loader evidence.
`PowerType` and `Movie` are complete local inputs; `Map` is
`local_file_incomplete` and blocks real typed initialization. Both incomplete
batches fail output verification; neither real typed loader executes during
the failing probes. An independent strict enUS attempt recovered neither file.
Four scalar spell dependency tables additionally match every ID and numeric
field through RustyCore's generic `Wdc4Reader` (270 rows, including 19 copies).
That four-table comparison does not invoke the 49-table `SpellRecords::load`
API. Synthetic comparisons
retain the generic reader's differences for copy chains, overwrites, source
zero and packed offsets; do not infer general copy/compression compatibility.
Typed `Cfg_Categories` acceptance is limited to ID/creation charset. Typed
acceptance covers `load` projections, without `finish` or SQL/hotfix overlays.
Parent lookup is optional for schemas with a parent; zero-section record
decoding requires complete primitive field metadata. See the recorded limits;
do not infer complete initialization, build-70170 or runtime compatibility.
The fixed target and issue #13 below describe the legacy suite/programme, not
an authorization to relabel its formats, metadata or acceptance as Forever.
Target-specific changes need versioned evidence and reader/output acceptance.

The explicit pinned metadata API now describes all 1,126 DB2 metadata
declarations, with synthetic value-shape coverage while preserving the 56
reviewed schemas and their hash gates. A separate diagnostic run and then an
explicit production v3 catalog batch accepted 21 additional local files /
5,302 rows against the Python oracle; the v3 schema/output verifier passes.
They did not run complete typed loaders or alter older manifests. The
78-file runtime preflight retains 30 `local_file_incomplete` results.
The six-file GT extraction/v1 verifier has real build-70205/esES evidence;
the subsequent v2 catalog run accepts all 20 source files (138,537 bytes),
22,082 exact f32 comparisons in 14 numeric tables, six raw-only files, and
both complete public RustyCore GT loaders for their six-file subset.
Default/all remains the six-file v1 plan. The camera component has complete
local evidence for ten raw camera files plus their source DB2 (26,962 bytes),
with independent discovery, exact pinned C++ bytes and v1 verification;
it does not parse M2 or establish waypoint/playback compatibility.
The pure Rust local-store API integrates explicit v7 indexes, authenticated
config/ENCODING/ROOT and bounded N/Z decoding. Its 59-request acceptance
matches 57 complete files / 528,633 bytes against pinned C++ artifacts and
rejects the two known incomplete inputs. Production Rust-local CLI acceptance
now passes five overlapping DB2/GT/camera plans and 61 artifact comparisons,
with DB2 v4, GT v3 and camera v2 profiles and offline verification. Native
remains default; unavailable native rejects before I/O, and Rust-local requires
explicit layouts and only permits downward file-budget overrides. The bounded sparse decoder accepts `ConversationLine` with
558 exact Rust/Python rows; no RustyCore sparse-loader acceptance is claimed.
The WDT/ADT, height/liquid projection and MHGT/MLIQ/MAPS foundation has 154
synthetic tests. Independent pinned C++ kernels match 47 height/area/hole
projections, 126 MHGT sections, 99 liquid projections and 311 MLIQ sections.
The typed MAPS assembler matches 380 synthetic complete files against an
independent layout and those C++ section kernels; all pass strict verification
and 54,340 height queries through RustyCore's unchanged GridMap API. This does
not execute the complete C++ ADT extractor or higher-level tile selection,
area/liquid/flight queries. Closed terrain DB2 source readers separately pass
89 positive and 36 negative synthetic cases and local LiquidType/LiquidMaterial
comparisons. Map remains incomplete; its selected null string pointers reject
explicitly instead of relying on the C++ consumer's undefined behavior.
The complete 1,126-table local survey decoded 1,023 artifacts and 719,466
rows against the Python oracle; 103 inputs remain explicit failures. Its frozen
binaries predate the separate sparse/text-budget changes, and failed batches
remain failed. The pure `conversion::convert_adt` API now composes a complete
supplied ADT with bounded context, liquid lookups, MFBO and one shared packing
option; its 20 additional synthetic tests pass. That integrated path now matches
248 complete synthetic outputs across two runs against the original pinned
C++ parser and ConvertADT, with strict verification and 35,464 exact GridMap
height queries; the dense run reaches the exact 202,112-byte MAPS maximum.
Twenty-two malformed/policy cases reject only through the Rust caller; no
native malformed-input safety or rejection parity is inferred. Real terrain
acquisition and full runtime acceptance remain separate gates. Follow the full component
gates and retained reports in `docs/forever-completion.md`; none of these
additions establishes full extraction, geometry or runtime compatibility.

Terrain dependency planning now has 22 passing synthetic tests for full-map,
all-map and explicit tile selections followed by sequential WDT expansion;
the planner itself performs no acquisition. A separate local terrain runner now
loads Map/LiquidMaterial/LiquidObject/LiquidType, then complete WDT/root ADTs,
converts MAPS and sends provisional tile/list outputs to a caller-owned sink.
Any present-tile failure rejects the whole selection. Tile subsets never emit
whole-map tile lists. The Linux QA example creates a new private output only
after successful conversion; it has no product manifest or resume policy.
Debug/release each pass 20 synthetic CLI cases, 20 existing-output guards and
three boundary/source guards. Each matches 48 MAPS and 24 tile lists with C++,
plus 6,864 exact queries through the actual RustyCore GridMap source. Native
CASC/DB2/WDT orchestration is not executed by that comparison. Workspace tests
pass 1,584 library tests and four example tests, with all-target Clippy; 46
terrain tests include the 22 preceding planner cases. One explicit corpus test
is ignored normally and executed separately in both profiles. The bounded real
two-tile attempt fails at the first Map read (`local_store_no_verified_encoding`),
with zero converted tiles, no output directory and no network attempts.
Complete real terrain, product publication and runtime acceptance remain gates;
details and retained reports are in docs/forever-terrain.md.
The pure `forever-vmap`
M2 collision parser/raw writer/verifier has 33 passing tests and 53 exact
synthetic outputs against original C++ coordinate conversion and writing.
That initial comparison did not execute `Model::open`. A subsequent complete
synthetic-input comparison executes 55 original `Model::open` calls through a
memory adapter, matching 53 raw files and two no-collision outcomes. It does
not execute the original assembler reader, placements, BIH or RustyCore
collision runtime. Raw INDX's
declared-u16/physical-u32 length discrepancy is preserved deliberately; do not
apply the M2-specific raw header rules to WMO. See `docs/forever-collision.md`.

Optional LocalStore name activation now exposes the separate conservative
`linux-ascii-stable-root-names-v1` capability. Thirty-two pure and fourteen
store tests pass; 135 synthetic cases / 3,468 queries match the original
C++ FileTree and name hash for the admitted profile. Real build-70205/esES
activation succeeds, but all 52 explicit candidate queries are absent in both
backends: this is negative evidence, not positive real named-file acquisition.
Session, native product bridge, CLI and manifest profiles remain FDID-only.
WMO borrowed root/group parsing and small-mode geometry projection have 27
tests, with strict framing/GFID order and checked offsets. Liquid mapping and
complete supplied-WMO raw conversion/verification add 13 and 19 tests. Native
comparison accepts 155 exact synthetic raw outputs and five matching
no-collision outcomes against original C++ parsing/writing. The raw reader
preserves WMO triangle counts and u32 indices; emitted liquid grids require
nonzero tiles and the assembler's matching height product. Pure WMO doodad
selection adds 19 tests; the bounded offline M2/WMO collector adds 33 tests,
including 796 native normalization vectors. It preserves owned source/doodad
metadata, rejects ambiguous output aliases and exposes no partial collection
after errors. Typed FDID/name routing is explicit; it does not reproduce the
original pseudo-name fallback or complete exporter startup sequence. Separate
placement and supplied-input VMap assembly APIs are described below; complete
real WMO/runtime acceptance remains pending.
Bounded build-70205/esES M2 acceptance matches 31 complete acquired files /
15,277,222 bytes with strict native CASC and 28 raw outputs / 27,100 bytes
plus three no-collision outcomes with original Model::open/writing. The
integrated collector reproduces all 31. Its 32-request plan rejects one
file_unavailable before output creation; no available-prefix fallback or
full-model-discovery claim is admitted. Private correlation fields, content
MD5s and exact output inventory are checked locally. This does not exercise
real WMO, placements, the original full exporter or collision runtime.
Pure direct MDDF/MODF placement projection and strict dir_bin encoding/reading
add 29 tests. Their 205 synthetic outputs / 15,710 bytes match original M2/WMO
projectors, independent wire calculations and original ModelSpawn read/write
roundtrips. IDs are caller-supplied; the original global allocator is not run.
This does not establish placement discovery, WMO doodad transforms, parent
traversal, spatial assembly or runtime compatibility.
The later object/ID/doodad increment adds 59 pure tests: complete ADT/WDT object
chunk readers, bounded serial ID allocation and one-doodad quaternion projection.
In both dev and release profiles, 573 synthetic doodad outputs / 31,894 bytes
match original ExtractSet and ModelSpawn read/write; 96,576 serial ID requests
match original GenerateUniqueObjectId. Native threaded/unordered order and
portable libm equality are not established. The Rust allocator additionally
rejects numeric collisions, wrap and excessive probes. Native QA retains two
original G3D implementation units externally with their notices, never in Git.
A pure collision placement bridge validates exact completed-model identities,
expands sorted references and returns all-or-error dir_bin batches. Its 29 tests
include four complete synthetic LocalStore-to-placement paths. It reserves the
ID memory ceiling conservatively and performs no parent-cache replay or I/O.
Real object/WMO discovery, parent traversal, BIH, assembled VMap and runtime
acceptance remain pending; these are not full extraction claims.
The later VMap Map projection retains WDT-zero rows and signed effective parents;
105 positive and 13 negative synthetic cases match Python. The graph/WDT planner
rejects duplicate/cyclic/missing/copy-only parent relationships and selects Obj0ADT
in explicit serial order. A persistent placement Session spans calls. Parent cache
replay matches original initFromCache on 124 synthetic cases / 11,144 records.
The local runner now joins WDT acquisition, global placements, own Obj0ADTs,
conversion and immediate-parent cache replay through a staging Sink; 20 synthetic
LocalStore integration tests pass. Requested read failures are fatal, empty own
ADTs block fallback, globals are not inherited and inherited tiles never populate
the child's cache. Cross-batch alias/deduplication and publication belong to the
caller. This does not execute the complete original exporter or accept real object
placements. Real Map remains incomplete; assembled VMap and runtime gates remain.
The pure BIH builder/codec/strict reader adds 27 tests. Both dev and release
comparisons match 420 synthetic outputs / 245,480 primitives / 5,545,784 bytes
against original construction and read/write roundtrips, including real depth65,
BVH2 and both absent-child forms. Empty qNaN bounds and dead infinity-marked
offsets are deliberate. The reader validates structure, not containment of
unseen geometry. That construction/storage comparison did not exercise query
traversal, VMap assembly or runtime loading; later increments below are separate.
Original G3D AABox/Vector3/g3dmath implementation units and all notices remain
external; sources and complete network traces are guarded in the retained reports.
The pure raw-to-VMO converter and geometry-aware reader add 22 tests. Both dev
and release match 251 complete synthetic `.vmo` outputs / 3,738,601 bytes
against original TileAssembler::convertRawFile and WorldModel read/write,
covering 823 groups, 205,233 vertices, 67,877 triangles and 114 liquids.
The reader binds mesh BIHs to triangles and group BIHs to declared group bounds;
it does not require group bounds to enclose vertices or canonical tree bytes.
Strict raw admission is retained; empty models and vertices-without-triangles
have separate reader-only evidence. This is supplied-model assembly, not map
assembly or real-model/runtime acceptance. Original sources, Boost archive,
synthetic provenance, binaries and complete network traces are guarded.
Transformed placement bounds and fresh single-map assembly add 16 and 21 tests.
Both dev and release match 274 bounds cases and 136 complete synthetic maps
against original calculateTransformedBound/readMapSpawns/convertMap, producing
62,006 exact map/tile/index files / 21,800,784 bytes per profile. Existing bounds
skip raw reads; missing bounds use raw group zero. First-ID-wins selection sorts
unsigned IDs, and tile indexes refer to that sequence rather than BIH permutation
positions. Own records precede parent records; parent-only tiles omit payloads.
Tile coordinates use f32 multiplication and truncation toward zero, within the
reviewed X0..32767/Y0..255 domain. No 0..63 clipping is added. Strict readers and
supplied-proof association checks do not authenticate the proof or execute
ancestor resolution, native MapTree runtime or RustyCore collision queries.
Forty-nine Rust-only negatives and two native existing-stage checks pass.
Full real acquisition and product integration remain pending. Native QA retains
complete byte/identity/inventory checks without
claiming crash durability for temporary outputs. Original source, build, binary,
corpus and network guards pass. That map increment passed 1258 library tests
(533 CASC, 274 DB2, 154 terrain, 297 VMap), two example tests and Clippy.
The pure GameObject list builder and complete borrowed codecs add 23 tests.
Both dev and release match 276 synthetic cases / 536 rows / 24,895 dtree bytes
against original public exportGameobjectModels, scanning 410,587 vertices and
6,709,056 cumulative raw bytes per profile. Bounds include every vertex in every
raw group, independent of declared bounds, triangles and liquids; complete raw
metadata validation still applies. Equal extrema retain the newest vertex bits.
Models resolve per row; the caller must enforce immutable same-name class/content
binding for a file inventory. Missing, malformed or vertex-empty models fail the
whole Rust build. Limits include 64 MiB per list/raw, existing stricter raw-reader
caps, 256 MiB cumulative raw visits and heap, and 128 million projection vertex
visits. Names are ASCII 1..499 without NUL in the codec; the Rust file QA caller
uses printable basenames up to 255 bytes. Exact-zero bounds serialize but native
GameObjectModel::initialize rejects them; no consumer, initialization or query
acceptance is established. That codec increment passed 1281
library tests (533 CASC, 274 DB2, 154 terrain, 320 VMap), two example tests and
Clippy. Source/build/binary/corpus/archive/network guards and 27 Rust-only
negatives pass; this does not establish complete extraction or runtime support.
The separate GameObjectDisplayInfo source adapter adds 19 tests and matches
50 synthetic files / 1,164 direct rows / 10 unreplayed copy pairs against original
DB2FileLoader in both dev and release. Physical field 1 is signed32 and cast to
u32 without changing bits (native lookup slot 7 includes ID plus six GeoBox
scalars). All nine fields/fourteen scalars validate; ignored GeoBox float bits
have no finiteness gate. Direct order, duplicate IDs/FDIDs, repaired IDs and zero
FDIDs remain in the snapshot; copies validate without a materialize operation.
QA-only filtering/naming yields 1,120 nonzero candidates; it performs no model
acquisition. The existing regular/minimal-empty profile, strict EOF and resource
caps remain. The 71,428-row scalar boundary and 100k-copy cases are Rust unit
evidence, not native boundary comparisons. That workspace passed 1300
library tests (533 CASC, 293 DB2, 154 terrain, 320 VMap), two examples and Clippy.
No original GameObject producer, CASC or runtime is executed by the DB2 comparison.
The subsequent bounded GameObject startup now connects complete DB2 reading,
first-four-byte classification and model conversion to both complete lists.
It uses explicit FDIDs, empty liquid lookups and the same verified source buffer;
cached typed models retain their original magic for later classification.
Zero FDIDs, textures, M2 NoCollision and WMO DoodadOnly are normal omissions.
Direct order and duplicates survive, copies are never replayed, and malformed or
incomplete dependencies are terminal. Context.with_lookup preserves the same
collector/cache/handles/counters; previous WMO raw is not reconverted.
Limits include 16 MiB DB2, 71,428 rows and 4,096 nonzero candidate requests,
including duplicates. Actual source and retained vector capacities are charged;
the decoder's 2 MiB + 32 * input-length reservation is a conservative admission
allowance, not an exact portable heap or RSS bound.
Dev/release each match 36 synthetic startup cases: 469 direct rows produce 127
listed rows, 46 raw files / 6,855 bytes and 9,720 bytes across both lists.
Original ExtractGameobjectModels, DB2 getters, cache, M2/WMO converters and dtree
export run over an authored memory inventory. Rust reads complete synthetic CASC
using explicit FDIDs/enUS/Contiguous/Header50893V2; native CASC routing equivalence
is not established. Seven Rust negatives and two exclusive-output guards pass
per profile, with unchanged inputs/sources/binaries and no network attempt.
The startup increment added 40 CASC tests and one DB2 test: its workspace passed
1,341 library tests (573 CASC, 294 DB2, 154 terrain, 320 VMap), two examples,
165 focused collision tests and all-target Clippy. That startup-only step's
native later-liquid check and Rust unit coverage did not by themselves establish
joint later-phase byte parity.
The QA example writes private outputs only after finish; it is not a product CLI
or atomic publication protocol. The older `collision::run::extract` API retains
its fresh collector per batch. The new consuming `collision::world::extract`
connects startup to authenticated Map, LiquidMaterial and LiquidType reads and
`run::extract_shared` on the same collector. Cached WMO raw, handles and all
cumulative counters survive lookup changes; one shared run is admitted per
collector and any failure poisons it. Map strings/rows, liquid vectors and graph
capacities remain charged until dropped before final finish. Text-bearing DB2s
reserve an additional 64 MiB cumulative-text allowance; this remains conservative
admission accounting, not exact allocator/RSS measurement. The shared QA example
stages bounded per-map `dir_bin` bytes and creates private output only after
whole success. The shared-world dev/release comparisons each pass 49 complete
synthetic cases: 43 listed rows, 82 RAW files / 13,537 bytes, 3,880 bytes across
both lists, and 79 per-map dir_bin files / 11,688 bytes / 176 placements. Fifteen
Rust negatives and two exclusivity guards pass per profile. Original startup,
Map/liquid loaders, ParsMapFiles, projectors, cache and dtree export execute with
Threads=1 against a memory inventory; Rust uses complete toy CASC. This closes
the synthetic joint later-phase byte comparison, not native CASC routing,
parallel/unordered scheduling or real-client acceptance. All source/build/binary/
input/Boost guards pass; each profile records 115 empty network traces,
98 exit-0 and 17 expected exit-4 processes. The workspace adds 37 CASC tests
(20 shared-run, two request-window, 15 world) and passes 1,378 library tests
(610 CASC, 294 DB2, 154 terrain, 320 VMap), two example tests, 202 focused
collision tests and all-target Clippy. Incomplete real Map/GameObject sources,
versioned product publication and full extraction/runtime acceptance remain
gates. See `docs/forever-collision.md`.
The subsequent `collision::assemble::extract(Startup, Limits)` consumes the same
world through a private placement stage and returns the complete final package.
Selected VMOs are the union of all pre-deduplication placement names and startup
list names; unused acquired dependencies are excluded. Output preserves nested
`<map>/<map>.vmtree`, `<map>/<map>_<x>_<y>.vmtile` and `.vmtileidx` names, with
VMOs and dtree at the root. Parent-only tiles retain child indexes and bind to
the first present payload in the actual effective-parent chain; a mismatch is
terminal, with no farther fallback. This differs from extractor cache traversal.
Limits only reduce 1 GiB logical memory, a 256 MiB staging reservation, 512 MiB
artifact bytes and 65,536 files, plus cumulative geometry, RAW, BIH, ancestor
and proof-work caps. Final extraction/data/topology capacities and caller-owned
store residency are charged separately without double counting; staging and
allocation overlap remain charged. The counters are not RSS or all CPU work.
The final debug/release profiles each match 53 complete synthetic cases:
650 final files / 56,019 bytes, 87 selected VMOs, 180 unique map spawns and
303 associations, including 27 parent-only tiles and one mixed tile. Original
startup/world processing and convertWorld2 execute against a memory inventory;
Rust reads complete toy CASC. Forty Rust-only negatives include 25 strict parent
association failures; two exclusivity guards pass. All guards pass, with 148
empty network traces per profile (106 exit-0, 42 expected exit-4). The workspace
adds 35 CASC tests and passes 1,413 library tests (645 CASC, 294 DB2, 154 terrain,
320 VMap), two examples, 237 focused collision tests and all-target Clippy.
Final report names/hashes are retained in docs/forever-collision.md; the earlier
1,378-test world evidence remains historical. Native CASC routing, original
parallel/unordered scheduling and runtime consumers are outside this evidence.
The private QA entry point writes final `vmaps/` artifacts only after successful
assembly and complete inventory preflight; it is not product publication.
At that assembly checkpoint RustyCore had only the LOS-provider interface.
The later checked terrain library below connects binary readers/queries;
the server still does not instantiate that capability.
Full real inputs, runtime acceptance, navigation and product publication remain
goal gates; this does not complete the extractor.
The pure VMap library now implements bounded model and instance ray queries:
`query::{Ray, Mode, Limits, Budget}` and
`query::model::{intersect, instance, Instance, Hit}`. Rays are finite, nonzero
and never normalized; maximum distance is a ray parameter and permits positive
infinity. Instance scales may be negative but must be finite, nonzero and have
finite reciprocals. Original cofactor-inverse, candidate-plane AABox, triangle
comparison and single-group bypass operations are preserved. Any-hit follows
traversal order; nearest-hit updates only on a strictly closer positive distance.
A cumulative budget only reduces 16 million node visits and 16 million triangle
tests, with 65 stack entries per nested tree and no query heap allocation.
Invalid transforms, exhausted budgets and executed unsafe infinity-marked tree
edges are errors, not misses. These are Rust work counters, not native counters,
all CPU operations or an RSS bound. The workspace passes 1,467 library tests
(645 CASC, 294 DB2, 154 terrain, 374 VMap), two example tests and all-target Clippy;
54 new VMap tests cover queries. Final debug/release each match 79 synthetic
cases / 14,416 queries against original WorldModel and ModelInstance consumers:
2,950 hits and 11,466 misses, 11,850 direct-model and 2,566 instance queries,
with exact result bits and no exclusions. Each profile also passes 14 Rust-only
negatives and two exclusivity guards, with 174 empty network traces (158 exit-0,
16 expected exit-4). Original WorldModel::readFile executes 79 times and spawn
reading 17 times per profile. Native query counters are not compared. The pinned
closure has 24 original TUs, including 16 G3D, with 124 inventory entries and
117 actual original dependency files plus the wrapper; no Boost is used. Build,
source, binary, input and trace guards pass; report hashes are retained in
`docs/forever-collision.md`. Both profiles use the same cases, with identical
case results/totals. Existing assembled-package and older reports remain
historical. No StaticMapTree/VMapManager package loading, filename/model
binding, RustyCore provider installation, area/liquid query, dynamic GameObject
state, real-client query or full runtime acceptance is implied.

The static package library now owns complete validated map trees, tiles/indexes,
exact-name shared VMOs and cached instance transforms. It provides world/internal
LOS, height and object-hit methods; Linux RootedSource anchors complete reads,
checks identities/EOF and hashes every read. Explicit verify_unchanged is
metadata-only and is not an atomic snapshot. Failed loads retain prior state;
observed input changes permanently invalidate the source/store. Repeated loads
append references; one tile unload releases all repeated references for that
tile. Parent binding always uses the child index and first present payload,
allowing only parent bit2 to differ from a known binding. Missing-model and
atomic-failure policy is stricter than native; failure-admission parity is not
claimed. Queries perform no I/O/reparse/allocation. Store capacities and source
metadata have separate downward-only limits documented in forever-collision.md.
The static-store checkpoint passed 1,558 library tests (645 CASC, 294 DB2, 154 terrain,
465 VMap), two example tests, 66 focused store tests and all-target Clippy.
Debug/release each match 75 synthetic static-manager cases/3,298 operations:
339 loads, 305 unloads and 2,654 world LOS/height/object-hit queries, with exact
state counts/result bits and 38,274 independent field checks. Fifty-eight Rust
negatives, two exclusivity guards and 210 empty traces pass per profile.
Original 25-TU native closure uses PERFORMANCE_PROFILING (logging, metrics and
ordinary Trinity assertions disabled); only reviewed complete positives run
after the identical Rust plan succeeds. Strict Rust failure/rollback behavior
is not native failure-admission parity. No malformed native execution is claimed.
The previous pure-query acceptance is rechecked on the owned-reader adapters;
real inputs, RustyCore provider installation, area/liquid/dynamic queries,
navigation and full product completion remain gates.

RustyCore's unpublished `forever-extractor-static-terrain` branch now composes
the actual GridMap reader and rooted StaticStore under an optional Linux
`forever-terrain` feature. A checked owner and wow-world facade supply static
height/LOS/object-hit and shared physical-tile leases with explicit cleanup.
Complete MAPS, primary/cosmetic topology and phase-resolved map IDs are supplied;
build 70170/70205 is declared, not authenticated. Queries do no I/O and cannot
turn missing coverage or source errors into clear LOS. Twenty-three focused
tests include 1,210 exact original C++ scalar height-selector cases; map/world/
data/database suites and workspace all-target compilation pass. General final
validation still fails inherited RustyCore architecture policy and the unchanged
extractor placement.rs formatting defect. This is source-hash-pinned synthetic
library acceptance, not real geometry, server activation or release dependency
acceptance. The server/managed-map lifecycle remains unwired; creation/world
entry stays paused. See docs/forever-collision.md and the integration runbook.

The pure `forever-navigation` input stage now projects checked MAPS into
solid/liquid geometry, explicit border/neighborhood selections and first-index
vertex compaction. Coordinates use filename row/column order, both 0..63.
NoHeight omits solid terrain; DarkWater and partial liquid repairs preserve the
original consumer behavior. Errors return no partial mesh. Downward ceilings are
five regions, 250,000 vertices, 500,000 triangles and 24 MiB logical capacity.
Debug/release each match 106 complete synthetic cases / 50,763,162 bytes against
original TerrainBuilder region/public-neighbor loading and cleanVertices, with
2,948,213 vertices and 1,246,502 triangles; 107 empty network traces per profile.
Twenty-one focused tests bring workspace library acceptance to 1,605, with four
example tests and all-target Clippy. The two ignored corpus tests have separate
explicit acceptance. No VMap geometry loading, ancestor fallback, real input,
Recast/Detour generation, MMap serialization or pathfinding runtime is established.
See docs/forever-navigation.md for exact scope, limits, reports and open gates.

Navigation now also appends complete supplied VMO/placement geometry to a terrain
prefix using copy-only accessors on the existing checked VMap Instance transform.
The previous constructor/query implementation remains unchanged. M2 winding
reverses; rectangular WMO liquids preserve their original indexing and retain
unknown/DarkWater types as area zero. Model liquids ignore the terrain skip flag,
matching the pinned loadVMapModel consumer. The sorted borrowed LiquidTypes table
must come from direct rows with last-ID-wins, without DB2 copy replay; banks >=31
reject the native signed-shift undefined domain. Off-mesh selection retains exact
target/order/duplicates and permutes endpoints without mirroring. Models/groups
are capped at 4,096/65,536 plus the preceding mesh ceilings; the consumed prefix
and replacement capacity are charged together. Connections have a separate
65,536-record / 2 MiB ceiling. All limits only reduce; caller input residency and
allocator overhead remain outside these logical budgets. Both profiles match
97 synthetic cases / 3,951,732 bytes against original consumers, with 239 model/
placement reads, 336 groups, nine terrain prefixes and 67,552 connections.
Previous terrain case arrays stay identical in both profiles; 410 empty network
traces cover both corpora. Navigation now passes 41 tests; workspace libraries
pass 1,625, four examples and all-target Clippy. Three ignored corpora retain
separate explicit evidence. Actual model-name/placement binding, selected map
inventory, parent/path-only filtering, real acquisition, Recast/Detour generation,
MMap output and runtime routes remain gates. This is not complete navigation.

The initial Rust Recast stage now classifies/rasterizes solids, applies the
three ordered walkability filters, rasterizes liquids and builds compact-cell
connections. Explicit grid/agent inputs do not select original map/subtile
configuration. It preserves 16-bit heights and Forever's merge-before-expansion
area rule; numeric, layer and capacity failures return no partial field.
Debug/release each match 162 synthetic cases / 2,628,564 bytes against original
Recast, including six intermediate fields per case and 37 composed API cases.
The Rust adaptation retains the original Zlib notice and dated modification
headers; unchanged native sources and their notices remain external. No new
Cargo dependency was added. Twenty-three new tests bring navigation to 64 and
workspace libraries to 1,648, with four examples and all-target Clippy. All
326 runtime network traces are empty. Budgets separately cap grid cells at
1,048,576, arena slots at 2,000,000, raster-call vertices/triangles at
2,000,000/1,000,000, heap capacity at 128 MiB and cumulative charged work at
128 million. Caller inputs and allocator overhead are excluded. More than
255 active spans per column, reachable layer >=63 and quantized lower height
65535 reject; native malformed/admission parity is not claimed. That checkpoint
did not include erosion or distance fields; their subsequent evidence follows.

Compact preparation now consumes the field through original-order erosion,
median area filtering and blurred distance generation. Geometry and connections
remain unchanged while areas may become null; distance boundaries use equal
areas, including null equality. Radius 0..127 is admitted; larger values reject
native threshold-byte wrap. The graph retains its inherited capacity/work limits,
charging scratch capacity and cumulative traversal. DistanceField binds the
immutable graph to its distances, exposing the pre-blur maximum separately.
Debug/release each match 164 cases / 65,069,060 bytes, with 634 compact snapshots,
531,071 compact spans and 69 composed calls; radius saturation and distance 512
have native evidence. The 162 earlier heightfield case arrays remain identical
in both profiles. Slopes::default is corrected to the executed native 55/55,
while the prior corpus explicitly selects 70/55. Seventeen new tests bring
navigation to 81 and workspace libraries to 1,665; four examples and Clippy pass.
All 656 runtime network traces are empty. Subsequent watershed evidence follows.

DistanceField::build_regions now implements the original watershed buckets,
staged expansion, flood, border paints, adjacency/floors, component filtering,
merging and ordered remapping. It preserves native empty IDs in the reported
maximum and exposes nonfatal overlap IDs explicitly. The native comparison
verifies public warning counts; overlap-vector IDs have separate synthetic
internal evidence. Both profiles match 191 cases / 13,870,280 bytes, with
162,781 compact spans, 114,038 ordinary-region spans, 18,897 border spans and
two nonzero warnings. Three paired cases exercise threshold-dependent merges.
Region counters reaching the border bit and adjacency walks unfinished at the
native loop guard reject without partial results. Existing capacity/work limits
carry through all buffers and traversals; growth charges old/new capacities.
Thirteen added tests bring navigation to 94 and workspace libraries to 1,678,
with four examples and Clippy. The 164/162 compact/heightfield case arrays remain
identical in both profiles; all 1,040 runtime network traces are empty. Region
adjacency walks are not the later rcBuildContours geometry API; that separate
increment follows. See docs/forever-navigation.md for exact evidence.

Regions::build_contours now traces raw corners, simplifies with native f32
deviation, subdivides explicit wall/area edges and preserves flags, padding and
ordered hole joins. Merged holes retain raw arrays and metadata with empty
simplified arrays. It preserves the pinned hole-ordinal diagonal index only
when the slot is initialized; uninitialized or overflowing scratch, incomplete
walks and unresolved joins reject. Encounter-order sort ties have evidence for
the pinned Linux profile, not general qsort/Windows compatibility. Numeric
admission requires finite error and square, edge length <=46340 and padding
that fits both dimensions. All stages retain the inherited capacity/work caps.
Both profiles match 155 cases / 5,896,408 bytes: 152 complete supplied-span
pipelines plus three explicitly supplied single-region cases joining 2, 3 and
8 holes. The latter establish contour behavior, not watershed generation of
those labels. The corpus includes 633 contour entries, 4,970 simplified and
27,528 raw vertices, with 19 merged hole slots. Fourteen new tests bring
navigation to 108 and workspace libraries to 1,692, plus four examples and
Clippy. The prior 191/164/162 region/compact/heightfield arrays remain identical
in both profiles; all 1,352 runtime network traces are empty. The subsequent
polygon increment follows; the full Rust extractor goal remains open.

ContourSet::build_mesh now implements original ordered ear clipping (including
the loose fallback), two-height-unit vertex welding, convex polygon merges,
border-vertex removal/repair, mixed-region metadata, adjacency and portals.
The explicit polygon width is 3..=6, matching the pinned Detour ceiling; flags
remain zero as in rcBuildPolyMesh. It retains the contour set and graph for
later detail construction. Native total input vertices >=65534 reject; Rust
also rejects polygon/edge indices colliding with portal/null flags, incomplete
triangulation and disconnected removal boundaries. Existing heap/work caps
carry through scratch and result storage. This is supplied-contour component
evidence, not complete TileBuilder or safe routing for arbitrary geometry.
Both profiles match 652 cases / 688,088 bytes, covering 16,258 vertices, 9,952
polygons, 1,442 portal edges, 18,530 linked edges and 79 mixed-region polygons.
Four additional reversed-outline cases return native partial-triangulation
warnings and are rejected by Rust. The prior 155 contour outputs are reused at
four polygon widths; nine additional synthetic shapes cover removal, mixed
regions/areas, welding and malformed contours. Seventeen added tests bring
navigation to 125 and workspace libraries to 1,709, plus four examples and
Clippy. All 155/191/164/162 prior contour/region/compact/heightfield arrays remain
identical in both profiles, with 2,666 empty network traces. Detail construction
and merging are recorded below; Detour/MMap output, complete real inputs and
usable routes remain required.

PolyMesh::build_detail now retains the original polygon mesh and builds native
height patches, edge samples, hull triangulation, jittered interior refinement,
Delaunay rebuilds and boundary flags. World vertices preserve the extra y cell
height. The explicit sample distance must be positive and finite; error must
be nonnegative with a finite square. Original dangling-face recovery uses the
same swap-removal order and exposes its warning count. Missing heights,
unreachable center seeds, invalid numeric domains and capacity exhaustion
reject without partial output. The inherited heap/work caps cover scratch and
results; each detail polygon admits at most 127 vertices and 255 triangles.
Debug/release match 149 cases / 6,477,240 bytes, including 613 submeshes, 6,637
vertices and 6,736 triangles. Eight cases explicitly override region metadata
to exercise center seeding; this does not establish watershed generation of
those labels. One dense case matches all output and six native dangling-face
warnings, without claiming safe routing. Sixteen new tests bring navigation
to 141 and workspace libraries to 1,725, plus four examples and Clippy. All
656/155/191/164/162 preceding case arrays remain unchanged in both profiles;
2,966 runtime network traces are empty. This is synthetic supplied-span
component evidence, not complete TileBuilder, real navmesh or full extractor
acceptance. See docs/forever-navigation.md for exact scope and retained reports.

DetailMesh::into_tile_mesh moves the paired polygon/detail arrays and releases
the compact graph and contours. TileMesh::merge now reproduces original ordered
polygon/detail merging: rounded x/z shifts, two-unit height welding, unchanged
region/area/flag metadata, exterior portals, rebuilt adjacency, descriptor base
offsets and unchanged detail coordinates/indices. Native merged maxpolys,
borderSize and maxEdgeError remain zero; they are not reconstructed metadata.
Inputs must share cell sizes, vertical origin and polygon width. Coordinate
wrap and marker collisions reject. MergeLimits admits at most 4,096 subtiles,
128 MiB of combined input/scratch/result capacities and 128-million merge work
units. Prior stage work and detail recovery counts are retained separately as
provenance; a merge does not inherit one subtile's work allowance for all inputs.
Debug/release match 330 cases / 1,665,448 bytes, including 3,319 supplied meshes,
8,504 welded vertices, 5,635 polygons/submeshes, 43,465 detail vertices and
38,820 triangles. The corpus reuses all 149 prior detail outputs individually
and in translated 2x2 groups. Four authored 625-square grids use target widths
40/80 in both input orders; each yields 676 vertices, 100 exterior portals and
2,400 reciprocal links. These supplied grids do not establish complete native
TileBuilder configuration/rasterization or real source acquisition. Eighteen
new tests bring navigation to 159 and workspace libraries to 1,743 plus four
examples and Clippy. All 149/656/155/191/164/162 earlier case arrays remain
unchanged in both profiles; 3,628 runtime network traces are empty. The later
Detour evidence below does not complete real inputs or full navigation.

The pure detour::build API now emits explicit little-endian Detour 7 tile
payloads for the 64-bit-reference ABI from paired TileMesh geometry. It preserves
polygon/portal encoding, compressed detail arrays, off-mesh classification and
height culling, link capacity and optional detail-based BV trees. Forever flag
projection rejects undefined negative shifts; Preserve mode retains flags.
BV equal-key ordering matches the current Linux native closure, without a
Windows qsort claim. Native u16 detail-cursor wrapping/overwrite and one-based
neighbor marker collisions are preserved and counted explicitly; native loading
of those diagnostic cases does not establish valid routes. Policy ceilings are
65,536 input connections, 64 MiB output, 128 MiB owned heap and 128-million work
units; borrowed source residency is separate. Nineteen new tests bring navigation
to 178 and workspace libraries to 1,762 plus four examples and Clippy.
Debug/release each match 728 exact payloads / 18,533,660 bytes and 40 native
empty-input rejections. All 330 previous merge outputs are reused with BV on/off.
Original Detour loads 728 and queries 149 cases: 147 complete, two expected
partial paths. Eight supplied 625-grid variants produce 49-polygon routes;
directed gap cases preserve one-way/bidirectional reachability. RustyCore's
unchanged public add_tile/remove_tile and nearest/path/straight APIs reproduce
all 728 loads and 149 query projections. The supplied MmapTileBlob truthfully
declares version 16, but add_tile reads only data: this is raw-payload evidence,
not acceptance of its legacy MMap 15 parser or a Forever container. Successful
status detail bits are dropped by that wrapper; returned data and destination
reachability are compared. All 330/149/656/155/191/164/162 prior case arrays remain
identical, with 5,894 empty runtime network traces. See docs/forever-navigation.md.
The later container evidence below is separate from this raw-payload checkpoint.

The mmap module now writes and strictly reads Forever version 16 global files
(40-byte header plus 44-byte OffMeshData records) and tiles (20-byte header plus
unchanged Detour 7 payload). Global output filters on map ID while retaining
input order, duplicates, all raw off-mesh bits and original coordinate axes.
The native maxPolys signed-minimum sentinel is preserved. Headers are explicit
little endian; readers reject wrong versions, missing/trailing bytes, invalid
parameters, wrong-map records, noncanonical tile flags/padding and inconsistent
Detour block lengths. They borrow inputs and establish framing, not topology or
arbitrary-input FFI safety. Off-mesh numeric/coordinate admission remains with
selection/Detour; raw records can contain nonfinite values. File/path helpers
use Forever's mmaps/{map:04}_{row:02}_{column:02}.mmtile separators. Parameters
are explicit; original parent/tile-inventory derivation is not implemented here.
Policy ceilings are 65,536 supplied records and 64 MiB + 20 bytes per file/owned
buffer; borrowed residency and runtime allocation are separate. Sixteen tests
bring navigation to 194 and workspace libraries to 1,778 plus four examples.
Debug/release each match 762 global files / 3,054,248 bytes / 68,722 selected
records and 728 tile files / 18,548,220 bytes against original native structs.
The native wrapper initializes original Detour from every emitted global and
loads/removes all tiles; it does not execute full MapBuilder/MMapManager.
The new strict readers feed unchanged actual RustyCore public Detour APIs:
728 loads/removals and 149 exact query projections, with 147 complete paths and
two expected partial paths. Its legacy parser explicitly still rejects v16;
no relabelling or runtime bootstrap installation occurs. All 768 preceding
Detour case arrays remain identical in both profiles. Clippy and scoped checks
pass, with 3,792 empty runtime network traces for this increment. See
docs/forever-navigation.md. Complete TileBuilder configuration, real source
inventory/acquisition, product orchestration and high-level runtime routing
remain required; the full extractor goal stays active.

The tilebuilder module now derives target configuration, bounds and all 625
ordered padded grids from supplied geometry. It composes every Recast stage,
merging and pristine Detour generation with a whole-operation heap/work budget.
Map 562 changes radius after border calculation; map 48 changes ch after detail
error calculation. Caller-selected complete unique ancestor tile inventories
produce global parameters; parent selection itself remains external. Any failed
subtile aborts the operation; native continue-on-failure partial builds are not
accepted. Debug/release match 107 configurations and 23 full supplied-geometry
builds against original buildMoveMapTile, producing 22 pristine payloads and
one empty result. Actual RustyCore public APIs reproduce 11 complete paths,
one partial path and ten absent-nearest errors; these negatives remain explicit.
Both profiles retain the same case data and source hashes. Fourteen focused tests and workspace Clippy pass (1,792 library tests plus four
examples; navigation 208). The earlier MMap 762-global/728-tile case arrays
remain identical in both profiles; 1,765 runtime network traces are empty.

A new exact-output gate was found by running original saveMoveMapTileToFile:
it calls navMesh.addTile before fwrite. This mutates polygon/link arrays and can
snap off-mesh endpoints; all 22 emitted original files differ from a header plus
the pristine payload. The focused flat-tile probe records 18,047 differing bytes
in a 103,912-byte payload. Do not claim the current mmap::encode_tile output is
the complete original producer file, even when runtime load/query data agrees.
Each ordinary original worker creates a fresh navmesh per tile; this fact is
relevant to deterministic initial salt/index state. The new
Data::prepare_for_save / tilebuilder::Data::into_saved path now ports fresh
index-0/salt-1 addTile state in pure Rust, including free/internal links,
128-candidate nearest projection, detail height/boundary fallback and local
off-mesh start/end connections. mmap::encode_saved_tile accepts its consumed
SavedData type; encode_tile remains the pristine codec. It reuses the byte
allocation with fixed stack scratch and a downward 128-million-work limit.
Builder detail-wrap and neighbor-marker diagnostics reject before mutation.
Debug/release each compare 750 cases: 747 exact saved payloads / 10,916,236
bytes, three diagnostic rejections, and all 22 original TileBuilder final files
exact. Strict MMap 16 readers and unchanged RustyCore public APIs load/remove
747; 171 queries reproduce 158 complete paths, three partial paths and ten
expected absent-nearest errors. Ten new tests bring navigation to 218 and
workspace libraries to 1,802 plus four examples; Clippy passes. This is bounded
supplied-geometry offline producer evidence, not arbitrary-input safety,
multi-tile runtime state, complete product orchestration or real/world acceptance.
See docs/forever-navigation.md for versioned reports. The full goal stays active.

The common Linux `forever_casc::package` stage and `forever-package verify`
implement artifact-integrity v1 with explicit context fingerprints, selected
component seals, receipts, bounded hashes, namespace closure and an incomplete
marker. This is not semantic/client/runtime acceptance. Seal only after the
corresponding producer returns complete. The final manifest stays v1. Newly
initialized internal state v2 permits explicit
same-context `Run::resume` after full preflight, retaining the incomplete marker.
Every retained unsealed artifact must be re-emitted before its component seals;
verified published component seals may skip finished producers. Existing complete
packages remain immutable; incomplete legacy v1 is not migrated. Supply every
client/reference root as a protected input. Three synthetic packages per
debug/release profile preserve 18 previously accepted navigation files /
1,433,160 bytes, and two 1,024-file inventories have identical manifests.
This increment reuses earlier native/reader evidence; it does not execute those
consumers again. State-v2 restart acceptance adds 31 focused recovery tests,
16 inventory and six navigation SIGKILL recoveries, mandatory retained-file
replay coverage and legacy complete-v1 verification. Same-context resume does
not checkpoint unfinished computation or supply a unified product driver.
See `docs/forever-package.md`.

The Linux `forever-extractor` product v1 now connects all six existing Rust
producers through canonical explicit TSV plans and inspect/extract/resume/verify.
It preserves final package v1/internal state v2. Context fingerprints bind the
canonical plan, loaded executable and versioned logical ROOT/ENCODING collection;
the last is not physical archive availability or a full archive snapshot. Every
read remains complete and content-checked. DB2 is catalog-backed complete decoding;
GT retains 14 numeric/six raw-only files; cameras remain opaque raw acquisition.
Navigation requires selected DB2 Map/LiquidType, terrain all and collision; no
external partial inventory or offmesh input is silently used. Complete empty Map
worlds admit the pure empty navigation plan for All only. Verified finished
components skip; unfinished producers rerun and replay retained artifacts.
Product verification adds exact DB2/GT/camera sets, explicit map/tile selections
and bounded intrinsic format checks through a shared-lock rooted callback API,
with final state/namespace revalidation. It does not prove full geometry inventory,
VMap associations, Detour topology or runtime compatibility. Debug/release each
pass 2,030 Rust tests, 72 contract checks and 17 synthetic CLI scenarios, including
six actual SIGKILL recoveries across profiles. One-tile six-component packages
retain 31 files/141,786 bytes/profile. Bounded real build-70205/esES extraction
matches 33 prior accepted DB2/GT/camera artifacts/193,794 bytes; incomplete Map
still rejects with no payloads. No new native/runtime consumer execution,
publication, Windows distribution or full extraction is claimed. See
`docs/forever-product.md` and retained `product-20261011T142606Z` base/supplement
reports. Other component gates remain open.

Do not trust an extractor because it builds or finishes. TrinityCoreLegacy output plus
RustyCore reader/runtime acceptance is the compatibility authority.

### Navigation source and inventory acceptance (2026-10-11)

The numeric Map source projection and supplied-descriptor navigation planner now
preserve original direct-record order, last-ID-wins assignment and effective
parent selection (negative ParentMapID uses CosmeticParentMapID); copies are
validated without replay. The planner unions terrain/VMap tiles, retains empty
entries, applies original filters and derives global parameters from the ultimate
ancestor's own inventory. Explicit map selection overrides filters. Complete
source graphs, tile-list framing/builds and ancestor inventories are required;
missing/cyclic topology and nonempty outputs with empty roots reject.
Debug/release each match 82 synthetic packages and 140 global files against the
pinned original DB2 getters and MapBuilder discovery/global construction.
Strict global decoding feeds unchanged RustyCore initialization and 3,500 exact
coordinate queries. Navigation passes 233 tests, with 299 DB2 tests and Clippy.
This is supplied-descriptor planning; it does not discover/authenticate a real
filesystem inventory or load selected source geometry. No real client was read
for this increment. The existing RustyCore 28-byte global parameter parser
accepts a 40-byte MMap 16 header as misplaced parameters; use the strict versioned
reader. The separately checked legacy tile parser rejects version 16.
The checked static store also exposes allocation-free active-model borrows in
original primitive-slot order. Navigation consumes cached transforms/owned VMO
groups, and its tile adapter loads then unloads the requested tile even on
projection failure. Path-only loading is mandatory; other active tiles remain
visible and one unload removes all references to the requested tile. Strict
source failures stay errors. Debug/release each match 154 synthetic packages /
1,507 operations against original VMapManager/TerrainBuilder, including 356
actual loadVMap calls. The older 97-case supplied-model regression is unchanged.
Navigation now passes 239 tests and VMap 467; no real acquisition or runtime
activation is established by this model bridge.
Terrain navigation loading now selects independently for each center/neighbor
through the validated effective-parent chain. Genuine absence permits fallback;
present flat/triangle-empty centers suppress neighbors, while corrupt/read/change
errors reject. The Linux rooted MAPS source hashes complete files, checks root/
file identities and bounds transfer/metadata capacity. Plan retains its build
for strict MAPS verification. Debug/release each match actual public C++ loadMap
for 110 synthetic cases / 217,721,658 bytes, including depth64 and both builds.
Twenty-five new tests include a 1,024-node/5,120-read boundary. Navigation now
passes 264 tests; the 82-case inventory arrays are unchanged. This supplies
terrain selection/loading, without authenticating a complete real inventory or
joining the complete source-to-TileBuilder/product/runtime pipeline.
The next navigation increment joins one selected tile's immutable MAPS/VMap
sources, compaction, explicit offmesh connections, all 625 subtiles, fresh Detour
save mutation and MMap 16 framing. Direct LiquidType reduction keeps the last
direct ID, including zero, and ignores DB2 copies. Inventory retains separate
terrain/collision promises; optional VMap absence never admits present partial
packages. The pipeline requires an exclusive store with the same effective
parent graph and path-only model policy, and releases target instances even on
projection failure. Debug/release each match nine complete synthetic scenarios:
seven final MMap files / 452,012 bytes and two empty outcomes. Original public
buildTile executes behind an explicit fresh-tile selection gate. Current
RustyCore public Detour APIs accept all seven/profile through a strict Forever
adapter: one complete path, five partial paths and one absent-nearest error
match original queries/profile. The legacy MMap parser still rejects version16.
Nineteen added tests bring navigation to 278 and VMap to 472; workspace libraries
pass 1,872 plus four examples and Clippy. Strict store regression retains all
75 case arrays; inventory retains all 82 outputs and semantic/work statistics,
with 1,024 additional heap bytes per discovered map for component bitsets.
This is a supplied single-tile pipeline. Full real acquisition/inventory, original
DB2 startup/full-map orchestration, product manifests/resume/distribution and
world/runtime acceptance remain open; the full extractor is not complete.
Linux navigation package discovery now enumerates canonical four-digit map
names and 0..63 tile names from held maps/vmaps roots. It applies the same source
filters before list reads/child traversal, verifies complete tile lists and
retains their hashes plus observed directory/file identities. Opening proves
inventory/identity admission, not unconsumed model/tile body completeness.
Package.run_tile binds the plan between full inventory verifications and fresh
rooted terrain/store contexts; observed mutation permanently poisons it.
Debug/release each reproduce 82 inventories / 140 globals / 20,244 inventory
bytes and 174 admitted tile-list hashes against original MapBuilder. Nine
package-bound scenarios retain all seven final files / 452,012 bytes, every
pipeline statistic and the existing RustyCore Detour comparisons. Fifteen new
tests bring navigation to 293; workspace libraries pass 1,887 plus four examples,
with Clippy and 200 empty traces. The canonical profile rejects native filename
aliases/malformed packages rather than claiming general filename parity.
Verification observes metadata/absence and is not an atomic snapshot or full
content rehash. Per-tile full-inventory verification has cost proportional to
observed entries; scalable full-map/product orchestration remains separate.
Complete real startup/acquisition, manifests/resume/distribution and world/runtime
acceptance remain open; the Forever Rust extractor is not complete.
See docs/forever-navigation.md for limits, reports and remaining product gates.

The sequential Linux navigation runner now consumes complete Map source records,
rooted inventory and explicit liquid/connection inputs. All/map/tile selections
emit deterministic provisional globals and tile/empty outcomes; only a returned
Completed admits the selection. Two full inventory verifications surround each
batch. Requested artifact/directory identities additionally bind every provider
read to the original sorted enumeration. Fresh contexts bound per-tile residency;
observed mutations permanently poison the package. Both profiles match
12 synthetic packages / 15 global files / 22 selected tiles against original MapBuilder discovery/globals
and original public fresh TileBuilder calls per scheduled tile. They retain
20 final files / 1,176,088 bytes, two empty outcomes, and the unchanged RustyCore
single-tile Detour query adapter. All nine preceding pipeline statistics/final
outputs are identical. Twenty-three focused tests bring navigation to 316;
workspace libraries pass 1,910 plus four examples and Clippy. There are 110
empty runtime network traces. This supplies synthetic sequential full-selection
execution; it does not run original worker threads, establish multi-tile query
linkage, repair real acquisition, or provide product manifests/resume/publication.
Complete real startup, product/distribution and world/runtime gates remain open.

Strict global/tile adapters now load multiple final tiles together through the
original pinned Detour and unchanged public RustyCore APIs. Complete authored
continuous-plane packages generate 14 exact final files / 1,433,000 bytes and
four original global files in debug/release. Both profiles' files pass 33 explicit
world/lifecycle scenarios and 1,248 query records/profile, including 11 required
cross-tile positive scenarios and 187 complete cross-tile paths in the fully
loaded states/profile. Both load orders and parent/child overlays are exercised.
Duplicate coordinates, stale references, removal/reload and empty worlds match;
one synthetic four-position selection exceeds its original three-slot root
capacity and explicitly matches OUT_OF_MEMORY in both engines. No capacity is
silently enlarged. Three focused fixture tests bring navigation to 319 and
workspace libraries to 1,913 plus four examples, with clean Clippy; three separate
standalone adapter tests and Clippy pass. There are 196 empty runtime network
traces. Public successful detail status flags are not exposed by RustyCore and
are not claimed. This is bounded explicit synthetic multi-tile acceptance,
without original MMapManager source/fallback execution, gameplay bootstrap,
real acquisition, product manifests/resume/distribution or full-world evidence.
The full Forever Rust extractor remains incomplete.


### Explicit supplied-key library scope (2026-10-11)

`keys::Keyring`, `blte::decode_with_keys`, `archive_record::decode_span_with_keys`
and `LocalStore::open_with_keys` form a separate bounded Salsa20/128-bit opt-in
API. The default store, acquisition profiles and product-v1 CLI retain N/Z.
Keys are provided directly by the caller: no discovery, downloads, built-in
key table or key logging. Complete CKey checks are mandatory for encrypted
results; resident key capacity and decrypted scratch count against limits.
Forty focused synthetic tests include encrypted ENCODING/ROOT and incomplete
input rejection. Original pinned CascDecrypt additionally supplies 272 synthetic
cases/544 calls, matched by 488 public Rust BLTE objects per debug/release
profile; this is decryption wire evidence, without native CASC routing. No actual client keys are inspected, and no missing real
input recovery or full extraction/runtime compatibility is established.
See `docs/forever-local-casc.md` and its licensing closure.

The separate product-v2 contract admits explicit `--keys` only with canonical
`decryption=provided-salsa20-v1`, ordered immediately after ROOT in its TSV
plan. V1 bytes/defaults and existing component acquisition profiles stay N/Z.
V2 client inspect/extract/resume require a regular bounded key file; plan-only
inspection and output verification require none. Verification forbids the
option. No key/path/count/keyset hash enters plans, contexts or manifests.
The key source is protected from output creation/resume; no discovery,
built-in lookup, download or limit increase is authorized. A different
explicit set may recover the same selected complete CKey on resume without
changing logical source identity; sealed components remain verified/skipped.
This does not establish actual real encrypted acquisition, recover the
known incomplete real inputs, or complete the extractor/runtime gates.
See `contracts/forever-product-contract-v2.json` and `docs/forever-product.md`.
V2 acceptance adds 18 Rust tests and 20 synthetic CLI cases/profile; the
workspace passes 2,088 Rust tests/profile, 76 contracts and Clippy. Sixteen
SIGKILL interruptions yield 14 recoveries and two expected protected-key-source
refusals; both real 33-artifact v1/v2 regressions pass. The real
v2 run supplies only an authored dummy key over already accepted N/Z inputs;
it establishes no real encrypted-data compatibility or missing-input recovery.

## Local Forever distribution boundary

The versioned `forever-distribution-v1` recipe produces local Linux x86_64
candidates only. It builds the frozen shipped source offline with Rust 1.98.0,
including current uncommitted product code; HEAD-only archives are insufficient.
Only the two already admitted exact registry packages may be vendored, with
their archives, checksums, original sources and all notices. Use new external
output directories, retain failed/successful runs, and never package client
assets, keys, extraction outputs, caches, secrets or native reference trees.
Archive integrity and reproducibility do not establish real extraction,
runtime acceptance, Windows support or release authorization. Windows needs
a versioned filesystem backend/acceptance and any new native dependency's
recorded license closure. See `docs/forever-distribution.md` and
`contracts/forever-distribution-contract-v1.json`.
The candidate must also retain `THIRD_PARTY_NOTICES.md` and the exact pinned
Rust standard-library copyright index/license texts under toolchain-notices;
these are notice documents, without a claim that every listed component links.
Executed local acceptance passes 41 packaging tests and 82 contracts, two
identical fresh archives/binaries and a moved-source offline rebuild. Fifteen
independent regressions pass; the packaged binary passes 37 CLI cases and
preserves 110 prior synthetic artifact hashes plus the real 33-artifact batch.
Production Rust/dependencies remain unchanged; Windows, full-world and real
complete geometry/runtime acceptance stay open. No publication is authorized.

## Fixed v0.1 target

- Product: `wow_classic`
- Client version/build: `3.4.3.51943`
- Map format: `MAPS`, version `10`
- VMap format: `VMAP_4.B` (`VMAP04B` raw intermediate)
- MMap format: `MMAP`, version `15`, with the exact supported Detour version

On the legacy `3.4.3` line, other products/builds remain outside this fixed contract.
The separately authorized Forever target does not change that legacy contract.

## Current-server paths and data boundary

The following inputs are **read-only references**. Do not edit, clean, move, patch, stage,
or write generated directories beneath them:

- WoW client: `/home/server/wow-client/World of Warcraft`
- C++ reference: `/home/server/woltk-trinity-legacy`
- Existing extracted reference: `/home/server/woltk-server-core/Data`

All builds, downloads, temporary files, logs, comparisons and generated outputs belong
outside the Git checkout under:

```text
/home/server/rustycore-extractor-work/
  artifacts/
  builds/
  cache/tact/
  inputs/                 # read-only symlinks to canonical inputs
  logs/
  reports/
  runs/reference/
  runs/candidate/
  worktrees/
```

Never run a legacy script whose cleanup/output behavior targets the client directory.
Invoke tools with explicit input/output paths or from an isolated run directory. The legacy
MMap generator uses its current working directory, so run it only inside its run directory.

## Copyright and licensing gate

- Project license target: GPL-3.0-or-later.
- Preserve all TrinityCore copyrights and GPL-2.0-or-later headers.
- Keep MIT, Zlib, BSD and other third-party notices with their source.
- Do not import upstream code until issue #1 records the exact dependency/license closure.
- Mark derived files with a prominent modification notice and date.
- Never claim ownership of upstream work or remove an upstream author's notice.

No Blizzard client file or client-derived `dbc`, DB2, map, VMap, MMap, model, camera, GT,
CASC cache or intermediate output may be committed, uploaded to CI, or attached to a release.
Only synthetic fixtures and hash/structured reports are allowed in Git.

## Network policy

- Local CASC and offline operation are the default.
- No hidden TACT-key, listfile, CDN or remote-CASC request.
- Network access requires an explicit option, a pinned source and checksum, and a displayed
  download plan.
- Tools never upload client paths or data.

## Working an issue

Follow the numbered order in issue #13 and respect dependency gates.

1. Read this file, the index and the selected issue.
2. Create issue branches from the version line: `forever` for Forever, `3.4.3` for legacy work.
3. One issue = one branch = one focused PR into that version line with `Closes #<N>`.
4. Compare with the fixed C++ reference before changing behavior or formats.
5. Add focused positive/negative tests and structured diagnostic output.
6. Run the smallest relevant local check during iteration.
7. Run the issue's compatibility acceptance before push.
8. Do not push, publish a release or make the repository public without explicit approval.

PR checks stay lightweight: formatting, synthetic tests and platform builds. Real-client full
extraction is local evidence because its inputs and outputs cannot be placed in CI.

## Long-running work

Do not attach a full extraction or MMap generation to an AI-agent process. Use a persistent
user service or `tmux`, write logs and manifests inside the run directory, and record the exact
command. Example shape:

```bash
systemd-run --user \
  --unit rustycore-extract-<run-id> \
  --working-directory /home/server/rustycore-extractor-work/runs/candidate/<run-id> \
  --collect \
  <command>
```

Never delete an older successful run until its report identifies it and another verified run
supersedes it. Clean only explicit tool-owned directories.

## Validation policy

- Fast iteration uses synthetic inputs or one representative map/tile.
- The compatibility harness compares file sets, parsed semantics and deterministic bytes.
- A successful process exit is not a parity claim.
- Full build-51943 extraction is reserved for issue closeout and release candidates.
- Existing `/home/server/woltk-server-core/Data` is convenient reference material, not final
  proof until its producing extractor version and options are recorded.

## OpenCode orchestration on Joe's workstation (2026-10-04)

The local checkout is `/home/joe/projects/dream/wow/rustycore-extractors`.
The `/home/server/...` paths above describe the reference-server layout; they are
not local paths to invent or automatically remap. Workstation generated files
belong in `/home/joe/projects/dream/wow/rustycore-extractor-work/`.

The project-local OpenCode configuration is `opencode.json`, with profiles in
`.opencode/agents/`. Start through `.opencode/start` so a private server inherits
the external Cargo build and temporary paths. See `docs/opencode-orchestration.md`.

- `orchestrator`: DeepSeek Go; plans and delegates, never implements.
- `general`: GLM Go; architecture and compatibility contracts, read-only.
- `scout`: DeepSeek Go; focused source discovery, read-only.
- `builder`: DeepSeek Go; default implementation worker, owns assigned files.
- `explore`: GLM Go; independent audit and checks, never implements.
- `local-coder`: Qwen/Strata; optional, only when explicitly requested for a small task.

The operator selected the cloud builder after the local-only trial proved slow.
Do not automatically fall back to Kimi, unavailable GPT models or local Strata.
Cloud-agent input is restricted to source code, synthetic fixtures and reviewed
redacted reports. Never place real client assets, keys or secret environment
values in model context. Starting a session does not automatically start extraction. Explicit bounded live acceptance for the already authorized 70170/70205 task may run through the configured helpers.

Before any task, record the current dirty files, assign non-overlapping ownership,
and preserve existing work. Do not change the active branch merely to initialize
these profiles. Use `.opencode/checks` for bounded validation; native checks require
an explicitly configured, already pinned local reference. Formatting unrelated
dirty files is prohibited. An explicit permission rejection is final for that
attempt: report it rather than seek another command to bypass it.

Persistent goal mode is installed globally through OpenCode Goal Plugin 0.1.54.
The orchestrator owns its lifecycle tools and must create/replace goals only
when explicitly requested by the user. Preserve the goal across delegation;
wait for active children, then require concrete acceptance evidence before
completion. Honor pause/stop and retain every project/data/license boundary.

### Authorized agent autonomy and local acceptance

The operator explicitly authorized the agents to run the task circuit and the
commands required for inspection, compilation, testing and local acceptance.
All profiles now permit task-relevant shell execution and scoped external
reference/laboratory access. The coordinator may validate directly and still
delegates implementation; do not ask the operator to execute available
commands or paste reference metadata. This does not authorize publishing,
client-data exposure, changing the task scope or discarding other work.

The helpers automatically select the pinned local reference and existing
OpenSSL development headers. `checks live` enables the local CLI tests;
`checks live-extract` executes the bounded two-table default plan offline and
returns only redacted structured evidence. Neither is full extraction.
Raw real-client outputs and live temporaries use RCE_OPENCODE_RUN_ROOT under
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate`, because the
neighboring laboratory is itself beneath the enclosing wow Git repository.
Retain previous outputs; reports remain in the usual external laboratory.
Read client data only inside local tools, never into model context.

### Windows read foundation and CI source (2026-10-11)

`forever-platform` is original dependency-free code for the separate
`forever-windows-read-ntfs-v1` read-only profile, not an enabled Windows product
backend. Its scoped Win32 FFI borrows std File handles; directory access must
include FILE_LIST_DIRECTORY (attribute-only opens do not fix sharing). Keep
full file snapshots, held ancestor identities and permanent invalidation; do
not replace these with length/mtime checks. Real Windows/NTFS, reparse/junction,
private DACL, publication/durability and executable identity acceptance remain
open. The synthetic CI source now tests the whole workspace on Linux and
Windows, including unsupported-output guards; it has not been run on hosted
CI or published. The Linux bundle's current mandatory source profile is
`forever-seven-crate-windows-navigation-readers-source-v7` inside the v1 envelope; older
bundles retain their own frozen verifier. See the completion
checklist and Windows profile contract for the precise evidence and limits.

The historical Windows-foundation closeout records 2,093 Linux tests/profile and 924
Windows GNU tests/profile under Wine, with 22/2 explicit ignored tests/profile
respectively; five separate metadata CLI cases also pass. Wine is preliminary,
and its opaque/detached trace markers do not prove exit of every descendant.
The seven-crate Linux source package preserves the prior exact product binary,
37 CLI scenarios and 33 real DB2/GT/camera outputs. Source CI remains unexecuted
on GitHub. Preserve these scope limits when integrating the new read leases.

CASC admission/LocalStore and explicit product input reads now consume Windows
leases. This does not include generic component manifest reads, metadata
snapshots or verifiers. DB2/GT/camera output creation and publication explicitly
reject Windows before I/O; the complete product remains disabled there. The
first integrated Wine batch failed with false `InputChanged` during concurrent
sibling changes. Object-only ancestor acquisition now ignores timestamp changes;
full final-file/directory snapshots and permanent poison remain strict. The
case-insensitive index fixture is corrected without weakening duplicate-generation
selection. Failed batches and diagnostics remain retained. The subsequent complete
Wine replay passed its runtime tests, then failed Windows Clippy; two cfg-only
source fixes and the later consumer closeout supersede that source-check failure.

`windows-consumer-closeout-20261011T180333Z.json` is accepted and terminal for
synthetic read-consumer checks, with 910 source files unchanged during the run.
Debug/release each pass 2,093 Linux tests (22 ignored, 40 targets) and 1,885
Windows GNU tests under Wine (18 ignored, 40 executables). All-target workspace
Clippy passes for Linux and Windows; 125 contract and 49 packaging checks pass.
Per Windows profile, 37 exactly SHA256-identical executables reuse validated prior
binary/log/trace evidence and three changed executables run afresh. Windows-side
TMP/TEMP settings are verified with an actual std::env::temp_dir probe.

AF_UNIX-only seccomp is enforced. Filtered network traces describe observed
calls; denied attempts may be absent and must not be reported as never attempted.
Natural exit and absence of foreground-server launchers do not prove individual
exit of every Wine descendant. Actual Windows/NTFS, reparse, private DACL,
publication/durability and executable identity gates remain open. This closeout
adds no real-client acceptance, Linux bundle or hosted CI result; the complete
Windows product and full extractor remain unaccepted.

The corrected Linux consumer v3 bundle separately reproduces the `ca528b69...`
binary across two builds and a moved-source rebuild. Its later bounded-real
supplement `distribution-v3-bounded-real-20261011T180333Z.json` (SHA256
`89a1a05a7acb41043a77875389d7194f1e9a10e293d0e4e1ce67e1854748685b`)
accepts 33 DB2/GT/camera artifacts / 193,794 bytes with that binary. Keep this
evidence tied to v3; no v4 package has been built or accepted by these results.

The separate `forever-windows-producer-seal-v1` API now captures an actual
volatile 64-byte main-module seal and measures its payload start RVA, then
binds protected complete file bytes through an original strict unsigned x64
PE32+ verifier. The offline `tools/seal_forever_windows.py` creates an exclusive
new sealed artifact; supplied tool payload/RVA values establish artifact
integrity only. Neither tool nor helper enables the Windows product. Preserve
the actual measured RVA, complete canonical hash and final identity recheck;
do not substitute disk-derived evidence or a pathname-only identity. Static
loader-metadata checks are not complete loader/startup-behavior validation.
Source profile v4 mandates the API, FFI, verifier, helper example, sealer,
tests, contract and documentation without changing the seven-crate/nine-package
dependency graph. The integrated candidate passes 139 contract and 94 distribution
checks (53 packaging, 41 sealer); these do not build a v4 package or prove actual
Windows/NTFS behavior. See docs/forever-windows-producer.md for exact scope.

The corrected production helper separately has accepted preliminary Wine
execution: 37 Rust tests/profile, 41 Python cases, Windows all-target Clippy and
six sealed/unsealed/tampered helper cases across debug/release. Measured payload
RVAs are 5,009,408/2,412,544. Preserve the final-control-check closeout/report
hashes recorded in the producer doc and distinguish this from actual
Windows/NTFS, an enabled product or a built v4 bundle. Filtered traces describe
observed calls, not all denied attempts or every descendant's individual exit.


The later canonical producer closeout freezes 921 sources and passes 2,130 Linux
and 1,925 Wine tests/profile (22/20 ignored, 41 targets/executables), workspace
Clippy on both targets, 139 contract and 94 distribution checks. Each Wine
profile validates 24 exact executable reuses and 17 fresh executables. The
accepted Linux v4 bundle is `ab2649ea...`, binary `42f1ed4f...`; its later bounded
real supplement accepts 33 DB2/GT/camera records / 193,794 bytes. Exact report
hashes and retained chronology are in docs/forever-completion.md. These supersede
pending-status checkpoints without relabeling older v3 or failed reports.

Source v5 adds mandatory Windows VMap read adapter/EOF source, pure key-free BLTE
framing and fixed metadata-only required-route diagnostics with tests/contracts/
docs. Only the Windows-local forever-vmap -> forever-platform edge is new; keep
seven crates, nine lock packages and the two exact registry archives. The routes
doc is copied byte-identically to docs/forever-required-routes.md; do not admit
generic crate Markdown. VMap's 28 Wine cases/profile are preliminary. The initial
routes handoff is superseded after an archive filename-format defect. Corrected
decimal naming now shares acquisition construction and passes 88 Linux tests per
profile, Linux/Windows Clippy and cross-build; Wine route runtime remains pending. Do not compose a route-byte adapter implicitly, expose
keys/routes/payload bytes or infer plaintext/key-only sufficiency from framing.
The isolated v5 composition is not a built bundle, complete payload diagnostic,
actual Windows/NTFS acceptance, enabled Windows product or publication authorization.


The corrected real route metadata QA is accepted/terminal in
required-route-live-20261011T1901Z-attempt2.json (SHA256
83b4ad2ba3a939207f194dc3b1f287b6307c33cffcb304dc500b9abfd7ad6ec1).
Map, SkillLineAbility and GameObjectDisplayInfo each have one Routed candidate
whose checked span FitsObservedLength. Payload/ciphertext remain NotInspected;
no requested archive-span read, framing, keys or content acceptance occurs.
Do not infer complete bytes or that only a key is missing. The first harness
attempt failed before client execution; preserve it. The route design doc is
retained byte-exact from before this QA; the completion doc records later scope.
The union has 935 sources/518 selected package sources and passes 156 contracts
plus 97 distribution checks; this is not a v5 build or full runtime acceptance.


Source v6 (`forever-seven-crate-windows-vmap-required-framing-source-v6`)
mandates the separate fixed-three-input complete-span key-free framing adapter,
its 23 synthetic cases, contract/static tests and qualified acceptance companion.
Keep the exact seven imported paths, including the byte-exact pre-live
implementation doc; the companion records later accepted Linux observations.
The v5 canonical source merge has 935 sources and is accepted, but neither this
increment nor that merge builds a new bundle. Seven crates, nine packages, two
registry archives and the v5 VMap-to-platform edge remain unchanged. Preserve
historical v4/v5 contract JSON.

Current build 70205/esES key-free acquisition reports encrypted 1 each for Map,
SkillLineAbility and GameObjectDisplayInfo, with missing/incomplete/corrupt/
unsupported 0 and `local_store_no_verified_encoding`; no decoded input succeeds.
The separate complete-span framing run accepts three `Complete(FramedMd5)`
candidates, 15 frames / 6 Z / 9 E / 15 MD5. See
`docs/forever-required-framing-acceptance.md` for full report hashes and limits.
Earlier `local_file_incomplete` outcomes are retained historical attempts, not
the current acquisition category. Typed-loader/full real geometry/runtime
acceptance remains pending; do not relabel failed batches.

The 4096-frame ceiling is per candidate BLTE, with independent 256 KiB total
record and 64/192 candidate ceilings. Preserve complete reads, 30-byte prelude
removal, nine-byte prefix-only scope, all frame MD5 checks and final store/file
guards on success/error. Do not infer full EKey/CKey, Z interior, E inner or
ciphertext interior, plaintext or key-only sufficiency. No keys, raw assets,
repair, network, CLI enablement, actual Windows/NTFS or publication is added.
Filtered runtime trace observations do not prove absence of denied attempts
or individual exit of every descendant.

### Navigation source integration and complete spell-reader QA (2026-10-12)

The current mandatory corresponding-source profile is
`forever-seven-crate-windows-navigation-readers-source-v7`. Its canonical merge
adds ten files and changes seventeen existing files, retaining 952 source files,
the branch/HEAD/index and all 925 unrelated source files. The exact nine-package
graph adds only the Windows-local navigation-to-platform edge relative to v6.
The isolated integrated source passes 2,225 tests across 41 targets per Linux
debug/release profile (22 ignored), Linux/GNU-Windows all-target Clippy and both
GNU-Windows compilation profiles. All 171 Forever source-contract/state and 104
distribution/sealer checks pass. These are source checks, not a new bundle or
actual Windows/NTFS acceptance. The older 112-case Wine semantic result retains
its rejected overall/network evidence, including 40 unknown syscall bodies and
783 denied nonlocal attempts; do not relabel it.

The separate external `rustycore-spell49-reader-check` now calls the unchanged
public `SpellRecords::load` with `Complete` for all 49 tables. Five populated
synthetic worlds per debug/release profile match every one of the 469 public row
fields, including arrays, raw integer/f32 bits and localized/raw text: 784 rows,
45,946 projection bytes and 148,168 input bytes per profile. The authored wire
values independently match Python. Copies, chains, overwrites and zero/missing
copy sources are explicit. Three probe unit tests per profile, tool Clippy and
eight expected negative calls pass; two existing RustyCore dependency warnings
remain. All 23 final traces contain no network calls; five contain only signal
annotations. No integration implementation is changed.
This accepts standard unencrypted regular single-section files with complete
column widths. Earlier zero-column-BitSize fixtures pass extractor decoding but
are rejected by RustyCore at its first inline-ID table; preserve this separate
reader-admission difference and every earlier failed report. Complete real spell
inputs, physical-variant coverage, finish/SQL/hotfix/gameplay and full extraction
remain unproven. Exact reports and limits are in docs/forever-completion.md.

### Private spooled collision source integration (2026-10-12)

The optional Linux collision library now provides immutable disk-backed model
payloads, row-wise GameObject list construction, spooled placement/shared-run
consumers and an authenticated world owner. The same collector, model handles,
cache flags and cumulative counters survive startup and Map/liquid preparation.
The owned Sink is precharged before preparation; failed paths destroy it before
rechecking both source and spool. Completed startup data and validated topology
remain charged through subsequent RAW reloads. The resident product driver,
formats, CLI and v7 corresponding-source profile retain their contracts.

The exact source union passes 2,391 synthetic tests across 41 targets per Linux
debug/release profile (22 ignored), Linux/GNU-Windows all-target Clippy, both
GNU-Windows compilation profiles, 171 source/state contracts and 104
distribution/sealer checks. The scoped world step passes 387 collision tests
per profile, including 19 new tests compared with the resident Rust world.
This adds no new original C++ or real-client acceptance. Separate hosted
Windows debug CI exposed checkout/fixture defects; their corrections retain
the original mutation assertions and require their own executed CI result.

WDT and parent-cache inventories remain resident. The 4,096 cumulative source
read ceiling and current same-mount private-spool policy remain explicit; this
is not scalable whole-world execution. Disk-backed inventories, selected
VMO/map assembly, product publication/resume integration, real mandatory DB2
and complete model inputs, runtime activation and reproducible Linux/Windows
distribution remain completion gates. See
docs/forever-collision-spooled-world.md and docs/forever-completion.md.

### Typed private collision inventory acceptance (2026-10-12)

The optional Linux `collision::inventory_spool` owner stores supplied complete
WDTs and normalized own-tile caches in an empty private Spool. Typed keys bind
map/FDID or map/cell associations; private ordinals never escape. Empty own
caches remain present. Caller-declared acquisition/provenance, the original
planner/cache codecs, downward limits and same-mount policy are preserved.
Protected source directories are object-identity anchors, not authenticated
membership/content snapshots. All observed failures poison the inventory;
callback causes survive independent guard failures.

Attempt 3 passes 25 focused and 412 complete collision tests per Linux profile,
CASC library/test Clippy, three formatting checks, 171 source/state contracts
and 104 distribution/sealer checks. Exact hashes and retained failed fixture
history are in docs/forever-collision-inventory-spool.md. The new owner is not
wired into world extraction; its WDT and cache collections remain resident.
Whole-world budgets, cross-volume support, final assembly, product/runtime and
real mandatory inputs remain completion gates.

Separately, hosted Rust run 38189148916 at 9e97d48 passes Linux debug/release
and Windows debug (2,116 tests, 41 targets, 20 ignored on Windows). This is
synthetic hosted execution of the preceding source, not acceptance of the new
Linux-only owner, actual-client NTFS behavior or Windows product publication.

### Authenticated disk-inventory world (2026-10-12)

The optional Linux `collision::run::inventory` now acquires complete WDT/own
Obj0ADT inputs through the established model collector and stores WDT/cache
bodies in disjoint private typed inventories. Sorted parent-key range lookup
retains x/y order, empty-own barriers and lazy inactive-parent failure without
per-absent-cell repository checks. The consuming `world::inventory` joins the
same closed Map/liquid preparation and validated topology to that runner.

Its explicit downward-only combined logical ceiling is 2 GiB: the established
collector ceiling plus both complete inventory heap ceilings, admitted before
DB2 I/O. Each inventory must fit the existing 256 MiB component cap. Owned
Sink/IDs/source/parser allocations remain in the collector; exact Sink precharge
transfers without a gap or double reservation. Returned cache bytes have a
separate pre-I/O output allowance. Data/topology remain retained through RAW
reloads. These are conservative reservations, not RSS or all CPU operations.
The actual owned Sink drops before all independent final source/model/WDT/cache
guards; typed causes survive observed changes.

The preceding runner passes 26 focused and 438 full collision cases per Linux
profile. Final world source passes 25 complete-world and 26 public-runner cases
per profile, CASC library/test Clippy, four scoped formatting checks, 171
source/state contracts and 104 distribution/sealer checks. Full 438-case scope
belongs to the preceding source; no final whole-workspace result is inferred.
All comparisons here use authored toy CASC against unchanged resident Rust;
no new native or real-client acceptance is added. Exact reports and preserved
interrupted/failed fixture history are in docs/forever-collision-disk-run.md and
docs/forever-collision-disk-world.md.

Resident/spooled v1 APIs and the product driver remain supported. The final
product still uses resident assembly. Scalable read/disk/reload policies,
cross-volume support, selected VMO/map/proof disk staging, product integration,
complete real inputs, runtime activation and distribution remain gates.
Hosted CI 38191467007 separately passes Linux debug/release and Windows debug
at preceding canonical 26f037d (2,116 Windows tests, 41 targets, 20 ignored).
That hosted result predates this source increment.

### Authenticated placement disk stage (2026-10-12)

The optional Linux `collision::assemble::staged` phase consumes the authenticated
startup and disk world through a private borrowed Sink; callers cannot supply
placements or forge a completed stage. Immutable RCEPLC1 envelopes bind map,
original map, global/tile coordinates, replay flag, callback sequence, record
count and complete extent. Empty successful callbacks and every duplicate ID /
pre-deduplication model name survive. Sorted compact descriptors reload exactly
one complete map in global-then-x/y order. Immediate-parent context validation
uses the closed topology; final first-present-ancestor proofs remain separate.

A full stage heap reservation is partitioned from the original combined world
ceiling before acquisition. The collector ceiling only decreases and survives
finish, preventing later RAW reloads from reusing that reservation. Descriptor
replacement overlap, complete map output, largest envelope reload and declared
callback allowance are admitted independently. This remains conservative
logical accounting, not RSS or all CPU work. Callback captures and rejected
values drop before final model/stage guards, preserving typed causes. All
upfront failures also check the actual source and all independent repositories.

Exact final source passes 28 focused and 491 complete collision tests per Linux
debug/release profile, CASC library/test Clippy, four scoped format checks, 171
source/state contracts and 104 distribution/sealer checks, with eight strict
IPC-only traces. Comparisons use authored toy CASC against the unchanged
resident Rust world; no new native or real-client claim is made. Full local
workspace/Wine checks and a new distribution bundle are not claimed here.
Exact hashes and retained fixture/checker failure history are in
`docs/forever-collision-placement-stage.md`.

This is an intermediate library owner, not a final VMap package or product mode.
The product driver remains resident. Selected-model VMO conversion, per-map
final artifacts/ancestor proofs on disk, scalable explicit read/disk/reload
policies, cross-volume support, publication/resume, complete real mandatory
inputs, runtime and reproducible distribution remain completion gates.
Preceding canonical 3629e8d passes hosted Rust/C++ CI independently, including
2,116 Windows debug tests across 41 targets (20 ignored); that result predates
this Linux source increment. The full extractor goal remains active.

### Selected VMO private disk preparation (2026-10-12)

The optional Linux `collision::assemble::staged::models::prepare` consumes the
actual closed placement/world owner and a fresh private Spool. Its exact-name
union includes every placement before ID deduplication plus every successful
startup TempList row; unused acquired dependencies are excluded. All selected
RAW geometry/bounds are scanned before the first conversion/write. Each complete
verified RAW is then converted individually, written under a bound RCEVMO1
envelope and dropped. Reload validates exact source identity/name/class, complete
VMO structure and M2 group bounds. Native VMAP_4.E bytes remain unchanged.

The ledger retains recorded store residency, exact compact/repository/Data and
topology, the full placement reservation, twice the archive descriptor ceiling,
a fixed guard allowance and actual catalog capacity. RAW/conversion/reader and
declared callback overlap are admitted before I/O. New crate-private completed
RAW access subtracts conversion allowance from the existing downward collector
budget without removing Data/topology charges. These remain logical admission
counters, not allocator/RSS or all CPU work. Typed callback causes survive;
uninvoked captures, buffers/readers and rejected successful values drop before
independent RAW, placement and VMO guards. All prior production bodies remain
byte-exact apart from the new module export and added private helper methods.

Final source passes 21 new VMO and 28 affected parent placement tests per Linux
debug/release profile, CASC library/test Clippy, four scoped formatting checks,
171 source/state and 104 distribution/sealer checks, with eight strict IPC-only
traces. The parent 491-case full collision acceptance remains a separate source
checkpoint. No new full-collision/workspace/Wine, original C++ or real-client
result is inferred. Tests use authored toy CASC versus resident Rust assembly;
only their five-repository fixture lifetime is bounded by a test-only mutex.
Exact hashes and retained failed attempts are in docs/forever-collision-vmo-stage.md.

This is selected-model preparation, not a final map/proof package or product
mode. Original same-mount/read/disk/reload v1 limits and the v7 corresponding-source
graph remain unchanged. Final per-map artifacts/first-present-ancestor proof
resolution on disk, scalable explicit policies, cross-volume, product publication /
resume, complete real inputs, runtime and reproducible distribution remain gates.
Preceding canonical 8596820 independently passes hosted Rust/C++ CI, including
2,116 Windows debug tests / 41 targets / 20 ignored, before this increment.
The full extractor goal remains active.


### Closed map/proof disk package (2026-10-12)

The optional Linux `collision::assemble::staged::package` consumes the actual
Prepared world/model owner and creates a complete private artifact inventory.
RCEPKG1 binds map/effective parent, complete tree/ordered proof and sorted tile
payload/index spans. Complete own associations are checked; parent-only tiles
retain child indexes and bind to the first present payload in the actual
effective-parent chain. A present mismatch is terminal. One map build, or one
copied child plus one borrowed ancestor, remains active. Metadata enumeration
performs no body I/O; visitation reads each selected VMO and map once.

The downward 2 GiB logical ledger retains recorded store/compact/Data/topology,
full placement reservation, VMO catalog and actual map/tile capacities, with
source/builder/callback overlap and the smaller package cap constraining older
VMO readers. Validation scans, ancestor and BIH work are explicitly bounded;
these are not RSS or all CPU counters. Typed errors retain independent guards
after callback/value destruction. Finished owners verify private bodies and
recorded client facts, not a live-client atomic snapshot; product integration
must retain and recheck its LocalStore before final Run completion.

Final source passes 39 package and 21 affected VMO cases per Linux debug/release
profile, CASC library/test Clippy, four scoped formatting checks, 171 source/state
and 104 distribution/sealer cases, with eight strict IPC-only traces. The 16-map
logical streaming case and 25 exact Association negatives are separately scoped
from complete toy-CASC versus resident Rust byte comparisons. No new native /
real-client, final full-workspace or full-collision local execution is claimed.
Exact hashes and retained failed attempts are in docs/forever-collision-map-package.md.
Preceding 6f731b4 passes hosted Rust/C++ CI, including Linux 2,516 tests/profile
and Windows debug 2,116 tests across 41 targets; that result predates this step.

The product remains resident. Existing same-mount/source/read/disk/reload v1
ceilings and the v7 corresponding-source profile remain unchanged. Scalable
versioned policies, cross-volume support, publication/resume, mandatory encrypted
real inputs, real geometry/runtime and distribution remain goal gates. No full
extractor completion, server activation, merge or release is claimed.


### Explicit captured-volume collision storage (2026-10-12)

Linux Spool now has an explicit `Profile::CapturedVolumesV2` capability through
`create_with_profile`; original `create` always keeps `SameMountV1`. V2 permits
separately captured source/private-output mounts. Every source/key/output
ancestor retains its own held/named object identity and mount ID; output root /
records remain bound to the captured destination mount. All original path,
private-parent, alias, exclusive-write, metadata/hash/EOF, sticky failure and
independent final guards remain exact. No mounts/copies/relocation occur.
All protected inputs still require explicit caller declaration; directory
identity does not authenticate membership or a live-client atomic snapshot.

All resource ceilings stay downward-only at their original v1 maxima. Final
source passes 40 Spool (nine new) and 39 closed-package tests per Linux profile,
plus an explicitly executed second-filesystem corpus per profile: four complete
toy-CASC packages match resident Rust bytes/inventory/facts and one nearer-parent
Association fails exactly. All six repositories use v2; default v1 rejects before
namespace creation. Clippy, three scoped format checks, 171 source/state and 104
distribution/sealer checks pass with ten strict IPC-only traces. Exact evidence
and the retained initial unused-argument Clippy failure are in
docs/forever-collision-captured-volumes.md.

The preceding 2fe8cfb passes hosted Rust/C++ CI with Linux 2,555 tests/profile and
Windows debug 2,116; observed result-summary counts differ and are recorded in
the companion. No final full-workspace/full-collision local, new native/client,
product activation or release claim is added. Scalable versioned cumulative
limits, product disk/publication/resume integration, mandatory encrypted real
inputs, geometry/runtime and distribution remain gates. The product/default
CLI and v7 corresponding-source profile retain their contracts.


### Explicit collision cumulative resource policies (2026-10-12)

The optional Linux library now carries `resources::Profile::DiskWorldV1` through
all six actual private spool owners. Default `BoundedV1` ceilings remain exact;
64-bit opt-in raises only declared cumulative source/read/request/model, disk /
record/reload, placement/model/package and repeated-row list work capacities.
Mixed policies and upward overrides reject before the relevant acquisition or
output. Per-file/parser/format, complete-input, component heap/overlap and combined
2 GiB logical ceilings remain. Counters are not RSS or every CPU operation.

GNU/Linux x64 Rust 1.98 debug/release each pass the complete 1,204 ordinary CASC
inventory plus two explicit ignored corpora, 20 pure streaming tests, 171 source /
state contracts and 104 distribution/sealer checks; Linux/GNU-Windows library /
test Clippy and scoped formatting pass. The 4,100-row repeated-model corpus has
1,612,694,000 RAW visit bytes and 134,352,900 vertex visits, without 4,100 distinct
model acquisition. Four complete second-filesystem toy-CASC packages match the
resident Rust bytes/inventory; one exact nearer-parent Association rejects.
All 203 final raw traces, source/tool inventories and exact terminal service
observations pass independent review. Prior partial/failed/cancelled validation
remains retained and unaccepted. Scope, caps and hashes are in
`docs/forever-collision-disk-resources.md`.

This is synthetic library source acceptance, not a final whole-workspace, new
native/client, Windows runtime or full-world throughput result. The current
file-per-record backend retains prior-record scans; the separate private append
candidate is not integrated into the product. Resident product/default CLI and
v7 corresponding-source profile remain. Physical-disk preflight, scalable product
publication/resume, encrypted mandatory complete inputs, geometry/runtime and
reproducible distribution remain gates. The full extractor is not complete.


## Linux private collision archive source profile v8 (2026-10-12)

The current mandatory corresponding-source profile is
`forever-seven-crate-linux-collision-leased-archive-source-v8`. GNU/Linux x86_64
write leases protect one private append archive per owner. All six actual
RAW/WDT/cache/placement/VMO/map owners bind the same backend, volume and resource
policies. FileSetV1 remains default. Complete framing/digests, source/namespace /
mount guards and terminal poisoning remain. Geometric descriptors and constant
archive guard counts do not claim whole-world linear work or throughput.

The disk-profile cache parser allowance is reduced to 64 MiB in the inventory
and world runner; no memory ceiling is raised. Metadata-only fixtures
force distinct UNIX timestamps. Production reader behavior is unchanged.
The complete captured root plus helper reservation is admitted before archive
creation; a downward-budget regression retains only the empty private namespace.

Frozen executable/test source passes 2,599 ordinary tests across 41 targets per
Linux debug/release profile (26 ignored), four explicitly executed ignored
corpora per profile, 180 contracts and 108 distribution/sealer checks. Workspace
all-target Linux/GNU-Windows Clippy, both Windows compile profiles and scoped
formatting pass. Each 4,097-record corpus performs 20,485 archive guards with
bounded steady FDs. Four complete six-owner M2/WMO/parent packages match 27
resident-Rust artifacts / 2,684 bytes; one exact terminal nearer-parent
Association is checked, also on a second filesystem. All 27 command traces,
immutable source/tools and live terminal observations pass independent review.
Earlier failed reports remain retained and rejected.

Seven crates, nine packages and two exact registry archives remain. A Linux-local
CASC-to-platform edge and 28 mandatory source paths enter v8. Historical v7
contracts/import pins retain their scope. No new bundle/rebuild, Windows runtime,
native/client/runtime or product acceptance is established. The product still
uses resident assembly. Physical-disk preflight, publication/resume integration,
encrypted mandatory complete inputs, real geometry/runtime and full distribution
remain gates. The extractor is not complete. See
`docs/forever-collision-leased-archive.md` for hashes and retained history.


## Explicit product collision disk source profile v9 (2026-10-12)

The mandatory corresponding-source profile is now
`forever-seven-crate-linux-product-collision-disk-source-v9`. It adds the explicit
product plan v3 parser, Linux GNU x86_64 borrowed-directory space observation,
guarded disk driver, CLI integration, synthetic checks, contract and scope
documentation. Seven crates, nine lock packages and two registry archives remain;
the extractor additionally has a local Linux platform edge. Historical v7/v8
profiles and their frozen verifiers retain their original claims.

The new plan selects `linux-leased-append-disk-world-v1`, seven downward byte
budgets and an explicit `--scratch` for extract/resume. Six fresh private leased
append owners finish the checked package before common product collision writes.
Unfinished collision resume uses a new scratch namespace; no archive adoption or
cleanup is implied. Physical preflight aggregates same-device allowances, but is
not an atomic reservation or capacity guarantee. Common Run live capacity is
reserved without raising conversion memory ceilings. Resident v1/v2 stays default.

Executable validation passes 2,643 ordinary Linux tests per debug/release
profile (41 targets, 26 ignored), four explicit ignored corpora per profile,
49 existing CLI cases per profile, all-target Linux/GNU-Windows Clippy, both
GNU-Windows compile profiles and scoped formatting. Final source association
reruns 187 contract and 112 distribution/sealer checks plus one added
cross-volume CLI case per profile against the exact full-run product binaries.
All Rust/Cargo and existing CLI-test source is unchanged from the full-run freeze.
This is synthetic GNU/Linux product/source acceptance; no new bundle,
real-client/native acceptance, Windows product or RustyCore runtime acceptance
is claimed. Complete encrypted inputs, real geometry, runtime/server integration
and reproducible distribution remain gates. The extractor is not complete. See
`docs/forever-product-disk.md` and the versioned disk contract.


## Accepted Linux v9 bundle of b692ed3 — 2026-10-12

A subsequent terminal run closes the bounded Linux reproducibility gate for
commit `b692ed3eb189f790dbf79609f8cd0958411ffac6`, source freeze
`495d58c7ab7345b5c65461d235c66bb1efd7c3bc772aa8e3872c5b5d179f8358`.
Two fresh builds produce identical archives/manifests/binaries; a moved-source
offline rebuild reproduces the binary. All 50 shipped v1/v2/v3 synthetic CLI
cases pass, with 129 retained actual CLI traces and 33 complete package contexts
bound to the shipped binary. The outer Python CLI harness is not syscall traced.
Archive SHA-256 is
`a99989e79e956822fb80d19668890096e6b1879093eac71ecf82e7d3606f91ab`;
independent terminal review SHA-256 is
`53b660e03bed16187bf4580e1921fc18ba7587e49a89300ac7d085dd61336d5e`.
Complete hashes, runtime dependencies, resource observations and qualified scope
are recorded in `docs/forever-product-disk.md` under this same heading.

This is acceptance of the frozen `b692ed3` bundle. Its contract flags
`bundle_built=false`, `validation.v9_reproducible_bundle_accepted=false` and the
pending distribution record retain their preceding checkpoint values. The later
documentation commit is not rebuilt and replaces no accepted archive bytes.
Historical source profiles and reports retain their original scope. Real complete
inputs/geometry, full-world resources, RustyCore runtime and complete Windows
product/distribution acceptance remain gates. The extractor is not complete.


## Windows private-output native foundation acceptance — 2026-10-12

The separate `forever-windows-private-output-v1` foundation has bounded ordinary
synthetic native Windows x86_64/fixed local NTFS acceptance for functional commit
`a220596da8a29f9f3a867dc7f2bd016d01cd3558`. [CI run 38254527798, attempt 1](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798)
and [Windows job 114829043981](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798/job/114829043981)
pass on Windows Server 2022, OS `10.0.20348.0`. Both focused debug/release
inventories contain exactly fourteen passing cases, zero failures and zero
ignored cases. Windows workspace debug has 2,151 passing / 21 ignored tests;
Linux debug/release each have 2,643 passing / 26 ignored tests, with 41 workspace
result summaries per job and zero failures.

The independent PowerShell oracle checks persisted directory/file owner and
protected single non-inheriting FullControl ALLOW DACL after the root and parent
leases close. Its live path probe is diagnostic only. The child removes only its
own `PSModulePath`; diagnostics admit two numeric codes, never raw output, SID
or fixture paths. Ordinary impersonation rejection and permanent poison pass;
the DACL restore helper is test-only and production guards remain strict.

The actual checkout merge `8a208b5d01c06dccea92f82cb5cca5822eb13a45` has the same
Git tree `5014b5f12020fc5324c6d4deca1d2335fa9077b4` as the functional commit.
Compile6 source freeze is
`1de7e5ae036d61f5fbabea51fdf2a152486a6622cd9ba6381b0958fc900479cf`
(1,023 files). Accepted terminal native review SHA-256 is
`353c2a48b8094eb4d2dbc5ba42bcc7fd61b505117cf3484f715edb60a757afda`.
The later documentation commit is not rebuilt or tested by this run and creates
no new binary or bundle. Frozen contract native/hosted false flags, pending
status and earlier oracle description retain their authored checkpoint;
the later report binds execution to actual functional source and persisted scope.
See `docs/forever-windows-output.md` for exact evidence and retained failed runs.

Privileged reparse remains ignored. Windows driver, package protocol, recovery,
directory-entry/crash durability and complete product/distribution acceptance
remain open, as do real complete inputs/geometry and RustyCore runtime/server
activation. The Linux v9 source profile, accepted b692ed3 bundle and hardlink
recovery protocol retain their own scopes. No global runner/harness/oracle
network exclusion is established. The extractor is not complete.


## Explicit Windows membership source candidate — 2026-10-12

The separate `forever-windows-private-membership-v1` capability exposes
`output::membership::verify_membership(&PrivateRoot, Limits)` for a complete
observed pass over all owned directories. Root/parent/security guards and full
transient directory snapshots bracket exact observed-name/type/FileId matching.
Unknown, missing, duplicate or mismatched entries fail and observed failures
permanently poison; no unknown directory is adopted or traversed. The original
identity/security `verify_unchanged` and mutations do not auto-enumerate.

Global downward limits are 128 directories, 512 admitted/decoded records
including dots, and 640 queries. A charged lookahead at the record cap can reach
EOF; extra data fails without decoding and poisons. The fixed class-38 buffer is
640 bytes, aligned to eight. Complete observed membership is not an atomic
snapshot or detection of every transient concurrent change, directory write
exclusion, same-SID isolation or a publication/recovery protocol.

This paragraph records the authored source candidate checkpoint, when new
native membership acceptance was pending. The earlier fourteen-case output acceptance of a220596 and its later
5ce8b83 documentary checkpoint retain their original scope. Separate membership
CI steps require exact listed/executed inventories in both profiles; declaring
them is not hosted-run evidence. Historical contracts, Linux mandatory v9 source
profile/bundle and Linux hardlink recovery are unchanged. Windows product,
package/recovery/durability, real complete inputs/geometry and RustyCore server /
runtime remain gates. See `docs/forever-windows-membership.md`.

## Windows membership executed acceptance — 2026-10-12

Functional commit `433d5ce02c5fa541ca9340a20de21889c10216bd`, tree
`7fee5b9f1abbfdc24c3832034ca0c1691b45f154`, passes
[synthetic CI run 38261424386](https://github.com/alseif0x/rustycore-extractors/actions/runs/38261424386).
Each actual job checked out `f4b8a1006d6317d87fd14235439008866840701c`;
its tree equals the functional tree. The Windows Server 2022 job
(`windows-2022`, reported OS `Microsoft Windows NT 10.0.20348.0`) admitted
fixed local NTFS and listed then passed all 24 exact membership cases separately
in debug and release, with zero failed or ignored cases. Thirteen cases use
real temporary NTFS fixtures; eleven exercise pure decoding, matching, status
and limits. The unchanged fourteen private-output cases also pass separately
in both profiles. The privileged reparse case remains ignored and unaccepted.

The full Windows workspace reports 2,175 passed, zero failed and 21 ignored
across 41 summaries. Linux debug and release each report 2,643 passed, zero
failed and 26 ignored across 41 summaries. The separate isolated remote
`compile3` validation passes Linux/GNU-Windows all-target Clippy, scoped format,
both Linux workspace profiles, 219 contracts, 112 distribution cases and both
GNU-Windows compile-only profiles. Its exact terminal invocation, unchanged
source/tools/binaries and independently classified network traces are retained;
GNU-Windows compilation does not supply the native execution evidence.

Retained review: `native-ci1-root-review.json`, SHA-256
`d3ff10adcfa4ec9254e1a54c15e863369c4431906cb2fe1cdad3f49d994231d2`. Source freeze:
`b192bebc605ef7daa039e86cd4bb4df515d02ec19fc49f8b34ab8004acb02713`.
Remote review: `compile3-root-review.json`, SHA-256
`feab62b4ea4537d54d84f2550685ab30bd421843edf52fe490f5aafbd38f3985`.
Reports reside under the external `windows-membership-20261012T133305Z`
acceptance directory. Compile1's permission preflight and compile2's two
Clippy style failures remain failed attempts; compile3 reran all nine commands
after the reviewed style correction. No native membership attempt failed.

This later acceptance does not rewrite the authored contract's pending/native
false flags. It accepts bounded complete observed membership, not an atomic
snapshot or same-SID isolation. Windows adoption, removal, locks, package
publication, restart recovery, product enablement and reproducible Windows
distribution remain open, alongside full real inputs/geometry and RustyCore
runtime acceptance. No new binary bundle or global hosted-runner network
exclusion is claimed. Documentation-only follow-up validation does not rebuild
the functional binaries or establish execution of its later document commit.

### Opt-in regular multi-section DB2 evidence — 2026-10-12

`forever-db2-regular-sections-numeric-v1` is an explicit supplied-byte API for
2..16 complete unencrypted WDC5 sections, using existing reviewed numeric
non-catalog schemas with external IDs and no text or parent fields. Cumulative
limits are 16 MiB, 100,000 direct rows, 100,000 copy pairs and 1,000,000
retained scalar values. The production/default reader and CLI remain single
section; catalog admission and inline IDs remain closed. Debug/release each pass
2,675 workspace tests and 83 original `SpellDuration` signed32 direct/getter/
materializer comparisons. Native does not cover floats, arrays, 64-bit fields or
real-client/runtime acceptance. The retained source checkpoint in the v1
contract records authored-time flags; executed evidence is a separate field and
is detailed in `docs/forever-db2-regular-sections.md`. Full extractor completion
remains open.
