# Forever product v37: dependency-heavy material-filtered WMO declarations

Product v37 is a distinct Linux build-70450 profile,
`Build70450DependencyHeavyMaterialFilteredWmoGeometryV1`, with component mask
**17** (`db2,vmaps`). It binds `DependencyHeavyV1` resource limits and the strict
RCVGEO1 v10 coverage layout. The profile only changes the cumulative dependency
request ceiling relative to `RequestHeavyV1`; it does not establish complete
geometry, full-world coverage, runtime acceptance or distribution. See the
[machine-readable contract](../contracts/forever-product-contract-v37.json).

## Acceptance state and prior evidence

The v37 real run `v37-dependency-heavy-wmo-20261026T120824Z` **failed**.
Inspect exited 0; extract exited 9 with `local_store_policy_limit`; probe and
verify did not run (exit status -1). The run took 25 seconds, attempted no
network access, left the client and input unchanged, and produced no accepted
prefix, audit or geometry. Its report is
`reports/acceptance/v37-dependency-heavy-wmo-20261026T120824Z-acceptance.json`,
SHA-256 `e86bf2370e84897b74b6df7ac3c6fb42fa508fccd67d763b6c72181804a1e552`.

The redacted diagnostic recorded stage `models`, resource `wmo_group`, FileDataId
7964744, ordinal 7029, result `typed Limit`, `guards_triggered=false`,
`tolerated_wdt=2`, and `tolerated_child=1`. Follow-up evidence
`reports/acceptance/v37-source-limit-diagnostic-20261026T130445Z-evidence.json`
(SHA-256 `a5ae7cf86a3613359bdf67c956377b26d5b0367a1707397e032cb2b4600c55a5`)
confirms `max_source_bytes` as the cause for this failure. Instrumentation in
`driver/disk/compatible.rs` emits cause and snapshot together on one line, without
duplicate diagnostic lines: `source_bytes=268259239`,
`max_source_bytes=268435456`, `source_bytes_remaining=176217`, declared group bytes
`407338`, excess `231121`, `resident_bytes=341795444`, `store_bytes=306605630`,
and `memory_limit_bytes=1073741824`; the same snapshot records `request_calls=32842`,
`reads=3878`, `dependency_requests=11286`, and `models=2362`. The isolated
407338-byte group parsed correctly in the prior report (SHA-256
`74058259c6aeba9b72594a37ad39449569800246bb974ceca2537d2a4a9ed838`). This rules
out memory exhaustion and a parser failure for this failure only; it does not
prove admission of WMO 462695 as `material_filtered_wmo`.

Software qualification associated with this run recorded 81 suites, 4,444
passed, 26 ignored and 525 contracts. Binary SHA-256:
`de894778f770fbb091331677dcb8d5592a5f5882091cac5e4bcd20d40e5881b0`; source
SHA-256: `06a58e70b99225dfb80bfa840432709705b27941799f8f9442730f108bcc4bc7`;
software SHA-256:
`44ddc1ecc3847cd591673b26254cb1cbbd7edfadd3f77ccb106f30852ff7c4cc`.

The intended list shape remains only a target, not observed output:
`missing_m2=[970793]`, `missing_wmo=[]`, `empty_wmo=[]`,
`unselected_wmo=[]`, `material_filtered_wmo=[462695]`. The failed run does not
establish that any such output was accepted. Acceptance, full-world and runtime
claims remain false.

The synthetic `MaterialFilteredWmoOnly` CLI fixture checks the intended list
shape. Its separate five-way mixed fixture uses `unselected_wmo=[462695]` and the
distinct synthetic `material_filtered_wmo=[462701]`; those IDs must remain
separate and all five lists pairwise disjoint. Synthetic evidence is not real
client evidence.

## Plan and versioned identity

