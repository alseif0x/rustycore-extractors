# Forever product v56 — native unset detail-height return

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. V56 consumes the same verified v51 combined
DB2/terrain/VMap package and canonical PLAN as v55. It never reacquires the
client and never rebuilds MAPS or VMaps. Incomplete, altered, mixed-context,
v25 terrain-only and v50 source-only packages reject.

The CLI retains the v55 interface: `--terrain-package` names the whole combined
v51 package and `--terrain-plan` its PLAN. `--vmap-package`, `--vmap-plan`,
`--client`, `--keys` and `--scratch` are forbidden. Plan-only inspection needs
no source.

```
forever-extractor inspect --plan v56.tsv
forever-extractor extract --plan v56.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v56.tsv --output OUTPUT
forever-extractor verify-links --plan v56.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v56.tsv --output INCOMPLETE_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
```

V56 preserves native partial triangulation, the v55 last-visited-node center
fallback and the v54 resource profile: 1,000,000 vertices, 500,000 triangles
and 48 MiB model/compact mesh heaps. All other limits remain unchanged.

The bounded map-509 diagnostic reproduced `DetailHeight` in `detail_polygon`,
not in the height seed. Native `getHeight` returns `RC_UNSET_HEIGHT` (`0xffff`)
when no valid sample is found; strict Rust returned `DetailHeight`. V56 returns
the same native sentinel and counts every affected sample. It does not replace
sentinel values with terrain, alter geometry or suppress other detail errors.
The strict `build_detail` API, v55's center-only fallback and all earlier
profiles preserve their prior behavior; `build_detail_v56` is the explicit
opt-in.

`mmaps/available-navigation-v56.tsv` records `partials`, `center-fallbacks` and
`unset-heights` per map. Empty and ROOT-absent maps must have all three counters
at zero. The parser rejects missing counters and invented values in maps that
did not execute.

Execution remains sequential in map-ID then tile row/column order. Empty maps,
empty tiles and declared ROOT-absent maps produce coverage facts without
invented artifacts. No off-mesh connections are added. Source and output
identity are domain-separated v56 and bind the v51 summary, v56 PLAN and
executing producer. Resume replays the complete sequence and accepts no partial
map sequence as success.

`verify` checks the complete output, intrinsic MMap16 envelopes and inventory.
`verify-links` additionally verifies the complete sealed v51 source and
rediscovers the navigation inventory without client access.

## Acceptance boundary

Focused validation passes three v56 library tests, three contract tests, eight
v56 CLI cases and all eight v55 CLI regression cases. The complete navigation
library passes 364 tests with 20 explicit ignores; the extractor library passes
867 tests with two explicit diagnostic ignores. Formatting and Clippy pass. The
frozen release producer is
`3c5dc57054c5d538c4057af07645ad64a877c68db577c61c53932abe03a0b681`.
Final closeout exited0 after 100 suites, 4,845 passed tests, zero failures and
28 explicit ignores, plus 610 contracts, formatting, Clippy, diff-check and the
locked release build. Its log is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/kimi/checks/closeout-20261031T024527Z-3550305.log`,
SHA-256 `5741e2548c66e652864c47589e79d699e13043fa207b5b465f3293e812d87c79`.

The real run `v56-available-navigation-esES-20261030T233828Z` completed in
2h46m8s with exit0, approximately 1.6 GiB peak memory and no client
reacquisition or network attempt. `verify` and `verify-links` both passed. The
independent auditor rehashed every artifact and matched the fresh inventory:
3,137 files / 1,589,926,833 bytes, inventory SHA-256
`722b10264ae1969dcf35198b253166376fb80ca72ae94bd2fc8b0713f817fd22`.
Coverage contains 71 maps: 69 planned and two ROOT-absent, with 3,722 candidate
tiles and 3,067 written tiles. It records 21 partial triangulations in seven
maps, four native unset-height samples in one map and zero center fallbacks.
Manifest SHA-256 is
`91e63500b41c2598d6ae60845ee1f2254ec235b2351118c61c8a8e4fa4139cd8`;
coverage SHA-256 is
`15b9c474ed630d406a4198d3fa281b87eb64f6c6e1e73f19b9a53765c22960f9`.
The redacted acceptance report SHA-256 is
`8599ae7572cf3ce9b60e105bb0c3f82fd61429b53a0541d85d8f324d57ed229d`;
its independent audit SHA-256 is
`89b6da7870314dad612430f0a60bc986d7d72e17b3b6782afd24a7ba14ec0d78`.

The first verification wrapper reached its 300-second caller limit after
`verify` had passed and interrupted `verify-links`; the latter was rerun
successfully with a larger caller limit. The initial external auditor also had
two tooling defects: its map-order predicate was reversed and its totals used
`root-absent` to index the `root_absent` counter. Both were corrected in the
external auditor only. The producer, plan and package bytes were unchanged, and
the corrected auditor independently rehashed and accepted the complete output.
These retained tooling failures are not extraction failures.

This accepts only the versioned available-navigation-v56 scope from sealed
v51. `accepted_prefix=false`, `full_world_complete=false`,
`runtime_accepted=false` and `full_extractor_complete=false` remain. Native
parity, RustyCore runtime acceptance and reproducible Linux distribution are
still open.
