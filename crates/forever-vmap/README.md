# Pure Forever VMap components

This original Rust crate implements M2 collision parsing, the **single-M2
raw `VMAP04E\0` intermediate**, and borrowed WMO root/group parsing with
default small-mode geometry/liquid projection, raw conversion, a strict raw
reader, pure doodad dependency/reference selection, complete object chunk
readers, serial object IDs, direct MDDF/MODF and WMO doodad placement projection
with a strict `dir_bin` codec, parent-cache replay and bounded BIH construction
with a complete storage reader, raw-to-`.vmo` model assembly, transformed placement
bounds, complete supplied-placement single-map assembly and GameObject model
list construction/validation, plus bounded model/instance ray queries. It has no
external dependencies, I/O or network. The separate `forever-casc` collector now connects
model converters and placement batches to local WDT/Obj0ADT acquisition, and
its bounded GameObject startup connects complete DB2/model reads to both lists.
A consuming shared-world wrapper now retains that cache through Map/liquid reads
and WDT/Obj0ADT placement processing. The map API produces `.vmtree`, `.vmtile`
and `.vmtileidx` bytes. Synthetic shared-world output now matches the original
producer. Supplied VMO/instance querying is now implemented; complete real inputs,
product publication, full map loading and runtime integration remain pending.

## Source contract

Facts were read from Git objects at
`02245dcd245e7433e524577656177723d3e4992e`:

- `src/tools/vmap4_extractor/modelheaders.h:24–94`: packed 304-byte header.
- `src/tools/vmap4_extractor/model.cpp:37–158`: first-MD20 scanner, collision
  arrays, transforms, triangle winding and raw serialization.
- `src/tools/vmap4_extractor/vmapexport.h:34–39`: IsM2 flag is u32 value 1.
- `src/common/Collision/VMapDefinitions.h:28–29`: raw and assembled magic.
- `src/tools/vmap4_assembler/TileAssembler.cpp:475–594`: raw consumer.
- `src/tools/vmap4_assembler/TileAssembler.cpp:405–466`: GameObject list
  serialization and all-vertex bounds.
- `src/tools/vmap4_extractor/wmo.h:54–94,110–166` and
  `wmo.cpp:46–192,220–572`: WMO fields, geometry, liquids and raw writers.
- `src/tools/vmap4_extractor/vmapexport.cpp:255–321`: group order, skip
  predicates, mutable split-group inheritance and liquid filtering.
- `src/common/Collision/Models/WorldModel.cpp:99–112`: liquid allocation
  used by the raw assembler consumer.
- `src/common/Collision/Models/WorldModel.cpp:34–81,390–505` and
  `ModelInstance.cpp:27–65`: triangle, group, world and instance ray queries.
- `src/common/Collision/BoundingIntervalHierarchy.h:89–229` and the pinned
  G3D Ray, CollisionDetection and Matrix3 helpers: traversal, candidate-plane
  box intersection and general inverse rotation.

No upstream implementation or G3D source is imported. Upstream copyright and
GPL-2.0-or-later notices remain with the external reference; this implementation
is GPL-3.0-or-later. The existing legacy source import manifest does not establish
a closure for a future modern C++ VMap build.

## M2 admission and output

`M2Collision::parse` borrows complete bytes and returns `None` only after finding
the first MD20 and its complete 304-byte header with zero collision index count.
That path does not read or validate unused geometry fields. Missing/truncated
headers are errors. A rejected first MD20 never falls through to a later marker.
The source does not gate the version field. Prefix bytes, unrelated header
fields, padding and trailing sections remain uninterpreted: this is not an MD21
container or general M2 validator. Relative collision offsets may be unaligned
or overlap if all used extents and values are valid.

With collision, all emitted vertex and bounds floats must be finite; bounds
must be ordered, and the nonzero index count must be divisible by three with
every u16 index in range. Bounds need not enclose vertices. These checks are
stronger than the C++ parser, which is not a malformed-input oracle.

The raw writer preserves the source's declared INDX size `4 + 2 * index_count`
while physically writing u32 indices. It swaps indices 1 and 2 per triangle.
The source's two vertex-coordinate transforms cancel exactly for admitted
finite values, including signed zero; the writer copies those original bits.
Header/group defaults, bounds, one branch and IsM2 are exact. No output is
returned until complete, and writing never mutates the borrowed input.

`RawM2::parse` is a stricter verifier of that exact writer subset. It rejects
unknown/default-field changes, groups other than one, empty geometry, liquid,
corrected INDX sizes, out-of-u16-domain indices, padding and trailing bytes.
Do not apply it to WMO: among other differences, raw header u32 at offset 8 has
different meanings between the pinned M2 and WMO exporters.

Default ceilings are 64 MiB input, 1,000,000 vertices, 3,000,000 indices and
32 MiB raw input/output. Caller limits may only reduce them, including to zero.
Parsing is borrowed; the writer performs one checked output allocation. Types
containing data have no Debug implementation and errors expose static codes.

