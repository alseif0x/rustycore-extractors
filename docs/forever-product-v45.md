# Forever product v45: PlacementExpanded material-filtered WMO geometry

V45 specifies the Linux build-70450 profile
`Build70450PlacementExpandedMaterialFilteredWmoGeometryV1` for DB2 and available
VMap geometry (component mask **17**). It raises the staged-placement ceiling to
**268,435,456 bytes** (256 MiB), matching the configured PLAN
`collision-placement-bytes` maximum; the separate per-map placement ceiling is
**67,108,864 bytes** (64 MiB). Other limits follow v44 `SourceExpandedV2` and are
unchanged. Coverage is versioned RCVGEO1 v18. The synthetic v45 integration and
extractor unit tests passed. A real attempt was made, but it failed without product
acceptance; `target-not-run` remains the acceptance status. See the
[machine-readable contract](../contracts/forever-product-contract-v45.json).

## Product identity

V45 binds these identities:

- publication: `Build70450PlacementExpandedMaterialFilteredWmoGeometryV1`;
- plan: `linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-placement-expanded-material-filtered-wmo-coverage-v18`;
- codec/source: `linux70450-placement-expanded-material-filtered-wmo-v1`;
- domain: `placement-expanded-material-filtered-wmo-v1`;
- resources: `linux-collision-placement-expanded-resources-v1`;
- completion: `linux70450-available-placement-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-placement-expanded-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_placement_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-placement-expanded-material-filtered-wmo-geometry-coverage-v18.bin`.

The canonical synthetic v45 PLAN fixture
`tests/contracts/fixtures/forever-product-v45/all.tsv` is used by concurrent PLAN
admission tests. Separately, the Linux CLI test derives its temporary PLAN in
memory from `tests/contracts/fixtures/forever-product-v44/all.tsv`, changing only
the product and v45 plan/coverage identities; it does not consume the canonical
v45 fixture. Both fixtures and the test client are synthetic and contain no client
data. The PLAN storage budgets are reduced synthetic budgets and do not exercise
the nominal production limits.

## Resource contract

The intended `PlacementExpandedV1` resource profile changes only staged placement
bytes relative to v44's `SourceExpandedV2`:

- staged placements: **134,217,728 → 268,435,456 bytes** (128 → 256 MiB);
- PLAN `collision-placement-bytes` maximum: **268,435,456 bytes** (256 MiB);
- per-map placement bytes: **67,108,864 bytes** (64 MiB);
- source bytes: **2,147,483,648** (2 GiB);
- model identities: **65,536**;
- reads: **1,000,000**;
- `request_calls_max`: **16,000,000**;
- `dependency_requests_max`: **16,000,000**;
- `raw_bytes_max`: **268,435,456** (256 MiB);
- staged model/model-output bytes and PLAN VMO: **536,870,912** each (512 MiB).

The remaining `SourceExpandedV2` capacities are preserved, including the 1 GiB
memory ceiling, 4,096 WDT/cache entries, and 4,096 callbacks per map/tile. The
staged-placement ceiling and per-map budget are separate values. Configured limits
and synthetic inspect metadata do not establish capacity sufficiency, nor do they
test a real 256 MiB placement boundary. `DiskWorldV1` remains explicit and is not
implied.

The CLI may omit `total_placements`, `per_map_placements`, and
`placement_domain`. If any are emitted, the synthetic CLI gate checks the exact
JSON type, structure, and value: the total is counted independently from package
placement records; per-map counts are independently parsed per active map (including
zero-count maps); and the ordered domain comes from the synthetic Map fixture.
The expected domain is `[1, 2, 4, 5, 6]`. None of these diagnostics is inferred
from another, and production currently emits none of them. This optional check is
synthetic only and makes no real-target claim.

## Coverage and omission semantics

