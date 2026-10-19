# Forever navigation geometry

`forever-navigation` provides pure Rust navigation geometry and Recast stages for the fixed
Forever reference `02245dcd245e7433e524577656177723d3e4992e`. It consumes complete,
already verified MAPS v10 objects and produces indexed solid/liquid geometry.
It also appends supplied VMO/placement geometry and projects explicitly supplied
off-mesh connections, as described in the later model-input section below.
The `recast` stage rasterizes this geometry, filters walkability, constructs
compact-heightfield connections and applies erosion, median area filtering and
distance-field generation, followed by watershed regions and simplified
contours with hole joins, polygon/detail meshes and paired subtile merging.
The Detour stage serializes version-7 tile payloads with native and RustyCore
synthetic reader/query acceptance. Versioned MMap 16 container writers and strict
readers feed those public runtime APIs. Supplied TileBuilder computation is now
implemented, including the original fresh-navmesh save-time mutation.
Numeric Map projection and supplied-descriptor ancestor inventory planning are
also implemented. Active static-store model selection and tile load/unload now
feed the geometry stage directly. Rooted terrain loading resolves center and
neighbor files through the effective-parent graph. Product/runtime integration and real navigation remain open. Each increment's
evidence is recorded below.

## Supplied terrain API

`terrain::project(&[Region], skip_liquids, Limits)` accepts one to five regions
in explicit order. Each `Region` contains a `forever_terrain::maps::MapView`,
filename-order `row` and `column` (both 0..63), and `Portion::{Entire, Top,
Right, Left, Bottom}`. These coordinates are **not** ADT `(x, y)` order:
`maps/{map:04}_{row:02}_{column:02}.map` becomes original TerrainBuilder
`tileX=row, tileY=column`. All declared build tags must match. Accepted tags
70170/70205 do not authenticate the provenance of caller-supplied bytes.

`project_neighborhood(center, Neighbors, skip_liquids, Limits)` requires an
Entire center. It processes the following supplied neighbors in original order:

| Neighbor | Portion |
| --- | --- |
| column + 1 | Left |
| column - 1 | Right |
| row + 1 | Top |
| row - 1 | Bottom |

If the center contributes no usable triangle, neighbors are not projected.
Admission still checks every supplied region and reserves its conservative
maximum before that decision. `None` is explicit absence supplied by the caller;
there is no acquisition or automatic conversion of a read error into absence.
Supplying an out-of-range neighbor rejects. Original ancestor-file lookup and
missing-file policy remain outside this API and its evidence.

The returned `Mesh` owns separately indexed solid and liquid vertices/triangles,
plus one navigation area byte per liquid triangle. Coordinates are Recast
`(y, height, x)` after mirroring both horizontal axes. All output vertices are
finite. Accessors borrow immutable slices. Any failure discards the whole result;
there is no caller-visible partial append.

### Preserved consumer behavior

- V9/V8 height reconstruction supports float, u8 and u16 MHGT samples with the
  original f32 operation order. `NoHeight` emits **no** solid surface. Flight
  bounds and area IDs do not participate in this navigation projection.
- Terrain keeps four triangles per square, in top/right/left/bottom order;
  liquid keeps two, with independent vertex indices. Hole bits remove solids.
- Water/ocean map to area 9, magma/slime to 8. Water takes precedence over
  magma/slime when both kinds are set. DarkWater disables both surfaces;
  `skip_liquids` also disables that exclusion. Other flags alone add no liquid.
- Variable liquid crops fill missing vertices with -2000. Partial triangles
  use their original, unmodified heights for the average; repairs affect the
  evolving output only. Validity is `h != -2000 && h < 5000`, while repair is
  `h == -2000 || h > 5000`. Exactly 5000 is deliberately neither valid nor
  repaired. An overflowing executed average rejects in Rust.
- The higher surface is selected by the original per-half-square extrema
  comparison, with its -2000/5000 initial extrema. Equal/overlapping ranges can
  retain both kinds. Unused vertices remain until explicit compaction.
- `Mesh::compact` consumes the mesh and renumbers each surface by first index
  encounter, removing unused vertices. It does not weld equal positions with
  different source indices. Liquid areas and triangle order are preserved.

Rust requires the existing strict complete MAPS profile. It additionally rejects
out-of-range coordinates, mixed builds, nonfinite computed geometry and policy
overruns. Native failure-admission parity or malformed-input safety is not claimed.

## Resource boundary

All overrides only reduce these limits:

| Quantity | Ceiling |
| --- | ---: |
| Supplied regions | 5 |
| Combined solid/liquid vertices | 250,000 |
| Combined solid/liquid triangles | 500,000 |
| Logical heap capacity | 24 MiB |

Before projection, the exact maximum vertex/triangle counts for each portion
are reserved, including geometry that filtering may later remove. Liquid repair
retains at most one 16,641-element original-height buffer. Actual Vec capacities
are rechecked; allocations are fallible. Compaction charges the resident mesh,
its complete remap and replacement vertex vector simultaneously. Five entire
regions exercise 248,330 input vertices and a 491,520-triangle reservation.
Caller-owned MAPS buffers, allocator overhead and RSS are outside this logical
capacity accounting. Work is bounded by the fixed region/grid limits; no claim
of exact native CPU counters is made.

## Executed synthetic acceptance — 2026-10-11

There are **21 passing unit tests**, including grid winding and signed-zero
coordinates, all borders, height encodings, holes, liquid types, cropped repair,
surface selection, distinct duplicate vertices, center gating, coordinate/build
errors, downward limits, overflow rejection and the maximum five-region case.
One ignored test writes a wholly authored corpus only when explicitly pointed
at a new external directory. It ran separately in both profiles.

Both debug and release match **106 cases / 50,763,162 serialized bytes** against
the original C++ consumer, with identical case arrays and Rust source hashes:

- 1,938,533 solid vertices and 1,009,680 liquid vertices;
- 822,040 solid triangles and 424,462 liquid triangles/area bytes;
- 96 varied single-region cases across both declared builds, eight neighborhood
  cases and two five-entire-region cases;
- every coordinate word, index and area byte agrees; independent extent/index/
  area checks also pass. Native outputs and Rust outputs remain outside Git.

The authored QA wrapper calls the **unchanged original region `loadMap` and
`cleanVertices`**; neighborhood cases call the **original public `loadMap`
dispatcher**, including its empty-center gate. Only access to the private region
entry point is widened in the wrapper's header inclusion. Native files are
complete synthetic MAPS accepted by Rust first. It does not run the actual MMap
generator, VMap geometry loading, parent fallback, Recast, Detour or runtime
readers. The native target uses the pinned source/headers, original supporting
objects and verified Boost 1.83.0 headers, with preserved notices externally.
No upstream implementation or new external crate was imported into this repo.
`PERFORMANCE_PROFILING` disables native logging/metrics/ordinary Trinity
assertions, as in the previous native consumer closure. Float contraction and
fast math are disabled. The reused object and actual dependency hashes are
guarded in the build/acceptance reports.

Retained reports under the external laboratory's `reports/acceptance/`:

| Report | SHA-256 |
| --- | --- |
| `navigation-terrain-debug-20261011T072046Z-c4175748.json` | `6b66ea6a3865f117f2f1d2d050241c504da2a4ab9e6b2e0fdf2f1de1f9d7672f` |
| `navigation-terrain-release-20261011T072046Z-1fe49760.json` | `354f8cc6d0deb156f5bba307abfeb11c602b84f42aabe5046ef1338f83fb05b7` |
| `navigation-workspace-20261011T072104Z-56e6695f.json` | `6541f8ac025f06583c5643cbea3e64772c45b84025c57b51ba9d3b5ddbcec462` |

Each profile has 107 empty network syscall traces: 106 original-consumer calls
and the Rust corpus execution. Runtime network syscalls were denied. An earlier
run was rejected by the harness because its trace included process signals;
inspection found no network syscall. Final traces omit signals and retain the
same complete network syscall denial. A missing native `Util.h` closure input,
initial fixture argument-order error and test-only Clippy suggestions were fixed
before acceptance. All failed attempts are distinct from the accepted reports.

The workspace passes **1,605 library tests** (669 CASC, 296 DB2, 154 terrain,
465 VMap, 21 navigation), four example tests and all-target Clippy through the
configured helpers. Both ordinary ignored corpus tests retain explicit separate
acceptance. Formatting checked only the new crate; unrelated dirty work remains.

## Remaining navigation work

Supplied VMap/off-mesh geometry is implemented below; real package loading,
instance selection and its complete source tables remain separate gates.
The later sections record Recast rasterization, filtering, regions, contours,
polygon/detail meshes, paired merging and exact supported Detour tile payloads.
The supplied synthetic payload/query acceptance does not establish real routes.
The Forever container is MMap **16** with its 40-byte global header, not legacy
MMap 15/28-byte parameters. Complete source acquisition, product orchestration,
real output comparison and usable pathfinding acceptance remain required by the
[full completion checklist](forever-completion.md).

## Supplied model and off-mesh input — 2026-10-11

`models::append(prefix: Mesh, models: &[Model], LiquidTypes, models::Limits)`
consumes a terrain mesh and appends each supplied complete VMO/placement in
caller order. It uses the actual `forever-vmap::query::model::Instance` checked
cofactor inverse, position and scale through three new copy-only accessors.
The pre-existing instance construction/query implementation is byte-for-byte
unchanged after removing those accessors; its 465 library tests still pass.

Each model contributes all groups and all vertices, including unreferenced
vertices until explicit `Mesh::compact`. M2 triangles reverse their index order;
WMO triangles keep it. The placement origin is shifted by `32 * GRID_SIZE` on
both horizontal axes. Vertex row-vector multiplication by the actual inverse,
scale and translation precede horizontal mirroring and `(y, height, x)` output.
This is not a transpose shortcut for the inverse. Existing checked instance
admission requires complete bounds and finite, nonzero scales with finite
reciprocals. Negative scales remain admitted; nonfinite computed geometry fails.

WMO liquid height data is read row-major, output vertices are x-major, and flag
lookup/triangle winding retain the original rectangular-grid behavior. Tile
flags whose low nibble is 15 suppress that liquid square. Flat 0/0 liquid grids
have no flag storage and contribute no liquid geometry. All other grid vertices
remain until compaction, even when their tiles are suppressed. The corner's Z
value does not replace stored liquid heights.

`LiquidTypes::new` borrows at most 65,536 sorted unique `(ID, SoundBank)` pairs.
The caller must reduce **direct** LiquidType rows with last-direct-ID-wins:
the pinned navigation `PathGenerator::LoadLiquid` does not replay copies.
This API performs no DB2 acquisition/reduction. Unknown IDs return zero flags.
SoundBank 0..30 uses the original shift and low-byte conversion; >=31 rejects
the native signed-shift undefined domain. Water/ocean select area 9 and
magma/slime area 8. Unknown/DarkWater/other types **retain their triangles with
area 0**. Unlike terrain MAPS projection, `loadVMapModel` does not consult the
original `skipLiquid` option, so this API always processes supplied model grids.

Model acquisition, VMO-name/placement binding, map-tree inventory and instance
selection, parent resolution and `LoadPathOnlyModels` remain caller obligations.
No visibility or bounds-containment filtering is silently substituted for the
native selected-instance inventory. This stage does not execute `loadVMap`'s
manager lifecycle. A supplied checked VMO is a structural proof, not proof that
it is the correct model for a placement.

### Off-mesh selection

`offmesh::select(Target, &[Source], Limits)` matches map ID and filename-order
row/column exactly. It keeps source order and duplicate records, reorders each
endpoint to `(y,z,x)` without mirroring, masks connection direction to bit 0,
and retains radius, all area bits and all 16 terrain-flag bits. Selected endpoints
and radii must be finite. Finite zero/negative radii remain unchanged at this
projection stage; the later Detour builder rejects negative radii. Unselected records
are not numerically inspected. Target row/column must be 0..63; map IDs are u32.
The immutable `Connections` result owns one typed record per selected source.
There is no text input parser, global MMap record codec, endpoint snapping or
pathfinding claim.

### Additional resource boundaries

Model projection retains the previous 250,000 combined vertices, 500,000 combined
triangles and 24 MiB mesh-capacity ceilings. It adds at most 4,096 supplied models
and 65,536 cumulative groups, including empty groups. All settings only reduce.
Complete counts are checked before output allocation. The consumed prefix and
its full replacement coexist during copying and are charged together; then the
prefix is dropped before model projection. No transform cache allocation occurs.
Each checked model/placement and liquid lookup is borrowed; their caller-owned
storage is outside this admission budget. These are logical capacities rather
than RSS or total-process memory. Failures expose no partial mesh.

Off-mesh projection separately caps input/output at 65,536 records and owned
capacity at 2 MiB. All selected numerics validate before allocation. This is
separate from the mesh budget; a future product coordinator must account for
both plus its retained input readers. Unit evidence reaches the exact model,
cumulative-group and connection ceilings; projection and compaction counters
do not establish general CPU/RSS limits for caller code.

### Executed model acceptance

Twenty added tests (13 model, seven off-mesh) bring navigation to **41 passing
tests**, with two corpus writers ignored normally and explicitly executed.
Both debug and release match **97 cases / 3,951,732 bytes** against original C++:

- 239 original VMO reads, 239 original placement reads/ModelInstance constructions,
  with 336 model groups and nine supplied MAPS prefixes;
- 135,560 solid vertices, 4,550 liquid vertices, 5,268 solid triangles and 3,204
  liquid triangles/area bytes;
- 67,552 selected off-mesh connections, including one exact 65,536-record case;
- original `loadVMapModel`, `loadOffMeshConnections` and optional `cleanVertices`,
  plus original MAPS region loading for terrain prefixes. The authored lookup
  callback supplies the same bounded type mapping; original DB2 loading is not run.

Every serialized coordinate bit, triangle index, liquid area and off-mesh field
matches. Independent array extent/index/area/direction checks also pass. Debug
and release case arrays and source hashes agree. Original C++/G3D/Boost sources,
used objects, binaries and all synthetic corpus inputs retain their hashes.
Native inputs are complete synthetic positives after strict Rust success;
native malformed/failure behavior is not inferred. Each profile has **98 empty
network syscall traces**. The prior terrain corpus is repeated in both profiles:
all 106 case arrays remain exactly equal to the earlier accepted checkpoint,
with another 107 empty traces per profile.

The workspace passes **1,625 library tests**, four example tests and all-target
Clippy. The three ordinarily ignored corpora have separate explicit evidence
(the unchanged CASC terrain corpus retains its earlier acceptance).

Retained external `reports/acceptance/` artifacts:

| Report | SHA-256 |
| --- | --- |
| `navigation-model-debug-20261011T072946Z-4fe49672.json` | `0f79ff6368a1747fac56321c2280349db236cf3847bfb0b030fe6ccbeb4deacc` |
| `navigation-model-release-20261011T072946Z-84cd392f.json` | `c6431fdfbd52938eeb2afdf087436d46333f01f3fd300a2a26b0100d819c26d5` |
| `navigation-terrain-debug-20261011T073002Z-1859c9e3.json` | `bd173ba534292a4e3fb96857f120732af6db6c293331140160e099dbd5a77ed1` |
| `navigation-terrain-release-20261011T073002Z-0b539e6c.json` | `1f2cd79b2615dd80e3d1f64faa1a1c9547b2a9a6f547032902316550d41378dc` |
| `navigation-workspace-20261011T072946Z-e818d243.json` | `940614ee3a1f8aa8bc868b2cf70c1cce2f8c336d61f3793be94a086161a3692d` |

Only an existing local project crate dependency was added. No upstream source,
new registry dependency or client artifact was imported. There is no real
navigation acceptance, navmesh generation or claim that the full extractor is
complete. Next stages still include actual input inventory/orchestration,
complete Recast processing, Detour/MMap output and usable runtime routes.

## Initial Recast heightfield — 2026-10-11

`recast::build(&Mesh, Grid, Slopes, Agent, Limits)` now performs the pinned
TileBuilder stage order: classify and rasterize solids; filter low obstacles,
ledges and insufficient clearance; rasterize liquids; construct initial compact
cells/spans and their four neighbor connections. `Heightfield` exposes the same
consuming stages separately, plus explicit voxel-span insertion. Failures return
no partial field. All owned arrays and iterators are immutable to callers.

The caller supplies a finite grid (each dimension 1..1024), positive cell sizes,
agent height/climb in voxel units and ordered slope thresholds in 0..90 degrees.
The initial comparison used 70/55-degree slopes explicitly. The subsequent
configuration correction below changes `Slopes::default()` to the actual native
55/55-degree defaults. Agent and grid configuration have no implicit map defaults.
Bounds need not equal dimensions multiplied by cell size.
This API does not select original subtiles, borders or map-specific settings.

