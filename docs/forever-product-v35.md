# Forever product v35: available geometry with unselected WMO declarations

Product v35 is a distinct Linux build-70450 profile,
`Build70450AvailableUnselectedWmoGeometryV1`, with component mask **17**
(`db2,vmaps`). It adds a separately reported `unselected_wmo` category after
`empty_wmo`; it does not weaken the intrinsic-empty definition. A present WMO
may contain real source geometry yet have no useful selected geometry. Such a
WMO is declared only as unselected, never as empty. V35 does not claim complete
geometry, full-world coverage, runtime acceptance or distribution. See the
[machine-readable contract](../contracts/forever-product-contract-v35.json).

## Exact acceptance target and evidence boundary

The real v35 run `v35-unselected-wmo-20261026T040320Z` was performed and
failed; it did not meet the acceptance target below:

- `missing_m2=[970793]`
- `missing_wmo=[]`
- `empty_wmo=[]`
- `unselected_wmo=[462695]`

No additional omission or accepted prefix is permitted. The failed run's
redacted acceptance report is
`reports/acceptance/v35-unselected-wmo-20261026T040320Z-acceptance.json`,
SHA-256
`dc088ff8159a276b2004586b86437bd86cb2c3c23abf37f3ec102c214b6a237a`.
`inspect` exited 0; `extract` exited 9 in component `vmaps` with
`collision_no_valid_collision`. `probe`, `verify` and `verify-links` did not run
(exit -1). The run took 12 seconds, attempted no network access, and left client
metadata and inputs unchanged. No accepted prefix, audit or geometry result
exists; the cause is in the retained diagnostic, and no `skip_reason` is
inferred. Software qualification separately passed 79 Rust suites (4,376
passed, 26 ignored, 0 failed) and 513 contract tests; it does not override the
real acceptance failure. The v35 target is **`failed`**, not `target-not-run`.
V34's failed evidence remains separate and no acceptance transfers between
profiles. V36 is not promoted by this result.

## Selection probe: diagnostic only

A separate redacted probe, `reports/acceptance/v35-selection-probe-20261026T041841Z-evidence.json`, is bound by report SHA-256
`9779537a4dccccfeac45f4b30eab90a4f03c7536b0ed0806d5685bdbbb013425`, canonical evidence SHA-256
`1d0d798af16e711e10c5ce1f6b16bfdc12f1cc3f33d801b0df01a5cce0ebcf3d`, and tool SHA-256
`df3d88bc4b8809f5f1a20321d49408ea2b41e0770d026691487b6fec547f1212`.
For WMO FDID **462695**, group **463228**, it records `skip_reason=null`, 368
source triangles / 1,104 indices / 591 vertices / one batch, no skipped source
geometry, and an unskipped source group. Its 368 MOPY flags are 38 (`0x26`)
60 times and 102 (`0x66`) 308 times. Both flag values fail the collision
predicate `flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)`, so the projection
has zero collision triangles, indices and vertices; no liquid is reported.

This explains the material-filtering branch of `collision_no_valid_collision`
and why the observed group cannot satisfy v35's `unselected_wmo` predicate,
which requires skipped source geometry. It establishes only that geometric
subpredicate for this WMO. It is not a RAW/doodad audit, does not prove every
condition for a future omission category, and does not change the v35 result:
`status=failed`, `accepted=false`, `audit=null`, `geometry=null`. No
`skip_reason` is inferred for the failed extraction.

## Future material-filtered WMO category: design only

Consider a separate `material_filtered_wmo` omission category in a possible
v36, with a new RCVGEO1 coverage layout v9 and no raised resource or package
limits. The proposed complete admission predicate is: WMO FileDataId origin;
`local_missing_m2=0`; `omitted_usable=false`; RAW output `None`; empty bindings
and selection references; selection sets present and nonempty; at least one
unskipped source group and no skipped groups; every source triangle filtered by
the established material rule; and no liquid groups, vertices or tiles. This
probe establishes only the group/material-filtering subpredicate, not the RAW,
doodad or other category-wide conditions. This remains design only:
`target-not-run`, not implemented or accepted. No v36 contract is created here.

V34 remains a failed prior result, not a v35 pass. Its retained report SHA-256
is `3a00385706ce85359bd5032fb47853d0a4fed9335436ddf3df1ffd81fdaf019c` and its
redacted diagnostic SHA-256 is
`8bbaa2e588292980ac2894d30e6c3bac446571439f233e051376d2ab43d3f68f`. The
reported WMO FDID **462695**, group **463228**, has real source geometry (368
triangles, 1,104 indices, 591 vertices and one batch), so v34 correctly did not
classify it as intrinsically empty. This evidence is preserved without
promotion or transfer of acceptance.

