# Forever product v25: authenticated terrain with declared partial Map

Product v25 is an additive Linux product profile for build 70450. It keeps the
v24 Db2|Terrain component mask **9**, `wowdbdefs-70450-v1`, offline local Rust
acquisition, resident storage, one `Map` DB2 output, and the v20 terrain
selection of `all-available-maps` or one `map:<id>`. It adds the canonical plan
gate `partial_encrypted_sections=declared-skip`. Separately gated later
revisions also admit that plan field; their manifest scopes remain distinct.
Keys, scratch, navigation, map-domain selection and off-mesh connections are
refused. The exact surface is in
[`forever-product-contract-v25.json`](../contracts/forever-product-contract-v25.json).

For the selected build-70450 `Map`, the authenticated available-source reader
checks the full encoded key and every frame checksum. Readable N/Z frames are
decoded normally. Each valid E wrapper must use cipher `S`; its nonce and
ciphertext are not retained or decrypted. The opt-in zero-filled address space
is passed to `forever_db2::availability::read_map_70450`, which skips only
sections declared by the Map WDC5 TACT metadata, following the
TrinityCore-style `zerofillEncryptedParts=true` / `HandleEncryptedSection →
Skip` policy. Extraction fails unless the E-frame count equals the
`tact_declared` section count and the available Map projection validates.

The package records deterministic redacted global declarations containing only
`lookup`, `frame_ordinal`, and `decoded_size`. Offline verification re-reads the
sealed `Map.db2`, derives the `tact_declared` count again and checks it against
the manifest. This records skipped encrypted sections; it does not claim
decryption, complete source decode, whole-client Map coverage or whole-client
terrain coverage. The existing v20 terrain producer, coverage v3 path and v24
package resource limits remain unchanged.

## Real acceptance — 2026-10-24

Both real esES runs used producer SHA-256
`3c214d49f33bc703460b55ef56a4b77787cc074cf556cca35f3b504d950f1767`, HEAD
`365221fe6f22c66c95d47cfa4c67b876cd719648` and pre-documentation worktree diff
SHA-256
`6b4f08c410bd1a74e84e0897484375c4e8a5483af1beb9f90fcb0c1f444f18cd`.
Their software qualification passed 4,023 workspace tests with 26 explicit
ignores, 456 contract tests, Clippy and formatting. No network attempt was
recorded; client metadata, producer and plan remained unchanged. Each run used
900-second, 3-GiB, one-CPU and 64-task limits.

The bounded `v25-map48-partial-20261024T103316Z-1943267` run selected `map:48`.
It passed extract, offline verify and verify-links in 33 seconds, publishing
three artifacts / 15,344 bytes: one partial `Map` and two terrain artifacts.
Inventory SHA-256 is
`a6dd9dedd49ca945d456c3f474145b3cb9e8da2b7dc575d4c0648f0d69f7566d`.
The redacted report `v25-map48-partial-acceptance-20261024T103316Z-1943267.json`
has SHA-256
`52bcb95f42713c21ed740ea778fe50e0659625fcd261863d7c13cfcc3013d9ea`.

The `v25-all-available-20261024T103622Z-1944471` run passed all three operations
in 357 seconds. It published 3,395 artifacts / 215,670,284 bytes: one partial
`Map`, 3,322 MAPS tiles, 71 tilelists and one coverage-v3 file. Inventory
SHA-256 is
`b36f7e063fc65a32acfdccd6ebfd23e8b8ce8b9efbfd265da0a469c7e0d551c9`.
An independent local rehash matched every artifact. Its redacted report
`v25-all-available-acceptance-20261024T103622Z-1944471.json` has SHA-256
`240d9e57b318b0448e2c6da1bf42d5504693403cc14fa3b76ac7d3f6a1586a0b`.
The partial `Map` retains three declared frames / 575 zero-filled bytes in both
runs; no encrypted records were decoded.

## Qualification boundary

These runs accept the selected partial Map and available terrain only. Complete
decoded DB2 content, complete or whole-client Map coverage, full-world terrain,
TrinityCore format parity, RustyCore runtime acceptance, Windows and release
acceptance remain unqualified. V20, v23 and v24 evidence is not transferred to
v25, and v25 evidence is not transferred to separately gated later revisions.
