# Forever product v21 — build-70450 map-domain navigation

Product v21 is a Linux-only Rust-local profile for `wow_classic_beta` build
`70450`. Its publication profile is `Build70450NavigationV1`, its DB2 catalog
is `wowdbdefs-70450-v1`, and its exact component mask is
`Db2 | Terrain | Collision | Navigation` (`57`). The 70450 package context
admits only masks `1`, `7`, `8`, `16`, and `57`, corresponding to the separately
implemented 70450 revisions. Windows product support remains deferred.

## Canonical plan

The plan uses ASCII TSV v21, build 70450, `enUS`, contiguous indexes,
`header-50893-v2`, disabled decryption, and Linux leased-disk collision storage
with explicit raw, WDT, cache, placement, VMO, map, and output budgets. Keys are
refused. The DB2 list must include `Map` (`1349477`) and `LiquidType`
(`1371380`). Terrain and navigation must select the same map-domain ID.

The domain profile is `map-and-ancestors-v1`, with gameobject scope
`global-startup-v1` and placement IDs `selected-closure-serial-ids-v1`. When
navigation is selected, plans carry the `inline-f32-bits-v1` off-mesh profile;
each off-mesh record must name the selected map. The canonical map-2 plan and a
map-2 plan with one off-mesh record are in
`tests/contracts/fixtures/forever-product-v21/`.

## Complete input policy

Domain derivation reads the complete build-70450 Map DB2 through the separately
pinned VMap, terrain, and navigation projections. It validates the selected map
and ancestor closure, then requires those projections to agree on map identity,
WDT file IDs, and effective parents. It does not use available-section or
partial Map readers. Encrypted, unavailable, unknown-schema, structurally
invalid, or disagreeing inputs are terminal before output package creation.
`LiquidType` is also read and validated through the complete 70450 reader.

The shared complete Terrain runner now dispatches `Map`, `LiquidMaterial`,
`LiquidObject`, and `LiquidType` through their 70450 catalogue readers; build
70450 uses the explicit 70450 tilelist and ADT conversion paths. Navigation
loads build-70450 MAPS through `verify_70450`, and verification checks the
build-specific terrain and tilelist formats as well as global/tile MMap framing.

## Synthetic evidence and real-run boundary

The synthetic end-to-end test executes the v21 extraction path, producing DB2,
MAPS and tilelist, VMap components, and global plus tile MMaps. It checks
container versions, headers, framing, and package verification. These are
authored fixtures only: there is no real-client navigation evidence, no
RustyCore runtime acceptance, and no full-world or release claim.

A real run requires the complete build-70450 Map table, terrain sources and VMap
inputs. The target Map table is currently encrypted, while v21 disables
decryption and refuses keys; the required VMap inputs are also unavailable.
Therefore v21 is admitted for the implemented gates but is currently
**unrunnable against the real target client**. Do not treat plan inspection,
synthetic extraction, or package verification as evidence of real navigation
compatibility.

## Synthetic checks

The Linux CI step runs the v21 Python contract and the synthetic CLI tests; the
workspace suite also runs the complete synthetic extraction test. No client
paths, key material, or network access are used by these checks.

```bash
source .opencode/environment.sh
cargo test --offline --locked --no-default-features -p forever-extractor \
  --lib driver::v21_navigation_tests -- --test-threads=1
python3 -B -m unittest discover -s tests/contracts \
  -p test_forever_product_v21_contract.py
```

The plan is not a real extraction recipe until the encrypted Map table and
complete VMap inputs are available through the keyless local readers.