Preserved behavior includes:

- Forever's **16-bit** voxel heights, touching-interval merging, and area merging
  **before** expanding overlapping height intervals. The arena allocates the
  new span before merging and reuses removed entries.
- Original f32 clipping/order, grid truncation and floor/ceil quantization, with
  at least one voxel of thickness. No extra lower-bound clipping is introduced.
- Solid areas 0, 10 and 11. Degenerate normals retain area 10 through the native
  comparisons; the finite-coordinate gate does not invent a degeneracy filter.
- Low-obstacle area inheritance uses the previous span's original walkability;
  it cannot propagate through a chain of newly marked blockers. Ledges use
  strict clearance `> height`; the clearance filter rejects `<= height`.
- Liquids enter after these filters. Compact adjacency instead permits a gap
  equal to agent height and uses the first reachable neighbor layer. Clearance
  saturates at 255; region IDs start at zero. Empty and null-only source columns
  retain the original distinct cell-index behavior.

Rust rejects nonfinite coordinates, invalid indices/areas, unsafe float-to-int
domains, clipping-capacity overflow and policy overruns. Explicit spans require
`min < max` and area 0..63. A quantized lower endpoint of 65535 rejects rather
than reproducing native upper-endpoint wrap. Compact columns reject more than
255 active spans and any reachable neighbor layer >=63; native code can wrap
its count or log and return incomplete connectivity. Those domains are **Rust
rejection evidence**, not claims of native failure parity. Isolated columns
with 255 layers and connected layer 62 have positive native comparisons.

### Recast resource limits

All overrides only reduce these separate ceilings:

| Quantity | Ceiling |
| --- | ---: |
| Cells | 1,048,576 |
| Arena span slots / one insertion slice | 2,000,000 |
| Vertices / triangles per raster call | 2,000,000 / 1,000,000 |
| Logical heap capacity | 128 MiB |
| Cumulative charged work across stages | 128,000,000 |

Growth charges old and replacement capacities simultaneously. Compaction charges
the source heightfield and destination arrays together. Actual Vec capacities
are rechecked and allocations are fallible. The span limit counts arena slots,
including reusable slots, rather than only currently live spans. Input meshes,
allocator overhead and RSS are excluded; the work counter is a bounded traversal
policy, not a CPU-time measurement. The product coordinator must budget its
retained geometry separately. Exact reduced capacity/work boundaries and their
one-unit failures have unit evidence.

### Executed heightfield acceptance

There are **23 new passing tests**, bringing navigation to 64. The workspace
passes **1,648 library tests**, four example tests, all-target Clippy and scoped
formatting. Four corpus tests across the workspace are ordinarily ignored;
the three earlier corpora retain their separate evidence and unchanged
implementation, and the new corpus runs explicitly in both profiles.

Both debug and release match **162 cases / 2,628,564 canonical serialized bytes**
against unchanged original `rcAddSpan`, slope classification, rasterization,
all three filters and `rcBuildCompactHeightfield`. Every area, interval,
cell index/count, compact height, region, connection and grid float bit is
compared. Each case records six intermediate heightfields as well as the
compact result. Independent framing, interval and range checks also pass.

Per profile the corpus covers 6,786 cells, 1,793 supplied spans, 4,632 solid and
1,540 liquid triangles, 32,562 final raw spans and 14,016 compact spans. It includes
all filter masks, merge thresholds, winding, slope thresholds, degenerate and
boundary geometry, liquid-after-filter behavior, high spans, connected layers,
96 deterministic varied meshes and 37 comparisons through the composed `build`
API. Those composed cases use authored Mesh values; they do not acquire real
terrain or execute the original complete TileBuilder.

The authored native wrapper links five original Recast implementation units
from `02245dcd245e7433e524577656177723d3e4992e`, with fast-math and float contraction
disabled. Original source, headers and notices remain in a hashed external
closure. The Rust adaptation preserves the original Mikko Mononen copyright
and Zlib notice and is prominently dated as modified/ported software. No new
Cargo dependency or original C++ implementation is added to the repository.
This local Linux comparison does not establish cross-platform libm parity.

Source, synthetic input and binary hash guards pass. Each profile records
**163 empty network syscall traces** under syscall denial, with compilation
performed offline beforehand. Client assets were not read. Reports remain under
the external laboratory's `reports/acceptance/`:

| Report | SHA-256 |
| --- | --- |
| `recast-heightfield-debug-20261011T074139Z-ebdab885.json` | `96cbd50745e2ea5976764a19ebf501f4b19e2f0ff48e1d10b86bb15745965c43` |
| `recast-heightfield-release-20261011T074142Z-9710e770.json` | `9a21672cda0f9dbec88c3ac992220fe46adfc2aa780798302e45d9c20adbb5b8` |
| `navigation-workspace-20261011T074148Z-f138c553.json` | `ddfacc097aa6acb7e8335996f3c2b2e1ef3c060f21d0cc35c9b0ac2b190225da` |

This initial checkpoint did not include erosion or distance fields; the next
sections record that addition, regions, contours, polygon and detail meshes.
Detour and MMap serialization remain unimplemented. Off-mesh connections are not consumed
by these stages. Actual selected-instance inventory, complete real inputs,
unified product orchestration and usable runtime routes remain separate gates.

## Compact area and distance processing — 2026-10-11

`Compact::prepare(radius)` now composes the next original TileBuilder stages:
`erode(radius)`, `median_filter()` and `build_distance_field()`. These operations
consume their input and return no partial result on failure. `recast::build`
continues to return the initial compact field; callers opt into preparation
explicitly. No default radius, map-specific configuration or border selection
is inferred.

Erosion uses the original saturated 8-bit distance transform: orthogonal cost
2, diagonal cost 3, and rejection below `2 * radius`. Different nonzero areas
do not create erosion boundaries. Radius 0 preserves the areas; repeated calls
operate on the newly null areas. The admitted radius is 0..127: larger values
reject with `recast_invalid_radius` instead of native threshold-byte wrap.
Erosion preserves all compact cells, spans and connections, marking only areas
as zero. It does not remove or rebuild spans.

The median takes nine original area samples, including clockwise diagonals.
Missing or null samples retain the center's area; a null center remains null.
All outputs are computed before any input area is replaced. The distance field
then uses equal-area boundaries, **including equality between null areas**,
16-bit distances and the original two ordered passes. Reversing cell order
does not reverse span order within a column. The final box blur retains values
at most 2 and otherwise uses the original `(sum + 5) / 9` rounding and missing
neighbor substitutions. `maximum()` is the maximum **before** this blur, not
the maximum of the returned blurred array.

`DistanceField` binds the immutable compact graph to its distances. It exposes
borrowed views and a consuming `into_compact()` which discards the distances;
area edits cannot leave a stale distance array attached. Region IDs remain
zero. A prepared graph has no regions until the subsequent explicit
`build_regions` operation described below, and still has no navigation polygons.

### Corrected slope defaults

The pinned `TileBuilder::GetMapSpecificConfig` executes
`m_maxWalkableAngle.value_or(55.0f)` and the same default for the non-steep angle.
Its earlier explanatory comment mentions 70/55; that comment was the source of
the initial Rust default. `Slopes::default()` is now **55/55**. The API still
accepts explicit 70/55, covered by a focused 60-degree triangle test. The older
heightfield corpus now supplies 70/55 explicitly and retains all 162 previously
accepted case arrays. This correction does not claim complete native config or
CLI integration.

### Capacity and executed comparisons

The existing `Limits` follow the graph through all consuming operations.
Cumulative work includes the earlier raster/compact stages and all later
traversals, scratch initialization and a conservative fixed charge for each
nine-value median. Memory admission charges resident compact capacities plus
one byte per span for erosion or median, or two simultaneous u16 arrays for
distance calculation. Actual reserved capacities are rechecked. The final
distance result retains one u16 array; the other is dropped. Input residency,
allocator overhead and CPU-time guarantees remain outside these counters.
There are no new external dependencies or registry downloads.

Sixteen area/distance tests plus the slope-default regression bring navigation
to **81 passing tests** and workspace libraries to **1,665**, with four example
tests, all-target Clippy and scoped formatting. The five ordinarily ignored
corpora retain explicit evidence, with the new compact corpus and the existing
heightfield corpus run in both profiles for this increment.

Both profiles match **164 cases / 65,069,060 canonical serialized bytes** against
the unchanged original compact builder, `rcErodeWalkableArea`,
`rcMedianFilterWalkableArea` and `rcBuildDistanceField`. Per profile:

- 514,526 cells, 539,128 supplied voxel spans and 531,071 compact spans;
- 143 erosion calls, 163 median calls and 164 distance-field builds;
- 634 complete compact snapshots, including each intermediate operation;
- 69 independent calls through the composed `prepare` API;
- exact native results for missing cells, alternating and isolated areas,
  null regions, repeated erosion/median, asymmetric connections, up to 63
  connected layers and 255 isolated layers;
- a 259×257 field exercises u8 erosion saturation, including radius 127;
  a 513×513 field reaches unblurred distance 512, exceeding u8 range.

Every graph field, area, distance and maximum agrees. Independent frame/range
checks verify that area/distance stages preserve the original graph. Both
profiles also repeat the **162-case heightfield corpus**; its case arrays remain
identical to the prior checkpoint. Source/input/binary guards pass and **656
network syscall traces** across both corpora are empty under syscall denial.
The new native closure contains seven unchanged Recast implementation units
with notices, plus the authored wrappers. This is local Linux synthetic
evidence; neither native malformed-input behavior nor real pathfinding is
established. Client data was not read.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-compact-debug-20261011T074723Z-5d2a4afa.json` | `97bc9e98909fce150cc5b50b861cca2ecb0083e2a20ba8ac5d03b0f958ef8c4e` |
| `recast-compact-release-20261011T074758Z-04421618.json` | `46d8eb425d901372e12a671265b4c57814b6b36d9249b2aa92da51c76dcde21c` |
| `recast-heightfield-debug-20261011T074807Z-4afe3489.json` | `3b9e54451b449d67387cbe089f08b2764717a1012baae66e6169c6b74680d038` |
| `recast-heightfield-release-20261011T074810Z-14b59f4e.json` | `fcbc41c5e8bf8e193c30f0f15f62e5a67495432d2648f665cd34dc96d638b5f9` |
| `navigation-workspace-20261011T074811Z-4979f076.json` | `398d792621eaaa79921de8de0e715963769fbe78448d98be43eb54495c8acc22` |

The next sections record original watershed `rcBuildRegions`, contour,
polygon, detail and merge acceptance. Detour/MMap output and usable
runtime acceptance remain pending. No complete TileBuilder
or full extractor is claimed by these supplied-field operations.

## Watershed regions — 2026-10-11

`DistanceField::build_regions(RegionOptions)` now consumes the prepared field
and returns immutable `Regions`, retaining the graph and distance values.
The caller explicitly supplies `border_size`, `minimum_area` and `merge_area`.
The latter two are **span-count thresholds**, already squared if starting from
the original linear configuration sizes (60² and 50²). Values exceeding signed
32-bit native thresholds reject. No map-specific defaults are inferred.

This ports the original watershed algorithm used by TileBuilder, including:

- four ordered border paints with the `0x8000` border bit, clamping painted
  widths/heights while retaining the requested border size;
- eight rotating level buckets, original leftover order and eight expansion
  iterations at positive levels; expansion at zero continues until no progress;
- staged expansion updates and ordered flood traversal, including the native
  distinction between direct border neighbors and diagonal region conflicts;
- connected-component minimum-area filtering, including mixed-area components
  and protection for regions touching a painted tile border;
- boundary adjacency walks, floor/overlap detection, smallest eligible neighbor
  merging, connection replacement and ordered region-ID compression.

The region adjacency walk is an internal neighbor-discovery step; it does **not**
implement the later `rcBuildContours` contour geometry API. Geometry, area IDs,
connections and distances remain unchanged; only per-span region IDs change.

`Regions::maximum()` preserves the native upper ID bound, including empty IDs
that the original remapping retains. It is not a count of populated regions.
For example, an unbordered flat plane can have one populated region with maximum
2; a field whose distance maximum is zero starts no regions and can retain
maximum 1. Border paints can overlap, and the last paint wins.

Overlapping floors retain the native nonfatal outcome, exposed explicitly by
`overlapping_regions()` in native algorithm order. The comparison checks the
public native warning count, including positive and subsequently removed
overlaps. The IDs in that diagnostic vector have focused synthetic internal
tests; the native public API does not expose the vector itself. This diagnostic
does not establish that later polygon construction or routes are valid.

### Strict failures and resource accounting

The field's existing 128 MiB logical capacity and 128-million cumulative work
ceilings continue through region construction. They cannot be raised. All
region buffers, level/expansion/flood stacks, metadata, adjacency/floor vectors,
merge replacements and overlap storage have fallible allocations. Growth
charges old and replacement capacities together, with actual capacities
rechecked. Traversals, remapping, scans, moves and scratch initialization are
charged cumulatively. Caller residency and allocator overhead remain excluded.

The next region counter reaching `0x8000` rejects with `recast_region_id_limit`
before ordinary region IDs collide with the border flag. The original only
checks the later u16 overflow. A boundary walk that has not closed after the
native 39,999-iteration body rejects with `recast_region_contour_incomplete`,
rather than returning the native truncated neighbor list. These are explicit
Rust rejection policies, not native failure-parity claims. Public-input tests
cover 36,864 isolated islands and a 1024×160 serpentine for those boundaries.
Errors consume and discard the whole result.

### Executed region acceptance

Thirteen new tests bring navigation to **94 passing tests**, workspace libraries
to **1,678**, with four example tests, all-target Clippy and scoped formatting.
There are six ordinarily ignored corpora across the workspace. This increment
explicitly runs the new regions, compact-preparation and initial-heightfield
corpora in both profiles; the earlier unchanged stages retain their evidence.

Debug and release each match **191 cases / 13,870,280 canonical serialized bytes**
against original public `rcBuildRegions`, after original compact, optional
erosion/median and distance construction. Per profile the new corpus covers
173,784 cells and 162,781 compact spans; final labels include 114,038 ordinary
region spans and 18,897 border spans. There are two nonzero native overlap
warnings. All fields, region IDs, upper bounds, border sizes and warning counts
agree. Independent framing and range checks also confirm unchanged graph,
area and connection fields across the region operation.

Fixtures include zero-distance fields, holes, islands, annuli, narrow bridges,
mazes, area boundaries, 63-layer fields, clamped overlapping borders, native
thresholds, 90×90 subtiles and a 129×129 multi-level distance field. Three paired
comparisons establish changes from the merge threshold: a bridge changes its
maximum from 3 to 2, and two maze comparisons change 9 to 8. The maximum charged
work in this positive corpus is 20,846,798; it is not a timing or RSS measure.

The **164 compact** and **162 heightfield** case arrays remain exactly identical
to their prior accepted checkpoints in both profiles. All source/input/binary
guards pass, and **1,040 network syscall traces** across these three corpora
are empty under syscall denial. Original Recast sources and their notices stay
external; modified Rust ports preserve the copyright and Zlib notice. There is
no new Cargo dependency, client read, release or complete navmesh claim.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-regions-debug-20261011T075711Z-4cc41624.json` | `ef7ae2c6133c215d0b01873e4ab7ff69e53faad1ac91801231d4b3cdde50ed27` |
| `recast-regions-release-20261011T075739Z-85e0523f.json` | `9de264f91d0b8df8f8f1cc98866028fb3218b792b7fff8da1bb33bbfbff73356` |
| `recast-compact-debug-20261011T075748Z-642b63b8.json` | `52202a68922bdc052c16ace8f5cf1fb94bd95ce7d487e9844aec6ee3c8be96f6` |
| `recast-compact-release-20261011T075757Z-4a0d6adb.json` | `f7d7879b268a7f4c24cc23974d0161c9d1c13ce9db2b4530331b35faf9ed4567` |
| `recast-heightfield-debug-20261011T075801Z-3257693b.json` | `ba2e1f7a0336082ba535f5d24ff086e76dd190f96ebd3b57c3ff8b187f49f35c` |
| `recast-heightfield-release-20261011T075804Z-615a023c.json` | `9a3d0907b856b50a109144fbaa2ecc6d8d8e56d74f9daad2379860661914f2d6` |
| `navigation-workspace-20261011T075806Z-875f5290.json` | `352385d6d8796a786aa5a0ea6f5f1d1f3ec4aa4d326e5ebd36cdd6b0cba08c07` |

