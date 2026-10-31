# Forever product v40: model-heavy material-filtered WMO geometry

Product v40 specifies a distinct Linux build-70450 profile,
`Build70450ModelHeavyMaterialFilteredWmoGeometryV1`, for DB2 plus available VMap
geometry (component mask **17**). It follows v39 ReadHeavyV1 and proposes a
coherent 65,536 model-identity ceiling across the applicable production routes,
while preserving the separate 4,096-entry WDT/cache inventory bound and 4,096
callbacks-per-map/tile bound, as well as existing read, request, byte, memory,
storage and other limits. RCVGEO1 v13 retains the v12 field layout and all five
WMO omission lists. See the [machine-readable contract](../contracts/forever-product-contract-v40.json).

## Status and evidence boundary

This document records the v40 contract, previously received synthetic
verification, and the subsequent failed real attempt. The canonical synthetic
esES PLAN fixture at `tests/contracts/fixtures/forever-product-v40/all.tsv`
contains no client data or real extraction evidence. Earlier coordinator/Sol
evidence reported CLI 12/12, library 757/757, and a synthetic CLI-plus-auditor
pass covering 46 cases and 125 reports. The real v40 attempt is now recorded
below as **failed**, not accepted; its network access was false and the client
and inputs were unchanged.

The prior v39 build-70450 real run
`v39-read-heavy-wmo-20261026T181306Z` failed after 64 seconds at
`collision_policy_limit`, cause `max_models`, with models=4,096 and reads=7,218
(below the 1,000,000 read cap). Its external report SHA-256 is
`443644827f14008e4b04ae9bcea53b965114d92614010c38f6a767e4be99e128`. This is
v39 failure evidence only, not v40 target evidence or acceptance. The intended
v39 target omission lists were not observed output. Model-capacity sufficiency,
WMO 462695 admission, full-world coverage, complete VMap geometry and runtime
acceptance remain unproven.

## Versioned identity and plan

The canonical synthetic esES fixture path is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v40/all.tsv); it contains
30 lines and a single Map entry. The required PLAN keeps
the v39 shape: build 70450, `wowdbdefs-70450-v1`, decryption disabled, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only **synthetic** 1 MiB budgets, one Map entry (FDID 1349477), mask
17, `MapSelection::All`, and no terrain or navigation. The small fixture budgets
are not target capacity evidence.

V40 binds these identities:

- publication: `Build70450ModelHeavyMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-model-heavy-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-model-heavy-material-filtered-wmo-coverage-v13`;
- codec/source: `linux70450-model-heavy-material-filtered-wmo-v1`;
- domain: `model-heavy-material-filtered-wmo-v1`;
- resources: `linux-collision-model-heavy-resources-v1`;
- completion: `linux70450-available-model-heavy-material-filtered-wmo-collision-v1`;
- verification: `linux70450-model-heavy-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_model_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-model-heavy-material-filtered-wmo-geometry-coverage-v13.bin`.

## ModelHeavyV1 capacity proposal

The intended capability delta from v39 ReadHeavyV1 is a 65,536 model-identity
ceiling across collector/startup model limits, spool model-identity records,
placement model identities, model staging and package model inventory. The CLI
harness checks each route's actual unit; placement limits count model identities,
not callbacks or WDT/cache entries.

The inventory and stage limits have separate units and do not scale with model
identities: `inventory_limits.max_entries` remains at 4,096 WDT/cache inventory
entries; `stage_limits.max_callbacks` remains at 4,096 callbacks per map/tile.
Both values are asserted equal to ReadHeavyV1. Neither is included in the 65,536
model-identity routes.

Preserve the v39 limits: reads 1,000,000; requests and dependency requests
16,000,000 each; source 536,870,912 bytes; raw 268,435,456 bytes; memory
1,073,741,824 bytes; and all storage and other route-specific limits. The
ModelHeavy profile does not imply `DiskWorldV1`. The synthetic 1 MiB PLAN
budgets continue to cap budgets downward and do not test the nominal target
ceiling. The proposal is not evidence that 65,536 identities suffice or that any
particular real model is admitted.

## Coverage and omission semantics

