# Forever product v43: model-bytes-expanded material-filtered WMO geometry

V43 specifies the Linux build-70450 profile
`Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1` for DB2 and
available VMap geometry (component mask **17**). Synthetic CLI checks passed:
v43 CLI **13/13**, v42 CLI regression **12/12**, extractor library **788/788**,
v43 PLAN admission **5/5**, and v43 Python contract checks **6/6**. A real target
attempt then **failed** at the configured 1 GiB source-byte limit while reading
WMO group FDID 248722. The failure provides no accepted extraction or geometry;
see the bounded evidence below. See the
[machine-readable contract](../contracts/forever-product-contract-v43.json).

## Product identity

V43 binds these identities:

- plan: `linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16`;
- codec/source: `linux70450-model-bytes-expanded-material-filtered-wmo-v1`;
- domain: `model-bytes-expanded-material-filtered-wmo-v1`;
- resources: `linux-collision-model-bytes-expanded-resources-v1`;
- completion: `linux70450-available-model-bytes-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-model-bytes-expanded-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_model_bytes_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-model-bytes-expanded-material-filtered-wmo-geometry-coverage-v16.bin`.

The PLAN admission tests use the canonical synthetic fixture
`tests/contracts/fixtures/forever-product-v43/all.tsv`. The CLI test constructs a
synthetic v43 PLAN in its private temporary directory by transforming the v42
fixture in memory. Both contain no client data. The PLAN retains build 70450,
`wowdbdefs-70450-v1`, disabled decryption,
`partial_encrypted_sections=declared-skip`, Map FDID 1349477, all-map selection,
component mask 17, and no terrain or navigation. Its storage budgets are reduced
synthetic test budgets, not tests of nominal production ceilings.

## Resource contract

The synthetically validated resource profile is `ModelBytesExpandedV1`. The two
byte limits are raised separately:

- `MAX_STAGED_MODEL_BYTES`: **402,653,184 → 536,870,912 bytes** (384 → 512 MiB);
- PLAN `collision-vmo-bytes`: **268,435,456 → 536,870,912 bytes** (256 → 512 MiB).

The effective model-output budget is the minimum of the PLAN and staging limits:
`min(536,870,912, 536,870,912) = 536,870,912` bytes (512 MiB). Keeping these
ceilings distinct is intentional. The proposed acquisition limits and other
profile capacities remain unchanged:

- source bytes: **1,073,741,824**;
- model identities: **65,536**;
- reads: **1,000,000**;
- requests and dependency requests: **16,000,000** each;
- raw bytes: **268,435,456**;
- memory: **1,073,741,824**; WDT/cache entries **4,096**; callbacks **4,096 per map/tile**.

These assertions validate configured limits and metadata only; they do not show
that a nominal real boundary is sufficient. `DiskWorldV1` is not implied.

## Coverage and omission semantics

RCVGEO1 v16 changes the version and v43 profile identities while preserving the
v15 field sequence and five ordered lists: `missing_m2`, `missing_wmo`,
`empty_wmo`, `unselected_wmo`, and `material_filtered_wmo`. Each list remains
ordered, unique, nonzero, and limited to 4,096 IDs; the lists remain pairwise
disjoint. The test plan includes rejecting all ten resealed pairwise overlaps and
checking each list at 4,096 and 4,097 entries with both verifiers. These are
synthetic coverage-list checks, not model-count or real-world-boundary evidence.

V42 and v43 have strict separate PLAN/package/coverage identities. The passed
CLI checks exercise both cross-version directions and resealed wrong-version,
coverage-profile, source-profile, domain-profile, and resource-profile cases.

## Synthetic CLI plan and evidence

The v43 CLI target passed **13/13** tests against the integrated extractor. It
exercises `inspect`, synthetic `extract`, `resume`, `verify`, and `verify-links`;
deterministic files and manifest across repeated extraction and resume; v42↔v43
cross-version rejection; resealed codec-version/profile/resource identity attacks;
all ten pairwise omission overlaps; all five omission-list 4,096/4,097 boundaries
with both verifiers; and reduced-plan metadata preserving
`source_bytes_max=1073741824`, `models_max=65536`, `reads_max=1000000`, and
`model_bytes_max=536870912`. The v42 CLI regression target passed **12/12**.
The extractor library passed **788/788**, and v43 PLAN admission passed **5/5**
(including the separate canonical-fixture check). The Python v43 contract tests
passed **6/6**.

The synthetic RCVGEO1 v16 golden is **1,033 bytes**, SHA-256
`3b05c44c5226eee5c163e3af878f32c61e0eb49fb5aa069b75b3e480def70e8a`. After
normalizing the v16 version and v43 coverage, codec, domain, and resource
identities to their v15 values, the bytes match the v15 encoding byte for byte.
This is deterministic synthetic codec evidence, not real-target acceptance.

