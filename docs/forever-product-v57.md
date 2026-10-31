# Forever product v57 — generated-detail vertical bounds

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. V57 consumes the same verified v51 combined
DB2/terrain/VMap package and canonical PLAN as v56. It never reacquires the
client and never rebuilds MAPS or VMaps. Incomplete, altered, mixed-context,
v25 terrain-only and v50 source-only packages reject.

The CLI retains the v56 interface: `--terrain-package` names the whole combined
v51 package and `--terrain-plan` its PLAN. `--vmap-package`, `--vmap-plan`,
`--client`, `--keys` and `--scratch` are forbidden. Plan-only inspection needs
no source.

```
forever-extractor inspect --plan v57.tsv
forever-extractor extract --plan v57.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v57.tsv --output OUTPUT
forever-extractor verify-links --plan v57.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v57.tsv --output INCOMPLETE_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
```

V57 retains v56's native partial triangulation, center fallback, native unset-
height return and resource profile: 1,000,000 vertices, 500,000 triangles and
48 MiB model/compact mesh heaps. `AvailableProfile::AllAvailableV57` alone
enables `expand_detour_vertical_bounds`; v56 and earlier profiles preserve
their prior behavior.

## Bounds correction

Real Detour queries exposed a defect not detected by tile load/unload alone.
On the original map-0 tile 20/35, every one of 677 vertices was above the
header's zero vertical range. A bounded search found polygons, but the normal
query volume found none. The map-0 audit found 270 of 736 tiles with vertices
outside their vertical header; 253 had all vertices outside, and 246 invalid
tiles declared a zero vertical range. The independent all-package audit found
1,309 invalid tiles among 3,067. These facts limit the earlier v56
available-navigation scope: it is not complete world pathfinding acceptance.

The source, compact spans, polygon mesh, detail mesh, merge and Detour payload
were inspected independently. The generated detail heights were finite and
coherent, but source-only Detour bounds did not include them. Expanding only
the vertical Detour bounds to include every generated detail vertex produced
zero out-of-range vertices and identical vertex bytes. Horizontal bounds,
polygon geometry, areas, flags and links are not rewritten by this option.

The corrected single tile completed a real native water query with both
endpoints resolved, Detour success status `1073741824` and a two-polygon
corridor. A separate valid ground tile also completed the real RustyCore
worker query with the same status and corridor length, clean unload and no
network attempt. These are bounded proofs, not all-map acceptance.

`mmaps/available-navigation-v57.tsv` retains the v56 counters and uses coverage
version 57 with domain-separated identity. Execution remains sequential in
map-ID then tile row/column order. Empty maps, empty tiles and ROOT-absent maps
produce only their declared facts. No off-mesh connections are added.

`verify` checks the complete output, intrinsic MMap16 envelopes, exact
inventory and every vertex against the tile's declared vertical bounds. A
synthetic negative test requires `product_navigation_vertical_bounds` when a
vertex escapes them. `verify-links` additionally verifies the complete sealed
v51 source and rediscovers the navigation inventory without client access.

## Current validation and remaining gates

Focused bounds-helper validation passes. V57 driver tests pass three cases
with two explicit local-diagnostic ignores. The v57 package suite passes six
cases, including base/detail vertical-bounds rejection. All eight v57 CLI cases pass in
370.38 seconds, including deterministic offline multi-map extraction, resume,
source tampering and forged inventory. The contract has three tests.

The real single-tile v56 regeneration was byte-identical before enabling the
new option. Two real tiles from the most affected maps have zero vertical
outliers and successful native queries; the map-0 case also preserves vertex
bytes. The accepted v56 package was not modified. The bounded redacted report is
[acceptance/v57-vertical-bounds-20261031.json](acceptance/v57-vertical-bounds-20261031.json),
SHA-256 `1db450c2a469d29acd9919f4892d78551b86e8ecb1b55e17765c2f697b23d253`.

The full v57 available-map run has now completed offline. `verify`,
`verify-links` and an independent rehash of every artifact pass for 3,137 files /
1,589,926,833 bytes with inventory SHA-256
`7bf0f9ae9c4d642375cfb37a5cfcd2ebe7d5e18e0e0a17947b44ce5cbd1a1dd2`.
Coverage retains 69 planned maps, two ROOT-absent maps, 3,722 candidate tiles
and 3,067 written tiles. All 3,067 tiles pass the independent vertical-bounds
audit with zero invalid tiles. The redacted extraction report is
[acceptance/v57-available-navigation-real-20261031.json](acceptance/v57-available-navigation-real-20261031.json),
SHA-256 `ca188d075f56157ee1281c027f14b615fe5bcab6a8b9ef1674016f1708466cfe`.
A first verification-wrapper timeout and an external auditor's SHA-name defect
were retained and corrected without changing producer or package bytes.

The synthetic RustyCore worker suite passes 11/11 and runtime admission 2/2.
The real worker admits the v57 package, loads the map-0 water tile, completes a
successful two-polygon Detour query and unloads with zero final tiles/queries.
Its report is [acceptance/v57-rustycore-real-20261031.json](acceptance/v57-rustycore-real-20261031.json),
SHA-256 `32e39189ac30225ff033f7ac1ab076bb32894abd3c5e5b7d0487fac45492ce6d`.
Final software closeout exits 0: 101 suites, 4,867 passed, zero failures, 31
explicit ignores and 613 contracts; log SHA-256 is
`f3fcbb882c50523507b8d6aeb9509fd87cc5cd2b6421235d43f00cd029756bad`
and release binary SHA-256 is
`08654fa118024e25a3cbc3c5d95f05d509a09e8d0c7a2e5cdc4b89aed6fdb836`.

The all-planned-map campaign has now passed 69 real native Detour queries in 69
unique maps. It loaded and unloaded 78 tiles: one tile in each of 66 maps and
four cross-linked tiles in each of maps 25, 29 and 42. Every query returned a
two-polygon corridor, all final workers were empty and no network was attempted.
The complete census found no other planned map without internal polygon links.
The redacted report is
[acceptance/v57-rustycore-world-20261031.json](acceptance/v57-rustycore-world-20261031.json),
SHA-256 `5ec61a01e6ee951d9035cfa6b50c5941a304f446a128c06ec7b3cce6322c15af`.
This accepts one real query per available planned map, not a query in every tile.

The world-server navigation loader accepts the real package in 215.13 seconds;
report [acceptance/v57-forever-runtime-20261031.json](acceptance/v57-forever-runtime-20261031.json),
SHA-256 `12db752453b8d9dbfd745066a28456e02039ccf743fb70209d7a9b748b01d6ee`.
This is verified resource admission, not a listener or full server bootstrap.
The v15 distribution is reproducible and its rebuilt executable passes real
`inspect`, `verify` and `verify-links`; receipt SHA-256 is
`a3d93f1ffed6a1ea7ea27aa3be2e17c465b807a94272d900dbc2704e1ef12a60`.

Per-tile pathfinding, full server bootstrap and gameplay acceptance remain open.
`accepted_prefix=false`, `full_world_complete=false`, `runtime_accepted=false`
and `full_extractor_complete=false`. No publication is authorized.