The following sections add contour, polygon, detail and merged geometry.
Detour/MMap serialization, complete real acquisition and usable
runtime routes remain required. This component evidence does not finish the
full Rust extractor goal.

## Contour geometry — 2026-10-11

`Regions::build_contours(ContourOptions)` consumes the regions and returns an
immutable `ContourSet`. It retains the original graph, distance values and
regions for later detail-mesh work. The set exposes padded bounds/dimensions,
the selected options and ordered `Contour` records. Each contour exposes its
region, area, raw vertices and simplified vertices as `[x, y, z, flags]`.
Coordinates use integer cells, with x/z border padding removed.

The options explicitly select `maximum_error`, `maximum_edge_length`,
`tessellate_walls` and `tessellate_areas`. Defaults are 1.8, 0, true and false:
zero edge length disables subdivision. They do not select TileBuilder's map or
subtile configuration. The supplied 90×90 cases explicitly choose edge length
81 and border 5. A zero border preserves even the sign bits of zero bounds.

This is the original `rcBuildContours` stage, distinct from region adjacency
discovery. It includes:

- ordered boundary marking and tracing, four-cell corner heights and the
  original border-vertex classification;
- mandatory region/area transition vertices, otherwise lexicographic extrema,
  followed by ordered f32 maximum-deviation insertion;
- optional wall/area edge subdivision with native midpoint rounding and the
  single forward degenerate-vertex cleanup pass;
- neighbor-region bits `0xffff`, border-vertex bit `0x10000` and area-border
  bit `0x20000`, preserving the native source corner for each flag;
- winding classification, ordered hole search, visibility tests and circular
  bridge assembly, including duplicated bridge endpoints.

After joining a hole, its entry remains in the original contour order with
empty simplified vertices. Its raw vertices, region and area remain present.
The outline receives the joined simplified vertices; its raw outline is not
rewritten. Empty and border-only fields can produce an empty set, including
zero padded width/height. No partial contour set is returned on failure.

### Hole compatibility and strict admission

The fixed C++ implementation uses `diags[i].vert` for an intersection skip,
where `i` is the hole ordinal, inside its candidate `j` loop. Rust retains that
index when its scratch slot has been initialized, including previously written
tail slots. A read of an uninitialized slot, or growth beyond the original
diagonal scratch length, rejects with `recast_contour_holes_unresolved`.
The implementation does not invent values for undefined native storage.

Hole and diagonal ties have explicit encounter-order tie-breaks, matching the
executed pinned Linux `qsort` profile. General platform/qsort tie equivalence
is not established. Missing outlines, conflicting outlines when holes are
present, and a hole with no visible bridge also reject rather than returning a
native warning with incomplete geometry. The positive comparison domain is
separate from these strict Rust rejection policies.

Boundary walks that do not close within the native 39,999 body iterations
reject with `recast_contour_incomplete`. Numeric admission requires a finite,
nonnegative error whose square is finite, and edge length at most 46,340 to
avoid native signed-square overflow. Winding accumulation is checked for
signed overflow. Twice the requested border must fit both grid dimensions;
padded bounds must stay finite and ordered.

The inherited 128 MiB logical capacity and 128-million cumulative work caps
continue through this stage. Contour records, raw/simplified vertices, boundary
flags, hole groups, diagonal scratch and old/new joins use fallible growth,
with overlapping capacities charged. Scans, traversals, insertions, sorting
and scratch initialization consume the same work budget. Caller residency and
allocator overhead remain excluded; these limits are not an RSS or CPU timer.

### Executed contour acceptance

Fourteen added tests bring navigation to **108 passing tests**, and workspace
libraries to **1,692**, plus four example tests. All-target Clippy and scoped
formatting pass. There are seven ordinarily ignored corpora in the workspace;
this increment explicitly executes four Recast corpora in both profiles.
Focused negatives cover numeric/border admission, work/capacity ceilings,
missing/conflicting outlines, an unbridgeable hole and winding overflow.

Debug and release each match **155 cases / 5,896,408 canonical serialized bytes**
against original public `rcBuildContours`. Of these, **152** run the complete
synthetic supplied-span pipeline through compact preparation and watershed
regions. Three separately supply one-region labels over synthetic compact
fields, joining **2, 3 and 8 holes**. Those three establish contour behavior for
supplied labels; they do not claim the watershed stage generates those labels.

Each profile covers 135,286 cells, 133,372 compact spans and 633 contour entries,
with 4,970 simplified vertices, 27,528 raw vertices and 19 merged hole entries.
Six merged holes come from generated-region cases and thirteen from the three
supplied-region cases. Comparisons include every raw/simplified coordinate and
flag, record order, bounds, padding, options and the retained compact graph.
Independent checks validate output framing, counts and ranges.

Fixtures include empty and isolated fields, signed-zero origins, border sizes,
all four tessellation flag combinations, three error thresholds, holes, annuli,
bridges, mazes, islands, area boundaries, varying heights, up to 63 layers and
90×90 supplied subtiles. The maximum charged work is 20,952,849 in this positive
corpus. This is not a real-client navigation acceptance run.

The **191 region**, **164 compact** and **162 heightfield** case arrays remain
identical to their previous checkpoints in both profiles. All source, binary
and input guards pass; **1,352 runtime network syscall traces** are empty under
syscall denial. The authored native wrapper links eight unchanged original
Recast implementation units, retained externally with their license. The Rust
ports preserve the Mikko Mononen copyright, Zlib notice and modification date.
No Cargo dependency, input-client read or server activation was needed.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-contours-debug-20261011T080828Z-1bfac5d0.json` | `c55c4d77cd1322d79737921e2c2e1ad007ca850f9889b5f3ef0a932427978a35` |
| `recast-contours-release-20261011T080845Z-de6f0f62.json` | `52f8170daff6e794408d03bdcc26590ff08e27bf02280ee5494c100fb414b0fe` |
| `recast-regions-debug-20261011T080845Z-5ab91566.json` | `ee77cf31fd50aba883ff22c51174e2d5fabd8c45e3ce67022ecdaebc77b0d5c4` |
| `recast-regions-release-20261011T080850Z-ac198f26.json` | `362771702d58f152388c321327d1929670404326954097cd4c6b3126afea143b` |
| `recast-compact-debug-20261011T080845Z-8337e044.json` | `f4db7d93d755e91388e086feceaa38839b1905ed3f86789321e11df6e367b357` |
| `recast-compact-release-20261011T080855Z-3e2fb2ff.json` | `991db357dedb97c8812a4fff54f8ffc181a43d432851c9465791ce72e7af3427` |
| `recast-heightfield-debug-20261011T080845Z-356c390c.json` | `70de725b901645e44a238baa8fcaf840c3a79d61d043875a33955decd66bde81` |
| `recast-heightfield-release-20261011T080849Z-2497e4b9.json` | `3c065de2340b52c044b0e0d19c1428a81feec49915931719f77ce311bc02c205` |
| `navigation-workspace-20261011T080933Z-3c51fb29.json` | `239e7b2de038b5115e19b718492548881495707d5940a76d0848f07017166626` |

The following sections add polygon, detail and merged meshes.
Detour/MMap serialization, complete real inputs and usable routing remain
pending. Full extraction remains an active goal.

## Polygon mesh — 2026-10-11

`ContourSet::build_mesh(vertices_per_polygon)` consumes the set and returns
`PolyMesh`, retaining the original contours, regions and compact graph for
later detail generation. The explicit polygon width is **3..=6**; the pinned
TileBuilder selects the original `DT_VERTS_PER_POLYGON` value of 6. This is an
admitted Detour-compatible domain, not a generic arbitrary-width Recast API.

The mesh exposes integer u16 vertices, ordered polygons, the selected width and
native `maxpolys` metadata (the sum of nonempty contour vertex counts minus two
per contour). This metadata is separate from Rust allocation capacity. Each
polygon exposes vertex indices, neighbors, region, area and flags. Flags are
zero, matching the original polygon stage; later navigation flag assignment
is not implemented here.

The port includes:

- ordered shortest-diagonal ear clipping, strict and loose cone/intersection
  tests, and the original loose-fallback scoring of `next(i2)`;
- original 4,096-bucket x/z hashing, accepting a matching y difference of at
  most two units and preferring the most recently inserted matching entry;
- triangle filtering by original contour indices, followed by longest-edge
  convex polygon merging and original swap-with-last ordering;
- removal of marked border vertices when the native topology checks allow it,
  ordered boundary reconstruction, retriangulation and polygon replacement;
- native mixed-region value 0 during boundary repair, retaining the first
  triangle/polygon area according to original order, including mixed-area cases;
- the original linked edge adjacency algorithm and ordered boundary portal
  selection after connected edges have been assigned.

Neighbor `0xffff` means no connection. Ordinary neighbors are polygon indices;
`0x8000 | direction` marks portals: x−, z+, x+, z− use directions 0, 1, 2, 3.
Unused slots remain `0xffff`. Bounds, cell sizes, border size and maximum edge
error remain the contour header values, including float sign bits.

### Polygon admission and resource policy

The native total simplified-vertex limit is preserved: counts at or above
65,534 reject before triangulation. Rust additionally limits polygon indices
to below `0x8000`, admitting at most 32,768 polygons, and edge-list indices to
below `0xffff`, admitting at most 65,535 edges. This prevents collision with
portal and null markers before the original u16 truncations can corrupt links.
Contour x/z coordinates must lie in 0..=1024 and y in 0..=65535; no coordinate
wrap is admitted.

An unfinished triangulation returns `recast_triangulation_incomplete` rather
than a mesh containing the original partial triangles. A disconnected removal
boundary returns `recast_mesh_vertex_removal_incomplete`; exceeding the native
replacement-polygon capacity also rejects instead of silently dropping output.
No partial mesh is returned after an error. These strict rejections are not
native failure-parity claims.

The same 128 MiB logical heap and 128-million cumulative work ceilings include
retained inputs, mesh storage, hash chains, ear indices/triangles, merge arrays,
removal boundaries and adjacency scratch. Fallible growth charges old and new
capacities together. Traversals, moves, initialization and candidate searches
consume the inherited work counter. Caller residency, allocator overhead and
fixed stack values remain outside logical heap accounting.

Native complete output is not necessarily valid walkable geometry for arbitrary
supplied contours: the synthetic crossing-outline case also produces native
triangles. Matching its bytes does not establish a usable pathfinding mesh.
Real acquisition, input validity and runtime routing remain separate gates.

### Executed polygon acceptance

Seventeen added tests bring navigation to **125 passing tests**, workspace
libraries to **1,709**, plus four examples. All-target Clippy and scoped
formatting pass. There are eight ordinarily ignored workspace corpora; five
Recast corpora are executed explicitly in both profiles during this closeout.
Tests cover a public supplied-span pipeline through polygon construction,
welding boundaries, portals, reciprocal adjacency, marked-vertex handling,
numeric/index/resource limits, reversed contours and disconnected repair.

Debug and release each compare **656 supplied-contour cases** with original
public `rcBuildPolyMesh`:

- **652 cases / 688,088 bytes match exactly**, including 16,258 vertices, 9,952
  polygons, 1,442 portal edges, 18,530 linked edges and 79 mixed-region polygons.
- **Four reversed-outline cases** produce native partial-triangulation warnings
  and strict Rust rejection. Their native partial meshes are retained as
  diagnostic evidence and are excluded from exact-output totals.

The corpus reuses all **155 previously accepted contour outputs** at widths
3, 4, 5 and 6. Nine additional synthetic shapes cover straight border removal,
an unremovable triangle tip, mixed region/area boundaries, an interior marked
vertex, y-welding at two/three units, reversed winding and crossing outlines.
The comparison checks all vertex slots, neighbors, regions, areas, zero flags,
counts and header fields. Independent framing/range checks and reciprocal
checks pass for every positive linked edge.

These are polygon-stage comparisons over supplied contours. They do not
re-execute the entire native TileBuilder pipeline in one invocation. The
separate **155 contour**, **191 region**, **164 compact** and **162 heightfield**
regressions retain identical case arrays in both profiles. The corpus resource
counter includes its synthetic empty-field harness baseline, not the original
source field's full pipeline work. The public composition test separately
checks retention of the real in-memory compact graph across the Rust stages.

All source/input/binary guards pass and **2,666 runtime network syscall traces**
are empty under denial. The native wrapper links nine unchanged original
Recast units, including `RecastMesh.cpp`, retained externally with notices.
The Rust adaptations preserve the Zlib notice, original copyright, modification
date and the original edge-algorithm attribution. There is no new dependency,
real-client read, publication or server activation.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-mesh-debug-20261011T081838Z-8f1906d9.json` | `e104beee5795a8c687adb157b7d74d25399f785a4f17e1c44d59836333aa1a58` |
| `recast-mesh-release-20261011T081857Z-dea1074b.json` | `b0758d3647c54e5207a32b50430e06548ee45f98ebfc0e1db380616016c330ca` |
| `recast-contours-debug-20261011T081838Z-b0fbf613.json` | `e784d96b491bf538f0a6084e5f27a3a379f80c6bd6913aa895ef64e4b885b552` |
| `recast-contours-release-20261011T081843Z-2e5ff952.json` | `5f50fd96c515b24ae73131fc57ff2b7b6326b1b007ce283117e002210febcfd6` |
| `recast-regions-debug-20261011T081838Z-c8004770.json` | `5996955eb4e39281348f727026ad65e62a0d81ab968ef85c7f0255fc24711f28` |
| `recast-regions-release-20261011T081844Z-aeb1c482.json` | `e68323118be17908e1f036f38664218c85d3feb9643579f935d7d0471209fca3` |
| `recast-compact-debug-20261011T081838Z-528e6028.json` | `f44abbed647056e67237ddabc2cfcc715ff53dedd77788549f24d10cd86a1489` |
| `recast-compact-release-20261011T081849Z-c3fb8c07.json` | `e43d3f1d98186316b9c8395686759537408a43e745954a5f12f2218b08af172a` |
| `recast-heightfield-debug-20261011T081838Z-bdf11b8c.json` | `865d015b0cca4d1ebb4e04c1d6061c4586736f67c17312da0c4d26a0f83f756d` |
| `recast-heightfield-release-20261011T081842Z-2c6ca37c.json` | `a603931b686e0bf94a8ca568b5abc7c366494dace3501a02efc690240ecbe46f` |
| `navigation-workspace-20261011T081958Z-b0d35bfa.json` | `8cc9671ccd732ed1e8bd8d3ed3733953760787ac0949c978f5a8e0e4b926a1d0` |

The following sections add detail meshes and merging. Detour/MMap
serialization, complete real inputs and usable routing remain required.
Polygon construction does not complete the full Rust extractor goal.

## Detail mesh — 2026-10-11

`PolyMesh::build_detail(DetailOptions)` consumes and retains the polygon mesh,
including its compact graph. It returns `DetailMesh` with one `DetailSubmesh`
per polygon, ordered world-coordinate f32 vertices and triangles containing
three local u8 indices plus edge flags in bits 0, 2 and 4. Submesh bases/counts
are u32. The selected options and cumulative recovery warning count are exposed.

Options are explicit world-unit values. `sample_distance` must be positive and
finite; `maximum_error` must be nonnegative and finite with a finite square.
The pinned TileBuilder normally selects `cell_size * 16` and `cell_height`,
respectively. This API does not select product configuration or map overrides.

The port preserves:

- polygon height-patch bounds, border offsets, same-region height assignment
  and boundary-seeded connected flood fill;
- nearest-vertex selection, preferred-direction center walk and connected
  fill for mixed or missing region labels;
- first-valid spiral-ring height selection, keeping encounter order on ties;
- lexicographic edge orientation, up to 31 sample segments and ordered 3D
  deviation simplification;
- shortest-perimeter initial hull ears and original left/right tie decisions;
- interior sample order, original squared-distance versus linear half-spacing
  comparison, deterministic wrapping jitter and maximum-error selection;
- original Delaunay edge/facet order and dangling-face swap removal;
- native boundary flags and world conversion, including the **extra cell-height
  y offset** applied to detail vertices.

`discarded_faces()` counts native dangling-face recovery warnings across all
interior rebuilds, including triangles discarded before the final rebuild.
The recovery retains the same output order as original Recast. Matching that
output does not establish manifold geometry or usable routes.

### Detail admission and resource policy

Each detail polygon admits at most **127 vertices and 255 triangles**. Native
final triangle truncation is rejected. Sample distance zero is excluded because
the original code leaves the hull uninitialized in that domain. Numeric casts,
world coordinates and sampling ratios must be representable. The height-search
radius derived from contour error is at most 23,169 to keep the original square
loop domain within signed integer range.