RCVGEO1 v18 is specified to preserve the v17 field sequence while changing the
coverage layout version and v45 identities. It retains the five ordered lists:
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`. Each list is ordered, unique, nonzero, and capped at
4,096 IDs; all five lists must be pairwise disjoint. The CLI suite covers the ten
resealed pairwise overlaps, and checks 4,096/4,097 entries in every list with both
verifiers. These are synthetic codec/package checks, not real-world capacity
results.

V44 and v45 have strict separate PLAN, package and coverage identities. The CLI
suite exercises both package-crossing directions and resealed wrong-version and
wrong coverage/source/domain/resource identity cases.

## Synthetic CLI validation scope

The synthetic Linux CLI integration test is
`crates/forever-extractor/tests/v45_placement_expanded_material_filtered_wmo_geometry_cli.rs`.
The test binary passes **16/16** Rust tests, covering these six CLI scenarios and the JSON/oracle checks:

- inspect, extract, resume, verify and verify-links;
- deterministic package files, coverage and manifest across extraction and resume;
- rejection of v44 packages under v45 and v45 packages under v44;
- resealed version and profile-identity attacks;
- all ten pairwise omission overlaps;
- both verifiers accepting 4,096 and rejecting 4,097 IDs in each omission list;
- inspect metadata for `source_bytes_max=2147483648`, `models_max=65536`,
  `reads_max=1000000`, `request_calls_max=16000000`,
  `dependency_requests_max=16000000`, `raw_bytes_max=268435456`,
  `model_bytes_max=536870912`, `placement_bytes_max=268435456`, and
  `placement_bytes_max_per_map=67108864`.

The `forever-extractor` library suite passes **803/803** tests, including v18
coverage round-trip/normalization and profile/list rejection checks. The repository
contract gate passes **574/574**. Arithmetic-only
staging tests preserve the 128 MiB cap for earlier profiles, cap v45 by the PLAN
budget up to 256 MiB, accept the 256 MiB aggregate across multiple maps, and reject
a single map above 64 MiB; they allocate no large placement buffers. These tests
and the CLI suite use only synthetic fixtures. No real client, network, full
extraction, or real 256 MiB placement-boundary test is part of this validation.

## Relationship to v44 and acceptance boundaries

V44's attempted real extraction ended with `product_selected_geometry_limit`,
but its report did not confirm the cause. V45 was attempted separately in run
`v45-placement-expanded-wmo-20261027T120702Z` (441 seconds). Inspect exited 0;
extract exited 9 with `product_selected_geometry_limit` in `vmaps`; probe, verify,
and verify-links did not run (exit -1). This generic error does not confirm the
underlying cause. Network was not attempted; inputs and client metadata were
unchanged. No prefix was accepted and no audit SHA-256 was produced. The retained
incomplete package size was 125,548,426 bytes and it included published geometry;
this observation does not establish capacity sufficiency or a diagnosis. Report SHA-256:
`482bca44af93d72cba8ea3b7a8f28b048fe63592296563070e98469b907c4f08`; software,
source and binary SHA-256 values were
`1a28e98bdf582f1f3bab5a18d16390e4ae5c5ab50e9a1057d05bf5541afe63d7`,
`8e126c04ec1716e9fd15d0236416fda00d168173c18b4cb2e66085e718b74c68`, and
`aec16378970121c8d521cf4e58671e9bcd4cf4fc0ae9b6136401b02f2ab9f690`. The stable
qualification recorded 96 suites, 4663 passed, 0 failed, 26 ignored, 574 contracts;
format, Clippy, diff, release and Windows GNU checks passed.

The `MAX_ASSEMBLED_DOMAIN_BYTES` ceiling of 134,217,728 bytes versus a 268,435,456
byte PLAN map ceiling is only a candidate for investigation in a separately
versioned follow-up. It is not a confirmed cause, proof of a real 256 MiB
placement boundary, nor a retrospective v45 limit change. V45 remains
**target-not-run** for acceptance and `real_extraction_accepted=false`. There is no
evidence of source, model-output, or placement capacity sufficiency, WMO 462695
admission, full-world coverage, complete VMap geometry, runtime acceptance, format
parity, Windows support, distribution, or release authorization. GODI/global
GameObjects, LiquidObject and MMaps also remain open. The synthetic contract and
CLI checks must not be described as evidence for those claims; `full_extractor_complete`
remains false.

Only synthetic fixtures and structured metadata belong in this repository. No
client assets, keys, caches, extracted data, or client-derived geometry belong in
Git, CI, reports, or a release. This product contract does not enable network
access.
