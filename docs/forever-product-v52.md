# Forever product v52 — available navigation from sealed v51

Build 70450, Linux, component `mmaps` (32), canonical selection
`navigation\tall-available-maps`. This profile consumes one existing verified v51
combined DB2/terrain/VMap package and its canonical PLAN. It never reacquires the
client and never rebuilds MAPS or VMaps. v25 terrain-only, v50 source-only,
incomplete, mixed-context and altered packages reject.

The existing option names are explicitly adapted for v52: `--terrain-package`
names the **whole combined v51 package**, and `--terrain-plan` names its v51 PLAN.
There is no second package. `--vmap-package`, `--vmap-plan`, `--client`, `--keys`
and `--scratch` are forbidden. Plan-only inspection needs no source.

```
forever-extractor inspect --plan v52.tsv
forever-extractor extract --plan v52.tsv --output NEW_OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor verify --plan v52.tsv --output OUTPUT
forever-extractor verify-links --plan v52.tsv --output OUTPUT \
  --terrain-package SEALED_V51 --terrain-plan v51.tsv
forever-extractor resume --plan v52.tsv --output INCOMPLETE_OUTPUT \
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

The versioned `AllAvailableV52` resource profile caps maps at 4096 and tiles at
4096 per map / 16,777,216 overall. It admits the 71-map source domain without
changing v51 geometry budgets. Builder/save work scales with the exact planned
tile count and is debited across maps; per-tile pipeline and owner heap ceilings
stay unchanged. This is sequential execution, not a multithread scheduler.
Coverage has a 128 MiB ceiling and records map dispositions, planned coordinates,
all written/empty outcomes, payload hashes and cumulative builder/save work.

The source identity is domain-separated v52 and binds the complete v51 summary,
including its context (source, producer and PLAN), inventory hash and byte/file
counts. Output context also binds the v52 PLAN and executing producer. Resume
replays the whole sequence and compares existing artifacts; any error leaves the
package incomplete. No partially completed map sequence is accepted as success.

## Acceptance boundary

Only synthetic/focused validation is authorized in this worker. The long real
attempt must run later in a persistent user service or tmux, with explicit
existing v51 input and new external output/log paths, never attached to the
agent. It must not repeat acquisition. This implementation does not establish
real full-world completion, complete navigation geometry, runtime acceptance or
native parity. The operator's v51 evidence (71 maps, 69 present, 3322 terrain
tiles) is input scope, not transferred v52 acceptance.
