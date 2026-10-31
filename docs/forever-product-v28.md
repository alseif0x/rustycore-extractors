# Forever product v28: authenticated selected VMap geometry

Product v28 is a Linux build-70450 profile,
`Build70450AuthenticatedSelectedGeometryV1`, with component mask **17**
(`db2,vmaps`). It adds one sealed `Map.db2` to the selected geometry producer of
v22 and requires offline authentication of its available-section VMap coverage
and selected active-ancestor domain. It does not relabel v22 or transfer its
acceptance. The machine-readable contract is
[`forever-product-contract-v28.json`](../contracts/forever-product-contract-v28.json).

The canonical plan requires `wowdbdefs-70450-v1`, disabled decryption,
`partial_encrypted_sections=declared-skip`, leased append-disk storage with its
seven budgets, `collision-domain=map:<id>`, exactly one `Map` entry and the
standard pack/terrain/navigation options. Extraction and resume require a new
private scratch; inspection and verification do not. Keys are refused.

## Closed scope and production

The selected map and its active ancestors use the existing available-section
VMap planner and the `geometry::extract_map` production path. WDT, ADT, M2, WMO
and WMO-group inputs required by that domain remain strict and fatal on failure.
`all-available-maps` is not admitted, and no complete global map coverage is
claimed.

GameObject globals and GODI startup remain excluded by policy, with zero GODI
startup reads. Models are limited to actual placement requests, with an exact
model/VMO inventory. `LiquidObject` is out of scope and is not requested;
`LiquidMaterial` and `LiquidType` retain their complete-source checks. MMaps
are not selected. These exclusions do not assert that those sources failed or
that their broader coverage is unnecessary for a complete extractor.

The only DB2 artifact is `dbc/<locale>/Map.db2`. Its authenticated
available-source path uses the v26/v27 structural and exact multi-frame
zero-fill rules. The manifest contains exclusively per-table declarations,
including `Map` when partial. The v25 global declaration format is forbidden;
the same-named field in the plan remains mandatory. No encrypted content is
decrypted or recovered.

`RCVGEO1` and `vmaps/selected-geometry-coverage-v1.bin` are preserved. v22 and
v28 have strict producer/parser wrappers for their respective masks 16 and 17;
neither accepts the other's package or coverage context. The v28 package total
is the checked sum of the selected collision-output budget and the 16-GiB DB2
ceiling. Existing geometry, per-table, frame and source limits are not raised.

## Offline Map and domain authentication

Verification performs one semantic DB2/VMap pass. It checks structural framing
and zero-fill, then reads the sealed Map with `read_vmap_maps_70450` and
materializes it once. It compares the declared and admitted section counts,
ordinals, direct/copy rows and available bytes derived from declared encrypted
extents. VMap materialization retains `WDT==0` rows needed by the graph; this is
not the terrain row count.

`Maps::from_materialized_selection` rebuilds the selected map and its active
ancestors. Verification compares `maps.stats().rows` and `active_maps`, then
checks every selected ID, effective parent, WDT FileDataID and depth through the
planner. `MapEntry::is_parent` is derived from the complete materialized graph,
including children outside the published selection, rather than inferred from
children present only in the coverage list.

`coverage_authentication=independently_derived` applies to those Map sections,
VMap direct/copy/materialized facts and selected domain. Acquisition CKey/EKey
flags remain `producer_reported`, not a new independent cryptographic proof.
The package then undergoes the existing VMO, placement, tree, tile, index and
association checks. A second package pass is only an inventory/stability guard,
not another semantic Map verdict. `verify-links` adds no MMap, terrain or
external-package acceptance.

Resume re-admits the selected Map before the component cycle so per-table
manifest declarations are reconstructed even when DB2 is sealed and no final
manifest exists. Identical artifacts and declarations are idempotent;
conflicts and missing source coverage remain fatal.

## Real selected-map acceptance — 2026-10-24

The matching producer SHA-256
`795a065776ffa24ca81b47ac52366f0c54a032bd352df921e5b203610e6e7430`
passed real build-70450 esES inspect, extract, offline verify and verify-links
for `map:48` and its active ancestors. The run used a fresh private scratch and
completed in 44 seconds under 900-second, 3-GiB, one-CPU and 64-task limits. No
network attempt was recorded, and client metadata, producer and plan remained
unchanged.

The package contains 67 artifacts / 2,090,335 bytes: one partial `Map.db2` and
66 VMap artifacts including coverage. Inventory SHA-256 is
`e848ec9121147667461918c9d38edbb5f88f0c679b7bbf92fe9a79f9a7514833`.
The `Map` payload matches v26 and all 65 non-coverage geometry payloads match
the retained v22 package. Coverage and manifest identities differ because the
new plan, profile and producer context are distinct.

The source record binds HEAD `365221fe6f22c66c95d47cfa4c67b876cd719648`,
pre-documentation worktree diff SHA-256
`8db7cdf176969e895ba3600c514eecdea1b302b4122dd71d75f30fc26637deee` and every
new source file by hash. Workspace tests passed 4,072 with 26 explicit ignores;
471 contract tests, Clippy and formatting passed. The redacted report
`v28-authenticated-geometry-map48-acceptance-20261024T125226Z-2022337.json` has
SHA-256
`0e9a83c3b56416fc31eeb4a93b89b062c37e5181dc5fda2561671cd138df8165`.

The sealed Map retains three per-table frames / 575 zero-filled bytes and no
global declaration. Offline verification independently re-derived the Map
sections, direct/copy/materialized VMap facts and active-ancestor graph. The
acquisition CKey/EKey flags remain producer-reported, and no encrypted content
was decoded.

## Qualification boundary

This accepts the selected map and its active ancestors, not complete decoded
DB2 content, whole-client or all-available VMap coverage, GODI/global
GameObjects, LiquidObject, navigation/full-world, RustyCore runtime,
C++/TrinityCore parity, a new distribution, Windows or publication.
`full_extractor_complete` remains false.
