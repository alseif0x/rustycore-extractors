# Forever product v31: available geometry with WDT and M2 ROOT omissions

Product v31 is a Linux build-70450 profile,
`Build70450AvailableModelRootAbsentGeometryV1`, with component mask **17**
(`db2,vmaps`). It extends the separate v30 profile with narrowly declared
WMO-doodad M2 ROOT omissions and removal of their derived spawns, while
retaining the parent WMO and rejecting direct ADT M2 omissions. It publishes
Map, VMOs actually present and geometry for present maps, without inventing a
model, accepting a prefix or claiming full-world, complete-component, MMaps,
runtime or distribution. The machine-readable contract is
[`forever-product-contract-v31.json`](../contracts/forever-product-contract-v31.json).

The v30 attempt is not acceptance for v31: tolerated WDT omissions and a later
M2 failure remain separate evidence. The real v31 run is **`failed`**: inspect
passed, extraction reached exactly `request_calls=4096` and exited 9 with
`collision_policy_limit`; the M2 probe and both package verifications did not
run. No final package or prefix was accepted. The retained redacted report is
`v31-resource-probe-20261024T234730Z-acceptance.json`, SHA-256
`6dd0daaaf3f601b91859789266e851a38f8367b1d37b47aa3fd1778b30e79444`.

The canonical esES synthetic fixture is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v31/all.tsv). Its seven
1,048,576-byte values exercise PLAN parsing only. They are not measured
requirements, real acceptance budgets or budgets certified for real extraction.

## Canonical plan and versioned identities

The plan requires `wowdbdefs-70450-v1`, disabled decryption and
`partial_encrypted_sections=declared-skip` immediately between `decryption` and
`collision-storage`. Storage is `linux-leased-append-disk-world-v1` with all
seven downward-only budgets. Extract and resume require new scratch; inspect,
verify and verify-links forbid it. Keys are refused.

The plan has exactly one trailing DB2 row, `Map` FDID 1349477, mask 17,
`collision-domain=all-available-maps` represented by `map_domain=None`, and
`MapSelection::All` in production and verification. Terrain and navigation are
`none`, off-mesh data is absent, packing is mandatory and flags/slopes retain
their defaults. There is no `requested_map`; zero and
`MapSelection::MapAndAncestors(0)` are not aliases for All.

The versioned literals are:

- plan profile `linux70450-available-map-model-root-omissions-vmap-product-v1`;
- plan coverage `linux70450-available-map-model-root-omissions-coverage-v4`;
- domain and codec profile `linux70450-available-map-model-root-omissions-v1`;
- completion `linux70450-available-map-model-root-omissions-collision-v1`;
- verification `linux70450-available-map-model-root-omissions-verification-v1`;
- coverage path
  `vmaps/available-model-root-omissions-geometry-coverage-v4.bin`.

RCVGEO1 v4 carries both
`run::MissingWdtPolicy::DeclareRootAbsent` and
`run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2`, plus a canonical ordered,
unique, nonzero `missing_m2` list after the map domain. It has no requested map
or sentinel. Selection All and the absence of omissions do not select a
version: v30 remains v3 with M2 `Reject`, and v31 with zero omissions still uses
v4. v22/v28 retain v1 and v29 retains v2. Sharing mask 17 does not permit
crossing versions, policies, profiles, paths or publication wrappers.

## Narrow WMO-doodad omission and spawn filtering

Only an M2 doodad requested by FDID while converting a present WMO is eligible.
Before acquiring that child, the producer must prove under unchanged guards
that raw ROOT has zero rows for the FDID. Each WMO child increments
`dependency_requests`; it does not add a top-level `request_calls` assertion.
A declared child has zero child `model_reads` and is recorded once in
`missing_m2`.

The omitted doodad has no model handle, VMO, `NameBinding`, valid name or spawn.
The present WMO root retains its VMO and WMO placement, and every present
doodad retains its own VMO, binding and spawn. Direct ADT M2 placements are not
filtered by this policy. `used_models` and the VMO inventory include only
present models, so a WMO with one present and one omitted child keeps the parent
and the present child without fabricating the absent one.

Direct ADT M2 requests (FDID or logical name), WMO roots, WMO groups, ADT,
liquids, malformed sources, encryption, central/encoding absence, ineligible
ROOT, corruption, conversion/no-collision outcomes, invalid associations, guard
changes and resource-limit failures remain fatal. They are not alternative
omission classes.

