# Forever product v33: available geometry with vacated WMO omissions

Product v33 is a distinct Linux build-70450 profile,
`Build70450AvailableVacatedWmoGeometryV1`, with component mask **17**
(`db2,vmaps`). It adds one narrow, producer-declared omission for a WMO that
becomes empty after a locally declared WMO-doodad M2 ROOT absence. V33 preserves
v32 and does not claim complete geometry, full-world coverage, runtime
acceptance or distribution. The machine-readable contract is
[`forever-product-contract-v33.json`](../contracts/forever-product-contract-v33.json).

The real v33 run is **`failed`**: inspect passed, but extract exited 9 with
`collision_no_valid_collision`; the probe and both verifications did not run,
and no package or prefix was accepted. Its retained report is
`v33-vacated-wmo-20261025T184652Z-acceptance.json`, SHA-256
`7ca5b12e7f78899a384617f79d6350e19d320415022f741471c38951bf11e1d4`.
A separate bounded diagnostic confirmed that WMO FDID **462695** was already
empty before omission: `local_missing_m2=0`, `local_missing_names=0` and
`omitted_usable=0`. V33 correctly rejects that intrinsic case and does not
become an omission for it. The diagnostic evidence has SHA-256
`9e5c6e05ff039e652dda5cf77f713146375066b6378fdfeffac6d8cb02b7e9b7`.
The preceding real v32 failure remains separate, with report SHA-256
`6dd3118c6df2878b5b1b03c18c199fdc1c979e008bc266d6046ffd88c294eccf` and
redacted diagnostic SHA-256
`304ed0879e6a5553e5e2a1461d518c14bf986681ef330ea913b9d657522d0c3f`.

## Canonical plan and versioned identities

The canonical esES synthetic plan is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v33/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only synthetic budgets, exactly Map FDID **1349477**, component mask
**17**, all available maps and `MapSelection::All`. `map_domain=None`; there is
no `requested_map` or zero sentinel. Terrain and navigation are absent, and the
ordinary plan flags remain unchanged. The 1 MiB fixture budgets exercise plan
parsing only and are not certified real-extraction budgets.

The versioned identities are:

- publication profile `Build70450AvailableVacatedWmoGeometryV1`;
- plan profile `linux70450-available-vacated-wmo-vmap-product-v1`;
- PLAN coverage `linux70450-available-vacated-wmo-coverage-v6`;
- source/codec profile `linux70450-available-vacated-wmo-v1`;
- domain profile `available-vacated-wmo-v1`;
- completion `linux70450-available-vacated-wmo-collision-v1`;
- verification `linux70450-available-vacated-wmo-verification-v1`, scope
  `artifact_integrity_available_vacated_wmo_and_independently_derived_map_vmap_domain_coverage`;
- resource identity `linux-collision-request-heavy-resources-v1`;
- RCVGEO1 v6 at
  `vmaps/available-vacated-wmo-geometry-coverage-v6.bin`.

V33 retains the v32 `RequestHeavyV1` capacity profile. Only `max_requests` is
raised relative to `BoundedV1`, to 16,000,000; v33 changes no limits relative
to v32. Read, dependency-request, source/RAW-byte, model, memory, storage,
spool, inventory, staging and package ceilings remain unchanged. This does not
select `DiskWorldV1`, decrypt content, use keys or enable network access. The
CLI fixture does not claim to generate more than 4,096 requests; that boundary
belongs to the separate CASC tests.

RCVGEO1 v6 carries the RequestHeavyV1 resource identity after `missing_m2`,
then the WMO omission policy and canonical `missing_wmo` list before artifact
facts. The list is ordered, unique, nonzero, bounded and disjoint from
`missing_m2`. WDT and M2 omission rules remain separately versioned. V32 stays
RCVGEO1 v5; sharing component mask 17 does not permit crossing version, plan,
coverage, resource or omission identities.

## Narrow vacated-WMO predicate

A WMO FileDataId may be declared vacated only when its own conversion observed
at least one locally declared ROOT-absent WMO-doodad M2, the WMO produced no
geometry, no binding remains, MODR participated in selection, and selection
references are empty after the omission. An originally empty WMO, an unused
missing dependency, absent MODR participation, remaining geometry or binding,
logical-name origin, unrelated failure, or any incomplete predicate remains
fatal (`collision_no_valid_collision` where applicable).

A vacated WMO contributes no VMO, binding, valid name, spawn, placement or
model ID. A different present WMO and each present doodad remain intact; only
the declared absent child and the separately declared vacated WMO are omitted.
Direct ADT M2, WMO root/group, ADT, liquid, malformed, encrypted, corrupt,
central/encoding-absent, guard-change and resource-limit failures are not
omission classes and remain fatal.

## Offline verification and synthetic CLI coverage

Offline verification independently re-derives Map section/materialization facts,
the complete active map graph, present-map artifacts, associations and package
inventory. It checks the canonical WDT/M2/WMO declarations and ensures no
artifacts or references are published for omitted models or vacated WMOs.
However, raw ROOT absence, the original omitted doodad requests/spawns, the WMO
vacating decision and source-authentication flags remain
`producer_reported`; offline verification cannot independently prove them.

The synthetic CLI tests exercise inspect, extract, resume, verify and
verify-links; retain a present parent/doodad; omit a vacated WMO and absent M2;
check per-table Map partial declarations, WDT/map boundaries and poisonous
GODI/LiquidObject sources; compare fresh-scratch resume output byte-for-byte;
and reject malformed, crossed or false omission declarations. Negative cases
also cover an originally empty WMO and an absent dependency not used in MODR
selection. Plan/contract/CLI tests are synthetic software checks only. They do
not run against a real client.

## Qualification boundary

The real v33 run **failed** and is not accepted. The failed v32 evidence is not
transferred. Full-world resources, complete VMap geometry/component,
GODI/global GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore
runtime acceptance, C++/TrinityCore parity, Windows, reproducible distribution
and release publication remain open. `full_extractor_complete` remains `false`.

Synthetic software validation performed for this documentation update passed
13/13 v33 CLI integration tests, 9/9 linked v33 plan tests and 4/4 v33 contract
tests. These checks do not alter the real-acceptance status.

See also the [retained v32 failure](forever-product-v32.md) and the
[completion gates](forever-completion.md).
