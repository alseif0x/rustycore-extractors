# Forever product v34: available geometry with intrinsically empty WMO declarations

Product v34 is a distinct Linux build-70450 profile,
`Build70450AvailableEmptyWmoGeometryV1`, with component mask **17**
(`db2,vmaps`). It separates a WMO that was intrinsically empty before any
omission from a WMO vacated by a locally declared M2 ROOT absence. V34 preserves
v33 and does not claim complete geometry, full-world coverage, runtime
acceptance or distribution. The machine-readable contract is
[`forever-product-contract-v34.json`](../contracts/forever-product-contract-v34.json).

Real v34 acceptance is **`failed`**. Inspect passed, but extract exited 9 with
`collision_no_valid_collision`; the probe and both verifications did not run,
and no package or prefix was accepted. The retained report
`v34-empty-wmo-20261025T231225Z-acceptance.json` has SHA-256
`3a00385706ce85359bd5032fb47853d0a4fed9335436ddf3df1ffd81fdaf019c`.
A separate bounded diagnostic proves that WMO FDID **462695** has group FDID
**463228** with **368 triangles, 1,104 indices, 591 vertices and one batch**,
without liquid. It has real source geometry, so `intrinsic_source=0` and v34
correctly rejects it even though collision selection and output are empty. The
redacted diagnostic has SHA-256
`8bbaa2e588292980ac2894d30e6c3bac446571439f233e051376d2ab43d3f68f`.
Any distinct policy for present-but-unselected geometry requires a separate
version and acceptance; it cannot redefine `empty_wmo`. Promotion requires exactly `missing_m2=[970793]`, `missing_wmo=[]` and
`empty_wmo=[462695]`, followed by a successful ROOT probe, `verify` and
`verify-links`. Any additional omission or accepted prefix rejects the run.
The earlier real v33 failure remains separate: WMO FDID **462695** had no
selected collision output before omission, with `local_missing_m2=0`,
`local_missing_names=0` and `omitted_usable=0`, so v33 correctly rejected it
with `collision_no_valid_collision`. The retained v33
report has SHA-256
`7ca5b12e7f78899a384617f79d6350e19d320415022f741471c38951bf11e1d4`;
its diagnostic has SHA-256
`9e5c6e05ff039e652dda5cf77f713146375066b6378fdfeffac6d8cb02b7e9b7`.
Neither is v34 evidence.

## Canonical plan and versioned identities

The canonical esES synthetic plan is
[`all.tsv`](../tests/contracts/fixtures/forever-product-v34/all.tsv). It binds
build 70450, `wowdbdefs-70450-v1`, disabled decryption, mandatory
`partial_encrypted_sections=declared-skip`, leased disk storage with seven
downward-only synthetic budgets, exactly Map FDID **1349477**, component mask
**17**, all available maps and `MapSelection::All`. `map_domain=None`; there is
no `requested_map` or zero sentinel. Terrain and navigation are absent, and the
ordinary plan flags remain unchanged. The 1 MiB fixture budgets exercise plan
parsing only and are not certified real-extraction budgets.

The versioned identities are:

- publication profile `Build70450AvailableEmptyWmoGeometryV1`;
- plan profile `linux70450-available-empty-wmo-vmap-product-v1`;
- PLAN coverage `linux70450-available-empty-wmo-coverage-v7`;
- source/codec profile `linux70450-available-empty-wmo-v1`;
- domain profile `available-empty-wmo-v1`;
- completion `linux70450-available-empty-wmo-collision-v1`;
- verification `linux70450-available-empty-wmo-verification-v1`, scope
  `artifact_integrity_available_empty_wmo_and_independently_derived_map_vmap_domain_coverage`;
- resource identity `linux-collision-request-heavy-resources-v1`;
- RCVGEO1 v7 at `vmaps/available-empty-wmo-geometry-coverage-v7.bin`.

V34 retains the v32/v33 `RequestHeavyV1` capacity profile. Only `max_requests`
is raised relative to `BoundedV1`, to 16,000,000; v34 changes no limits relative
to v33. Read, dependency-request, source/RAW-byte, model, memory, storage,
spool, inventory, staging and package ceilings remain unchanged. This does not
select `DiskWorldV1`, decrypt content, use keys or enable network access.

