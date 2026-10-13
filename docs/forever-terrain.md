# Forever terrain foundation

`forever-terrain` is an original, dependency-free Rust library for complete
chunk framing, WDT/ADT metadata, finite height/liquid projection, bounded section
serialization, MAPS container assembly/verification and conversion of one
supplied ADT into MAPS. It performs no I/O,
CASC acquisition or runtime loading. Parsed framing does not
authenticate file contents: the caller must supply complete bytes through a
separately checked acquisition chain.

The factual reference is TrinityCore commit
`02245dcd245e7433e524577656177723d3e4992e`. This crate is independently authored
under GPL-3.0-or-later; no upstream implementation was imported. Referenced
upstream sources retain their TrinityCore copyright and GPL-2.0-or-later terms.

## API and bounded framing

```rust
use forever_terrain::{Chunks, Limits, Wdt};

// `bytes` are supplied by the caller; these functions have no I/O.
let chunks = Chunks::parse(bytes, Limits::default())?;
let wdt = Wdt::parse(bytes, Limits::default())?;
let tile = wdt.tile(7, 2); // WDT row 2, column 7
let references = wdt.modern_root_adts()?;
```

`Limits::default()` permits 16 MiB and 4,096 top-level chunks. A first pass
checks the complete file and count before reserving the bounded frame vector.
Every header, payload size, endpoint and count uses checked arithmetic. The
second pass exposes borrowed payloads. No partial projection escapes after a
bad extent or tail. An empty byte slice is a valid empty frame sequence, but
cannot be a WDT because its required chunks are absent.

Each frame has four tag bytes followed by a little-endian u32 payload length;
the length excludes the eight-byte frame header. Stored tags are reversed:
`REVM`, `DHPM`, `NIAM`, `DIAM` represent `MVER`, `MPHD`, `MAIN`, `MAID`.
Normalization never changes the input. `Chunk::fourcc()` and `raw_fourcc()`
expose both forms as byte arrays, without assuming UTF-8. `offset()`, `range()`
and `payload()` expose the checked location and borrowed contents.

`Chunks::all()` preserves physical order, duplicate unknown tags and opaque
payloads. `unique(tag)` reports duplicate tags explicitly. Unknown frames are
skipped only by their declared complete extent; an apparent known signature
inside them is never interpreted. Zero-length frames still consume their
eight-byte headers. Trailing one-to-seven bytes are rejected.

Sources: `src/tools/map_extractor/loadlib/loadlib.h:27-48` and
`src/tools/map_extractor/loadlib.cpp:87-97,109-164`.

## WDT structures and selection

`forever_casc::terrain::plan` adds a pure two-phase dependency planner. It
borrows the complete materialized Map source and accepts `AllMaps`, explicit
map IDs, or explicit tile coordinates. Selected maps retain source order;
coordinates use y then x. Whole maps expose all 4,096 coordinates, including
absent MAIN cells; explicit tile requests remain subset scope even if they
list every coordinate. Duplicate selected map IDs, missing maps and duplicate
requests reject explicitly. Unselected duplicate map IDs remain irrelevant.

The planner consumes one checked `Wdt` per expansion, retaining no WDT bytes.
Its association check compares the supplied FDID to the next request; the
caller still has to authenticate those bytes. A modern present tile retains
its root ADT FDID, including zero or aliases. Name mode produces an ASCII
uppercase/backslash logical CASC reference, never a local output path. Output
names use `maps/{mapId:04}_{y:02}_{x:02}.map`, preserving the full u32 ID.
Name hashing, CASC `MAX_PATH`, actual acquisition and tile-list publication
remain outside this API.

All limits are reducible: 200,000 source/selected maps, 1,000,000 explicit tile
requests, 819,200,000 sequential coordinates, 64 MiB logical planning capacity,
16 MiB per map expansion and 1,024 bytes per logical name. The total coordinate
limit does not allocate a vector containing every map's tiles. Capacity
accounting includes conservative scratch, but excludes borrowed source rows,
allocator overhead and expansions retained by the caller. Failures never
truncate a successful plan; any expansion error prevents `finish()` from
reporting complete selection. Completion here means dependency planning,
without claiming acquired files or written MAPS.

The coordinator executed all 22 focused planner tests and package Clippy
across all targets with warnings denied. Independent source review found no
functional defect. This does not establish WDT acquisition or a terrain
extraction command.

| Chunk | Payload bytes | Contract |
| --- | ---: | --- |
| MVER | 4 | Required unique little-endian u32 value 18 |
| MPHD | 32 | Optional unique eight-word header |
| MAIN | 32,768 | Required unique 64×64 array of two-word entries |
| MAID | 131,072 | Optional unique 64×64 array of eight-word entries |

Known chunk sizes are exact, including optional chunks unused by the selected
reference mode. Their order is unrestricted. The exact structures are from
`src/tools/map_extractor/wdt.h:25-86`; separate exact-size assertions appear in
`src/tools/vmap4_extractor/wdtfile.cpp:65-79` against
`src/tools/vmap4_extractor/wdtfile.h:31-65`.

MPHD exposes `flags` and seven FDIDs in order: Lgt, Occ, Fogs, Mpv, Tex, Wdl,
Pd4. MAIN exposes `flags` and its second word, named `data1` by map_extractor
and `AsyncId` by vmap4_extractor. MAID exposes all eight words:

1. `root_adt`
2. `obj0_adt`
3. `obj1_adt`
4. `tex0_adt`
5. `lod_adt`
6. `map_texture`
7. `map_texture_n`
8. `minimap_texture`

Only two flags acquire meaning in this component: MAIN bit `0x1` means the
tile is present; MPHD bit `0x200` selects file-ID references. All other bits and
words remain intact. No global-WMO or ancillary-file extraction is inferred.
`tile(x, y)` checks both coordinates against 64 and addresses the physical
array as `[y][x]`. `tiles()` visits y outermost, then x.

`modern_root_adts()` emits each present tile's coordinates and `root_adt` in
that order. Duplicate IDs are retained per coordinate, and zero/maximum u32
IDs are not silently filtered. Actual reader rejection belongs to acquisition.
If a modern WDT has any present tile, absent MAID is an error. A modern MAIN
with no present tiles may omit MAID and yields an empty reference iterator.

Missing MPHD or clear `0x200` preserves the WDT metadata but makes the modern
projection return `terrain_unsupported_reference_mode`, even if MAID exists.
There is no automatic path lookup. The C++ name mode needs Map.Directory and
uses `World\Maps\Directory\Directory_x_y.adt`; its MAPS filename uses
`maps/{mapId:04}_{y:02}_{x:02}.map`. Preserving this distinction prevents an
axis transpose. Source: `src/tools/map_extractor/System.cpp:1104-1125`.

## Deliberate stricter behavior

The C++ map loader scans forward one byte at a time when it sees an unknown
tag, reverses recognized tag bytes in place, and scans their payloads
recursively (`loadlib.cpp:132-155,169-191`). Its length checks do not prove the
full payload extent before reinterpret casts. This library requires complete
sequential framing, does not scan arbitrary payloads and rejects malformed
known structures. It is not byte-scanner compatibility for damaged files.

C++ `GetChunk` returns null for duplicate recognized chunks. Some callers
dereference required chunks without checks; duplicate MPHD also falls into
name mode. The Rust typed parser rejects duplicates for all four known chunks
and requires MVER/MAIN, avoiding that accidental fallback. These are explicit
admission differences, not claims that the native parser rejects every input
rejected here.

## Implemented conversion and acquisition boundary

The current complete Map.db2 acquisition is blocked by the recorded local
incomplete-file result. No prefix may be treated as complete. Its metadata is
FileDataID 1349477, layout `0xD43AFAC3`, physical field 21 signed32
`WdtFileDataID`, field 0 `Directory`, and field 1 `MapName`
(`DB2Metadata.h:14538-14579`, `DB2LoadInfo.h:3934-3970`). The signed FDID's bits
must follow the source's conversion to u32; inventing a positive-only filter
would change it.

