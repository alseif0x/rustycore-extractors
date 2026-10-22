# Forever product v25: authenticated terrain with declared partial Map

Product v25 is an additive Linux software profile for build 70450. It keeps the
v24 Db2|Terrain component mask **9**, `wowdbdefs-70450-v1`, offline local Rust
acquisition, resident storage, one `Map` DB2 output, and the v20 terrain
selection of `all-available-maps` or one `map:<id>`. It adds the canonical plan
gate `partial_encrypted_sections=declared-skip`; no other plan revision admits
that field. Keys, scratch, navigation, map-domain selection and off-mesh
connections are refused. The exact surface is in
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

The package records deterministic redacted declarations containing only
`lookup`, `frame_ordinal`, and `decoded_size`. Offline verification re-reads the
sealed `Map.db2`, derives the `tact_declared` count again and checks it against
the manifest. This records skipped encrypted sections; it does not claim
decryption, complete source decode, whole-client Map coverage or whole-client
terrain coverage. The existing v20 terrain producer, coverage v3 path and v24
package resource limits remain unchanged.

### Qualification boundary

This revision is software scope only until its synthetic lifecycle and package
verification gates pass. The assigned validation uses synthetic BLTE/WDC5 data;
no keys, network access or real-client extraction are part of this change.
Real extraction, complete or whole-client Map coverage, full-world terrain,
TrinityCore format parity, RustyCore runtime acceptance, Windows and release
acceptance remain unqualified. V20, v23 and v24 evidence is not transferred to
v25.
