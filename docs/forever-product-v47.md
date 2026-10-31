# Forever product v47: GameObjectExpanded material-filtered WMO geometry

V47 is a separately versioned Linux build-70450 DB2 and available-VMap geometry
product (component mask **17**). It extends the v46 available-map placement and
assembly product with `GameObjectDisplayInfo`-driven game-object models and the
native `vmaps/GameObjectModels.dtree` output. RCVGEO1 coverage is version 20 with
separate v47 identities. Synthetic PLAN, lifecycle, package-verification,
resume, determinism, attack, and boundary tests pass; this does not constitute
real-target acceptance. The real target was attempted and failed without an
accepted prefix; overall extractor completion remains false.

## Product identity

V47 binds these identities:

- publication: `Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1`;
- resource profile: `GameObjectExpandedV1`,
  `linux-collision-gameobject-expanded-resources-v1`;
- PLAN: `linux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1`;
- coverage: `linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20`;
- codec/source: `linux70450-gameobject-expanded-material-filtered-wmo-v1`;
- domain: `gameobject-expanded-material-filtered-wmo-v1`;
- completion: `linux70450-available-gameobject-expanded-material-filtered-wmo-collision-v1`;
- verification: `linux70450-gameobject-expanded-material-filtered-wmo-verification-v1`;
- verification scope:
  `artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage`;
- coverage artifact:
  `vmaps/available-gameobject-expanded-material-filtered-wmo-geometry-coverage-v20.bin`;
- GODI scope: `available-fdid-direct-v1`;
- model scope: `placement-and-gameobject-v1`;
- Dtree: `vmaps/GameObjectModels.dtree`, encoded as `VMAP_4.E`.

The canonical synthetic PLAN is
`tests/contracts/fixtures/forever-product-v47/all.tsv`. It has 31 lines,
component mask 17, mandatory `partial_encrypted_sections\tdeclared-skip`, and
ordered DB2 entries `GameObjectDisplayInfo=1266277` followed by `Map=1349477`.
The fixture contains no client data and does not represent a real extraction.

## Intended verification and boundaries

Offline verification is intended to authenticate the sealed Map and
GameObjectDisplayInfo physical rows, order, display IDs and FDIDs; check Dtree
entries and their `FILE<FDID>.xxx` VMO references; reconstruct the temporary
unbounded tree from Dtree bytes for its SHA-256 check; and retain v46 checks for
available-map VMap domain coverage, placements, assembly, model inventory,
associations, omission lists and resource limits. Classification of GODI rows
as `Texture`, `M2NoCollision`, `WmoDoodadOnly` or collision is producer-reported;
verification checks those dispositions for consistency with the sealed source
rows but does not independently classify the model bytes. The temporary source
tree is not published. Direct rows are processed in physical order; copy rows do
not trigger re-execution.

The intended verification boundary distinguishes independently derived Map and
GODI physical source rows, placement/assembly and package evidence from
producer-reported dispositions, Dtree bounds and transformed raw geometry. It
does not claim parity with a complete native extractor. LiquidObject, complete
full-world coverage, complete VMap geometry, MMaps, runtime acceptance, Windows,
distribution and release authorization remain out of scope.

## Validation status

Current status is `attempted-failed-unaccepted`: `real_target_attempted=true`,
`real_extraction_accepted=false`, and `full_extractor_complete=false`. Run
`v47-gameobject-expanded-wmo-20261028T032635Z` failed at extract (exit 9) with
`file_unavailable` in `vmaps`; its redacted source diagnostic identifies phase
`db2`, FDID 2198569. Inspect exited 0; probe, verify and verify-links did not run
(exit -1), and no accepted prefix was produced. Report SHA-256:
`d8d1f10bc62d38363d4e277c83c06b4de037d3d08111ff22b1fe50c093b6fc95`.
Synthetic validation passed: the v47 CLI suite is 7/7, the v47 PLAN admission
tests are 4/4, the complete forever-extractor library is 824/824, the v46 CLI
regression is 16/16, and the Python v47 contract tests are 3/3. The v47 CLI exercises
synthetic `inspect`, `extract`, `resume`, `verify`, and `verify-links`, byte-stable
resume/determinism, both v46/v47 package-crossing directions, a missing GODI VMO,
Dtree and referenced VMO parsing, reconstructed temporary-tree and Dtree hashes,
and an independent placement oracle. It also rejects resealed version/profile
identity mutations and all ten omission-list overlaps with both verifiers, accepts
4096 entries and rejects 4097 in each of the five lists with both verifiers, and
checks the ten emitted JSON resource limits for exact numeric values, type, and
presence. The fixture contains synthetic data only. These software tests are not
real extraction, runtime acceptance, complete VMap/full-world coverage, or
complete extractor qualification. Do not transfer v46's real acceptance to v47.

A separate software-only qualification snapshot records 91 workspace suites
(4,712 passed, 0 failed, 26 ignored), forever-extractor library 824/824, v47 CLI
7/7, PLAN admission 4/4, CASC available 43/43, compatible_vmaps replay 81/81,
and repository contracts 582/582. Formatting, Clippy, diff-check, Windows GNU,
and release build passed. The recorded release executable is
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v47-qualified-software-20261028T025339Z/forever-extractor`
(SHA-256 `fe8b4245ae05e647a36933f0f0b4cf2ff2334a36e0971441c186d5faaa41d415`).
Its reported offline inspect passed the ten numeric-limit checks and v47 resource,
source, coverage, and scope identity checks, with
`full_extractor_complete=false`. These checks record the current identities and
limits; this qualification update did not change them. The executable and external
logs were not independently reopened for this documentation update; hashes and
per-gate evidence are recorded in the machine-readable contract. The qualification
predates the later failed real attempt; its historical `target-not-run` fields do
not describe current v47 status. Scoped real-target acceptance and full-world
acceptance remain false.

Only synthetic fixtures and structured metadata belong in this repository. No
client assets, keys, caches, extracted data, or client-derived geometry belong
in Git, CI, reports, or releases. This contract does not enable network access.
See the [machine-readable v47 contract](../contracts/forever-product-contract-v47.json)
for the machine-readable scope and identities.
