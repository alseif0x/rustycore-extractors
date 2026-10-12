# Forever Rust extractor completion gates

This is the completion checklist for the operator's goal, **complete the Rust
extractor for WoW Forever**. Passing a bounded DB2 batch does not complete this
goal. The first full local acceptance target is the authorized `1.60.1.70205`
client. Build `70170` requires its own evidence; admission of either build is
not a compatibility result.

The reference is TrinityCore commit
`02245dcd245e7433e524577656177723d3e4992e`. The legacy programme in issue #13
and its `3.4.3.51943` output contract remain separate. No release, publication
or client-data upload is authorized by this checklist.

## Required components

| Component | Current evidence | Completion requirement |
| --- | --- | --- |
| Local acquisition | Explicit Rust-local component CLI passes five DB2/GT/camera plans with 61 artifact comparisons and closed-profile verification; native remains default for those component CLIs; the Rust-only unified product admits explicit v2 supplied keys with synthetic acceptance; separate name API has original C++ synthetic comparison and real negative-query evidence | Complete supported-profile and actual real encrypted acceptance; positive real named-file reads; no implicit network or zero-filled missing data |
| DB2 catalog | 56 reviewed schemas retain their v2 policy; pinned catalog accounts for all 1,126 declarations; complete local survey matches 1,023 artifacts / 719,466 rows against Python, with 103 explicit failures; separate closed terrain sources have synthetic and two-table local acceptance | All required extraction tables and physical variants accounted for; unsupported/incomplete inputs reported without success claims; complete required reader acceptance |
| DB2 reader acceptance | Separate names, appearance and item specialization loaders accepted; Birth and initialization have synthetic complete-loader evidence; all 49 spell tables now have complete public typed-load projection acceptance on five populated synthetic worlds; the older four-table generic-reader comparison remains separate | Complete public Forever loaders, including Birth, initialization and all 49 spell tables, on complete local inputs; explicit treatment of encrypted/multiple/sparse sections and output TACT metadata |
| Game tables | Complete 20-file catalog with native v2 and Rust-local v3 verification; 14 numeric tables / 22,082 f32 cells match C++; six raw-only files; both public six-file RustyCore loader groups accepted with both backends | Preserve this component evidence in the unified product interface; retain numeric versus raw-only scope |
| Cameras | Native v1 and Rust-local v2 acquisition/output verification; ten camera files plus source DB2 match C++ bytes, independent discovery and filenames | Preserve component evidence in the unified interface; M2/waypoint/runtime acceptance needs a separate reader contract |
| Terrain maps | Local startup/WDT/root-ADT/MAPS/tile-list runner has 20 synthetic CLI cases per debug/release profile, with 48 MAPS / 24 lists and 6,864 exact GridMap queries per profile; real probe fails at the first complete Map read; integrated supplied-ADT conversion matches 248 complete synthetic files from the original pinned C++ parser/converter across two runs, with strict verification and 35,464 actual GridMap queries; 154 synthetic tests; preceding assembly acceptance covers another 380 files / 54,340 queries | Complete real local terrain acquisition; versioned real MAPS semantics, including liquids and bounds, compared against Forever C++ output and appropriate RustyCore readers |
| Collision | Static map/tile/model store and world LOS/height/object-hit match 75 synthetic cases/3,298 operations against original VMapManager in debug/release, including 2,654 queries; Linux rooted source and 465 VMap tests (66 focused store tests); bounded VMO model/instance ray queries match 79 synthetic cases / 14,416 queries against original readers/queries in debug and release; 54 focused query tests and 374 pure VMap tests; shared-world final-package assembly matches 53 synthetic cases / 650 files / 87 selected VMOs against original startup/world/convertWorld2 in dev/release, with strict ancestor binding; 49 complete synthetic shared-world cases match original startup/Map/liquid/WDT/ADT output in dev/release, including 82 RAW and 79 per-map dir_bin files / 176 placements; 36 integrated synthetic GameObject startups / 469 direct rows / 127 listed rows match the original producer in dev/release, following 276 supplied-list and 50 direct-DB2 comparisons; 274 transformed-bound cases and 136 complete supplied-map outputs match original C++ in dev/release, including 62,006 map/tile/index files and strict associations; 251 synthetic VMO models match original assembly/read/write; local WDT/Obj0ADT/model/placement runner has 20 integration tests; 11,144 cached records match original replay; bounded BIH matches 420 outputs; M2/WMO raw comparisons cover 53/155 synthetic outputs; real M2 sample matches 31 acquired files and 28 raws; direct/doodad placements match 205/573 outputs; 96,576 serial ID requests match C++ | Real object and WMO acquisition acceptance; complete real Map/GameObject sources; accepted real inventory/ancestor associations and versioned product publication; complete real assembled-output and collision/height/liquid reader acceptance |
| Navigation | Supplied geometry, Recast, Detour 7 and MMap 16 stages have bounded native/reader acceptance; complete supplied TileBuilder matches 107 configurations and 23 builds; pure Rust final save state matches 747 payloads, including all 22 original saved TileBuilder files; numeric Map projection and parent inventory planning match 82 synthetic packages and 140 globals, with 3,500 exact RustyCore coordinate queries; active VMap model selection/loading matches 154 packages / 1,507 operations against original VMapManager/TerrainBuilder; rooted terrain/ancestor/neighborhood loading matches 110 complete cases / 217,721,658 bytes; joined single-tile pipeline matches nine scenarios and seven final files / 452,012 bytes per profile, with complete/partial/absent-nearest RustyCore Detour query acceptance; 319 focused tests; canonical Linux discovery matches 82 inventories / 140 globals and binds all nine single-tile scenarios to observed filesystem identities; sequential all/map/tile execution matches 12 synthetic packages / 15 global files / 22 selected tiles, with 20 final files / 1,176,088 bytes, provisional sinks and per-read original-identity binding; continuous-plane packages add 14 exact files / 1,433,000 bytes and 33 multi-tile lifecycle scenarios / 1,248 query records per profile, including 187 complete cross-tile paths in fully loaded states through original Detour and unchanged public RustyCore APIs | Complete real inventory/source acquisition and real model/source tables; original DB2 startup and scalable full-map/product orchestration; versioned product/runtime integration and high-level real pathfinding acceptance |
| Product interface | Canonical v1 N/Z and separate v2 explicit supplied-key plans; Linux inspect/extract/resume/verify connect all six Rust producers; final package v1/internal state v2. Recovery, one-tile six-component packages and bounded real DB2/GT/camera evidence are recorded in the [product scope](forever-product.md); strict Map failure remains | Complete real all-component acquisition/output and runtime acceptance, scalable full-world operation and distribution; intrinsic verification does not prove complete geometry inventories/associations or Detour topology |
| Distribution | Legacy C++ suite; Linux v9 bundle of `b692ed3` has two identical fresh archives/manifests/binaries, an exact moved-source offline rebuild and 50 packaged synthetic CLI cases; frozen corresponding source, exact registry archives/notices and qualified system linkage are retained; Windows read foundation is [recorded](forever-distribution.md); the separate [private-output foundation](forever-windows-output.md) has fourteen ordinary native fixed-NTFS cases per debug/release profile, while privileged reparse and Windows product/recovery/distribution remain open | Reproducible Linux/Windows Rust packages with recorded acceptance, exact dependency/license closure, notices and corresponding source, synthetic CI and local full-output/runtime acceptance |

## Version differences that must be resolved

The pinned Forever source declares `MAPS` version 10, `VMAP_4.E` (raw
`VMAP04E`), and `MMAP` version 16. The MMap global header contains magic,
version, Detour parameters and an off-mesh connection count (40 bytes).
Detour still declares navmesh version 7 and 64-bit references. Those shared
Detour values do not make the MMap 15 and 16 containers interchangeable.

Source locations at the fixed commit:

- `src/common/Collision/Maps/MapDefines.h` and `MapDefines.cpp`;
- `src/common/Collision/VMapDefinitions.h`;
- `src/common/mmaps_common/MMapDefines.h`;
- `dep/recastnavigation/Detour/Include/DetourNavMesh.h`;
- `src/tools/map_extractor/System.cpp` and
  `src/tools/map_extractor/loadlib/DBFilesClientList.h`;
- `src/server/game/DataStores/DB2Metadata.h`.

RustyCore's existing `wow-map` reader describes a 3.4.3 height/holes port.
Its optional Linux `forever-terrain` library now verifies supplied Forever MAPS
before that reader and connects the actual rooted static collision store, with
synthetic checked-query/lease acceptance below.
`wow-recastdetour` expects MMap 15 and a 28-byte legacy global parameter
file. Its public Detour API separately accepts synthetic raw version-7 tile
payloads; that API bypasses the container header. The extractor's new MMap 16
readers now decode global/tile files before feeding that API in an external
acceptance runner; the integration repository and legacy parser are unchanged.
The new global acceptance also confirms that the legacy parameter parser only
requires 28 bytes: it accepts a 40-byte v16 global while misreading its header
as parameters. It must not initialize a navmesh from that result. Strict global
decoding supplies the correct parameters to the accepted public API.
Forever runtime bootstrap does not yet install these geometry
components. Reader work and versioned end-to-end evidence are required;
changing a magic number alone cannot establish compatibility.

A subsequent entry-point audit distinguishes the legacy `world-server` from
`forever-world-server`. Legacy `app.rs` uses provider-free `LiveTerrainHeights`;
Forever's account-phase bootstrap has no spatial consumer yet. The canonical
managed-map type still uses no-op terrain loading/lifecycle. A checked provider
and terrain lease adapter now have production-library acceptance, which is
not live Forever gameplay acceptance. Complete original parent/cosmetic fields
are required; the current Forever initialization projection and available-prefix
mode do not supply the required complete topology. Preserve that runtime scope
while implementing extractor/reader capabilities.


## Local input gaps and retained historical attempts

In the initial recorded build-70205/esES attempts, strict reads of `Map` and
`SkillLineAbility` fail with `local_file_incomplete`. Failed manifests are
rejected by the output verifier, and real initialization/Birth probes stop
before their public typed loaders execute. A separate strict enUS attempt
did not recover either input. Available prefixes are not admitted.

A subsequent bounded preflight of the 78 distinct files in RustyCore's
acquisition/spell catalogs plus `ChrClasses`/`ChrRaces` completed on
build-70205/esES. It found 48 complete artifacts and 30 strict
`local_file_incomplete` results. Of the complete artifacts, 21 were acquired
with metadata-only status by the existing extractor; they have not thereby
received record-decoding or typed-loader acceptance. All three failed batches
were rejected by the verifier with exit 8 / `INCOMPLETE_RUN`.

The subsequent decoder-API run below accepted those 21 complete artifacts.
It does not repair the 30 missing inputs or upgrade the earlier incomplete
production manifests.

Retained structured evidence:
`runtime78-preflight-summary-20261011T002601Z.json` in the external acceptance
reports directory, with three linked private runs. Network syscall denial
recorded no attempt, client build metadata remained unchanged, and the two
acquisition/inspection binary hashes matched across the three batches. No
complete typed loader ran in this preflight. An earlier summary retains an
invalid verifier invocation; the named report records the corrected command
and supersedes that summary.

These strict-read failures prevent those particular real acceptance runs. They
do not prevent implementing the other required components. Do not mark the
whole goal blocked while useful implementation or synthetic acceptance
remains possible.

The later fixed-route key-free acquisition of `Map`, `SkillLineAbility` and
`GameObjectDisplayInfo` reports encrypted input and
`local_store_no_verified_encoding`, with no successful decoded file. Separate
complete-span framing accepts three `Complete(FramedMd5)` candidates and checks
all 15 frame MD5s (six Z and nine E frames). This does not prove plaintext,
complete CKey/EKey identity, cipher interiors or that supplying keys alone would
recover the inputs. Preserve the earlier incomplete outcomes as historical
attempts; see [the framing acceptance](forever-required-framing-acceptance.md).