These synthetic results remain separate from the real-target run below; they do
not override its failure or establish real capacity sufficiency.

## Real-target failure and bounded source-limit probe

Run `v43-model-bytes-expanded-wmo-20261027T070025Z` completed in **220 seconds**.
`inspect` exited 0; `extract` exited 9 with `local_store_policy_limit` in
`vmaps`; probe, verify, and verify-links did not run (exit **-1**). No network was
attempted, the client inputs and metadata were unchanged, and there was no
accepted prefix, audit, or geometry. The acceptance report SHA-256 is
`69c44bfae09f60200640758c741d2f0607dd893fa5bbede96fae3bfed96e1a81`.
Its recorded software JSON SHA-256 and the local `software.json` SHA-256 are both
`be1ea515cbe30f3989ceeefcac56378385dd3cd24a0ec37fbd7cfb59e71eeac6`; this is
not the v42 software hash.

The diagnostic is `diagnostic.jsonl` (SHA-256
`cc44adc0b1d2648ecb8fac9d9d9d5fa4485a61d292c8ee5722775a6a518ad0d7`), not
`diagnostic.json`. It identifies phase `models`, request `wmo_group`, FDID
248722, ordinal 1034, typed cause `Limit`, with guards false. Source bytes were
1,073,430,591 of the configured 1,073,741,824, leaving 311,233 bytes. Other
observed counters: raw bytes 174,255,498; models 7,432; reads 16,805; requests
638,555; dependency requests 52,704; resident bytes 520,358,907; external bytes
294,467,663; external source bytes 11,866,356; store resident bytes 306,605,630;
memory limit 1,073,741,824; tolerated WDT absences 2 and child absences 10.
These are counters at failure, not completeness or omission lists.

A separate bounded offline diagnostic probe for the same build, FDID, and ordinal
has redacted-report SHA-256
`aad35ce11039b604a01e2d8a1bc4b220147c6dd735d06ee70d07506d882f586d` (the
original probe output hash is `7585ab52cdc9151f402ac97c8ccb9438e0148c9bbf8e0f175ed8cf707466e25b`).
The strict read failed with `local_store_policy_limit`; an authenticated complete
diagnostic read and WMO group parse succeeded. It observed 314,478 decoded source
bytes against 311,233 remaining, an excess of **3,245** and a hypothetical total
of 1,073,745,069 bytes. The API exposes no independently declared catalogue
length, so this excess is computed from decoded bytes only. The parsed group had
a 314,458-byte MOGP payload, two outer chunks, 12 group chunks, 6,777 triangles,
20,331 indices, 5,863 vertices, 12 batches, and no liquid. This single-group probe
is diagnostic evidence only; it is not an extraction acceptance or proof that a
2 GiB cap suffices.

## V44 follow-up design only

The proposed follow-up is a separately versioned `SourceExpandedV2` product: raise
only `max_source_bytes` from 1 GiB to 2 GiB; preserve the model, model-output,
staging, read, request, dependency-request, and raw-byte limits; use planned
coverage layout v17; and keep the disk-world profile explicit. **V44 is design-only
and has not been implemented or run.** Source-capacity sufficiency, model-output
sufficiency, WMO 462695 admission, full-world coverage, and runtime acceptance
remain unproven.

## V42 predecessor evidence and limits of inference

Real run `v42-model-output-expanded-wmo-20261027T044414Z` failed with
`product_selected_geometry_model_bytes_limit`; report SHA-256:
`347548262472d679b08efa89b4a988dfa3e05c9cb5307ff4586fe5af24ccc651`.
Its final source/model/read counters are unavailable and remain unknown. The
specific error establishes only that the failing model passed v42's configured
model-count check (maximum 65,536) before the converted-byte budget check. It
does **not** establish the total model count or that more than 4,096 models were
present.

For v42, PLAN VMO was capped at 268,435,456 bytes (256 MiB) and the separate
staging ceiling was 402,653,184 bytes (384 MiB); the effective byte budget was
`min(268,435,456, 402,653,184) = 268,435,456` bytes (256 MiB). The v42 evidence
does not prove the 384 MiB staging ceiling was reached and does not transfer
acceptance to v43.

## Acceptance boundaries

V43 real-target acceptance, the real 512 MiB boundary, source or model-output
sufficiency, WMO 462695 admission, full-world coverage, complete geometry, and
runtime acceptance are all **unproven**; the v43 extraction attempt failed at the
1 GiB source policy limit. The planned 2 GiB v44 cap has not been tested. Windows,
MMaps, format parity, reproducible distribution, and publication are outside this
contract's current evidence.

Only synthetic fixtures and structured metadata belong in this repository. No
client assets, keys, caches, extracted data, or client-derived geometry belong in
Git, CI, reports, or a release. This design does not enable network access.