Missing height seeds, unreachable center walks and the unset height marker
`65535` return `recast_detail_height_unavailable`. Empty or invalid final
triangulation returns `recast_detail_triangulation_incomplete`; native capacity
exhaustion returns `recast_detail_native_capacity`. These checked failures do
not claim equivalence with native undefined behavior, assertions or partial
output. No partial `DetailMesh` escapes an error.

The inherited 128 MiB logical heap and 128-million cumulative work policy
includes retained inputs, height patches, flood/center queues, samples, edge
and face records, triangulation scratch and results. Fallible vector growth
charges old/new overlap; sample grids charge their full iteration domain
before iteration. Caller residency, fixed stack arrays and allocator overhead
remain outside the logical heap count. The corpus maximum is 21,495,115 work
units for a complete supplied-span pipeline, within the per-case ceiling.

### Executed detail acceptance

Sixteen new tests bring navigation to **141 passing tests**, workspace libraries
to **1,725**, plus four examples. All-target Clippy and scoped formatting pass.
Nine workspace corpora are ordinarily ignored; the six Recast corpora are run
explicitly in debug and release. Tests cover world/y offsets, empty meshes,
rough sampling, mixed/missing region seeds, height-ring ordering, disconnected
center walks, missing heights, numeric admission, native recovery order and
inherited resource limits.

Both profiles match **149 cases / 6,477,240 bytes** against original public
`rcBuildPolyMeshDetail`, following the original supplied-span pipeline through
compact preparation, watershed regions, contours and polygon construction:

- 157,839 compact spans and 613 polygon/detail submeshes;
- 6,637 detail vertices and 6,736 detail triangles;
- 4,435 vertices added beyond the original polygon corners;
- up to 127 vertices and 216 triangles in an individual detail submesh;
- one dense case with **six native dangling-face warnings**, identical recovered
  triangles, coordinates, flags and exposed warning count.

Cases include varied cell sizes/origins, borders, polygon widths 3 and 6,
ramps, stairs, waves, holes, area boundaries, islands, narrow shapes and up to
63 compact layers. Twelve 90×90 supplied subtiles use erosion/median filtering
and target-like thresholds. Eight cases **explicitly override polygon region
metadata** with zero or an absent label to exercise center seeding; they do
not establish watershed generation of those labels. Signed-zero origins,
large origins, empty/padded cases and dense zero-error refinement are included.
These supplied spans do not establish complete rasterization/TileBuilder or
actual source acquisition. Independent framing/range checks validate bases,
counts, local indices, flags and original-corner world coordinate bits.

