# Forever product v30: available geometry with declared ROOT omissions

Product v30 is a Linux build-70450 profile,
`Build70450AvailableRootAbsentGeometryV1`, with component mask **17**
(`db2,vmaps`). It publishes the canonical available-section `Map.db2` and VMap
geometry for present active maps while declaring only WDT inputs whose raw ROOT
contains no row for the requested FDID. It is not a retroactive repair or
acceptance transfer from v29, and it is not a complete-extractor, full-world or
runtime claim. The machine-readable contract is
[`forever-product-contract-v30.json`](../contracts/forever-product-contract-v30.json).

The canonical esES synthetic fixture is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v30/all.tsv). Its seven
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

- plan profile `linux70450-available-map-root-omissions-vmap-product-v1`;
- plan coverage `linux70450-available-map-root-omissions-coverage-v3`;
- domain and codec profile `linux70450-available-map-root-omissions-v1`;
- completion `linux70450-available-map-root-omissions-collision-v1`;
- verification `linux70450-available-map-root-omissions-verification-v1`;
- coverage path `vmaps/available-root-absent-geometry-coverage-v3.bin`.

RCVGEO1 v3 carries an explicit
`run::MissingWdtPolicy::DeclareRootAbsent` discriminant. Selection All does not
choose the codec version or policy, and a v30 package with zero omissions still
uses v3. v22/v28 retain v1 and v29 retains v2; sharing mask 17 does not permit
crossing paths, profiles, selections, versions or publication wrappers.

## ROOT absence is a narrow acquisition fact

A WDT `MissingFile` is not sufficient evidence. The opt-in producer path must
subsequently prove, under unchanged guards, that raw ROOT has zero rows for
that WDT FDID. Only then may it record `RootAbsent { map_id, wdt_file_data_id }`.
The older strict routes retain `run::MissingWdtPolicy::Reject` and do not use
`allow_missing` or suppress sticky errors.

ROOT present but ineligible, missing central CKey, absent or unverified encoding,
corruption, encryption, malformed WDT, absent ADT/model/WMO resources, guard
changes and resource-limit failures remain fatal. None becomes ROOT-absent.
WDT==0 rows still participate in Map materialization and graph construction,
but are neither active maps nor ROOT omissions. IDs hidden by omitted Map
sections are never invented as active or ROOT-absent maps.

A ROOT-absent map retains its active `MapState` and its entry in `Coverage.maps`,
but skips WDT parsing, globals, processing, placements, model requests, ID
assignment, cache and callbacks. No `SourceFacts`, tree, placement proof, tile
or VMO is fabricated for it. `DomainMapFact.root_absent` is encoded as zero for
Present and one for RootAbsent. Coverage contains the complete active domain,
not only present maps, and production checks
`maps = wdt_files + root_absent`.

Present maps retain the strict runner. A present empty map has a tree and an
empty placement proof, with no invented tiles. A populated map has exact
proof/tree/tile/model associations. Thus present-empty and ROOT-absent are
distinct states, including when both publish no tiles.

## Parent replay and artifact verification

Graph fallback is evaluated before source replay. If the immediate parent is
ROOT-absent, parent-only inheritance is forbidden; replay must not skip that
parent and borrow payload from a populated grandparent. A child with its own
WDT and payload continues without inheritance. Active parents that are present
retain the existing shared-parent behavior.

Offline verification re-reads sealed Map with `read_vmap_maps_70450`, retains
WDT==0 rows for the complete graph and independently re-derives sections,
direct/copy/materialized facts, active IDs, parents, depths, WDT FDIDs and
`MapEntry::is_parent`. Present maps replay exact tree/proof/tile/model
associations. ROOT-absent IDs are checked for the absence of those artifacts,
and inventory comparison provides a second barrier.

`coverage_authentication=independently_derived` applies to Map, graph,
artifacts and associations. ROOT absence itself is
`root_absence_authentication=producer_reported`: offline verification cannot
prove that the source WDT had zero raw ROOT rows. CKey/EKey acquisition flags
also remain `source_authentication_flags=producer_reported`, not independent
cryptographic source authentication.

The only DB2 artifact is `dbc/<locale>/Map.db2`. Its available-source route uses
the v26/v27 structural, exact multi-frame and zero-fill rules. The manifest has
only per-table `partial_encrypted_tables` declarations for selected Map, with
no global or foreign-table declarations. Resume rejects recovered foreign
manifest state before replay or final sealing. No content is decrypted.

GODI/global GameObject startup and LiquidObject remain outside the profile and
are not requested. Models are placement-only with exact used-model/VMO
inventory. Terrain, navigation and MMaps are not selected.

## Limits and synthetic CLI coverage

No previous numeric ceiling is raised. This includes 128 MiB assembled-domain
and placement limits, 384 MiB staged models, 4,096 maps, 65,535 coverage
artifacts and 65,536 package files **including Map.db2 and coverage**. The
package byte ceiling remains
`checked_add(collision-output-bytes, 17179869184)`, with overflow rejected.

Synthetic CLI coverage exercises inspect/extract/verify/verify-links, exact
present/ROOT-absent inventory, a present empty map, populated maps, WDT==0 and
hidden Map rows, and a ROOT-absent parent between a populated grandparent and a
child with its own WDT. Resume is checked after DB2 sealing, after Collision
sealing without a final manifest, and with Collision incomplete. Mutated
packages reject false, overlapping or wrong-FDID omissions, omitted-map
artifacts, missing required omissions, parent-only inheritance through
ROOT-absent, absent proofs for present empty maps, and v29/v30/context/profile
crossings. Diagnostic records are synthetic and redacted.

## Real acceptance failed at an M2 dependency

`real_acceptance.status` is **`failed`**, not `target-not-run`. The real esES
run `v30-root-absent-geometry-20261024T193255Z` passed inspect and declared two
WDT ROOT absences, but extraction then failed with `file_unavailable` for the
M2 child dependency FDID 970793. That model is not a WDT and v30 admits no model
omission. Verify and verify-links did not run; no final manifest or accepted
package exists. The redacted failure report has SHA-256
`47b3057c3f6be62bdeca22af4da5a4807ad031ac5c43c32f126543e8621bd17e`.
Prior v28 acceptance and failed v29 attempts remain separate evidence and do
not transfer to v30. A later model-omission profile cannot relabel this failed
v30 attempt as accepted.

This profile does not claim complete decoded DB2 content, whole-client map
coverage, full-world coverage, complete VMap component qualification, offline
independent ROOT-absence proof, GODI/global GameObjects, LiquidObject, terrain,
navigation/MMaps, RustyCore runtime acceptance, C++/TrinityCore parity, Windows
support, a distribution or release publication. `full_extractor_complete`
remains `false`.
