# Forever product v16: Linux build 70450 DB2-only

Authored 2026-10-18. Revision 16 is an explicit, bounded Linux product profile
for `wow_classic_beta` build `70450` (`1.60.1.70450`). It binds the
`wowdbdefs-70450-v1` schema catalog and publishes only the DB2 entries named in
the canonical plan. It mirrors the build-70374 DB2-only scope without changing
the separate build-70450 v15 compatible-VMaps profile.

## Identity and admitted scope

| Field | v16 contract |
| --- | --- |
| Plan revision / contract | `16` / `forever-product-v16` |
| Build / schema catalog | `70450` / `wowdbdefs-70450-v1` |
| Component | `db2` only (mask `1`) |
| Storage | Resident |
| Decryption | Disabled; `--keys` is forbidden |
| Platform / backend | Linux / Rust-local |
| Map domain, terrain, navigation, offmesh | None; offmesh field must be absent |
| Commands | `inspect`, `extract`, `resume`, `verify`, `verify-links` |

The plan's repeatable DB2 tail names exact catalog table names and FileDataIDs,
in alphabetical table-name order with unique IDs. It must be nonempty and may
contain at most 1,161 entries. A selected table is mandatory: absence, failed
decode, schema/framing failure, or a resource-limit failure remains fatal. The
local admission route is `admit_db2_70450` followed by
`LocalStore::open_db2_70450`; this does not add build 70450 to the global
`SUPPORTED_CLIENTS` list.

Selected files are written under the existing DB2 artifact path
`dbc/<locale>/<table-name>.db2`. Each source is read through LocalStore's strict
full-file decode route with keys disabled, then checked using the
plan-bound 70450 catalog and preserved as source bytes. The normal bounded DB2
limits are 16 MiB per file and 16 GiB total. Existing catalog-aware section and
string validation rules still apply; this profile introduces no encrypted,
missing-file, or available-prefix fallback.

In particular, a frame-encrypted DB2 source that cannot be decoded without a
key fails with the existing `local_store_no_verified_encoding` code. Other
source errors remain fatal under their existing codes. This encrypted-source
failure must never be relaxed into an omission to produce a successful
component. The plan cannot supply keys, discover keys, or use network access.

## What v16 can and cannot establish

`inspect` can show the explicit plan identity without opening a client.
`extract` and `resume` can acquire and publish only the exact selected DB2
tables; `verify` and `verify-links` validate the selected package artifacts and
intrinsic DB2 formats offline. Keys are forbidden for all v16 client commands,
including both verification commands.

This revision does not establish that all 1,161 catalog tables are selected or
available, nor that every build-70450 DB2 encoding variant is supported. It
does not produce or establish GameTables, cameras, terrain, collision/VMaps,
navigation/MMaps, geometry, full-world coverage, client runtime compatibility,
or Windows support. `full_extractor_complete` remains false. It makes no
real-client extraction or runtime-acceptance claim.

V15 remains the separate build-70450 collision-only profile:
`Build70450CompatibleCollisionV1`, component mask 16, and
`linux70450-compatible-gnu15-vmap-product-v1`. V16 neither selects nor changes
that profile.