The Linux-only `m2_raw` example is a bounded QA adapter:
`m2_raw <input.m2> <max_output_bytes> <output.raw>`. It rejects non-regular or
symlink inputs, checks file identity and metadata before/after a bounded full
read, verifies generated raw bytes, and creates a new 0600 output exclusively.
No-collision inputs return a count-free status without creating output. Other
successful reports contain only byte/vertex/index/triangle counts; errors expose
static codes. This adapter is not the product acquisition or extraction CLI.
Validation errors occur before output creation. An output I/O failure can retain
the newly created partial file for inspection; it never returns success and
does not automatically remove a path that another process might replace.

## WMO foundation

`wmo::Root::parse` and `wmo::Group::parse` borrow complete inputs. Supply the
complete detailed GFID inventory as ordered `GroupInput` values to
`wmo::resolve_contexts`, then call each context's `project_geometry`. Root and
group fields, source chunks, doodad records and liquid bytes remain available
through checked getters. No acquired-model validity or output-file completion
is inferred from these views.

GFID must follow MOHD and have the exact declared group/LOD shape. Detailed
selection uses LOD zero, skips zero IDs and retains duplicate IDs in order;
missing GFID has no name fallback. Contexts preserve unreachable/antiportal
skip precedence and the source's mutable single forward pass for split IDs.
A skipped group projects to `None`; unskipped empty collision geometry remains
a successful empty value.

MOPY/MPY2 and MOVI/MOVX support narrow and wide triangle/index fields. The
small-mode predicate keeps collision faces or rendered faces without Detail:
`flags & 0x08 != 0 || (flags & 0x20 != 0 && flags & 0x04 == 0)`. Vertex
compaction preserves original vertex order and f32 bits; triangle order and
winding are unchanged. Bounds, resolved group ID, flags and original MOBA
batch values are retained.

MODS/MODD and either MODN or MODI are structural metadata, with checked set
spans and finite transforms. MODN uses NUL-terminated bytes without assuming
UTF-8. MLIQ retains dimensions, position, signed material, vertices and tile
bytes, including zero dimensions. Pure doodad selection and the external
acquisition collector are described below.

Admission is stricter than the unchecked native parser: full framing, exact
record extents, unique consumed chunks, exclusive alternative arrays, valid
indices, finite values and ordered bounds are checked even for later omitted
data. MOGP's declared container bounds its children. Unknown chunks stay opaque;
optional MVER is exactly one u32 without a version-value gate. This is not a
general malformed-input parity claim.

WMO limits only reduce: 64 MiB per input, 4,096 outer/child chunks per file,
1,000,000 vertices, 3,000,000 indices, 1,000,000 groups and 1,000,000 records
per checked metadata array. Context construction and each geometry projection
separately cap live vector capacities at 64 MiB, including projection scratch.
Borrowed input, allocator overhead and caller-retained prior outputs are
outside that allocation counter. Data-bearing views omit Debug output and
errors use static codes. See [the collision contract](../../docs/forever-collision.md)
for the API policies and remaining extraction dependencies.

## WMO liquids and raw output

`wmo::liquid::project(root, group, lookup)` returns borrowed flags/type/view
facts. `LiquidLookup` supplies u8 material IDs and u32 material flags, matching
the closed terrain DB2 reader facts. The local collision QA caller supplies
their materialized values; this crate does not acquire those tables. Missing
type/material entries are not ignored. Root flag `0x4` maps the header ID
directly; otherwise ID 15 becomes zero and other IDs receive a wrapping u32
increment. Legacy IDs 1–20 cycle through 13/14, 14, 19 and 20, with group ocean
flag `0x80000` selecting 14 in the first category. Zero and IDs above 20 remain
unchanged.

An admitted nonzero header type sets `0x2`; MLIQ adds `0x1`. A zero type can
fall back to the first tile low nibble other than 15, without adding `0x2`.
VisualOnly material flag `0x2` suppresses liquid; the MLIQ check consults type
zero too. All four output flag values 0–3 are supported. Lookups must be stable.

`wmo::raw::convert(root, groups, lookup, ConversionLimits)` validates the full
inventory, resolves skips/split IDs, caches one liquid projection per visited
source group and performs two pure geometry passes. `ConvertedWmo` exposes
complete bytes and counts; zero total collision triangles returns `None`, even
with liquids or doodads, and means no raw file. It does not decide whether
doodads make the overall extraction successful. Unskipped empty groups remain
in output when another group has faces.

The raw header stores **triangle count** at offset 8, emitted groups, root ID
and model flags zero. GRP/INDX/VERT preserve source ordering and the declared
u16/physical u32 index anomaly. LIQU stores the mapped type, and for flag `0x1`
the exact 30-byte packed header, height words and tile bytes. Header-only flag
`0x2` stores just the type. `RawWmo::parse` validates the complete borrowed
format, totals, exact EOF, finite values, bounds, indices and liquid shape;
it permits WMO u32 indices beyond the M2 u16 domain.

Emitted MLIQ requires nonzero tile product and height product equal to
`(xtiles + 1) * (ytiles + 1)`, with checked arithmetic, without imposing
per-axis equality. The native assembler infers its allocation from tile
dimensions; mismatched height counts are unsafe, and zero tile bytes fail its
read check. This stricter raw admission leaves source views unchanged. Ignored
liquid, header-only liquid and no-output conversions bypass the surface gate.

