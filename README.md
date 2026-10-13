<h1 align="center">RustyCore Extractors · Forever</h1>

<p align="center">
  Local client data. Versioned tools. Reproducible evidence.<br>
  Preparing the extraction tooling for <strong>WoW Forever 1.60.1 · builds 70170 / 70205 · Beta x64</strong>.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="GPL-3.0-or-later"></a>
  <img src="https://img.shields.io/badge/default%20branch-forever-6f42c1" alt="Default branch: forever">
  <img src="https://img.shields.io/badge/tools-C%2B%2B20%20%2B%20Rust-orange" alt="Tools: C++20 plus Rust metadata preview">
  <img src="https://img.shields.io/badge/70170-support%20pending-e09f3e" alt="Build 70170 support pending">
</p>

<p align="center">
  <a href="#current-status">Status</a> ·
  <a href="#build-the-existing-suite">Build</a> ·
  <a href="#forever-development-direction">Forever</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#data-stays-local">Data policy</a>
</p>

## What this repository is

Standalone, versioned client-data extraction tools for
[RustyCore](https://github.com/alseif0x/rustycore). The tools read a locally
installed WoW client and produce data for server readers without requiring a
TrinityCore checkout to build the standalone suite.

**`forever` is the default development branch.** The previous `main` branch
was renamed to [`3.4.3`](https://github.com/alseif0x/rustycore-extractors/tree/3.4.3),
preserving the WotLK Classic baseline as a separate version line.

> **Development preview, not a 70170-ready extractor.** Changing the branch
> does not port the implementation. The code currently builds the existing
> **C++ suite for WoW Classic 3.4.3, build 51943**. Its version metadata,
> formats and compatibility contract still describe that baseline.
> Rust development now includes bounded WDC5 decoding, versioned **DB2, GT and
> camera extraction** with native or explicit Rust-local acquisition, and a pure
> **MAPS assembler and strict verifier** with synthetic reader acceptance, plus
> a bounded shared collision pipeline that assembles final VMap artifacts.
> **The full extractor replacement and Forever/70170 extraction compatibility
> remain pending**; complete real terrain, VMap/MMap output and runtime
> acceptance still require work. The unified Rust product interface and
> manifest now have bounded synthetic and DB2/GT/camera integration evidence.

## Current status

The [Forever completion checklist](docs/forever-completion.md) tracks the full
Rust replacement, including acquisition, tables, terrain, collision, navigation,
packaging and runtime acceptance. The pinned Forever reference uses VMap 4.E
and MMap 16; the legacy 3.4.3 formats cannot be reused as a Forever acceptance
claim.

The [common package integrity stage](docs/forever-package.md) adds a versioned
manifest, explicit component completion seals, persistent incomplete markers
and the offline `forever-package verify` command for Linux. Its byte-integrity
contract preserves component acceptance separately. New state v2 adds explicit
same-context resume with retained-file replay checks. The new
[product CLI](docs/forever-product.md) connects all six Rust producers through
explicit plans and inspect/extract/resume/verify commands. Its bounded real
DB2/GT/camera package matches 33 prior accepted files; incomplete Map remains a
strict failure. Full real geometry/runtime acceptance and Windows distribution
remain open.
Product v1 preserves N/Z acquisition. Its separate v2 plan requires an
explicit `provided-salsa20-v1` capability and `--keys` for client commands;
offline verification needs no keys. Keys are never discovered, downloaded or
included in output metadata. See the [bounded key contract](docs/forever-product.md#explicit-supplied-keys-product-v2).

The [local distribution recipe](docs/forever-distribution.md) packages the
Linux product with its exact current source, vendored dependencies and notices.
Its offline reconstruction and archive integrity checks remain separate from
complete real extraction/runtime acceptance. Windows uses a separate
[resident NTFS backend](docs/forever-windows-product.md); its native product
evidence and remaining distribution gates are recorded separately.

The [Rust navigation input stage](docs/forever-navigation.md) now projects
verified MAPS into solid/liquid triangles, joins neighboring borders and removes
unused vertices. Debug and release each match **106 synthetic cases** against
the original TerrainBuilder. Complete Recast/Detour generation, the Forever MMap container
and real pathfinding acceptance remain unfinished.
Supplied VMO/placement geometry, model liquids and off-mesh connections now add
**97 exact synthetic C++ comparisons** in each profile, including terrain/model
composition. Actual instance discovery and complete real navigation inputs
remain separate from these supplied-input APIs.
The initial Rust Recast stage now rasterizes geometry, applies the three
walkability filters and constructs compact-cell connections. Debug and release
each match **162 synthetic cases** against the pinned original Recast. This
stage does not yet produce navigation polygons or usable routes.
Erosion, area median filtering and the distance field now add **164 exact
synthetic C++ comparisons** per profile. The prior heightfield comparison is
unchanged. Watershed region construction now adds **191 exact synthetic C++
comparisons** per profile, including region filtering, merging and border IDs.
Contour tracing, simplification and hole joins now match **155 exact synthetic
C++ comparisons** per profile: 152 use generated watershed regions, while three
separately supply region labels to exercise multiple holes. Polygon construction
now adds **652 exact synthetic C++ comparisons** per profile across polygon
widths 3–6, plus four explicit rejections where native triangulation warns of
incomplete output. Detail construction now matches **149 exact synthetic C++
comparisons** per profile, including native height sampling, Delaunay recovery
diagnostics and world-coordinate bits. Ordered polygon/detail merging now adds
**330 exact synthetic C++ comparisons** per profile, including four supplied
625-subtile grids. Rust Detour 7 payloads now match **728 exact synthetic C++
outputs** per profile, with 40 matching empty-input rejections. RustyCore's
unchanged public Detour APIs load all 728 and reproduce 149 navigation queries,
including two expected partial paths. Versioned **MMap 16** writers and strict
readers now match 762 synthetic global files and 728 tile files per profile;
the new readers feed RustyCore's public Detour APIs with the same query results.
Product/runtime integration and real routes remain pending.

The supplied-geometry TileBuilder now runs all **625 subtiles** in Rust. Its
debug/release comparisons match 107 configurations and 23 complete builds against
the original implementation. Pure Rust fresh-tile save mutation now initializes
links and snaps off-mesh endpoints before final encoding. Debug/release match
747 saved Detour payloads, including all 22 complete files written by original
`saveMoveMapTileToFile`. RustyCore loads all 747 through the strict MMap 16 reader
and reproduces 171 query cases. Real acquisition, unified product output and
high-level world/runtime acceptance remain open.

Navigation source/ancestor planning now matches **82 synthetic packages** and
**140 global files** against original C++ MapBuilder discovery and construction.
RustyCore initializes those files through the strict reader and reproduces
**3,500 coordinate queries**. This pure API requires complete supplied directory
descriptors. Active VMap model loading now feeds navigation directly in original
slot order; 154 synthetic packages / 1,507 operations match original C++ in both
profiles. Rooted terrain loading and ancestor/neighbor selection now match
110 complete synthetic cases against public C++ `loadMap` in both profiles.
The supplied single-tile pipeline now joins those sources with compaction,
connections, all 625 subtiles and final MMap save/framing. Nine scenarios per
profile match original public C++ buildTile, producing seven exact final files /
452,012 bytes and two empty outcomes. RustyCore's public Detour APIs accept the
seven/profile through a strict Forever adapter and reproduce complete, partial
and absent-nearest query results. The legacy MMap 16 reader remains incompatible.
Linux rooted package discovery now reproduces all 82 inventory cases and 140
globals from canonical source folders. Package-bound generation retains the same
seven final files in both profiles and rejects observed input changes.
A sequential runner now executes all/map/tile selections with provisional sinks
and two full inventory boundary checks, plus original-identity checks per read.
Debug/release match 12 synthetic packages / 15 global files / 22 selected tiles, including
20 final files / 1,176,088 bytes and two empty outcomes. The preceding nine
pipeline results remain identical; navigation passes 316 tests.
Synthetic multi-tile acceptance now loads the original global parameters and
multiple final files together: 33 scenarios / 1,248 query records per profile,
including 187 complete cross-tile paths in the fully loaded states. Duplicate,
removal/reload, stale-reference and explicit capacity rejection behavior match
original Detour and unchanged public RustyCore APIs. Complete real startup,
product manifests/resume/distribution, MMapManager integration and high-level
real world/runtime acceptance remain open. See the
[multi-tile contract](docs/forever-navigation.md#multi-tile-detour-and-public-reader-acceptance--2026-10-11).

| Area | What exists today |
| --- | --- |
| Version lines | `forever` for 1.60.1.70170 development; `3.4.3` for the legacy baseline |
| Implementation | C++20 legacy suite; Rust WDC5 inspector and bounded decoder with 56 reviewed schemas plus an explicit pinned 1,126-schema catalog; DB2/GT/camera extraction and versioned verification; native or explicit Rust-local CASC acquisition; terrain foundation |
| Existing client contract | `wow_classic`, 3.4.3.51943 — not a Forever product/build claim |
| Existing tools | `mapextractor`, `vmap4extractor`, `vmap4assembler`, `mmaps_generator` |
| Conformance | Build-70205/esES batch: 54 available tables match the Python oracle and pass output verification; RustyCore typed names, appearance and item specialization loaders accept their inputs, and four spell dependency tables match its generic reader. Complete real Birth and initialization acceptance remain blocked by incomplete `SkillLineAbility` and `Map`. Full 70170/70205 extraction remains pending |
| Client assets | Never distributed through Git, CI artifacts or releases |

The current tools cover map/client-data extraction, collision geometry and
navigation generation. They are distinct from the target-specific Rust DB2
readers and diagnostic probes currently developed in RustyCore itself; those
components have **not** been migrated into this repository as a native
extractor suite.

The first independent Rust step is [`forever-db2-inspect`](docs/forever-rust-foundation.md):
read-only, offline header/section diagnostics for an explicit local WDC5 file.
It reports metadata, not record contents or verified client provenance, and
does not depend on the server's architecture refactor.

The same guide documents the bounded `forever-db2` record decoder and the
`forever-db2-extract` batch CLI: `ChrClasses` (1361031) and `ChrRaces`
(1305311) form the default plan; explicit offline lists can select the other
reviewed schemas. Support is limited to one unencrypted regular section, 16 MiB per
file / 256 MiB per batch, redacted JSON reports. Reports carry the matched build;
`table_schema_validated`, `record_data_decoded` and `declared_records` are
distinct and no full-catalog or encrypted decoding is claimed.

The [pinned metadata catalog](docs/forever-metadata-catalog.md) adds explicit
decoder selection for all 1,126 source declarations. It preserves the reviewed
56-schema policy and has synthetic coverage of the supported value shapes.
A separate explicit v3 catalog batch matches **21 additional files / 5,302
rows** against the Python oracle and passes output verification, including
`LiquidType` string arrays. Select it with `--schema-catalog
pinned-metadata-v1` and an explicit manifest. Complete typed-loader and
whole-catalog acceptance remain pending.

The same guide now documents the separate `wowdbdefs-70205-v1` target catalog
with 1,161 build-specific definitions. A bounded comparison of the 1,023
complete local survey headers found no table-hash, layout-hash or field-count
mismatches against those definitions. This expands schema facts for explicit
build-70205 batches; it does not establish full decode, acquisition or runtime
compatibility.

The [game-table component](docs/forever-game-tables.md) extracts and verifies
all **20 files** in the pinned C++ catalog with `--group catalog`. On
build-70205/esES, **22,082 f32 cells** from its 14 numeric tables match the
C++ oracle; six files are preserved and verified as raw bytes. The complete
RustyCore `InitialGameTables::load` / `SpellValueGameTables::load` calls also
pass for their six-file subset. Default `--group all` preserves the original
six-file v1 contract; the catalog uses v2. This does not establish gameplay
or a complete Rust CASC backend.

The [camera component](docs/forever-cameras.md) preserves its source DB2 and
all referenced raw camera files. Build-70205/esES acceptance verifies **10
cameras plus the source DB2**, with exact bytes against pinned C++ CascLib,
independent discovery, filenames, manifest schema and output verification.
This covers acquisition and inventory; M2 parsing, waypoints and runtime
playback remain separate work.

The [pure Rust local-store API](docs/forever-local-casc.md) now matches
**57 complete DB2, GT and camera files** against pinned C++ bytes and rejects
the two known incomplete inputs. It authenticates the local ENCODING and
ROOT metadata and uses an explicit bounded N/Z profile. The production
commands now pass five Rust-local plans and **61 artifact comparisons**
(including overlapping plans), with DB2 v4, GT v3 and camera v2 manifests.
Select `--backend rust-local` with explicit index and ROOT layouts; native
remains the default. The store API also has explicit conservative name reads:
**3,468 synthetic queries** match the original C++ hash/ID/CKey selection.
The local name index builds successfully, but the 52 queried candidate names
are absent in both implementations; positive real name-read acceptance and
product CLI integration remain pending. The bounded sparse decoder also accepts all
**558 `ConversationLine` rows** against the Python oracle; this has no
RustyCore sparse-reader acceptance. The terrain foundation has WDT/ADT
parsing, height/area/hole projection, MCLQ/MH2O liquid projection, MHGT/MLIQ
serialization and MAPS assembly. **380 synthetic MAPS files** match the
independent layout and accepted C++ section kernels, pass strict verification
and produce **54,340 matching height queries** through RustyCore's actual
`GridMap` API. Integrated ADT conversion additionally matches **248 complete
synthetic outputs from the original C++ parser/converter**, with **35,464
matching height queries**. The local terrain runner now loads all four startup
tables, WDTs and root ADTs, converts MAPS and emits complete-map tile lists to a
provisional sink. Debug/release each pass 20 synthetic CLI scenarios, with
**48 MAPS / 24 tile lists** matching C++ and **6,864 exact GridMap queries**.
The real two-tile attempt still fails its first `Map.db2` read with no verified
local encoding, creates no output and attempts no network. Complete real terrain
and area/liquid runtime acceptance remain separate gates. See the
[local terrain contract](docs/forever-terrain.md#local-terrain-acquisition-and-map-execution).
The complete local catalog
survey decoded **1,023 tables / 719,466 rows** against the Python oracle;
103 inputs remain explicit failures. See the
[completion gates and retained evidence](docs/forever-completion.md).

The [collision foundation](docs/forever-collision.md) adds a pure M2 collision
parser and raw04E writer/verifier. **53 synthetic outputs** match the original
C++ `Model::open` and writer through a complete-memory QA adapter. WMO root/group parsing, geometry/liquid projection and complete
supplied-WMO raw conversion/verification add **59 synthetic tests**, including all
65,536 material-flag values. **155 complete synthetic raw WMO files** match
the original C++ parsers/writers. Pure doodad selection and bounded offline
M2/WMO acquisition now preserve the metadata needed by later placements;
explicit FDID/name routing, budgets and failure states are separate from the
product CLI. Product publication and collision runtime remain pending.
The bounded real M2 sample matches **31 complete acquired files**, **28 raw
geometry outputs** and **three no-collision outcomes** against C++; its original
32-file plan rejects the absent input without creating output.
Direct MDDF/MODF projection and `dir_bin` serialization match **205 synthetic
outputs**, including the original C++ `ModelSpawn` reader/writer roundtrip.
Object readers, serial IDs and WMO doodad projection now connect finished model
acquisition to complete placement batches. **573 synthetic doodad outputs** and
**96,576 serial ID requests** match the original C++ in dev/release profiles;
the later parent-cache comparison matches **11,144 records** against original
replay. The local object runner now joins WDT/Obj0ADT acquisition, model conversion,
persistent IDs and immediate-parent replay through a staging destination, with
**20 synthetic integration tests**. Full real discovery, publication and
real assembled-output/runtime acceptance remain pending; real Map input is still incomplete.
The bounded BIH constructor and strict storage reader match **420 synthetic
C++ outputs** in dev/release, including empty trees and the actual depth-65
boundary. Supplied-raw model assembly now matches **251 complete synthetic
`.vmo` files** against original C++ in dev/release, including the original
model reader/writer roundtrip and geometry-aware strict validation.
Transformed placement bounds and supplied single-map assembly now match **274
bounds cases and 136 complete synthetic maps** against the original C++ in both
profiles. All **62,006 map/tile/index files** match, with strict association
checks. Supplied GameObject model lists now match **276 synthetic cases / 536
rows** against original `exportGameobjectModels` in both profiles. Their bounds
include every raw vertex, including unused vertices. The separate direct DB2
adapter now matches **50 synthetic files / 1,164 rows** against the original
loader, preserving signed FDID bits and validating copies without replay.
Bounded GameObject startup now joins complete DB2 acquisition, classification,
model conversion and both lists. Dev/release each match **36 synthetic startup
cases / 469 direct rows / 127 listed rows**, including **46 raw files**,
against the original producer and exporter using a complete memory inventory.
Rust reads complete synthetic CASC with explicit FDID routing; this does not
establish native CASC or name-first routing equivalence. Empty-liquid startup
results remain cached across a consuming lookup transition. The new bounded
`collision::world::extract` now connects startup, authenticated Map/liquid reads
and WDT/Obj0ADT processing through one collector and one placement-ID session.
Dev/release each match **49 complete synthetic shared-world cases** against
original startup, Map/liquid loaders and `ParsMapFiles`: **82 raw files**, both
startup lists, and **79 per-map `dir_bin` files / 176 placements**. Fifteen Rust
negatives and two exclusivity guards pass per profile, with no network attempt.
The older per-batch API remains available. Complete real acquisition, product
publication and runtime remain gates; real Map and `GameObjectDisplayInfo`
inputs are still incomplete. That shared-world workspace passed **1,378 library
tests**, **2 example tests**, **202 focused collision tests** and all-target
Clippy; the earlier **1,341-test** startup report is retained separately.
The new consuming `collision::assemble::extract` adds a complete selected-model
inventory and final VMO, nested map/tile/index and dtree artifacts. Model names
are selected before placement-ID deduplication; unused acquired dependencies
are excluded. Parent-only indexes bind to the first present ancestor payload,
and a mismatch fails the whole package. Explicit limits cover 1 GiB logical
capacity, 256 MiB staging, 512 MiB output, 65,536 files and cumulative work.
Debug/release each match **53 complete synthetic cases / 650 final files /
56,019 bytes**, including **87 VMOs**, **180 unique spawns** and **303 tile
associations**, against original startup/world processing and `convertWorld2`.
Forty Rust-only negatives, including 25 strict parent-association failures, and
two exclusivity guards pass per profile, with no network attempt. That assembly
workspace passed **1,413 library tests**, **2 example tests**, **237 focused
collision tests** and all-target Clippy. This remains synthetic local-CASC versus
native memory-inventory evidence. Its private QA entry point writes only after
complete success; versioned publication and real input/runtime acceptance remain
pending. The later optional RustyCore terrain library connects the actual MAPS
reader and rooted VMap store, with checked static queries and shared tile leases.
Its 23 focused synthetic tests include 1,210 native height-selector comparisons;
the server does not yet instantiate it. See the
[reader acceptance](docs/forever-collision.md#checked-rustycore-terrain-library)
and the
[assembled-package contract](docs/forever-collision.md#shared-world-collision-assembly)
and [historical world evidence](docs/forever-collision.md#shared-startup-and-world-execution).

The extractor library now implements bounded **VMO model and instance ray
queries** over its complete readers. Rays keep their supplied direction;
nearest and first-in-traversal hits use one cumulative node/triangle budget.
Instance rotation, finite nonzero scales (including negative scales) and
explicit errors remain distinct from misses. This adds **54 query tests**;
that query increment passed **1,467 library tests**, **2 example tests** and
all-target Clippy. Debug/release each match the original model/instance consumers on
**79 synthetic cases / 14,416 queries**, including **2,950 hits** and **11,466
misses**, with exact distance bits and no excluded case. Fourteen Rust-only
negatives and two exclusivity guards pass per profile, with no network attempt.
That comparison did not cover map loading or placement-to-model binding;
the subsequent static-store increment below addresses those library operations.
See the
[model/instance query contract](docs/forever-collision.md#model-and-instance-ray-queries).

The library now also provides an immutable **static map/tile/model store**,
Linux file source, and world/internal LOS, height and object-hit methods. It
retains shared models, repeated tile references and strict ancestor bindings;
failed loads preserve earlier state and detected file changes invalidate the
source. Queries use cached geometry without filesystem I/O. The workspace
passes **1,558 library tests**, **2 example tests**, **66 focused store tests**
and all-target Clippy. Debug/release each match the original static manager on
**75 synthetic cases / 3,298 operations**, including **2,654 queries**, with exact
query words and visible model/slot counts; 58 Rust-only negatives and two output
guards pass. RustyCore provider installation, area/liquid queries,
dynamic objects and real-client runtime acceptance remain separate gates.
See the [static package contract](docs/forever-collision.md#static-package-loading-and-manager-queries).

The [2026-10-11 acceptance](docs/forever-rust-foundation.md#executed-54-table-and-generic-spell-reader-acceptance)
validates 54 available tables out of 56 reviewed schemas: **141,594 rows** match
the Python oracle and the output verifier accepts the batch. Eight tables are
empty in this client.
RustyCore's typed names loader accepts four tables with **9,256 rows**; its
category comparison covers only ID and creation charset. The typed appearance
loader accepts all seven input tables with **12,859 rows** in the compared
projections. The item specialization loader accepts `ItemSpecOverride` with
**9 rows** plus empty `ItemSpec` and `GemProperties`; populated variants for
the latter two are covered synthetically. Four Birth tables add **1,300 rows**.
`PowerType` and `Movie` add six and three rows. `SkillLineAbility` and `Map`
fail local acquisition with `local_file_incomplete`; complete real typed Birth
and initialization acceptance remain blocked. Their complete five-table and
seven-table loaders have positive and negative synthetic coverage.
Four spell dependency tables add **270 rows**, with exact ID/field agreement
through RustyCore's generic reader. That comparison does not execute the full
`SpellRecords::load` API and retains the documented limits for other copy and
compression variants.
The decoder accepts omitted parent lookups according to the reference and
rejects truncated empty tables.
Typed checks exercise `load`, without later `finish` steps or SQL/hotfix
overlays. This is bounded build-70205/esES evidence;
complete typed initialization and full runtime acceptance remain pending.

The same guide documents the optional `forever-casc-probe`: exact
`wow_classic_beta`/70170 and 70205 admission, local FileDataID reads and byte comparisons,
with a pinned C/C++ backend. It is **not** a complete extractor or pure-Rust CASC.

The historical conformance evidence is scoped: the recorded build-51943 run
compares 17,391 files, but MMap generation covers one deterministic map-0 tile.
The recorded RustyCore VMap-reader boundary remains unsupported at that audit.
See [the conformance guide](docs/conformance-harness.md) for the exact coverage
and limitations, rather than treating these results as full runtime support.

## Build the existing suite

Requirements: **CMake 3.25+**, a **C++20 compiler**, **Python 3** and
**OpenSSL 3 development files**. Other compiled dependencies are pinned in
the audited source closure.

Keep builds, installations and extraction outputs outside this checkout and
outside the client directory:

```bash
git clone --branch forever https://github.com/alseif0x/rustycore-extractors.git
cd rustycore-extractors

cmake -S . -B ../rustycore-extractor-work/builds/standalone \
  -DCMAKE_BUILD_TYPE=Release
cmake --build ../rustycore-extractor-work/builds/standalone --parallel 1
ctest --test-dir ../rustycore-extractor-work/builds/standalone --output-on-failure
cmake --install ../rustycore-extractor-work/builds/standalone \
  --prefix ../rustycore-extractor-work/artifacts/standalone
```

All four binaries provide `--version` and `--version-json`. The latter reports
the existing product/build, upstream commit and output-format versions;
**expect the 3.4.3/51943 baseline even when building this branch**.

For Windows build instructions and safe real-client invocation, use the
[standalone suite guide](docs/standalone-cpp-suite.md). This README does not
recommend running the existing suite against a 70170 client or changing a
build-number constant to bypass compatibility checks.

## Forever development direction

The next work must establish the actual Forever contract before declaring
support:

1. Identify the target client product, CASC inputs, DB2 layouts and required
   outputs from build-70170 evidence.
2. Reuse existing C++ components only where target evidence supports them.
3. Introduce Rust-native extraction stages with explicit inputs, diagnostics
   and synthetic positive/negative coverage.
4. Compare outputs with an appropriate pinned reference and exercise the
   corresponding Forever readers in RustyCore.

These are development goals, **not completed features**. Preserve the existing
3.4.3 contracts as versioned evidence; do not silently relabel MAPS/10,
VMAP_4.B or MMAP/15 as validated Forever formats.

The [original delivery plan](https://github.com/alseif0x/rustycore-extractors/issues/13)
describes the **3.4.3 programme**. Its old `main` references and build-51943
milestones are historical context, not proof of Forever readiness.

## Data stays local

- Current extraction is **local-CASC-only**. Remote CASC and implicit key
  downloads are disabled; changing branches does not enable network access.
- Treat the client installation as read-only. Use explicit paths and isolated
  run directories; the MMap generator works relative to its working directory.
- Never commit or upload client-derived DB2, maps, VMaps, MMaps, models,
  cameras, game tables, CASC caches, keys or intermediate files.
- Use synthetic fixtures and reviewed hash/structured reports for evidence.
  A successful process exit alone does not establish compatibility.

## Documentation

The following guides describe the **existing 3.4.3 suite**, unless explicitly
marked otherwise:

- [Development environment](docs/development-environment.md)
- [Forever Rust metadata preview](docs/forever-rust-foundation.md) — separate from the legacy suite
- [Explicit Linux collision disk product plan](docs/forever-product-disk.md) — v3 capability; full real-client/runtime acceptance remains open
- [Standalone C++ suite](docs/standalone-cpp-suite.md)
- [Output conformance harness](docs/conformance-harness.md)
- [Versioned compatibility and network contract](docs/compatibility-contract.md)
- [Source-closure audit](docs/licensing/source-closure.md)
- [Machine-readable dependency inventory](licensing/inventory.json)

## License and attribution

Project code is licensed under **GPL-3.0-or-later**. Preserve the original
TrinityCore and third-party notices. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
the [license directory](LICENSES).

World of Warcraft and its related trademarks belong to Blizzard
Entertainment. This project is not affiliated with or endorsed by Blizzard.
No Blizzard client data is stored or distributed by this repository.
