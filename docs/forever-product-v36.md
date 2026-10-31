# Forever product v36: material-filtered WMO declarations

Product v36 is a distinct Linux build-70450 profile,
`Build70450MaterialFilteredWmoGeometryV1`, with component mask **17**
(`db2,vmaps`). It adds `material_filtered_wmo` after `unselected_wmo` in the
RCVGEO1 v9 coverage layout. This category is only for a FileDataId-origin WMO
whose source geometry is present in unskipped groups but every source triangle is
removed by the established MOPY collision-material filter. It is not an
intrinsically empty WMO and does not establish complete geometry, full-world
coverage, runtime acceptance or distribution. See the
[machine-readable contract](../contracts/forever-product-contract-v36.json).

## Acceptance state and prior evidence

The v36 real run `v36-material-filtered-wmo-20261026T085714Z` **failed**. Inspect
exited 0; extract exited 9 with `collision_policy_limit`; probe and verify did
not run (exit status -1). The run took 23 seconds, attempted no network access,
and left the client and input unchanged. It produced no accepted prefix, audit
or geometry. The retained report is
`reports/acceptance/v36-material-filtered-wmo-20261026T085714Z-acceptance.json`,
SHA-256 `dfcca8f3c29b655d5d3a0a2e9794cad7d2484bbf2edb25e29bbe9eefff039e03`.
The failure does not prove that WMO 462695 was admitted as `material_filtered_wmo`.

Diagnostic counters were `request_calls=19243`, `reads=3362`,
`dependency_requests=4096`, `models=2118`, `source_bytes=221090913`,
`raw_bytes=31135205`, `external_bytes=269362435`,
`external_source_bytes=11872896`, `resident_bytes=314993930`,
`store_bytes=306605630` and `memory_limit_bytes=1073741824`.

The real target remains the intended list shape, not observed output:

- `missing_m2=[970793]`
- `missing_wmo=[]`
- `empty_wmo=[]`
- `unselected_wmo=[]`
- `material_filtered_wmo=[462695]`

The synthetic `MaterialFilteredWmoOnly` CLI exercises that list shape. Its
five-way mixed fixture deliberately uses `unselected_wmo=[462695]` and the
distinct synthetic `material_filtered_wmo=[462701]`, with no cross-list overlap.
That synthetic evidence does not change the real run's failed status or establish
admission of the target WMO.

The prior v35 run `v35-unselected-wmo-20261026T040320Z` failed; it remains a
v35 failure and is not transferred to v36. Its redacted report SHA-256 is
`dc088ff8159a276b2004586b86437bd86cb2c3c23abf37f3ec102c214b6a237a`. The
separate diagnostic probe report SHA-256 is
`9779537a4dccccfeac45f4b30eab90a4f03c7536b0ed0806d5685bdbbb013425`, canonical
evidence SHA-256
`1d0d798af16e711e10c5ce1f6b16bfdc12f1cc3f33d801b0df01a5cce0ebcf3d`, and tool
SHA-256 `df3d88bc4b8809f5f1a20321d49408ea2b41e0770d026691487b6fec547f1212`.

For WMO FDID **462695**, group **463228**, that probe reported `skip_reason=null`,
368 source triangles, 1,104 indices and 591 vertices. Its MOPY flags were 38
(`0x26`) 60 times and 102 (`0x66`) 308 times. The established material rule is
the KEEP predicate
`flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)`; its logical complement is
the REJECT predicate
`flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)`. Thus `0x26` and `0x66`
are rejected, while `0x20` and `0x08` are kept. The probe's triangles all match
REJECT, producing zero projected collision triangles, indices and vertices,
with no liquid. This establishes only a source-group/material-filtering
subpredicate. It is not a complete RAW/doodad audit or proof of the v36 omission
predicate, and does not change the v35 failed status or establish v36 acceptance.

## Plan and versioned identity

