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
it zero-fills unavailable encrypted frames. A table with partial frames requires
bounded `read_structure_70450_for_table` WDC5 validation and gets deterministic
redacted declarations under its exact `dbc/<locale>/<table-name>.db2` path. A
complete table has no declaration, skips the partial structural reader and uses
Metadata/intrinsic validation during offline verification.

Structural validation admits 16 total WDC5 sections per table, except `Item`,
which admits 17 total sections: one ordinary section plus up to 16 TACT-declared
sections. E-frame declarations use the existing CASC `max_blte_frames` limit of
65,536 per table and 18,576 total. One or more E frames must exactly tile each
TACT extent. The default structural value budget remains 1,000,000. `ItemSparse`
admits its exact 19,263-record * 68-field shape (1,309,884 values); `SpellEffect`
admits its exact 44,027-record * 29-field shape (1,276,783 values). One additional
record in either table rejects, and every other table retains the default. Other
structural ceilings are unchanged. Framing, zero-fill, source-authentication and
size failures are fatal.

Offline `verify` checks each selected DB2 artifact's WDC5 framing and zero-fill,
then matches declarations to the exact table path and ordered TACT payload
extents. One or more `decoded_size` values must sum exactly to each extent;
missing, crossing or extra frames reject. It also rejects global v25-style
declarations, missing or extra per-table declarations, declared files absent
from the package, and missing selected DB2 artifacts. `verify-links` is unchanged.

Synthetic tests exercise a two-table package (one partial, one complete) and
negative cases for a non-zero TACT payload, missing/extra declarations, a
declared DB2 file absent from the package, and a missing selected DB2 file.
These synthetic driver/package and offline-verification checks do not use real
client data.

## Bounded real five-table acceptance — 2026-10-23

The retained `v26-five-tables-verified-20261023T022644Z-35eff781` run selected
five esES tables offline with disabled decryption. Extraction exited zero with
`selected_producers_complete`; separate offline verification exited zero with a
verified package of five files / 49,533 bytes and inventory SHA-256
`9bd9f8c799f361a58a388a356a95398f3e8d38fc9d5cb5dfe5d69778e9089e0e`.
`Achievement.db2` has the sole partial-section declaration; the other four
selected tables are complete. The redacted report records no network attempt
and unchanged client metadata.

The final failed five-table attempt used the same plan SHA-256
`e34c66a671bc5217f4b6cd0a62f66a30b73bd0e909e118af74c830d993ce320d` as the
successful run. That failure and a separate ActionBarGroup-only failure record
`record_data_bounds`; an earlier five-table verification exit 9 also remains
retained. None of those failed verifications is an accepted prefix.

## Five-table closeout — 2026-10-24

The then-current debug executable's SHA-256
`2a349ad61751c42ef417f4e4272021bea09ce254c370d9495c2f384d635e34ff` exactly
matched the retained real-run producer. Plan-only `inspect` and offline
`verify-links` both exited zero in the separate
`v26-current-binding-20261023T230433Z-1634544` closeout; the package remained
verified with the same inventory. Workspace all-target tests passed 4,004 with
26 explicit ignores, all 456 contract tests passed, and Clippy and formatting
passed. These historical software results cover that pre-documentation source
delta.

The redacted external addendum
`v26-db2-current-binding-20261023T230433Z-1634544.json`, SHA-256
`37d55b708115e31fc06de182bdf189d21f8ca86de1645ea49623bf868acf4351`, binds
that source, plan, producer, successful run, offline closeout and retained
failures. No raw client data is included in that report.

## Selected-catalog artifact acceptance — 2026-10-24

The structural corrections admit `Item`'s observed 17-section shape, exact
multi-frame TACT coverage for `ItemSparse`, and the exact declared value
budgets documented above without raising the general section or value ceilings.
Bounded real runs for all three tables passed extraction, offline verification
and `verify-links` before the complete-catalog attempt. `SpellEffect` also
exercised the admitted parent-lookup structure. Its redacted report is
`v26-spelleffect-acceptance-20261024T101802Z-1929806.json`, SHA-256
`a014070e0c9444ff8784ea3d47232b38fb7fe774126e1d5f61d9e49cef7b5438`.

Run `v26-full-after-spelleffect-20261024T102054Z-1936654` passed `extract`,
`verify` and `verify-links` for the complete 1,161-entry selected esES catalog.
It published 1,161 artifacts / 55,733,168 bytes: 1,055 complete tables and 106
explicitly partial tables, with 478 declared zero-filled frames / 350,009
bytes. Inventory SHA-256 is
`9c001a3ab0047482cce5ccc889e7af0794499a650a76b55e9a33d6d9adce26a8`.
An independent local rehash matched every artifact and every expected plan
path. The three operations exited zero; no network attempt was recorded, and
client metadata, the producer and plan remained unchanged. The persistent run
used 900-second, 3-GiB, one-CPU and 64-task limits and completed in 102 seconds.

Producer SHA-256 is
`3c214d49f33bc703460b55ef56a4b77787cc074cf556cca35f3b504d950f1767`; plan
SHA-256 is
`8ba2aed1f9f7e701d6b9ddd6a7f72bc8438b00eeff040e694b2535ac2a71a1e6`.
The pre-documentation worktree diff SHA-256 is
`4a0b218a3c8923888f00d410cc2c49189a6d2a7ea11b6d66be9ae4665de82ff2`.
Workspace all-target tests passed 4,023 with 26 explicit ignores, all 456
contract tests passed, and Clippy and formatting passed. The redacted report
`v26-full-db2-catalog-acceptance-20261024T102054Z-1936654.json` has SHA-256
`a1b84ce4c1e19de2c76f0f3d0419e66ea438b3d43661c5d0751a1298aa271f74`.

The earlier complete-catalog attempts remain retained failures at `Item`
(475-table prefix), `ItemSparse` (547-table prefix) and `SpellEffect`
(905-table prefix). None is an accepted prefix or part of this result.

## Compatibility and qualification boundary

V16's disabled-key DB2-only contract and v25's DB2+terrain partial-Map contract
retain their own version/profile identities and canonical grammars. No v16 or
v25 acceptance is transferred to v26. This preliminary DB2-only profile does
not decrypt encrypted sections or establish complete decoded DB2 content,
whole-client coverage, format parity, RustyCore runtime compatibility, Windows
support or a release. Real extraction now covers all 1,161 selected catalog
artifacts, including the declared partial tables; comparison, runtime
validation, complete geometry/world, a new distribution and release acceptance
remain separate open gates.