The earlier **656 polygon, 155 contour, 191 region, 164 compact and 162
heightfield** case arrays remain identical in both profiles, including the
four earlier strict polygon rejections. All source/input/binary guards pass;
**2,966 runtime network traces** are empty under syscall denial. The native
wrapper links ten unchanged original Recast units and uses the original
zero-initializing `rcAllocPolyMeshDetail` allocator. Source closure, license
and notices remain external; the five Rust algorithm files retain the original
Zlib notice and dated modification headers. No dependency or integration
repository change, real-client read, publication or server activation occurs.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-detail-debug-20261011T084021Z-fbc48f2f.json` | `cfd784f29d8f1a53d049768b40191178943d4654232e49afbe9d76ef70535daf` |
| `recast-detail-release-20261011T084039Z-64efb098.json` | `c5a87baeb6c9db36b598efa84876489f7c9c83ce81fea3ab5c70ad92960a4a0b` |
| `navigation-workspace-20261011T084039Z-f2a55e65.json` | `2939411bdc10775355e80304c2c4289266e17213535e39dc4d23575735d06003` |

The following section adds merging. Exact Detour/MMap serialization, complete
real inputs, product configuration/orchestration and usable runtime routing
remain required. The full Rust extractor goal remains active.

## Subtile merging — 2026-10-11

`DetailMesh::into_tile_mesh()` moves polygon/detail arrays into a `TileMesh`
and releases the retained compact graph, distance/region data and contours.
The geometry remains paired and immutable; no vertex/triangle copy occurs.
This matches the original TileBuilder's lifetime boundary before assembly.
The original source-stage work and recovery count remain available.

`TileMesh::merge(Vec<TileMesh>, MergeLimits)` consumes an ordered batch and
implements original `rcMergePolyMeshes` plus `rcMergePolyMeshDetails`. It
returns paired merged geometry for the later Detour stage. It preserves:

- source order, first-source cell sizes/polygon width and aggregate bounds,
  including native right-operand selection on equal signed-zero bounds;
- native x/z offset rounding: `floorf((source_min - merged_min) / cs + 0.5f)`;
- 4,096-bucket vertex hashing, latest matching entry and inclusive two-unit
  height tolerance, with the first retained vertex's actual coordinates;
- polygon order and unchanged region, area and flag metadata;
- source portals only on the aggregate exterior, followed by original linked
  adjacency construction over the welded indices;
- ordered detail descriptors with rebased vertex/triangle bases, while detail
  world-coordinate bits, local triangle indices and flags remain unchanged.

The native merger leaves `maxpolys`, `borderSize` and `maxEdgeError` at their
zero-initialized values. `MeshHeader` preserves these values even when the
result has polygons and exterior portals. Native `maxpolys` is not allocation
capacity. Empty supplied meshes participate in bounds; an empty batch returns
the native zero header and empty arrays. Repeated merging is supported for
meshes with a common admitted header contract.

### Merge admission and resources

Inputs must have the same cell size, cell height, vertical origin and polygon
width; mismatches return `recast_merge_incompatible_meshes`. The original
merger does not adjust vertex y for a changed origin, so that mismatch is not
admitted. x/z offsets and shifted coordinates must fit u16 without wrapping.
At most 65,534 unique vertices, 32,768 polygons and 65,535 adjacency edge
records are admitted, preserving the existing null/portal index boundaries.
Malformed or undefined native-domain failure parity is not claimed.

`MergeLimits` is a separate downward-only assembly policy: at most **4,096
input meshes, 128 MiB logical heap and 128-million merge work units**. It counts
the consumed input Vec capacity, each input's geometry capacities, all scratch
and output capacities, and old/new allocation overlap. Fixed stack values,
allocator overhead and unrelated caller residency remain excluded. Source
fields are already released, and the temporary vertex remap is dropped before
detail output growth. Allocation failure exposes no partial result.

`merge_work()` reports the work of this assembly. `processing_work()` sums all
source-stage work and this assembly; it may exceed one stage's work ceiling.
`discarded_faces()` sums the existing detail recovery diagnostics, without
claiming that merging emitted those warnings. The product coordinator still
needs a complete tile/run policy; a single subtile's work allowance is not a
whole-tile budget. Corpus assembly work peaks at **81,968 units**.

### Executed merge acceptance

Eighteen new tests bring navigation to **159 passing tests**, workspace
libraries to **1,743**, plus four example tests. All-target Clippy and scoped
formatting pass. Ten workspace corpora are ordinarily ignored; all seven Recast
corpora are run explicitly in debug and release. Tests exercise ownership
transfer/released storage, zero headers, adjacent/gapped grids, y-welding and
encounter order, detail bases, metadata, signed zeros, empty-mesh bounds,
configuration mismatches, rounding/overflow, memory/work limits, marker
collisions, source provenance and repeated merging.

Each profile matches **330 cases / 1,665,448 bytes** against both original
public merge APIs:

- 3,319 supplied meshes and 18,140 input polygon vertices;
- 8,504 welded vertices and 5,635 polygons/detail submeshes;
- 43,465 detail vertices and 38,820 triangles;
- 1,700 exterior portal edges and 15,758 linked edges, all reciprocal;
- 30 carried source detail recovery warnings and **zero native merge warnings**.

All **149 prior detail outputs** are reused individually and in translated 2×2
groups. Additional authored shapes exercise offset rounding, height chains,
metadata, signed zeros and empty/gapped inputs. Four authored **625-square
grids** use the target subtile widths 40 and 80, their respective cell sizes,
and forward/reversed input order. Each yields 676 welded vertices, 625 polygons,
2,500 detail vertices, 1,250 detail triangles, 100 outer portals and 2,400
reciprocal links. Changing source order preserves that order's native output;
different orders are not asserted to produce identical serialized bytes.

These are supplied-mesh merge comparisons. The authored grids do not execute
625 original rasterization/configuration pipelines or establish real terrain
acquisition. Independent checks validate framing, ranges, shifted/welded
positions, preserved polygon metadata, reciprocal links and exact concatenated
detail arrays/descriptors. Repeated/overlapping supplied geometry matching
native behavior is not a safe-routing claim.

The prior **149 detail, 656 polygon, 155 contour, 191 region, 164 compact and
162 heightfield** case arrays remain unchanged in both profiles. Source,
input, closure and binary hash guards pass; **3,628 runtime network traces**
are empty under syscall denial. Ten original Recast source units remain
unchanged in the external native closure. The Rust merge algorithm retains
the Zlib notice and dated modification header. No dependency/integration
repository change, client read, publication or server activation occurs.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `recast-merge-debug-20261011T084758Z-8893aef1.json` | `c5e3cd202f93bf271f4b4f55f02c79e9d5bde792b8e338474d31d8acf57eb817` |
| `recast-merge-release-20261011T084838Z-67704d55.json` | `951df24bb5c436657edc59799647eeafbb15b2dcb0c9faddcd201d5fc169abd0` |
| `navigation-workspace-20261011T084838Z-9d7beac1.json` | `1838daa5ef95b50ee99eea16eb36fe55790bcd3495c907095f6e57799a8b4b3a` |

Detour payload evidence follows. MMap 16, complete TileBuilder/configuration,
actual source acquisition, product orchestration and usable real runtime routing
remain required. The full Rust extractor goal remains active.

## Detour tile payloads — 2026-10-11

`detour::build(&TileMesh, &[offmesh::Connection], Options, Limits)` borrows
paired polygon/detail geometry and returns immutable bytes plus structured
statistics. Options explicitly supply tile x/y/layer, user ID, world bounds,
agent dimensions, BV-tree selection and `FlagPolicy::{Preserve, Forever}`.
The source supplies cell sizes and detail geometry. There is no implicit map
configuration, MMap container, acquisition or filesystem operation.

### Wire and native behavior

The writer encodes little-endian **Detour 7 with 64-bit references** explicitly,
independently of Rust struct layout:

| Structure | Bytes |
| --- | ---: |
| Mesh header | 100 |
| Polygon, at most six vertices | 32 |
| Link | 16 |
| Detail descriptor | 12 |
| BV node | 16 |
| Off-mesh connection | 36 |

Unused link storage, padding and the final reserved BV node are zero. The
header reserves twice the ground-polygon count for BV nodes; the tree fills
one fewer. Polygon vertices use the explicit world origin and source cell
sizes. Original detail corners are replaced with polygon vertices; remaining
detail vertices and all triangle bytes follow native storage order. Recast
portals map to Detour sides 4/2/0/6; internal neighbors become one-based.

Preserve mode keeps flags and masks areas to six bits. Forever mode preserves
flags for area zero, maps areas 1..7 to flag 1 and areas 8..11 to the original
terrain bits. Areas 12..63 reject because the original signed shift is
undefined. This admits the defined target projection without assigning a
meaning to undefined native behavior.

Off-mesh classification uses half-open horizontal bounds, eight external side
values and inside value 255. Starts outside the tight source-detail height
range plus/minus walkable climb are culled; end height is not culled. A culled
start with an inside endpoint still contributes native link allocation.
Stored connections retain order, direction, flags and masked area, with zero
user IDs. Zero and negative-zero radii are retained; negative radii reject.

BV bounds use all source detail vertices, reciprocal cell-size quantization
on every axis, truncation and u16 clamping. Nonfinite or unrepresentable native
float-to-int conversions reject. Longest-axis ties and recursive partitioning
follow the original. The bounded stable sort reproduces equal-key order for
the current Linux/glibc native closure; Windows or arbitrary qsort tie behavior
is not established.

Two defined native storage defects remain visible in `Statistics`:

- `detail_base_wraps` counts the original u16 detail cursor wrapping. A final
  unused wrap can be benign; subsequent copies can overwrite earlier vertices
  and leave a zero tail. The byte writer preserves that behavior.
- `neighbor_marker_collisions` counts one-based neighbor values overlapping
  the external-link marker, including native neighbor 32767 becoming 0x8000.

The diagnostic corpus matches those bytes and loads them, but performs no route
probe on those cases. Loading is not proof of valid geometry or safe routing;
product admission still needs to account for these diagnostics.

### Detour resource and admission boundary

Downward-only limits cap input connections at **65,536**, output at **64 MiB**,
owned heap at **128 MiB** and work at **128 million units**. Total stored vertex
and polygon counts are capped at 65,536 to preserve u16 indices. Class buffers,
output and BV items/sort scratch are charged; borrowed mesh residency, fixed
stack values and allocator overhead are separate. The corpus peaks at
16,154,856 work units. Allocation/numeric/policy failures expose no partial data.

Bounds, agent dimensions and emitted coordinates must be finite, agent
dimensions nonnegative, and layer nonnegative. Empty geometry rejects.
The optional native branch without a supplied detail mesh is outside this API;
`TileMesh` always pairs polygon and detail arrays. No general malformed-input
or undefined-domain native rejection equivalence is claimed.

### Executed Detour acceptance

Nineteen new focused tests bring navigation to **178 passing tests** and
workspace libraries to **1,762**, plus four examples. Clippy, scoped formatting
and whitespace checks pass. Eleven workspace corpora are ordinarily ignored;
this stage and the seven preceding Recast stages ran explicitly in both profiles.

Debug and release each compare **768 cases**: **728 exact positive payloads /
18,533,660 bytes**, and **40 matching empty-input rejections**. All 330 preceding
merged outputs are reused with BV on/off. Additional cases cover all areas,
connection sides and directions, boundary/height culling, explicit origins and
metadata, signed zero, exact maximum off-mesh indices, detail cursor wrapping
and the neighbor marker collision. Independent parsing checks block framing,
coordinate bits, zero blocks, BV bounds/escape indices and off-mesh records.

The unchanged original `dtCreateNavMeshData` produces each reference payload.
Original `dtNavMesh` separately loads both payloads and validates link chains.
Original nearest-poly, path and straight-path queries run on **149 cases**:
**147 complete paths and two expected partial paths**. Eight 625-square grid
variants have 49-polygon complete paths; forward order yields two straight-path
points and reversed order three. Directed gap cases accept forward-only and
bidirectional traversal; reverse one-way and disconnected gaps remain partial.

An external runner uses the unchanged actual RustyCore `wow-recastdetour` crate
at integration commit `544721324cf0a47553caae1ea0299a0ff2917ce7`. Public
`DetourNavMesh::{add_tile, remove_tile}` accepts all **728 payloads**. Public
nearest-poly, path and straight-path APIs reproduce the **149 query results**,
including exact refs, coordinate bits, paths, straight-path flags and destination
reachability. The wrapper drops successful status-detail bits, so those bits
are not compared.

This runner supplies a `MmapTileBlob` with truthful version **16** metadata,
but `add_tile` reads only its `data`. It therefore proves raw Detour payload
acceptance, **not** the existing legacy MMap 15 parser, Forever MMap 16/global
header loading, multi-tile world routing or `PathGenerator` policy.

All prior **330/149/656/155/191/164/162** case arrays remain identical in debug
and release. Source/input/binary/native dependency guards pass; **5,894 runtime
network traces** are empty under syscall denial, including 728 actual RustyCore
calls. Seven original Detour implementation units remain external and unchanged.
The Rust algorithms preserve the Zlib notices and dated modification headers;
the flag projection also retains the TrinityCore GPL notice. Dependencies and
the integration repository are unchanged. No client read or publication occurs.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `detour-debug-20261011T090513Z-ddf96768.json` | `f6c8916fa821706c040ab49170a3b568c984ca6451b95f8a6b3bcf90f262f4b1` |
| `detour-release-20261011T090513Z-4c33151b.json` | `1d419336b79688edcbbb55deefb129c8a88e6db7b0f4aa16f74c6deca3c08ec8` |
| `detour-rustycore-20261011T090539Z-732237ab.json` | `372e8e14f5fe9279b3da82c43048638aac61d77f51c191ca199d37af8af9d960` |
| `navigation-workspace-20261011T090513Z-32e85caf.json` | `acb8d94abea6f3311dc5930232f01dd4fdbce3458c95482c2f7165fcfc6b8c6a` |

The following section adds the versioned MMap 16 container. Complete TileBuilder
configuration, real geometry acquisition, product orchestration and high-level
world/runtime acceptance remain required; the full extractor goal is active.

## MMap 16 containers — 2026-10-11

`mmap::encode_global(map_id, Params, &[offmesh::Source], Limits)` writes the
Forever global file. `mmap::encode_tile(&detour::Data, uses_liquids, Limits)`
wraps the paired builder's unchanged payload. Both return owned byte vectors
without filesystem effects. The explicit liquid flag describes the configured
producer option; it is not inferred from whether the tile contains liquid.

The format is little endian and deliberately separate from legacy MMap 15:

| Record | Layout |
| --- | --- |
| Global header, 40 bytes | magic, MMap 16, origin xyz, tile width/height, signed maximum tiles/polys, off-mesh count |
| Off-mesh record, 44 bytes | u32 map/row/column, from xyz, to xyz, radius, u8 connection flags/area, u16 terrain flags |
| Tile header, 20 bytes | magic, Detour 7, MMap 16, payload byte count, canonical liquid byte, three zero padding bytes |

Global writing selects records by map ID, retaining encounter order and
duplicates. Coordinates stay in their original source axes. Every raw float
bit, connection/area byte and flag bit survives; signed zero, nonfinite values,
negative radii and out-of-grid record coordinates are not silently rewritten.
Those raw records are metadata, not accepted runtime connections. The existing
`offmesh::select` and Detour builder impose their own selection/numeric rules.

`Params` requires finite origins, positive finite tile dimensions, maximum
tiles in 1..2^21, and a positive maximum-poly value or **i32::MIN**. That last
value preserves the original fixed-64-bit `MapBuilder` expression `1 << 31`;
the fixed-bit Detour initializer does not derive reference widths from it.
Explicit params are not a recreation of original parent-chain/tile-inventory
selection. Admission of a tile capacity does not reserve or authorize runtime
memory allocation.

### Strict borrowed readers and naming

`GlobalView::parse(bytes, map_id, Limits)` requires the complete global header
and exact declared record array. All records must belong to the supplied map
ID. `params()` exposes checked parameters, and `sources()` decodes an exact-size
iterator of typed sources without allocating. The whole original byte slice
remains available. The global header itself contains no map ID, so an empty
file cannot authenticate its external name or supplied map identity.

`TileView::parse(bytes, Limits)` requires the exact complete tile length, MMap
16 and Detour 7, a 0/1 liquid byte and zero padding. It also checks the inner
Detour header/magic/version and every declared array's block length for the
accepted 64-bit ABI. Negative counts, zero polygon/vertex counts and inconsistent
lengths reject. The view exposes the original payload and liquid option.

These are framing checks. They do **not** establish valid polygon/detail
indices, BV traversal, adjacency, arbitrary-input FFI safety, geometric
soundness or source provenance. Prior diagnostic payloads retain their
documented native wrapping/marker defects. A runtime integration must not use
container parsing as its sole safety proof for untrusted Detour bytes.

Readers reject every truncation, trailing bytes, incompatible versions and
noncanonical headers rather than accepting prefixes. Their stricter admission
is not a claim of native malformed-input rejection parity: original loading
may ignore trailing data, padding or optional off-mesh arrays.

`global_path(map_id)` produces `mmaps/{map:04}.mmap`; `tile_path` produces
**`mmaps/{map:04}_{row:02}_{column:02}.mmtile`**, with row/column restricted to
0..63. Widths are minimum padding; larger u32 map IDs are retained. The
underscore-separated Forever names differ from the legacy concatenated names.
The helper performs no lookup, directory creation, parent fallback or publishing.

Limits only reduce **65,536 supplied records** and **64 MiB + 20 bytes** per
file/owned output capacity. Global input limits include unselected records,
bounding filter work. The tile ceiling includes its 20-byte header. Writers
check capacity and use fallible allocation; readers borrow without allocation.
Caller input residency, allocator overhead, coordinated multi-file budgets and
runtime allocations remain separate. No partial output escapes a failure.

### Executed container acceptance

Sixteen new tests bring navigation to **194 passing tests** and workspace
libraries to **1,778**, plus four example tests. Twelve corpora are ordinarily
ignored; the MMap and previous Detour corpora ran explicitly in both profiles.
Tests include exact layouts, filtering/duplicates, raw record bits, connection
selection, every truncation of representative files, trailing bytes, version
and parameter errors, record limits, malformed payload counts, padding, names
and a public supplied-span Recast → Detour → MMap composition.

Debug/release each match:

- **762 global files / 3,054,248 bytes / 68,722 selected off-mesh records**;
- **728 tile files / 18,548,220 bytes**, reusing every previous positive Detour
  payload with alternating liquid options;
- 34 additional global-only cases, including the full 65,536-record ceiling,
  varied origins/capacities, and explicit raw nonfinite/negative-radius bits.

Independent wire calculations check every emitted byte. The native wrapper
uses the **unchanged original `MMapDefines.h` structs**, static size/offset
assertions and original Detour implementation. It initializes a native mesh
from each serialized/read-back global, then loads/removes all 728 tile payloads.
This validates the native ABI and Detour use of the decoded values. It does
not execute the full original `MapBuilder`, `TileBuilder::saveMoveMapTileToFile`
or `MMapManager`; their writing/reading source is retained in the audited closure.

An external integration runner first calls the **new public GlobalView and
TileView readers**, then passes their decoded parameters and unchanged payloads
to actual RustyCore `wow-recastdetour` at commit
`544721324cf0a47553caae1ea0299a0ff2917ce7`. All **728 loads/removals** and
**149 nearest/path/straight projections** agree with the previous original
Detour oracle: **147 complete and two expected partial paths**. Exact refs,
coordinate bits, paths and straight-path flags remain unchanged. Successful
status-detail bits are still outside the RustyCore wrapper comparison.

The runner additionally asserts that RustyCore's **existing legacy tile parser
rejects MMap 16**. The new reader lives in `forever-navigation`; this acceptance
does not edit/install the integration repository's manager or bootstrap, or
relabel its legacy contract. Parent/global-parameter derivation, multi-tile world
queries, inventory, high-level path policy and real routes remain separate gates.

The earlier **768-case Detour arrays** remain identical in both profiles,
including the 40 empty rejections and explicit native diagnostics. All source,
input, binary and dependency guards pass. **3,792 runtime network traces** are
empty under syscall denial across MMap, Detour regression and typed integration.
Clippy, scoped formatting and whitespace checks pass. No dependency change,
client read, publication, integration edit or server activation occurs.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `mmap-debug-20261011T091417Z-a48ea1df.json` | `5752e43161da361dbb2595ad877529152535ae5e1e4a660cd6ee28a28a6d087d` |
| `mmap-release-20261011T091423Z-505e8135.json` | `843aeac186bccd30a8a4d933c920ca42c0cba6d657456e1538115a8e7c83c0a8` |
| `mmap-rustycore-20261011T091521Z-ff073b83.json` | `5638fee52ffcbaf6874a70a3359fd6dd8a39026b59e48c04de8375dbcfa6f267` |
| `navigation-workspace-20261011T091424Z-6c91660a.json` | `c28c82f9751a3600e7b9b4827992e640bcaddd1f4ef7d133415ab71fc0279d39` |

The following section adds supplied TileBuilder configuration/computation and
explicit inventory-based global parameters. Real source acquisition, final
producer-file bytes, coordinated output and high-level runtime acceptance remain
required; the full extractor goal stays active.

## Complete supplied TileBuilder — 2026-10-11

`tilebuilder::build(&Mesh, &[offmesh::Connection], Request, mmap::Params, Limits)`
now composes the original target's complete supplied-geometry computation. It
returns a pristine Detour payload with statistics, an explicit empty result, or
an error identifying the failing stage/subtile. It performs no acquisition,
instance selection, file publication or implicit parent discovery.

`Config::new` preserves the small/big base units 0.2666666/0.5333333, tile widths
80/40, and all **25×25** subtiles. Slope overrides remain explicit, defaulting
to 55/55 degrees. The original fixed thresholds, filtering order, radius,
border, contour tessellation, polygon width six and detail sampling are retained.
Two map exceptions preserve their original assignment order:

- Map 562 sets erosion radius to zero after calculating the ordinary border.
- Map 48 doubles cell height after setting the detail maximum error. Detour's
  world agent dimensions still multiply the original base unit, not the changed ch.

Bounds use filename row/column axes and scan liquid vertices before solids for
height, including unreferenced vertices. Equal signed-zero extrema select the
right operand, matching the reference. Empty geometry retains FLT_MIN/FLT_MAX
height bounds. Nonfinite source coordinates and invalid slope/coordinate domains
reject. Horizontal grid arithmetic keeps the original f32 operation order and
y-major/x-minor subtile traversal, with padding applied after each world bound.

`global_params(&[Coordinate])` computes bounds/maxima, tile count, GRID_SIZE
dimensions and the i32::MIN maximum-poly sentinel from a complete **caller-selected
ancestor inventory**. It admits 1..4096 unique grid positions and rejects
duplicates/out-of-range coordinates. This API does not select that ancestor or
authenticate inventory completeness. Configured Detour tile indices truncate
the bounds center relative to the supplied origin using the fixed GRID_SIZE.

The consuming pipeline rasterizes solids, applies all three filters, rasterizes
liquids, compacts, erodes, median-filters, builds distance/regions/contours,
polygons and detail, releases intermediate fields, merges all 625 meshes and
builds Detour. It retains empty subtiles in the merge. An error aborts the whole
operation, unlike the original's continue-on-failed-subtile behavior. Native
detail cursor/neighbor marker diagnostics reject at the final payload boundary;
detail recovery counts remain visible.

Whole-operation downward limits are **512 MiB logical owned capacity** and
**16 billion work units**. Each underlying stage retains its smaller ceilings.
Already retained subtile geometry and its vector are charged while admitting
the next stage; merge accounts for the consumed vectors itself. Detour admission
charges retained merged geometry plus its own output/scratch. Borrowed caller
mesh/connections and allocator overhead remain outside this policy. Statistics
report cumulative stage work and a conservative maximum reservation, not RSS.

### Executed supplied TileBuilder acceptance

Fourteen focused tests bring navigation to **208**, workspace libraries to
**1,792**, plus four examples. All-target Clippy, scoped formatting and whitespace
checks pass. The original complete `TileBuilder.cpp` and supporting Recast/Detour
units compile unchanged in an external closure using the previously pinned
native/Boost dependencies. The wrapper directly calls original
`getTileBounds`, `GetMapSpecificConfig` and **`buildMoveMapTile`**; it does not
replace the pipeline with reimplemented native stage calls.

Debug and release each match **107 configuration cases / 1,883,200 bytes** and **23 full
builds / 14,375 processed subtiles**. There are **22 exact pristine Detour payloads
/ 2,247,764 bytes**, plus one matching empty result. Cases retain both base units,
both map exceptions, coordinate boundaries, slope overrides, ramps, water/magma,
islands, empty geometry and an off-mesh gap. Eight added cases include a genuine
out-of-tile triangle extending the height bounds. The original flat cases are
retained as negative query evidence. Maximum recorded work is 2,018,720,688
units; all detail recovery counts are zero. Case data match between profiles
after excluding the different external output paths. The earlier MMap 762-global
and 728-tile case arrays remain unchanged. All source, native closure, input and
binary guards pass; 1,765 runtime network traces are empty, including the separate
save-time observation. Integration sources, dependencies and the branch remain
unchanged; no client read, publication or server activation occurs.

Original Detour and actual RustyCore public APIs load all 22 payloads. They
produce **11 complete paths, one partial path and ten absent-nearest errors**.
The original tight flat height bounds can exclude raised rasterized surfaces
from BV nearest-poly lookup. Those cases retain native success for loading and
native invalid-parameter failures for path queries with missing nearest refs;
they are not successful-route evidence. The added height-context cases provide
separate positive routing evidence without deleting the flat negatives.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `tilebuilder-debug-20261011T092728Z-8e46e947.json` | `d0baf13ee19997dcd52a09ebdea5619e7eb3ce8a9d4047f12292af1b427b40e3` |
| `tilebuilder-release-20261011T092728Z-c82f7767.json` | `55b0df857afb0204b33395fa53901a636b688307fbfdc906c64db38f0789cb37` |
| `tilebuilder-rustycore-20261011T093013Z-52ffa8ae.json` | `b3ee3c0f422cb1bd901a9a42295dfa671de9e4159d67dea13a43e3b6d8bb4111` |
| `navigation-workspace-20261011T092728Z-02b466f6.json` | `a39d86779b46fa36a300c114a449820aba90c2d3f438ef9e0f9edb552cb0954e` |

### Save-time gate discovery

Running the original **`saveMoveMapTileToFile`** exposes a separate final-file
gate. It calls `navMesh->addTile` on the generated buffer **before `fwrite`**.
That insertion initializes link storage, polygon heads and internal connections;
off-mesh connections can additionally snap their vertices. The original worker
creates a fresh navmesh for each tile, so initial salt/index state is part of
the required deterministic producer behavior.

All **22 actual saved files** differ from simply prepending the MMap header to
the pristine builder payload (390,326 differing payload bytes cumulatively).
The focused flat-tile observation has 18,047 differing bytes within the polygon
and link blocks of its 103,912-byte payload. Its evidence is
`tilebuilder-publish-probe-20261011T092808Z-04cc74a8.json`, SHA-256
`34bcdab26766012b184b9f8da29605f1a3c4e4bf492da112bddb6ff2e4443de3`.

The preceding MMap codec acceptance proves the native structs and container
framing around a supplied pristine payload. It **does not prove final producer
file bytes**. At that checkpoint pure Rust save-time mutation was missing.
Successful runtime loading of pristine payloads did not close that gate. The
implementation and final-byte acceptance below now close it for the bounded
synthetic corpus.
An independent Python wire audit reconstructs fresh-tile link initialization
and internal edges exactly for 21 of the 22 saved files. The remaining gap case
also snaps both off-mesh endpoint heights and connects their links. This audit
locates the missing producer step; it is not a Rust implementation. Its report
is `tilebuilder-save-state-audit-20261011T093318Z.json`, SHA-256
`ad76b27ee8b63305aa9ffaf115007ca325d93c862a703a1ef4d0d7c66065b7f3`.
Real inventory/acquisition, unified output and high-level world acceptance also
remain required; the full extractor goal stays active.

## Pure Rust final save state — 2026-10-11

`detour::Data::prepare_for_save(params, SaveLimits)` consumes builder output and
returns a distinct `SavedData`. `tilebuilder::Data::into_saved` exposes the same
step after complete supplied-geometry construction. `mmap::encode_saved_tile`
frames that final payload. `mmap::encode_tile` retains its explicitly pristine
codec contract. Saved data cannot be passed through the consuming step again.

The port preserves the original worker's **fresh navmesh, tile index 0, salt 1**
state, including its fixed 64-bit reference base `1 << 52`. It initializes every
unused free-list entry and every polygon head, inserts internal edges in reverse
edge order, connects off-mesh starts and then same-tile ends, and snaps accepted
endpoint vertices. Start backlinks are unconditional; end backlinks require the
bidirectional flag. Link allocation order and exhaustion behavior are retained.
It does not create connections to absent neighboring tiles.

Nearest projection retains the **128-candidate cap**, BV traversal (including
its reserved node), non-BV polygon order and strict first-tie selection. It
uses original detail triangles for barycentric height and detail edges for
boundary/degenerate fallback. The final horizontal radius check remains separate
from candidate lookup. Undefined native numeric conversions, invalid topology,
nonfinite executed arithmetic and absent fallback edges reject explicitly.
Builder detail-cursor wraps and neighbor-marker collisions reject before mutation;
this includes a reported unused final cursor wrap. No general arbitrary-payload
or native malformed-input admission compatibility is claimed.

This step reuses the consumed byte allocation with **no additional heap scratch**;
its candidate array has 128 entries on the stack. Downward `SaveLimits.work` caps
preflight, free-list/poly visits, query traversal and projection work at
**128 million visits**. Builder allocation limits still apply. These are logical
algorithm bounds, not process RSS or allocator-overhead limits. Errors discard
the consumed result; no partially modified output escapes.

### Executed final-save acceptance

Ten focused tests cover links, both BV modes/directions, endpoint snapping,
outside ends, boundary/radius rejection, detail fallback, the candidate cap,
first ties, malformed detail/float inputs, diagnostic gates and exact work limits.
Navigation now passes **218 tests** (13 ignored external corpora); the workspace
passes **1,802 library + four example tests**, with 14 ignored corpora overall.
All-target Clippy, scoped formatting and whitespace checks pass.

Both debug and release compare **750 retained synthetic cases**: **747 exact
saved Detour payloads / 10,916,236 bytes**, and three expected diagnostic
rejections (`detail-cursor-533`, `detail-cursor-534`, `neighbor-marker-collision`).
The original broad 728-payload corpus contributes 725 admitted cases; the complete
TileBuilder corpus contributes 22. Its **22 full MMap files also match actual
original `saveMoveMapTileToFile` output byte for byte**, including the gap's two
snapped endpoints and links. The unchanged pinned native Detour `addTile` is the
mutation oracle. Existing guarded construction outputs are reused; this step does
not claim a new rerun of every 625-subtile build or a real-client extraction.

Strict `TileView` parsing and unchanged actual RustyCore public APIs load/remove
all **747** files and reproduce **171** query cases: **158 complete paths, three
partial paths and ten expected absent-nearest errors**. The legacy container
parser still rejects version 16; it has not been relabeled. Public Detour APIs
drop successful partial-status detail bits, so query evidence compares their
exposed path/point/reference/error projections. No `MMapManager`, world
`PathGenerator`, neighboring-tile runtime or server bootstrap acceptance is inferred.

Source/native/input/binary guards pass. Both profiles have identical case data;
**2,243 runtime network traces are empty**. An earlier harness attempt traced the
compiler itself and denied its local socket-based process plumbing; that failed
report is retained. Final corpus runs precompile outside the runtime trace, then
deny and trace network syscalls during Rust/native/reader execution. No client
asset read, dependency download, publication or activation occurs. Previous
successful runs and unrelated dirty work are retained.

External `reports/acceptance/` evidence:

| Report | SHA-256 |
| --- | --- |
| `detour-save-debug-20261011T094832Z-5a353b2f.json` | `ea1b3b1f4e938ee7904fc61085c2c5729033743893d0015f73f761d72d197e89` |
| `detour-save-release-20261011T094903Z-df88c6d0.json` | `412f631ca20d81c8424022e603f0d43ae2650ceab7a5ace1a619eb9a246decb4` |
| `detour-save-rustycore-20261011T094920Z-6ad7a878.json` | `12ea148cd1f41120ece1bb300acfd630660ed29f2f34d7e459cb19b375c1d857` |
| `navigation-workspace-20261011T094739Z-994a3a90.json` | `fe1c254902e3080d4cd36ef898961b88cabf5ad3c93052ffe7023e4c08a1630a` |

This closes the observed save-time producer gate for supplied synthetic geometry.
Real source discovery/acquisition and parent selection, unified product output,
resumable manifests, platform distribution and full real/runtime acceptance
remain. The full Forever extractor goal stays active.

## Navigation source and ancestor inventory — 2026-10-11

`forever_db2::terrain_sources::read_navigation_maps` returns direct numeric Map
records plus validated copy metadata. It preserves both original signed parent
fields, MapType, signed InstanceType and Flags1. The effective parent is
ParentMapID unless negative, then CosmeticParentMapID. Original startup assigns
records directly: the last duplicate ID wins (including zero), copies are not
replayed and zero WDT IDs are not filtered. This consumer never reads strings;
null unused Name/Directory pointers are admitted without weakening the existing
terrain/VMap string projections. The same complete closed Map schema and
regular/minimal-empty unencrypted profile apply; the reviewed schema count stays
56. This does not make an incomplete real Map artifact usable.

`inventory::Plan::from_source` consumes that snapshot and complete immutable
`Input::{Terrain, CollisionMap, CollisionTile}` descriptors. `from_rows` is also
available; source provenance remains the caller's responsibility. The planner
performs no filesystem discovery, acquisition or side effects. Its QA corpus
bridge reads canonical four-digit map filenames only; it is not a production
rooted inventory reader or proof of complete directory binding.

The plan preserves the pinned MapBuilder rules:

- Terrain tile-list and VMap filename coordinates form a deduplicated union,
  sorted by packed `(row << 16) | column`; empty lists/directories retain entries.
- All-map selection applies the ten continent IDs, Flags1 bit 2/MapType 3 junk
  filtering and InstanceType 3 battleground filtering.
- Explicit map selection overrides every skip flag and retains its ancestors;
  only the requested map is an output. All-map outputs use each map's own tiles.
- Nonempty outputs derive global parameters from the ultimate ancestor's own
  complete union, rather than the child or a chain-wide union.

Before selection, the complete chosen source graph is validated. Missing parents,
cycles, invalid negative sentinels and excessive depth reject. A nonempty output
requires a present, nonempty ultimate-root inventory. These strict diagnostics
replace native default insertion/invalid-navmesh behavior on bad inputs; native
malformed-input rejection parity is not claimed. Tile lists require exact
4,108-byte MAPS v10 framing, matching authorized build 70170/70205 and ASCII 0/1
flags. Native ignored-build/trailing-byte admission is not reproduced. Filtered
lists are not decoded. Map IDs remain u32; explicit selection rejects values
outside the native signed CLI range. Native unordered parallel scheduling is
outside this deterministic sorted plan.

Default limits are 200,000 direct rows, 4,096 maps, 1,000,000 descriptors,
1,024 chain nodes, 64 MiB of charged logical heap capacity and 128 million work
units. Overrides only reduce them. Actual vector capacities and temporary chains
are charged conservatively; this is not an RSS or allocator-overhead bound.
Parameter construction uses a fixed 4,096-coordinate / 8 KiB stack array.
The returned statistics expose source/selected counts, work and heap reservation.
Failures expose no partial plan. Unit tests include all continent IDs, exact
work/heap limits and a complete 1,024-node chain; 1,025 nodes reject.

### Executed inventory acceptance

Fifteen inventory tests plus three source tests bring navigation to **233 passing
tests** (14 ignored external corpora), DB2 to **299**, workspace libraries to
**1,820** and example tests to **four**. Sixteen external corpus tests are ignored
normally; the relevant two are executed explicitly. Workspace Clippy and scoped
formatting pass.

Debug and release each match **82 synthetic packages**, containing 602 direct
rows, 177 discovered map entries, 4,366 union tiles and 4,323 requested output
tiles. Direct-source dumps total 15,128 bytes, inventory dumps 20,244 bytes and
**140 global files / 5,600 bytes**. Cases cover both authorized build tags,
cosmetic and zero parents, last duplicate assignment, unreplayed copy chains,
all filter combinations and explicit-map overrides, empty inputs/directories,
full 4,096-tile maps, boundary coordinates and a 64-deep native parent chain.
The larger 1,024-node limit has Rust unit evidence only.

The native wrapper uses original DB2FileLoader/MapLoadInfo and numeric getters,
then executes actual MapBuilder construction, `discoverTiles`, `getTileList` and
`buildNavMesh`. Original code, Boost filesystem implementation and build objects
are pinned/hash guarded outside Git. Private inventory access is exposed only
for QA. Original full tile builds and parallel workers are not invoked here.

The strict `GlobalView` reader feeds unchanged RustyCore public navmesh
initialization and `calc_tile_loc`: **140 files and 3,500 exact coordinate
queries**, including native maximum-tile counts. Debug/release and typed runtime
checks retain **306 empty network traces**. No real client was read.

This acceptance reveals a separate legacy global-reader hazard:
`DetourNavMeshParams::parse` accepts at least 28 bytes, so it accepts a 40-byte
MMap 16 global and interprets the header as raw parameters. The runner proves
that those bytes differ from the correct decoded parameters, without initializing
a navmesh from them. Actual initialization uses the strict reader. The earlier
legacy **tile** parser rejection remains valid. Integration source and HEAD are
unchanged. No tile loading, pathfinding or gameplay acceptance is inferred from
these coordinate tests.

The first typed harness incorrectly expected legacy global rejection; its failed
report and the preceding compile/link diagnostics are retained. The corrected
harness records the observed reader behavior. Production directory discovery,
terrain ancestor-file selection, active model-instance loading, complete real
sources, unified manifests/resume and world/runtime acceptance remain open.

### Retained inventory evidence

Reports are under the external laboratory's `reports/acceptance` directory;
raw synthetic outputs remain in the private candidate run root.

| Report | SHA-256 |
| --- | --- |
| `navigation-inventory-debug-20261011T101731Z-f05e78a1.json` | `3d02438dba1c61e6b5fbef4b477e49eba40ce0ebcdb316e522662cb1bb78b7a9` |
| `navigation-inventory-release-20261011T101732Z-aab6d899.json` | `9943da5335f799c7d3d6d06eab9014c46c3abafed6b9df34e95efc2a5928ec5c` |
| `navigation-inventory-rustycore-20261011T101747Z-9968fce3.json` | `614eabe2f1e0de19e3542314de087edfd647156ece6ba52f0e75e17e782add93` |
| `navigation-workspace-20261011T101749Z-0078daa7.json` | `c0b8296255379ca7f97a907ce2226437e469adccc949fa2efff9d76db97485ad` |

## Active VMap model loading — 2026-10-11

`forever_vmap::store::StaticStore::loaded_models(map)` borrows active models in
original primitive-slot order. Each item exposes its slot, first-bound placement,
checked owned VMO and cached transform. Unloaded slots and null models are
skipped; repeated references yield one instance. Unknown maps and poisoned stores
reject; declared uninitialized maps yield an empty iterator. Borrowing prevents
concurrent store mutation. The iterator is cloneable/fused, performs no source
I/O or allocation, and does not reparse model geometry. A bounded placement
record is decoded without allocation to expose its exact first-binding name and
numeric bits. Source verification remains an explicit store operation; these
borrows refer to the already checked immutable in-memory snapshot.

`models::append_loaded` consumes a terrain prefix and the store snapshot. It uses
owned O(1) group views and cached transforms through the same private projection
kernel as the supplied-model API. There is no temporary selected-model Vec or
new model/group descriptor allocation. Slot scans are bounded by store limits
(up to 1,000,000 slots per pass); the two projection passes retain the existing
4,096-model, 65,536-group and mesh capacity limits. The model iterator is private
to the common kernel; callers cannot supply an unbounded iterator. Store memory,
provider inventory/scratch and the consumed prefix retain their separate stated
capacity policies. No total-process RSS bound is claimed.

`models::append_tile` implements the pinned TerrainBuilder::loadVMap lifecycle:
load the requested tile, project every active model on that map, then unload the
requested tile. Other loaded tiles are intentionally visible and remain loaded.
One unload removes all requested-tile references, including preceding loads;
this follows the original rather than restoring a previous reference count.
Successful loads always unload after projection, including geometry-limit or
transform failure. Store load failures preserve the existing transactional
contract; missing/corrupt inputs remain explicit errors. A map may have been
initialized by a successful load before later projection rejects. No empty
success/fallback is fabricated from a source error. Navigation requires
LoadPathOnlyModels=true and coordinates 0..63 before I/O. Existing strict parent
payload fallback, child-owned index binding and first-binding policies belong
to the store. Direct LiquidType reduction is still supplied by the caller.

### Executed loaded-model acceptance

Six navigation tests and two store tests add **eight passing tests**. Navigation
now passes **239**, VMap **467**, and workspace libraries **1,828** plus **four
examples**. Seventeen external corpus tests are ignored normally; this increment
executes the new loaded corpus and the older supplied-model corpus explicitly.
Clippy and scoped formatting pass.

Debug/release each match **154 synthetic packages / 1,507 operations / 227,004
bytes** against original C++. The fixed navigation policy adapts 72 complete
static-store packages into persistent snapshots and tile lifecycle cases, plus
ten independent WMO-liquid bank fixtures. Original static-store coordinates
outside 0..63 are excluded from this navigation corpus. Cases cover own/cosmetic
ancestor payloads, child indexes, first-binding parent bits, reverse load order,
repeated/shared references, null/unloaded slots, shared models across maps, long
basenames, M2 winding, rotations/negative scales, rectangular liquid grids and
liquid banks including area-zero mapping. Their output contains 10,242 solid vertices, 3,414
solid triangles, 480 liquid vertices, 240 liquid triangles and 240 area bytes,
cumulatively across operation snapshots. Every output byte and post-operation
map slot/active count matches.

Per profile, **543 loads** project original `getModelsOnMap` slot snapshots using
actual TerrainBuilder::loadVMapModel; **356 tile appends invoke actual original
TerrainBuilder::loadVMap**, including its manager load and scope-exit unload.
Other operations unload tiles/maps. The Rust side uses the actual rooted VMap
source/store and new adapters. Complete supplied-model regression still matches
all **97 cases / 3,951,732 bytes**, with exactly the prior case arrays in both
profiles. Combined loaded/regression checks retain **506 empty network traces**.
Original sources, reused objects, Boost source closure, wrappers, binaries and
synthetic inputs are hash guarded. The initial native link failure is retained;
the final executable links the previously pinned original Boost filesystem
implementation, with no algorithm stubs or downloads.

This is synthetic source selection and geometry acceptance. The increment does
not read a real client, execute new RustyCore path queries, discover complete real
map/model inventories, reduce LiquidType DB2 automatically or perform the full
source-to-TileBuilder/product/runtime pipeline. Terrain ancestor-file selection,
complete real source acceptance, versioned manifests/resume and gameplay routing
remain open.

### Retained loaded-model evidence

External laboratory acceptance reports (raw outputs remain private):

| Report | SHA-256 |
| --- | --- |
| `navigation-loaded-debug-20261011T103724Z-e07108a5.json` | `379554b4276299019288eef0f228a17f5e8b9a0b2fe0bcd2dd07d964f724bb5b` |
| `navigation-loaded-release-20261011T103728Z-f005ec64.json` | `5370e3c188786e9fa087bd55192bfb2adbf2a0707c876fbdf0a68c0628301c50` |
| `navigation-model-debug-20261011T103752Z-ae0a00c1.json` | `57bf36dc55e9c4c40bcfc12565d3a09d95792dd251e2411adc361be2e97dbc7d` |
| `navigation-model-release-20261011T103754Z-add6ad3a.json` | `f71812539735bd48a4b8eca907d9bbe950f2d862645eacd501fe2656cce9db6b` |
| `navigation-workspace-20261011T103756Z-6fbe6ab8.json` | `d6b7cd27e72d0f5df33f092e765f9c87ce96f04f3e96aec6e06adac4a87740c4` |

## Rooted terrain selection and loading — 2026-10-11

`terrain::load::neighborhood` accepts an inventory plan, an admitted output tile,
a complete immutable source, liquid-skip policy and downward-only limits. The
plan now retains its authorized build tag; every present MAPS file is verified
against it before projection. Source coordinates are filename `(row,column)`.
The result owns the mesh and exposes selected origin map IDs plus read/byte/
capacity statistics. Origins follow center, column+1, column-1, row+1, row-1;
None also covers suppressed neighbors and out-of-domain boundaries.

Selection follows the original public TerrainBuilder::loadMap rules:

- Each coordinate starts with the target map and walks the complete validated
  effective-parent chain independently on genuine absence.
- Present complete flat or triangle-empty center geometry blocks fallback and
  suppresses every neighbor read. Unreferenced center vertices are retained
  when triangulation produced no usable face, matching the original.
- A usable center enables Left, Right, Top, Bottom neighbor strips in that order;
  source origin IDs do not change target tile coordinates.
- Present corrupt/incomplete files, I/O failures, mutation and build mismatches
  are explicit errors. They never become an empty successful result or trigger
  ancestor fallback. This is stricter than native fopen/reader error tolerance;
  malformed-native rejection parity is not claimed.

Neighbors outside 0..63 are absent without I/O. The admitted package contains
only canonical coordinates; no claim covers externally supplied files with
native unsigned-underflow/coordinate-64 filenames. Unknown, filtered or
non-inventory center selections reject before source I/O. Source verification
runs before loading, after the center, and after a complete nonempty selection.
No partial mesh is exposed after a failed selection. The center is projected
once to decide neighbor reads, then its temporary mesh is dropped before further
source reads and the final projection. Computation is bounded to six region
projections, without simultaneous old/new center meshes.

### Transfer and rooted source limits

Default loader ceilings are MAPS_MAX bytes per file (**202,112**), five times that
in cumulative transferred bytes (**1,010,560**), **5,120 read attempts**, and
**2 MiB source vector capacity**. The existing geometry limit remains separate
(250,000 vertices, 500,000 triangles, 24 MiB logical capacity). Buffer growth
charges actual capacities and old/new overlap. Overrides only reduce ceilings.
Provider-owned inventory/scratch, fixed stack and allocator overhead are outside
these ledgers; this is not an RSS bound. Source errors preserve distinct I/O,
mutation and limit diagnostics. Missing with nonempty transferred bytes rejects.
A zero-byte present file remains present and fails strict MAPS verification.

The optional Linux `terrain::load::fs::RootedSource` anchors a supplied MAPS
directory through a retained descriptor. Generated basename-only requests use
exact map/row/column identities, with no file guessing, directory enumeration,
CASC or network. File opens reject symlinks/nonregular objects and check complete
EOF; root/file metadata are guarded before and after reads. Full bytes are
streaming-SHA-256 fingerprinted and subsequent reads compare previous content.
Observed mutation permanently poisons the source. Even unrelated root entry
changes invalidate the root identity. `verify_unchanged` checks recorded
identities/absence and root metadata, without rereading every content byte or
providing an atomic filesystem snapshot. It does not authenticate unseen files
or prove a complete real inventory. Explicit full post-run hashes remain separate
evidence. Relative roots retain the unchanged cwd requirement.

The flat-file mechanism is adapted from this repository's authored VMap rooted
source; no upstream implementation or dependency is imported. Its private
streaming SHA-256 keeps the same authored code and three known-vector tests.
Metadata defaults retain 65,536 entries, 128 MiB actual capacity (including growth
overlap), 4,096 root-path bytes and 262,144 metadata steps per operation. These
are separately reducible from loader/mesh limits. The source never writes input.

### Executed terrain-loading acceptance

Fourteen selection/transfer tests, eight filesystem tests and three streaming
hash tests add **25 passing tests**. Navigation now passes **264**, workspace
libraries **1,853**, and examples **four**; Clippy passes. Eighteen external
corpora are ignored normally. New tests cover exact read/byte/capacity limits,
1,024 parent nodes with exactly 5,120 reads, empty/dark centers, corrupt center/
neighbor inputs, independent fallback, changed source identities, symlinks,
complete EOF, invalid selections and pre/post-verification failures.

Debug/release each match **110 complete synthetic cases / 217,721,658 output
bytes** against actual original public loadMap dispatch, using the real pinned
VMapManager parent lookup. Both build tags, u8/u16/f32 sampled heights, flat and
liquid-only centers, skip-liquid modes, dark/hole handling, source precedence,
independent neighbor ancestors, boundary coordinates and depth64 chains are
covered. Native positives remain complete canonical inputs; error/reduction
cases are checked only in Rust. The larger 1,024-node boundary is unit evidence.

Per profile the compared geometry totals **11,492,700 solid vertices**,
**4,421,636 solid triangles**, **1,664,100 liquid vertices** and **521,402 liquid
triangles/areas**. The rooted Rust source performs 1,584 reads and transfers
37,716,190 bytes across 356 found files. Maximum observed per-case source vector
peak is 1,005,056 bytes; that observation does not replace the declared caps.
Every serialized array byte and independently checked index extent matches.

The prior **82-case** inventory regression remains identical in both profiles,
including all 140 global files. Combined runtime comparisons retain **388 empty
network traces** (222 new terrain-loader traces plus 166 inventory traces).
Original source/Boost/reused objects, wrappers, binaries, synthetic inputs and
complete outputs are hash guarded outside Git. No real client is read and no
integration repository is changed. The retained first workspace run reports a
Clippy test-style failure; the scoped fix passes the final complete checks.

This closes complete supplied-package terrain loading/selection. Real source
acquisition and immutable complete inventory discovery, automatic LiquidType
startup reduction, the joined terrain/model/offmesh-to-TileBuilder pipeline,
versioned product manifests/resume/distribution and world/runtime acceptance
remain open. No full extraction claim follows from this component evidence.

### Retained terrain-loading evidence

External laboratory reports (raw synthetic inputs/outputs stay private):

| Report | SHA-256 |
| --- | --- |
| `navigation-terrain-load-debug-20261011T105119Z-3d1208e2.json` | `e0986b2293613b3909bad54c76655a7547e704d4199bfd8adc53d9eb8d5c1608` |
| `navigation-terrain-load-release-20261011T105139Z-b2d74b22.json` | `ee13eb5925d636f28fef77a9109c8cafe2716d47d5b5e304a36363d2fa59418a` |
| `navigation-inventory-debug-20261011T105204Z-8be777e4.json` | `5beebbaafebdb9268ca278a49278f3fdeb9706da19b9594dbbf6309b3de6e5fc` |
| `navigation-inventory-release-20261011T105205Z-dbe3f992.json` | `9d8b3047af70a1eb7c5efd7c6260bcc4ec5dbae1f747afbf1ecf877470d0013d` |
| `navigation-workspace-20261011T105206Z-9a2ea053.json` | `2b1217d98ab6faeeb90c34456a12bd6a7b17d4338b9d83f28a7cc9cc876afb9a` |


## Single-tile source-to-final pipeline — 2026-10-11

`pipeline::run` joins supplied inventories and immutable package sources for one
selected output tile. This closes the missing composition API from the preceding
component checkpoints. It performs no directory discovery, DB2/CASC acquisition,
filesystem publication, product manifest, resume or full-map scheduling.

### Selection and failure contract

- `inventory::Map` retains independent terrain and collision bitsets alongside
  its unchanged sorted union. A promised terrain center must resolve completely
  through the effective ancestor chain. A promised collision tile uses strict
  `load_tile`. Missing promised components fail the whole request.
- Terrain-only tiles use `StaticStore::load_optional_tile`. A genuine absent tree,
  checked empty tree, or absent own payload **and** own index may return absence.
  A present partial pair, missing model/ancestor payload, corrupt file, I/O error
  or observed source change remains fatal. Strict `load_tile` is unchanged.
- `Plan::parent_rows` exposes the normalized effective graph. The pipeline checks
  the complete plan against the store before I/O and requires path-only model
  loading and no loaded tiles/active instances. Empty initialized trees are
  allowed. This exclusive context reproduces a fresh manager for each tile.
- Terrain is loaded first, then VMap models in original active-slot order. Target
  map state is released after projection, including projection errors and an
  optional absent tile. Nothing partially generated is returned on failure.
- No vertices returns `NoVertices`. Compaction discards unreferenced vertices;
  empty compacted geometry returns `NoReferencedVertices`. Connections are
  selected/validated only after nonempty geometry. The complete ordered builder
  runs all 625 subtiles; no polygons returns `NoPolygons` without a file.
- Save consumes the construction allocation, initializes fresh Detour state and
  frames MMap 16. `usesLiquids` is **!skip_liquids**, including WMO liquids that the
  original still projects when terrain liquid loading is skipped. Both sources
  are verified again before returning a successful result.

`models::liquid::Table::from_source` reduces the closed LiquidType snapshot's
**direct rows only**. `from_rows` accepts the equivalent explicit rows. IDs sort
uniquely, the last original ordinal wins, ID zero is retained, and MaterialID is
unused. Copies are never replayed. A bad overwritten SoundBank does not shift;
only final banks must be below 31. The borrowed callback retains zero for unknown
IDs and the original u8 truncation for high supported banks. This is reduction
of caller-supplied sources, not execution of the original DB2 startup loader.

### Resource and source boundaries

The pipeline defaults to 512 MiB of logical owned capacities. Terrain source
buffers reserve their own ceiling before geometry admission. Projection and
compaction preserve their existing overlap ledgers; selected connection capacity
and mesh residency are subtracted from the builder's budget. Mesh/connections
are dropped before final save; saved payload and final framed output overlap is
charged. Provider metadata/scratch, persistent store residency, caller-owned
source records and allocator/OS overhead have separate policies; no RSS claim
is made. Existing component work/shape ceilings remain in force. Every override
can only decrease its default, and upward overrides reject before source I/O.

Liquid reduction defaults to 200,000 direct rows, 65,536 unique IDs and 2 MiB of
actual index/output Vec capacity including overlap. Sorting indices by ID and
source ordinal requires no separate sort heap. The final table borrows no source
storage and exposes its actual retained capacity. Inventory's two new bitsets
add exactly 1,024 reserved bytes/discovered map; union output and work are unchanged.
Rooted source verification retains its earlier observed-identity/absence policy,
not an atomic filesystem snapshot. Complete post-run input hashes are checked
separately by the acceptance harness.

### Executed single-tile acceptance

Debug/release each pass **nine complete authored synthetic scenarios**, producing
**seven exact final MMap files / 452,012 bytes** and two no-vertex empty outcomes:
terrain-only for both build tags, both base-unit choices, flat terrain, liquid-only
terrain, skipped liquid, models-only, combined sources and offmesh, cosmetic/direct
ancestor loading with a border neighbor, and WMO liquid with skipped terrain liquid.
The liquid case supplies a duplicate direct ID, ID zero and an overwritten bad
bank, with matching native final-bank callback behavior.

The native wrapper invokes the untouched original public `TileBuilder::buildTile`,
including loadMap/loadVMap, cleanVertices, connection selection, complete Recast,
Detour and save. The base class always skips non-debug tiles; QA overrides **only
that virtual selection gate** to admit the explicitly requested fresh tile. It
installs a bounded explicit VMapManager factory/topology/liquid callback. The full
PathGenerator startup, MapBuilder discovery and production worker are not invoked
by this comparison. Original algorithm implementation units are reused with source,
object, dependency and binary hashes; no algorithm stubs replace them.

A current separate adapter preserves the actual usesLiquids flag, parses MMap 16
strictly and invokes RustyCore's unchanged public `wow-recastdetour` add_tile/query/
remove_tile API at integration HEAD
`544721324cf0a47553caae1ea0299a0ff2917ce7`. Across both profiles it performs 14 loads
and 56 compared query calls: **two complete paths, ten partial paths and two
absent-nearest errors**, matching native results. Fresh one-tile runtime params
come from payload bounds, as in the earlier query adapter. RustyCore's legacy tile
parser rejects MMap 16; this is not full reader/world/gameplay compatibility.

Nineteen focused tests were added: nine pipeline, five optional-store, four liquid
reducer and one component/topology inventory test. Navigation passes 278, VMap 472;
workspace libraries pass 1,872 plus four examples, with 19 normally ignored explicit
corpora and clean all-target Clippy. Debug/release strict-store regression keeps
all 75 case arrays/3,298 operations identical, including 58 Rust-only negative cases.
The 82-case inventory retains every output and its first seven semantic/work
statistics; only its heap reservation gains 1,024 bytes/discovered map. There are
**634 entirely empty runtime network traces** across these pipeline, new reader and
regression runs. Source/dependency/input/binary guards pass; old outputs remain.

The bounded MMap QA runs in a persistent local user service with exact commands
and logs under the private run root. The initial harness attempt retained a failed
inventory comparison because its base-class selection gate skipped generation;
the corrected wrapper executes the complete build and supersedes that attempt.

Complete real inventories/acquisition, complete real source tables, original DB2
startup/full-map scheduling, versioned product CLI/manifests/resume/distribution,
multi-tile/high-level world acceptance and publication remain open. The complete
Forever Rust extractor is **not finished**.

### Retained single-tile evidence

External laboratory reports; raw synthetic files remain private:

| Report | SHA-256 |
| --- | --- |
| `navigation-pipeline-debug-20261011T112037Z-f91c0864.json` | `e4ca4ff1efbd21cc64a62cb688f246b34da9515ace772c03e1d419de3e2f58d8` |
| `navigation-pipeline-release-20261011T112257Z-de001a70.json` | `aaee1768afabe5c561bf0fbf2e377653f412cc027910a9d871756fd68b59584d` |
| `navigation-pipeline-rustycore-20261011T112714Z-46b18311.json` | `5b8e08af729d3df5d19a367e118b2ecf8abeebdc90b98a018e1d54f9f1050dd3` |
| `navigation-inventory-debug-20261011T112047Z-c92285f8.json` | `ff41bf10f623be02283d9b04d925cc74f675198f9e61c278104fe7a80f746ca7` |
| `navigation-inventory-release-20261011T112048Z-2784bd7c.json` | `0cc23494e4cf9cc2cf2d79ad673faa4b1a52a8aac8f5dfb59de1828e15e6d38b` |
| `native-static-store-debug-20261011T112117Z-85b5cdef.json` | `e5c247520f375869542c7b19a4bf569b16875a9a732d78296b3ccf3975ef9eae` |
| `native-static-store-release-20261011T112120Z-b9a64b0f.json` | `21b9299eb7ea8bffcb76a9f766dbb5af942046c72af39786a9d0a4251191e533` |
| `navigation-workspace-20261011T112159Z-75564055.json` | `ecdc07dfcf28d8679e9e535e0208fbd63a21e24eaf3660d356ddd6dec54ef0df` |


## Rooted package discovery and binding — 2026-10-11

`inventory::fs::Package` is the Linux local-source adapter for the preceding
single-tile pipeline. `open` accepts a complete closed numeric Map snapshot;
`open_rows` accepts equivalent explicit direct rows. Both retain navigation's
last-direct-ID-wins graph and ignored-copy semantics. Map/LiquidType acquisition,
full startup, product publication and full-map scheduling remain caller work.

The admitted canonical-name profile uses exactly four decimal map digits, padded
with zero, and two decimal row/column digits in 0..63. Terrain discovery reads
`maps/NNNN.tilelist`; collision discovery enumerates `vmaps/NNNN` and its complete
`NNNN_RR_CC.vmtile` names, retaining an empty collision directory as an empty map.
Other root entries are observed but do not create descriptors. Native's more
permissive unpadded/aliased/long filenames and out-of-grid coordinates are not
admitted or claimed compatible. Existing assembler output uses the canonical
names. Relevant nonregular/symlink entries reject instead of being followed.

The complete effective graph is checked before filesystem opening. Source filters
are applied through the original plan predicate before reading a skipped map's
list or traversing its collision directory. A selected map still admits its
required ancestors and overrides skip filters. Unknown maps, incomplete ancestor
inventories, corrupt/truncated/wrong-build lists and ambiguous names fail. There
is no available-prefix or unreadable-file fallback.

Both roots are opened without symlink/dot/dotdot aliases and retained by directory
FD. Enumeration, complete list reads and subsequent metadata checks use anchored
paths. File identity includes device/inode, mode/link/owner/length and mtime/ctime
with nanoseconds; atime is deliberately excluded. Lists require exact framing,
EOF and matching before/held/after identities, then retain SHA-256 fingerprints.
Observed entries sort deterministically. Directory changes, new/deleted entries,
file identity changes and root replacement reject and permanently poison an
existing package. The API checks observed metadata and namespace state; it does
not provide an atomic snapshot or rehash every file on verification. Acceptance
performs separate complete post-run input hashes.

Opening authenticates the admitted inventory/identity observations and tile-list
bytes. It **does not** decode/verify every unused terrain, VMap or model body.
`run_tile` checks the full observed inventory, creates fresh rooted terrain and
static-store providers with mandatory path-only loading, checks the inventory
again, runs the unchanged pipeline, and checks again before returning. Child
providers retain their complete-read/parser/source policies. Observed source
mutation poisons the package even when reported by a child provider. Source and
store caches drop after each call, so their metadata residency does not grow
across tiles. Temporary parent-row capacity is charged and released separately.
Caller connections/liquid lookup remain explicit immutable inputs.

### Bounds and execution cost

Discovery defaults to 1,000,000 observed entries, 4,096 admitted tile lists,
1,000,000 descriptors, 4,096 root-path bytes and 256 MiB of logical owned Vec
capacity, including old/new growth overlap and a fixed 64 KiB temporary filesystem
allowance. Plan, caller records, rooted-provider inventories, store residency and
pipeline capacities use their existing separate ledgers. Allocator/OS/library
iterator overhead and RSS are not measured by these logical capacities.

The discovery phase and each subsequent verification have an 8,000,000-metadata-
call ceiling. Opening performs a separate renewed verification after planning;
its diagnostic total can include both phase and final verification calls. Every
policy override only decreases a default, with admission before allocation or
metadata work. Statistics retain observed counts and capacity reservations.

Each `run_tile` performs full observed-inventory verification. Its metadata cost
is proportional to the observed entry count, independent of the selected tile's
geometry size. A scalable full-map runner, provisional sink/publication policy
and restart/resume belong to the remaining product orchestration work; do not
infer that this single-tile API is an accepted full-world scheduler.

### Executed discovery/binding acceptance

Debug/release each pass **82 canonical synthetic discovery cases**, reproducing
**140 globals / 20,244 inventory bytes** against the actual original MapBuilder
constructor/discoverTiles/getTileList/buildNavMesh. The prior synthetic corpus is
copied privately with numeric VMap directories normalized to canonical four-digit
names; geometry, DB2 bytes and options are unchanged. **174 admitted tile-list
hashes** are checked/profile. Cases include complete/empty/mixed inventories,
all filter combinations, explicit map overrides, ID zero, duplicate direct rows,
ignored copies, both build tags, depth64 and the full 4,096-coordinate grid.

The preceding nine pipeline packages additionally receive canonical tile lists
from their explicit synthetic request metadata. The production `Package` opens
and discovers them, then executes `run_tile`. All **seven final files / 452,012
bytes** and both empty outcomes match original public TileBuilder in each profile;
every pipeline statistic matches its preceding supplied-descriptor result.
The unchanged strict Forever/actual RustyCore public Detour adapter reproduces
one complete path, five partial paths and one absent-nearest error/profile.
Legacy MMap 16 parser/world compatibility is not established.

Fifteen focused tests cover union/component provenance, empty directories,
ancestor/filter admission, source failures, strict names/kinds, symlink/root
aliases, permanent mutation poisoning, capacity/metadata ceilings and bound tile
execution. Navigation passes 293; workspace libraries pass 1,887 plus four example
tests, with 20 normally ignored explicit corpora and clean all-target Clippy.
**200 runtime network traces are entirely empty** across the new runs. Post-run
source/input/native dependency/object/binary guards pass; previous successful
runs and other dirty work remain. Bounded MMap acceptance runs in a persistent
local user service with retained commands, script and logs.

This advances real filesystem integration for the admitted canonical profile.
It does not acquire the missing real Map input, invoke original DB2 startup/full
workers, establish full-map extraction or add manifests/resume/distribution.
Complete real client/product/runtime acceptance remains open; the complete
Forever Rust extractor is **not finished**.

### Retained discovery evidence

External laboratory reports; raw synthetic packages stay private:

| Report | SHA-256 |
| --- | --- |
| `navigation-discovery-debug-20261011T114849Z-cf6e0fed.json` | `340ef3886ec12f1836af58ab8c89442c6514f696b1a714b6014a1f1cf6462b01` |
| `navigation-discovery-release-20261011T115109Z-a22ea3c7.json` | `934a3063a06ecf226b660d4e009a678726e22d7b8fb5b491aa89119140d25b34` |
| `navigation-workspace-20261011T114935Z-36864eaa.json` | `4e45575dbd4d8eec6550a87deabc40a4910fd0b1a7de004ab8cf5dcd75cfbbbb` |


## Sequential navigation selection execution — 2026-10-11

`inventory::fs::run::execute` composes the existing checked `Package`, liquid
lookup, explicit offmesh source slice and unchanged single-tile pipeline.
`Selection::All` visits planned output maps in ascending ID; `Map(id)` and
`Tile { map_id, tile }` select an admitted nonempty output map/tile. Tiles are
row major, then column, as original FlatSet packed IDs. Empty All selections
complete without outputs. Explicit empty, ancestor-only, unknown or out-of-range
selections reject. Ancestor inventories remain available for inherited geometry
and root parameters, without becoming extra selected output maps. Explicit
selection does not undo Package filters: use discovery's explicit map option
when an override is required.

Each map emits its strict MMap 16 global, retaining all raw offmesh records for
that map in direct input order. Single-tile selections retain whole-map/root
parameters. Each tile then emits its complete MMap or an explicit empty outcome
with the existing pipeline statistics. The original producer uses fresh Detour
state per queued tile; this runner does likewise. Its deterministic serial order
does not reproduce native thread interleaving or existing-file skip policy.

### Source binding and execution cost

The runner verifies the complete observed inventory at batch start and finish.
Before and after each complete rooted-provider read it checks the requested
artifact and its map directory against the original sorted metadata observations,
including absence. Both original held roots are also checked around each fresh
tile context. Changing a file after the initial check therefore cannot be admitted
as a new valid source by a fresh provider. Observed InputChanged/poisoned errors
permanently poison Package, even if cleanup exhausts another budget.

Discovery-guard cost is two O(entries) passes per batch plus O(log entries)
lookups and bounded metadata work per artifact read. The test with four tiles
and 600 extra unused files observes exactly 1,200 additional guard metadata calls.
Fresh rooted providers still perform their own bounded identity/hash/parser
checks; their local costs are separate. Contexts and static stores drop per tile,
so accumulated reads across a selection do not grow their metadata/model caches.
The existing `Package.run_tile` API keeps its stronger immediate full-inventory
checks and remains unchanged. Neither API provides an atomic filesystem snapshot
or full content rehash on verification. Batch acceptance separately rehashes all
inputs. Unused body completeness is not inferred from inventory admission.

### Provisional sink and failure contract

`Sink::emit` receives borrowed `Global` and `Tile` events. The runner retains no
schedule or output collection; global bytes drop before tile generation, and
tile output drops after its callback. Callback-owned copies, sink staging and
publication are caller policies. Every callback is provisional until `execute`
returns `Completed` after final inventory verification. The sink must retain its
failed-run marker until that success; no failure admits an available prefix.

`Failure` preserves either a structured run error or the exact sink error, plus
counts for acknowledged provisional callbacks. A map count means its global was
acknowledged, not that all its tiles succeeded. Missing promised terrain,
present partial/corrupt VMap packages, source mutation, policy ceilings and sink
failure reject the entire selection. Final verification can fail after every
tile callback; the result remains failed. Stopping through a sink error requires
no worker cancellation or background-thread cleanup. Retrying does not imply
resume safety or authorize overwriting earlier outputs.

Defaults admit at most 4,096 maps, 16,777,216 selected tiles and 1 TiB of cumulative
global/tile bytes, with only downward overrides. Map/tile counts, global input
record limits, slopes and nested output/pipeline policies are admitted before
source verification or callbacks. Cumulative bytes are checked before each
callback. Discovery's 8,000,000 metadata-call ceiling renews for each tile and
full verification. The original plan/provider/store/pipeline capacity ledgers
remain separate; temporary parent-row Vec capacity is charged/released against
discovery memory. Logical capacities exclude allocator/OS overhead and RSS.
Caller source records, liquid table, connection slice and sink storage are not
charged as newly owned runner allocations.

### Executed batch acceptance

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

The three new packages exercise multiple maps/tiles, neighboring geometry,
both build tags, both base-unit settings, explicit map selection and independent
ancestor fallback. QA reads complete synthetic Map DB2 files with the production
closed source reader. The original native DB2 getters/MapBuilder enumerate the
same output map/tile sets and produce exact global parameters. Independent raw
offmesh framing is checked where that inventory oracle has no connection input.
Original public TileBuilder::buildTile is called for every scheduled tile with
its real map global parameters and the same complete source package. Output sets,
bytes, deterministic callback order, complete markers and all nine preceding
pipeline statistics are checked. The unchanged RustyCore public Detour adapter
validates each generated tile independently against the original query oracle;
this does **not** load all tiles together or prove cross-tile pathfinding.

The 23 focused tests include all/map/tile ordering and parameters, empty maps,
ancestor-only handling, preflight and cumulative limits, sink errors, exact
filename admission, source failures, permanent mutation poisoning between
callbacks, unused-file/final-boundary changes and guard cost. One additional
corpus test is normally ignored and runs explicitly in both profiles. Workspace
passes 1,914 total tests, with 21 normally ignored explicit corpora and clean
all-target Clippy. Full native worker scheduling, real complete startup/acquisition,
unified product manifests/staging/resume/distribution and high-level multi-tile
world/runtime acceptance remain pending. The full Forever Rust extractor remains
**incomplete**.

### Retained batch evidence

External laboratory reports; generated synthetic packages remain private:

| Report | SHA-256 |
| --- | --- |
| `navigation-batch-debug-20261011T121223Z-e7564263.json` | `8c9ce4480e9e22781619e10acaf9938c2df2601340501029f0791220f411a69c` |
| `navigation-batch-release-20261011T121939Z-f53db196.json` | `9c21d2f3b0f70a871ddde1d844074fafc8d50a843645150387268d114026b221` |
| `navigation-workspace-20261011T121240Z-31beab2e.json` | `9466aa7c570eaa5408ab726f39721c20d991bc9a248a93820f95a2fd0c28e264` |


## Multi-tile Detour and public reader acceptance — 2026-10-11

The standalone QA adapters load complete strict MMap 16 global parameters and
multiple verified final MMap tiles simultaneously. Original Detour uses its
pinned unchanged seven implementation units. The other adapter invokes the
unchanged public RustyCore `DetourNavMesh` and query APIs, using strict Forever
`GlobalView`/`TileView` admission and truthful `MmapTileBlob` metadata. Its frozen
debug binary reads both debug/release generated file sets. It does not use the
incompatible legacy MMap parser or initialize a live server.

Global origin, tile dimensions, root capacity and the original `i32::MIN`
max-polys sentinel pass through unchanged. The 64-bit reference layout uses the
public 21 tile / 31 polygon bits. No per-file one-slot navmesh, rewritten global
capacity or coordinates hide multi-tile interactions. Known complete synthetic
inputs pass the strict adapters before their unchanged native runtime use.
Malformed-native safety/rejection parity is outside this comparison.

### Positive cross-tile geometry and lifecycle

The previously accepted mixed-terrain package had no external polygon links,
so loading its files together could not prove a positive cross-tile route.
Three additional wholly authored packages use binary-exact low-slope, non-flat
height planes. Their shared V9 samples match in both axes; all ground triangles
are present and no liquids/holes mask the seam. They cover build70205/big-base,
build70170/small-base, and a child with two override tiles raised by 0.125 over
its four-tile parent. The normal checked sequential runner reads complete
synthetic Map DB2s, discovers the rooted package and generates every output.

Debug/release each match all **14 final files / 1,433,000 bytes** and **four globals**
against original DB2/MapBuilder discovery/global construction and original public
fresh `TileBuilder::buildTile` calls. No extraction/kernel behavior changes were
needed. Three small focused tests check exact borders, complete non-flat files
on both tags and 65,536 ground triangles; the authoring corpus is ignored normally
and invoked explicitly with private synthetic paths.

The multi-tile runtime suite covers these packages and all 12 preceding batch
packages. Each explicit world loads its own files, or a stated child-overlay/
parent-fallback selection, in sorted, reversed or rotated order. The original
runtime selects bounded query witnesses from its actual external polygon links;
endpoint centroids and native results are retained. RustyCore receives exactly
the same positions, filters and query capacities. A four-tile plane has 200
external links. The suite requires complete paths whose polygon references span
multiple tile slots, including after reload, in each designated positive world.
It also exercises disconnected/partial and absent-nearest queries.

The lifecycle is full load, duplicate-coordinate rejection, removal of the first
loaded tile, queries after removal, re-addition with a new salt/reference, queries
after reload, stale-reference removal rejection, complete reverse unload and
empty-world queries. Per-call native failure statuses, returned tile/polygon
references, path and straight-path counts, flags and coordinate bits match.
RustyCore does not expose successful Detour detail status flags; those flags are
normalized only for this public-API comparison, and are not acceptance claims.
Complete-versus-partial paths are additionally classified by the returned final
reference. The native binary remains an independent original Detour implementation,
not a reimplementation of the Rust public wrapper.

### Explicit capacity limit retained

One preceding synthetic child has an extra position outside its root's three-tile
inventory. Its own two files plus two parent fallback files require four slots.
Both engines reject the fourth load with `DT_FAILURE | DT_OUT_OF_MEMORY`
(`0x80000004`). This is **negative capacity evidence**, not a successfully loaded
complete world. The three-slot original global is retained, and no available-prefix
success or larger rewritten capacity is admitted. A separate valid four-slot
parent/child overlay has positive cross-tile evidence. Real inventory/root-capacity
and reader selection acceptance remain required before full-world claims.

### Executed results and bounds

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

The 33 scenarios include the explicitly expected capacity failure; the other
initial world selections load every supplied file. The 11 required cross-tile
positive scenarios cover root and valid parent-overlay selections in multiple
orders. All fully unloaded states report absent nearest polygons and the expected
path/straight-path errors. The standalone Rust adapter's three tests cover strict
query framing/numeric values, public fixed reference bits and unmodified global
capacity/sentinel handling. Its own all-target Clippy and workspace Clippy pass.

Each QA world admits at most 16 files, a root capacity up to 4,096, 64 query pairs,
4,096 nodes/path/straight points, bounded complete input/output files, and a
128 MiB cumulative tile input limit in the strict Rust adapter. Full MMap generation
runs in a persistent user service with retained exact command/script/logs; the
bounded query comparisons run locally without network access. Reports retain
input/source/native object/binary/consumer hashes and post-run guards. Previous
successful runs and unrelated dirty files remain.

These increments add authored QA sources and evidence, without modifying the
production extraction algorithms, integration source, formats or legacy version
line. They close bounded synthetic linking/lifecycle acceptance for the explicit
public API adapter. They do not execute original MMapManager acquisition/parent
resolution, its global offmesh connection manager, or Forever gameplay/bootstrap.
Complete real Map/model acquisition, high-level real world/pathfinding, unified
product manifests/staging/resume, packages and distribution remain open. The
operator's full Forever Rust extractor goal remains **active and incomplete**.

### Retained multi-tile evidence

External laboratory reports; all generated data is synthetic and remains private:

| Report | SHA-256 |
| --- | --- |
| `navigation-world-tiles-debug-20261011T123008Z-b0db8eeb.json` | `9a5bb3a7008a6a1a4b1850f04693987821de19a149e56da726147a531cf0c848` |
| `navigation-world-tiles-release-20261011T123823Z-02447c2a.json` | `fa1286816a475e36f0b2ffd0b6b4295550773cffb0f72ef8d5567ca1295236da` |
| `navigation-world-debug-20261011T123907Z-e3bc2ba7.json` | `2faa2d2bed7a882ec1c24a1952c66a3cb89e39e701f4c492ce7294761df0fdf1` |
| `navigation-world-release-20261011T124135Z-4f44f912.json` | `673fdeb209c866f7e1f100ab2039a85bb59105891e16e8d5c19fa9fd6da0ec40` |
| `navigation-workspace-20261011T123316Z-ecfb79d7.json` | `f6eacc6c6705fc82fdf79ff3668ab250e63f225e1a73dc93ed6379da2aac1013` |

## 2026-10-19: selected navigation from available 70450 records

The Linux API surface now includes an explicitly partial-capable build-70450
path. `forever_db2::availability::read_navigation_maps_70450`
returns `AvailableRecords<NavigationMapFields>` under the
`wowdbdefs-70450-available-navigation-v1` reader profile and
`wowdbdefs-70450-v1` catalog. It preserves admitted direct-row order and
duplicate IDs, copy pairs as metadata, and each section's coverage state. It
does not replay copies, synthesize bytes for unavailable source ranges, or
promote omitted sections to empty data. Callers remain responsible for binding
a stable authenticated source whose range reads contain the requested bytes.

`inventory::AvailableSelectedPlan::from_available_70450` builds the
`navigation-selected-available-70450-linux-v1` plan from direct rows only. It
uses the final direct row for a duplicate ID, then follows the effective-parent
chain through admitted direct rows. A selected ID found only in copy metadata,
a missing active parent, a cycle, a depth violation, or a missing required
inventory descriptor rejects the plan. The selected map is the sole output;
ancestors provide context. The owner keeps section coverage and partial source
counts attached and has no public conversion into the complete-source `Plan`.
Callers must establish complete inventories for the selected domain; this API
does not certify that an available snapshot covers the whole catalog.

`pipeline::run_available_selected` accepts that owner together with `Sources`,
`LiquidTypes`, off-mesh sources, `Request`, and `Limits`. It reuses the existing
single-tile pipeline and shared private kernel. It rejects a request outside the
selected output before reading terrain or collision sources, retains source
verification, and accounts work and heap against the existing cumulative limits
with downward reductions. Callers still provide authenticated acquisition,
complete selected-domain and ancestor inventories, the complete build-70450
`LiquidType` source (so a genuinely absent ID is distinguishable from a row the
caller omitted), and authoritative evidence for terrain absence; an I/O failure
must not become an absence result.

The library implementation and synthetic validation are checked: DB2 has 458
passes / 0 failures / 1 ignore, navigation has 329 passes / 0 failures / 20
ignores, all-target Clippy with warnings denied passes for both crates, and the
focused formatting, diff and 430-contract checks pass. These are software and
synthetic results. This remains a library API boundary, not a CLI profile and
not admission to product v21 when Map coverage is partial. No real MMap result
or v23 CLI profile exists yet, and this makes no full-world, native-parity,
RustyCore-runtime, Windows, or release claim. See
`contracts/forever-selected-navigation-contract-v1.json`.