`ConversionLimits` only reduces source limits, a 64 MiB raw cap and a 256 MiB
combined allocation cap. Context/plan capacities, the single output vector and
each live geometry plus scratch are charged. Borrowed inputs and caller lookup
storage are excluded. Geometry size can reject an impossible output during
the first pass once collision exists; all-zero-triangle input still returns
`None` with a zero raw cap. No partial bytes escape and every result passes
the strict reader before return. The `wmo_raw` example is a bounded synthetic
`RCEWM001` QA caller with exclusive output creation, not a client extractor.

## Doodad selection and external acquisition

`wmo::doodads::requests(&root)` borrows all MODN names in physical byte-offset
order or all nonzero MODI entries by ordinal, including unreferenced targets.
`resolve(&root, &groups, valid_name_indices, limits)` revalidates complete
sources and retains references only from non-skipped groups. Out-of-range
MODR entries and spawns without validated names are omitted. Valid indices
must identify real requests; interior string offsets are rejected. A name
is valid only after the acquiring caller produces positive M2 collision raw
output, not merely `M2Collision::parse` returning `None`.

The owned `Selection` retains sorted unique spawn references and set ranges;
its memory ceiling includes scratch and reports retained vector capacities.
`outcome(has_geometry)` admits a doodad-only result only when both sets and
valid references exist. `set_references(set)` iterates set zero followed by
a valid nonzero requested set, retaining overlap without copying a vector.
It makes no portable ordering claim about the native unordered container.
Nineteen focused tests cover this selection API; the separate transform and
placement APIs are described below.

`forever_casc::collision::Extractor` separately acquires M2/WMO roots, eagerly
requests every root doodad dependency, and loads all detailed GFID groups
before geometry skips. Successful `finish()` returns owned raw buffers and
WMO root/binding/reference metadata; any request failure poisons completion
and no partial extraction escapes. Typed FDID and logical-name requests stay
distinct. Names require explicit LocalStore activation; `FILEXXXXXXXX.xxx`
text never silently switches to numeric routing. Cache reuse requires the
same class and canonical origin; conflicting basenames fail.

Its default limits include 64 MiB files, 4,096 reads/models/explicit requests
and dependency requests each, 256 MiB source/raw totals and a 1 GiB combined
logical store/collector/scratch budget. Dependency cache hits still count.
The lookup is caller-owned and has a separate budget. The Linux
`forever-casc` example `collision_local <client> <plan> <new-output> [--names]`
uses explicit esES/contiguous-v7/Header50893V2 acquisition and a 256-request,
64 KiB plan. It writes private ordinal raw files only after successful finish,
and loads materialized liquid tables for WMO plans. It is QA, not original
GameObject startup or a versioned product CLI. Real acceptance is pending;
see [the detailed contract](../../docs/forever-collision.md).

## Direct placement API

`placement::Mddf::parse(bytes, limits)` and `Modf::parse` borrow exact 36-byte
and 64-byte source records. Both expose source/unique IDs at offsets 0/4,
position/rotation at 8/20, scale and flags. MODF additionally retains bounds,
DoodadSet and NameSet. `uses_file_data_id()` reports source flag `0x40` for
MDDF or `0x8` for MODF; it performs no routing or acquisition. Unknown source
flags remain opaque. Finite used floats, ordered bounds and complete extents
are stricter admission policies than the original unchecked reader.

`project_m2(&source, assigned_id, name, Context, limits)` and `project_wmo`
produce a placement borrowing the caller's already-normalized ASCII name
(1–500 bytes, no NUL). The caller supplies IDs; no native allocator, name
normalization or collision filtering runs. Both permute position to `(z,x,y)`
and preserve rotation. MDDF uses scale/1024, NameSet zero and ignores
`Context.global_wmo`. MODF permutes its bounds, uses scale/1024 only with
source flag `0x4` (otherwise 1), and truncates NameSet to u8. Zero scale is
valid. Global WMO adds `(533.33333f*32, 533.33333f*32, +0.0f)` to position and
both bounds, including the third-axis addition that changes negative zero.
Bound/parent/path output bits are `0x1`/`0x2`/`0x4`; the parent bit compares
context map IDs, and the path bit follows MODF source flag `0x1`.

`placement::encode(&[Placement], limits)` emits complete concatenated records;
`DirBin::parse` validates the entire stream before exposing borrowed records.
There is no magic/count/padding: flags and NameSet are u8, followed by LE u32
ID, position/rotation/scale words, optional six bound words, and u32 name
length plus bytes. Size is **38 + name length**, or **62 + name length** with
bounds. The codec admits flags 0–7, preserves order/duplicate IDs/f32 bits,
and does not infer model class from the bound bit. Empty streams are valid;
truncated tails never yield partial success.