The WDT ROOT policy is unchanged. WDT absence requires `MissingFile` followed
by guarded zero-row raw ROOT proof. WDT==0 and IDs hidden in omitted Map
sections belong to neither active maps nor omission lists. WDT-omitted maps
remain in `Coverage.maps` but receive no tree, proof, tiles or models. Present
empty maps retain tree and empty proof. Production preserves
`maps = wdt_files + root_absent`.

If the immediate parent is WDT ROOT-absent, parent-only inheritance is
forbidden; replay cannot substitute payload from a populated grandparent. A
child with its own WDT and payload continues without inheritance. Present
active parents retain their existing shared-parent behavior.

## Offline verification and authentication boundary

Offline verification re-reads sealed Map with `read_vmap_maps_70450` and
independently re-derives sections, direct/copy/materialized facts, active IDs,
parents, depths, WDT FDIDs and `MapEntry::is_parent`. Present maps replay exact
tree/proof/tile/model associations. The verifier also rejects artifacts for
WDT-omitted maps and parent-only inheritance through them.

For every canonical `missing_m2` FDID, offline verification derives
`FILE{id:08X}.xxx` and rejects its VMO or any placement reference. The remaining
used models and VMOs must correspond exactly. Production separately rejects a
handle, `NameBinding`, valid name or spawn for that child. The original source
requests and binding/spawn set remain producer-reported rather than independently
reconstructed offline. Duplicate, zero or unsorted FDIDs and an M2 policy other
than `DeclareRootAbsentWmoDoodadM2` reject.

`coverage_authentication=independently_derived` applies to Map, graph,
artifacts and associations. WDT absence is
`root_absence_authentication=producer_reported`; model absence and the original
filtered doodad request/spawn set are
`model_absence_authentication=producer_reported`. Offline verification cannot
independently prove raw ROOT absence or reconstruct those original requests and
spawns. CKey/EKey acquisition flags also remain
`source_authentication_flags=producer_reported`, not independent cryptographic
source authentication.

The only DB2 artifact is `dbc/<locale>/Map.db2`. The manifest contains only
per-table `partial_encrypted_tables` declarations for selected Map, with no
global or foreign declarations. Resume rejects recovered foreign manifest
state before replay or final sealing. No content is decrypted. GODI/global
GameObjects and LiquidObject are not requested; terrain, navigation and MMaps
are not selected.

## Limits and synthetic CLI coverage

No previous numeric ceiling is raised: 128 MiB aggregate and placements,
384 MiB staged models, 4,096 maps/models and omitted M2 IDs, 65,535 coverage
artifacts and 65,536 package files **including Map.db2 and coverage**. The
package byte ceiling remains
`checked_add(collision-output-bytes, 17179869184)`, with overflow rejected.
Each new omitted doodad must fit below both the placement and collector model
ceilings before insertion. An excess returns `Error::Limit` before further
callbacks; it cannot reallocate beyond the reservation or hide uncharged vector
growth. `Completed::resident_bytes` charges the actual capacities of both
omission vectors.

Synthetic CLI coverage uses a present WMO whose doodads are one present M2 and
FDID 970793 without ROOT or encoding. It checks deduplicated `missing_m2`, the
WMO root VMO and placement, the present child VMO/binding/spawn, and zero child
`model_reads`, VMO, binding, valid name or spawn for the omitted child. A direct
ADT M2 request for that same absent FDID remains an explicit fatal case. The
fixture preserves WDT omissions, present empty/populated maps, WDT==0 and hidden
Map rows, per-table declarations and poisonous GODI/LiquidObject.

The scenarios cover inspect/extract/verify/verify-links, three resume
boundaries with new scratch and byte-identical baseline, foreign declarations,
false/overlapping/wrong-FDID omissions, omitted-doodad VMOs and spawns,
present-map proof loss, valid and forbidden inheritance, v30/v31/context/profile
crossings, direct-M2/WMO/group/name/ADT fatal sources and redacted opt-in
diagnostics.

## Qualification boundary

The failed real v31 attempt establishes no completed extraction, offline
package verification or runtime acceptance. Prior v30 behavior is not
transferred. This profile does not claim complete decoded DB2 content,
whole-client map coverage, full-world coverage, complete VMap component
qualification, independent offline ROOT/model-absence proof, WMO/group/ADT
omissions, GODI/global GameObjects, LiquidObject, terrain, navigation/MMaps,
RustyCore runtime acceptance, C++/TrinityCore parity, Windows support, a
distribution or release publication. `full_extractor_complete` remains `false`.
