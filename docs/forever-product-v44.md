# Forever product v44: SourceExpandedV2 material-filtered WMO geometry

V44 defines the Linux build-70450 profile
`Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1` for DB2 and available
VMap geometry (component mask **17**). Its proposed resource delta raises only the
source-byte ceiling from **1 GiB** to **2 GiB**; all other configured acquisition,
model, staging, storage, and geometry limits remain unchanged. Synthetic PLAN
admission and CLI integration tests have passed. A real extraction was attempted
and failed; acceptance remains **target-not-run**, and no real 2 GiB boundary is
claimed. See the
[machine-readable contract](../contracts/forever-product-contract-v44.json).

## Product identity

V44 binds these identities:

- plan: `linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17`;
- codec/source: `linux70450-source-expanded-v2-material-filtered-wmo-v1`;
- domain: `source-expanded-v2-material-filtered-wmo-v1`;
- resources: `linux-collision-source-expanded-resources-v2`;
- completion: `linux70450-available-source-expanded-v2-material-filtered-wmo-collision-v1`;
- verification: `linux70450-source-expanded-v2-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_source_expanded_v2_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-source-expanded-v2-material-filtered-wmo-geometry-coverage-v17.bin`.

The PLAN admission tests use the canonical synthetic v44 PLAN fixture at
`tests/contracts/fixtures/forever-product-v44/all.tsv`. Separately, the CLI test
derives its v44 PLAN in a private temporary directory by transforming the v43
synthetic fixture in memory; the CLI does not consume the canonical v44 PLAN
fixture. Neither fixture contains client data or constitutes real extraction
evidence. Both plans retain build 70450, `wowdbdefs-70450-v1`, disabled
decryption, `partial_encrypted_sections=declared-skip`, Map FDID 1349477, all-map
selection, component mask 17, and no terrain or navigation. Their storage budgets
are reduced test budgets, not nominal production boundaries.

## Resource contract

`SourceExpandedV2` changes only `max_source_bytes`:

- source bytes: **1,073,741,824 → 2,147,483,648** (1 → 2 GiB);
- model identities: **65,536**, unchanged;
- reads: **1,000,000**, unchanged;
- requests and dependency requests: **16,000,000** each, unchanged;
- raw bytes: **268,435,456** (256 MiB), unchanged;
- PLAN VMO and staged model-output bytes: **536,870,912** each (512 MiB), unchanged;
- memory: **1,073,741,824** (1 GiB), unchanged;
- WDT/cache entries: **4,096**; callbacks: **4,096 per map/tile**, unchanged.

The same limits are intended for collector and startup collision acquisition. Model
identity routes remain coherently capped at 65,536. `DiskWorldV1` remains explicit
and is not implied. These configured values and synthetic inspect assertions do
not establish that 2 GiB or any other nominal real boundary is sufficient.

## Coverage and omission semantics

RCVGEO1 v17 changes the version and v44 profile identities while preserving the
v16 field sequence and the same five ordered lists: `missing_m2`, `missing_wmo`,
`empty_wmo`, `unselected_wmo`, and `material_filtered_wmo`. Each list remains
ordered, unique, nonzero, and bounded to 4,096 IDs; the lists remain pairwise
disjoint. The synthetic test plan rejects all ten resealed pairwise overlaps and
checks each list at 4,096 and 4,097 entries with both verifiers. These are
coverage-list checks, not model-count or real-world boundary evidence.

V43 and v44 have strict separate PLAN/package/coverage identities. The CLI tests
cover both verifier crossing directions and resealed prior-version and wrong
coverage/source/domain/resource identity cases.

## Synthetic CLI test scope

The Linux CLI integration test is
`crates/forever-extractor/tests/v44_source_expanded_v2_material_filtered_wmo_geometry_cli.rs`.
Its synthetic cases cover:

- inspect, extract, resume, verify, and verify-links;
- deterministic package files and manifest across repeated extract and resume;
- rejection of v43 packages under v44 and v44 packages under v43;
- resealed v16 coverage-version and wrong v44 profile identity rejection;
- all ten pairwise omission overlaps;
- both verifiers accepting each 4,096-entry omission list and rejecting 4,097;
- reduced-plan inspect reporting `source_bytes_max=2147483648`, `models_max=65536`,
  `reads_max=1000000`, and `model_bytes_max=536870912`.

