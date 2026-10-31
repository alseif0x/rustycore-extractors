# Forever product v38: source-heavy material-filtered WMO declarations

Product v38 is a distinct Linux build-70450 profile,
`Build70450SourceHeavyMaterialFilteredWmoGeometryV1`, with component mask **17**
(`db2,vmaps`). It binds `SourceHeavyV1` resource limits and the strict RCVGEO1
v11 coverage layout. Relative to v37 `DependencyHeavyV1`, it raises only the
nominal `max_source_bytes` capacity to 512 MiB; cumulative request ceilings and
the 256 MiB `max_raw_bytes` cap remain unchanged. This profile does not establish
that the new source capacity suffices for real extraction, complete geometry,
full-world coverage, runtime acceptance or distribution. See the
[machine-readable contract](../contracts/forever-product-contract-v38.json).

## Acceptance state and prior evidence

The valid v38 real run `v38-source-heavy-wmo-20261026T163808Z` **failed**. Inspect
exited 0; extract exited 9 with `collision_policy_limit`; probe, verify and
verify-links did not run (exit -1). It took 25 seconds, attempted no network,
left input and client metadata unchanged, and produced no accepted prefix, audit
or geometry. The external report
`/home/joe/projects/dream/wow/rustycore-extractor-work/reports/acceptance/v38-source-heavy-wmo-20261026T163808Z-acceptance.json`
has SHA-256
`67a07b1b6fcbe87eb89eba504541252b26ada1295d899dd4ef5c9a3e93b4d54d`; its
`diagnostic.jsonl` has SHA-256
`caf57ba9f6687544fd16d237517dae87de9fd04cb24946d57dbb2c8a86acb2ee`.

The diagnostic is `kind=resource_limit`, `guards_triggered=false`, and identifies
`max_reads` at `reads=4096`. Its counters/snapshot are `request_calls=34486`,
`dependency_requests=11497`, `source_bytes=305392091`,
`raw_bytes=43744216`, `models=2424`, `resident_bytes=330762107`,
`external_bytes=270738115`, `external_source_bytes=11875616`,
`store_resident_bytes=306605630`, `memory_limit_bytes=1073741824`,
`tolerated_wdt=2`, and `tolerated_child=1`. For this run, the 512 MiB source cap,
256 MiB raw cap, memory, models and request ceilings were not the cause; the
observed limiting frontier was the 4,096-read cap. This identifies the cause of
this failed run only, not the sufficiency of a proposed larger read budget.

Software qualification associated with the valid run recorded 82 suites, 4,479
passed, 26 ignored and 535 contracts. Software SHA-256:
`e82599a49f1a393023eb782956946deb4af117bef33cd82cb5cf44a4728b0314`; binary
SHA-256: `1e49c7cba7c725fb0c666c5da7a19199213dec4bfec0f2e2351db19f825195c1`.

The intended lists are still **not observed output**:
`missing_m2=[970793]`, `missing_wmo=[]`, `empty_wmo=[]`,
`unselected_wmo=[]`, `material_filtered_wmo=[462695]`. WMO 462695 was not
admitted; `full_world_coverage` and runtime acceptance remain false.

### Earlier invalid setup attempt (not a product failure)

The earlier setup attempt `v38-source-heavy-wmo-20261026T150518Z` is separate
from the valid run above. Its report SHA-256 is
`d180720a5800c2531bb76a45dc43ee86c005dad97749d95dd8e627e769f4df31`; it used
an old incorrect binary (`f9ebb...`), `inspect` exited 2 with
`product_plan_version`, and extract did not run (exit -1). This is neither a
v38 product failure nor evidence about resource capacity.

The preceding v37 run `v37-dependency-heavy-wmo-20261026T120824Z` remains prior
evidence, not v38 acceptance. Its report SHA-256 is
`e86bf2370e84897b74b6df7ac3c6fb42fa508fccd67d763b6c72181804a1e552`; the
redacted diagnostic SHA-256 is
`a5ae7cf86a3613359bdf67c956377b26d5b0367a1707397e032cb2b4600c55a5`. It
confirmed a source-byte limit for that specific v37 failure and does not prove
admission of WMO 462695 or sufficiency of SourceHeavyV1.

## Plan and versioned identity

