# Forever product v15: Linux 70450 compatible VMaps

The v15 admission contract admits only the offline Rust-local Linux capability
for client build `wow_classic_beta 1.60.1.70450`. It admits collision/VMap output
only: component mask `16`, component `vmaps`, schema catalog
`wowdbdefs-70450-v1`, disabled decryption, leased disk-world storage, no terrain
output, and no navigation output. This is a versioned software contract, not
evidence of real extraction, full-world resources, or runtime acceptance.

## Plan and verification literals

The canonical plan uses `forever-product-v15`, build `70450`, and the separate
publication identity `Build70450CompatibleCollisionV1`. It uses the shared,
build-neutral `linux-leased-append-disk-world-v1` storage grammar and collision
profile `linux70450-compatible-gnu15-vmap-product-v1`. The coverage selector is
`compatible-collision-coverage-v1`; the domain is either
`all-available-maps` or `map:<canonical-u32>`. Its collision source profile is
`linux70450-compatible-gnu15-v2`.

The implementation binds separate build-70450 identities for the compatible
model spool, startup, world preparation, serial placements, world inventory, and
coverage inventory. In particular, the coverage profile is
`linux70450-compatible-disk-world-gnu15-native-order-coverage-v1`; it is not the
70425 profile. These identities and the plan's schema catalog are independently
gated to build 70450.

Verification uses profile `linux70450-compatible-gnu15-vmap-product-v1` and
scope
`artifact_integrity_complete_declared_vmap_associations_and_producer_reported_available_source_coverage`.
Offline verification checks producer-reported source coverage, not an independent
client snapshot.

## Storage, components, and key policy

The plan admits only the Linux compatible-VMaps collision component. It reuses
the existing leased disk storage grammar and its downward-only budgets, requires
fresh private scratch for extraction/resume, forbids supplied keys, and selects
neither terrain nor navigation. `inspect`, `extract`, `resume`, `verify`, and
`verify-links` are the declared command surface. The parser, publication profile,
build admission, catalog selection, and package context keep 70450 separate from
70425 and all earlier build contracts.

## Coverage file

`vmaps/compatible-collision-coverage-v1.bin` has the shared format magic
`RCVMCV1\0`, with the build-70450 coverage profile embedded and checked against
the coverage context. It records Map and GameObjectDisplayInfo source availability
and crypto flags; liquid material/type source facts; ordered startup rows and
dispositions; map topology; WDT status and each present-WDT map's 4096 own/replay
cell states; and complete ID-ordered placement proofs with size/digest bindings
for published VMap artifacts. The package stores per-map tree/tile outputs under
`vmaps/` and proof files under `vmaps/coverage/`.

Coverage is producer-reported. It binds the source, plan, producer, build, locale,
selection, and pack identities; verification checks the package context, plan
hash, selected map, profile identity, and listed artifact paths, sizes, and
SHA-256 values. These identities do not authenticate an independent client
snapshot and do not claim complete global Map coverage. A missing WDT is unknown
and emits no tree or proof; selecting `map:<id>` when that map's WDT is missing
rejects without fallback. Mandatory startup, Map, and liquid sources are required.
A present WDT with malformed required ADT input is fatal.

A sealed resume rebuilds the genuine six-owner result and compares every
artifact against the immutable published component. A complete output refuses
writes. API limits are downward-only and cap failures do not truncate output.

## Contract and fixture pins

The v15 contract test pins the canonical plan fields and compares the
`all-available-maps` fixture byte-for-byte with the Rust `V15` plan constant. The
plan tests reject cross-build/catalogue pairs, legacy profiles, unsupported
component masks, key policy changes, and altered domain/profile identities.

## Unqualified status

No real-client extraction has been performed for this admission. It does not
establish complete full-world resources, RustyCore runtime compatibility, release
acceptance, or real full-world geometry/world acceptance. Windows execution is
deferred. This contract makes no claim of full encrypted Map coverage or complete
extractor status.
