# Forever product v29: authenticated all-active available VMap geometry

Product v29 is a Linux build-70450 profile,
`Build70450AuthenticatedAvailableGeometryV1`, with component mask **17**
(`db2,vmaps`). It publishes the canonical available-section `Map.db2` and VMap
geometry for every active map admitted by that source, using
`MapSelection::All` in both production and offline verification. It is a new
publication and coverage profile, not a relabelling of v22 or v28. The
machine-readable contract is
[`forever-product-contract-v29.json`](../contracts/forever-product-contract-v29.json).

The canonical esES synthetic fixture is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v29/all.tsv). Its seven
1,048,576-byte storage values exercise PLAN parsing only. They are not measured
requirements, real acceptance budgets or budgets certified for real extraction.

## Canonical plan and production boundary

The plan requires `wowdbdefs-70450-v1`, disabled decryption and
`partial_encrypted_sections=declared-skip` immediately between `decryption` and
`collision-storage`. Storage is `linux-leased-append-disk-world-v1` with all
seven downward-only budgets. Extraction and resume require scratch; inspect,
verify and verify-links forbid it. Keys are refused.

The collision literals are:

- plan profile `linux70450-authenticated-available-map-geometry-vmap-product-v1`;
- domain `all-available-maps`, represented internally by `map_domain=None`;
- coverage profile `linux70450-available-map-geometry-coverage-v2`;
- codec/source profile `linux70450-available-map-geometry-v1`;
- collision completion profile
  `linux70450-available-map-geometry-collision-v1`.

There is exactly one trailing DB2 declaration: `Map` FDID 1349477. No other
table is selected. Packing is mandatory; terrain and navigation are `none`,
off-mesh data is absent, and the base-unit, liquid and slope options retain
their defaults. v29 is not part of `has_selected_geometry`, which remains the
v22/v28 selected-map dispatch.

`all-available-maps` does not admit a requested map through another spelling.
Coverage has no `requested_map` or replacement field. In particular,
`requested_map=0` and `MapSelection::MapAndAncestors(0)` are rejected rather
than treated as `MapSelection::All`.

## All-active map semantics

The sealed Map is read through the build-70450 VMap projection. The complete
materialized graph is built before active-map narrowing, and
`Maps::from_materialized_selection` receives `MapSelection::All`. The selected
domain contains every active admitted map, including a genuinely active map
with no placements or tiles. Such an empty map still requires its own VMap tree
and placement proof.

An absent, unknown or omitted ID is never invented or converted into an empty
active map. Source unavailability, malformed input, failed associations and
resource-limit failures remain fatal; none is evidence that a map was empty.
Rows whose WDT FileDataID is zero participate in materialization, parent
derivation and graph validation, but do not participate in `active_maps` and do
not acquire active-map artifacts. `MapEntry::is_parent` therefore comes from
the complete graph rather than only the active coverage list. A source with no
active rows has an explicitly empty geometry domain, without fabricated trees
or placement proofs; the selected-map profiles retain their non-empty domain
rule. Neither case establishes coverage of unknown sections or the whole client.

The only DB2 artifact is `dbc/<locale>/Map.db2`. Its available-source route uses
the v26/v27 structural, exact ordered multi-frame and zero-fill rules. The
manifest contains only per-table `partial_encrypted_tables` declarations,
including `Map` when partial. A complete table has no declaration. Global or
package-wide `partial_encrypted_sections` declarations are forbidden even
though the same-named PLAN gate remains mandatory. No encrypted content is
decrypted or recovered.

GODI and global GameObject startup are excluded by policy, with no GODI request
or global startup model acquisition. Models are placement-only and must match
the exact used-model/VMO inventory. `LiquidObject` is out of scope and is not
requested; `LiquidMaterial` and `LiquidType` retain their existing source
checks. MMaps, terrain and navigation outputs are not selected. These policy
exclusions are not claims about source failure or complete-extractor coverage.

## Coverage v2 and strict profile wrappers

v29 writes `RCVGEO1` layout **version 2** at
`vmaps/available-geometry-coverage-v2.bin`. Version 2 removes `requested_map`
and has no substitute for it. Its domain label is
`available-map-all-active-v1`, and its selection is `MapSelection::All`.

The wrappers remain strict:

- v22: RCVGEO1 v1, mask 16, selected coverage v1 and
  `available-map-active-ancestors-v1`, with producer-reported coverage;
- v28: RCVGEO1 v1, mask 17, selected coverage v1 and
  `available-map-active-ancestors-v1`, with independently derived coverage;
