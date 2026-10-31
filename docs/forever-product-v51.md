# Forever product v51 — DB2, available terrain and VMap

V51 combines the existing build-70450 v25 terrain producer and v50 geometry
producer in one Linux, offline package. Its final qualification is **scoped real
extraction accepted**, with a separate successful software closeout. Acceptance
belongs to the v51 run and binary recorded below, not inherited v25/v50 evidence.
This does not establish full-world coverage or complete the extractor.

## Package contract

The canonical [PLAN](../tests/contracts/fixtures/forever-product-v51/all.tsv)
selects five DB2 tables in this order: GameObjectDisplayInfo (1266277), Map
(1349477), LiquidObject (1308058), LiquidMaterial (1132538), LiquidType (1371380).
The stable component mask is **25** everywhere: `db2=1`, `maps=8`, `vmaps=16`.
There is no public/internal translation and mask 21 is not a v51 alias. Older
products retain their own bits, masks and identities.

Terrain uses `all-available-maps` and v25's available-Map behavior. Its coverage
is `maps/compatible-terrain-coverage-v3.tsv`, with the combined package context.
Geometry retains the v50 budgets and omission policies. Its RCVGEO1 binary
layout remains **22**, but the coverage path and profile identities are v51:
`vmaps/available-db2-terrain-liquid-gameobject-root-encrypted-unavailable-geometry-coverage-v1.bin`.

LiquidObject source facts remain a separate **112-byte RCLIQ01** artifact at
`vmaps/liquid-object-complete-source-v1.bin`. This is not terrain geometry.
LiquidObject, LiquidMaterial and LiquidType must be complete. Partial sources
are fatal, and partial manifest declarations for these tables are rejected by
resume, verify and verify-links. GODI and Map retain explicit per-table partial
admission. Verification authenticates terrain Map facts, geometry Map/GODI facts,
complete liquid tables, source evidence and the sealed artifact inventory.
Root absence, encrypted unavailability and model omission outcomes retain their
producer-reported scope; offline verification does not reread client assets.

The seven leased-disk budgets and ten geometry resource limits are unchanged
from v50. The common package byte ceiling includes 64 GiB terrain, the selected
geometry output budget, the DB2 allowance and 112 bytes of RCLIQ01. The common
262144-file ceiling includes all selected components; geometry retains its own
bounded inventory limits.

The [JSON contract](../contracts/forever-product-contract-v51.json) records exact
profile strings, resource numbers and boundaries. `inspect`, `extract`, `resume`,
`verify`, `verify-links` and product diagnostics recognize v51. Inspect without a
client reports the plan; extraction/resume need `--client`, `--output` and the
leased-disk `--scratch` directory. Verification needs no client.

## Synthetic checks

- `tools/checks test -p forever-extractor --lib v51`: five PLAN cases plus the
  combined package-budget test.
- `tools/checks test -p forever-extractor --test v51_db2_terrain_liquid_vmap_cli`:
  three product tests plus five hash-fixture tests. Exercises inspect/extract,
  both verifiers, interrupted-component resume, deterministic payloads, v25/v50
  bidirectional profile rejection, shared payload parity, ten exact numeric CLI
  limits, three partial liquid sources, sealed forbidden declarations and
  tampered coverage/evidence/DB2 payloads.
- `python3 -m unittest discover -s tests/contracts -p 'test_forever_product_v51_contract.py' -v`:
  three contract checks.

Shared v25 map/tilelist and v50 collision/RCLIQ01 payloads are byte-identical in
these fixtures. Terrain coverage facts match except for the package context;
geometry coverage has intentionally distinct v51 identity and context.

## Final scoped real acceptance — 2026-10-29

Run `v51-db2-terrain-liquid-vmap-20261029T220401Z` passed with
`target_status=accepted` and `real_gate=true`. Inspect, extract, probe, verify and
verify-links all exited 0 in **1,190 seconds**. `red=false`, client metadata and
inputs were unchanged, `accepted_prefix=false`, and fresh output matched the
promoted audit.

The package contains **15,448 files / 708,743,828 bytes**, component mask **25**,
and five DB2 artifacts. Available terrain covers 71 maps: 69 loaded WDTs, two
unavailable WDTs, 3,322 tiles and 71 tilelists. Complete LiquidObject source facts
are 57 direct rows, 1,213 ordered copies and 1,270 materialized rows. All three
liquid tables are complete. Shared-payload parity with both v25 and v50 is
`matched`; v49's producer-reported GODI outcomes and omission scope remain
unchanged. Historical and candidate terrain coverage are rehashed against their
respective manifests before comparing facts with only package context excluded.

Retained SHA-256 evidence:

- Report: `058a607138d0f12bb1115f55045acbf3aeae58b9dc286b12ef61e55da4f13e1e`.
- Audit: `16e71e43c46c47928596bddbbc817ec4f46f68176d4b9fbed6e7cf3576c6f35b`.
- Binary: `3ac80914a56388d74d2d90f717c277b8a446f1142b513843d0d31af48bb3d700`.

## Final software closeout

The first closeout failed because the package-context test still expected eight
masks and omitted 25. Its test-only correction admits nine masks and checks the
exact six-field wire and roundtrip for 25, preserving platform rejection.
Production was unchanged; the rebuilt executable was byte-identical with the
same SHA-256 before the final closeout. No new real attempt was needed for this
test-only correction.

Final closeout exited 0: **95 suites, 4,788 passed, 0 failed, 26 ignored, 595
contracts**, with formatting, Clippy and release build passed. Log:
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/kimi/checks/closeout-20261029T223522Z-2741011.log`,
SHA-256 `11322eae6c14fdb1df1d87e55b776103a64a9bb914e9cd9b15dadafe6d65c916`.
Final independent Astra review passed. Software qualification is separate from
real extraction acceptance.

`real_target_attempted=true` and `real_extraction_accepted=true` apply only to this
combined available-domain scope. `full_real_target_acceptance=false` and
`full_extractor_complete=false` remain. Navigation/MMaps, RustyCore runtime,
full-world completeness, complete VMap coverage, Windows, distribution
qualification and publication remain out of scope.
