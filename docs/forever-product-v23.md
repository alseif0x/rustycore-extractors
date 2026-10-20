# Forever product v23: selected Linux navigation

> A bounded real Linux map-48 CLI run passed ten lifecycle cases. Complete
> extraction, full-world resources and RustyCore runtime acceptance remain open.
> The connected synthetic workflow and final extractor-target checks also pass.
> A later nonempty-terrain run produces eight MMaps; the qualified local Linux
> package reproduces all nine navigation payloads and passes both verifiers.

V23 adds a Linux command-line route for build **70450** navigation from two
already sealed packages: v20 terrain and v22 selected-map geometry. It uses
available `Map` sections and complete `LiquidType` from the local client, with
**decryption disabled**. The output component mask is **32** (`mmaps`).

The profile is defined by
[`forever-product-contract-v23.json`](../contracts/forever-product-contract-v23.json).
It does not change v20, v21 or v22 admission. V21 retains its complete-source
navigation contract.

## Inputs

Provide all five inputs explicitly:

| Input | Requirement |
| --- | --- |
| Navigation plan | Canonical v23 TSV; build 70450; exactly one selected map; resident storage; disabled decryption; no DB2 output entries |
| Client directory | The matching local build; read only |
| Terrain package and producer plan | Sealed v20 package with the selected map's required terrain and active ancestors; canonical v20 plan |
| VMap package and producer plan | Sealed v22 package for that selected map and its active ancestors; canonical v22 plan |
| New output directory | Separate from the client, input packages and producer plans |

Both packages must match the client's build, locale and source identity. Their
producer and plan hashes may differ from each other and from the new binary.
Their exact inventories, coverage records and producer plans are bound into the
new package context. Valid v20 and v22 producer plans have **no DB2 output
entries**; their source tables are resolved through their own pinned catalogs.

Terrain must have a loaded WDT and complete conversion of its known tile set
for every required map. A loaded WDT with an empty tilelist is distinct from an
unavailable WDT. This profile does not turn missing terrain into empty terrain.
Coverage remains producer-reported and available-section scope.

## Commands