RCVGEO1 v7 carries `missing_wmo` after the RequestHeavyV1 resource identity,
then the separate canonical `empty_wmo` list before artifact facts. Both lists
are ordered, unique, nonzero and bounded by 4,096 entries. They are disjoint
from each other and from `missing_m2`. V33 stays RCVGEO1 v6 with the old policy;
sharing component mask 17 does not permit crossing plan, coverage, resource or
omission identities.

## Separate vacated and intrinsic predicates

`missing_wmo` retains the v33 meaning: a WMO FileDataId vacated only after its
own conversion observed at least one locally declared ROOT-absent WMO-doodad
M2, with no geometry output or remaining binding, MODR participation in
selection and empty selection references after omission.

`empty_wmo` is different. It admits only a WMO FileDataId that was already empty
before omission: `local_missing_m2=0`, `omitted_usable=false`, `output=None`,
empty bindings, nonempty selection sets and empty selection references. Any
remaining geometry, binding or usable dependency, logical-name origin, absent
selection sets or unrelated failure remains fatal. Neither category is inferred
from a request count, file-name prefix or an unrelated no-collision result.

Both categories emit no VMO, binding, valid name, spawn, placement or model ID.
A different present WMO and each present doodad remain intact. Direct ADT M2,
WMO root/group, ADT, liquid, malformed, encrypted, corrupt,
central/encoding-absent, guard-change and resource-limit failures are not
omission classes and remain fatal.

## Offline verification and synthetic checks

Offline verification independently re-derives Map section/materialization facts,
the complete active map graph, present-map artifacts, associations and package
inventory. It checks the canonical separate WDT/M2/WMO declarations and ensures
no artifacts or references are published for omitted models, vacated WMOs or
empty WMOs. Raw ROOT absence, original omitted doodad requests/spawns,
conversion-time vacating, intrinsic geometry absence and source-authentication
flags remain `producer_reported`; offline verification cannot independently
prove them.

Independent review retained three failures in the initial external auditor:
synthetic report promotion, incomplete GODI/Map-section/liquid validation and
possible evidence overwrite on a reused run. The corrected parser now checks
those fields and canonical contexts. The report requires every exit and control,
exact identities/lists and a linked ROOT probe, and rejects synthetic markers.
The runner rejects noncanonical paths and preexisting evidence before writing.
The revised auditor passes 18 cases with 16 negatives, 32 additional parser
negatives, 56 promotion cases with zero synthetic promotions, eight runner cases,
four placement-bounds cases and 65-ID/two-batch probing. Its `selftest.json` has
SHA-256 `7d8b53877363808e036c4a7561b9d0d21c5de606482eff7c44db75dee88027f5`.
The initial evidence is retained, not promoted. `RCE_V34_REAL_ACCEPTANCE=1`
alone is not authorization or proof. No real-client acceptance has occurred.

## Qualification boundary

Real v34 acceptance failed and is not promoted. Full-world resources, complete VMap
geometry/component, GODI/global GameObjects, LiquidObject, terrain,
navigation/MMaps, RustyCore runtime acceptance, C++/TrinityCore parity, Windows,
reproducible distribution and release publication remain open.
`full_extractor_complete` remains `false`.

The current synthetic checkpoint passes 4/4 v34 contract tests, 9/9 linked plan
tests, 29/29 coverage tests, 65/65 compatible-VMap verifier tests and 688/688
extractor library tests. CASC passes 1,820 library tests with five ignores and
all its crate targets. Independent CLI review found uncovered residual-WMO
modes, verifier-only mutations, FDID and ID-allocation requirements. The
corrected CLI passes 23/23 and Clippy, explicitly covering those findings,
EmptyWmoOnly with FDID 462695, v33/v34 crossings and duplicate-key rejection.
Full workspace qualification remains separate from real acceptance. See also the
[retained v33 failure](forever-product-v33.md) and the
[completion gates](forever-completion.md).