A later read-only audit of the workstation's esES `DBCache.bin` completely
framed 28,377 HTFX v9 records, but its header is build 70170 while the target
client is 70205. It contains 128 invalid `TactKey` records with zero-byte data,
no valid `TactKey` or `TactKeyLookup` rows, and no `BroadcastText` records. It
therefore supplies no usable key for the missing files and is stale evidence for
this target build. The audit retained only aggregate metadata at
`/mnt/nvme2/datos/tools/rea/investigation-wow-20261012/dbcache-audit-v1.json`
(SHA-256 `e9577f74b6922b93d1065382e8d194d6780700de854775a7dd105337ee714b95`);
record IDs and payloads were not retained. Format and table-hash references
were pinned to [wowdev/DBCD](https://github.com/wowdev/DBCD/tree/e732093f8864240fc5884bd1bba6b02f3dfc0d56)
and [WoWDBDefs](https://github.com/wowdev/WoWDBDefs/blob/07d008f28907c00827cacadf2d7c1a40f578b6a3/manifest.json).

The adjacent client `Hotfix.log` is also stale (mtime 2026-10-02). A metadata-
only scan found 325 lines mentioning `TactKey`, but no `TactKeyLookup`,
`BroadcastText`, target DB2 names, or printable 16-/32-hex key-shaped tokens.
The aggregate report
`/mnt/nvme2/datos/tools/rea/investigation-wow-20261012/hotfix-log-audit-v1.json`
(SHA-256 `916656eb523d87af757b3336070a883dde5af026622f4668c329a8fc1f5dc13d`)
retains counts only, not lines or record identifiers. This bounded text scan
does not prove the absence of binary or non-printable material.

A second private cache copy from the RustyCore local client fixture is also
build 70170. It has 112 valid `TactKey` rows, 588 zero-byte `TactKey`
invalidations, and no `TactKeyLookup` rows. Joining its additions to the current
70205 static tables produced 113 candidate joins and 84 unmatched lookup rows.
Three in-memory keyring variants (current static tables, cache additions only,
and push-ordered cache semantics) were tried against `Map`,
`SkillLineAbility`, and `GameObjectDisplayInfo`; every acquisition returned
`local_store_no_verified_encoding`. No plaintext or decoded output was
accepted. This rules out that stale cache as a sufficient key source for these
inputs; it does not establish that no other target-build key source exists.
The mode-0600 aggregate report
`/mnt/nvme2/datos/tools/rea/investigation-wow-20261012/dbcache-cross-build-key-probe-v1.json`
has SHA-256
`439039a8be1b118bc704682e7e09efc4dab829038b5bc109351bea6c31e20949` and
contains no key bytes, key identifiers, record identifiers, or client payloads.

## Additional executed evidence — 2026-10-12 (Europe/Madrid)

### Complete 49-table public spell loader, synthetic inputs

The external original `rustycore-spell49-reader-check` preflights all 49 complete
inputs through the pinned catalog decoder and closed spell header gates, then
calls unchanged RustyCore `SpellRecords::load(directory, Complete)` once per
world. Its independent expected projection follows catalog physical-column
order and public record declarations, without typed getters or Debug text.
It compares every one of the 469 public row fields, preserving integer aliases,
f32 bits (including signed zero, infinities and NaN payloads), every array
element, raw text and locale presence. RandPropPoints allocation metadata also
includes unresolved copy destinations.

Five populated 49-table synthetic worlds cover direct rows, copies, chains,
overwrites and zero/unresolved copy sources. Python independently matches every
authored logical value in the 245 input files. Debug/release each execute five
complete public loads and match 784 rows / 45,946 projection bytes / 148,168
input bytes. Each profile passes three probe unit tests and four expected
negative calls. Tool Clippy passes; two inherited RustyCore dependency warnings
remain. All 23 final traces contain no network calls; five contain only signal
annotations. The five probe sources, 577 dependency
files, 636 retained synthetic inputs, logs, traces and binaries are rechecked.
No RustyCore implementation or extractor dependency is changed.

The accepted profile is unencrypted regular single-section data with declared
column widths. Earlier zero-column-BitSize fixtures pass catalog decoding but
RustyCore rejects the first inline-ID table (`SpellEmpower`); that difference is
retained and explicitly repeated as a negative, not silently repaired in either
reader. An earlier MinId fixture authoring error and a private compiler-cache
disk-cap failure remain failed reports. The final QA compiler-cache ceiling is
1 GiB, with a 3 GiB free-space floor and 2 GiB service memory ceiling; these are
QA resource policies, not product limits or exact RSS admission accounting.
Input files and expected projections have separate 256 MiB QA limits, with
16 MiB per input. The complete typed loader keeps its own narrower policies.
Post-read byte equality is not an atomic input snapshot.

Retained private run: `spell49-complete-reader-20261011T2210Z`.
Final report SHA256:
`db09cd6e760f662c957e51da543809f57977f84e14c42818d629cf821bfe56d7`;
independent root acceptance:
`12b1ef505a7b80b86c3dfab1d9968102ea556486db7065be55ddef499eb9e8ab`;
source/input freeze:
`5635e09f83028f1efef4518ee04ae496a9178d957be7e46df415e2c20bf8896d`.
This extends the older four-table generic-reader evidence with complete typed
load projections. It does not accept complete real spell inputs, every physical
variant, `finish`, SQL/hotfix overlays or gameplay/runtime compatibility.

### Navigation reader corresponding-source integration

Source profile `forever-seven-crate-windows-navigation-readers-source-v7` is now
integrated into the canonical checkout: 27 reviewed paths, including ten new
files, with 952 source files and all 925 unrelated files preserved. Branch,
HEAD and index remain unchanged. The exact seven-crate/nine-package closure and
two registry archives remain, adding only the Windows-local navigation edge to
the platform crate relative to v6. Linux debug/release each pass 2,225 tests
across 41 targets (22 ignored), with Linux/GNU-Windows all-target Clippy and both
GNU-Windows compilation profiles. All 171 Forever contract/state and 104
distribution/sealer checks pass. This does not build a new package or establish
actual Windows/NTFS or complete extraction acceptance. The older Wine diagnostic
retains its rejected overall/network evidence.

Retained run: `windows-v7-navigation-union-20261011T214345Z`.
Rust checks SHA256:
`f63152d94ca1d2e1b8a2f347876b3e35d55e5af2746e3d54fd44528235827de7`;
canonical merge:
`b369b20f8f5ade669e4cce82529176736e5eb61c3b1c39124b5aa72819e80e4c`.
The preceding 161-contract/63-packaging subset and two earlier failed static
attempts are retained. The final broader checks additionally include package
state and Windows sealer regressions.

## Private spooled collision source integration — 2026-10-12

The optional Linux library joins disk-backed immutable collision payloads,
row-wise startup lists, placement/shared-run consumers and authenticated world
preparation under one collector. Successful completion retains startup data and
validated topology with their exact logical reservations. Failed owned-Sink
paths destroy the Sink before final source/spool checks. Existing resident
product extraction and v7 source/format contracts remain in force.

The accepted union passes 2,391 synthetic tests across 41 targets per Linux
debug/release profile (22 ignored), Linux/GNU-Windows all-target Clippy and both
GNU-Windows compilation profiles. All 171 source/state and 104 distribution/
sealer checks pass. The world-only step passes 387 collision tests/profile,
including 19 new tests against the unchanged resident Rust world over complete
authored toy CASC. It adds no new C++ or real-client comparison. Detailed scopes,
limits and retained report hashes are in the
[world owner contract](forever-collision-spooled-world.md). Later Windows-only
test fixture and exact-JSON-checkout corrections have separate hosted CI
evidence; cross compilation alone does not execute those tests.

The product driver still collects resident final assembly. WDT/cache inventories,
selected VMO/map/proof output staging and product integration remain required.
The current 4,096 cumulative read ceiling cannot admit a full 4,096-own-tile map
with its startup dependencies. The private spool's same-mount requirement also
remains; a separate-volume policy requires its own contract and acceptance. No
real input, full-world, product publication/resume, live runtime or distribution
gate is closed by this library integration.

## Additional executed evidence — 2026-10-11

- **Unified product interface:** `forever-extractor` now connects all six Rust producers with canonical explicit plans, dependency/selection gates, logical collection/loaded executable identities, shared package publication and explicit same-context recovery. Complete empty Map worlds use a validated pure empty navigation plan. Debug/release each pass 2,023 library + four example + three binary tests, 72 contract checks and 17 synthetic CLI scenarios, with six actual SIGKILL recoveries across profiles. Finished DB2 skipping is tested after corrupting only its private toy archive region; recovered outputs match uninterrupted packages. One-tile all-component packages publish 31 files/141,786 bytes/profile. Bounded real 70205/esES extraction matches 33 prior accepted DB2/GT/camera files/193,794 bytes; Map still fails before payload publication and the verifier rejects its marker. There are 74 empty network traces and six fsync/SIGKILL traces without network calls. This is product integration and bounded intrinsic verification, without new C++/RustyCore consumer execution or full real geometry/runtime/distribution acceptance. See the [product contract](forever-product.md).

- **Multi-tile navigation/reader acceptance:** Strict global/tile adapters now load multiple final tiles together through the original pinned Detour and unchanged public RustyCore APIs. Complete authored continuous-plane packages generate 14 exact final files / 1,433,000 bytes and four original global files in debug/release. Both profiles' files pass 33 explicit world/lifecycle scenarios and 1,248 query records/profile, including 11 required cross-tile positive scenarios and 187 complete cross-tile paths in the fully loaded states/profile. Both load orders and parent/child overlays are exercised. Duplicate coordinates, stale references, removal/reload and empty worlds match; one synthetic four-position selection exceeds its original three-slot root capacity and explicitly matches OUT_OF_MEMORY in both engines. No capacity is silently enlarged. Three focused fixture tests bring navigation to 319 and workspace libraries to 1,913 plus four examples, with clean Clippy; three separate standalone adapter tests and Clippy pass. There are 196 empty runtime network traces. Public successful detail status flags are not exposed by RustyCore and are not claimed. This is bounded explicit synthetic multi-tile acceptance, without original MMapManager source/fallback execution, gameplay bootstrap, real acquisition, product manifests/resume/distribution or full-world evidence. See the [multi-tile contract](forever-navigation.md#multi-tile-detour-and-public-reader-acceptance--2026-10-11).

- **Sequential navigation selection:** The sequential Linux navigation runner now consumes complete Map source records, rooted inventory and explicit liquid/connection inputs. All/map/tile selections emit deterministic provisional globals and tile/empty outcomes; only a returned Completed admits the selection. Two full inventory verifications surround each batch. Requested artifact/directory identities additionally bind every provider read to the original sorted enumeration. Fresh contexts bound per-tile residency; observed mutations permanently poison the package. Both profiles match 12 synthetic packages / 15 global files / 22 selected tiles against original MapBuilder discovery/globals and original public fresh TileBuilder calls per scheduled tile. They retain 20 final files / 1,176,088 bytes, two empty outcomes, and the unchanged RustyCore single-tile Detour query adapter. All nine preceding pipeline statistics/final outputs are identical. Twenty-three focused tests bring navigation to 316; workspace libraries pass 1,910 plus four examples and Clippy. There are 110 empty runtime network traces. This supplies synthetic sequential full-selection execution; it does not run original worker threads, establish multi-tile query linkage, repair real acquisition, or provide product manifests/resume/publication. The real startup/product/runtime/distribution completion requirements remain open. See the [batch contract](forever-navigation.md#sequential-navigation-selection-execution--2026-10-11).

- **Rooted navigation discovery/binding:** canonical Linux source folders now
  supply the plan directly. Complete admitted enumeration, tile-list verification/
  hashes and held directory/file identities bind a fresh provider/store context
  for each tile. Observed mutation permanently poisons the package. Both profiles
  reproduce 82 original MapBuilder inventories / 140 globals / 20,244 inventory
  bytes, with 174 checked admitted tile-list hashes. All nine package-bound
  scenarios retain seven exact final files / 452,012 bytes, every preceding
  pipeline statistic and the unchanged RustyCore Detour query comparisons.
  Fifteen tests bring navigation to 293 and workspace libraries to 1,887 plus four
  examples, with clean Clippy and 200 empty runtime traces. Opening admits the
  inventory; unused model/tile bodies are not thereby verified. Canonical names
  are explicit, metadata checks are not atomic snapshots/content rehashes, and
  per-tile inventory checking scales with observed entries. This closes the
  supplied descriptor/discovery gap for the admitted local profile. Complete
  real startup/acquisition, scalable full-map orchestration and versioned
  product/runtime/distribution gates remain open. See the
  [discovery contract](forever-navigation.md#rooted-package-discovery-and-binding--2026-10-11).

- **Joined single-tile navigation pipeline:** the explicit API consumes the
  validated inventory, immutable terrain/VMap sources, a reduced direct-row
  LiquidType table and explicit connections. It preserves source selection,
  compaction, all 625 subtiles and final fresh-save/header behavior. Both profiles
  match nine complete synthetic scenarios: seven exact final files / 452,012
  bytes plus two empty outcomes against original public C++ buildTile, with an
  explicit fresh-tile virtual selection gate. Current unchanged RustyCore Detour
  APIs load seven/profile through a strict Forever adapter and reproduce one
  complete path, five partial paths and one absent-nearest error/profile. This
  does not use its incompatible legacy MMap 16 parser. Nineteen added tests bring
  navigation to 278, VMap to 472 and workspace libraries to 1,872 plus four example
  tests; Clippy passes. The 75-case store regression is identical. Inventory's
  82 outputs and work/semantic statistics are identical; its component bitsets
  add exactly 1,024 heap bytes/discovered map. There are 634 empty runtime network
  traces across the new pipeline/reader and these regressions. Real discovery,
  DB2 startup/full-map orchestration, product CLI/manifests/resume/distribution and
  real world/runtime acceptance remain open. This supersedes the missing joined
  API gate in earlier component checkpoints, without changing their historical
  scope. See the [pipeline contract](forever-navigation.md#single-tile-source-to-final-pipeline--2026-10-11).

- **Terrain source selection/loading:** the navigation plan retains its build;
  a bounded transfer API and Linux rooted source load complete MAPS files for
  selected output tiles. Each requested center/neighbor walks effective parents
  independently on genuine absence. Present empty geometry suppresses neighbors;
  corruption, read/change failures and build mismatches reject without fallback.
  Debug/release each match actual public original loadMap on **110 synthetic
  cases / 217,721,658 bytes**. The corpus includes both builds, depth64, edge
  coordinates and all three sampled height encodings. Twenty-five added tests
  include the exact 1,024-parent/5,120-read ceiling and rooted source mutation,
  absence, symlink and capacity checks. Navigation passes 264; workspace
  libraries pass 1,853 plus four examples and Clippy. The previous 82-case
  inventory arrays remain identical in both profiles; 388 runtime traces are
  empty. This closes supplied-package terrain selection, while complete real
  inventories, the joined source-to-TileBuilder pipeline and product/runtime
  integration remain open. See the
  [terrain loading contract](forever-navigation.md#rooted-terrain-selection-and-loading--2026-10-11).

- **Active VMap model to navigation bridge:** borrowed loaded-instance iteration
  preserves primitive-slot order, skips null/unloaded models and uses checked
  owned VMO groups/cached transforms without reparsing models or allocating a
  selection list. The tile adapter uses original load/append/unload semantics,
  including removal of all requested-tile references on projection failure.
  Debug/release each match **154 packages / 1,507 operations / 227,004 bytes**
  against actual original VMapManager and TerrainBuilder, including **356 actual
  loadVMap calls** per profile. The previous 97-case supplied-model corpus stays
  identical in both profiles. Eight added tests bring navigation to 239, VMap to
  467 and workspace libraries to 1,828 plus four examples; Clippy passes and all
  506 runtime traces are empty. Missing/corrupt artifacts remain explicit
  failures. Complete real models, terrain ancestor-file selection and unified
  source-to-product/runtime acceptance remain open. See the
  [loaded-model contract](forever-navigation.md#active-vmap-model-loading--2026-10-11).

- **Navigation source and ancestor inventory:** direct Map records, last-ID-wins
  assignment, effective parent fallback, filters, terrain/VMap tile union and
  ultimate-root parameters match original DB2 getters and actual MapBuilder
  discovery/global construction for **82 synthetic packages** in debug/release.
  The corpus contains 602 direct rows, 177 discovered map entries and 4,366
  inventory tiles; all 140 globals / 5,600 bytes match. Strict decoding feeds
  unchanged RustyCore initialization and **3,500 exact coordinate queries**.
  Fifteen inventory tests and three DB2 tests bring navigation to 233, DB2 to
  299 and workspace libraries to 1,820 plus four examples; Clippy passes.
  All 306 runtime network traces are empty. Complete supplied descriptors are
  a caller contract; production filesystem discovery, selected-instance loading
  and real extraction remain pending. See the
  [inventory contract](forever-navigation.md#navigation-source-and-ancestor-inventory--2026-10-11).

- **Complete supplied-geometry TileBuilder:** target config,
  all 625 padded subtiles, every Recast stage, paired merge and pristine Detour
  generation match original `buildMoveMapTile` across 107 configuration cases
  and 23 complete build cases. Twenty-two payloads load; one build is empty.
  Actual RustyCore queries reproduce 11 complete paths, one partial path and
  ten absent-nearest errors. Both profiles match; the earlier MMap case arrays
  remain unchanged, with 1,765 empty runtime network traces. Fourteen
  added tests bring navigation to 208 and workspace libraries to 1,792, plus
  four examples and Clippy. This implements supplied geometry/configuration;
  it does not discover parent inventories or real source geometry.

- **Pure Rust final save state:** the original producer inserts each tile into a
  fresh Detour navmesh before writing. `Data::prepare_for_save` now initializes
  free/internal links and projects local off-mesh endpoints; `encode_saved_tile`
  frames the consumed `SavedData` type. Debug/release each match **747 saved
  payloads / 10,916,236 bytes**, including **all 22 actual final TileBuilder
  files**. Three builder diagnostic cases reject explicitly. Strict MMap 16
  readers feed unchanged RustyCore APIs: 747 loads/removals, 171 query cases,
  158 complete paths, three partial paths and ten absent-nearest errors.
  Ten focused tests bring navigation to 218, workspace libraries to 1,802 plus
  four examples; Clippy passes. This closes the observed final-file gate for
  the bounded synthetic corpus. Fresh single-tile offline save evidence does
  not establish multi-tile runtime state, real source inventories, unified
  product output or high-level world routing. See the
  [save checkpoint](forever-navigation.md#pure-rust-final-save-state--2026-10-11).

- **MMap 16 containers:** explicit global/tile codecs preserve 40/20-byte
  headers, 44-byte off-mesh records, source order/filtering and the original
  signed-minimum maximum-poly sentinel. Borrowed strict readers reject
  incomplete/noncanonical framing and old versions; they do not certify
  arbitrary Detour topology. Debug/release match 762 globals / 3,054,248 bytes
  and 728 tiles / 18,548,220 bytes through original native struct definitions.
  Original Detour initializes all globals and loads/removes every tile. New
  strict readers feed the unchanged RustyCore public APIs with 728 successful
  loads and the same 149 query results (147 complete, two expected partial).
  The legacy tile parser is explicitly checked to reject v16. Sixteen new tests
  bring navigation to 194 and workspace libraries to 1,778, plus four examples
  and Clippy. The 768-case Detour regression remains identical in both profiles;
  3,792 runtime traces are empty. Original complete MapBuilder/MMapManager,
  parameter derivation, world inventory, real inputs and product/runtime
  activation remain outside this evidence. See the
  [container contract](forever-navigation.md#mmap-16-containers--2026-10-11).

- **Detour tile payloads:** pure explicit little-endian version 7 serialization
  matches 728 original C++ outputs / 18,533,660 bytes in both profiles; 40 empty
  cases reject in both. The corpus reuses all 330 merged meshes with BV on/off
  and covers flags, portals, off-mesh classification/culling, BV bounds, index
  boundaries and explicit native truncation diagnostics. Original Detour and
  RustyCore's unchanged public APIs load all 728; 149 query projections match,
  with 147 complete routes and two expected partial routes. Eight authored
  625-grid variants yield 49-polygon paths; gap cases exercise both directions.
  Diagnostic detail wrapping and neighbor marker collisions are counted and
  receive no routing-validity claim. The typed wrapper ignores the truthful
  MMap 16 blob header, so no legacy container compatibility is inferred.
  Nineteen new tests bring navigation to 178 and workspace libraries to 1,762,
  plus four examples and Clippy. Earlier 330/149/656/155/191/164/162 case arrays
  remain identical; 5,894 runtime network traces are empty. Complete TileBuilder,
  MMap 16, real inputs and world routing remain open. See the
  [Detour contract](forever-navigation.md#detour-tile-payloads--2026-10-11).

- **Subtile merging:** consuming detachment releases compact fields/contours;
  `TileMesh::merge` preserves the original polygon and detail merge order,
  coordinate rounding/welding, metadata, exterior portals and rebuilt links.
  Debug/release each match 330 cases / 1,665,448 bytes, reusing all 149 prior
  detail outputs. Four authored 625-subtile grids use target widths 40/80 and
  both orders; each has 676 welded vertices, 100 outer portals and 2,400 linked
  edges. All detail arrays are copied exactly with rebased descriptors. A
  separate aggregate memory/work policy preserves prior work provenance.
  Eighteen new tests bring navigation to 159 and workspace libraries to 1,743,
  plus four examples and Clippy. Prior 149/656/155/191/164/162 case arrays remain
  identical; 3,628 runtime network traces are empty. Complete TileBuilder,
  Detour/MMap, real inputs and routing still require acceptance. See the
  [merge contract](forever-navigation.md#subtile-merging--2026-10-11).

- **Detail mesh:** `PolyMesh::build_detail` preserves original height-patch
  seeding/fill, ordered sample lookup, edge refinement, hull triangulation,
  jittered interior samples, Delaunay recovery, world-coordinate bits and flags.
  Debug/release each match 149 synthetic supplied-span cases / 6,477,240 bytes:
  613 submeshes, 6,637 vertices and 6,736 triangles. Eight cases explicitly
  supply region overrides for center seeding. One dense case matches six
  native dangling-face warnings and complete recovered output; this is not
  a routing validity claim. Sixteen added tests bring navigation to 141 and
  workspace libraries to 1,725, plus four examples and Clippy. All prior
  656/155/191/164/162 case arrays remain identical; 2,966 network traces are
  empty. Merge evidence is recorded above; Detour/MMap, complete real inputs
  and runtime acceptance remain. See the [detail contract](forever-navigation.md#detail-mesh--2026-10-11).

- **Polygon mesh:** `ContourSet::build_mesh` now preserves original ordered
  triangulation, vertex welding, convex merges, border removal/repair,
  mixed-region metadata, linked adjacency and portals. Debug/release each
  match 652 supplied-contour cases / 688,088 bytes for polygon widths 3–6.
  Four reversed-outline cases trigger native partial-output warnings and
  explicit Rust rejection. All positive linked edges pass reciprocal checks.
  Seventeen new tests bring navigation to 125 and workspace libraries to
  1,709, with four examples and Clippy. All 155/191/164/162 prior stage arrays
  remain identical; 2,666 network traces are empty. Detail evidence is recorded
  above, together with merging; Detour/MMap remains pending. See the
  [polygon contract](forever-navigation.md#polygon-mesh--2026-10-11).

- **Contour geometry:** a consuming Rust API now preserves original raw corners,
  f32 simplification, wall/area subdivision, flags and hole joins. Debug/release
  each match 155 cases / 5,896,408 bytes, including 633 contour entries and 19
  merged hole slots. Of these cases, 152 use generated watershed regions; three
  independently supply single-region labels to exercise 2, 3 and 8 holes.
  Undefined native scratch reads and unresolved or incomplete outputs reject.
  Sort-tie evidence remains scoped to the pinned Linux profile. Fourteen new
  tests bring navigation to 108 and workspace libraries to 1,692, with four
  examples and Clippy. All 191/164/162 earlier stage arrays remain identical;
  1,352 runtime network traces are empty. Polygon evidence is recorded above;
  detail and merge evidence is also recorded above; Detour/MMap remains. See the
  [contour contract](forever-navigation.md#contour-geometry--2026-10-11).

- **Watershed regions:** the actual original algorithm now has a consuming Rust
  API for level buckets, staged expansion, flood, adjacency/floors, component
  filtering, merges and remapping. Debug/release match 191 cases / 13,870,280
  bytes, including two native nonzero overlap warnings. Three paired cases
  exercise threshold-dependent merges. Empty native IDs and ordered border
  paints are preserved. Border-flag collisions and unfinished native adjacency
  walks reject explicitly. Thirteen added tests bring navigation to 94 and
  workspace libraries to 1,678, with four examples and Clippy. The 164/162 prior
  compact/heightfield case arrays remain identical; all 1,040 traces are empty.
  That checkpoint covers region construction; contour evidence is recorded
  above. See the [region contract](forever-navigation.md#watershed-regions--2026-10-11).

- **Recast compact preparation:** consuming erosion, median area filtering and
  distance-field generation match 164 synthetic cases / 65,069,060 bytes per
  profile against original Recast. The 634 intermediate snapshots preserve
  graph geometry/connections; 69 composed `prepare` calls agree. Cases cover
  erosion saturation, distance 512, area boundaries, null spans and layers.
  Radius >127 rejects the native threshold-wrap domain. Slope defaults are
  corrected to actual native 55/55; explicit 70/55 preserves the previous
  162-case regression in both profiles. Seventeen added tests bring navigation
  to 81 and workspace libraries to 1,665, with four examples and Clippy.
  All 656 runtime network traces are empty. Later region and contour evidence
  is recorded above, together with polygon/detail evidence; Detour/MMap remains pending. See the
  [compact preparation contract](forever-navigation.md#compact-area-and-distance-processing--2026-10-11).

- **Initial Recast heightfield:** the consuming Rust API now classifies and
  rasterizes solids, applies the three ordered filters, rasterizes liquids and
  constructs compact-cell connections. Debug/release each match 162 synthetic
  cases / 2,628,564 bytes against original Recast, including six intermediate
  fields per case and 37 composed API cases. Strict numeric/layer/resource
  rejection is separate from positive native parity. Twenty-three added tests
  bring navigation to 64 and workspace libraries to 1,648; four examples,
  Clippy and scoped formatting pass. All 326 runtime network traces are empty.
  No full TileBuilder, erosion/regions/polygons, Detour/MMap or route acceptance
  is claimed. See the [heightfield contract](forever-navigation.md#initial-recast-heightfield--2026-10-11).

- **Navigation model/off-mesh input:** a consuming append API combines a terrain
  prefix with supplied VMO/placement solids and rectangular liquid grids using
  the actual checked VMap inverse. M2 winding reverses; WMO liquid unknown types
  retain area zero. The supplied direct-row lookup does not replay DB2 copies.
  Off-mesh selection preserves exact target/order/duplicates and source bits.
  Both profiles match 97 cases / 3,951,732 bytes, with 239 model/placement reads,
  336 groups, nine terrain prefixes and 67,552 selected connections, including
  the exact 65,536-record ceiling. The previous 106 terrain case arrays remain
  unchanged in both profiles. Twenty added tests bring navigation to 41 and
  workspace libraries to 1,625; four examples and all-target Clippy pass.
  Actual selected-instance inventory, source acquisition, Recast/Detour output
  and real routes remain gates. See the
  [model input contract](forever-navigation.md#supplied-model-and-off-mesh-input--2026-10-11).

- **Navigation terrain input:** the new pure `forever-navigation` crate consumes
  verified MAPS and projects indexed solids/liquids, all border portions and an
  explicit center/neighborhood selection, followed by optional first-encounter
  vertex compaction. It retains original holes, dark-water exclusion, partial
  liquid repair and surface selection, with strict input/finite/resource gates.
  Both profiles match **106 synthetic cases / 50,763,162 bytes**, including
  **2,948,213 vertices / 1,246,502 triangles**, against original TerrainBuilder
  region loading, public neighbor dispatch and compaction. There are 21 focused
  unit tests and one explicit corpus test; the workspace passes 1,605 library
  tests, four examples and all-target Clippy. No real source, VMap geometry,
  Recast/Detour generator or pathfinding runtime is accepted by this increment.
  See the [navigation contract and exact reports](forever-navigation.md).

- **Local terrain execution:** the runner now loads all four complete startup
  DB2s, acquires WDT/root ADTs through one Rust LocalStore, converts MAPS and
  emits whole-map tile lists to a provisional sink. The Linux QA command
  rejects source/conversion failures before output creation; any sink failure
  rejects the entire selection. Tile subsets cannot claim whole-map lists.
  Debug/release each pass 20 synthetic CLI scenarios, 20 existing-output guards
  and three boundary/source guards. Both match 48 MAPS / 24 tile lists against
  C++, plus 6,864 exact actual GridMap queries/profile. Workspace acceptance
  passes 1,584 library tests, four example tests and all-target Clippy.
  `terrain-local-closeout-20261011T070801Z.json` retains the complete evidence.
  The actual two-tile probe fails its first Map read with
  `local_store_no_verified_encoding`, no converted tiles/output/network.
  This implements the local path but does not establish real complete terrain,
  native CASC/DB2/WDT orchestration parity, product manifests or runtime queries.
  See the [local runner contract](forever-terrain.md#local-terrain-acquisition-and-map-execution).

- **Checked RustyCore terrain library:** optional Linux `forever-terrain`
  composes actual GridMap and rooted VMAP readers with checked static height,
  LOS/object-hit and shared tile leases. All 23 focused tests pass, including
  1,210 exact C++ scalar height-selector cases and three production-library
  integration tests. Full map/world/data/database suites pass 770/4,490/971/392
  tests respectively; default-feature workspace all-target compilation passes.
  General final validation retains inherited architecture/format failures.
  Closeout `forever-terrain-closeout-20261011T064614Z.json` records all attempts
  and source hashes. Complete real topology/geometry, server lifecycle wiring,
  area/liquid/dynamic queries and navigation remain open. The source-path
  dependencies are laboratory pins, not distribution acceptance. See the
  [checked reader contract](forever-collision.md#checked-rustycore-terrain-library).

- **DB2 catalog:** `catalog21-accept-20261011T003802Z-8d87411a.json`
  records byte-identical Rust/Python dumps for 21 additional build-70205/esES
  files and 5,302 rows. Six files are empty; `LiquidType` exercises string
  arrays on real input. Source, binary and input hashes remained unchanged;
  syscall denial recorded no network attempt. This invokes the diagnostic
  decoder API, not public typed loaders or a catalog production manifest.
  All 1,126 pinned metadata shapes have synthetic regular/empty coverage;
  this does not establish all physical WDC5 variants. The extractor's source
  file list has 1,132 entries and differs from the metadata catalog by seven
  list-only and one metadata-only table, so the two inventories are distinct.
- **GT runtime groups:** `gt6-accept-20261011T003319Z-eca4882f.json`
  records six complete files (85,794 bytes), successful extraction and v1
  verification, exact comparison of 15,988 f32 cells with the pinned Linux
  C++ numeric behavior, and complete calls to `InitialGameTables::load` and
  `SpellValueGameTables::load`. Counts include six implicit zero rows.
  Synthetic positive inputs and four failing probes also pass. Source,
  binary, client metadata and integration checkout guards remained stable;
  no network attempt was recorded. This does not cover the remaining 14
  files, Windows numeric conversion or gameplay.
- **DB2 production catalog batch:**
  `catalog-v3-accept-20261011T004108Z-aa77d0a1.json` subsequently re-acquired
  all 21 additional files, validated the v3 schema and output verifier, and
  reproduced every input hash, full Rust/Python dump and 5,302 decoded rows
  from the diagnostic run. Source, binary and client guards passed; network
  denial recorded no attempt. This is one explicit catalog batch, without
  complete typed-loader, encrypted-section or all-1,126-table acceptance.
- **GT complete extraction catalog:**
  `gt20-accept-20261011T004112Z-6ad435fc.json` records all 20 complete files
  (138,537 bytes), successful v2 schema/output verification, exact comparison
  of all 22,082 f32 cells in the 14 source-backed numeric tables, and both
  public RustyCore GT loaders for their six-file subset. Six catalog files
  remain raw-only; no numeric schema is invented for them. Synthetic
  all-column fixtures and four negative probes pass, including the 32-column
  `CombatRatings` shape and a known C++/Rust narrowing difference. All
  source, binary, client and integration guards passed; no network attempt
  occurred. Full gameplay and the Windows conversion branch remain outside
  this evidence.
- **Rust CASC pieces:** 30 BLTE tests cover bounded N/Z decoding, frame MD5,
  optional content MD5, zlib integrity, exact extents and malformed inputs.
  Twenty-four local-index tests cover version 7, explicit contiguous/paged
  layouts, limits, hashes, offset geometry and generation selection.
  `jenkins-differential-20261011-v1/report.json` in the external candidate
  laboratory records 1,428 exact synthetic comparisons against pinned
  `lookup3.c` and five checks of the pinned page predicate. The stricter
  paged parser intentionally rejects corruption that C++ treats as an end;
  it does not claim whole-loader parity. The subsequent store API evidence
  below integrates these pieces for the explicit N/Z profile. BLTE E/F remain rejected.
  A further 15 synthetic tests pass for the archive-record adapter: exact
  planned ranges, mandatory full CKey, index EKey-prefix comparison and the
  pinned 30-byte prelude behavior. Full EKey authentication is not claimed;
  the reference ignores the prelude's unreliable key/checksum fields.
- **Local index acquisition:**
  `local-index-diagnostic-20261011T004720Z-152b823c.json` records all 16 latest
  local indexes (25,165,824 bytes / 1,328,926 entries) parsed as explicit
  contiguous v7 files with valid Blizzard checksums. Explicit paged-layout
  probes reject these files; no layout fallback occurs. Input/source guards
  passed and no network attempt occurred.
- **Pure Rust ENCODING acquisition:**
  `encoding-bootstrap-20261011T005242Z-dba696cd.json` records admitted build
  metadata, MD5-bound config, local indexes, an exact archive span and BLTE
  N/Z decoding. All 160,565,873 decoded bytes match pinned CascLib. Full
  content MD5 and 23,429 CKey page hashes validate 2,506,807 content routes
  and their encoded keys. ESpec and the post-CKey tail remain opaque; this
  is not full ENCODING semantics or an integrated FDID backend. Source and
  input guards passed with no network attempt. An earlier diagnostic failed
  its 1,024-assignment config budget; the successful run explicitly budgets
  4,096 assignments and retains that failed report. Twenty config, 24
  ENCODING and 26 ROOT synthetic tests also pass. ROOT selection received an
  independent source audit against the same pin.
- **Pure Rust ROOT acquisition:**
  `root-bootstrap-20261011T005813Z-7b8c65b8.json` subsequently resolved ROOT
  through the authenticated ENCODING, selected local index and complete
  archive span. All 56,903,464 decoded bytes match pinned CascLib. The
  explicit `Header50893V2` parser validates 1,192 blocks and 2,747,538 entries,
  including 241,552 stored name hashes. Full CKey validation, source/input
  guards and network denial pass. This establishes complete ROOT acquisition
  and parsing for this input; subsequent FDID store evidence is recorded below.
- **Complete raw camera component:**
  `camera-accept-20261011T005622Z-9564f358.json` records ten cameras plus the
  666-byte source DB2, 26,962 bytes total. All eleven artifacts match pinned
  C++ reads byte for byte. The independent Python source projection matches
  all ten discovered IDs and the C++ filename rule; extraction, v1 schema
  and output verifier pass. Source, binary and client guards passed with
  no network attempt. M2 parsing, waypoints and runtime playback were not
  executed and remain unclaimed.
- **Pure Rust local-store API:**
  `local-store-accept-20261011T011040Z-c84f6f43.json` records 59 requests,
  including 57 complete files / 528,633 bytes equal to previously accepted
  pinned C++ artifacts. The selection covers DB2, all 20 GT files and ten
  cameras plus their source DB2. `Map` and `SkillLineAbility` remain rejected
  without output. Index, ENCODING and ROOT metadata stay shared in one store;
  its logical resident payload is 341,831,934 bytes, not an RSS measurement.
  Source, client and reference guards pass; no network attempt occurred.
  This exercises the API and explicit contiguous-v7/Header50893V2/N/Z
  profile, before production CLI integration. It does not admit encryption,
  missing ranges or all CASC variants.
- **Bounded sparse DB2:**
  `sparse-conversation-accept-20261011T011040Z-24845f24.json` records a
  production v3 catalog run for `ConversationLine`: 541 direct records plus
  17 copies, 558 rows total, with full Rust/Python dump equality, exact pinned
  C++ input bytes and valid schema/output verification. Four unsupported
  probes remain failed batches without artifacts. Source, binary and client
  guards pass with no network attempt. This admits only the documented
  single-section scalar sparse profile. RustyCore's generic reader rejects
  WDC5 sparse, so this is not typed or runtime acceptance.
- **Terrain foundation:** the dependency-free `forever-terrain` crate has
  43 passing synthetic tests and clean Clippy for WDT/ADT framing and bounded
  height, area and hole projection. The report
  `terrain-projection-20261011T011040Z-08a187f0.json` records 47 exact synthetic
  comparisons of all projected grids against an independent transcription
  of the pinned C++ arithmetic, plus three nonfinite-input rejections without
  output. It does not execute the complete C++ ADT parser. No client terrain,
  liquids, height packing or MAPS writer is covered. See
  [the bounded contract](forever-terrain.md), including stricter framing and
  the Map DB2 consumer's distinct copy semantics.
- **Height section serialization:**
  `terrain-section-20261011T012203Z-b532194b.json` records 126 exact synthetic
  MHGT byte comparisons and eight rejected nonfinite/budget cases without
  output. It covers flat, uint8, uint16 and float32 encoding, strict threshold
  boundaries, signed zero, extrema sentinels, the fixed -2000 clamp, output
  order and optional MFBO flight bounds. The independent C++ kernel is an
  original transcription of the pinned arithmetic and serialization, not the
  complete ADT extractor or runtime reader. All source/binary guards pass.
  The expanded terrain crate has 65 passing tests and clean Clippy; this
  does not establish complete MAPS, liquid or runtime compatibility.
- **Liquid projection:**
  `terrain-liquid-20261011T013406Z-d88b629b.json` records 99 exact synthetic
  comparisons of all visibility, entry, flag and height matrices against an
  independent original C++ kernel checked against the pinned source. Ten
  malformed-input cases reject in both QA callers; nine nonfinite or unknown
  vertex-format cases reject in Rust while C++ retains its documented behavior.
  Both callers preserve pre-existing output files. Source and binary guards
  pass. This covers MCLQ/MH2O projection, not MLIQ packing, the complete C++ ADT
  parser, real-client terrain or a runtime reader.
- **Closed terrain DB2 sources:**
  `terrain-sources-20261011T013829Z-455396ac.json` records 89 positive and
  36 negative synthetic cases plus two complete local files against the
  independent Python source/copy/materialization oracle. LiquidType has 53
  direct rows and no copies; LiquidMaterial has four direct rows and one copy,
  producing five final entries. The Map consumer's distinct source-only copy
  index is preserved, and selected null string pointers reject explicitly;
  strings pointing to an empty terminator remain valid. The shared decoder
  has 260 passing Rust tests, 181 Python tests (one skip), and clean Clippy.
  Real Map input remains incomplete and its reader is not invoked. Source,
  binary and prior-input guards pass without network attempts. No C++ typed
  loader or full terrain runtime is executed by this comparison.
- **MLIQ serialization:**
  `terrain-liquid-section-20261011T014336Z-5daf200e.json` records 311 exact
  synthetic section comparisons and ten rejected finite/budget/visibility
  cases. It includes 198 combinations from the 99 accepted liquid projections
  with both packing settings, empty crops, omitted sections, signed zero,
  strict thresholds, invisible height replacement and untouched outer borders.
  All source/binary/input guards pass. The independent original C++ kernel
  follows the pinned serialization; it is not the complete C++ ADT parser.
- **Actual GridMap height reader:**
  `terrain-grid-20261011T014347Z-e6d7456e.json` records 378 synthetic containers
  and 54,054 exact height-query bit comparisons using RustyCore's unchanged
  `GridMap::parse/get_height` API and an independent pinned C++ query kernel.
  The containers wrap 126 accepted MHGT sections with absent, zero or patterned
  holes. Six malformed cases reject in both readers. A truncated flight tail
  is accepted by RustyCore's height-only reader and rejected by the C++ kernel;
  the strict MAPS verifier rejects it. All guards pass. This does not accept
  higher-level tile selection, area/liquid/flight queries or real terrain.
- **Typed MAPS assembly:**
  `terrain-map-20261011T014623Z-650148fb.json` records 380 complete synthetic
  files from the production typed assembler, matching an independent layout
  composed with previously accepted C++ MHGT/MLIQ sections. All pass the strict
  production verifier and 54,340 exact height-query bit comparisons through
  actual RustyCore GridMap versus the C++ kernel. Six negative cases create no
  output. It covers both admitted build tags, optional and omitted liquids,
  uniform/full areas, holes including present-zero masks, unaligned section
  offsets and the 68-byte minimal container. Synthetic build-70170 tags do not
  establish client compatibility. All guards pass; terrain has 134 passing
  tests and clean Clippy. An earlier retained report failed only because its
  minimal fixture accidentally included the optional 36-byte flight tail;
  correcting that fixture and rerunning the full plan produced this report.
  Complete ADT conversion, real acquisition and full C++ extractor/runtime
  acceptance remain separate work.
- **Regression after terrain/source integration:** the workspace passes 340
  CASC, 260 DB2 and 134 terrain tests and Clippy across all targets. The native
  helper separately passes 341 CASC tests, three probe tests and 239 Python
  checks with six explicit skips. These regression commands do not select
  real-client CLI tests; the separately recorded local comparisons above
  retain their own evidence and limits.
- **Integrated supplied-ADT conversion:** the pure `conversion::convert_adt`
  API composes complete framed input, height/area/hole and liquid projections,
  unique MFBO flight bounds, one shared packing setting and strict MAPS
  assembly. Context and reducible limits validate before parsing; statistics
  retain coverage, repeated cells, missing MCVT and optional sections. Twenty
  additional synthetic tests pass, bringing terrain to 154; the example builds,
  workspace Clippy is clean and independent source review has no findings.
  Its subsequent original-C++ comparison is recorded below. This API does not
  acquire ADT bytes, authenticate WDT associations or establish real terrain
  or complete runtime compatibility.
- **Original C++ ADT conversion:**
  `native-adt-20261011T015737Z-de5fe310.json` records 214 complete synthetic
  ADT-to-MAPS byte comparisons against the unchanged pinned `parseChunks`,
  `prepareLoadedData` and `ConvertADT(ChunkedFile&, ...)`. All outputs pass the
  strict Rust MAPS verifier and 30,602 height queries through the unchanged
  RustyCore `GridMap` API against the independent C++ query kernel. Twenty-two
  malformed/policy probes run only in Rust and reject without output; both
  callers preserve pre-existing outputs. The reference build uses immutable
  Git blobs, original notices, a checked transitive include inventory and
  GCC 15.2 with floating contraction/fast math disabled. Link inspection
  confirms retained parser/converter functions and discarded CASC/DB2 APIs;
  the inert storage global and its shared-pointer lifecycle remain. The local
  Boost path shim only typechecks discarded CLI code and performs no I/O.
  Source, binary, original-input and prior-fixture guards pass; network syscall
  denial records no attempts. This executes the original parser on trusted,
  structurally valid synthetic fixtures, without a malformed-input safety or
  rejection-parity claim. CASC/WDT acquisition, higher-level tile selection,
  real terrain and area/liquid/flight runtime queries remain untested here.
- **Dense original-C++ ADT conversion:**
  `native-adt-dense-20261011T020000Z-00dd5c2f.json` adds 34 complete synthetic
  comparisons and 4,862 exact GridMap query results using the same pinned
  original parser/converter and guards. Full 256-cell old-liquid fixtures
  exercise uniform types, invisible crops and omitted sections; six dense
  MH2O vertex formats and a mixed full-section fixture exercise both packing
  settings. This supplements the 214-case run without changing its scope
  or establishing real acquisition or additional runtime APIs.
- **Pure terrain dependency planner:** 22 focused tests and package Clippy
  across all targets pass for materialized Map selection followed by
  sequential WDT expansion. All-map/full-map plans include every coordinate;
  explicit tile plans preserve their subset scope and absent cells. Source
  order, signed FDID bits, aliases, logical-name versus output-name rules,
  reducible capacity/count budgets and poisoned failure state are covered.
  Independent source review found no functional defect. The planner performs
  no I/O, authenticates no FDID-to-WDT binding and publishes no tile lists;
  a completed dependency plan is not a completed extraction.
- **M2 collision foundation:**
  `native-m2-20261011T020651Z-78669c6e.json` records 53 exact synthetic raw04E
  comparisons against original C++ `fixCoordSystem` and
  `Model::ConvertToVMAPModel`, 18 Rust-only rejections, two no-collision
  outcomes and both callers preserving existing outputs. The Rust raw verifier
  runs before publication; all source/binary/reference guards pass and no
  network attempt occurs. The original writer receives supplied geometry;
  `Model::open`, real assets, the original assembler reader, WMO, placements,
  BIH and RustyCore collision runtime are not executed. The dependency-free
  crate has 33 passing tests and an independent clean source audit. Workspace
  regression passes 362 CASC, 260 DB2, 154 terrain and 33 VMap tests (809 total),
  plus Clippy across all targets. See [the collision contract](forever-collision.md).
- **Complete catalog survey:**
  `catalog1126-survey-20261011T004624Z-a6ba728c.json` finished all 36 batches
  and 1,126 requested tables. Its frozen tools decoded 1,023 complete artifacts
  and 719,466 rows, each matching the Python oracle. The other 103 inputs report
  82 incomplete local files, three record-policy limits, six unsupported record
  variants, 11 unsupported schema cases and one invalid string. Failed batches
  remain failed; individual complete artifacts do not turn them into successful
  batches. The binaries predate the separate sparse and text-budget changes;
  their newer coverage is not attributed to this survey. Client metadata guards
  pass and no network attempt occurs. No typed loader or full extraction is
  established by these counts.
- **Rust-local production commands:**
  `rust-cli-accept-20261011T012203Z-2aa558be.json` records five successful
  build-70205/esES plans: two reviewed DB2s, 22 catalog DB2s (including sparse
  `ConversationLine`), six GT files, the complete 20-file GT catalog and the
  camera source plus ten cameras. All 61 artifact comparisons match previously
  accepted C++ outputs; these plans overlap and are not 61 distinct inputs.
  DB2 v4, GT v3 and camera v2 schemas and offline verifiers pass. The GT run
  repeats all 22,082 numeric comparisons and both public six-file loaders.
  Binaries have no native feature; native remains the CLI default and is
  rejected before I/O when unavailable. Two incomplete inputs, three reduced
  byte-budget runs, 15 early option/capability probes, two wrong-layout
  bootstrap probes and 15 tampered profiles all reject as expected. Source,
  binary, client and reference guards pass; no network attempt occurs. This
  establishes the explicit contiguous-v7/Header50893V2/N/Z production profile,
  without encrypted CASC, build-70170 or full-extractor acceptance.

- **Conservative name API:** 32 pure index tests and 14 synthetic LocalStore
  tests pass. `name-index-20261011T021550Z-2ee1ce9d.json` records 135 accepted
  cases / 3,468 exact hash, FDID and selected-CKey comparisons against original
  FileTree and CalcFileNameHash, plus two Rust-only ambiguity rejections.
  The local report `local-name-accept-20261011T021802Z-675f74b5.json` confirms
  successful activation and 52 matching absent-name outcomes on 70205/esES.
  It reads zero complete named files: positive real named-file acquisition is
  still unproved. Source, binary, client and original-source guards pass with
  no network attempt. Session, native product bridge and manifest/CLI name
  integration remain pending. See [the local CASC contract](forever-local-casc.md).
- **Borrowed WMO geometry:** 27 synthetic tests cover complete root/group
  framing, detailed GFID inventory, skips and mutable split IDs, collision
  filtering, compacted vertices/indices, MOBA values and structural doodad/MLIQ
  views. The filter test checks every u16 material-flag value. Independent
  review found and fixed GFID-before-MOHD admission and a potential 32-bit
  offset overflow; both have regressions. At that stage no original WMO geometry execution,
  raw WMO conversion, liquid-type mapping, doodad acquisition or runtime reader
  was claimed. That workspace run passed 408 CASC, 260 DB2, 154 terrain and
  60 VMap tests (882 total), plus Clippy across all targets with warnings denied.
- **WMO liquids and complete supplied raw conversion:** 13 liquid and 19 raw
  tests bring VMap to 92 and the workspace to **914 passing library tests**,
  with all-target Clippy denying warnings. Liquid projection preserves direct
  and legacy types, tile fallback, ocean mapping and VisualOnly filtering.
  The bounded converter retains split/skip ordering and the raw INDX length
  discrepancy; its strict reader checks framing, totals, indices and the
  original assembler's liquid-height storage shape. It does not execute that
  assembler. `native-wmo-20261011T023236Z-4ba2f941.json` records **155 exact
  synthetic raw outputs / 1,589,618 bytes**, five matching no-collision outcomes,
  ten Rust-only rejections and two exclusive-output checks. Original pinned
  `WMORoot::open` runs 160 times and `WMOGroup::open` 171 times, followed by
  original type mapping, skips and writers. The independently framed synthetic
  root, complete-memory CASCFile adapter, synthetic liquid lookup and guarded
  unused doodad-acquisition symbol are explicit harness boundaries. All source,
  binary and dependency guards pass, including wrapper stability during build
  and the executed native binary matching its build report; network tracing
  records no attempts.
  No original DB2 loading, native CASC backend, complete exporter, doodad
  acquisition, assembler, real models or RustyCore collision runtime executes.
  See [the collision contract](forever-collision.md).
- **Original M2 input parsing:**
  `native-m2-read-20261011T024720Z-df38bfbd.json` records 55 executions of
  original `Model::open` using complete synthetic memory inputs. The 53
  collision outputs total 898,640 bytes and match Rust exactly; two inputs
  agree on no collision. Eighteen Rust-only negatives and two exclusive-output
  checks pass. This extends the earlier writer-only comparison to the original
  input parser. It executes neither native CASC nor `ExtractSingleModel`, and
  provides no real-client, assembler or runtime acceptance.
- **Original basename normalization:**
  `model-name-vectors-20261011T023835Z-b9494a9f.json` records 796 vectors from
  original `GetPlainName`/`NormalizeFileName`, all matched by the current Rust
  naming unit test and the focused replay recorded in
  `model-name-comparison-20261011T025056Z-db140b5d.json`. Legacy M2 extension rewriting has separate Rust/source
  regressions; no original C++ rewrite execution is attributed to these vectors.
- **Pure doodads and integrated collision acquisition:** nineteen pure doodad
  tests cover ordered MODN/MODI requests, validated-name membership, non-skipped
  group references, sorted deduplication, set overlap and doodad-only outcomes.
  The public `forever_casc::collision` collector now connects LocalStore reads
  to M2/WMO conversion, eagerly processes all root dependency requests and
  detailed groups, and retains owned raw/source/binding/reference metadata.
  Any failed request poisons finish; no partial extraction escapes. Explicit
  FDID/name routing has no textual FILE-name fallback, cache aliases fail
  unless class and canonical origin match, and dependency cache hits consume
  a separate bounded request count. Thirty-three additional CASC tests include
  four naming tests. The current coordinated workspace passes **966 library
  tests**: 441 CASC, 260 DB2, 154 terrain and 111 VMap, plus two example tests.
  All-target Clippy passes with warnings denied after three test-only lint
  corrections. `collision-workspace-20261011T025127Z-ade49eb5.json` records
  the final check snapshot.
  The `collision_local` QA example accepts an explicit 64 KiB/256-request plan,
  uses esES/contiguous-v7/Header50893V2 with optional name activation, supplies
  bounded materialized liquid tables for WMO requests and publishes private
  ordinal raw outputs only after finish succeeds. It does not reproduce
  GameObjectDisplayInfo startup or define a product manifest/profile. Real
  WMO acceptance, placement transforms, assembly and RustyCore collision
  runtime remain pending. See the
  [collision contract](forever-collision.md) for budgets and API boundaries.
- **Bounded real M2 acceptance:**
  `collision-live-20261011T024957Z-933c1b49.json` records 31 complete
  build-70205/esES files / 15,277,222 bytes matching strict native CASC.
  Original `Model::open` and writing agree with Rust on 28 raw outputs /
  27,100 bytes and three no-collision outcomes; the integrated collector
  independently reacquires and reproduces all 31. The sample comes from the
  individually accepted `CreatureModelData` artifact, without upgrading its
  failed historical batch. The original 32-request plan includes one absent
  file; its collector run rejects before publishing any output. Existing
  successful output is preserved on a repeated destination. All private
  correlation fields, content MD5s and the exact 29-file output inventory
  are checked locally. Source/binary/input/client guards pass, with no network
  attempt. This covers explicit M2 FDIDs, not full discovery, real WMO,
  placements, assembled VMap, product manifests or runtime.

- **Direct MDDF/MODF placement and dir_bin:** 29 additional pure tests bring
  VMap to 140 tests. `native-placement-20261011T025944Z-fce79711.json` compares
  205 synthetic records / 15,710 bytes against the original direct projectors
  (91 M2, 114 WMO), independent Python serialization and 205 original
  ModelSpawn reader/writer roundtrips. Twenty-five Rust-only negatives and two
  existing-output checks pass. The native wrapper observes 125 optional cache
  entries; Rust cache replay and the global ID allocator are excluded. The
  external closure has 62 original source/notice files, 57 actual dependency
  inputs and 22 G3D headers, without compiled G3D implementation units.
  Source, binary, build and network guards pass. The pure API preserves
  caller-assigned IDs, global WMO float arithmetic, NameSet truncation, zero
  scales and complete EOF framing. It performs no placement discovery,
  acquisition, quaternion doodad transforms, parent traversal or BIH.
  `placement-workspace-20261011T030011Z-9b026ecc.json` records **995 library
  tests**, **two example tests** and clean all-target Clippy with warnings denied.

- **Object sources, IDs, doodad transforms and placement bridge:** 59 further
  pure VMap tests cover complete ADT/WDT chunks, serial ID allocation and
  quaternion projection. `native-doodad-20261011T031311Z-18d49b22.json` matches
  573 synthetic outputs / 31,894 bytes and original ModelSpawn roundtrips,
  plus 430 native cache observations, 22 Rust-only negatives and two exclusive
  output cases. `native-object-ids-20261011T031311Z-8df167b0.json` matches ten
  sequences / 96,576 requests against original C++ and a Python state model;
  two numeric collision cases record Rust's deliberate stronger rejection.
  Both corpora also pass optimized release, recorded in
  `native-doodad-release-20261011T031403Z-0a14b6d2.json` and
  `native-object-ids-release-20261011T031403Z-8295fe73.json`. Source, build,
  binary and network guards pass. Doodad QA compiles original external
  Matrix3/Quat units with retained notices; ID QA uses archive-verified local
  Boost headers. No dependency download or source import into Git occurs.
  The pure bridge validates exact model identities, binds WMO dependencies,
  shares one serial allocator and returns complete bounded dir_bin batches.
  Its 29 tests include four synthetic complete LocalStore/converter/placement
  paths. Sorted references, repeated-input counter consumption and conservative
  memory reservation are explicit; parent-cache replay and native unordered
  or threaded emission order are not claimed. No real placement acceptance,
  full discovery, assembled VMap or collision runtime is established.
  `objects-workspace-20261011T031435Z-07179263.json` records **1,083 library
  tests** (470 CASC, 260 DB2, 154 terrain, 199 VMap), **two example tests**
  and clean all-target Clippy with warnings denied.

- **Map planning and local object execution:** the separate VMap DB2 projection
  preserves effective signed parents, direct-source copies and WDT-zero rows.
  `vmap-sources-20261011T032141Z-34d1c1a1.json` records 105 positive and 13
  negative synthetic Rust/Python comparisons. The graph/WDT planner adds 34
  tests and strict parent validation, with deterministic serial Obj0 selection.
  Persistent placement sessions add nine tests; cache replay adds twelve.
  `native-placement-cache-20261011T032335Z-2d96d15e.json` compares 124 cases /
  11,144 records / 650,970 bytes against original `initFromCache`, including
  three empty caches, with source/build/binary/network guards. The original
  ADT parser, ID allocator and CASC backend are excluded from that comparison.
  The integrated local runner adds 20 synthetic tests, joining WDT globals,
  Obj0ADT acquisition, actual model conversion, one ID session and parent-cache
  replay. Only own ADTs populate parent caches, including empty outputs;
  inherited tiles do not populate a child's cache. Requested read failures
  are fatal. Callbacks are staging only, with cross-batch alias checks and
  publication owned by the caller. No real Map/object acceptance or original
  complete-exporter execution is claimed. The final snapshot
  `object-orchestration-workspace-20261011T032907Z-4d6328eb.json` passes
  **1,172 library tests** (533 CASC, 274 DB2, 154 terrain, 211 VMap), two
  example tests and all-target Clippy. Source hashes remain unchanged through
  both workspace checks. The Python suite runs 185 tests with one skipped.

- **Bounded BIH construction and storage:** 27 tests cover the complete supplied
  bounds builder and strict borrowed reader. The producer preserves native
  f32 operation order, partition swaps, signed zero, BVH2 and absent-child
  sentinels; a geometric witness reaches depth 65. The reader validates graph,
  ranges, coverage, object permutation and EOF, with stronger rejection than
  native fread. It cannot prove containment without supplied geometry.
  `native-bih-debug-20261011T033906Z-47418c1c.json` and
  `native-bih-release-20261011T033906Z-5e3fa9da.json` each match 420 complete
  outputs / 245,480 primitives / 5,545,784 bytes, including original BIH
  read/write roundtrips, 35 Rust-only negatives and an existing-stage check.
  The external closure retains 66 original source/notice files, 61 actual
  dependency inputs and three original G3D implementation units, without
  replacing the algorithm or support symbols. G3D release configuration is
  explicit because debug assertions reject empty NaN bounds during reading.
  Source/build/inventory/binary/fixture guards and complete run-linked network
  traces pass. No real geometry, query traversal, model parsing, VMap assembly
  or RustyCore collision runtime executes. The final snapshot
  `bih-workspace-20261011T033949Z-19cedf95.json` passes **1,199 library tests**
  (533 CASC, 274 DB2, 154 terrain, 238 VMap), two example tests and all-target
  Clippy, with unchanged source hashes during both workspace checks.

Reports above are retained outside the repository; raw assets and full decoded
comparisons remain in the private run root. See the
[metadata catalog](forever-metadata-catalog.md),
[game-table contract](forever-game-tables.md) and
[registered Rust dependency closure](licensing/forever-rust-casc.md).

## Evidence and completion audit

Keep raw files and full numeric comparisons only in the private run root
outside every Git checkout. Reports may contain source revisions, tool
hashes, counts, static error codes and comparison results. Each report must
identify the exact component, build, locale, reader entry point and whether
that entry point actually ran.

Before completing the goal, audit every row of the table above against
concrete retained evidence. Record any exclusions and obtain authorization
for scope changes rather than silently reducing the goal. A successful
build, process exit, schema match or subset report is insufficient.

Detailed prior decoder evidence remains in
[`forever-rust-foundation.md`](forever-rust-foundation.md). New component
evidence should link back to this checklist and retain the distinction
between metadata coverage, decoding support and real runtime acceptance.

- **Complete supplied-raw VMO assembly:** 22 further pure tests cover model
  serialization and a strict geometry-aware reader. Mesh BIHs use triangle
  bounds and leaf three; group BIHs use declared group bounds and leaf one.
  Geometry order, signed zero, IDs, flags and liquids are retained. Empty
  vertex groups omit their liquid. Existing raw input restrictions remain;
  the VMO reader separately admits empty models and vertices without triangles.
  `native-vmo-debug-20261011T035006Z-5ea3e8c6.json` and
  `native-vmo-release-20261011T035036Z-33b64a0f.json` each match **251 complete
  synthetic models / 3,738,601 bytes** against original `convertRawFile` and
  `WorldModel` read/write, including 823 groups, 205,233 vertices, 67,877
  triangles, 114 liquids and embedded depth 65. Independent source-field and
  geometric containment checks pass, along with 21 Rust-only rejections,
  two reader-only fixtures and an exclusive-stage check. The external original
  source/G3D/Boost closure, prior synthetic raw provenance, binaries and full
  network traces are guarded. No real-model assembly, map/tile artifacts or
  runtime queries are accepted. `vmo-workspace-20261011T035100Z-a11e64fc.json`
  passes **1,221 library tests** (533 CASC, 274 DB2, 154 terrain, 260 VMap),
  two example tests and all-target Clippy through the configured helpers.

- **Transformed bounds and complete supplied single-map assembly:** 16 bounds
  tests and 21 assembly tests cover group-zero transforms, first-ID-wins sorting,
  tile membership, own/parent ordering, strict readers and supplied-proof
  associations. `native-map-assembly-debug-20261011T041356Z-7bc0b061.json` and
  `native-map-assembly-release-20261011T041531Z-0166bc88.json` each match
  **274 bounds outputs / 19,264 bytes** and **136 maps / 62,006 files /
  21,800,784 bytes** against original C++. The maps retain 1,828 unique placements
  from 2,136 inputs and contain 298,392 associations, including 15,204 tiles
  with only an index. Corpus maxima are 4,096 tiles and 262,144 associations.
  Original BIH and ModelSpawn roundtrips pass; index checks are local QA,
  not original MapTree runtime. Forty-nine Rust-only negatives, two existing-stage
  guards and 401 representative original-file reader calls pass per profile.
  Full inventory/byte equality and independent Python geometry/association checks
  pass. Source, build, binary, corpus and attributed network guards pass; no
  network attempt occurs. A failed initial timeout and two interrupted harness
  runs are retained as unaccepted attempts; only the named complete runs count.
  `map-assembly-workspace-20261011T041717Z-a01d5804.json` passes **1,258 library
  tests** (533 CASC, 274 DB2, 154 terrain, 297 VMap), two example tests and
  all-target Clippy with unchanged Rust sources. No real object/map assembly,
  ancestor resolution, full exporter or runtime queries are accepted here.

- **Supplied GameObject model lists:** 23 new tests cover complete borrowed
  temporary/final codecs and a bounded all-vertex builder. All raw groups and
  unreferenced vertices contribute to bounds; declared bounds, liquids and
  triangle membership do not, while complete strict raw validation remains.
  Physical rows and duplicate IDs/names survive; equal extrema retain the last
  vertex's bits. Each row supplies its model independently, so an acquisition
  caller must enforce immutable same-name class/content binding. Missing,
  malformed or vertex-empty dependencies reject the whole Rust build. Names
  are ASCII 1..499 without NUL in the codec; the Rust file QA caller uses
  printable basenames up to 255 bytes. Exact-zero bounds serialize but the
  original dynamic initializer rejects them, so codec admission is not runtime
  acceptance. Downward caps include 64 MiB per list/raw (plus stricter existing
  raw limits), 256 MiB cumulative raw visits, 128 million projection vertex
  visits and 256 MiB logical heap covering both outputs and temporary bounds.
  `native-gameobject-models-debug-20261011T042545Z-d346aa38.json` (SHA-256
  `3bbc12be276abe5fce2edf8dc75fdcac5b0fc71345ac27951a9d37e1e07d15b2`)
  and `native-gameobject-models-release-20261011T042545Z-71878bb6.json` (SHA-256
  `0cffe03cbb65d58d65c7a07523ce2ed55440773e67582cd4bb7090857ea938b6`)
  each match **276 synthetic cases / 536 rows / 24,895 dtree bytes**, with
  410,587 projection vertex visits and 6,709,056 cumulative raw-read bytes.
  Original public `exportGameobjectModels` executes 276 times and its raw
  reader 536 times. There are 552 positive Rust reader calls plus two reader-only
  pairs/four calls, 27 Rust-only negatives and one existing-stage guard.
  Source/build/binary/corpus/archive/network guards pass. The source snapshot
  stays unchanged through validation, and debug/release case arrays agree.
  `gameobject-models-workspace-20261011T042809Z-b7c23e5e.json` (SHA-256
  `645310a4be02bc395173481f600353404ea9fe873125e991f94145650b8a6488`)
  passes **1,281 library tests** (533 CASC, 274 DB2, 154 terrain, 320 VMap),
  two example tests and all-target Clippy. No DB2 producer, CASC acquisition,
  native dynamic consumer, initialization or queries run in this comparison.
  Source-adapter acceptance is recorded below; incomplete real
  `GameObjectDisplayInfo` prevents complete discovery acceptance. See the
  [precise contract](forever-collision.md#gameobject-model-lists).

- **GameObjectDisplayInfo direct source adapter:** 19 additional DB2 tests
  validate the closed regular/minimal-empty source profile, all nine physical
  fields/fourteen scalars, whole-file EOF and inherited resource limits.
  Physical field 1 is signed32 cast to u32 with exact bits; native flattened
  lookup slot 7 includes the ID and six preceding GeoBox values. Direct order,
  duplicate/repaired IDs, FDID duplicates and zero FDIDs survive. Raw copies
  validate without replay or a `materialize` operation. Ignored GeoBox float
  bits receive no new finiteness restriction. Unit tests exercise 71,428 rows
  under the scalar cap and 100k copies; these are not native boundary tests.
  `native-gameobject-db2-debug-20261011T043847Z-66502228.json` (SHA-256
  `af25579167849b8727689316314f4efcbf474e9b753f293e003c0edadc6aa35e`)
  and `native-gameobject-db2-release-20261011T043847Z-d32159d5.json` (SHA-256
  `1a8753b4c4147711232c978662906ee31005821a6f5bd1810e33f7bacf52e11e`)
  each match **50 synthetic files / 1,164 direct rows / 10 raw copies**, with
  **1,120 nonzero QA candidates**, **90,090 input bytes** and **58,000 exact
  JSON output bytes**. Original DB2FileLoader/LoadInfo, GetRecord/GetId,
  named GetUInt32 and GetRecordCopy execute; AutoProduceData and copy replay
  do not. Candidate zero-filtering/naming belongs to the QA example, not the
  source API. Thirty-five Rust-only negatives produce no output and two
  existing-output preservation guards pass. The guarded external closure has
  25 original source/notice files, six original translation units including
  Common and 23 actual dependency inputs, with no Boost/G3D inputs. All source,
  build, binary, corpus, archive and network guards pass.
  `gameobject-db2-workspace-20261011T043550Z-1046b96b.json` (SHA-256
  `1f652c99842a2a350fe73e36d3db4d7d2f3b3b9907e027d801b78a62c37bfb52`)
  records **1,300 library tests** (533 CASC, 293 DB2, 154 terrain, 320 VMap),
  two example tests and Clippy. Native acceptance was pending when that
  workspace report was written; the two later native reports close the
  projection gate. They execute no GameObject producer, model acquisition,
  conversion, CASC or runtime. The subsequent increment below connects startup
  classification and acquisition while retaining this historical scope.
  The real table is incomplete. See the
  [source and startup contract](forever-collision.md#gameobjectdisplayinfo-direct-source-projection).

- **Bounded GameObject startup:** `extract_startup_fdid` connects complete
  DB2 acquisition to first-four-byte dispatch, M2/WMO conversion and both lists.
  It validates the whole source before model reads, keeps physical order and
  duplicates, and never replays copies. Zero FDIDs, textures, M2 NoCollision
  and WMO DoodadOnly are explicit normal omissions; incomplete, malformed,
  unknown and over-budget dependencies are terminal. The same verified buffer
  supplies classification and conversion, including stored original magic on
  typed-cache hits. Startup uses empty liquid tables; consuming
  `Context::with_lookup` preserves cache, handles, source/raw/request counters
  and previous geometry. `finish` rechecks inputs and rejects a poisoned cache.
  Limits include 16 MiB DB2, 71,428 direct rows, 100k copies and 4,096 nonzero
  candidate requests, including duplicates. Actual source and retained vector
  capacities are charged. Decoder reservation of 2 MiB + 32 × input length is
  a conservative admission allowance, **not an exact portable heap/RSS bound**.
  `native-gameobject-startup-debug-20261011T044940Z-a0a16610.json` (SHA-256
  `9554e99e1b83168e1b99c3e777bade409f138c85e416e3b58b3921042edbd5a4`)
  and `native-gameobject-startup-release-20261011T044956Z-5517b73f.json` (SHA-256
  `8317026e8cb715aa3c526909840002891044ce04e05a1a9082c664ef60983717`)
  each match **36 synthetic cases / 469 direct rows / 127 listed rows**,
  **46 raw files / 6,855 bytes** and **9,720 bytes across both lists**.
  Original `ExtractGameobjectModels`, DB2 getters, cache, M2/WMO converters
  and the dtree exporter execute against an authored memory inventory. Rust
  reads complete synthetic CASC using explicit FDIDs, enUS, contiguous indexes
  and Header50893V2; native CASC/name-first routing equivalence is not established.
  Seven Rust-only negatives and two exclusive-output guards pass per profile,
  with 72 successful and nine expected exit-4 processes. Source/build/binary/
  input guards pass and syscall denial records no network attempt; debug/release
  case arrays agree. One native-only case exercises original liquid DB2 readers
  and later WMO cache reuse. Rust later-phase behavior has unit coverage, but
  that step did not compare the joint native/Rust later phase; the shared-world
  acceptance below supplies that separate evidence.
  Forty added CASC tests and one DB2 test bring
  `gameobject-startup-workspace-20261011T045032Z-3cd5b4e5.json` (SHA-256
  `d66355be3754c6cb47bf9e56d736179c9053df1b6b4572547ab73eee0d8f9b3a`)
  to **1,341 library tests** (573 CASC, 294 DB2, 154 terrain, 320 VMap),
  two example tests, 165 focused collision tests and all-target Clippy.
  The private QA example writes only after successful finish; it is not a
  product CLI or atomic publication protocol. The older `run::extract` object
  runner retains a fresh collector per batch. The subsequent addition below
  integrates the lifecycle; incomplete real DB2s, real inventory/publication and
  runtime acceptance remain gates. Full extraction is not complete. See the
  [startup evidence](forever-collision.md#gameobject-startup-acquisition-and-cache).

- **Shared startup/world implementation:** `collision::world::extract` consumes
  startup and loads complete authenticated Map, LiquidMaterial and LiquidType
  sources through its collector, preserving cache/handles and cumulative limits.
  It replays liquid direct duplicates and ordered copies with missing-source
  defaults, then runs WDT/Obj0ADT acquisition and placements with one ID session.
  Source buffers, owned Map strings/rows, lookups, graph, runner storage and
  overlapping growth remain charged; DB2 decode uses a conservative allowance,
  not a portable allocator/RSS bound. Lookups/tables are dropped before their
  reservations are released and final finish transfers output. Any failure is
  terminal; a second shared run is rejected. The private synthetic QA entry point
  stages bounded per-map `dir_bin` output, including empty global files, and
  writes only after full success. Debug/release each match **49 complete synthetic
  cases / 43 listed rows / 82 RAW files / 13,537 bytes**, both startup lists
  (**3,880 bytes**) and **79 per-map dir_bin files / 11,688 bytes / 176 placements**.
  Reports are `native-collision-world-debug-20261011T050621Z-ad431060.json`
  (SHA-256 `01ee320239a5aa8a2f967377e8f1f14449f847a29038a3c610a0e336e89f18cf`)
  and `native-collision-world-release-20261011T050635Z-e5bf638a.json`
  (SHA-256 `9600d30bf11f0ec82e71a7baf6321aa192e49d638c556dc4ed52150dea453fa9`).
  Original startup, Map/liquid loaders, `ParsMapFiles`, parsers, converters,
  projectors/cache and dtree export execute over an authored memory inventory;
  Rust uses complete toy CASC. Fifteen Rust negatives and two exclusivity guards
  pass per profile: 98 exit-0, 17 expected exit-4, and 115 empty network traces.
  Inputs, sources, builds, used Boost closure and binaries remain unchanged;
  both profiles' case arrays/source hashes agree. The 13,186,148 fixture bytes
  are wholly synthetic. Workspace report
  `collision-world-workspace-20261011T050602Z-dc864d9b.json`
  (SHA-256 `9b157ba7c18ad735996012f650c0100f1134fe589310248347c32ea101808a2f`)
  passes **1,378 library tests** (610 CASC, 294 DB2, 154 terrain, 320 VMap),
  two example tests, 202 focused collision tests and all-target Clippy. The 37
  new CASC tests comprise 20 shared-run, two request-window and 15 world tests.
  This closes synthetic joint later-phase output comparison. It does not
  establish native CASC routing, parallel/unordered scheduling, complete real
  input availability, product publication or runtime acceptance. Full extraction
  remains incomplete. See the [shared-world contract](forever-collision.md#shared-startup-and-world-execution).


- **Shared-world final-package integration:** `collision::assemble::extract`
  consumes startup, seals a private bounded world-placement stage and builds the
  selected VMO union, all active maps and startup dtree. All placement names
  select models before first-ID-wins map deduplication; unused acquired
  dependencies do not become VMOs. Native nested map paths are preserved.
  Parent-only child indexes bind to the first present actual-ancestor payload;
  missing or mismatched payloads fail, without trying a later matching ancestor.
  The owned topology includes inactive/copied rows and the direct prefix.
  Limits only reduce 1 GiB logical memory, 256 MiB reserved staging, 512 MiB final
  bytes, 65,536 files and cumulative geometry/RAW/BIH/ancestor/proof work. Final
  world and store residency, scratch, growth overlap and retained proofs are
  accounted separately. This is capacity accounting, not portable RSS or a
  measurement of every CPU operation. Both final profiles match **53 complete
  synthetic cases / 650 files / 56,019 bytes**, **87 selected VMOs**, **180
  unique map spawns**, **303 associations**, **27 parent-only tiles** and **one
  mixed tile**, including complete inventories and final byte equality.
  Reports are `native-collision-assembled-debug-20261011T052153Z-c05f0c6c.json`
  (SHA-256 `a9a1430bf8681b2d90d1e7b65eb65fbecea5bd9b152058f463c17a6f765402be`)
  and `native-collision-assembled-release-20261011T052200Z-2aa3d576.json`
  (SHA-256 `984c08487eebd6fe8631797334ca3d55ccf6e5c1ab4284239416ab82b71f8e99`).
  Original startup/world processing and `convertWorld2` execute against a
  memory inventory with Threads=1; Rust uses complete toy CASC. Native CASC
  routing, parallel/unordered order and runtime consumers are not exercised.
  **40 Rust-only negatives**, including **25 strict parent-association failures**,
  and **two exclusivity guards** pass per profile. All guards pass with **148
  empty network traces**, 106 exit-0 and 42 expected exit-4 processes; both
  profiles' output/semantic case fields and source hashes agree. The only case
  difference is 38 additional LocalStore resident bytes in the release run.
  The workspace report
  `collision-assembled-workspace-20261011T052113Z-b8a01209.json`
  (SHA-256 `058e7fa8f9fc9deb6d6ffa1f016d1a42b2c0f94b86beabbfbc999bd9cb9e0d68`)
  passes **1,413 library tests** (645 CASC, 294 DB2, 154 terrain, 320 VMap),
  two example tests, 237 focused collision tests and all-target Clippy. The 35
  new CASC tests cover assembly (25) and residency/topology (10), including final
  RAW-pass and inherited-index scratch accounting. Historical 1,378-test
  shared-world and supplied-map evidence remains unchanged.
  The QA entry point writes final `vmaps/` artifacts only after full success and
  preflight. Product manifests/publication, complete real sources, RustyCore
  loading/query acceptance and navigation remain gates. Current RustyCore has a
  LOS-provider interface but its server installs no VMap provider; MAPS height
  support is not VMap height support. No VMAP binary runtime integration is
  claimed. See the [assembled-package contract](forever-collision.md#shared-world-collision-assembly).


- **Supplied-model and instance ray queries:** the pure `query` API traverses
  validated group/mesh BIHs with the pinned triangle test and checked instance
  transforms. It preserves nonnormalized rays, traversal-order Any hits,
  nearest-hit comparisons, single-group bypass, cofactor inverse and source
  AABox behavior. Public maxima permit zero and positive infinity; finite
  negative scales with finite reciprocals remain admitted. Invalid transforms,
  budget exhaustion and executed phantom infinity-marked edges return errors,
  distinct from misses. One cumulative budget only reduces 16 million nodes
  and 16 million triangle tests, using 65 stack entries per nested tree and no
  query heap. These are Rust work counters, not exact C++ counters or RSS.
  Workspace report `collision-query-workspace-20261011T053327Z-a226406e.json`
  passes **1,467 library tests** (645 CASC, 294 DB2, 154 terrain, 374 VMap),
  two example tests, **54 focused query tests** and all-target Clippy with
  warnings denied; its SHA-256 is
  `729e269805d65230725515149958a3fd2a41d5ef3ef27f4e6a06e4a627c16746`.
  Final comparisons each pass **79 synthetic cases / 14,416 queries**, with
  **2,950 hits / 11,466 misses**, **11,850 direct-model / 2,566 instance queries**
  and **10,038 nearest / 4,378 Any queries**. Hit flags and distance bits agree
  without exclusions; the corpus includes 26 known-result checks and a safe-leaf
  positive-infinite maximum. Reports are
  `native-collision-query-debug-20261011T053650Z-d98ea106.json`
  (SHA-256 `cc010a04040dc8ed38440d50e6f71041fd6f3117e6648e3e47b600ea82bafd82`)
  and `native-collision-query-release-20261011T053701Z-e165ffbe.json`
  (SHA-256 `4f016f32d5063d36197258e68a3b39f908ddd6ce957a39667ac43531fd467545`).
  Both profiles use the same cases and have identical source hashes, case arrays
  and totals. Original WorldModel reading runs 79 times and spawn reading
  17 times per profile, followed by original model/instance geometry queries;
  no substitute query kernel is used. The closure contains 24 original TUs
  (16 G3D), 124 inventory entries and 117 actual original dependency inputs
  plus the wrapper, without Boost. Each profile passes **14 Rust-only negatives**
  and **two exclusivity guards**, with **174 empty network traces**, 158 exit-0
  and 16 expected exit-4 processes. Source/build/binary/input/provenance guards
  pass. Native work-counter equality is not claimed.
  The private QA caller validates its complete VMO/optional
  placement/ray plan and completes all queries before creating output; miss
  distance bits retain the supplied maximum. Model-name binding, complete
  StaticMapTree/VMapManager loading, runtime coordinates/LOS/height contracts,
  area/liquid queries, dynamic GameObject state and RustyCore provider
  installation remain gates. Earlier assembly/storage reports retain their
  historical scope. See the [query contract](forever-collision.md#model-and-instance-ray-queries).

- **Static collision package consumer:** Debug/release each match 75 complete
  synthetic cases/3,298 lifecycle/query operations against original
  StaticMapTree/VMapManager, including 339 loads, 305 unloads and 2,654 queries.
  Exact state and payload bits, 38,274 independent field checks, 58 Rust-only
  negatives, two exclusivity guards and 210 empty traces pass/profile. The
  owned-reader/query regression retains all 79 cases/14,416 exact queries. The
  workspace now passes 1,558 library tests, two examples, 66 focused store tests
  and all-target Clippy. This accepts supplied synthetic package loading and
  queries, with explicit stricter Rust failure admission, not actual Forever
  gameplay. See the [static package evidence](forever-collision.md#executed-static-manager-acceptance).

## Supplied-key acquisition increment — 2026-10-11

Explicit 128-bit Salsa20 keys now have bounded original Rust library APIs,
without discovery, built-in keys or new dependencies. Forty focused synthetic
tests cover the parser/primitive, BLTE, archive adapter and LocalStore,
including encrypted ENCODING/ROOT, wrong/missing keys, mandatory complete CKey,
strict ciphertext hashes, frame ordinals, incomplete spans and memory limits.
The v1 product plan and existing acquisition profiles remain N/Z. The
subsequent production key option has a separate v2 plan/contract, described
in the [product interface](forever-product.md#explicit-supplied-keys-product-v2).
No real key was inspected or supplied, and this increment does not recover
Map, SkillLineAbility or GameObjectDisplayInfo. Real all-component output and
reader/runtime acceptance remain open.

A bounded read-only source survey found the current 70205 client and a second
installation of the legacy 51943 client. The reachable reviewed remote paths
provided no additional Forever client. One configured reference-server SSH
route was unavailable. This is a scoped survey, not an exhaustive machine or
network inventory; legacy inputs are not substituted for Forever.

The associated workspace regression passes 2,070 Rust tests and 17 product CLI
scenarios in each debug/release profile, all 72 contract tests, formatting and
Clippy. Six actual SIGKILL recovery cases remain accepted; 70 ordinary network
traces are empty and six interruption traces contain no network calls. The
previous accepted real 33-artifact product batch remains the real acquisition
evidence; it is not counted as a new client run for this increment.

The supplied-key primitive also matches the original pinned CascDecrypt:
272 native synthetic cases/544 calls feed 488 complete public Rust BLTE decodes
per debug/release profile (89,120 bytes), with Python plaintext/CKey comparison
and default no-key rejection for every object. Three runtime traces are empty
under network denial; native compilation has another empty trace, while Rust
compiler traces separately record local AF_UNIX linker IPC. This establishes
bounded supplied-key wire/decryption evidence, without native CASC routing,
real encrypted acquisition or new RustyCore/runtime acceptance.

## Versioned supplied-key product increment — 2026-10-11

The explicit v2 production plan/CLI now integrates the bounded supplied-key
API without broadening v1. Acceptance passes 2,088 Rust tests, 17 existing v1
and 20 new v2 CLI scenarios per debug/release profile, all 76 contracts and
Clippy. Sixteen actual SIGKILL interruptions yield 14 recoveries and two
expected protected-key-source refusals; encrypted synthetic
six-component packages retain the same 31 artifact bytes as plaintext v1.
Both bounded real v1/v2 runs reproduce 33 previously accepted artifacts,
using only an authored nonsecret dummy key for v2. This is not actual real
encrypted acquisition. Map still rejects without a payload or final manifest.
No dependencies or new native/runtime acceptance are added. Complete real
inputs, all-component output/reader acceptance, full-world operation and
Linux/Windows distribution remain open. See the [executed product evidence](forever-product.md#executed-supplied-key-product-v2-evidence-2026-10-11).


## Reproducible local Linux distribution — 2026-10-11

Two fresh offline frozen-source builds (one path with spaces) produce identical
archive/binary bytes, and a moved-source rebuild with empty Cargo caches
matches the binary. The package includes exact registry archives/vendor
sources/notices, the build recipe, mandatory project third-party provenance
and 13 pinned Rust standard-library notice documents. The audited packager
passes 41 focused tests and 82 contracts; independent review rechecks 15
regressions. The shipped binary passes 37 CLI scenarios, retains all 110
artifact hashes from four preceding synthetic packages, and repeats the real
33-artifact DB2/GT/camera result. Production Rust/dependencies are unchanged.
This closes the local Linux reconstruction increment, without publication,
full-world scalability, complete real geometry/runtime acceptance, synthetic
CI or functional Windows distribution. See [distribution scope and evidence](forever-distribution.md).

## Windows read foundation and synthetic CI — 2026-10-11

The new dependency-free `forever-platform` crate defines the separate
`forever-windows-read-ntfs-v1` profile: bounded read-only file leases with held
ancestors, NTFS handle identities, explicit hardlink admission and permanent
invalidation after observed change. Ancestor opens request directory-list and
attribute access; attribute-only opens did not enforce the required sharing in
the first Wine diagnostic and were corrected. The original scoped Win32 FFI
imports no implementation or registry dependency.

The source now contains a [synthetic CI workflow](forever-ci.md): Linux workspace
tests in both profiles and Windows tests for DB2, terrain, VMap and platform.
CASC output adapters, navigation discovery and the complete product Windows
backend remain open. No hosted workflow has been run or published.

The corresponding-source distribution profile is now
`forever-seven-crate-ci-source-v2`. Its envelope remains distribution v1; the
new verifier requires this source profile. Earlier six-crate artifacts retain
their frozen historical verifier. Windows toolchain preparation uses exact
official Rust 1.98.0 archive checksums from the locally installed channel
manifest, staged outside the checkout. Windows/Wine execution and Linux package
regressions are recorded separately in external acceptance reports; these
source changes alone do not establish their acceptance.

At that foundation snapshot, no producer consumed the Windows lease. Real Windows/NTFS,
actual reparse/junctions, private DACL creation, publication/durability and loaded
executable identity remain required before enabling a Windows product backend.
The full-real-input and RustyCore runtime gates above remain unchanged.

### Executed Windows-foundation and CI checks

The external `windows-foundation-20261011T161038Z.json` records 2,093 passing
Linux tests per debug/release profile, with 22 explicit ignored external-corpus
targets per profile, and workspace Clippy with warnings denied. The independent
Wine replay/audit records 24 frozen Windows GNU test executables per profile:
924 passing tests and two ignored tests per profile. Seventeen passing tests
exercise the platform crate; the actual privileged reparse test is one of the
ignored tests, alongside an existing DB2 corpus test. The portable metadata
Windows executable separately passes five synthetic CLI cases through Wine.
These executions do not run the complete product on Windows.

Contract ID `forever-windows-read-v1` names profile
`windows-x64-fixed-ntfs-read-lease-v1`; the corresponding public API constant is
`forever-windows-read-ntfs-v1`. These names identify the same bounded foundation.
The original attribute-only ancestor open failed a real Wine rename test; the
corrected directory-list access passes both rename and direct DELETE-access
regressions. The first failure and its diagnostic binary are retained.

Wine runs inherit a seccomp filter allowing AF_UNIX socket families only. The
audited replay decodes 768 AF_UNIX sockets/pairs and 336 denied AF_NETLINK
attempts, with no AF_INET/IPv6 socket creation observed. Sixteen opaque terminal
strace lines in fourteen files, including seven detached markers, remain outside
that decoded-call proof. Individual exit of every Wine descendant is not proven.
The explicit initial foreground-server probe has verified identity, live seccomp
state, natural exit zero and subsequent PID absence. Historical failed helper
and overlong-keepalive observations are retained; none is Windows acceptance.

The Linux source package also retains its exact prior binary, 37 packaged CLI
scenarios and 33 real accepted artifacts under the seven-crate source profile.
The detailed distribution and package QA reports retain reproducibility,
negative verification and schema evidence. No new C++/RustyCore runtime result,
real encrypted read, full real world extraction, hosted CI run or publication
was performed. All remaining completion requirements above remain open.

## Windows read consumers and distribution v3 — 2026-10-11

CASC admission/LocalStore and explicit product input `bytes`/`keys` now use
Windows read leases, including retained key-source leases and bounded chunked
reads. Generic component manifests, metadata snapshots and verifiers remain
outside this protected-read scope. DB2/GT/camera output adapters refuse Windows
before output I/O, and the complete Windows product remains disabled.

The initial integrated source snapshot passes 2,093 Linux tests per debug/release
profile, workspace Clippy, 124 contract checks and 49 packaging checks. Its
Windows GNU workspace compiles, but the concurrent Wine batch failed with
`InputChanged`. Diagnosis traced this to full timestamp consistency checks
during ancestor identity acquisition, before the intended object-only comparison.
Unrelated sibling creation must be admitted by that acquisition policy while
final-file and full final-directory snapshots remain strict. The failed runs
and diagnostics are retained; they do not establish Windows acceptance.

The correction separates object-only acquisition from full snapshot acquisition
inside the original three-query Win32 helper. Ancestor capture/rechecks and
explicit directory object verification compare volume, file ID and attributes;
zero links still reject. Final file/directory capture and full verification keep
their complete comparisons. Deterministic query-comparator tests and concurrent
sibling churn cover the distinction. Execution results belong to the subsequent
frozen-source report; this implementation description is not an acceptance claim.

The corrected-source diagnostic batch then passed all 28 runnable platform
tests and 638 CASC tests under Wine. One inherited test failed while copying
`0000000001.idx` to `0000000001.IDX`, which refers to the same file on the
admitted ordinary Windows fixture. The adjusted test keeps Unix's two-entry
rejection and verifies Windows's same-identity/single-entry acquisition instead.
The platform-independent index selector still rejects duplicate generations.
Missing-index and corrupt-newest-index checks remain shared. The failed batch
is retained as `windows-ancestor-fix-20261011T172447Z.json`. The subsequent
`windows-ancestor-replay-20261011T173726Z.json` completed all runtime tests in
both profiles, then failed Windows Clippy. Two cfg-only source fixes in the
extractor binary test imports and navigation inventory test imports precede
the corrected consumer closeout below; the failed replay remains retained.

### Executed corrected Windows read-consumer closeout

The external `windows-consumer-closeout-20261011T180333Z.json` (SHA256
`b9870491ad7523331170744c9b16fc7b8c55dab7bd20e2affedc8316f94bfc79`)
is accepted and terminal for this synthetic consumer scope. Its 910-file source
snapshot remains unchanged during execution. Debug/release each pass 2,093
Linux tests, with 22 ignored tests across 40 targets, and 1,885 Windows GNU tests
under Wine, with 18 ignored tests across 40 executables. Workspace all-target
Clippy with warnings denied passes for Linux and the Windows target; 125
contract checks and 49 packaging checks also pass.

Per Windows profile, 37 executables are exactly SHA256-identical to the prior
replay and reuse its runtime evidence only after validating binary, log and
trace hashes. The three changed executables run afresh after the two cfg-only
fixes. This is explicit artifact reuse, not a claim that all 40 executables were
rerun in the closeout. Windows cmd sets TMP/TEMP separately for every executable;
a std::env::temp_dir probe verifies each effective path. Foreground Wine-server
launchers exit naturally with zero status and are absent afterwards; individual
exit of every Wine descendant remains unproven.

The AF_UNIX-only seccomp policy is enforced and self-tested. Filtered network
traces record observed calls; higher-priority seccomp ERRNO decisions may not
appear, so these traces do not establish absence of attempted nonlocal calls.
Wine is preliminary. Actual Windows/NTFS, real reparse/junction behavior,
private DACL, publication/durability and loaded-executable identity acceptance
remain open. Generic component manifests, metadata snapshots and verifiers
remain outside protected read-consumer scope; Windows output/product gates stay
closed. This closeout adds no real-client or RustyCore runtime result, replacement
Linux bundle, hosted CI execution or full-extractor acceptance.

### Earlier separate Linux distribution v3 acceptance

The separate Linux distribution v3 acceptance records two identical isolated
builds, a matching moved-source rebuild, 37 packaged synthetic CLI cases and
eight actual interruptions: seven recoveries plus one expected protected-source
refusal. Its product binary hash is
`b805026133be7a15399554592d815d7f7b3777f456a506e1f0681c777de9f681`;
it differs from the historical foundation package. The mandatory source profile
is `forever-seven-crate-windows-input-source-v3`, still inside envelope v1.
This run used no real client inputs or real keys and does not repeat the prior
33-file real acceptance with the new binary. This distribution run predates
the corrected consumer closeout; that closeout does not rebuild or accept a new
distribution bundle. All complete-input, full-world, actual Windows and RustyCore
runtime gates remain open.

### Corrected consumer v3 distribution and bounded real supplement

The later `distribution-v3-consumer-closeout-20261011T180333Z.json` (SHA256
`7e3e61553a18d5050738b41e717a6b342836e1af3a192c8f67bea9f0caa66f75`)
accepts two matching archives/manifests and a matching moved-source rebuild.
Archive SHA256 is `4ab8fffa9b118e0d13fd6663950c83b272d2a0b69462b01cb87d6216a538855d`;
the corrected binary is
`ca528b696128622a0c9b624b351e0a18b34ece9abd43cb154821cf20201f27d5`.
Its 37 synthetic CLI cases and eight actual interruptions retain seven
recoveries and one expected protected-key-source refusal. This source/binary
acceptance supersedes neither historical failed reports nor their scope.

The subsequent `distribution-v3-bounded-real-20261011T180333Z.json` (SHA256
`89a1a05a7acb41043a77875389d7194f1e9a10e293d0e4e1ce67e1854748685b`)
accepts 33 bounded DB2/GT/camera artifacts / 193,794 bytes with that corrected
`ca528b69...` binary. It supplies the new-binary real supplement absent from the
preceding synthetic-only bundle report. It is not full extraction, a new
RustyCore runtime result or Windows acceptance. Neither report builds source v4.

## Windows producer source v4 — 2026-10-11

The v4 mandatory source profile was
`forever-seven-crate-windows-producer-source-v4` in the unchanged v1 envelope,
with the same seven crates, exact nine-package graph and two registry sources.
The separate original `forever-windows-producer-seal-v1` implementation adds a
safe LoadedProducer acquisition API, scoped platform FFI, a pure extractor PE
verifier, loaded-identity helper/example, offline sealer, tests and contract/doc.
The prior v3 bundle and its bounded-real supplement retain their frozen profile.
No v4 bundle or complete Windows distribution is accepted by this source change.

Loaded acquisition reads the actual main-module static payload with a volatile
operation, measures its payload start RVA and retains a protected ReadFile
lease. Complete bounded bytes must match the strict unsigned x64 PE32+ layout,
loaded RVA and loaded digest. Canonical hashing zeroes exactly the digest slot;
success returns SHA256 of the complete sealed file. The final retained-file and
loaded-evidence recheck also runs after failed PE verification. Declared static
loader write/control checks do not validate arbitrary startup behavior.
The sealer creates a new exclusive file and retains failed outputs without
pathname unlink cleanup; supplied verification values are artifact integrity,
not loaded-process evidence or publisher authentication.

The integrated candidate passes 139 contract tests (14 new static producer
checks) and 94 distribution tests (53 packaging guards, 41 sealer cases), with
all 921 source files unchanged during each focused run. Temporary files and
caches remain external. These checks do not execute the loaded Windows helper
or build a v4 bundle. Actual Windows/NTFS, private DACL, alias-safe output/key
containment, publication/durability/recovery, complete real extraction and
RustyCore runtime remain gates; the Windows product stays disabled.

The corrected production-API closeout
`windows-producer-consumer-20261011T180421Z/final-control-check/closeout.json`
(SHA256 `e086d4459640c898fa0fc7e63c9c6d788a7c230c0aa6bd3688bd4532e9dcdbd1`)
is accepted and terminal for preliminary Wine execution; runtime report SHA256
is `ef7e8b62a50d34629d5d687437c838120c7b86b578ef25c8d9eb46c63f9878df`.
Unchanged corrected source passes 37 Rust tests/profile, 41 Python tests and
Windows workspace all-target Clippy. Six actual debug/release helper cases
accept sealed binaries and reject unsealed/tampered binaries with the expected
static errors. Actual measured payload RVAs 5,009,408/2,412,544 match the strict
file layouts. Earlier pre-correction reports remain historical. This does not
accept actual Windows/NTFS, enable a Windows product or build a v4 bundle.
Filtered traces establish observed calls only; denied attempts may be absent,
and individual exit of every Wine descendant remains unproven.

## Canonical producer closeout and Linux v4 bundle — 2026-10-11

The later accepted/terminal `windows-producer-closeout-20261011T182651Z.json`
(SHA256 `ae9e3761d6d1ab6848dec6c54f4739c53181eb47ba3f0334d528e900fcb2ddf6`)
keeps all 921 canonical sources unchanged. Debug/release each pass 2,130 Linux
tests (22 ignored, 41 targets) and 1,925 Windows GNU tests under Wine (20 ignored,
41 executables). Each Wine profile reuses 24 exactly SHA256-identical executables
with validated prior evidence and runs 17 fresh executables. Linux/Windows
workspace all-target Clippy, 139 contracts and 94 distribution checks pass.
Independent audit SHA256 is
`82891385f5903f74f3c6c462ffd45e24e2d8ac8f19770782861441d4222d2d78`.

`distribution-v4-producer-closeout-20261011T182651Z.json` (SHA256
`9f48f5537f3c8521f4b597cdc2fe208e582094e7028a1182859b91f5e2d62ad5`)
separately accepts two identical archives/manifests and the moved-source rebuild:

- Archive SHA256: `ab2649ea72fd6d6605b4377dd484e87128a89becec06a69c3fe9bbaa94aac919`.
- Binary SHA256: `42f1ed4fcae892af7cb8f22cb3ada0ad7165395ae500feefa483a45a59c6060b`.

Its 37 packaged synthetic CLI cases and eight interruptions preserve seven
recoveries plus one protected-source refusal. The subsequent accepted/terminal
`distribution-v4-bounded-real-20261011T182651Z.json` (SHA256
`a0e7a62e5cdfb54f0efb869f2f36898b653900cfb4437b2f1a6d34f807796613`)
accepts 33 bounded DB2/GT/camera artifact records / 193,794 bytes with that v4
binary, identical to the prior bounded artifacts. Build-info and 921 sources
remain unchanged. Earlier v3 bundles and failed/pre-correction reports retain
their original binaries, source snapshots and scope.

These results do not accept actual Windows/NTFS, enable the Windows product,
add real encrypted-key/native/runtime evidence or complete extraction. Filtered
seccomp traces record observed calls; denied attempts may be absent. Natural
launcher exit does not prove individual exit of every descendant.

## Windows VMap and input diagnostics source v5 — 2026-10-11

At the v5 source checkpoint the profile was
`forever-seven-crate-windows-vmap-diagnostics-source-v5`. The archive envelope
remains v1 with seven workspace crates, nine lock packages and the same two
registry archives/notices. The sole new dependency edge is Windows-only
`forever-vmap -> forever-platform`. Prior v1/v2/v3/v4 archives retain their
bundled frozen verifiers; the v5 verifier rejects their older source profiles.

The mandatory closure adds [Windows VMap read source](forever-windows-vmap.md),
[whole-span pure BLTE framing](forever-blte-framing.md), fixed required-input
route metadata, and their source/tests/contracts/docs. The route document is
relocated byte-for-byte from the worker's new crate-doc path to
`docs/forever-required-routes.md`, retaining author/date. Generic crate Markdown
and unrelated tools remain outside the allowlist. Omitted mandatory source or
inventory rejects; the exact VMap platform dependency edge is required.

Framing validates all framed encoded MD5s and outer N/Z/E structure, with
explicitly unhashed headerless evidence; it does not decrypt, inflate or prove
plaintext/CKey/EKey validity. Routes report metadata observations only, with
payload and ciphertext NotInspected. There is no route-to-byte adapter or key-only
sufficiency claim. The first routes Linux handoff is superseded after an archive
filename-format defect. Corrected decimal naming shares the acquisition path
constructor and passes 88 Linux tests/profile, Linux/Windows Clippy and Windows
cross-build. Accepted Linux handoff SHA256 is
`879b0be0fe82874f5d57a91722103114177f5e006ecfb73196d15fa7c55a80c4`;
Wine route runtime remains pending; the subsequent metadata-only real QA is
recorded below.
VMap's 28 Wine cases/profile remain preliminary, not actual NTFS/product proof.
No v5 bundle, complete real-input payload diagnosis, full extraction or
publication is accepted by this isolated source composition.


### Corrected required-route real metadata observation

The accepted/terminal `required-route-live-20261011T1901Z-attempt2.json` (SHA256
`83b4ad2ba3a939207f194dc3b1f287b6307c33cffcb304dc500b9abfd7ad6ec1`)
executes the corrected fixed-three-FDID diagnostic on Linux build-70205/esES.
Each input has one `Routed` candidate and `FitsObservedLength`:

| Input | Encoded record bytes | Declared decoded bytes |
| --- | ---: | ---: |
| Map | 5,042 | 10,181 |
| SkillLineAbility | 94,617 | 347,118 |
| GameObjectDisplayInfo | 74,592 | 116,054 |

Payload and ciphertext remain `NotInspected`. This observes admitted routing and
held-file extents; it does not read requested archive spans, inspect BLTE, consult
keys or establish complete ciphertext/plaintext. It cannot establish that a key
is the only missing requirement. Build-info, corrected source and canonical
sources remain unchanged. Attempt 1 failed in the external harness's toolchain
pin/lock setup before client execution and remains preserved. The relocated
route design document retains its original pre-QA author/date/content; this
later record supplies its first real metadata observation. This is not an
implemented route-to-framing adapter, repair plan or v5 package acceptance.

The isolated 935-source composition passes 156 static contract checks and 97
synthetic distribution checks (56 packaging guards plus 41 sealer cases), with
all source hashes unchanged. Its selected package-source closure contains 518
files. These checks perform no Rust integration runtime acceptance or v5 build.


## Required-input complete-span framing source v6 — 2026-10-11

The current mandatory profile is
`forever-seven-crate-windows-vmap-required-framing-source-v6`, retaining the v1
envelope and the v5 graph of seven crates, nine packages and two registry
archives. It adds the separate fixed-three-input key-free complete-span framing
API and its exact source, tests, contract and documentation. The product CLI
and Windows product remain disabled. Historical v4/v5 contracts and imported
design documents retain their initial scopes. The
[framing acceptance supplement](forever-required-framing-acceptance.md) records
the later real Linux observation and exact report hashes. No v6 bundle has been
built by this source increment.

The current key-free acquisition diagnostic for Map, SkillLineAbility and
GameObjectDisplayInfo rejects all three with `local_store_no_verified_encoding`,
each encrypted 1 and missing/incomplete/corrupt/unsupported 0 (report SHA256
`d1979922ac941a5c7363f7e014f023f47cffd8372523df49d4d35bd6c5c42536`).
A separate accepted/terminal framing run (SHA256
`43f2ccf6a92a4d9e33239be3cc3e0e3de87bb43d6b69d7e9baea93c515ed3f01`)
returns one `Complete(FramedMd5)` candidate each: 15 frames, 6 Z, 9 E and 15
verified encoded-frame MD5s. The bounded adapter reads complete records, ignores
30 prelude bytes, checks only the nine-byte index prefix, and retains final
guards on all results. The 4096-frame cap is per candidate BLTE object, not
aggregated per FDID. No full EKey/CKey, Z interior, E inner mode, ciphertext
interior or plaintext validity is established. Key sufficiency remains unknown.

Earlier `local_file_incomplete` statements for these three inputs describe
their historical attempts; they are not the current acquisition category. Old
failed manifests remain failed. No newly decoded input or typed-loader success
is established. Complete Birth/initialization/GameObject real acceptance, full
geometry/runtime, actual Windows/NTFS and product completion remain pending.
The 935-source v5 canonical merge is accepted as source integration only
(closeout SHA256
`dc55520190ea017eedda82436469cd8199f2079d07a95fb28a4a4d04c63be77f`).
This adds no v5/v6 bundle, publication or hosted CI result.

## Private collision inventory source acceptance — 2026-10-12

The optional Linux [typed WDT/cache inventory](forever-collision-inventory-spool.md)
now has supplied-input synthetic acceptance: 25 focused and 412 complete
collision tests per debug/release profile, CASC library/test Clippy, scoped
formatting, 171 source/state and 104 distribution/sealer checks. The component
uses immutable typed private envelopes and preserves empty own-tile presence,
parent-bit replay and terminal failure policies. The companion records exact
source/report hashes and the two preserved failed fixture attempts.

This is a library primitive. The authenticated world runner still retains its
WDT and parent-cache inventories in memory. Integrating separate disk owners
requires a combined memory ledger and whole-selection source/spool checks.
The cumulative 4,096 source-read ceiling, same-mount spool boundary, selected
VMO/map/proof disk staging, product publication/resume, real mandatory DB2/model
inputs, runtime activation and reproducible distribution remain gates.

The preceding source commit `9e97d48` independently passes hosted
[Rust run 38189148916](https://github.com/alseif0x/rustycore-extractors/actions/runs/38189148916):
Linux debug/release and Windows debug, with 2,116 Windows tests across 41
targets and 20 ignored. Hosted workflow downloads are distinct from local
IPC-only trace evidence. That run precedes this Linux inventory increment and
does not establish real-client, general NTFS, Windows release/product or full
extractor acceptance. No new bundle is built here.

## Authenticated WDT/cache disk world — 2026-10-12

The new optional Linux [shared disk runner](forever-collision-disk-run.md)
acquires WDT/Obj0ADT through the same collector, stores complete WDT and own-tile
cache bodies on disk and skips absent parent ranges by sorted existing keys.
Its [consuming world owner](forever-collision-disk-world.md) also authenticates
closed Map/liquid inputs and retains the validated graph/topology and startup
Data. Exact owned Sink precharge transfers through preparation/run; it drops
before independent final source/model/WDT/cache checks. The explicit 2 GiB
combined logical limit does not raise any component ceiling or claim RSS bounds.

The preceding runner passes 26 focused and 438 complete collision tests per
debug/release profile. Final source rechecks all 26 public runner cases plus
25 complete-world cases in both profiles, CASC library/test Clippy, scoped
formatting, 171 source/state contracts and 104 distribution/sealer checks.
Exact hashes, source scopes and retained interrupted/failed fixture histories
are in the companion docs. Comparisons use authored toy CASC against unchanged
resident Rust; no new original C++ or real-client acceptance is established.

The earlier v1 resident inventories remain supported. The product driver's
final VMap assembly is still resident. Selected VMO/map/proof disk staging,
unified product publication/resume, realistic full-world read/disk/reload and
cross-volume policies, complete encrypted mandatory DB2/model inputs, runtime
activation and reproducible Linux/Windows distribution remain open.

Preceding canonical `26f037d` independently passes hosted
[Rust run 38191467007](https://github.com/alseif0x/rustycore-extractors/actions/runs/38191467007),
including Linux debug/release and 2,116 Windows debug tests across 41 targets
(20 ignored). That result predates this source. No new bundle, real geometry,
runtime activation or full extractor completion is claimed.

## Authenticated placement disk phase — 2026-10-12

The optional Linux [closed placement stage](forever-collision-placement-stage.md)
now stores complete authenticated world callbacks in immutable typed private
files and reloads one map at a time. It preserves all pre-ID-deduplication names,
duplicate IDs, empty callbacks and exact context/sequence/count/extent binding.
The full stage reservation partitions the original combined world limit before
acquisition and permanently lowers the collector ceiling; no allowance is
reused across consuming finish. Independent guards execute after callback
capture/value destruction and preserve typed causes.

Final source passes 28 focused and 491 complete collision cases per Linux
profile, CASC library/test Clippy, scoped formatting, 171 source/state and 104
distribution/sealer checks. Exact report hashes and retained unsuccessful
fixture/checker attempts are in the companion. All new comparisons are against
the resident Rust world using authored toy CASC; no new original C++ or real
client evidence is inferred. The prior 3629e8d independently passes hosted Rust
and C++ CI, including 2,116 Windows debug tests / 41 targets / 20 ignored.

This is an intermediate library phase. Selected VMO geometry/conversion, final
per-map artifacts and ancestor proofs still need disk integration; the product
continues to use resident assembly. Scalable source/read/disk/reload policies,
cross-volume support, publication/resume, complete mandatory encrypted inputs,
real geometry/runtime and reproducible Linux/Windows distribution remain open.
No new bundle, server activation or full extractor completion is claimed.

## Selected VMO disk preparation — 2026-10-12

The optional Linux [selected VMO phase](forever-collision-vmo-stage.md) consumes
the authenticated placement/world owner, derives the pre-ID-dedup placement/list
name union and excludes unused acquired dependencies. It scans all aggregate
RAW geometry before writing, then converts and verifies one complete selected
model at a time in a private bound archive. Exact source/name/class/M2-bounds
association and complete VMO reader checks survive reload. No caller model or
proof inventory can construct the owner.

Final source passes 21 new VMO and 28 affected placement tests per debug/release
profile, CASC library/test Clippy, scoped formatting, 171 source/state and 104
distribution/sealer checks, with eight strict IPC-only traces. Exact hashes and
retained failed checker/fixture history are in the companion. The preceding
placement source retains its separate 491-case full collision acceptance;
full-workspace/full-collision execution is not inferred for this increment.
Comparisons use authored toy CASC versus resident Rust; no new original C++ or
real-client result is added. The logical ledger retains all recorded world /
store/stage ownership and admits source/conversion/reader/callback overlap.

Final per-map artifacts and first-present-ancestor proofs still need disk
integration. The product driver remains resident, and v1 same-mount/read/disk /
reload limits are unchanged. Scalable explicit policies, cross-volume support,
product publication/resume, complete mandatory encrypted inputs, real geometry /
runtime and reproducible Linux/Windows distribution remain open. Preceding
8596820 independently passes hosted Rust/C++ CI, including 2,116 Windows debug
tests across 41 targets (20 ignored). No new bundle or full completion is claimed.


## Closed map/proof disk package — 2026-10-12

The optional Linux [closed disk package](forever-collision-map-package.md)
consumes the authenticated Prepared owner and assembles final native map/tree /
tile/index artifacts into private complete map/proof bundles. Selected VMOs
retain their closed pre-ID-dedup union. Parent-only tiles preserve child indexes
and use the first present ancestor payload; nearer mismatch is terminal.
One complete map at a time is built/read, and metadata enumeration performs no
body I/O. The complete inventory is preflighted before returning its owner.

Final candidate passes 39 new package and 21 affected VMO cases in both Linux
profiles, CASC library/test Clippy, scoped formatting, 171 source/state and 104
distribution/sealer checks with eight strict IPC-only traces. Complete authored
toy-CASC outputs match the resident Rust assembler; 16-map logical streaming
and the private 25-identity proof harness retain separate scopes. Exact hashes
and failed compiler/intermediate histories are in the companion documentation.
No new native/real-client, final full-workspace/full-collision local execution
or runtime acceptance is inferred. Preceding 6f731b4 independently passes hosted
Rust and C++ CI: Linux 2,516 tests/profile, Windows debug 2,116 across 41 targets.

The product driver remains resident. Completed private owners do not replace
live-client verification by the product. Scalable explicit source/read/disk /
reload and cross-volume policies, product publication/resume, complete encrypted
mandatory inputs, real geometry/runtime and reproducible Linux/Windows
distribution remain open. Existing formats, default CLI and v7 source profile
retain their contracts. No new bundle or full goal completion is claimed.


## Explicit collision storage across volumes — 2026-10-12

The optional Linux [captured-volume capability](forever-collision-captured-volumes.md)
allows the six private collision repositories to live on a separately captured
volume. The default constructor keeps its v1 source/output mount equality.
Every source/key/output node remains bound to its captured mount and held/named
identity, with unchanged alias/privacy/record/hash/EOF and sticky final guards.
All existing resource and parser ceilings remain downward-only and unchanged.

Exact source passes 40 private Spool and 39 closed-package cases per Linux
profile, plus an explicit second-filesystem corpus in both profiles. Four
complete authored toy-CASC packages match resident Rust native inventories /
bytes/counts/facts and one nearer-parent mismatch requires exact Association.
Clippy, scoped formatting, 171 source/state and 104 distribution/sealer checks
pass with ten strict IPC-only traces. Exact source/report hashes and failed
Clippy history are in the companion; no new C++/real-client or final local
whole-workspace/full-collision acceptance is inferred.

Preceding 2fe8cfb independently passes hosted Rust/C++ CI: Linux 2,555 tests per
profile and Windows debug 2,116. Current source needs its own hosted result.
The product VMap driver remains resident. Versioned scalable source/read/disk /
reload ceilings, product integration/publication/resume with live LocalStore
verification, complete encrypted mandatory inputs, real geometry/runtime and
reproducible distribution remain open. No full goal completion is claimed.


## Explicit collision cumulative resource policies — 2026-10-12

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


## Explicit Linux product collision disk plan v3 (2026-10-12)

The v9 source profile now connects all six leased append owners to the actual
product CLI through explicit v3 plans and `--scratch`. Rooted disjoint guards,
checked physical-byte/inode admission, whole-package output limits and existing
common publication/resume are integrated. Physical admission observes available
space; it does not reserve capacity. Unfinished collision resume recomputes into
a new namespace and preserves previous private attempts. v1/v2 resident defaults
and their canonical bytes remain unchanged.

Executable validation passes 2,643 ordinary Linux tests per debug/release
profile (41 targets, 26 ignored), four explicit ignored corpora per profile,
49 existing CLI cases per profile, all-target Linux/GNU-Windows Clippy, both
GNU-Windows compile profiles and scoped formatting. Final source association
reruns 187 contract and 112 distribution/sealer checks plus one added
cross-volume CLI case per profile against the exact full-run product binaries.
All Rust/Cargo and existing CLI-test source is unchanged from the full-run freeze.
This is synthetic GNU/Linux product/source acceptance; no new bundle,
real-client/native acceptance, Windows product or RustyCore runtime acceptance
is claimed.

This closes the bounded synthetic product disk preflight/publication/resume gate.
Complete encrypted mandatory inputs, real geometry and whole-world resources,
RustyCore runtime/server integration, complete Windows product execution and
reproducible v9 distribution remained open at that source-validation checkpoint;
the later bounded Linux bundle closeout is recorded below. The extractor is not
complete. See `docs/forever-product-disk.md` for the versioned contract, hashes and exact scope.


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

The original `forever-platform` private-output API now has bounded ordinary
synthetic native Windows x86_64/fixed local NTFS acceptance for functional commit
`a220596da8a29f9f3a867dc7f2bd016d01cd3558`. [CI run 38254527798, attempt 1](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798)
and [Windows job 114829043981](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798/job/114829043981)
pass on Windows Server 2022 / `windows-2022`, OS `10.0.20348.0`, with fixed local
NTFS admission recorded for the temporary fixtures. Focused debug/release each
list and execute the exact fourteen ordinary tests: 14 pass, zero fail and zero
ignored. The Windows workspace debug log has 2,151 pass / zero fail / 21 ignored;
Linux debug/release each have 2,643 pass / zero fail / 26 ignored. Each workspace
job has 41 result summaries; these are observed hosted counts.
The separate [C++ CI run 38254528030](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254528030)
builds that same functional commit with Linux Ninja and Windows Visual Studio
17; this is build evidence, not extraction or runtime acceptance.

The independent ACL oracle validates persisted directory/file process-user
ownership, protected DACL, one explicit non-inheriting FullControl ALLOW ACE,
without inherited ACEs, after output and parent leases close. Its live path probe
is only diagnostic; no sharing/NTFS cause is inferred. Child PowerShell alone
removes `PSModulePath`; only bounded numeric exception diagnostics are exposed.
Ordinary thread impersonation rejection/permanent poison and the test-only
DACL restore helper pass without weakening production security guards.

The actual checkout merge `8a208b5d01c06dccea92f82cb5cca5822eb13a45` and functional
commit share tree `5014b5f12020fc5324c6d4deca1d2335fa9077b4`. Compile6 source
freeze is `1de7e5ae036d61f5fbabea51fdf2a152486a6622cd9ba6381b0958fc900479cf`
(1,023 files). Accepted terminal native review `native-ci4-root-review.json`
has SHA-256
`353c2a48b8094eb4d2dbc5ba42bcc7fd61b505117cf3484f715edb60a757afda`;
Windows job log SHA-256 is
`962c0fe700c4f1e57e3eb77ad0a372db82d6dc168b2b699ce8680ab1aa42bb81`.
Complete log/report hashes and local source-check scopes are recorded in
[the private-output scope](forever-windows-output.md). Native CI1/CI2/CI3 and
static compile1/compile4 failures remain retained and unaccepted.

This closes the ordinary synthetic native private-output foundation gate for
that functional source. It does not certify the later documentation commit,
which this run neither rebuilds nor executes, and creates no new binary/bundle.
The frozen contract's native/hosted false flags, pending status and earlier
oracle description remain the authored checkpoint before this later external
closeout. Actual functional source and terminal evidence bind the persisted
oracle scope; product/recovery/publication flags remain false in current scope.

The privileged reparse case remains ignored and needs explicit native privilege /
NTFS execution. Windows driver, package protocol/receipts/sealing, recovery,
durable directory entries, crash durability and reproducible Windows product
packages remain gates. No atomic namespace, closed membership inventory,
same-SID/administrator isolation or global runner/harness/oracle network
exclusion is inferred. Real complete inputs/geometry, full-world operation and
RustyCore runtime/server activation remain open. The historical Linux v9 bundle,
its source profile and Linux hardlink recovery are not relabeled or migrated.
The full extractor is not complete.


## Explicit Windows membership source candidate — 2026-10-12

A separate `forever-windows-private-membership-v1` API compares complete observed
children of every live private owned directory with current entries, including
post-rename names/parents. Retained root/parent identity/security guards and full
transient directory snapshots before/after remain required. Unknown entries are
fatal without traversal/adoption; missing/duplicate/name/type/FileId mismatch and
observed scan/guard failures permanently poison rather than return a partial
successful summary. The existing `verify_unchanged` remains identity/security
only, with no automatic membership scan during append/flush/rename.

The pass has downward global ceilings of 128 directories, 512 admitted/decoded
records including dots, and 640 native calls. A charged query at the record cap
may establish EOF; additional data fails before decoding and poisons. A fixed
aligned 640-byte class-38 single-entry buffer admits complete bounded records;
strict completion/status/framing rejects partial results. Concurrent changes
may or may not be observed. This is not an atomic namespace snapshot, directory
write exclusion, same-SID/administrator isolation or durable package completeness.

New static/CI source checks are separate from the historical fourteen native
output cases accepted for a220596 and recorded later by 5ce8b83. Native membership
execution acceptance is pending; neither planned focused debug/release CI nor
Wine/cross-compilation closes that gate. See [the versioned membership scope](forever-windows-membership.md).
No Windows product driver, publication/receipts/sealing, recovery, directory-entry
or crash durability, new Linux/Windows bundle or Linux source-profile relabel is
claimed. The accepted v9 bundle and Linux hardlink protocol retain their scope.
Real complete inputs/geometry, full-world resources, RustyCore server/runtime and
full extractor completion remain open.

## Windows membership native acceptance — 2026-10-12

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

## Opt-in numeric regular multi-section DB2 — 2026-10-12

`forever-db2-regular-sections-numeric-v1` adds the explicitly invoked
`DecodedTable::read_regular_sections` library API. It admits only 2..16 complete,
unencrypted sections for existing reviewed numeric non-catalog schemas with
external IDs, no strings or parent blocks, contiguous exact framing, and whole-
file limits. The production reader, CLI, sparse path and reviewed hash gates do
not change. Global ID repair and one ordered copy pass match the pinned original
for the admitted profile; no partial result is returned. Catalog schemas,
inline IDs, TACT, sparse/text/parent blocks and nonzero catalog metadata remain
outside the profile.

Linux debug/release pass 2,675 tests each (zero failures, 26 existing ignored),
including 32 new unit cases. Linux and GNU-Windows Clippy, scoped formatting,
GNU-Windows compile-only profiles, 233 contract tests and 112 distribution tests
pass. The bounded pinned-native comparison executes all 33 retained original
source/header/notice entries at `02245dcd245e7433e524577656177723d3e4992e` and
actual depfiles use 30 original inputs plus the wrapper. Each native profile
passes 83 complete synthetic `SpellDuration` positives: 634 direct records,
107 copies and 732 materialized rows, with 1,902 direct and 2,196 materialized
signed32 scalar-bit checks. Both profiles also pass 36 Rust-only negatives,
five section-ceiling checks and two exclusive-output guards. Their 209 actual
CLI process network traces per profile are empty. The outer Python acceptance
harness is not syscall traced. The rejected initial native permission preflight
and the successful fresh native-only rerun are retained and described in
`docs/forever-db2-regular-sections.md`; the earlier nine regression commands
passed on the same source freeze.

Native coverage is signed32 `SpellDuration` only. Other API numeric widths have
unit evidence without native parity. No float, array, 64-bit, real-client,
full-extraction or runtime acceptance is added. The contract preserves its
source-time checkpoint flags and records execution in a separate field. Exact
report hashes and scope are in the companion. This increment does not complete
the extractor; full encrypted real inputs, product integration and RustyCore
runtime acceptance remain open.
