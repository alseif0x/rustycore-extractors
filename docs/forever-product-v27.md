# Forever product v27: selected DB2 catalog and authenticated terrain

Product v27 is an additive Linux build-70450 profile,
`Build70450PartialCatalogTerrainV1`. It combines the selected DB2 catalog of
v26 with the authenticated available-Map terrain route of v25. Its component
mask is **9** (`db2,maps`), its schema catalog is `wowdbdefs-70450-v1`, storage
is resident, decryption is disabled and packing is mandatory. Keys, scratch,
navigation, map-domain selection and off-mesh connections are refused.

A canonical plan selects one through 1,161 exact catalog name/FileDataID pairs,
including exactly one `Map`. Terrain is either `all-available-maps` or one
`map:<id>`. The plan requires `partial_encrypted_sections=declared-skip`; that
plan field is distinct from the package manifest fields. The machine-readable
contract is
[`forever-product-contract-v27.json`](../contracts/forever-product-contract-v27.json).

## Acquisition and per-table declarations

Every selected table, including `Map`, uses the v26 authenticated
available-source route. Readable frames are decoded normally. Unavailable
encrypted frames are zero-filled only after the existing source and checksum
checks; their ciphertext and keys are not retained or recovered. Bounded WDC5
validation requires every declared TACT extent to be covered exactly by one or
more frames, without gaps, crossings or extra frames.

The manifest uses only `partial_encrypted_tables`, including for a partial
`Map`. Each declaration contains only a redacted lookup, frame ordinal and
decoded size. The v25 global `partial_encrypted_sections` manifest declaration
is forbidden in v27, although the same-named plan gate remains mandatory. A
complete table has no partial declaration.

`Map` additionally passes `terrain_v20::authenticate_map`, which reads and
materializes the available build-70450 Map projection. This independently
re-derives the selected identities needed by terrain coverage v3 from the
sealed artifact. It does not decrypt omitted sections or establish whole-client
Map coverage. Terrain retains the v20 MissingFile-only WDT/root-ADT omission
policy and its existing source reads; avoiding duplicate artifacts does not
imply a single CASC read.

The DB2 limits remain 16 MiB per table, 16 GiB total, the existing CASC frame
ceiling per table and 18,576 declarations across all selected tables including
`Map`. The v27 package limit is 80 GiB: the existing 64-GiB terrain budget plus
16 GiB DB2, added with checked arithmetic. Older profile limits do not change.

## Resume and offline verification

Resume re-admits all selected DB2 sources before the ordinary component cycle,
even when DB2 was already sealed. This reconstructs every per-table declaration
before final sealing; identical artifacts and declarations are idempotent and
conflicts remain fatal. Terrain retains the authenticated v3 replay behavior.

Offline verification checks package integrity and the exact selected inventory.
Complete DB2 artifacts use the existing intrinsic/metadata route; partial ones
use structural zero-fill validation and exact multi-frame declaration sums.
`Map` is also re-read through the typed available-map projection and bound to
coverage v3. Any global declaration, missing or extra table declaration,
incorrect coverage or inconsistent terrain identity rejects verification.
`verify-links` performs the same v27 checks; it adds no navigation or external
package-link claim.

Synthetic tests cover canonical plans, catalog and option boundaries, Map
multi-frame declarations, the combined package budget, extract/resume,
verify/verify-links, reconstruction with DB2 sealed but no final manifest, and
rejection of global, missing, inconsistent and corrupt declarations. These
checks use synthetic fixtures only.

## Real selected-catalog and available-terrain acceptance — 2026-10-24

The matching producer SHA-256
`7045052db299b8d7dbf7b43673f6edfb9ca549fa8ee8012dd6452702dba0d6b8`
passed real esES extract, offline verify and verify-links for both selections.
The source record binds HEAD `365221fe6f22c66c95d47cfa4c67b876cd719648`,
pre-documentation worktree diff SHA-256
`513b42c06d94709daf34c194348cf4b6a7bdfe44bb998264bcc43a8f1bb8f791`
and every newly added source file by hash. Software qualification passed 4,044
workspace tests with 26 explicit ignores, 464 contract tests, Clippy and
formatting. No network attempt was recorded; client metadata, producer and plan
remained stable.

The bounded `v27-catalog-map48-20261024T111647Z-1965505` run completed in 104
seconds and published 1,163 artifacts / 55,737,931 bytes. All 1,161 DB2 payloads
match v26 and the one non-coverage terrain payload matches v25. The selection
has one tilelist and one coverage file, but zero converted MAPS tiles; it is not
evidence of nonempty terrain. Inventory SHA-256 is
`b90ca193ae7b3b909a27d99094e5ce4602d75a27c5c2ed988992bc87b5b7bee9`.
The redacted report `v27-catalog-map48-acceptance-20261024T111647Z-1965505.json`
has SHA-256
`bfd0db301e521dd330a1110094b4c319639b184fe44822b7616bb84788c15ace`.

The expanded `v27-catalog-all-available-20261024T113558Z-1989523` run completed
in 445 seconds and published 4,555 artifacts / 271,392,871 bytes: 1,161 DB2
artifacts, 3,322 MAPS tiles, 71 tilelists and one coverage-v3 file. All 1,161
DB2 payloads match v26 and all 3,393 non-coverage terrain payloads match v25.
Inventory SHA-256 is
`8ba5d6866ce779a97b410c82283e1c89d213a5e414d75b78d508b3ee4ffe6157`.
Its redacted report
`v27-catalog-all-available-acceptance-20261024T113558Z-1989523.json` has SHA-256
`de15dce370d7b9526f66e7a11c3f84dcff077726763a816a81480c66499eb3b7`.
Every artifact hash was independently rechecked.

Both packages retain 1,055 complete and 106 declared-partial tables, with 478
zero-filled frames / 350,009 bytes across all tables. `Map` has three frames /
575 bytes exclusively in its per-table declaration; global declarations are
absent. These runs used 900-second, 3-GiB, one-CPU and 64-task limits.

## Qualification boundary

This accepts the selected DB2 artifact catalog and available terrain, not
complete decoded DB2 content, whole-client coverage, complete
geometry/full-world, navigation, RustyCore runtime, format parity, a new
distribution, Windows or publication. `full_extractor_complete` remains false.
