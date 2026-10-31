# Forever product v55 — native detail-center fallback

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. V55 consumes the same verified v51 combined
DB2/terrain/VMap package and canonical PLAN as v54. It never reacquires the
client and never rebuilds MAPS or VMaps. Incomplete, altered, mixed-context,
v25 terrain-only and v50 source-only packages reject.

The CLI retains the v54 interface: `--terrain-package` names the whole combined
v51 package and `--terrain-plan` its PLAN. `--vmap-package`, `--vmap-plan`,
`--client`, `--keys` and `--scratch` are forbidden. Plan-only inspection needs
no source.

```
forever-extractor inspect --plan v55.tsv
forever-extractor extract --plan v55.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v55.tsv --output OUTPUT
forever-extractor verify-links --plan v55.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v55.tsv --output INCOMPLETE_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
```

V55 preserves v53's native partial triangulation and v54's resource profile:
1,000,000 vertices, 500,000 triangles and 48 MiB model/compact mesh heaps. All
other map, tile, work, output and owner limits remain unchanged.

The retained v54 failure at map 509, subtile 179 exposed a separate difference
in detail-height seed selection. When Recast's depth-first walk cannot reach a
polygon center, it logs a warning and uses the last visited node as the height
seed. Strict v54 returned `DetailHeight`. V55 implements only this native
last-visited-node fallback. It does not relax absent compact seeds, invalid
height sentinels, disconnected sampling, numeric guards or other detail errors.
The existing strict `build_detail` API and all earlier product profiles retain
their previous behavior; `build_detail_v55` is the explicit opt-in.

Every fallback is counted. TileBuilder aggregates the per-polygon facts across
subtiles and tiles, and `mmaps/available-navigation-v55.tsv` records both
`partials` and `center-fallbacks` per map. Empty and ROOT-absent maps must have
zero in both counters. The parser rejects missing counters and invented values
in maps that did not execute.

Execution remains sequential in map-ID then tile row/column order. Empty maps,
empty tiles and declared ROOT-absent maps produce coverage facts without
invented artifacts. No off-mesh connections are added. Source and output
identity are domain-separated v55 and bind the v51 summary, v55 PLAN and
executing producer. Resume replays the complete sequence and accepts no partial
map sequence as success.

`verify` checks the complete output, intrinsic MMap16 envelopes and inventory.
`verify-links` additionally verifies the complete sealed v51 source and
rediscovers the navigation inventory without client access.

## Acceptance boundary

Focused validation passes three v55 library tests, three contract tests, eight
v55 CLI cases and all eight v54 CLI regression cases. The complete navigation
library passes 364 tests with 20 explicit ignores; the extractor library passes
864 tests. Formatting and Clippy pass. The frozen release producer is
`57ede1dc9004f7a6307e3da8d64349b0c610477f8beca77cf6355225825041da`.
The final software closeout passed 99 suites, 4,834 tests with zero failures
and 27 explicit ignores, plus 607 contracts and the locked release build. Its
log SHA-256 is `e397b4dcc632590d3ef47b6c9c6cc9902eb3d47e84cc259558dd4f71ab447841`.

The real attempt `v55-available-navigation-esES-20261030T192515Z` used the
sealed v51 package without client reacquisition or network access. It closed
39 maps with 2,320 provisional tiles and then reproduced `DetailHeight` at map
509, subtile 179, after 7,603 seconds. There is no final coverage, manifest,
verifier result or counter total; no prefix is accepted. The redacted report is
`reports/acceptance/v55-available-navigation-esES-20261030T192515Z-acceptance.json`,
SHA-256 `3c7846fcd4b0730200f8f404a5b2107734b960cd31b60beb8c8d22d04edb4f44`.

The center-walk fallback alone was insufficient. The bounded map-509 diagnostic
reproduced subtile 179 and identified `detail_polygon`, not `height_patch`, at
row 49, column 28 (previous successful tile 49,27). Native `getHeight` returns
`RC_UNSET_HEIGHT` when no valid sample exists; strict Rust returns
`DetailHeight`. Diagnostic report SHA-256 is
`b6b9be87528eff26d44b49636963911b9a5a104d5ab91f81aae16f5432e8c65f`.
A later profile may preserve and count only that native sentinel result.
Complete extraction, full-world resources, native parity, RustyCore runtime and
distribution remain open.