## Plan and versioned identities

The canonical esES plan is the existing, unchanged
[`all.tsv`](../tests/contracts/fixtures/forever-product-v35/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only **synthetic** budgets, exactly Map FDID **1349477**, mask **17**,
and `MapSelection::All`. It has no requested map or zero sentinel. Terrain and
navigation are absent; keys, network and decryption remain disabled.

V35 identities are:

- publication profile `Build70450AvailableUnselectedWmoGeometryV1`;
- plan profile `linux70450-available-unselected-wmo-vmap-product-v1`;
- plan coverage profile `linux70450-available-unselected-wmo-coverage-v8`;
- source/codec profile `linux70450-available-unselected-wmo-v1`;
- domain profile `available-unselected-wmo-v1`;
- completion profile `linux70450-available-unselected-wmo-collision-v1`;
- verification profile `linux70450-available-unselected-wmo-verification-v1`;
- RCVGEO1 v8 at `vmaps/available-unselected-wmo-geometry-coverage-v8.bin`.

RequestHeavyV1 retains its existing single `max_requests` increase to 16,000,000
relative to BoundedV1. V35 raises no other request, dependency, read, source,
raw, model, memory, spool, stage, disk or package ceiling.

## Four distinct declaration lists

The strict v8 codec serializes `missing_m2`, resource profile, the WMO policy,
`missing_wmo`, `empty_wmo`, then `unselected_wmo`, followed by artifact facts.
All four lists are canonical ordered, unique, nonzero and bounded by 4,096;
they are pairwise disjoint. Zero declarations still use v8. V34 stays on v7 and
cannot be relabeled as v35 despite the shared component mask.

- `missing_m2`: only guarded ROOT absence for a WMO-doodad M2. Direct ADT M2 is
  never admitted.
- `missing_wmo`: a WMO vacated after a local declared missing M2, with the
  existing MODR and empty-reference predicate.
- `empty_wmo`: only intrinsically empty source geometry before any omission,
  with no useful dependency, output or binding.
- `unselected_wmo`: the producer may declare a WMO only for a FileDataId origin
  with `local_missing_m2=0`, `omitted_usable=false`, `output=None`, empty
  bindings and empty selection references, and present, nonempty root selection
  sets. At least one group must have a `skip_reason` while its source triangle,
  index and vertex counts are all nonzero. No unskipped group may have all three
  source counts nonzero. Geometry in a skipped group is source geometry, not
  selected useful geometry, and does not make the WMO intrinsically empty.
  A non-skipped group whose collision triangles are all removed by material
  filtering is not `unselected_wmo`.

Useful output, selection/references or bindings prevent the `unselected_wmo`
classification, but do not automatically fail extraction: they may follow the
normal `Collision` or `DoodadOnly` result path. `RAW` output `None` alone is not
fatal when a normal `DoodadOnly` result exists. Failure remains fatal when
`RAW` output is absent, there is no normal `DoodadOnly` result, and no complete
admitted WMO omission predicate applies. Missing or empty selection sets and
logical-name origin cannot qualify as `unselected_wmo`. Required-source
acquisition or parser errors are always fatal and cannot be compensated by any
WMO omission declaration; malformed/unavailable source evidence is never made
admissible by a WMO declaration. V34's prior failure/geometry evidence is not
acceptance for v35; all v35 claims are bound to the v35 synthetic gate until
separately qualified.

No omitted ID emits a VMO, binding, valid name, spawn or placement, and omission
must not consume a placement ID. Present WMOs/doodads and their placements are
preserved. Placement attacks in synthetic tests update coverage digests and
rebuild assembly outputs before submitting packages to the production
`verify` and `verify-links` commands.

## Offline verification and synthetic qualification

Verification independently authenticates the synthetic PLAN-bound Map facts,
partial-section declarations, materialized domain, present-map artifacts,
placements, associations and model inventory. It checks all four canonical
lists and rejects package-visible references/artifacts for omitted identities.
The producer's source-absence, vacating, intrinsic-empty and unselected
classification decisions remain `producer_reported`; offline verification
cannot independently prove them.

The v35 CLI test uses only locally generated synthetic source files and the
existing v35 plan fixture. It exercises `inspect`, `extract`, `resume`,
`verify` and `verify-links`, malformed coverage and category-crossing attacks,
list boundaries, and present placement identity/bytes against a no-omission
control. It does not read real client assets or constitute real acceptance.

Full-world resource qualification, complete VMap geometry, GODI/global
GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore runtime
acceptance, TrinityCore parity, Windows, reproducible distribution and release
publication remain open. `full_extractor_complete` remains `false`.
