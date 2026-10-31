# Forever product v39: read-heavy material-filtered WMO geometry

Product v39 defines a distinct Linux build-70450 profile,
`Build70450ReadHeavyMaterialFilteredWmoGeometryV1`, for DB2 plus available VMap
geometry (component mask **17**). It retains the v38 `SourceHeavyV1` resource
limits and raises only `max_reads`, from 4,096 to 1,000,000. The profile and
RCVGEO1 v12 layout are versioned separately; no WMO omission rule changes.
See the [machine-readable contract](../contracts/forever-product-contract-v39.json).

## Status and evidence boundary

A real v39 target attempt **failed**; it is not an accepted extraction. Run
`v39-read-heavy-wmo-20261026T181306Z` inspected successfully (exit 0), but
extract exited 9 with `collision_policy_limit`, cause `max_models`, after 64
seconds. Probe, verify and verify-links did not run (exit -1). Network access was
false, client metadata and inputs were unchanged, `accepted_prefix=false`, and
audit and geometry are null. The report remains in the external workstation
laboratory at
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v39-read-heavy-wmo-20261026T181306Z-acceptance.json`
(SHA-256 `443644827f14008e4b04ae9bcea53b965114d92614010c38f6a767e4be99e128`);
it is not present in this checkout and is not copied into the repository. The
local diagnostic is
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate/v39-read-heavy-wmo-20261026T181306Z/diagnostic.jsonl`
(SHA-256 `03819305bb2c919d5b36287cc81158e80ba560c1689ffafadb4a2aec361bd701`).
Software and executable SHA-256 values are `aa2fa1e905b27c0c8766f28d275d84d178b8f6e228473592a7b3f2af80ee8c53`
and `86cdfd8bee9a29e377e68bc407459313ef338246253508e10a3096010b046992`.

At failure: 128,773 request calls, 7,218 reads, 29,131 dependency requests,
507,339,175 source bytes, 77,203,828 raw bytes, and exactly 4,096 models. Reads
were below 1,000,000; source bytes were below 536,870,912; raw bytes were below
268,435,456. Other counters: resident 376,113,281; external 279,511,403;
external-source 11,870,712; store 306,605,630; memory limit 1,073,741,824;
tolerated WDT/child 2/2; guards were false. This establishes the model ceiling
as the frontier for this run only. The candidate v40 design raises model-identity
capacities in collector/startup, spool identity records, placement identities,
model staging and package inventory. WDT/cache inventory entries (4096 entries)
and stage callbacks (4096 callbacks per map/tile) are separate units, remain
unchanged from ReadHeavyV1, and are not model-identity capacities. This design
proposal does not alter the failed v39 run evidence or prove that the additional
model-identity capacities suffice.

Software evidence was 83 suites, 4,504 passed, 26 ignored and 542 contracts.
The intended five target omission lists are **not observed output**; WMO 462695
admission, capacity sufficiency, full-world coverage, complete VMap geometry,
RustyCore runtime acceptance and release qualification remain unproven. The
prior v38 run `v38-source-heavy-wmo-20261026T163808Z` failed at `reads=4096`
(report SHA-256
`67a07b1b6fcbe87eb89eba504541252b26ada1295d899dd4ef5c9a3e93b4d54d`) and is
prior evidence only.

## Versioned identity and plan

The canonical synthetic esES fixture is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v39/all.tsv). It retains
the v38 plan shape: build 70450, `wowdbdefs-70450-v1`, decryption disabled,
mandatory `partial_encrypted_sections=declared-skip`, leased disk storage with
seven downward-only **synthetic** 1 MiB budgets, one Map entry (FDID 1349477),
mask 17, `MapSelection::All`, and no terrain or navigation. The small fixture
budgets remain distinct from the nominal resource profile and are not target
capacity evidence.

V39 binds these identities:

- publication: `Build70450ReadHeavyMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-read-heavy-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-read-heavy-material-filtered-wmo-coverage-v12`;
- codec/source: `linux70450-read-heavy-material-filtered-wmo-v1`;
- domain: `read-heavy-material-filtered-wmo-v1`;
- resources: `linux-collision-read-heavy-resources-v1`;
- completion: `linux70450-available-read-heavy-material-filtered-wmo-collision-v1`;
- verification: `linux70450-read-heavy-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_read_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-read-heavy-material-filtered-wmo-geometry-coverage-v12.bin`.

