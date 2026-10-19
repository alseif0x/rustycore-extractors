# Forever product v22 — selected build-70450 collision geometry

Product v22 is a Linux-only Rust-local build-70450 profile for the `vmaps`
component only (mask `16`). Its publication identity is
`Build70450SelectedGeometryV1`; its own plan profile is
`linux70450-selected-map-geometry-vmap-product-v1`. It is deliberately separate
from the existing v15/v18 GNU15 and v21 navigation profiles.

## Closed scope and canonical plan

The plan is canonical ASCII TSV v22 with build `70450`, locale, index and root
layouts, `wowdbdefs-70450-v1`, and `decryption disabled`. Keys are refused. The
storage grammar is the existing Linux leased append disk-world grammar, so
`extract` and `resume` require `--scratch`; `inspect` and `verify` do not.
Plans select exactly `map:<canonical-u32>`. The product derives that map's
active ancestor closure from the available-section Map reader. It does not admit
`all-available-maps` and does not declare global map coverage.

The canonical synthetic plan is
`tests/contracts/fixtures/forever-product-v22/map2.tsv`. Its own coverage
identity is `linux70450-selected-map-geometry-coverage-v1`; the literal
`gameobject-scope excluded-by-profile-v1` means global GameObject startup is
excluded as policy, not that any source failure or cause was observed. The
`model-scope placement-only-v1` policy is emitted by the producer only after
the executed startup has no global TempList rows, its GODI startup read count is
zero, and its published VMO inventory exactly matches the models registered by
real placement requests. This is a structural proof from the v22 route, not a
constant inherited from the staged VMO API.

## Sources, outputs, and omissions

The implementation uses the existing
`forever_casc::collision::geometry::extract_map` kernel. It reads the Map domain
with available-section coverage and ancestor closure, validates complete
`LiquidMaterial` and `LiquidType` sources, and strictly acquires selected WDT,
ADT, M2, WMO, and WMO group inputs. Missing or malformed requested geometry is
fatal. The runner makes no GODI request. `LiquidObject` is out of scope and is
not requested, even when a synthetic store contains a poisonous table.

`prepare_placement_only` and `ModelScope::PlacementOnly` in
`assemble/staged/models.rs` remain part of the separate staged VMO API. That
constructor requires a spooled `Staged` owner whose startup reads GODI; v22 does
not call it. The v22 placement-only scope instead follows from startup without
global lists plus the exact `used_models`/VMO-artifact correspondence above.

The producer stages callback models and placement streams under fixed limits.
Only after the kernel returns a successful completion does it assemble VMO
models, placement proofs, VMap trees and tile payload/index files into the
common `Run` state. A failed producer does not seal/publish the collision
component; successful writes remain provisional until the shared package
lifecycle seals it. Resume uses the existing common package replay rules.

Published artifacts live under `vmaps/`, including
`vmaps/selected-geometry-coverage-v1.bin`. Offline verification checks the
package inventory and digests, reconstructs selected-map trees/tiles from the
retained placement proofs, checks model bindings and inherited tile
associations, and validates the producer-reported coverage facts. Verification
does not reread a client snapshot.

## Coverage facts and limits

Coverage declares the selected map and active ancestors, the available-section
Map bytes/rows/section states, global GameObject exclusion by policy and no GODI
request with zero startup reads, structurally derived placement-only models,
complete liquid sources,
LiquidObject out of scope with no request made, and artifact size/SHA-256
bindings. It records
`complete_global_map_coverage_claim: false`.

The seven storage budgets only reduce their existing leased-disk ceilings.
Additional downward-only product caps bound coverage to 128 MiB, 4096 maps and
65,535 non-coverage artifacts; staged VMO output to 384 MiB; placement staging
to 128 MiB total and 64 MiB per map; combined callback staging to 512 MiB;
models to 4096; placement records per map to one million; and per-map assembly
memory to 512 MiB and assembly output to 128 MiB, with a 128 MiB aggregate
selected-domain VMap tree/tile/proof ceiling. Limit failures are terminal and
never truncate a package.

## Bounded real acceptance

The post-audit producer binary
`d75acb087b49bdd759c25ff64904085f7909f7e5b2536ddcfc593123b66cce89` passed a
bounded real build-70450 `esES` plan for map 48, component mask 16 (`vmaps`),
with decryption disabled and leased-append disk storage. The profile excluded
GameObject globals, selected placement-only models, kept `LiquidObject` out of
scope, and used available `Map` with its active-ancestor closure and complete
liquid sources. `inspect`, `extract`, `verify`, and `verify-links` each returned
0; extraction reported `selected_producers_complete` and produced a verified
package. The bounded run used a persistent service limited to
`RuntimeMaxSec=180` and `MemoryMax=2G`, with its private execution directory
outside every checkout.

The package contains 66 files / 2,079,754 bytes. Its
`inventory_sha256` is
`aa9d5cee883beea9789a3e43572d22d7062540696cd168a79729484118e31a0b`, its
`producer_sha256` is
`d75acb087b49bdd759c25ff64904085f7909f7e5b2536ddcfc593123b66cce89`, matching
the accepted binary above, and its `source_sha256` is
`8caee5a30da7aa1b0917d04b0cc79f1c214385d853e63d350a73343fe3983c8e`. It records
`full_extractor_complete: false`,
`complete_global_map_coverage_claim: false`, and
`coverage_authentication: producer_reported`. Source coverage is declared by the
producer; the verifier checks the published facts and artifacts but does not
reread the source `Map` to authenticate that coverage.

Against execution of the same plan with the preceding binary
(`0ea038c4…`), 65 of the 66 published artifacts are byte-identical (55 .vmo, 4
.vmtile, 4 .vmtileidx, 1 .vmtree, 1 .placements). Only
`vmaps/selected-geometry-coverage-v1.bin` differs among the published artifacts.
`forever-package.json` and `.state/sealed/vmaps` also differ because they
incorporate producer context. Thus the audit fixes changed no client-derived
VMap bytes.

A private read of the VMO files from this package using the producer's own
reader reports 55 nonempty models, 77 groups, 42,337 vertices, 47,175 triangles,
15 liquid groups, and 1,992,541 VMO bytes. This is evidence from the producer's
own parser only: it is not parity with the C++ reference, RustyCore runtime
acceptance, or navigation acceptance.

Earlier attempts using `enUS` with the same plan and preceding binary failed
with `local_store_no_verified_encoding` and produced no package. The local
installation has `esES` sources; those failures are locale-specific and do not
establish an equivalent blockage in `esES`. They are retained as failed attempts,
not as a v22 profile barrier or real-extraction acceptance.

## Unqualified status

The bounded package does not establish format parity with the C++/TrinityCore
reference, RustyCore runtime consumption, navigation/MMaps, full-world or
whole-client coverage. Windows and release publication remain out of scope.
Acceptance from v14, v15, v18, or v21 does not transfer to v22, nor does this
v22 result transfer to another profile or target.