RCVGEO1 v13 preserves the v12 field layout and five ordered WMO omission lists:
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`. The v13 version and profile identities change, but
there are no additions or removals in the serialized fields. Each list remains
limited to 4,096 ordered, unique, nonzero IDs; all five lists remain pairwise
disjoint. The material-filtered list remains serialized after
`unselected_wmo`. V39 v12 and v40 v13 wrappers must be strict and reject
cross-version plan/package/coverage use; older layouts remain separately
versioned.

No WMO predicate changes. `material_filtered_wmo` remains producer-reported and
requires FileDataId origin, no local missing M2 or useful omission, absent RAW
output, empty bindings/references, present nonempty selection sets, at least one
unskipped source-geometry group and no skipped source groups, all source
triangles rejected by the existing material predicate, no liquid, and successful
conversion without residual useful geometry. Useful geometry and normal
Collision/DoodadOnly outputs remain preserved. Offline verification checks the
package-visible coverage and artifact boundaries but cannot reproduce
source-side acquisition or omission classification.

## Synthetic and software validation gate

The synthetic CLI harness exercises `inspect`, `extract`, `resume`, `verify`,
`verify-links`, deterministic package bytes, genuine v39-package/v40-plan
rejection, v39/v40 plan crossing, a resealed v13-to-v12 codec mutation, and all
ten resealed pairwise omission-list overlaps. Each negative case invokes both
production verifiers. Received coordinator/Sol verification evidence reports
CLI **12/12**, including the lifecycle, resume, determinism, cross-version,
codec, overlap, and five-list 4,096/4,097 checks with both verifiers; library
tests **757/757**; and an auditor pass covering **46 cases / 125 reports**.
The synthetic CLI-plus-auditor run passed. These are received synthetic software
results, not real target acceptance.

The reported effective `geometry_limits` values are ModelHeavy **65,536** and
ReadHeavy **4,096**. The regression placement check accepts an exact synthetic
4,100-model inventory and rejects 4,101 under a reduced exact limit; the five
omission lists remain bounded to 4,096. This reduced-limit check does not
exercise the nominal 65,536 cap. The 65,536 model-identity value is a configured
profile/resource limit, not a structural 65,536/65,537 identity boundary.

The v40 structural contract suite passes 5/5, and the v40 contract parses as
JSON. The full repository formatter was reported passing after formatting the
v40 coverage helper with the workspace edition, without touching unrelated
files. The evidence above was supplied for this documentation update and was
not rerun here.

## Real v40 attempt and v41 follow-up

Real run `v40-model-heavy-wmo-20261027T000044Z` failed after 72 seconds:
`inspect` exited 0; `extract` exited 9 with `local_store_policy_limit`; probe,
verify and verify-links did not run (exit -1). No prefix was accepted; audit and
geometry are null. Network access was false and client/input state was unchanged.
The external acceptance report is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v40-model-heavy-wmo-20261027T000044Z-acceptance.json`,
SHA-256 `f1851ba06cf85679f4f83d5c080352c81dff533c655843f78232f2b67b3cc0c9`.
The diagnostic SHA-256 is
`bcebb01353a9908a19fe2e9c94c4ee8482609fb438cccbc63f5ec08dbecba10b`; software
and binary SHA-256 are `f7776c05e70428a3a0477ee5d84f11ab836b0f66f0f4da163f143dfe27fe5bac`
and `ba3f30864d4066c3f7963fa38463452bb65511169df27c5df2ae951e867e80b6`.
Software validation recorded 84 suites, 4,534 passed, 26 ignored and 547
contracts.

The redacted diagnostic identifies a typed limit in phase `models`, stage
`wmo_group`, FDID 7957271, ordinal 174, with guards false. At the frontier,
`source_bytes=536853486` of 536870912 (17,426 remaining), `raw_bytes=80938708`,
`models=4450`, `reads=7885` of 1,000,000, `request_calls=150556`,
`dependency_requests=30466`, resident=391147989, external=280577537,
external_source=11840280, store=306605630 and memory limit=1073741824; tolerated
WDT/child counts were 2/2. This points to the source-byte local-store policy
limit, not the model or read cap.

The bounded source probe report is
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate/v40-source-limit-probe-20261027T000438Z/report.redacted.json`,
SHA-256 `93cbe5e506bd21d0f8799001b148c83168001fb718ce5e676ce1f1a5b9d29051`; its
original evidence SHA-256 is
`9c68fc294b28bc0628b7ca7baf85e7b8ec6abfac1f109a66ad9155bd4b184784`. It records
22,224 decoded bytes and 4,798 excess, correct parsing, 313 triangles, 939
indices, 502 vertices, 13 batches, two outer and 14 group chunks, with no liquid.
This supports the source-byte limit as the observed failure frontier rather
than parser failure, memory exhaustion, model-capacity exhaustion or read-cap
exhaustion. It does not establish WMO admission or accepted output.

Follow-up v41 `SourceExpandedV1` is **design-only; target-not-run**. The proposal
raises only `max_source_bytes` from 536870912 to 1073741824, retains
`models_max=65536`, `reads_max=1000000`, requests/dependency requests at
16000000 each, `raw_bytes_max=268435456` and all other limits. Planned coverage
layout is v14. It implies no `DiskWorldV1` and proves neither sufficiency nor
acceptance.

## Open boundaries

V40 is **failed, not accepted**. No target omission lists were observed; WMO
462695 admission, capacity sufficiency, full-world coverage, complete VMap
component, runtime acceptance, format parity, Windows support, distribution and
release publication are not established. `full_world_coverage`, `runtime_claim`
and `full_extractor_complete` remain `false`.

Full-world resources, complete real geometry, GODI/global GameObjects,
LiquidObject, terrain, navigation/MMaps, RustyCore runtime acceptance, TrinityCore
parity, Windows, reproducible distribution and publication remain open.

## Validation record

The historical v39 failure, failed v40 attempt and proposed v41 follow-up are
retained as described above. See the [progress log](forever-progress-log.md) for
this checkpoint.
