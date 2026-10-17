# Forever product v14: Linux 70425 compatible VMaps

The v14 admission contract describes the offline Rust-local Linux capability for
client build 70425. It admits collision/VMap output only: component mask `16`,
component `vmaps`, schema catalog `wowdbdefs-70425-v1`, disabled decryption,
leased disk-world storage, no terrain output, and no navigation output. It is a
versioned product contract, not evidence of full extraction or runtime acceptance.

## Plan and verification literals

The plan uses `forever-product-v14`, build `70425`,
`linux-leased-append-disk-world-v1`, and collision profile
`linux70425-compatible-gnu15-vmap-product-v1`. Its coverage selector is
`compatible-collision-coverage-v1`; its domain is either
`all-available-maps` or `map:<canonical-u32>`. The source profile is
`linux70425-compatible-gnu15-v2`.

Verification uses profile `linux70425-compatible-gnu15-vmap-product-v1` and
scope
`artifact_integrity_complete_declared_vmap_associations_and_producer_reported_available_source_coverage`.
Offline verification checks producer-reported source coverage, not an independent
client snapshot.

## Coverage file

`vmaps/compatible-collision-coverage-v1.bin` has the source magic `RCVMCV1\0`.
It records Map and GameObjectDisplayInfo source availability and crypto flags;
liquid material/type source facts; ordered startup rows and dispositions; map
topology; WDT status and each present-WDT map's 4096 own/replay cell states; and
complete ID-ordered placement proofs with size/digest bindings for published
VMap artifacts. The package also stores per-map tree/tile outputs under `vmaps/`
and proof files under `vmaps/coverage/`.

This coverage is producer-reported. It binds and verifies the source, plan,
producer, build, locale, selection, and pack identities: the package context carries
source/plan/producer hashes, build, locale, and component mask; verification checks
that context against the package manifest, checks the plan hash and selected map,
and matches every listed artifact path, size, and SHA-256 to the package. The
coverage-profile identifier is also checked against the fixed v14 schema. These
producer-reported identities do not authenticate an independent client snapshot,
and this coverage does not claim complete global Map coverage. A missing WDT
is recorded as unknown and emits no tree or proof; selecting `map:<id>` when
that map's WDT is missing rejects without fallback. Mandatory startup, Map, and
liquid sources are required. A present WDT with malformed required ADT input is
fatal.

A sealed resume rebuilds the genuine six-owner result and compares every
artifact against the immutable published component. A complete output refuses
writes. API limits are downward-only and cap failures do not truncate output.

## Contract and fixture pins

The v14 contract test pins all 26 canonical plan fields and compares both fixture
byte strings with the Rust `V14` constant (the `map0` fixture changes only the
selection). This exact-field coverage was strengthened after the independent
mutation audit included with the integration handoff found 18 of the 26 fixture
field values and several contract numbers unpinned.

## CLI admission gate

Commit `5f890a9` adds four synthetic CLI integration tests for the v14 contract.
`inspect` discloses source profile `linux70425-compatible-gnu15-v2`,
`coverage_authentication: producer_reported`,
`whole_client_map_coverage: false`, `full_extractor_complete: false`, and
`collision_domain: all-available-maps`. Both `verify` and `verify-links` refuse a
sealed collision package when coverage is absent (`package_inventory`) or present
but truncated (`product_collision_coverage_schema`); neither falls back to
semantic verification. The tests also pin `--keys` to `product_keys_forbidden`,
omitted `--scratch` to `product_scratch_required`, `--backend native` to
`product_usage`, legacy `collision-domain all` to
`product_plan_collision_domain`, `build 70374` to `product_plan_build`, and a
wrong component mask to `product_plan_component_scope`. The test is synthetic;
it does not establish real-client extraction or runtime compatibility.

## Unqualified status

No real-client extraction has been performed for this admission. There is no
RustyCore runtime verification, release acceptance, or real full-world
geometry/world acceptance. Windows execution is deferred. This contract makes
no claim of full encrypted Map coverage or complete extractor status.
