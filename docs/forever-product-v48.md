# Forever product v48: GameObject missing-ROOT material-filtered WMO geometry

V48 is a separately versioned offline Linux build-70450 product for DB2 and
available-map geometry (component mask **17**). It carries forward v47's GODI
models and adds a bounded declaration for direct nonzero GODI FDIDs whose raw
ROOT has no rows. Its coverage artifact uses RCVGEO1 version 21. Synthetic plan,
CLI lifecycle, verifier, ceiling and codec-attack checks pass, including the
partial-GODI publication and geometry continuation regression. The recorded real
target attempt failed during DB2 acquisition and remains unaccepted; no
full-extractor claim is made.

## Product identity and scope

V48 uses these distinct identities:

- publication: `Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1`;
- resource profile: `GameObjectMissingRootExpandedV1`,
  `linux-collision-gameobject-missing-root-expanded-resources-v1`;
- PLAN: `linux70450-gameobject-missing-root-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21`;
- codec/source: `linux70450-gameobject-missing-root-material-filtered-wmo-v1`;
- domain: `gameobject-missing-root-material-filtered-wmo-v1`;
- completion: `linux70450-available-gameobject-missing-root-material-filtered-wmo-collision-v1`;
- verification: `linux70450-gameobject-missing-root-material-filtered-wmo-verification-v1`;
- coverage artifact:
  `vmaps/available-gameobject-missing-root-material-filtered-wmo-geometry-coverage-v21.bin`;
- GODI scope: `available-fdid-direct-missing-root-v1`;
- model scope: `placement-and-gameobject-with-missing-root-v1`.

The canonical synthetic PLAN is
`tests/contracts/fixtures/forever-product-v48/all.tsv`. It has 31 lines, component
mask 17, and the ordered DB2 entries `GameObjectDisplayInfo=1266277` followed by
`Map=1349477`. The fixture contains no client data.

The profile caps **direct nonzero GODI candidate rows at 4,096**. Separately, the
coverage list is capped at **4,096 unique FDIDs**. Repeated direct rows may share
the same missing FDID: `missing_root_rows` counts each MissingRoot outcome, while
the canonical sorted list contains each FDID once. For example, two display rows
with the same absent FDID produce `missing_root_rows=2` and a one-element list.
Replay enforces the direct candidate-row ceiling even when repeated rows collapse
to one list entry.

A declaration is emitted only when extraction's raw ROOT lookup proves there are
no rows for the FDID. An FDID present in ROOT without an available body remains a
fatal extraction error. Verification checks producer-reported outcomes against
authenticated sealed GODI physical rows, checks their deduplicated FDID set against
the canonical list, and does not independently authenticate the producer's ROOT
absence assertion.

V48 keeps the v47 resource limits unchanged. The new v21 codec binds the missing-
ROOT outcome count and unique FDID list. The count is not required to equal the
list length. Cross-profile v47/v48 packages and coverage are rejected.

## Verification boundary

Offline verification independently derives Map and GODI physical row order,
display IDs and FDIDs from sealed DB2, checks the Dtree and VMO references,
and retains the available-map domain, placement and assembly checks. GODI
dispositions and ROOT absence remain producer-reported facts checked for
consistency, not independently classified/authenticated results. The product does
not establish full-world or complete VMap coverage, LiquidObject, MMaps, runtime
acceptance, Windows support, distribution qualification or release authorization.

## Synthetic validation status

The v48 PLAN admission tests passed **4/4**. The v48 CLI integration target passed
**11/11** tests, including its helper hash vectors and product scenarios:
extract/verify/resume, deterministic bytes and v47 cross-rejection; raw ROOT
absence versus ROOT-present unavailable body; authenticated partial GODI DB2
publication followed by geometry continuation through `GameObjectModels.dtree` and
resume from partial GODI; redacted DB2-acquisition failure diagnostic and the
ROOT-present/model failure diagnostic, attributed respectively to `db2` and
`vmaps`; and the 4,096-entry ceiling with resealed count-mismatch and over-limit
codec attacks. The affected v47 CLI regression passed **7/7** after the shared
synthetic fixture changes. The offline locked workspace suite passed **4,735 tests,
0 failed, 26 ignored across 92 test targets**. All-target Clippy with `-D warnings`,
the locked release build, scoped format check, and repository contract suite also
passed. These are software-only checks and do not alter the recorded real-target
attempt, `v48-gameobject-missing-root-wmo-20261028T153819Z`, which failed at extract
(exit 9) in component `db2` with `local_store_no_verified_encoding`; its source
diagnostic was
null, no prefix was accepted, and no package was published. Its retained report is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v48-gameobject-missing-root-wmo-20261028T153819Z-acceptance.json`.
This correction's checks are synthetic/software validation only:
`real_target_attempted=true`, `real_extraction_accepted=false`,
`full_real_target_acceptance=false`, and `full_extractor_complete=false`.

Only synthetic fixtures and structured metadata belong in Git. No client assets,
keys, caches, extracted output or client-derived geometry belong in the repository,
CI or releases. See the [machine-readable v48 contract](../contracts/forever-product-contract-v48.json).