The map extractor's DB2 projection has its own copy semantics
(`System.cpp:275-315`): it collects direct rows, builds an ID-to-direct-index
map, then appends copies whose source appears in that map. It never extends
that index with copied destinations, and a copied destination does not replace
the existing vector entry. Finally it drops WDT ID zero. Consequently the
generic DB2 decoder's final copied/overwritten rows cannot directly stand in
for this consumer. The implemented `forever_db2::terrain_sources::read_map`
returns ordered direct rows and copy descriptors; consuming `materialize()`
applies this exact source-only index and append/filter behavior. Its synthetic
acceptance includes collisions, chains and zero IDs. The incomplete real Map
input was not used in that acceptance. See
[the closed DB2 source contract](forever-metadata-catalog.md#typed-db2-sources-for-the-terrain-consumer).

Checked ADT conversion now includes MCNK coordinates and heights/holes, MCVT,
old MCLQ, modern MH2O and MFBO. Liquid lookups use LiquidMaterial (1132538) and
LiquidType (1371380); the pinned extractor loads LiquidObject (1308058) but
does not consult it in conversion.
Source contracts are `map_extractor/adt.h:47-133,148-258` and
`System.cpp:320-389,473-722`. The implemented pipeline serializes area, height,
liquid, holes and flight bounds under explicit options. WDT/ADT acquisition,
cross-file identity and tile-list generation remain outside that pure pipeline;
the subsequent local runner below now supplies those operations.
A WDT dependency list alone cannot establish converted tile output.

The pinned modern source declares MAPS version 10
(`src/common/Collision/Maps/MapDefines.cpp:20-24`). RustyCore at integration pin
`544721324cf0a47553caae1ea0299a0ff2917ce7` reads that magic/version, but
`crates/wow-map/src/grid_map.rs:1-10,90-147` has legacy scope and loads heights
and holes only. It does not validate the build, areas, liquids or flight
bounds. Its tile-list consumer
`crates/wow-world/src/map_manager/terrain.rs:219-285` ignores build and can
fall back to scanning tile files. Those interfaces do not by themselves
establish complete Forever output or runtime acceptance.

## Synthetic evidence scope

The crate includes original positive/negative fixtures for framing, byte
order, borrowed data, unknowns, empty frames, malformed lengths/tails,
duplicates, exact sizes, version and policy gates, coordinates, opaque flags,
file-ID ordering, zero/duplicate IDs, and the required-MAID dependency. The
coordinator runs tests and builds centrally. Later sections record scoped
synthetic C++ byte comparisons and checks through the actual RustyCore height
and hole reader. They do not establish real-client WDT acquisition or complete
Forever extraction/runtime acceptance.

## Root ADT metadata and prepacking projection

`Adt::parse(bytes, limits)` requires one four-byte MVER payload containing 18.
It does not require MHDR: the pinned `prepareLoadedData` checks only MVER
(`loadlib.cpp:87-97`). Top-level unknown frames, including MHDR, are retained
without interpretation. Every top-level MCNK is processed in physical order;
there is no sorting or deduplication by coordinates. Coordinates must lie in
the 16×16 cell grid. A file with no MCNK is accepted as metadata with zero
observed cells, without a claim of full tile coverage.

### MCNK prefix and nested admission

The initial admitted MCNK variant has a 128-byte prefix followed by complete
consecutive frames. Offsets in this table are relative to the **payload**;
add eight for offsets relative to the MCNK frame:

| Offset | Field(s) | Representation |
| --- | --- | --- |
| 0 | flags | u32 |
| 4, 8 | ix, iy | u32, each less than 16 |
| 12, 16 | nLayers, nDoodadRefs | u32 |
| 20–27 | offsMCVT/offsMCNR or HighResHoles | two u32 or eight bytes |
| 28, 32 | offsMCLY, offsMCRF | u32 |
| 36, 40 | offsMCAL, sizeMCAL | u32 |
| 44, 48 | offsMCSH, sizeMCSH | u32 |
| 52, 56, 60 | areaid, nMapObjRefs, holes | u32 |
| 64, 66 | s[2] | u16 |
| 68, 72, 76 | data1, data2, data3 | u32 |
| 80, 84 | predTex, nEffectDoodad | u32 |
| 88, 92 | offsMCSE, nSndEmitters | u32 |
| 96, 100 | offsMCLQ, sizeMCLQ | u32 |
| 104, 108, 112 | zpos, xpos, ypos | raw f32 bits |
| 116, 120, 124 | offsMCCV, props, effectId | u32 |

Source: `src/tools/map_extractor/adt.h:87-132`. `McnkHeader::raw()` preserves
every prefix byte; accessors interpret the fields used in this increment.
The vertical base is the source's **ypos at payload offset 112**, despite the
coordinate naming. Non-finite bits in unused positions remain opaque.

Only the suffix after the prefix is passed to checked nested framing. The
top-level and direct nested frames share one aggregate `max_chunks` budget;
default 4,096. There is no recursive payload scan. `Cell::children()` offsets
are relative to that suffix; `Cell::offset()` is the absolute outer frame
offset. Metadata offsets such as offsMCVT/offsMCNR are preserved, never
followed as pointers. The high-resolution-hole flag changes the interpretation
of their shared eight-byte storage.

The pin contains no reviewed MCNR size/padding grammar: its sole MCNR field is
`adt.h:104`, and MCNR is absent from the scanner's interesting-tag list
(`loadlib.cpp:109-120`). The scanner walks through unrecognized bytes
(`loadlib.cpp:169-191`). This library does not invent a universal replacement
for that behavior. MCNR is an opaque complete frame, including any bytes
declared inside its payload. A synthetic 435-byte MCNR followed by thirteen
undeclared padding bytes is rejected with `terrain_unsupported_nested_layout`,
as are other unframed gaps or tails. No automatic thirteen-byte skip occurs.
This explicit initial shape may reject files the native scanner accepts.

### MCVT and finite height projection

A direct nested MCVT must be unique and contain exactly 580 bytes: 145 f32
values, comprising 81 corner and 64 center samples (`adt.h:47-54`). Raw bits
are available through `Mcvt::bits()`, `value_bits()` and `raw()`. Missing MCVT
is valid and gives the cell its base height alone. Duplicate MCVT is rejected;
the C++ `GetSubChunk` instead returns null and thus uses base height. That is
an explicit stricter admission choice (`loadlib.cpp:194-200`).

`Adt::project_heights()` returns a `HeightProjection` before clamping, extrema,
flatness selection or integer packing. Its private arrays have public bounded
accessors:

- `area_ids()`: 256 u16 values in `[iy][ix]` order; the raw u32 area ID is
  truncated exactly as the C++ assignment, while the header retains all bits.
- `v9_bits()`: 129×129 u32 float bits; `v9_at(x,y)` uses `y*129+x`.
- `v8_bits()`: 128×128 u32 float bits; `v8_at(x,y)` uses `y*128+x`.
- `hole_bytes()`: 16×16×8 bytes in `[iy][ix][row]` order;
  `holes_at(ix,iy)` returns the eight rows.
- `has_holes()`, `visited_cells()`, `unique_cells()` and `coverage()` retain
  the distinction between no observed cell and an observed zero-height cell.

All arrays begin at zero. For each MCNK, local corner `(x,y)` reads MCVT sample
`17*y+x`; local center `(x,y)` reads `17*y+9+x`. The destination starts at
`(ix*8,iy*8)`. The value is one f32 addition of ypos and the relative sample.
Without MCVT, assignment preserves the base bits, including signed zero.
Later physical cells overwrite heights and areas. This includes shared V9
edges between different cells and repeated coordinates. Sources:
`System.cpp:482-509,528-575`.

Metadata accepts every raw float representation. Projection rejects a used
non-finite base, any non-finite MCVT sample, or a non-finite addition result
with `terrain_nonfinite_height`. Finite inputs include subnormals and signed
zero. This explicit projection policy is stricter than the C++ source, which
does not test finiteness. It avoids a portable NaN payload claim and returns
no partial arrays after rejection. The separate height-section stage below
implements the packing facts at `System.cpp:759-864`.

### Hole and duplicate semantics

When MCNK flag `0x10000` is clear, only the low sixteen bits of `holes` are
used. Each low-resolution bit `(row/2)*4+column/2` expands to a 2×2 block in
the eight-row mask, using OR with existing bytes. A zero low-resolution mask
does not clear a previous mask. When `0x10000` is set, the eight union bytes
replace the existing mask. The source's `hasHoles` flag is sticky: a nonzero
earlier mask leaves it true even when a later high-resolution duplicate
replaces the final mask with zero. The projection preserves these operations
instead of recomputing the boolean from final bytes. Sources:
`System.cpp:449-461,496-501,632-644`.

Synthetic cases cover base-only cells, all 256 coordinates, physical-order
duplicates and shared edges, low-resolution OR, high-resolution replacement,
sticky holes, exact float bits and finite rejection, aggregate budgets, strict
nested boundaries, and the MCNR padding limitation. The coordinator accepted
43 terrain tests and an independent C++ comparison of 47 original synthetic
projections, plus three non-finite rejections. The redacted evidence is
`terrain-projection-20261011T011040Z-08a187f0.json`. This proves the recorded
synthetic projection scope. No ADT
live acquisition, liquid interpretation, MAPS writer or runtime acceptance is
claimed by this component.

## Height-section serialization

Authored 2026-10-11 from the pinned format and packing facts. The pure
`height` module emits one **MHGT** section. It does not emit a MAPS container,
omit liquids from an existing tile, select a client build or write files.
Callers must retain the separate area, hole, liquid and discovery obligations.

```rust
use forever_terrain::height::{FlightBounds, HeightSection, Options, MAX_SECTION_BYTES};

let projection = adt.project_heights()?;
let flight = adt.chunks().unique(*b"MFBO")?
    .map(|chunk| FlightBounds::parse(chunk.payload()))
    .transpose()?;
let section = HeightSection::from_projection(
    &projection, Options::default(), flight.as_ref(), MAX_SECTION_BYTES,
)?;
let mhgt_bytes = section.bytes();
```

`HeightSection::from_bits(v9_bits, v8_bits, options, flight, max_bytes)` is the
equivalent independently checked entry point for complete projected arrays:
exactly 16,641 V9 values and 16,384 V8 values in source `[y][x]` order. Both
entry points reject non-finite input, including values that clamping or flat
selection might otherwise discard. They do not modify input. Coverage remains
the projection's separate metadata; zero-filled unvisited cells are serialized
according to the source without implying a complete observed ADT tile.

`Options { pack_integers: bool }`, default true, models the effective `-f`
boolean. The executable uses `atoi(...) != 0` at `System.cpp:208-213`.
Although its comments mention `-h`, that case exits through `Usage` at
lines 253-255. The executable's minimum-height limit is therefore fixed at
−2000; this API does not expose a fictitious supported clamp switch. Flat and
integer thresholds remain the pinned constants, not caller-defined tolerances.

### Extrema, clamping and selection

The scan visits **V8 first, then V9**, with minimum initialized to +20000 and
maximum to −20000. Strict `<` and `>` comparisons preserve the first equal
value's bits, including signed zero. The sentinels remain observable: uniform
samples at 20001 produce minimum 20000 and maximum 20001, so the packed result
is not flat. Replacing these sentinels with infinities or a first-sample
initialization would change the source output.

Every height below −2000 is replaced by −2000 for serialization, and extrema
below that bound are raised to it. Header extrema are the resulting f32 bits.
The packing selection uses the f32 difference `maximum - minimum`:

| Condition, in priority order | Encoding | Flag | Sample bytes |
| --- | --- | ---: | ---: |
| Equal extrema | Flat | 1 | 0 |
| Packing enabled and difference < 0.005 | Flat | 1 | 0 |
| Packing enabled and difference < 2 | Uint8 | 4 | 33,025 |
| Packing enabled and difference < 2048 | Uint16 | 2 | 66,050 |
| Otherwise | Float32 | 0 | 132,100 |

All thresholds are strict; equality advances to the next representation.
Packing disabled still removes a truly constant surface. A nearly flat packed
header retains both extrema even though no samples follow it. Quantization
uses f32 operations in order: `step = 255 / difference` or `65535 / difference`,
then `(height - minimum) * step`, then `+ 0.5`, then truncation to the unsigned
integer. It uses neither f64 intermediates nor fused multiply-add. A checked
conversion rejects non-finite or out-of-range results instead of relying on
Rust's saturating casts or C++ undefined conversion behavior. This is an
explicit stricter safety boundary.

### Exact bytes and flight bounds

The 16-byte header is `MHGT`, flags as u32 little endian, minimum f32 little
endian, maximum f32 little endian. Non-flat samples always follow as **V9 then
V8**, each in row-major order. Float output preserves finite sample bits after
the stated clamp. This byte order is explicit in Rust and matches the pinned
x86 reference; it does not assume the host's native endian order.

`FlightBounds::parse(payload)` requires the exact 36-byte MFBO payload, excluding
the ADT frame. It reads nine signed i16 maxima followed by nine signed i16 minima,
each a row-major 3×3 array. Public fixed-size `max` and `min` arrays can also be
supplied directly. All i16 values and inverted pairs are preserved; the writer
does not invent ordering constraints. If supplied, flag 8 is set and those
36 bytes follow the sample data **even for Flat**. If absent, no flight flag or
tail is emitted. The caller supplies this choice explicitly. Duplicate MFBO is
rejected by the checked `unique` lookup in the example; the native scanner
returns null on duplicates, an already documented stricter admission difference.

Sources: `adt.h:299-313`, `System.cpp:716-722,760-864,987-1012`, and
`src/common/Collision/Maps/MapDefines.h`/`MapDefines.cpp:20-24`. The module stores
bounds only. The C++ reader constructs eight planes from the minima; no plane
construction, minimum-flight-height query or RustyCore flight acceptance is
claimed here.

`HeightSection` exposes `bytes`, `into_bytes`, `encoding`, `flags`,
`min_height_bits` and `max_height_bits`. The caller's `max_bytes` applies to the
entire section before a fallible exact reservation. The maximum possible
section is 132,152 bytes: 16-byte header, 132,100 bytes of float samples and
36 bytes of flight bounds. Fixed dimensions bound all scans and arithmetic;
input-derived lengths never select a larger allocation. Errors use stable
`height_*` codes and do not return partial sections.

Original synthetic tests cover all four representations, exact payload bytes,
f32 values immediately below/at/above every threshold, clamp boundaries,
sentinels, signed zero, subnormals, finite extremes, MFBO signedness/order/tails,
sample dimensions, non-finite rejection, input immutability and output limits.
The coordinator runs the new tests and independent C++ byte comparison; their
acceptance is recorded separately from the prior projection evidence.

### Container composition and reader scope

The MAPS v10 container has a 44-byte header with build and offset/size pairs
for AREA, MHGT, MLIQ and holes. AREA is an eight-byte header plus either no
payload for a uniform area or 256 u16 IDs. Holes, when the source's sticky
`hasHoles` is set, are 2,048 raw bytes. MLIQ has its own 16-byte header, type
and visibility projection, rectangle and flat-height rules. The liquid
dependencies, projection, section and integrated conversion implementations
are described below with their separate evidence. Flight bounds belong inside
MHGT, not a fifth container section.

RustyCore's reviewed `wow-map::GridMap` recognizes MAPS v10 and reconstructs
Flat, Uint16, Uint8 and Float32 heights, plus hole queries. It does not validate
declared height-section sizes, build policy, areas, liquids, flight bounds or
all flag combinations. The scoped GridMap query comparison recorded below
covers synthetic containers, integer reconstruction, asymmetric axes, holes
and flat surfaces against an independent kernel of the modern pinned C++
algorithm and the actual RustyCore reader. Merely loading an MHGT section
inside that reader cannot establish full Forever MAPS compatibility.

### MHGT synthetic oracle acceptance

The coordinator accepted the isolated height-section kernel in
`terrain-section-20261011T012203Z-b532194b.json`: 126 positive cases matched
an independently authored C++ oracle byte for byte, and eight negative cases
were rejected. The terrain suite at that snapshot contained 65 passing tests.
This evidence covers the original synthetic C++ prepacking/section kernel and
finite inputs; it does not establish full ADT acquisition, a complete MAPS
container, liquid consumption or Forever runtime acceptance.

## MCLQ/MH2O liquid projection

`forever_terrain::liquid` implements borrowed metadata views and a separate
bounded projection. It is original Rust code from facts at the pinned commit,
principally `src/tools/map_extractor/adt.h:60-81,148-296` and
`System.cpp:430-446,577-630,647-713,1056-1080`. No upstream implementation is
imported. There is no I/O, DB2 acquisition, MLIQ packing or runtime query here.
The existing ADT parser and height projection keep their previous behavior.

### API and output dimensions

`project_liquids(&Adt, &impl LiquidLookup, ignore_deep_water)` returns a
`LiquidProjection` only after every selected input has validated. All getters
use conventional row-major `[y][x]` order:

| Getter | Elements | Meaning |
| --- | ---: | --- |
| `show()` | 16,384 u8 | 128×128 visible tiles, exactly zero or one |
| `entries()` | 256 u16 | 16×16 liquid type IDs |
| `flags()` | 256 u8 | 16×16 accumulated liquid flags |
| `height_bits()` | 16,641 u32 | 129×129 raw finite f32 height bits |

The four arrays start at zero. Fixed dimensions bound allocation and indexing;
allocations use checked reservation. The supplied `Adt` has already enforced its
file/chunk policy, normally 16 MiB and 4096 aggregate top/nested frames. Nested
MCNR padding remains subject to the previously documented ADT framing limit.

`LiquidLookup` supplies deterministic already validated `LiquidType` and
`LiquidMaterial` values by u32 ID. Public fields are `sound_bank: u8`,
`material_id: u8`, `flags: u32` and `vertex_format: i8`. No LiquidObject lookup
is requested: although the source loads that table, this converter never uses
it. The traits do not load files, reinterpret DB2 schemas, or assert that a
caller has populated its tables correctly.

### Borrowed views and checked extents

`Mclq::parse(payload)` checks the entire declared 804-byte structure prefix:
height bounds at offsets 0/4, 81 `(light:u32,height:f32)` pairs at offset 8 with
stride 8, 64 tile flags at offset 656, and 84 opaque bytes at offset 720. Extra
bytes inside a completely framed payload remain opaque. Methods expose raw
bytes, bound bits, individual light/height/flag values and the opaque tail.
Requiring the full declared prefix is stricter than merely bounding the 720
bytes that this particular C++ consumer reads.

`Mh2o::parse(payload)` checks the 3072-byte fixed index. Each of 256 row-major
entries is `(instance_offset:u32, used:u32, attributes_offset:u32)`. Its `cell`
accessor uses `(x,y)` order. A `LiquidCell` exposes those values and checked
`first_instance`/`attributes` accessors. Every offset is relative to the MH2O
**payload**, and is checked against that chunk rather than later file bytes.
Inactive entries do not follow offsets. `used != 0` selects only the first
24-byte instance; it does not allocate or interpret further declared layers.
This is the exact pinned converter selection, not a complete multilayer parser.

An instance stores LiquidType/LVF u16 at offsets 0/2, two raw f32 bounds at 4/8,
rectangle bytes at 12–15, and bitmap/vertex offsets at 16/20. Its raw methods do
not follow pointers. Referenced spans validate on access, preserving the
source's filter order. Offsets may alias immutable payload bytes; no unsupported
non-overlap or alignment assumption is added. A nonzero exists offset requires
all eight bitmap bytes. Zero means all bits set. Attributes require 16 bytes
when present; a used entry with no attributes has both masks set to `u64::MAX`,
and an unused entry has both zero. Bounds and Fishable are retained but unused
by projection.

### Projection semantics

MCLQ is considered only when `MCNK.sizeMCLQ > 8`, and only a unique child is
accepted. Missing MCLQ is allowed. Repeated selected MCLQ or MH2O chunks fail
explicitly; the C++ scanner instead treats non-unique lookups as absent. The
Rust module does not scan or resynchronize arbitrary payload bytes.

MCLQ runs in physical MCNK order. A tile is visible unless its **entire flag
byte** equals `0x0F`; `0x8F` therefore remains visible. Visible bit 7 adds
DarkWater unless disabled. MCNK bits 2/3/4 successively assign IDs 1/2/3 and OR
Water/Ocean/Magma. The last matching ID wins. All 81 heights are assigned even
when every tile is invisible. Repeated cells accumulate visibility/flags and
replace affected heights/IDs; shared vertices follow last physical writes.

MH2O runs after every MCLQ, regardless of its top-level frame position. It visits
cells by row, then column. Raw LVF below 42 is direct. At 42 or above, raw
rectangle fields are ignored and the rectangle becomes `(0,0,8,8)`; original
LiquidType 2 resolves to Depth, otherwise original LiquidType → Material.LVF.
A resolved Depth format forces the effective type ID to 2. The effective ID is
written **before** skipping an unknown LiquidType or a material with
`VisualOnly=0x2`. Those skipped cases leave preceding MCLQ flags, visibility and
heights untouched and do not inspect unused attributes/bitmap/vertex offsets.
A missing material does not skip an explicitly specified format.

For admitted layers, bitmap bits are consumed LSB first over width, then height.
Visibility is ORed and never cleared. SoundBank 0/1/2/3 adds Water/Ocean/Magma/Slime
(bits 1/2/4/8); other banks add no flag. Ocean adds DarkWater (16) when the whole
Deep mask is nonzero and the option permits it. Deep is not intersected with the
visible bitmap. Rectangles must fit the cell's 8×8 extent; zero dimensions and
edge coordinates are valid, and a zero-size rectangle still has one height
vertex. This checked rectangle policy is stricter than the C++ pointer arithmetic.

For `N=(width+1)*(height+1) <= 81`, formats 0/1/3 read contiguous f32 heights;
formats 4/5 read the f32 at offset 4 of each eight-byte vertex. Format 2 and zero
vertex offsets produce positive-zero heights. The source does not use the
stored min/max levels as a fallback. Depth/texture planes are opaque to this
consumer and are not claimed decoded. Formats outside 0–5 are rejected after
selection filters, including an unresolved indirect format; the source instead
returns zero for unknown formats. This restriction is explicit even when an
unknown format has a zero vertex offset.

Every used MCLQ/MH2O height must be finite. No addition, clamp or normalization
is performed: signed zero, subnormals and finite extremes retain their bits.
Unused metadata float bits are retained, including NaN/Inf. A failure returns
only a static `liquid_*` code and no partial projection.

`ignore_deep_water(map_id, tile_y, tile_x)` deliberately takes **row before
column**, matching `System.cpp:1117`, which passes those values into the source
helper's misleadingly named `(x,y)` parameters. Map 0 suppresses fatigue at
rows39–40/columns24–26 and rows41–46/columns18–26; map1 at row43/columns39–40.

### Dependencies and remaining gates

LiquidMaterial (1132538, layout98E5D7AA) projects signed32 Flags and signed8 LVF;
LiquidType (1371380, layoutD1ECEEC9) projects physical fields3/14 as unsigned8
SoundBank/MaterialID. LiquidObject (1308058, layoutCB0D39E8) is loaded but unused.
All have external IDs and no parent. The closed DB2 terrain readers and their
`materialize()` methods implement the map extractor's **specific** copy loop:
direct assignment followed by ordered `map[new] = map[source]`, which
creates a default source entry when missing and permits source zero, chains and
overwrites (`System.cpp:328-340,353-364,377-389`). Generic final DB2 rows do not
establish this consumer contract. The conversion API consumes these already
materialized values through a borrowed lookup; acquisition remains separate.

MLIQ packing is separate. The pin computes visible bounds/extrema using only
128×128 top-left vertices, fills invisible vertices with −2000, and includes an
extra border vertex in the emitted rectangle. Those transformations do not
occur here. RustyCore `wow-map/src/grid_map.rs:8-10,90-147` and
`terrain.rs:11-17` currently exclude liquid parsing/queries; projection parity
cannot establish runtime compatibility.

Synthetic tests cover all six formats, direct/indirect type resolution, exact
filter ordering, zero/inactive offsets, the first-layer rule, explicit/default
bitmaps and attributes, source flag combinations, repeated/shared cells,
rectangle boundaries, truncated/out-of-range spans, opaque metadata/tails,
finite-bit preservation and rejection, and deep-water coordinate boundaries.
The coordinator's `terrain-liquid-tests` and `terrain-liquid-clippy` logs record
91 passing terrain tests (including 26 liquid cases) and a clean Clippy check.
An independent source review against the pin found no concrete defects.
The report `terrain-liquid-20261011T013406Z-d88b629b.json` records 99 positive
synthetic projections matching the independently authored C++ kernel byte for
byte, ten malformed cases rejected by both programs, nine explicit Rust policy
rejections, and two exclusive-output checks. The Rust-policy differences cover
the documented stricter LVF/finite admission; they are not native rejection
parity. This evidence does not establish client acquisition, MLIQ serialization
or liquid runtime acceptance.

## MLIQ section serialization

The pure `liquid_section` module serializes one projected liquid section, or
returns explicit omission. It is original code from the pinned
`System.cpp:103-112,208-213,253-255,868-955,1014-1026`,
`src/common/Collision/Maps/MapDefines.h:87-130` and `MapDefines.cpp:24`.
It performs no I/O, dependency loading, MAPS offset assignment or runtime query.

```rust
use forever_terrain::liquid_section::{LiquidSection, Options, MAX_SECTION_BYTES};

let section = LiquidSection::from_projection(
    &liquid_projection, Options::default(), MAX_SECTION_BYTES,
)?;
// None means the enclosing MAPS file must omit this section.
if let Some(section) = section {
    let mliq_bytes = section.bytes();
}
```

`LiquidSection::from_arrays(show, entries, flags, height_bits, options, max_bytes)`
is the equivalent independent entry point. Exact lengths are 16,384 visibility
bytes, 256 u16 entries, 256 flag bytes and 16,641 height-bit values. Visibility
must be zero or one; every height must be finite, including invisible, border
and otherwise omitted values. These explicit safety checks precede the omission
decision. All u8 liquid-flag bits are retained opaquely. Input arrays are never
modified or cloned for mutation. The result exposes `bytes`/`into_bytes` and
getters for header flags, liquid flags/type, x/y offsets, width/height and level
bits. Errors expose fixed `liquid_section_*` codes only.

`Options { pack_integers: bool }`, default true, models the same effective `-f`
boolean as height packing. Liquid samples remain f32; this switch controls only
the near-flat omission rule. The invisible height is fixed at −2000, and the
flatness threshold at 0.001. The pinned executable's `-h` branch calls Usage;
there is no configurable minimum-height claim.

### Omission, extrema and rectangle

The source first compares all 256 `(entry, flag)` pairs. When every pair is
equal and the common flag is zero, the result is `None`, even if the common ID
is nonzero or some tiles are visible. No section bytes are allocated; a zero
output budget permits this result. Otherwise a complete section is required.

Extrema start at minimum +20000 and maximum −20000, then scan the 128×128
top-left height samples by row and column. Visible samples update both with
strict comparisons. Invisible samples become −2000 when serialized and lower
the minimum to at most −2000, without updating the maximum. Visible values
below −2000 remain unchanged. The outer row/column at coordinate 128 contributes
neither extrema nor visibility and retains its raw finite bits if emitted.
Invisible samples outside the eventual crop still affect the minimum. The
sentinels and first-equal-value signed-zero behavior are preserved.

Visible cells determine the rectangle, with one extra row and column of border
vertices. `NoHeight` is set when visibility is empty, extrema are equal, or
packing is enabled and the f32 difference is strictly below 0.001. A finite
extrema difference that overflows to positive infinity simply fails that
threshold comparison; no non-finite sample or header value is emitted.

When a section is present but visibility is empty, the source keeps offsets
`(128,128)` and converts the signed extent `0 - 128 + 2` to u8, giving width
and height **130**. This intentionally preserved header has `NoHeight`, so it
never indexes or emits that rectangle. A verifier must not apply the ordinary
129×129 crop bound to this no-height case.

### Exact byte layout and limits

The section starts with the sixteen-byte little-endian header:

| Offset | Field | Representation |
| --- | --- | --- |
| 0 | magic | `MLIQ` |
| 4 | section flags | u8: NoType=1, NoHeight=2 |
| 5 | common liquid flags | u8 |
| 6 | common liquid type | u16 |
| 8, 9 | offset x, y | u8 each |
| 10, 11 | width, height | u8 each |
| 12 | minimum liquid level | f32 bits |

Uniform `(entry, flag)` pairs set `NoType` and populate the common header
fields. Otherwise those two fields are zero, followed by all 256 entries as
u16 LE and then all 256 flag bytes, both in row-major order. If `NoHeight` is
clear, the cropped f32 heights follow in row-major order, including the extra
border. Invisible interior values use −2000; outer-border values stay raw.
There is no integer sample encoding or extra alignment padding.

The maximum section is **67,348 bytes**: sixteen header bytes, 768 type/flag
bytes and 129×129×4 height bytes. Checked size arithmetic and fallible exact
reservation enforce both that ceiling and the caller's possibly lower budget.
No partial section is returned on failure. This module emits neither a MAPS
header nor any other section, and provides no byte parser or standalone file
verifier.

Synthetic cases cover omission independent of visibility/ID, uniform and full
type records, opaque flags, empty visibility, asymmetric and border crops,
invisible-height effects, below-floor values, sentinels, signed zero, finite
extremes, strict threshold neighbors and the packing switch, exact budgets,
invalid shapes/visibility/floats, input immutability and the projection adapter.
The coordinator recorded 113 passing terrain tests at the MLIQ snapshot,
including 22 MLIQ cases, and clean Clippy. An independent source review found
no concrete defects. The report
`terrain-liquid-section-20261011T014336Z-5daf200e.json` records 311 positive
synthetic sections matching the independently authored C++ kernel byte for
byte and ten negative cases rejected. The positives include 198 cases derived
from the 99 previously accepted liquid projections, each with both packing
modes. All recorded guards passed. This is synthetic section evidence; no real
client extraction, full C++ ADT parser or runtime acceptance is claimed at
this stage.

## AREA and MAPS container

The `maps` module assembles the already typed sections into one bounded
MAPS v10 byte vector and verifies complete container bytes with borrowed views.
It is original code from `MapDefines.h:38-130`, `MapDefines.cpp:20-24` and
`System.cpp:725-757,958-1031` at the same pin. This supplies format assembly;
it does not acquire or discover tiles, prove ADT coverage, write files or load
a server runtime.

```rust
use forever_terrain::maps::{self, AreaSection, MapSections, MAX_MAP_BYTES};

let area = AreaSection::from_projection(&height_projection);
let bytes = maps::assemble(70205, MapSections {
    area: &area,
    height: &height_section,
    liquid: liquid_section.as_ref(),
    holes: optional_hole_array,
}, MAX_MAP_BYTES)?;
let checked = maps::verify(&bytes, 70205, MAX_MAP_BYTES)?;
```

`AreaSection::from_ids(&[u16;256])` is the equivalent independent entry point.
It stores a fixed 520-byte backing array and exposes only its active bytes.
Uniform area IDs produce `AREA`, u16 flags1 and the common u16 ID: eight bytes.
Nonuniform IDs produce flags0, gridArea0 and all 256 u16 LE IDs in `[y][x]`
order: 520 bytes. The projection adapter retains its existing truncation of
raw area IDs to u16, physical cell overwrite order and zero initialization.

`MapSections` borrows a mandatory AREA and MHGT, optional MLIQ, and optional
`&[u8;2048]` holes. Hole presence is explicit. `Some([0;2048])` is preserved
because the source's sticky `hasHoles` can remain true after a later cell clears
its final mask. Absence must not be inferred from the final hole bytes.

### Container bytes and bounds

The forty-four-byte header contains these little-endian u32 fields after its
literal four-byte tag:

| Offset | Field |
| --- | --- |
| 0 | `MAPS` |
| 4 | version 10 |
| 8 | explicit build |
| 12, 16 | AREA offset, byte size |
| 20, 24 | MHGT offset, byte size |
| 28, 32 | MLIQ offset, byte size, or zero/zero |
| 36, 40 | holes offset, byte size, or zero/zero |

AREA begins at44, followed immediately by MHGT, then MLIQ if present, then
holes if present. There is no alignment padding or trailing data. A uint8
height section has odd size; a uint16 section can leave the next offset at two
modulo four. All reads use bytewise LE decoding and admit those source offsets.
MFBO flight bounds remain the optional thirty-six-byte tail **inside MHGT**.

Both APIs require an explicit build70170 or70205. That is the authorized local
policy; the native format itself stores an unrestricted u32. Verification
requires exact equality with the requested build, with no guessing. A matching
tag does not authenticate the originating client or acquisition history.

The minimum assembled container is 68 bytes: header44, uniform AREA8 and flat
MHGT16. This can represent an empty projection without proving observed tile
coverage. The maximum is **202,112 bytes**: header44 +AREA520 +MHGT132152
+MLIQ67348 +holes2048. Checked offset/size arithmetic, u32 conversions and
fallible exact reservation enforce this bound and the caller's lower cap before
allocation. The assembler preserves section bytes and checks the completed
container before returning it.

### Borrowed strict verification

`verify(bytes, expected_build, max_bytes)` makes no heap allocation, follows no
external reference and changes no input. It checks magic/version/build, exact
contiguous section order, offset/size pairs, all selected section extents and
exact EOF. Mandatory sections cannot be absent; optional sections require
zero/zero when absent. Gaps, overlaps, padding, inconsistent absent pairs,
unknown flags, malformed section sizes and trailing bytes fail with fixed
`maps_*` errors.

The returned `MapView` exposes the original bytes, build, AREA/MHGT views,
optional MLIQ view and an optional borrowed 2048-byte hole array. `AreaView`
provides bounded ID lookup. `HeightView` exposes encoding, flags, extrema bits,
the encoded V9/V8 byte slices and the optional raw flight array. `LiquidView`
exposes header facts, optional full entry/flag bytes and cropped height bytes.
Raw views do not implement Debug. They decode no terrain queries or planes.

AREA verification requires the canonical eight/520-byte representation: full
arrays must be nonuniform and their gridArea field zero. MHGT permits exactly
Flat, Uint8, Uint16 or Float32, optionally combined with flight flag8. It checks
the exact sample count and optional tail, finite ordered header extrema, and
finite Float32 samples. Flight i16 values, including inverted bounds, are
preserved. Flat header extrema may differ because the near-flat rule omits
samples; verification does not recompute the source packing decision or claim
that all encoded samples reproduce its original extrema.

MLIQ accepts only its two defined section flags while preserving liquid-type
flag bytes opaquely. Full type arrays require zero common header fields and
nonuniform pairs; a uniform header with zero liquid flags is rejected because
the writer would omit it. All stored float values are finite. With height data,
the crop starts below128, dimensions are at least2 and endpoints fit129. The
same ordinary rectangle is valid with NoHeight; the additional canonical empty
case is `(128,128,130,130)`, NoHeight and level−2000. Height payload sizes must
match their exact crop. No visibility bitmap is stored, so the verifier cannot
reconstruct the full projection or reestablish its original flatness decision.

These checks are deliberately stronger than the reviewed readers. RustyCore's
`wow-map::GridMap` ignores build, declared section sizes, AREA/MLIQ and flight
tails, and does not reject every unknown flag combination. Its flat-height path
also returns before hole queries; parsing still loads declared holes. The separate public
`wow_world::map_manager::terrain_grid_area_id_for_position_like_cpp` reads AREA
but ignores build/area size and unknown flags. Neither reader's successful
parse alone establishes full Forever terrain compatibility. No integration
behavior is changed here.

Synthetic coverage includes exact AREA/header bytes, both builds, all height
encodings and flight tails, unaligned next offsets, optional liquid/holes,
present zero holes, opaque liquid flags and empty rectangles, maximum caps,
truncation/gaps/overlaps/absent pairs/tails, malformed tags/flags/sizes,
noncanonical AREA/type arrays, non-finite floats and immutable input. The
coordinator's workspace validation recorded 134 passing terrain tests and clean
Clippy. The independent MAPS source review found no concrete defects.
The GridMap query evidence below first used independently constructed
containers. The later production-assembler comparison records its distinct
full-container acceptance separately.

### Scoped GridMap reader evidence

The local report `terrain-grid-20261011T014347Z-e6d7456e.json` records 378
positive synthetic MAPS wrappers: 126 previously C++-exact MHGT sections, each
with holes absent, present and zero, or present with a synthetic pattern.
For each wrapper, 143 height queries produced identical raw u32 result bits
between the actual public RustyCore `GridMap::parse`/`get_height` API and the
independently authored pinned-C++ algorithm kernel: **54,054 exact results**.
The integration source was included unchanged, with SHA-256
`20325e8559528b5c8d4da6ae8b90a0924c9ea6a692f9433a3b57e726f6c24dbb`.
The report records its source-header hash snapshot and passing source, binary
and prior-input guards.

Six malformed cases were rejected by both readers. A seventh intentionally
truncated MFBO case was accepted by RustyCore, whose height reader does not
consume the flight tail, and rejected by the C++ kernel, which requires those
36 bytes. The strict production MAPS verifier rejected that case as well.
It ran for every wrapper, accepting all positives with exit0 and rejecting all
negative cases with exit4. This documents the reader difference rather than
claiming identical malformed-input admission.

These wrappers were assembled independently in Python around prior accepted
MHGT bytes. The production Rust MAPS assembler was exercised by unit tests,
not by this differential run. The C++ side is an original comparison kernel,
not the complete upstream GridMap or ADT parser. The evidence covers synthetic
height queries and hole behavior only: it includes no real client input,
higher-level tile-file selection, AREA or liquid queries, flight-plane queries,
or complete Forever terrain/runtime acceptance.

### Scoped production MAPS assembly evidence

The report `terrain-map-20261011T014623Z-650148fb.json` records **380 positive
containers** emitted by the production Rust assembler and matched byte for
byte against an independent Python layout using previously C++-accepted
height/liquid section bytes. The cases comprise 126 height cases with three
AREA/liquid/hole configurations each, plus the minimum 68-byte container and
one additional full-liquid case. Both supported build tags, odd section
offsets, full area arrays and present zero/pattern hole arrays are represented.
The 70170 cases use synthetic header tags; they do not establish acceptance of
a build70170 client.

The strict verifier accepted every positive container, six negative cases
were rejected, and **54,340** queries through the unchanged actual RustyCore
GridMap API matched the independent C++ height kernel exactly. All recorded
source, binary and input guards passed. This run exercises the production
assembler, unlike the earlier independent-wrapper GridMap run.

The retained earlier report
`terrain-map-20261011T014539Z-b4dc62e0.json` failed because its supposedly
68-byte minimal fixture selected an MHGT section containing the extra
36-byte flight tail. The fixture was corrected to flat MHGT without flight
bounds, and the entire run was repeated successfully. No production source
defect or code fix was involved; the failed report remains part of the record.

This acceptance composes synthetic section bytes into MAPS and checks scoped
height queries. It does not yet exercise a pure ADT-to-MAPS conversion,
acquisition of real terrain, the complete C++ ADT parser, higher-level tile-file
selection, AREA/liquid runtime queries or full Forever terrain compatibility.

## Conversion of one supplied ADT

`conversion::convert_adt` composes the production parser, projections and
serializers into one operation. It returns a complete, structurally verified
MAPS object or an error; no partial bytes or statistics escape.

```rust
use forever_terrain::conversion::{convert_adt, Context, Limits};

// Complete ADT bytes and a deterministic LiquidLookup are supplied by the caller.
let tile = convert_adt(
    adt_bytes,
    lookup,
    Context {
        map_id: 0,
        tile_y: 39, // WDT row
        tile_x: 24, // WDT column
        build: 70205,
        pack: true,
    },
    Limits::default(),
)?;
let complete_maps_bytes = tile.bytes();
let coverage = &tile.stats().coverage;
// tile.into_bytes() transfers ownership of the complete output.
```

The context requires both tile coordinates below 64 and build 70170 or 70205.
These checks and limit validation precede ADT parsing. The map ID is a u32
without an invented map whitelist. Context declares the output build and
deep-water selection; it does not authenticate the input's map or build.
The one `pack` boolean is passed to both height and liquid serializers, keeping
the pinned executable's shared option and each section's distinct thresholds.

| Conversion limit | Default and maximum |
| --- | ---: |
| Input bytes | 16 MiB |
| Top-level and direct child frames together | 4,096 |
| Complete MAPS output bytes | 202,112 |

`conversion::Limits` exposes `max_input_bytes`, `max_chunks` and
`max_output_bytes`. They may only reduce these ceilings. A larger requested
limit returns `conversion_invalid_limits` even for an empty input. Zero is a
valid reduction and rejects required data. Fixed-size projections retain their
existing bounds; section allocations and the final container allocation are
checked separately. The output cap is not a claim about total process memory.

### Composition and source semantics

The operation parses all admitted ADT frames, selects optional MFBO, projects
heights/areas/holes, constructs AREA and MHGT, projects liquids, constructs
optional MLIQ, and assembles MAPS. The assembler runs its structural verifier
before returning. Input bytes and the borrowed lookup are not modified.

- **MFBO:** selection uses the unique top-level chunk tag, as in
  `System.cpp:716-723`; MHDR flags and offsets are not consulted. MHDR remains
  an opaque framed chunk and is not required. The exact 36-byte MFBO payload
  contains nine signed-i16 maxima followed by nine minima. Its presence adds
  the flight flag and tail inside MHGT, including flat MHGT. Duplicate MFBO or
  a payload of another size retains the components' stricter rejection policy.
- **Cell coverage:** no requirement for 256 MCNK records is invented. Missing
  cells retain zeros, and a present cell without MCVT uses its base height.
  Repeated coordinates and shared-edge writes preserve physical input order.
  Coverage is reported separately from completeness of the acquired file.
- **Holes:** the source's persistent `has_holes` value controls section presence.
  A later overwrite that clears every bit still emits a present all-zero hole
  section when that value was set earlier.
- **Liquids:** `ignore_deep_water(map_id, tile_y, tile_x)` is computed internally
  with row first, matching `System.cpp:1117`. Raw liquid heights feed MLIQ;
  they never pass through MHGT's terrain floor or integer quantization.
  Existing MCLQ/MH2O ordering, filtering and admission policies remain intact.

`ConvertedTile` exposes `bytes()`, `into_bytes()` and `stats()` and deliberately
does not implement `Debug` for its payload. Statistics contain
`coverage: [bool;256]` in source `[y][x]` order, `visited_cells`, `unique_cells`,
`missing_mcvt`, `has_flight_bounds`, `has_liquid`, `has_holes` and
`ignore_deep_water`. Physical duplicate cells count toward `visited_cells` and
`missing_mcvt`; they do not inflate `unique_cells`. `has_liquid` describes
serialized MLIQ presence, which can include an empty visibility rectangle.

Errors preserve the parser/height/liquid/MLIQ/MAPS static codes. Invalid tile
coordinates use `conversion_invalid_coordinates`; unsupported build tags use
`maps_unsupported_build`. The closed conversion does not add fallback parsing,
repair an incomplete source or bypass any component's finite-value checks.

### Caller responsibilities and synthetic evidence

WDT selection remains outside this function: a caller can use
`modern_root_adts()` to select the root ADT and retain its coordinates and file
identity. The conversion function does not establish that the supplied bytes
belong to that reference. The borrowed `LiquidLookup` must already contain the
intended materialized type/material tables. The closed DB2 terrain readers and
their consumer-specific copy replay provide that separate operation; generic
decoded rows do not substitute for those semantics.

The diagnostic example `convert_adt <adt> <lookup> <context> <output.map>` calls
this public API. Lookup uses the existing `RCELIQ01` ABI, at most 256 type and
256 material records with last duplicate ID winning, and a 3,856-byte ceiling.
Its length must agree with the declared record counts.
Context is exactly 31 bytes: `RCECONV1`, map ID (`u32`), tile Y (`u8`), tile X
(`u8`), build (`u32`), pack (`u8`, zero or one), then the three limit values
(`u32` each); integers are little endian with no padding. The helper bounds
regular-file reads, requires exact EOF,
creates output exclusively after successful conversion and syncs it before
success. Stdout contains only counts, booleans and static status/error codes.
It adds no acquisition, client discovery, network or production extraction CLI.

Twenty new integrated synthetic tests cover literal minimal MAPS bytes, both
build tags, all four height encodings, early context/limit validation, aggregate
frame limits, exact output budgets, missing and repeated cells, staggered
MCVT layout, shared edges, persistent zero holes, MFBO selection and failures,
deep-water coordinate boundaries, the shared packing option, separate liquid
heights, lookup/filter behavior, invalid rectangles/formats, non-finite input,
late bounds errors and immutable input. The coordinator recorded **154 passing
terrain library tests**, a successful example build and clean workspace Clippy
for all targets in `terrain-conversion-{tests,build,clippy}` logs. Independent
source review reported no concrete findings.

### Integrated comparison with the original pinned converter

`native-adt-20261011T015737Z-de5fe310.json` records **214 positive complete
synthetic ADT inputs**. Production Rust `convert_adt` output matched the
original pinned C++ output byte for byte for every input. The C++ adapter
executed the unchanged original `ChunkedFile::parseChunks`,
`prepareLoadedData` and `ConvertADT(ChunkedFile&, ...)` overload. This extends
the earlier component-kernel comparisons to the actual parser and converter
on these supplied, structurally valid synthetic fixtures.

Every positive MAPS output passed strict production verification. The actual
RustyCore GridMap reader and the independent C++ height-query kernel produced
identical raw result bits for **30,602 queries**, 143 per case. The run also
recorded **22 Rust-only negative cases** and **two exclusive-output checks**.
Malformed negative inputs were not passed to the original C++ parser, whose
byte scanner is not a bounds-safe validator. No malformed-input safety or
rejection parity is inferred.

The original sources were materialized outside Git at pin
`02245dcd245e7433e524577656177723d3e4992e`, with an immutable inventory of
38 source/license-notice files and retained attribution. Compilation dependency
records were checked against the verified source inventory and local adapter
files. GCC 15.2 compiled three translation units with fast-math disabled,
`-ffp-contract=off` and section garbage collection. The retained binary used
the original memory parser/converter and retained no CASC or DB2 acquisition
APIs, original CLI main or compatibility filesystem I/O operations. Inert
global data and pathname construction/destruction do not constitute acquisition.
The minimal compatibility declarations serve discarded CLI typechecking;
their filesystem I/O operations fail if called.

The adapter captured original diagnostics in a retained private staging
directory, checked the native output extent and published it exclusively after
syncing. All recorded source, executable and input guards passed. The syscall
trace recorded **zero network calls**. No client assets entered this comparison.

The follow-up report `native-adt-dense-20261011T020000Z-00dd5c2f.json` adds
**34 positive dense synthetic cases** with identical Rust/original-C++ output
and **4,862 exact height-query results**. Across the two runs, that is
**248 positive cases and 35,464 exact queries**. It covers 256-MCNK old-liquid
fixtures with uniform, empty and omitted liquid output, all six dense MH2O
vertex formats, and complete containers with every section that reach the
**202,112-byte maximum** in both packing modes. The follow-up uses the same
original build and evidence scope; source, executable and input guards passed,
and its trace again recorded zero network calls. It repeats two exclusive-output
checks and adds no negative cases. The 22 Rust-only negatives remain evidence
from the first run.

This is acceptance of bounded conversion of supplied synthetic ADTs and scoped
height/hole queries. It does not establish CASC or WDT acquisition, higher-level
tile selection, WDT-to-ADT authentication, real client terrain, tilelist
generation, AREA/liquid/flight-plane runtime queries or full engine acceptance.
Previous reports remain at their original scope. The incomplete local Map.db2
input still blocks the full discovery/extraction run and is not replaced with
a prefix.

## Local terrain acquisition and MAP execution

The 2026-10-11 `forever_casc::terrain::run` API joins the preceding components
through one already admitted `LocalStore`. It loads complete **Map,
LiquidMaterial, LiquidObject and LiquidType** in pinned `ExtractMaps` startup
order, then uses the materialized Map selection to read WDTs and their root
ADTs. `LiquidObject` has a new closed all-field reader and native copy semantics;
its unused lookup is released after validation. Material/type lookups retain
ordered-copy/missing-source semantics and are compacted into sorted vectors.

`run(store, selection, pack, limits, sink)` takes the existing `AllMaps`,
`MapIds` or `Tiles` selection and returns an opaque `Completed` only after all
selected coordinates, all present files and final store metadata checks succeed.
Build comes from the admitted store; the caller cannot substitute an output
build. The same packing option reaches height and liquid serialization.
Source callbacks expose numeric identity/content key/length, and tile callbacks
receive actual complete converted bytes with their checked source association.
They do not expose raw input bytes. `Failure` carries a static error, numeric
phase/coordinates and attempted-read/provisional-output counters.

Every sink callback is **provisional until the entire run succeeds**. A missing
WDT, invalid present ADT, source mutation, exhausted limit or sink error rejects
the selection, including prior maps. Native partial extraction/empty tile-list
behavior is deliberately not a Rust success policy. An absent MAIN coordinate
requires no file; a present coordinate with zero, reserved, missing or incomplete
root FDID fails. Obj0/Obj1 and name-based alternatives cannot replace that root.
Name mode requires the caller's explicit prior `enable_names`; opening a store
does not opt in or trigger a fallback.

Only a successful whole-map selection produces a tile list. Explicit `Tiles`
never does, even when all 4,096 coordinates are listed. `tilelist::TileList`
encodes `MAPS`, version 10, the admitted build and exactly 4,096 ASCII bits.
The native `std::bitset::to_string` order emits bit 4,095 first; a tile's bit is
`y * 64 + x`. Bits become true only after successful conversion and sink delivery.
The separate strict codec checks size/header/build/ASCII, not file existence.
Canonical output names retain full-u32 map IDs and row-before-column order.

### Runner admission and output ownership

| Policy | Default and ceiling |
| --- | ---: |
| One DB2 / one WDT / one root ADT | 16 MiB / 1 MiB / 16 MiB |
| Direct rows / copy records per table | 100,000 / 100,000 |
| Materialized liquid keys / retained table capacity | 300,000 / 256 MiB |
| Selected maps / selected coordinates | 4,096 / 16,777,216 |
| Read attempts | 1,000,000 |
| Cumulative successfully read / emitted payload bytes | 64 GiB / 64 GiB |
| Combined store and runner logical memory | 1 GiB |

All limits can only decrease. The planner retains its independent row, explicit
tile, name and capacity limits. Each table decode reserves `2 MiB + 32 * file
length`, plus 64 MiB for Map/LiquidType text, before decoding/replay. Its ceiling
is 578 MiB. Actual retained Vec/String capacities replace that temporary charge.
Planner/expansion maximum capacities, 256 KiB WDT framing and 4 MiB conversion
scratch are reserved while relevant; store reads charge their archive/decode
phases jointly with live runner allocations. These are conservative logical
admission budgets, not portable allocator/RSS bounds or instrumentation of all
CPU work. Source decoders retain their existing infallible standard-library
allocations. Sink-owned memory/storage needs its own caller budget. Read/output
totals do not cap or describe the already opened store's separate bootstrap.

The Linux QA example is built with
`cargo build --offline --locked -p forever-casc --example terrain_local`:

```text
terrain_local <client> <all|maps:ID,ID|tiles:ID,X,Y;ID,X,Y> <new-output>
              [--names] [--no-pack] [--locale=enUS|--locale=esES]
```

Locale defaults explicitly to esES. The helper tightens the run to 16 maps,
65,536 coordinates, 1,024 explicit tiles, 4,096 reads, 256 MiB source bytes,
64 MiB staged memory/output and 1,024 artifacts. It reserves that staging
ceiling outside the runner's reduced memory allowance. Conversion finishes
before creating a new output directory. The destination must be outside Git
and the client/Data boundary; directory descriptors anchor exclusive writes.
Previous output is never opened for replacement. Files are synced and a
`complete.json` QA summary is written. Any write/sync/identity failure is a
failed invocation and can leave files for diagnosis; the summary is not an
atomic product commit, resume manifest or independent file verifier.

### Executed local-runner evidence

All report names below are in the external laboratory's `reports/acceptance/`.
The closeout `terrain-local-closeout-20261011T070801Z.json` records exact dirty
source hashes, preserved prior work, commands, failed attempts and timing.

- `terrain-local-20261011T070110Z-9de63241.json` passes **1,584 library tests**:
  669 CASC, 296 DB2, 154 terrain and 465 VMap. Four example tests and workspace
  all-target Clippy with warnings denied pass. The **46 terrain tests** consist
  of 22 earlier planner, 21 runner and three tile-list tests. The new DB2 reader
  adds two tests and extends the existing malformed-identity matrix. One opt-in
  synthetic corpus test is ignored normally and executed explicitly.
- Debug and release each execute **20 complete synthetic CASC/CLI scenarios**,
  covering both admitted builds, both packing modes, FDID/name routing,
  liquid-copy/VisualOnly behavior and repeated WDTs through copied map rows.
  Each profile also passes **20 existing-output guards** and **three negative
  guards** for client/Git destinations and an invalid startup source.
- Native comparisons `terrain-local-native-20261011T070110Z-9de63241.json` and
  `terrain-local-native-20261011T070425Z-13417e8b.json` each match **48 complete
  MAPS files and 24 tile lists**. Original pinned `parseChunks`,
  `prepareLoadedData` and `ConvertADT` execute on reviewed complete synthetic
  ADTs. The separate native tile-list helper uses original MapDefines and
  `std::bitset`; it does not run native CASC, DB2 or WDT orchestration.
  The actual current RustyCore GridMap source matches the independent C++
  height kernel on **6,864 raw result words/profile**, 143 per MAPS file.
  Original sources/notices and build dependency closure remain external.
  Each profile's native comparisons have **168 empty network traces**; the
  corpus/CLI trace is also empty. Source/binary guards pass, and the final
  release comparison additionally hashes and rechecks every corpus file.
- The optimized corpus report is
  `terrain-local-20261011T070425Z-13417e8b.json`. A preceding attempt recorded
  the network-syscall denial intercepting Cargo's local `AF_UNIX socketpair`
  while spawning an unbuilt release test binary. It never executed the test.
  Compiling that test separately offline and then running the unchanged corpus
  under denial passes with an empty trace. The failed attempt is retained;
  it was compiler IPC, not a remote client-data request.

The visual-liquid fixture initially expected no MLIQ for VisualOnly. Original
conversion writes the liquid entry before filtering, so nonuniform entries
retain MLIQ with an empty visibility crop. The corrected assertions independently
check entry 22, zero flags and `(128,128,130,130)` empty bounds; visible input has
water flags and `(0,0,9,9)`. The subsequent original converter comparison agrees.
No production conversion behavior changed to satisfy that test.

The actual local attempt is
`terrain-local-live-20261011T070621Z-3a59ec61.json` (2.42 seconds): explicit
esES tiles `(map 0, x 24, y 39)` and `(map 0, x 25, y 39)`. It returns exit 4,
`local_store_no_verified_encoding`, on the **first Map read**, with zero converted
tiles and no output directory. The trace is empty; client build metadata and
the release binary hashes are unchanged. This aggregate local-store error does
not identify the cause of every missing candidate or repair the earlier native
`local_file_incomplete` evidence. No real WDT/ADT conversion or runtime acceptance
ran. The completed library/QA path is synthetic acceptance only. Real complete
inputs, unified versioned product publication, area/liquid/flight runtime and
full extraction remain gates.
