# Forever product v41: source-expanded material-filtered WMO geometry

Product v41 defines a distinct Linux build-70450 profile,
`Build70450SourceExpandedMaterialFilteredWmoGeometryV1`, for DB2 plus available
VMap geometry (component mask **17**). Relative to v40 ModelHeavyV1, it raises
only the cumulative source-byte ceiling from 512 MiB to 1 GiB. It retains the
65,536 model-identity capacities, 1,000,000 reads, 16,000,000 requests and
dependency requests, 256 MiB raw-byte ceiling, and all other limits. RCVGEO1 v14
retains the v13 field layout and all five WMO omission lists. See the
[machine-readable contract](../contracts/forever-product-contract-v41.json).

## Status and evidence boundary

V41 synthetic implementation and validation are complete. A real build-70450
attempt was made and **failed**; this is not acceptance. Run
`v41-source-expanded-wmo-20261027T020021Z` took 115 seconds: inspect exited 0;
extract exited 9 in `vmaps` with `product_selected_geometry_limit`; probe,
verify and verify-links exited -1. `accepted_prefix=false`, network was not
attempted, inputs and client metadata were unchanged, and audit/geometry are null.
The report SHA-256 is
`d3c2bef14736c0d7523ccf3e7469a2db9c18b92665283a2b9db9e93d753cad72` at
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v41-source-expanded-wmo-20261027T020021Z-acceptance.json`.
Software SHA-256 is
`4ca63bee25f512c13452d129b5bb3fa209170380d98a186c7793b1b43e42497d`; reported
binary SHA-256 is
`18219365db26c56466deaa0257687228475bf7ebc25fd7377a345a464d2700d9`. Software
qualification recorded 85 suites, 4,558 passed, 26 ignored and 552 contracts.
The run's `diagnostic.jsonl` is absent and the report has no final CASC counters;
source/model/read counts at the failure are unknown and are intentionally not
inferred from configured ceilings.

The driver contains a relevant inventory guard at
`collision_geometry_v22.rs:2555`: `self.models.len() >= 4096`, below the profile's
65,536 model capacity. This is a concrete potential output-inventory blocker, but
the generic error and missing diagnostic do not prove it triggered in this run or
whether the failing condition was count- or byte-based. No WMO 462695 admission,
target omission lists, full-world coverage, complete geometry or runtime claim
is made. The proposed 1 GiB source limit does not establish real-boundary reach
or capacity sufficiency.

The v40 real attempt `v40-model-heavy-wmo-20261027T000044Z` failed at the
512 MiB source-byte local-store policy limit. Its report SHA-256 is
`f1851ba06cf85679f4f83d5c080352c81dff533c655843f78232f2b67b3cc0c9`; the
bounded source-probe report SHA-256 is
`93cbe5e506bd21d0f8799001b148c83168001fb718ce5e676ce1f1a5b9d29051`. At failure,
source bytes were 536,853,486 of 536,870,912, with 17,426 remaining; the probe
recorded 4,798 excess and correct parsing of the implicated geometry. These are
historical v40 facts only and do not establish v41 success, source-capacity
sufficiency or WMO admission.

## Versioned identity and plan

The CLI harness and plan-admission tests use the canonical synthetic v41 PLAN at
`tests/contracts/fixtures/forever-product-v41/all.tsv`; it contains no client data
and is not real extraction evidence. The plan retains build 70450,
`wowdbdefs-70450-v1`, disabled decryption,
`partial_encrypted_sections=declared-skip`, seven downward-only synthetic 1 MiB
budgets, one Map entry (FDID 1349477), component mask 17, all-map selection, and
no terrain or navigation. Its reduced budgets do not test the nominal source
limit.

V41 binds these identities:

- publication: `Build70450SourceExpandedMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-source-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-source-expanded-material-filtered-wmo-coverage-v14`;
- codec/source: `linux70450-source-expanded-material-filtered-wmo-v1`;
- domain: `source-expanded-material-filtered-wmo-v1`;
- resources: `linux-collision-source-expanded-resources-v1`;
- completion: `linux70450-available-source-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-source-expanded-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_source_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-source-expanded-material-filtered-wmo-geometry-coverage-v14.bin`.

## SourceExpandedV1 resource profile

Only `max_source_bytes` is raised versus v40: 536,870,912 to 1,073,741,824.
Keep `max_models=65,536`, `max_reads=1,000,000`, requests and dependency
requests at 16,000,000 each, `max_raw_bytes=268,435,456`, and all memory,
storage, WDT/cache inventory, per-map/tile callback and other limits unchanged.
Model identities, inventory entries and callbacks retain their distinct units.
The profile does not imply `DiskWorldV1`. A synthetic 1 MiB PLAN budget remains
a downward cap and does not test the nominal 1 GiB source limit.

## Coverage and omission semantics

RCVGEO1 v14 changes the serialized version and profile/resource identities while
retaining the v13 field sequence and the five ordered lists:
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`. Each remains ordered, unique, nonzero and limited to
4,096 IDs; all lists remain pairwise disjoint. WMO predicates do not change.
V40 v13 and v41 v14 wrappers must reject cross-version plans, packages and
coverage.

Offline verification checks package-visible coverage and artifacts but cannot
reproduce source-side acquisition or omission classification. Omission evidence
remains producer-reported; no output list is asserted as real target observation.

## Synthetic CLI validation scope

The synthetic Linux CLI suite passes **11/11**. Its lifecycle runs inspect, two
extracts for deterministic output, then resumes from a copied package after
removing `forever-package.json` and restoring `.forever-package-incomplete`. The
resumed package files, coverage artifact and manifest match the original bytes;
`verify` and `verify-links` pass on the resumed package. Both v40/v41 package
crossings reject. Resealed v14-to-v13 and profile-identity attacks reject, as do
all ten pairwise omission-list overlaps; each of the five lists accepts 4,096 and
rejects 4,097 in both verifiers. CLI reports expose
`source_bytes_max=1073741824`, `models_max=65536`, and `reads_max=1000000`.

Supporting synthetic evidence: v41 plan admission **7/7**, coverage codec v14
**2/2**, forever-extractor library **766/766**, and the contract suite **5/5**.
The 1 MiB PLAN budget is only a downward cap. None of these tests asserts a real
1 GiB source boundary, capacity sufficiency, WMO admission, full-world coverage or
runtime acceptance. No client data or network was used.

## V42 follow-up — design only

V42 is a **design-only; target-not-run** follow-up named `ModelOutputExpandedV1`.
It proposes a coherent output inventory capacity for 65,536 models while
retaining `source_bytes_max=1073741824`, `reads_max=1000000`, request and
dependency-request maxima of 16,000,000, `raw_bytes_max=268435456`, and all
remaining v41 limits. The output profile must separate model-count and
model-output-byte diagnostics. Planned coverage codec version: v15. It does not
implicitly select `DiskWorldV1`. No production implementation or target run is
claimed, and this design proves neither resource sufficiency nor WMO 462695
admission, full-world coverage or runtime acceptance.

Full-world resources, complete real geometry, GODI/global GameObjects,
LiquidObject, terrain, navigation/MMaps, RustyCore runtime acceptance, format
parity, Windows, reproducible distribution and publication remain open.
`full_world_coverage`, `runtime_claim`, `real_source_1gib_boundary` and
`full_extractor_complete` remain false.