The canonical esES plan is the existing, unchanged
[`all.tsv`](../tests/contracts/fixtures/forever-product-v36/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only **synthetic** budgets, exactly Map FDID **1349477**, mask **17**,
and `MapSelection::All`. It has no requested map or zero sentinel. Terrain and
navigation are absent; keys, network and decryption remain disabled.

V36 identities are:

- publication profile `Build70450MaterialFilteredWmoGeometryV1`;
- plan profile `linux70450-material-filtered-wmo-vmap-product-v1`;
- coverage profile `linux70450-material-filtered-wmo-coverage-v9`;
- source/codec profile `linux70450-material-filtered-wmo-v1`;
- domain profile `material-filtered-wmo-v1`;
- completion profile `linux70450-available-material-filtered-wmo-collision-v1`;
- verification profile `linux70450-material-filtered-wmo-verification-v1`;
- verification scope
  `artifact_integrity_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- RCVGEO1 v9 at
  `vmaps/available-material-filtered-wmo-geometry-coverage-v9.bin`.

RequestHeavyV1 retains its existing single `max_requests` increase to
16,000,000 relative to BoundedV1. V36 raises no other request, dependency, read,
source, raw, model, memory, spool, stage, disk or package ceiling; no limits
change from v35. The seven 1 MiB plan budgets remain synthetic fixture values,
not certified real extraction budgets.

## Future v37 design (not implemented)

A future DependencyHeavyV1 profile is proposed to address dependency request
capacity without changing v36: raise only `max_dependency_requests` to
16,000,000, while preserving RequestHeavyV1's `max_requests=16,000,000` and all
other v36 limits. Its planned coverage format is RCVGEO1 v10. Its intended target
lists remain `missing_m2=[970793]`, `missing_wmo=[]`, `empty_wmo=[]`,
`unselected_wmo=[]`, `material_filtered_wmo=[462695]`.

This is design documentation only: v37 has no implementation or acceptance in
this change, and its real target remains `target-not-run`. The v36 failed run
above does not demonstrate admission of WMO 462695 and cannot be transferred as
v37 evidence. Any implementation must have its own versioned contract and
synthetic/real acceptance gates.

## Material-filtered WMO admission

A `material_filtered_wmo` declaration requires all of the following:

1. The WMO was requested by FileDataId, not a logical name.
2. `local_missing_m2=0` and `omitted_usable=false`.
3. RAW output is `None`; bindings and selection references are empty.
4. Root selection sets are present and nonempty.
5. At least one source group is unskipped and has source geometry; no source
group is skipped.
6. Every source triangle matches the REJECT predicate, the logical complement of
   the KEEP predicate:
   - KEEP: `flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)`.
   - REJECT: `flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)`.
   - Rejected examples: `0x26`, `0x66`; kept examples: `0x20`, `0x08`.
7. No liquid groups, vertices or tiles exist, and conversion completes without
   residual useful geometry or placement bindings.

The classification is producer-reported; offline verification cannot replay the
source conversion or independently prove the original omission decision. A WMO
whose groups combine skipped source geometry with material-filtered geometry
is not admitted. Nor are useful selection/references/bindings, local missing M2,
logical-name origin, absent or empty selection sets, liquid, acquisition/parser
errors, conversion errors, or resource-limit failures. Such failures remain
fatal rather than becoming omissions. Ordinary `Collision` and `DoodadOnly`
results remain valid when their normal output is present; material filtering
alone cannot reclassify useful results as omitted.

The prior `missing_m2`, `missing_wmo`, `empty_wmo` and `unselected_wmo`
classifications retain their distinct v35 predicates. In particular, skipped
group geometry remains separate from unskipped geometry removed by the material
filter, and material-filtered source geometry is not `empty_wmo`.

## Coverage, package and verification boundaries

RCVGEO1 v9 serializes the five ordered, canonical lists
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`; each accepts at most 4,096 nonzero, strictly ordered,
unique IDs, and all five lists are pairwise disjoint. The new list follows
`unselected_wmo`. Zero omissions still use layout v9. Older profile wrappers
and layouts remain strict and cannot accept a v36 package through a shared
component mask.

No omitted WMO ID may produce a VMO, binding, valid name, spawn or placement, or
consume a placement ID. Present WMO/doodad outputs and placement bytes remain
preserved. Synthetic tests exercise `inspect`, `extract`, `resume`, `verify` and
`verify-links`, exact material-filtered-only and five-way mixed categories,
normal-result controls, source acquisition/parser failures, malformed and
cross-profile coverage, omitted artifacts/placements, WDT and policy/profile
boundaries, and each list at 4,096/4,097 entries. Tests use locally generated
synthetic source only; they do not use real client data or constitute real
acceptance.

Verification independently derives package integrity, Map/VMap domain facts,
present artifacts and associations, model inventory and coverage binding. WMO
absence and material-filtered classification remain producer-reported. The
verifier rejects package-visible artifacts for omitted identities, but cannot
independently prove source-side ROOT absence, RAW/doodad facts or conversion-time
classification.

Full-world resource qualification, complete VMap geometry, GODI/global
GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore runtime
acceptance, TrinityCore parity, Windows, reproducible distribution and release
publication remain open. `full_extractor_complete` remains `false`.
