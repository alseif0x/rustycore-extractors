# Forever product v18: keyed Linux build-70450 compatible collision

Authored 2026-10-18. Revision 18 is the keyed counterpart to the disabled-key
build-70450 collision profile in revision 15. It exists to complete the Linux
compatible GNU15-v2 collision component once a valid caller-supplied key source
is available. It uses the already admitted build-70450 keyed LocalStore path;
it does not add build 70450 to generic, native, legacy, or default admission.

## Identity and scope

| Field | v18 contract |
| --- | --- |
| Plan revision / contract | `18` / `forever-product-v18` |
| Publication profile / build | `Build70450CollisionProvidedKeysV1` / `70450` |
| Catalog | `wowdbdefs-70450-v1` |
| Component | `vmaps` / collision mask `16` only |
| Collision profile / storage | `linux70450-compatible-gnu15-vmap-product-v1` / Linux leased disk-world v1 |
| Disk budgets | The same seven downward-only budgets and defaults as revision 15 |
| Decryption | `provided-salsa20-v1`; `--keys PATH` required for client-side inspect, extract and resume |
| Platform / backend | Linux / Rust-local |
| Domain | `all-available-maps` or a canonical `map:<u32>` collision selection |
| DB2, terrain, navigation, offmesh | No DB2 list; terrain/navigation are `none`; offmesh field absent |
| Other options | `pack=true`, big-base-unit/skip-liquids false, slopes 55/55 |

Extract and resume require `--scratch PRIVATE_DIRECTORY` in addition to the
caller key file. The scratch namespace, key source, client and output remain
subject to the same leases, guard checks, protected-root checks and budgets as
the existing disk collision route. A missing or mismatched key for a mandatory
encrypted source is fatal; it cannot produce partial VMaps or turn an encrypted
source into an omission.

The caller key list is explicit input, not key discovery, built-in key
material, or permission to fetch keys. V18 performs no TACT/CDN/network or
launcher access. Key names, values, counts, hashes and input paths are excluded
from plans, package identities, outputs, diagnostics, logs and errors.

## What v18 does not prove

A public key list, a matching key name, or a partial key match never proves
complete decryption. Only successful full-frame decode and complete content
CKey validation prove decryption of that particular source. The v18 coverage
record is producer-reported; it is not an independent client snapshot and does
not prove full encrypted Map coverage or global map completeness.

V18 does not establish DB2, GameTables, cameras, terrain, navigation, complete
world resources, runtime compatibility or Windows support. The
`full_extractor_complete` marker remains `false`. No real-client extraction,
runtime or release acceptance is claimed.

Revision 15 remains the separate disabled-key collision profile. Revisions
14-17 retain their own admission and key policies; v18 changes neither those
profiles nor their contract identities.