Build `forever-extractor` using the [Rust quick start](../README.md#build-the-rust-product).
Use a navigation plan whose locale matches both input producer plans. The
[synthetic canonical example](../tests/contracts/fixtures/forever-product-v23/map48.tsv)
uses `enUS`; it is not evidence that a particular installed locale is available.

The variables below refer to explicit local paths. Keep generated output outside
the checkout and client directory.

```bash
forever-extractor inspect --plan "$PLAN"

forever-extractor extract \
  --plan "$PLAN" --client "$CLIENT" --output "$NEW_PACKAGE" \
  --terrain-package "$TERRAIN_PACKAGE" --terrain-plan "$TERRAIN_PLAN" \
  --vmap-package "$VMAP_PACKAGE" --vmap-plan "$VMAP_PLAN"

forever-extractor verify --plan "$PLAN" --output "$NEW_PACKAGE"

forever-extractor verify-links \
  --plan "$PLAN" --output "$NEW_PACKAGE" \
  --terrain-package "$TERRAIN_PACKAGE" --terrain-plan "$TERRAIN_PLAN" \
  --vmap-package "$VMAP_PACKAGE" --vmap-plan "$VMAP_PLAN"
```

`inspect` validates the plan only. Client inspection is refused for v23.
`extract` and `resume` require both input package/plan pairs. Keys and scratch
storage are refused.

To resume a retained incomplete package, replace `extract` with `resume` and
keep the exact plan, input identities and producer binary. Resumption reopens
and revalidates the inputs and local source before replaying recorded outputs.
A completed sealed component is not an incomplete run to resume.

## Verification and output

The Navigation component contains its MMap global/tile outputs and
`mmaps/selected-navigation-coverage-v1.tsv`, with the normal package manifest,
component seal and run receipts. Coverage records the selected domain and
provenance; it is not a whole-client completeness claim.

| Operation | Established scope |
| --- | --- |
| `verify` | Offline package integrity, intrinsic navigation formats and stored provenance |
| `verify-links` | Offline verification plus exact links to the supplied sealed terrain/VMap inventories, coverage and producer plans |
| `extract` / `resume` | Fresh local source acquisition and guarded input checks in addition to generation/replay |

Offline verification does not reread client `Map`, authenticate unavailable
sections or independently establish producer-reported source coverage. Guarded
source stability is not an atomic client snapshot. Per-domain limits do not
constitute a global RSS guarantee. Every v23 CLI result retains
`full_extractor_complete: false`.

Complete-source coverage, production
`MMapManager` integration and filenames, off-mesh/full-world navigation,
TrinityCore format parity, complete runtime/server acceptance, Windows delivery
and release publication remain outside this checkpoint.

## Bounded real checkpoint with empty terrain

The real esES map-48 package has six public artifacts / 842,691 bytes: four
nonempty tiles, a 40-byte global and coverage. All five navigation payloads match
the preceding filesystem-owner output. Fresh and resumed manifests are
identical; offline verification, linked-input verification, expected-error cases
and interruption/resume passed. Input packages stayed unchanged.

This selected terrain inventory is authoritatively empty, so it gives no
nonempty ground-MAPS acceptance. The independent-reader native SDK checkpoint
uses the earlier filesystem-owner outputs; this CLI run does not add production
manager or server acceptance. See the [redacted report](acceptance/forever-v23-selected-navigation-70450-20261020.json)
and [navigation checkpoint](forever-navigation.md#2026-10-20-bounded-real-v23-selected-navigation-cli).

## Bounded real checkpoint with nonempty terrain

A subsequent build-70450 esES run used the full sealed v20 terrain package and
another sealed v22 geometry package (149 artifacts / 2,199,694 bytes). Fresh
v23 admission confirmed one active-closure map, a loaded WDT, six known-present
and converted terrain tiles, and zero missing root ADTs. The two upstream
producer identities are pinned separately; the older v20 producer differs from
the binary used for later verification and v22/v23 generation.

All four commands passed: plan-only inspection, extraction, offline verification
and linked-input verification. Eleven candidate navigation tiles yielded eight
nonempty outputs and three recorded empty outcomes. The output has ten public
artifacts / 3,787,736 bytes, including a 40-byte global and coverage. Execution
took 52.846 seconds under observed 900-second / 3-GiB / one-CPU / 64-task limits;
source, binary, plans and both input trees remained stable.

This establishes one bounded navigation package with nonempty real terrain
inputs. A subsequent typed native SDK probe accepts its original eight outputs;
production manager/server acceptance remains open. Earlier native results retain
their original inputs. Complete-source,
full-world, Trinity parity, Windows and release gates remain open. See the
[ground checkpoint](forever-navigation.md#2026-10-20-bounded-real-v23-navigation-with-nonempty-terrain)
and [redacted report](acceptance/forever-v23-selected-navigation-ground-70450-20261020.json).

## Qualified local Linux v12 candidate

The local archive frozen at `719faee` has two matching fresh offline builds and
a matching moved-source rebuild. Its extracted binary passes 37 synthetic product
cases and ten v23 CLI/fixture cases. A separately executed real ground run passed
all four commands in 54.066 seconds and reproduced the earlier eight MMap tiles
and global byte-for-byte; metadata records the new packaged producer identity.
This adds bounded packaged-binary acceptance, with no release or runtime claim.
See the [distribution qualification](forever-distribution.md).

The later [native SDK checkpoint](forever-navigation.md#2026-10-20-native-sdk-accepts-the-eight-ground-navigation-tiles)
loads and unloads the original eight ground tiles, resolves 98 nearest-poly
queries and finds six complete cross-tile routes in 256 attempts; the other 250
do not reach the requested final polygon and remain unqualified. This uses a
strict typed adapter and preserves v16 headers/payloads; it does not accept the
legacy v15 reader, production filenames/manager or server runtime.
