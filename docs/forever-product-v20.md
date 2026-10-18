# Forever product v20 — build-70450 available-map terrain

Product v20 is an explicit Linux-only Rust-local terrain profile for
`wow_classic_beta` build `70450`. Its publication token is
`Build70450CompatibleTerrainV1`; its DB2 catalogue binding is
`wowdbdefs-70450-v1`; and its exact component mask is `Terrain` (`8`). It emits
MAPS tiles, one tilelist per selected materialized Map, and a producer-reported
coverage record.

## Closed plan scope

The plan uses canonical ASCII TSV v20, resident storage and disabled decryption.
Keys are forbidden. The DB2 producer list must be empty: Map and all three
liquid startup tables are acquired by their fixed file data IDs inside the
terrain runner. Terrain selection is `map:<canonical-u32>` or
`all-available-maps`; per-tile selection is forbidden. DB2 output, game tables,
cameras, collision, navigation, offmesh and map-domain selection are outside
this profile. The accepted commands are `inspect`, `extract`, `resume`, `verify`
and Linux-only `verify-links`.

## Available-source and omission policy

The runner uses the build-70450 available-section Map reader and separately
pinned build-70450 liquid readers. Unknown TACT sections remain inaccessible and
are recorded as unknown; unavailable required bytes in an admitted section are
fatal. Map CKey verification and encoded-key authentication remain separate
facts. A partially readable Map must report `full_ckey_verified=false` and
`encoded_key_authenticated=true`; if `available_bytes < declared_bytes`, full
CKey verification must be false. The coverage format contains per-section
facts, not an aggregate complete-Map or whole-world flag.

Only explicit `MissingFile` for a WDT or root ADT is an admitted omission. An
unavailable WDT yields a zero tilelist with MAIN unknown; a known-present root
ADT that is `MissingFile` is omitted from the tilelist and counted. Other
acquisition, integrity, parsing, conversion, resource and publication errors
remain fatal. Output callbacks remain provisional until completion.

The coverage authentication is exactly `producer_reported`, and the contract
sets `complete_global_map_coverage_claim` to `false`. Offline verification checks
package integrity, declared coverage and tile/list associations; it is not an
independent client snapshot.

## Explicit unresolved format assumption

The lower-level build-70450 terrain converter records an explicit assumption
that ADT/MAPS uses the pinned build-70374 wire grammar, with exact build and
format gates. This assumption is not independently proven format parity. The
v20 contract and tests do not claim build-70450 runtime compatibility, real
extraction evidence, full-world coverage or release acceptance.

## Synthetic plans

- `tests/contracts/fixtures/forever-product-v20/map2.tsv` selects Map 2.
- `tests/contracts/fixtures/forever-product-v20/available.tsv` selects all
  available materialized maps.

All v20 tests use authored synthetic sources. No real client, key material or
network access is part of this profile's CI checks.

## Bounded real-run command for when reads resume

This selects one map, does not supply keys or scratch, and writes only beneath
the configured external candidate-run root. It was not executed; real client
reads remain deferred.

```bash
source .opencode/environment.sh
RUN_ID="70450-v20-map2-$(date -u +%Y%m%dT%H%M%SZ)"
cargo run --offline --locked -p forever-extractor --bin forever-extractor -- extract \
  --plan "$PWD/tests/contracts/fixtures/forever-product-v20/map2.tsv" \
  --client "/absolute/path/to/build-70450-client" \
  --output "$RCE_OPENCODE_RUN_ROOT/$RUN_ID"
```
