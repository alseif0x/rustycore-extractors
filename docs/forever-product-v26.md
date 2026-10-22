# Forever product v26: DB2-only with declared partial encrypted sections

Product v26 adds a separate Linux build-70450 DB2-only plan profile. It binds
`Build70450Db2PartialV1` to `wowdbdefs-70450-v1`, component mask `1`, resident
storage, disabled decryption, and the canonical required field
`partial_encrypted_sections=declared-skip`. It accepts one through 1,161 exact
catalog table/FileDataID pairs. Terrain, navigation, map-domain selection and
off-mesh are absent; packing is enabled, and the DB2 option flags/slopes retain
the fixed values in the plan contract. The machine-readable contract is
[`forever-product-contract-v26.json`](../contracts/forever-product-contract-v26.json).

The synthetic plan fixtures cover both one and multiple selected DB2 entries.
For the later acquisition/package phase, partial-section declarations are
scoped to their containing selected DB2 table and contain only a redacted
lookup, frame ordinal and decoded size. This phase does not implement that
manifest field, extraction, or verification behavior. A non-zero payload in a
TACT-declared zero-filled extent now has the distinct `product_partial_zerofill`
diagnostic rather than the generic unsupported-record error.

### Compatibility and qualification boundary

V16's disabled-key DB2-only contract and v25's DB2+terrain partial-Map contract
retain their own version/profile identities and canonical grammars. No v16 or
v25 acceptance is transferred to v26. This plan/contract increment does not
decrypt encrypted sections, establish complete DB2 or client coverage, prove
format parity, establish RustyCore runtime compatibility, or qualify Windows or
a release. Product extraction and per-entry package/verification gates remain
for later phases.