Reducible limits are 64 MiB per operation, one million records and 500 bytes
per name. Encoding checks actual output capacity, while parsing/projection
borrow data. Twenty-nine passing synthetic tests bring VMap to **140 tests**.
The native QA wrapper has built with original `Doodad::Extract`,
`MapObject::Extract` and ModelSpawn read/write retained; its supplied-ID guard
is explicitly authored. `native-placement-20261011T025944Z-fce79711.json`
records 205 exact synthetic outputs / 15,710 bytes (91 M2, 114 WMO), each
matching Rust, an independent Python wire calculation and the original
ModelSpawn reader/writer roundtrip. The wrapper checks 125 native cache
entries without implementing Rust cache replay. Twenty-five Rust-only negative
cases and two exclusive-output checks pass. Source/binary/build guards and
network denial pass; no G3D implementation unit is compiled.
This does not implement WMO quaternion doodad transforms, placement discovery,
map-parent traversal, ID allocation or spatial assembly. See
[the direct placement contract](../../docs/forever-collision.md#direct-mddfmodf-placements-and-dir_bin)
for source field offsets and boundaries.

## Acceptance boundary

Local structured report
`native-m2-20261011T020651Z-78669c6e.json` records:

- 53 positive synthetic cases: complete raw bytes from Rust M2 parsing and
  serialization exactly equal the original pinned C++ `fixCoordSystem` and
  `Model::ConvertToVMAPModel`, with public Model fields populated from the
  corresponding synthetic geometry. Every Rust output passed `RawM2` validation.
- 18 Rust-only rejection cases, two no-collision cases and two exclusive-output
  cases. The native parser/writer was not used as a malformed-input oracle;
  no-collision handling was not exercised through C++.
- Source, binary and original-reference guards all passed; network tracing
  recorded no network use.
- The external source closure materialized 44 original files/notices, with
  38 actual dependency-file inputs. Seventeen G3D headers were needed for
  compilation; no G3D implementation unit was compiled. `license.cpp` was
  retained only as notice evidence. Retained C++ code had no CASC, G3D or Doodad
  calls and did not include `Model::open`.

The M2 acceptance run passed 33 crate tests. Its coordinated workspace run passed
809 tests (362 CASC, 260 DB2, 154 terrain and 33 VMap), and workspace Clippy
passed with warnings denied.

The later WMO increment adds 27 synthetic tests, including exhaustive coverage
of all 65,536 u16 material flags, GFID ordering and checked getters, split/skip
semantics, narrow/wide geometry, metadata preservation and strict failures.
That foundation's coordinated run passed 882 workspace tests: 408 CASC, 260 DB2,
154 terrain and 60 VMap (33 M2 plus 27 WMO). Workspace Clippy across all
targets passed with warnings denied.

The liquid/raw increment passes 92 VMap tests (33 M2, 27 WMO foundation,
13 liquid and 19 raw). The final coordinated workspace passes 914 tests
(408 CASC, 260 DB2, 154 terrain and 92 VMap), the example build, and Clippy
across all targets with warnings denied.

`native-wmo-20261011T023236Z-4ba2f941.json` records 155 exact complete raw
outputs totaling 1,589,618 bytes, five matching no-collision outcomes,
10 Rust-only negatives and two exclusive-output cases. It executed original
`WMORoot::open` 160 times and `WMOGroup::open` 171 times, with original
`GetLiquidTypeId`, `ShouldSkip`, group writer and root-header writer. An authored
memory I/O adapter implements the original CASCFile class; roots contain
independently framed MOHD/MOGN/GFID chunks. An `ExtractSingleModel` guard is
retained with zero calls, and roots contain no MODN/MODI acquisition requests.
An authored synthetic-table predicate supplies `IsLiquidIgnored`. Original
DB2/native CASC backends, the full exporter, assembler and runtime are not
executed. All guards pass; network tracing records zero network attempts.
The wrapper stayed unchanged during compilation and the executed binary's SHA
matches the recorded build. The external closure has 29 original
files/notices, 24 dependency inputs and no G3D headers/TUs. Original
Errors/StringFormat/fmt and assertions are compiled with inlining, fast math
and floating contraction disabled.

The later report `native-m2-read-20261011T024720Z-df38bfbd.json` also executes
original `Model::open`: 55 complete synthetic inputs produce 53 exact raw
outputs / 898,640 bytes and two matching no-collision outcomes. Eighteen
Rust-only negatives and two exclusive-output cases pass. Its I/O adapter
reads complete memory fixtures; native CASC and `ExtractSingleModel` do not
execute. This adds original input-parser evidence to the earlier writer-only
comparison without claiming real-client acquisition.

`model-name-vectors-20261011T023835Z-b9494a9f.json` records 796 original
`GetPlainName`/`NormalizeFileName` vectors, all matched by the Rust naming unit
test, with the focused replay retained in
`model-name-comparison-20261011T025056Z-db140b5d.json`. Separate legacy-extension rewrite regressions are Rust/source checks;
an original C++ rewrite comparison has not run.

The doodad/collector increment passes 111 VMap tests and 441 CASC tests,
including 19 new pure doodad tests and 33 collector tests (four naming).
Together with 260 DB2 and 154 terrain tests, the current workspace passes
966 library tests and two example tests. All-target Clippy passes with
warnings denied after three test-only lint corrections; the final snapshot
is `collision-workspace-20261011T025127Z-ade49eb5.json`.

The later `collision-live-20261011T024957Z-933c1b49.json` separately validates
31 complete real M2 files / 15,277,222 bytes against strict native CASC.
Original `Model::open` and writing match 28 Rust raw files / 27,100 bytes and
three no-collision outcomes. The integrated collector reacquires and reproduces
all 31; a 32-request plan containing one absent file rejects without output.
Private correlation fields, content MD5s and the exact output inventory are
checked locally. Guards and network denial pass. This explicit FDID sample
does not establish full model discovery or real WMO acquisition.

The direct-placement increment separately passes 995 workspace library tests,
two example tests and all-target Clippy with warnings denied, recorded in
`placement-workspace-20261011T030011Z-9b026ecc.json`.

The later object/ID/doodad increment passes **199 VMap tests**: 20 object
reader tests, 15 serial allocator tests and 24 transform tests supplement the
140 prior tests. Object ADT/WDT views preserve ordinal names and physical
order. The allocator preserves the original counter-before-cache-hit rule,
with explicit stronger collision/wrap/probe limits. One selected doodad's
projection preserves quaternion normalization, matrix order and Euler output.
See [the exact contract and native reports](../../docs/forever-collision.md#object-sources-serial-ids-and-wmo-doodad-transforms):
573 synthetic doodad outputs match original C++ bytes and ModelSpawn
roundtrips, while 96,576 serial ID requests match the original allocator.
These are toolchain-specific synthetic comparisons; no portable libm or
unordered/threaded emission-order equivalence is inferred.

The separate CASC placement bridge joins complete Objects batches with
finished model acquisition, validates identity, expands selected doodads and
returns complete bounded `dir_bin` batches. Its `Session` retains IDs across
calls and poisons after the first error; projection budgets remain per call.
The pure bridge performs no acquisition or publication itself.

`placement::cache::Cached` adds twelve tests, bringing this crate to **211
tests**. It validates complete `DirBin` input, clears only the parent flag bit,
and replays with the appropriate parent bit while retaining IDs and every
other byte. Empty caches succeed. The default 128 MiB budget covers retained
cache and new output capacity; it excludes borrowed inputs and outputs kept
by the caller from previous calls. Replay consumes no allocator IDs.
`native-placement-cache-20261011T032335Z-2d96d15e.json` matches 124 synthetic
cases / 11,144 records / 650,970 bytes against original `initFromCache`,
including three empty caches. The original ADT parser and CASC backend are
not part of this comparison. See the
[planning and replay contract](../../docs/forever-collision.md#map-planning-persistent-ids-and-parent-replay).

The separate `bih` API adds 27 tests, bringing this crate to **238 tests**.
It constructs the original hierarchy from finite ordered bounds with leaf
sizes 1/3, then validates complete serialized bytes. Empty NaN bounds and dead
infinity-marked edges are preserved; live graph/range/permutation checks reject
unsafe structures. Depth 65 is required even though the original entry cutoff
is 64. Bounds/graph validation alone cannot prove containment of unseen geometry.
The dev and release comparisons each match **420 original C++ outputs** /
245,480 primitives / 5,545,784 bytes, including original reader/writer roundtrips
and 35 Rust-only negatives. That BIH comparison did not execute query traversal
or assembly. See
[the BIH contract and evidence](../../docs/forever-collision.md#pure-bih-construction-and-complete-storage)
for capacity/work ceilings, source closure and precise native scope.

The separate `vmo` API converts complete validated raw models, preserving
geometry, declared group bounds and liquids while building mesh/group BIHs.
Its strict reader validates framing, finite values, indices and containment
against the embedded geometry. It owns bounded group descriptors and borrows
geometry/liquid bytes. Twenty-two further tests bring this crate to **260 tests**.
See [the assembled-model contract](../../docs/forever-collision.md#assembled-vmo-models)
for native comparison evidence and the precise input and resource limits.

## Transformed bounds and single-map assembly

`placement::bounds::complete` fills missing bounds from explicitly supplied raw
group-zero bounds. It retains existing bounds without consulting a model.
`transform` preserves the original scale, Z*(Y*X) rotation, eight-corner merge
and final translation order, including signed-zero ties. Both APIs are pure
and allocate no heap; zero/negative scales are allowed and nonfinite results
reject. Their 16 tests include input and enlarged-output budget checks.

`assembly::build` validates all supplied records, keeps the first record per
unsigned ID, sorts the retained records and builds the map BIH with leaf size
three. It returns the complete tree, tile payloads, tile indexes and a retained
ordered `dir_bin` proof. Index entries are ordinals in that sorted placement
sequence, not BIH permutation positions. Own placements precede inherited
placements in each tile. Parent-only tiles have indexes but no payload.
Invalid duplicates and missing bounds reject the whole call.

Tile selection preserves f32 multiplication and truncation toward zero, with
inclusive rectangles and no 0..63 clipping. The admitted coordinates after
truncation are X=0..32767 and Y=0..255, avoiding the native packed-key aliases
outside that range. Globals have no separate tile rule.

Downward limits cover 64 MiB input/proof and each tile file, one million spawns
and tiles, sixteen million associations, 512 MiB artifact bytes and 1 GiB
tracked heap capacity. The retained proof counts toward heap, not artifact
bytes. BIH limits also apply. Caller input/name storage, bounded stack and
allocator overhead are excluded. Twenty-one assembly tests cover construction,
strict readers, associations and reduced limits.

`Tree`, `TileView` and `IndexView` require complete framing and EOF. Association
verification needs the complete supplied ordered proof. The inherited variant
compares the actual ancestor payload in its existing order, ignoring only the
parent flag; it does not resolve an ancestor or reorder its records. These
checks do not authenticate the proof, establish the full tile inventory or
prove containment of geometry absent from the files.

The bounded `map_raw` example is a QA adapter with ordinary file I/O. Its map
stdout bundle is a QA transport, not a production VMap format. See the
[single-map contract](../../docs/forever-collision.md#transformed-bounds-and-single-map-assembly)
for original-source comparisons and the remaining integration gates.

Both dev and release compare 274 bounds cases and 136 complete maps against
original C++, including all 62,006 map/tile/index files and 49 Rust-only negative
cases per profile. That increment passed **297 tests**. Native MapTree
fallback, real map acquisition and runtime queries remain separate gates.

## GameObject model lists

`gameobject_models::build(&[ResolvedRow], Limits)` takes explicit M2/WMO raw
bytes for each display-ID/name row and returns both the `VMAP04E\0` temporary
list and `VMAP_4.E` `GameObjectModels.dtree` bytes. `TempList` and `Dtree` are
complete borrowed readers. Rows retain order and duplicates. Bounds merge every
physical vertex of every group, including vertices unused by triangles; declared
bounds and liquids do not contribute. The existing raw readers still validate
all metadata, indices, bounds and liquid framing. New vertices win equal extrema,
including signed zero. Missing, malformed or vertex-empty models fail the entire
operation. Each row resolves independently; the caller must bind repeated names
to immutable, identical class/content when representing a native file inventory.

The codecs require ASCII names of 1..499 bytes without NUL, finite ordered bounds
and exact EOF. They perform no path normalization or acquisition. The ordinary
file QA example applies printable basename admission up to 255 bytes separately.
Exact-zero bounds can serialize, although the native dynamic-model initializer
rejects them. Neither that initializer nor the native dynamic consumer runs here.

Downward limits cover 64 MiB per list/raw input (with existing narrower raw
profiles retained), one million rows, 256 MiB cumulative raw-byte visits, 128
million projection vertex visits and 256 MiB counted heap. Repeated references
are charged again. Preflight validates row/dependency lengths before raw parsing
or allocation. Heap counts actual capacities of both outputs and temporary row
bounds together; borrowed inputs, stack and allocator overhead are excluded.
The projection counter covers the bounds fold; raw validation passes are bounded
by cumulative raw bytes. These are logical budgets, not an RSS guarantee.

Twenty-three new tests bring the crate to **320 passing tests**. Both dev and
release match **276 synthetic lists / 536 rows / 24,895 final-list bytes** against
original public `exportGameobjectModels`; each scans 410,587 vertices and 6,709,056
raw bytes. Twenty-seven Rust-only negatives and an existing-stage guard pass.
See [the complete comparison scope](../../docs/forever-collision.md#gameobject-model-lists).
The companion `forever-db2::terrain_sources::read_gameobject_display_info`
adapter now validates the complete closed nine-field/fourteen-scalar profile and
preserves direct ordered rows, duplicates, repaired IDs and zero FDIDs. It casts
physical signed32 field 1 to u32 without changing bits; native lookup slot 7
includes the external ID and six GeoBox scalars. Ignored GeoBox float bits are
not finiteness-gated. Copy pairs validate and remain inspectable but have no
materialize/replay operation. Filtering zero FDIDs and generating candidate
names belong to its QA caller or the separate acquisition layer.

Nineteen DB2 unit tests and 50 native synthetic cases / 1,164 direct rows pass in
dev/release; that increment raised the workspace to 1,300 library tests while
VMap stayed at 320. See [the source contract and comparison](../../docs/forever-collision.md#gameobjectdisplayinfo-direct-source-projection).

The separate `forever_casc::collision::gameobjects` startup now feeds this pure
list builder from complete DB2 and model acquisitions. It classifies the first
four bytes of the same verified source, preserves direct order and duplicates,
and omits zero FDIDs, textures, M2 NoCollision and WMO DoodadOnly from both lists.
Copies are validated without replay. `Context::with_lookup` retains models made
with initially empty liquid lookups, handles and acquisition counters.

Both dev/release startup comparisons match **36 synthetic cases / 469 direct
rows / 127 listed rows**, **46 raws / 6,855 bytes** and **9,720 bytes in both
lists**, executing the original GameObject producer and exporter against an
authored memory inventory. Rust uses complete synthetic CASC with explicit FDIDs;
native CASC routing equivalence is not claimed. Seven Rust negatives and two
exclusive-output guards pass per profile. Forty new CASC tests and one DB2 test
bring the workspace to **1,341 library tests**, with VMap unchanged at **320**,
two example tests and all-target Clippy.

Startup limits include 16 MiB DB2, 71,428 direct rows and 4,096 candidate calls.
Its decoder admission allowance is conservative, not exact heap/RSS accounting;
source and retained capacities remain charged separately. The private startup
QA example writes only after finish and is not a product publication protocol.
The subsequent `forever_casc::collision::world::extract` consumes startup,
loads authenticated Map → LiquidMaterial → LiquidType, then uses the shared
WDT/Obj0ADT runner without rebuilding cached models or resetting counters. Its
lookups replay direct last-wins values and ordered copies with missing-source
default insertion. Owned tables and graph memory stay in the collector ledger;
all are dropped before final transfer. The old per-batch runner is unchanged.
Joint dev/release acceptance now matches **49 complete synthetic cases** each:
**43 listed rows**, **82 RAW files / 13,537 bytes**, **3,880 list bytes**, and
**79 per-map `dir_bin` files / 11,688 bytes / 176 placements**. Original startup,
Map/liquid loaders, `ParsMapFiles`, model converters/projectors/cache and dtree
export execute against a memory inventory with one worker; Rust uses complete
toy CASC. Fifteen Rust-only negatives and two exclusivity guards pass per
profile, with 115 empty network traces. This does not establish native CASC
routing or native parallel/unordered scheduling. That workspace passed
**1,378 library tests** (610 CASC, 294 DB2, 154 terrain, VMap still **320**),
two examples, 202 focused collision tests and all-target Clippy. The preceding
1,341-test startup acceptance remains a separate historical step.

Real source tables, product publication and runtime remain gates. See the
[shared world contract and reports](../../docs/forever-collision.md#shared-startup-and-world-execution)
and retained [startup evidence](../../docs/forever-collision.md#gameobject-startup-acquisition-and-cache).

## Shared-world package integration

The separate `forever_casc::collision::assemble::extract` now consumes an
authenticated startup, runs the shared world with a private placement stage,
and uses this crate's existing bounds, VMO and map builders. It returns all
selected models, maps and dtree as one complete in-memory result. Names are
selected from all placements before first-ID-wins map deduplication and from
startup list rows. Unused acquired dependencies are not converted. Parent-only
child indexes must match the first existing actual-ancestor payload; present
mismatches and missing payloads reject the package without farther fallback.

Artifacts use native nested `<map>/<map>.vmtree` and
`<map>/<map>_<x>_<y>.vmtile` / `.vmtileidx` paths; VMOs and dtree stay at the
root. Limits cover 1 GiB logical memory, a 256 MiB staging allowance, 512 MiB
artifact bytes, 65,536 files and cumulative geometry/RAW/BIH/proof work. Actual
capacity and growth overlap are counted, with the final caller-owned store
snapshot separate from returned world storage. These are not RSS guarantees.
Both debug and release compare **53 complete synthetic cases / 650 final files /
56,019 bytes**, including **87 selected VMOs**, **180 unique spawns** and
**303 associations** (27 parent-only tiles and one mixed tile), against original
startup/world processing and `convertWorld2`. Forty Rust-only negatives include
25 strict parent-association failures; two exclusivity guards and 148 empty
network traces pass per profile. That workspace passed **1,413 library tests**
(645 CASC, 294 DB2, 154 terrain, VMap still **320**), two example tests, 237
focused collision tests and all-target Clippy. The preceding 1,378-test world
report retains its historical scope. Native CASC routing and runtime consumers
are not exercised. The private QA example publishes no product manifest or
crash-safe transaction. See the
[complete assembly contract](../../docs/forever-collision.md#shared-world-collision-assembly).

RustyCore currently exposes a static VMap LOS provider interface; that
interface alone does not establish a reader of these intermediate or assembled
formats. No complete VMap extraction or runtime compatibility is claimed.


## Model and instance ray queries

`query::{Ray, Mode, Limits, Budget}` provides finite rays and a cumulative work
ledger. `query::model::intersect` borrows a complete `vmo::View`; `Instance::new`
precomputes a checked transform from a bounded `Placement`, and
`Instance::intersect` reuses it. The one-shot `model::instance` is equivalent.
`Result<Option<Hit>, Error>` distinguishes errors, misses and hits; a hit exposes
the distance bits and group/triangle ordinals. Model identity and placement-name
binding remain the caller's responsibility.

Directions are never normalized. Distances are ray parameters, and become
physical distances only for unit directions. Public maxima admit zero and
positive infinity. The two-sided triangle test preserves the strict determinant
threshold, inclusive barycentric edges and strict positive/less-than-maximum
distance comparisons. A single group bypasses the group BIH and declared group
bounds; group queries use their mesh BIHs. `Mode::Any` follows traversal order,
while `Mode::Nearest` retains the closest admitted hit. `ignore_m2` uses the
model's IsM2 flag; path-only placement filtering belongs to a later map loader.

Instance queries use Z*(Y*X), a general cofactor inverse, the source AABox
candidate-plane gate and the source origin/direction/scale operation order.
Finite negative scales are accepted when their reciprocal is finite; the local
maximum may consequently be negative. Zero scales, invalid transforms and
nonfinite arithmetic outside the admitted infinity behavior are explicit errors.
No coordinate flip, segment LOS, height, area or liquid convenience is supplied.

One budget spans nested traversals and successive calls. Defaults only reduce
16 million node visits and 16 million triangle tests; exhaustion is an error.
Traversal uses 65 stack entries per nested tree and no heap. Attempting a native
phantom/dead infinity-marked edge returns `UnsafeTraversal`; it neither returns
a fabricated miss nor falls back to a full scan. Counters describe Rust work,
not exact C++ counters or all arithmetic/validation work.

The 54 query tests raise this crate to **374 tests** and the workspace to
**1,467 library tests** (645 CASC, 294 DB2, 154 terrain, 374 VMap), with two
example tests and all-target Clippy passing. Final debug/release each match
**79 synthetic cases / 14,416 queries** against original WorldModel and
ModelInstance consumers: **2,950 hits / 11,466 misses**, with exact hit/distance
bits and no excluded case. The same corpus contains 11,850 direct-model and
2,566 instance queries, 10,038 nearest and 4,378 Any queries, 26 known-result
checks and a safe-leaf positive-infinite-distance case. Each profile passes
14 Rust-only negatives and two exclusivity guards; all 174 network traces are
empty. This executes original model/spawn readers and geometry queries, without
StaticMapTree/VMapManager package loading, model-name binding or RustyCore.
The Linux `collision_query` QA example accepts one complete VMO, an optional
single bounded placement and an entire bounded ray plan; every query completes
before private output is created.
It preserves the original maximum bits on misses and uses one shared budget
across the plan. It performs no acquisition or runtime initialization. See the
[query contract](../../docs/forever-collision.md#model-and-instance-ray-queries)
for the QA envelope, numeric admission and remaining map/runtime gates.


## Static package store and manager queries

`store::StaticStore<S>` owns complete map trees, exact-name VMOs, tile references
and cached instance transforms. `Source` transfers complete artifacts through a
bounded `ReadBuffer`; the caller supplies explicit primary/cosmetic parent rows.
Tree/model readers retain immutable bytes and validated offset layouts, so a
query neither reparses artifacts nor performs filesystem I/O or heap allocation.

Loading prepares every resource and checks the source before committing. A failed
load preserves existing logical state; a detected input change permanently
invalidates the store. Repeated loads append references, and one tile unload
releases all of that tile's loads. Models are shared by exact name across maps.
Parent payloads use the first present file in the actual parent chain with the
child's index; a present invalid file is terminal. Known bindings may differ only
in the parent flag, and the first binding is retained. Intrinsic indexes, bounds
and known ID order do not prove an unavailable assembler inventory. Path-only
placements are filtered by an explicit option. Missing models and inconsistent
bindings reject atomically; native failure-admission parity is not claimed.

Limits only reduce 128 MiB per artifact, 1 GiB store capacity, 4,096 declared maps,
65,536 tiles/models, one million primitive slots and 16 million tile references.
Each load also caps source bytes at 1 GiB and admitted validation work at
128 million units. Actual capacities and allocation overlap are charged; source
metadata, stack and allocator overhead are separate. Queries retain their shared
16-million-node/triangle budget. These counters are not RSS or all CPU work.

The Linux `store::fs::RootedSource` anchors reads to directory descriptors,
checks identities and exact EOF, and hashes every read with streaming SHA-256.
Repeated reads must match; `verify_unchanged` checks recorded metadata/absence,
not every byte again and not an atomic filesystem snapshot. Metadata capacity
has a separate 128 MiB/65,536-entry ceiling, with 262,144 metadata operations per
read/verification. Model names retain their case. No CASC or network is involved.

The `line_of_sight_*`, `height_*` and `object_hit_*` methods offer world and
already-reflected internal coordinates. They preserve native f32 reflection,
segment normalization, strict hit distances, negative hit-offset clamping and
the double reflection of loaded world misses. Height returns `Option<f32>`;
`INVALID_HEIGHT` is available for the native sentinel projection. Global LOS
and height switches are independent. The native default per-map callback is
assumed. This supplies neither area/liquid queries nor dynamic objects and does
not install a RustyCore provider. See the
[static package contract](../../docs/forever-collision.md#static-package-loading-and-manager-queries).


Debug/release each match the original static manager on **75 synthetic cases /
3,298 operations**, including 339 loads, 305 unloads and 2,654 LOS/height/object-hit
queries. All result words and visible slot/model counts agree, with 38,274
independent field checks. Fifty-eight Rust-only negatives, two output guards
and 210 empty network traces pass per profile. The native profile disables
logging/metrics/ordinary assertions, while retaining original geometry code;
malformed inputs never reach it. The current workspace passes **1,558 library
tests**, **two example tests**, **66 focused store tests** and all-target Clippy.
The prior 79-case/14,416-query comparison also passes unchanged in both profiles.
Reports and admission limits are in the static package contract above.

The separate RustyCore `forever-terrain` feature now composes this actual rooted
store with its GridMap reader and a shared physical-tile lease owner. Its checked
static height/LOS/object-hit library has synthetic production-module acceptance;
the Forever server does not instantiate it. See the
[reader contract](../../docs/forever-collision.md#checked-rustycore-terrain-library).