- v29: RCVGEO1 v2, mask 17, available coverage v2 and
  `available-map-all-active-v1`, with independently derived coverage.

Sharing mask 17 does not allow v28/v29 coverage, contexts, selections or
publication profiles to cross. Foreign or mixed masks and profile contexts are
rejected before coverage acceptance. The v22/v28 plan grammars, layout v1 and
selected active-ancestor semantics remain unchanged.

## Offline authentication and resource limits

Verification uses profile
`linux70450-authenticated-available-map-geometry-verification-v1` and report
scope
`artifact_integrity_available_map_geometry_associations_and_independently_derived_map_vmap_domain_coverage`.
It checks package integrity, intrinsic DB2/VMap structure and associations,
per-table Map declarations, and mandatory trees and placement proofs for every
active map. `verify-links` adds no terrain, navigation, MMap or external-package
claim.

`coverage_authentication=independently_derived` applies to Map section states,
VMap declared/direct/copy/materialized identities and counts, and the complete
graph/all-active domain re-derived from sealed Map. Acquisition CKey/EKey flags
remain `source_authentication_flags=producer_reported`; they are not independent
cryptographic source authentication. Inherited tiles require every consulted
ancestor to be active in the selected domain; an artifact from a WDT-zero row
cannot satisfy their payload. Own tiles retain their existing parent behavior.

Verification does not reread WDT/ADT or source-cell masks. Consequently, an
empty tree/proof proves its required presence and intrinsic associations, not
that the source was present or readable. Distinguishing a genuinely empty
source from an acquisition omission remains outside this offline check and does
not turn producer-reported acquisition into independent source authentication.

No existing numeric ceiling is raised. In particular, the limits remain 128 MiB
for the assembled domain, 128 MiB for staged placements, 384 MiB for staged
models, 4,096 coverage maps and 65,535 coverage artifacts. Map is limited to
16 MiB, DB2 to 16 GiB total, partial declarations to 65,536 frames per table
and 18,576 overall. The package contains at most 65,536 files including Map and
coverage; resume rejects recovered declarations outside the selected profile
before replay or sealing. The package byte ceiling is
`checked_add(collision-output-bytes, 17179869184)`; overflow rejects and the
maximum is 85,899,345,920 bytes.

## Redacted failure diagnostics

Setting `FOREVER_PRODUCT_DIAGNOSTIC` to a new file outside the client and package
opts into one redacted first-source-failure record when its code matches the
terminal error. It records phase, request kind, optional FDID, ordinal and typed
cause, without paths, payloads or keys. WDT, tile ADT, root M2/WMO, child and
WMO-group requests are distinguished. The file is never overwritten and the
observer does not change terminal results, acceptance or network policy; normal
production can still leave provisional package state after a failure.

## Real acceptance remains pending

`real_acceptance.status` is **`pending-after-failed-attempt`**. The retained
initial run `v29-available-geometry-20261024T144152Z` failed during extraction
with `file_unavailable` in `vmaps` after 38 seconds. DB2 sealed, but no final
manifest, VMap coverage or accepted prefix exists; verify and verify-links did
not run. No network attempt or input change was recorded. The redacted report
`v29-available-geometry-20261024T144152Z-acceptance.json` has SHA-256
`20b0735f0fc85808b502451c7a642109bfa441f37ec57cba20aa6a6d294043a9`.
The follow-up run failed on the same source after 37 seconds; its report has
SHA-256 `597ad8e3e276f087d2bf8fdc273d0ca6f71c6c3338b10bb2fa59185ac4f867aa`.
Its observer identified `placements` / `wdt` / FDID 857697 / `MissingFile`.
An offline ROOT census found 71 active maps, 69 with ROOT routes and two
root-missing maps (13 / 857697 and 3109 / 7937200), with no
root-present/encoding-absent case. The redacted census report has SHA-256
`2a8483ddce57924f157be0c04d855a228df7665a869b2e900aba14e92144a48d`.
These are absent ROOT entries, not encrypted payloads to recover by brute
force. Strict v29 remains unaccepted; any omission requires a separate
versioned profile and must not manufacture geometry or claim full-world.
Existing v28 real evidence covers its separate selected-map/active-ancestor
product and is not transferred to v29.

This profile does not claim complete decoded DB2 content, whole-client map
coverage, full-world coverage, complete VMap component qualification, GODI or
global GameObjects, LiquidObject, terrain, navigation/MMaps, RustyCore runtime
acceptance, C++/TrinityCore parity, Windows support, a distribution or release
publication. `full_extractor_complete` remains `false`.
