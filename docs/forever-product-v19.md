# Forever product v19: Linux build-70450 DB2, GameTables and cameras

Authored 2026-10-18. Revision 19 mirrors the bounded Linux build-70374
DB2+GameTables+cameras product profile, with the build-70450 catalog identities.
It selects only the exact DB2 list in its plan, all 20 fixed GameTables, and the
camera FileDataIDs discovered from the selected `CinematicCamera.db2` source.
The profile uses disabled decryption: caller keys are refused.

## Identity and scope

| Field | v19 contract |
| --- | --- |
| Revision / contract / profile | `19` / `forever-product-v19` / `Build70450Db2GtCamerasV1` |
| Build / DB2 catalog | `70450` / `wowdbdefs-70450-v1` |
| Component mask | DB2 + GameTables + cameras (`7`) only |
| Storage / platform / backend | Resident / Linux / Rust-local |
| Decryption | Disabled; `--keys PATH` is refused |
| Terrain / navigation / map domain / offmesh | None; map-domain and offmesh fields are absent |
| Commands | `inspect`, `extract`, `resume`, `verify`, `verify-links` |
| Scratch | Resident profile: `--scratch` is forbidden |

The DB2 list must be nonempty, sorted, unique and resolved against the
70450 catalog. It must include `CinematicCamera` (FileDataID `1294214`). The
existing DB2 rule still rejects cameras without that DB2 component/source.
All 20 GameTable files use the existing `gametables::catalog::Table::ALL`
readers. Camera discovery uses the existing direct-row `cameras::discover`
reader and the source is sealed before discovered camera files are published.
No new DB2, GameTable or camera reader is introduced. Generic/native admission
for build 70450 remains closed.

## What v19 can and cannot establish

The product validates and publishes the exact selected DB2 files, all 20 fixed
GameTables, and raw camera files identified by the selected
`CinematicCamera.db2`. Bounded metadata preflight and per-file/cumulative byte
limits remain in force. `verify` and `verify-links` are offline and reject
`--keys`.

V19 does not produce terrain, VMaps, MMaps, or any other component; it does not
establish complete geometry, full-world coverage, runtime compatibility, or
Windows support. `full_extractor_complete` remains false. No real-client,
runtime, or release acceptance is claimed.

Revisions 14 through 18 retain their existing build, component, key and
verification policies. Revision 19 adds only the explicit Linux build-70450
DB2+GameTables+cameras gate.