The integrated CLI suites passed: v43 **13/13** and v44 **15/15**; v44 PLAN
admission passed **5/5**, and the full `forever-extractor` library suite passed
**794/794**. Clippy passed for all `forever-extractor` targets.

The v17 codec golden is **1,022 bytes**, SHA-256
`6ca62a384a97721adfaba63477df1b95921832d72571bbdc9298ccadbc60b418`. For the
no-omission sample, normalization to v16 matches the static v16 golden encoding
(1,033 bytes, SHA-256
`3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a`). The test
also compares normalized v17 bytes dynamically against freshly encoded equivalent
v16 samples for both empty and populated omission lists.

These CLI cases use a synthetic client fixture and reduced PLAN budgets. They do
not exercise a real 2 GiB boundary or establish model/source capacity sufficiency
or runtime acceptance.

## Real v44 attempt and limits of inference

Run `v44-source-expanded-v2-wmo-20261027T092051Z` completed in 426 seconds.
Its acceptance report is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v44-source-expanded-v2-wmo-20261027T092051Z-acceptance.json`,
SHA-256 `2a29530f77a5f6f72415288f6c6c266d8697fc424d95b6e0c7db8e7891a1966d`.
The software SHA-256 is
`312a01d66144e8fbb6ab76b54871c2eb030bd67c43ba9c2762d47b30a865c3ad`; binary
SHA-256 is `622b3c88d9c24c30109a90a8f3e09d7f1bfb9fb689ea6e5b0f9ddb74442b65e3`.

Inspect exited 0; extract exited 9 with `product_selected_geometry_limit` in
`vmaps`. Probe, verify, and verify-links did not run (exit -1). Network was not
attempted; inputs and client metadata remained unchanged. No accepted prefix,
audit artifact digest (`audit_sha256`), or geometry was produced; the source
diagnostic and final counters are unavailable. The report does not confirm a root
cause.

The report records an effective staged-placement limit of **128 MiB**
(`MAX_STAGED_PLACEMENT_BYTES`) against the existing **256 MiB** PLAN placement
budget. This is a plausible follow-up candidate only, not an observed diagnosis.

## V45 follow-up — synthetic integration complete, target not run

The `PlacementExpandedV1` profile raises aggregate staged-placement capacity from
128 MiB to at most the 256 MiB PLAN cap, retains the 64 MiB per-map placement
limit and all other limits, and versions coverage as v18. Synthetic integration
and unit checks passed, including arithmetic-only checks of the aggregate and
per-map boundaries; no large staging buffers were allocated. The CLI asserts the
64 MiB per-map inspect limit when emitted. Diagnostics keep total placements,
per-map placements, and placement domain separate; one is not inferred from
another. V45 remains **target-not-run**. This does not imply `DiskWorldV1`,
placement/source/model capacity sufficiency, WMO 462695 admission, full-world
coverage, or runtime acceptance.

## V43 predecessor evidence and limits of inference

Real v43 run `v43-model-bytes-expanded-wmo-20261027T070025Z` failed at its
configured 1 GiB source policy with `local_store_policy_limit` while reading WMO
group FDID 248722. Its acceptance-report SHA-256 is
`69c44bfae09f60200640758c741d2f0607dd893fa5bbede96fae3bfed96e1a81`. At failure,
311,233 source bytes remained. A separate bounded offline probe observed 314,478
decoded bytes for the group, a **3,245-byte** excess. The API did not expose an
independent declared catalogue length; that excess is based on decoded length
only. This predecessor evidence does not establish that v44's 2 GiB cap suffices,
that any WMO is admitted, or that extraction will complete.

## Acceptance boundaries

V44 real extraction was attempted but failed; acceptance remains `target-not-run`.
There is no real 2 GiB boundary evidence. Source-capacity sufficiency, model-output sufficiency, WMO
462695 admission, full-world coverage, complete VMap geometry, runtime acceptance,
format parity, Windows support, distribution, and publication remain unproven.
The synthetic contract and CLI cases must not be described as evidence for those
claims.

Only synthetic fixtures and structured metadata belong in this repository. No
client assets, keys, caches, extracted data, or client-derived geometry belong in
Git, CI, reports, or a release. This product contract does not enable network
access.