## ReadHeavyV1 resource delta

`ReadHeavyV1` raises only `max_reads` versus v38 `SourceHeavyV1`:

| Limit | V38 SourceHeavyV1 | V39 ReadHeavyV1 |
| --- | ---: | ---: |
| `max_reads` | 4,096 | 1,000,000 |
| `max_requests` | 16,000,000 | 16,000,000 |
| `max_dependency_requests` | 16,000,000 | 16,000,000 |
| `max_source_bytes` | 536,870,912 | 536,870,912 |
| `max_raw_bytes` | 268,435,456 | 268,435,456 |

All other SourceHeavyV1 caps and extraction choices remain unchanged;
`DiskWorldV1` is not implied. In particular, the fixture's reduced 1 MiB PLAN
raw budget still constrains source and raw geometry limits downward. Synthetic
runs using that plan do not exercise or certify the nominal 1,000,000-read
real-target ceiling.

## Coverage and omission semantics

RCVGEO1 v12 keeps the v11 field layout and the same five ordered WMO omission
lists: `missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`. Each list remains limited to 4,096 ordered, unique,
nonzero IDs, and the lists remain pairwise disjoint. The material-filtered list
is serialized after `unselected_wmo`. V38 v11 and v39 v12 wrappers are strict
and reject cross-version package/coverage use; older layouts remain separately
versioned.

No WMO predicate changes. `material_filtered_wmo` remains producer-reported and
requires FileDataId origin, no local missing M2 or useful omission, absent RAW
output, empty bindings/references, present nonempty selection sets, at least one
unskipped source-geometry group and no skipped source groups, all source
triangles rejected by the existing material predicate, no liquid, and successful
conversion without residual useful geometry. Useful geometry and normal
Collision/DoodadOnly outputs remain preserved. Verification independently checks
the package-visible coverage and artifact boundaries but cannot reproduce
source-side acquisition or omission classification.

## Synthetic and software validation

The synthetic CLI gate passed **16/16** against the available production and
fixture integration. It covers `inspect`, `extract`, `resume`, `verify`,
`verify-links`, deterministic package bytes, v38/v39 crossings, omission lists,
coverage/policy/profile attacks, and list boundaries. Synthetic validation also
passed: contract **5/5**, plan **6/6**, RCVGEO1 v12 codec **2/2**, and
`collision_geometry_v22` **38/38** after correcting the v38 oracle. The real
attempt's software evidence was 83 suites, 4,504 passed, 26 ignored and 542
contracts. These software and synthetic results do not override the failed
real extraction.

## Design-only follow-up: proposed v40 ModelHeavyV1

A possible v40 follow-up is **design-only** and **target-not-run**. It would
consider a coherent 65,536 model/identity capacity across collector and startup
model limits, spool model-identity records, placement model identities, model
staging identities and package model inventory.
WDT/cache inventory entries remain capped at 4,096 entries, and stage callbacks
remain capped at 4,096 callbacks per map/tile, unchanged from ReadHeavyV1; neither
is a model-identity count. The proposed coverage layout is v13. Preserve
ReadHeavyV1 limits (`reads=1,000,000`,
requests/dependency requests 16,000,000, source 536,870,912 bytes, raw
268,435,456 bytes) and all existing byte, memory, storage and other caps. Do not
imply `DiskWorldV1`. This is not a v40 implementation or a capacity-sufficiency,
WMO admission, full-world or runtime claim.

No client or network was used by the synthetic/software validations. For the
reported real attempt, network access was false and inputs/client remained
unchanged. Full-world resources, complete VMap geometry, GODI/global GameObjects,
LiquidObject, terrain, navigation/MMaps, runtime acceptance, TrinityCore parity,
Windows, reproducible distribution and publication remain open.

`full_world_coverage`, `runtime_claim` and `full_extractor_complete` remain
`false` until separately qualified.
