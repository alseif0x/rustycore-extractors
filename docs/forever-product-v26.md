# Forever product v26: DB2-only with declared partial encrypted sections

Product v26 selects the preliminary Linux build-70450 DB2-only profile,
`Build70450Db2PartialV1`, bound to `wowdbdefs-70450-v1`, component mask `1`,
resident storage and disabled decryption. It accepts one through 1,161 exact
catalog table/FileDataID pairs. Terrain, navigation, map-domain selection and
off-mesh are absent; packing is enabled, and the DB2 option flags/slopes retain
the fixed values in the plan contract. The machine-readable contract is
[`forever-product-contract-v26.json`](../contracts/forever-product-contract-v26.json).

## Pinned reference

The source reference is `advocaite/TrinityCore`, branch `forever`, commit
`367ac384fa207c8f923782634a5e19d8bb141d68`, product `wow_classic_beta`. Its
recorded build scope is `1.60.1.70338` primary and `1.60.1.70450` preliminary;
license metadata is GPL-2.0 with GPL-2.0-or-later headers. As recorded in
`doc/Classic70450.md`, 70450 data is unchanged from 70425. No 70450 auth key is
used. This reference pin is not a comparison: no comparison was performed, and
no 70338, 70425 or 3.4.3 acceptance transfers to v26.

## Driver and offline verification

The v26 driver reads every selected entry through the build-70450 disabled-key
available-source route. After the existing authentication and resource gates,
it zero-fills unavailable encrypted frames and requires bounded
`read_structure_70450` WDC5 validation. A table with TACT-declared sections gets
deterministic redacted declarations in the package manifest under its exact
`dbc/<locale>/<table-name>.db2` path; a complete table gets no partial-section
declaration. Limits remain 16 sections per table and 18,576 total. Framing,
zero-fill, source-authentication and size failures are fatal.

Offline `verify` checks each selected DB2 artifact's WDC5 framing and zero-fill,
then matches declarations to the exact table path and ordered TACT payload
sizes. It rejects global v25-style declarations, missing or extra per-table
declarations, declared files absent from the package, and missing selected
DB2 artifacts. `verify-links` is unchanged.

Synthetic tests exercise a two-table package (one partial, one complete) and
negative cases for a non-zero TACT payload, missing/extra declarations, a
declared DB2 file absent from the package, and a missing selected DB2 file.
These are synthetic driver/package and offline-verification checks only; they
do not use real client data.

## Compatibility and qualification boundary

V16's disabled-key DB2-only contract and v25's DB2+terrain partial-Map contract
retain their own version/profile identities and canonical grammars. No v16 or
v25 acceptance is transferred to v26. This preliminary DB2-only profile does
not decrypt encrypted sections, establish complete DB2 or client coverage,
prove format parity, establish RustyCore runtime compatibility, or qualify
Windows or a release. No real extraction, comparison, runtime validation or
release acceptance is claimed.