The canonical esES plan is the existing, unchanged
[`all.tsv`](../tests/contracts/fixtures/forever-product-v37/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only **synthetic** 1 MiB budgets, exactly Map FDID **1349477**, mask
**17**, and `MapSelection::All`. It has no requested map or zero sentinel.
Terrain and navigation are absent; keys, network and decryption remain disabled.

V37 identities are:

- publication profile `Build70450DependencyHeavyMaterialFilteredWmoGeometryV1`;
- plan profile `linux70450-dependency-heavy-material-filtered-wmo-vmap-product-v1`;
- coverage profile `linux70450-dependency-heavy-material-filtered-wmo-coverage-v10`;
- source/codec profile `linux70450-dependency-heavy-material-filtered-wmo-v1`;
- domain profile `dependency-heavy-material-filtered-wmo-v1`;
- resource profile `linux-collision-dependency-heavy-resources-v1`;
- completion profile
  `linux70450-available-dependency-heavy-material-filtered-wmo-collision-v1`;
- verification profile
  `linux70450-dependency-heavy-material-filtered-wmo-verification-v1`;
- verification scope
  `artifact_integrity_dependency_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- RCVGEO1 v10 at
  `vmaps/available-dependency-heavy-material-filtered-wmo-geometry-coverage-v10.bin`.

`DependencyHeavyV1` sets both `max_requests` and
`max_dependency_requests` to 16,000,000 in collector and startup collision
limits. Relative to `RequestHeavyV1`, it raises only
`max_dependency_requests`; relative to `BoundedV1`, it raises those two request
ceilings. Spool, list, inventory, stage, model, package, memory, read, source,
raw, per-file, parser and geometry-safety limits stay at their corresponding
Bounded values. `DiskWorldV1` is not implied. The plan's seven 1 MiB budgets are
synthetic fixture values, not certified real extraction budgets.

## Material-filtered WMO admission

A `material_filtered_wmo` declaration requires all of the following:

1. The WMO was requested by FileDataId, not a logical name.
2. `local_missing_m2=0` and `omitted_usable=false`.
3. RAW output is `None`; bindings and selection references are empty.
4. Root selection sets are present and nonempty.
5. At least one source group is unskipped and has source geometry; no source
group is skipped.
6. Every source triangle matches the REJECT predicate, the logical complement
of the KEEP predicate:
   - KEEP: `flags&0x08!=0 || (flags&0x20!=0 && flags&0x04==0)`.
   - REJECT: `flags&0x08==0 && (flags&0x20==0 || flags&0x04!=0)`.
   - Rejected examples: `0x26`, `0x66`; kept examples: `0x20`, `0x08`.
7. No liquid groups, vertices or tiles exist, and conversion completes without
residual useful geometry or placement bindings.

The classification is producer-reported; offline verification cannot replay the
source conversion or independently prove the original omission decision. A WMO
whose groups combine skipped source geometry with material-filtered geometry is
not admitted. Nor are useful selection/references/bindings, local missing M2,
logical-name origin, absent or empty selection sets, liquid, acquisition/parser
errors, conversion errors, or resource-limit failures. Such failures remain
fatal rather than becoming omissions. Ordinary `Collision` and `DoodadOnly`
results remain valid when their normal output is present; material filtering
alone cannot reclassify useful results as omitted.

The `missing_m2`, `missing_wmo`, `empty_wmo` and `unselected_wmo`
classifications retain their distinct predicates. In particular, skipped group
geometry remains separate from unskipped geometry removed by the material filter,
and material-filtered source geometry is not `empty_wmo`.

## Coverage, package and verification boundaries

RCVGEO1 v10 strictly binds the dependency-heavy coverage, source/codec, domain
and resource profiles. It serializes the five ordered lists
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`; each accepts at most 4,096 nonzero, strictly ordered,
unique IDs, and all five lists are pairwise disjoint. The material-filtered list
follows `unselected_wmo`. Zero omissions still use layout v10; v9 and older
wrappers remain strict and cannot be used as a v37 layout.

No omitted WMO ID may produce a VMO, binding, valid name, spawn or placement, or
consume a placement ID. Present WMO/doodad outputs and placement bytes remain
preserved. Synthetic tests exercise `inspect`, `extract`, `resume`, `verify` and
`verify-links`, exact material-filtered-only and five-way mixed categories,
normal-result controls, source acquisition/parser failures, malformed and
cross-profile coverage, omitted artifacts/placements, WDT and policy/profile
boundaries, and each list at 4,096/4,097 entries. The CLI carries forward v36's
Map, WDT, GODI/GameObject, LiquidObject, JSON, predicates/acquisition,
Collision/DoodadOnly, lifecycle, synchronized attack, list-overlap, truncation
and suffix oracles; it adds v37 resource identity and v36/v37 cross-version
rejection. It does not duplicate the existing 4,097 MODI case. Tests use
locally generated synthetic source only; they do not use real client data or
constitute real acceptance.

Verification independently derives package integrity, Map/VMap domain facts,
present artifacts and associations, model inventory and coverage binding. WMO
absence and material-filtered classification remain producer-reported. The
verifier rejects package-visible artifacts for omitted identities, but cannot
independently prove source-side ROOT absence, RAW/doodad facts or conversion-time
classification.

Full-world resource qualification, complete VMap geometry, GODI/global
GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore runtime
acceptance, TrinityCore parity, Windows, reproducible distribution and release
publication remain open. `full_world_coverage`, `runtime_claim` and
`full_extractor_complete` remain `false`.

## Synthetic validation status

The v37 structural contract suite passes **6/6** tests, the synthetic CLI suite
passes **15/15** tests, targeted Clippy passes with `-D warnings`, and
`tools/checks fmt-check` passes. These results validate only synthetic and
structural gates; they do not change the failed status of the v37 real run or
prove admission of WMO 462695.
