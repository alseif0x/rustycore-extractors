# Forever product v17: Linux build 70450 DB2-only with supplied keys

Authored 2026-10-18. Revision 17 is an explicit, bounded Linux product profile
for `wow_classic_beta` build `70450` (`1.60.1.70450`). It exists to read
frame-encrypted build-70450 DB2 sources with caller-supplied
`provided-salsa20-v1` key provenance. It binds `wowdbdefs-70450-v1` and
publishes only the nonempty set of exact DB2 names/FileDataIDs in the canonical
plan.

## Identity and scope

| Field | v17 contract |
| --- | --- |
| Plan revision / contract | `17` / `forever-product-v17` |
| Build / schema catalog | `70450` / `wowdbdefs-70450-v1` |
| Component | `db2` only (mask `1`) |
| Storage | Resident |
| Decryption | `provided-salsa20-v1`; `--keys PATH` required for client-side `extract`, `resume` and `inspect --client` |
| Platform / backend | Linux / Rust-local |
| Map domain, terrain, navigation, offmesh | None; offmesh field must be absent |
| Commands | `inspect`, `extract`, `resume`, `verify`, `verify-links` |

The key file is read only from the explicit caller-supplied path, using the
existing `forever-tact-keys\t1` parser and keys-enabled BLTE decoder. A key list
is caller-provided input, not discovery, built-in key material, or network
authorization. Key names, values, list counts, hashes and input paths do not
enter plans, package context, manifests, diagnostics, logs or errors. The
complete canonical plan binds only the provided-key capability, not the
contents of a key list. `verify` and `verify-links` validate published
artifacts offline and do not accept keys.

Each selected table is mandatory and follows the existing complete-source
route: all BLTE frames must decode with the supplied keyring and the complete
content CKey must validate before catalog validation and byte-preserving
publication under `dbc/<locale>/<table-name>.db2`. Existing 16 MiB per-file,
16 GiB cumulative DB2, guard and allocation limits remain in force. A missing
or mismatched key remains fatal as `local_store_no_verified_encoding`; no
partial table or encrypted-source omission is published.

A public key list, a matching key name, or a partial key match never proves
complete decryption. Only a successful full-frame decode and complete CKey
validation for every selected DB2 source establish that those selected sources
were fully decoded. The key list does not prove coverage of unselected tables
or all possible encodings.

## What v17 can and cannot establish

`inspect` without a client reports only the plan. Client-side `inspect`,
`extract` and `resume` require `--keys PATH`; disabled-key revisions 14, 15 and
16 continue to reject keys. V17 can publish only the exact selected DB2 set and
does not establish that all 1,161 catalog tables are selected or readable.

It does not produce or establish GameTables, cameras, terrain, collision/VMaps,
navigation/MMaps, geometry, full-world coverage, runtime compatibility or
Windows support. `full_extractor_complete` remains false. No real-client,
runtime or release acceptance is claimed.

Revision 16 remains the separate disabled-key build-70450 DB2-only profile.
Revision 15 remains collision-only. V17 neither changes those profiles nor
adds build 70450 to global/native admission.
