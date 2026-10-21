# Forever product v24: sealed Map and available terrain

Product v24 is an additive Linux software profile for build 70450. Its
canonical plan selects exactly two components: Db2 bit 1 and Terrain bit 8,
combined mask **9**. It requires the `wowdbdefs-70450-v1` catalog, disabled
decryption, resident storage, one `Map` DB2 output, and terrain selection of
`all-available-maps` or one `map:<id>`. Keys, scratch, navigation, map-domain
selection and off-mesh connections are refused. The exact plan and acceptance
surface are recorded in
[`forever-product-contract-v24.json`](../contracts/forever-product-contract-v24.json).

Extraction seals the `dbc/<locale>/Map.db2` artifact and runs the existing v20
available-map terrain producer in the same package. Resume preserves the DB2
component and replays terrain using the shared package lifecycle. WDT/root-ADT
omissions and terrain conversion outcomes retain v20's existing
MissingFile-only policy and producer-reported coverage facts.

Offline verification re-reads the sealed Map artifact using
`read_map_70450().materialize()`. It independently compares the Map source
section/count facts, materialized row count, and ordered selected Map IDs,
WDT FileDataIDs, ordinals, and terrain outcomes against coverage v3. The CLI reports
`coverage_authentication=independently_derived`. This applies to the stored Map
projection and its alignment with coverage; WDT dispositions, tile conversion
results, source authentication flags, and omissions remain producer-reported.
The profile does not claim complete or whole-client Map coverage.

The v24 evidence here is limited to implementation and synthetic/software
checks. No real-client v24 extraction, full-world coverage, MAPS format parity,
RustyCore runtime acceptance, Windows execution, or release acceptance is
established. The build-70450 v20 and v23 acceptance scopes remain separate and
are not transferred to v24.
