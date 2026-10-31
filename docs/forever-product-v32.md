# Forever product v32: request-capacity geometry

Product v32 is a distinct Linux build-70450 profile,
`Build70450AvailableModelRootAbsentRequestCapacityGeometryV1`, with component
mask **17** (`db2,vmaps`). It preserves v31's available-map geometry and narrowly
declared WDT and WMO-doodad M2 ROOT omissions, adding only the separately
versioned resident `RequestHeavyV1` request ceiling. It does not retroactively
change v31 or claim complete geometry, full-world coverage, runtime acceptance
or distribution. The machine-readable contract is
[`forever-product-contract-v32.json`](../contracts/forever-product-contract-v32.json).

The real v31 run remains **failed** at `request_calls=4096` with
`collision_policy_limit` (inspect exit 0, extract exit 9; probe and both
verifications not run; no package or prefix accepted). Its retained report has
SHA-256 `6dd0daaaf3f601b91859789266e851a38f8367b1d37b47aa3fd1778b30e79444`.
This evidence motivates the one v32 capacity change; it is not v32 evidence.
The real v32 run is **`failed`**: RequestHeavyV1 crossed the former ceiling, but
extract exited 9 with `collision_no_valid_collision` after 79 seconds. Inspect
passed; the probe and both verifications did not run, and no package or prefix
was accepted. The retained report is
`v32-request-capacity-20261025T162439Z-acceptance.json`, SHA-256
`6dd3118c6df2878b5b1b03c18c199fdc1c979e008bc266d6046ffd88c294eccf`.
A separate bounded GDB diagnostic identified WMO FDID **462695** with zero
geometry, zero bindings and zero selected references after its declared M2
omission, at `request_calls=6217`. The redacted diagnostic evidence has SHA-256
`304ed0879e6a5553e5e2a1461d518c14bf986681ef330ea913b9d657522d0c3f`.

## Canonical plan and versioned identities

The canonical esES synthetic plan is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v32/all.tsv). It selects
`wowdbdefs-70450-v1`, disabled decryption, `declared-skip` immediately between
`decryption` and `collision-storage`, leased disk with seven downward-only
budgets, exactly Map FDID **1349477**, mask **17**, all available maps and
`MapSelection::All`. `map_domain=None`; there is no `requested_map` or zero
sentinel. Terrain/navigation are absent, packing is enabled, and the ordinary
flags/slopes are unchanged. Its 1 MiB budget values are synthetic plan-parser
fixtures only, not certified real budgets.

Versioned identities:

- publication profile `Build70450AvailableModelRootAbsentRequestCapacityGeometryV1`;
- plan profile `linux70450-available-map-model-root-omissions-request-capacity-vmap-product-v1`;
- PLAN coverage `linux70450-available-map-model-root-omissions-request-capacity-coverage-v5`;
- source/codec and domain closure `linux70450-available-map-model-root-omissions-request-capacity-v1` / `available-map-model-root-omissions-request-capacity-v1`;
- completion `linux70450-available-map-model-root-omissions-request-capacity-collision-v1`;
- verification `linux70450-available-map-model-root-omissions-request-capacity-verification-v1`, scope `artifact_integrity_available_map_model_root_omissions_request_capacity_and_independently_derived_map_vmap_domain_coverage`;
- resource identity `linux-collision-request-heavy-resources-v1`;
- RCVGEO1 v5 at `vmaps/available-request-capacity-geometry-coverage-v5.bin`.

RCVGEO1 v5 adds the `RequestHeavyV1` resource identity after `missing_m2` and
before artifact facts. `missing_m2` remains canonical, ordered, unique and
nonzero. WDT and M2 omission policies, all-map selection, Map graph rules,
artifact inventory and producer-reported ROOT absence are otherwise v31's
strict boundaries. Zero omissions still use v5. V31 remains v4/BoundedV1;
v22/v28 remain v1, v29 v2 and v30 v3. Sharing mask 17 does not permit crossing
versions, profiles, resource identity, omission policies, context or plan.

## The only increased ceiling

`RequestHeavyV1` changes only `collector.max_requests` and
`startup_limits().collision.max_requests`, to **16,000,000**. This provides
headroom beyond v31's 4,096 top-level request-call ceiling; it is a maximum, not
a promise of request count, coverage or run duration. In particular it does not
raise source reads, dependency requests, source bytes, RAW bytes, model count,
per-file/parser limits, package/storage budgets, or memory. It does not opt into
`DiskWorldV1`, spool, inventory, stage, package or broader list profiles.

Requests—including cache-hit calls—still retain request handles, so unchanged
logical memory guards remain active. The existing 4,096 model/missing-M2/map
limits, byte ceilings, coverage/package ceilings and per-plan downward-only
budgets all remain in force. A new reduced request ceiling fails on N+1; memory
or any other limit remains fatal and no partial prefix is accepted. CASC's
focused unit coverage separately exercises >4,096 repeated requests against one
source/model, the reduced N+1 failure and memory enforcement; that is a collector
unit test, not a claim that the CLI fixture itself induces >4,096 calls.

## Synthetic CLI and verification scope

The v32 CLI fixture preserves v31's synthetic cases: a present WMO parent with
one present doodad M2 and FDID **970793** with no ROOT; Map with declared
partial-section zero-fill; present empty and populated maps; WDT ROOT-absent
maps, WDT==0/hidden/inactive Map rows; poisonous GODI and LiquidObject payloads;
and a child whose immediate parent WDT is ROOT-absent. It checks that the WMO
and present child retain their VMO/placement/binding/spawn, while the absent
child has no VMO, binding, valid name, spawn or placement reference. Direct ADT
M2 and WMO root/group/name/ADT, corruption, ineligible ROOT, missing central or
encoding and other source failures remain fatal. Parent-only inheritance cannot
skip through a missing parent to a populated grandparent.

Lifecycle tests invoke the compiled CLI for plan-only inspect, extract, offline
verify and verify-links. Resume scenarios use fresh scratch and compare Map,
context, manifest, v5 coverage and artifact inventory byte-for-byte with a
baseline. Resealed negatives cover profile/version/policy/mask crossings,
missing/duplicate/unsorted omissions, false or invented WDT/M2 facts, omitted
artifacts, missing proofs, bad partial declarations, source poison cases and
parent replay. Help/capability output and verifier reports must retain v32/v5 and
RequestHeavyV1 identity without full-world or complete-component claims.

Map and active-domain coverage is independently derived from the sealed Map;
WDT and WMO-doodad raw ROOT absence, child read counts and source acquisition
flags remain producer-reported. The verifier cannot independently prove those
ROOT absences or reconstruct original omitted child requests/spawns. No client
assets, keys or client-derived files are included in repository fixtures.

## Qualification boundary

The contract, fixture, plan and CLI tests are synthetic software checks. The
failed real v32 run establishes no completed extraction or acceptance. A later
profile would need a separately versioned, producer-declared omission for a WMO
left with no geometry and no usable doodad selection; v32 does not admit that
omission. Full-world resources, complete collision geometry / VMap component
qualification, GODI/global GameObjects, LiquidObject, terrain, navigation/MMaps,
RustyCore runtime acceptance, format parity, Windows and reproducible
distribution/release remain open. `full_extractor_complete` stays `false`.