The canonical esES plan is the v38 synthetic
[`all.tsv`](../tests/contracts/fixtures/forever-product-v38/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
**synthetic-only** downward-only 1 MiB budgets, exactly Map FDID **1349477**,
mask **17**, and `MapSelection::All`. The fixture is distinct from the real
resource profile: its small caps remain in force for synthetic tests and are not
a statement of real target capacity. Terrain and navigation are absent; keys,
network and decryption remain disabled.

V38 identities are:

- publication profile `Build70450SourceHeavyMaterialFilteredWmoGeometryV1`;
- plan profile `linux70450-source-heavy-material-filtered-wmo-vmap-product-v1`;
- coverage profile `linux70450-source-heavy-material-filtered-wmo-coverage-v11`;
- source/codec profile `linux70450-source-heavy-material-filtered-wmo-v1`;
- domain profile `source-heavy-material-filtered-wmo-v1`;
- resource profile `linux-collision-source-heavy-resources-v1`;
- completion profile
  `linux70450-available-source-heavy-material-filtered-wmo-collision-v1`;
- verification profile
  `linux70450-source-heavy-material-filtered-wmo-verification-v1`;
- verification scope
  `artifact_integrity_source_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage`;
- RCVGEO1 v11 at
  `vmaps/available-source-heavy-material-filtered-wmo-geometry-coverage-v11.bin`.

`SourceHeavyV1` keeps `max_requests=16000000` and
`max_dependency_requests=16000000` in collector and startup collision limits.
It raises `max_source_bytes` to 512 MiB while retaining `max_raw_bytes` at 256
MiB; relative to v37, this is a source-only capacity change. Reads, per-file,
models, spool, inventory, stage, package, list, memory and remaining geometry
safety limits match v37 `DependencyHeavyV1`; notably, `max_reads` remains 4,096
and was the limiting resource in the valid v38 real run. `DiskWorldV1` is not implied.
The `SourceHeavyV1` profile's nominal ceilings do not override the plan's
synthetic downward caps: `geometry_limits` applies `budget.raw_bytes` to both
`max_source_bytes` and `max_raw_bytes`. Thus the unchanged fixture still caps
both at 1 MiB; it has not been inflated to either 256 or 512 MiB and does not
exercise or certify those real-profile ceilings.

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

RCVGEO1 v11 strictly binds the SourceHeavyV1 coverage, source/codec, domain
and resource profiles. It serializes the five ordered lists
`missing_m2`, `missing_wmo`, `empty_wmo`, `unselected_wmo`, and
`material_filtered_wmo`; each accepts at most 4,096 nonzero, strictly ordered,
unique IDs, and all five lists are pairwise disjoint. The material-filtered list
follows `unselected_wmo`. Zero omissions still use layout v11; v10 and older
wrappers remain strict and cannot be used as a v38 layout.

No omitted WMO ID may produce a VMO, binding, valid name, spawn or placement, or
consume a placement ID. Present WMO/doodad outputs and placement bytes remain
preserved. Synthetic tests exercise `inspect`, `extract`, `resume`, `verify` and
`verify-links`, exact material-filtered-only and five-way mixed categories,
normal-result controls, source acquisition/parser failures, malformed and
cross-profile coverage, omitted artifacts/placements, WDT and policy/profile
boundaries, and each list at 4,096/4,097 entries. The CLI carries forward v37's
Map, WDT, GODI/GameObject, LiquidObject, JSON, predicates/acquisition,
Collision/DoodadOnly, lifecycle, synchronized attack, list-overlap, truncation
and suffix oracles; it adds v38 SourceHeavyV1 resource identity and v37/v38
cross-version rejection plus rejection of v10 and older layouts. It does not
duplicate the existing 4,097 MODI case. Tests use
locally generated synthetic source only; they do not use real client data or
constitute real acceptance.

Verification independently derives package integrity, Map/VMap domain facts,
present artifacts and associations, model inventory and coverage binding. WMO
absence and material-filtered classification remain producer-reported. The
verifier rejects package-visible artifacts for omitted identities, but cannot
independently prove source-side ROOT absence, RAW/doodad facts or conversion-time
classification.

## Follow-up design only

A proposed v39 `ReadHeavyV1` would raise only `max_reads`, from 4,096 to
1,000,000. It would retain `max_requests=16000000`,
`max_dependency_requests=16000000`, `max_source_bytes=536870912`,
`max_raw_bytes=268435456` and all other SourceHeavyV1 limits. The proposed
coverage layout is v12. This remains **design-only; target-not-run**: it does
not establish read-cap sufficiency or WMO 462695 admission, and no v39
implementation or acceptance is claimed.

Full-world resource qualification, complete VMap geometry, GODI/global
GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore runtime
acceptance, TrinityCore parity, Windows, reproducible distribution and release
publication remain open. `full_world_coverage`, `runtime_claim` and
`full_extractor_complete` remain `false`.

## Synthetic and structural validation status

The v38 structural contract suite previously passed **8/8** tests and the
synthetic CLI suite **15/15** tests; targeted Clippy and formatting checks also
passed for that checkpoint. The current real run is a separate failed local
acceptance, not target-not-run. Its failure at `max_reads` does not establish
sufficiency of the proposed ReadHeavyV1 ceiling, admission of WMO 462695,
full-world coverage or runtime acceptance.
