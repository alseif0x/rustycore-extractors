# Forever product v54 — expanded vertex budget for available navigation

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. V54 consumes the same verified v51 combined
DB2/terrain/VMap package and canonical PLAN as v53. It never reacquires the
client and never rebuilds MAPS or VMaps. Incomplete, altered, mixed-context,
v25 terrain-only and v50 source-only packages reject.

The option names deliberately retain the v53 interface: `--terrain-package`
names the whole combined v51 package and `--terrain-plan` names its PLAN.
`--vmap-package`, `--vmap-plan`, `--client`, `--keys` and `--scratch` are
forbidden. Plan-only inspection needs no source.

```
forever-extractor inspect --plan v54.tsv
forever-extractor extract --plan v54.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v54.tsv --output OUTPUT
forever-extractor verify-links --plan v54.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v54.tsv --output INCOMPLETE_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
```

V54 preserves v53's native partial triangulation and its `partials` counter per
map. Strict v52 remains strict, v53 keeps its 500,000-vertex profile, and all
other geometry, work, map, tile, pipeline, output and owner ceilings remain
unchanged. No alternative triangulation or geometry fabrication is introduced.

The only resource change responds to the retained v53 diagnostic: that run had
437,718 solid and 66,564 liquid vertices (504,282 total), exceeding its
500,000-vertex ceiling by 4,282, while 278,495 triangles remained below the
500,000-triangle ceiling. `AllAvailableV54` therefore permits at most 1,000,000
vertices and 48 MiB for each model/compact mesh heap. The triangle ceiling is
unchanged. The 48 MiB heap accounts for the larger mesh and compaction index
storage; the selected-owner-plus-one-pipeline heap ceiling is unchanged.

Execution remains sequential in map-ID then tile row/column order. Empty maps,
empty tiles and declared ROOT-absent maps produce coverage facts without
invented artifacts. No off-mesh connections are added. Source and output
identity are domain-separated v54 and bind the v51 summary, v54 PLAN and
executing producer. Resume replays the complete sequence and accepts no partial
map sequence as success.

`mmaps/available-navigation-v54.tsv` records source context and inventory,
map dispositions, planned coordinates, written/empty outcomes, bytes and
SHA-256, per-map partial triangulations and cumulative builder/save work.
`verify` checks the complete output, intrinsic MMap16 envelopes and inventory.
`verify-links` additionally verifies the complete sealed v51 source and
rediscovers the navigation inventory without client access.

## Acceptance boundary

Focused validation passes three v54 library tests, three contract tests, eight
v54 CLI cases and all eight v53 CLI regression cases. The complete navigation
library passes 363 tests with 20 explicit ignores; the extractor library passes
861 tests. Formatting and Clippy pass. The frozen release producer is
`64d1fa98af7796100aeb3a9b61b743379c980606e098eaf52e0cd413afb7e0bf`.
The final software closeout passed 98 suites, 4,822 tests with zero failures
and 26 explicit ignores, plus 604 contracts and the locked release build. Its
log SHA-256 is `a26a24913afb140ae0eaec92f36dd6f566eb094b48ea4079ff37848e9260a888`.

The real attempt `v54-available-navigation-esES-20261030T154933Z` used the
sealed v51 package without client reacquisition or network access. It crossed
the former vertex failure and closed 39 maps with 2,320 provisional tiles, then
exited 9 after 7,447 seconds at map 509, subtile 179, `DetailHeight`. There is
no final coverage, manifest, verifier result or partial-triangulation total;
no prefix is accepted. The redacted report is
`reports/acceptance/v54-available-navigation-esES-20261030T154933Z-acceptance.json`,
SHA-256 `fd5e735c7d4e40d32b37f35f347f335ff54cb318e70907664e2538ef9dd7acdc`.

Native Recast logs a warning when its polygon-center walk is exhausted and
uses the last visited node as the height seed; strict v54 returns `DetailHeight`.
The existing disconnected-center synthetic case reproduces this distinction.
A later version may enable only that native fallback and account for it. No
complete extractor, full-world resource coverage, native parity, RustyCore
runtime acceptance or reproducible Linux distribution is claimed.
