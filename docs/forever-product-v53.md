# Forever product v53 — available navigation from sealed v51

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. This profile consumes one existing verified v51
combined DB2/terrain/VMap package and its canonical PLAN. It never reacquires the
client and never rebuilds MAPS or VMaps. v25 terrain-only, v50 source-only,
incomplete, mixed-context and altered packages reject.

The existing option names are explicitly adapted for v53: `--terrain-package`
names the **whole combined v51 package**, and `--terrain-plan` names its v51 PLAN.
There is no second package. `--vmap-package`, `--vmap-plan`, `--client`, `--keys`
and `--scratch` are forbidden. Plan-only inspection needs no source.

```
forever-extractor inspect --plan v53.tsv
forever-extractor extract --plan v53.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v53.tsv --output OUTPUT
forever-extractor verify-links --plan v53.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v53.tsv --output INCOMPLETE_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
```

`verify` checks the complete output package, intrinsic MMap16 envelopes and the
coverage/output inventory. `verify-links` additionally verifies the entire v51
package (including sealed DB2, terrain, geometry and liquid evidence), binds its
context and inventory digest, and independently rediscovers the navigation
inventory. Neither verifier consumes client data. Package integrity does not
prove the producer's ROOT-absence claims or authenticate an external publisher.

Execution keeps the partial-source owner, rather than calling a complete-source
API with partial rows. It enumerates every authenticated v51 active map in ID
order, retains declared ROOT-absent maps in coverage, and opens one selected
available owner at a time for present maps and their ancestor dependencies.
Copies remain metadata rather than navigation direct rows. Unavailable direct
ancestors or inconsistent inventories remain fatal. Each map's union of terrain
and VMap tiles executes in row/column order. Empty maps have coverage but no
invented global; empty tile outcomes have coverage but no invented MMap tile.
No off-mesh connections are added.

Recast's native negative-`ntris` case is preserved: if ear clipping cannot find
another valid ear after producing some triangles, v53 uses exactly those partial
triangles and records the count under `partials` for that map. It does not retry
an alternative triangulation or fabricate geometry. Strict v52 behavior and all
other errors remain fatal. Coverage cannot remove or alter this declaration.

The versioned `AllAvailableV53` resource profile caps maps at 4096 and tiles at
4096 per map / 16,777,216 overall. It admits the 71-map source domain without
changing v51 geometry budgets. Builder/save work scales with the exact planned
tile count and is debited across maps; per-tile pipeline and owner heap ceilings
stay unchanged. This is sequential execution, not a multithread scheduler.
Coverage has a 128 MiB ceiling and records map dispositions, planned coordinates,
all written/empty outcomes, payload hashes and cumulative builder/save work.

The source identity is domain-separated v53 and binds the complete v51 summary,
including its context (source, producer and PLAN), inventory hash and byte/file
counts. Output context also binds the v53 PLAN and executing producer. Resume
replays the whole sequence and compares existing artifacts; any error leaves the
package incomplete. No partially completed map sequence is accepted as success.

## Acceptance boundary

Focused validation passes three v53 library tests, the strict/partial Recast
controls, three contract tests, eight v53 CLI cases and all eight v52 CLI
regression cases. The complete navigation library passes 363 tests with 20
explicit ignores; the extractor library passes 858 tests. Formatting and Clippy
pass. The public legacy client-entry APIs explicitly reject v53 before I/O.
The final software closeout passed 97 suites, 4,811 tests with zero failures
and 26 explicit ignores, plus 601 contracts and the locked release build. Its
log is `reports/kimi/checks/closeout-20261030T144413Z-3193782.log`, SHA-256
`410b0a1b4e62e217ea6743143270f33f473181fbcbed0588b9ec0b1941294fba`.

The real esES run `v53-available-navigation-esES-20261030T140814Z` used the
frozen producer `2406e28ef3a6053be117f73c79814d3e06b7687a4650761cb293dca2e661cb19`
and the sealed v51 package without client reacquisition or network access. It
passed inspect, crossed the former v52 triangulation failure and then exited 9
after 1,895 seconds with `navigation_policy_limit` at map 0,
`Models(Geometry(Limit))`. One global and 473 tiles remain provisional under an
incomplete marker. No final coverage, manifest, verify/verify-links result or
partial-triangulation total exists; no prefix is accepted. The retained redacted
report is `reports/acceptance/v53-available-navigation-esES-20261030T140814Z-acceptance.json`.
The unchanged-binary diagnostic `v53-available-navigation-esES-diagnostic-20261030T144545Z`
reproduced the failure and identified the exact counter: 437,718 solid plus
66,564 liquid vertices total 504,282, exceeding the 500,000 ceiling by 4,282.
The 278,495 triangles remain below their separate 500,000 ceiling. Its redacted
report SHA-256 is `3ff709885e9e9cbc12fe43de5965c760868ab07cde6807de29141da160324294`.
A later version may change only the vertex budget and its associated heap
accounting. Full-world resources, complete navigation, RustyCore runtime and
reproducible distribution remain open.
