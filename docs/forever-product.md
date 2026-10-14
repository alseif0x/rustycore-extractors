# Forever product interface: v1–v6 and Linux v8–v11

`forever-extractor` connects the existing Rust-local acquisition, DB2 catalog,
20 game tables, camera discovery, terrain conversion, collision assembly and
sequential navigation producers. The [v1 contract](../contracts/forever-product-contract-v1.json)
uses an explicit canonical TSV plan and the existing [common package](forever-package.md)
final manifest v1. Linux uses internal recovery state v2; the separate
[Windows resident backend](forever-windows-product.md) uses state v3 and retains
its own native acceptance record. Linux x86_64 has local evidence; Windows
distribution and full real extraction remain open. V1–v3 plans use the
legacy pinned DB2 catalog. V4 binds build 70205 plans to the separately pinned
`wowdbdefs-70205-v1` catalog; it does not rewrite older package or plan identities.
V5 binds build 70338 plans to `wowdbdefs-70338-v1` and admits DB2 only; it does
not extend the older component profiles to this build.
The separate [v2 contract](../contracts/forever-product-contract-v2.json) admits
explicitly supplied Salsa20 keys under the bounded profile described below.
The [v6 contract](../contracts/forever-product-contract-v6.json) adds explicit
inline off-mesh navigation connections to the legacy pinned metadata profile
for builds 70170/70205. V6 does not extend build 70338. The separate Linux
[v9 contract](../contracts/forever-product-contract-v9.json) admits exactly
DB2, GT and raw cameras on build 70338; its bounded real acceptance is recorded
below. V5 remains DB2-only, and 70338 geometry remains unaccepted.
The separate Linux [v10 contract](../contracts/forever-product-contract-v10.json)
admits explicit build 70374 DB2-only plans with `wowdbdefs-70374-v1`.
Its bounded TACT acceptance and negative startup diagnostic are recorded below;
70374 geometry remains unaccepted. The current bounded local Linux package is
separately accepted in issue #62; see [distribution evidence](forever-distribution.md).
The separate Linux [v11 contract](../contracts/forever-product-contract-v11.json)
admits explicit 70374 DB2/GT/raw cameras with sealed-source proof. Its fresh
synthetic/native evidence and independently accepted bounded real26 lifecycle
are recorded below. Issue #62 separately accepts the current local Linux bundle;
release and full extraction remain open.

## Build and inspect

```bash
source .opencode/environment.sh
cargo build --offline --locked --release -p forever-extractor
"$CARGO_TARGET_DIR/release/forever-extractor" inspect --plan /absolute/private/plan.tsv
```

No selected file is read by `inspect --plan`. Adding `--client` admits the client
and explicitly specified ROOT/index layouts, builds the local metadata store
and returns its context. This is not an availability survey. `--help` documents
the available commands; every ordinary result/error is path-free JSON.

A bounded DB2/GT/camera example for build 70205/esES follows. Separators are
literal tabs; the file must end with LF. Names and IDs are public pinned metadata.

```text
forever-product	1
build	70205
locale	esES
index	contiguous
root	header-50893-v2
components	db2,gt,cameras
terrain	none
pack	true
navigation	none
big-base-unit	false
skip-liquids	false
maximum-slope	55
non-steep-slope	55
db2	ChrClasses	1361031
db2	ChrRaces	1305311
db2	CinematicCamera	1294214
```

The grammar requires all fixed lines in that order and alphabetically ordered
DB2 entries. Unknown keys, duplicate components/entries, case aliases, leading
zero decimal aliases, missing final LF and physical layout inference reject.
V1–v3 plans are at most 128 KiB / 1,126 DB2 entries. Only catalog-backed complete
record decoding is admitted; this product CLI does not issue metadata-only DB2 success.
Existing component CLIs, schemas and acquisition defaults remain separate.

## Select components and dependencies

Component order is `db2,gt,cameras,maps,vmaps,mmaps`. Select a comma-separated
subsequence explicitly for legacy v1–v6/v8 plans. Linux v9 requires exactly
`db2,gt,cameras`. GT selects all 20 files (14 numeric and six raw-only).
Cameras requires selected DB2 and its `CinematicCamera` entry. Its source remains
in `dbc/<locale>/`; the discovered complete opaque camera files go in `cameras/`.
No M2/waypoint parsing is added.

Terrain/navigation selectors are `none`, `all`, `map:ID`, or `tile:ID:X:Y`, with
X=column/Y=row in 0..63. `pack` controls existing terrain height packing.
Navigation requires terrain **all**, collision, DB2 and explicit `Map` and
`LiquidType` entries; this supplies complete producer-owned local input folders.
No external available-prefix folder is silently selected. Explicit integer slopes
and booleans configure the existing navigation API. Inactive navigation requires
its defaults, 55/55 and false booleans. Off-mesh connections remain explicitly
empty in product v1–v5. V6 supplies the separate capability described below.

Selecting maps/collision enables the existing bounded local ROOT-name capability.
This is a local logical-name index, without a listfile, network or OS-path fallback.
Collision runs the complete existing startup/shared-world assembler, which retains
its bounded in-memory staging policy. Navigation runs fresh sequential tile contexts
against the sealed maps/vmaps folders. Complete empty Map direct rows use the
validated empty pure navigation plan for `all`; explicit Map/Tile requests still
reject. Other empty geometry can legitimately produce globals without tile bodies,
or no navigation outputs, as admitted by the component planner.

## Inline off-mesh navigation (v6)

V6 requires navigation and its existing DB2, terrain and collision dependencies.
It uses `pinned-metadata-v1` exclusively, with build 70170 or 70205. The plan has
`forever-product<TAB>6` and an explicit `decryption` line (`disabled` or
`provided-salsa20-v1`) after `root`. Optional leased collision storage retains
its existing grammar. There is no `schema-catalog` line in this profile.

After `non-steep-slope`, include `offmesh<TAB>inline-f32-bits-v1`, followed by
zero or more connection lines, then the alphabetically ordered DB2 entries:

```text
offmesh-connection<TAB>MAP<TAB>ROW<TAB>COLUMN<TAB>FROM_X_BITS<TAB>FROM_Y_BITS<TAB>FROM_Z_BITS<TAB>TO_X_BITS<TAB>TO_Y_BITS<TAB>TO_Z_BITS<TAB>RADIUS_BITS<TAB>DIRECTION<TAB>AREA<TAB>FLAGS
```

Replace `<TAB>` with literal tabs. Float fields are exactly eight lowercase
hexadecimal digits containing IEEE 754 f32 bits, without a `0x` prefix. Other
fields use canonical unsigned decimal. Coordinates use the original world XYZ
axes; the navigation library performs its existing axis projection. Row/column
are each 0..63, direction is 0 (one way) or 1 (both ways), area is 0..63 and
flags are 0..65535. Endpoints must be finite and radius must be finite and
positive. Coordinate signed zero bits, record order and duplicates are retained.
All records validate before client or output I/O, including unselected records.

The limits are 1,024 connections and the unchanged 128 KiB complete plan budget.
An explicit empty off-mesh profile still selects v6. The complete canonical plan
hash binds every connection bit, order and duplicate to extraction and recovery;
there is no external mutable connection file. Existing component parser, work
and output limits remain in force.

Global MMap files retain the plan's records for that map in source order,
including records for tiles outside an explicit tile selection. Tile generation
selects connections using the existing map/row/column rules. Valid records for
maps that the navigation planner does not emit are unused; this capability does
not discover new maps, tiles or endpoints. A completely empty Map source rejects
nonempty connections. No endpoint snapping or route existence is inferred from
the global record list.

Product verification compares every stored global connection with that map's
exact planned records, including float bits, flags, order and duplicates. The
tile verifier retains its existing intrinsic-format scope; it does not prove
general routing or bind all tile polygons to their source connections. Legacy
profiles continue to reject nonempty global connections. Executed synthetic
acceptance and remaining gates are recorded in the completion runbook.

## Extract, resume and verify

Use absolute private output paths outside Git and the client. The parent directory
must exist; `extract` requires a new destination. Client/Data/archive/config roots
are protected. Inputs are read-only and the only backend is explicitly Rust-local.
There are no network, native, missing-data or build fallbacks. Product v1
retains N/Z decoding; encrypted input requires the explicit v2 capability.

```bash
BIN="$CARGO_TARGET_DIR/release/forever-extractor"
"$BIN" extract --plan /absolute/private/plan.tsv \
  --client '/absolute/client' --output /absolute/private/new-run
"$BIN" verify --plan /absolute/private/plan.tsv \
  --output /absolute/private/new-run
```

Run full/long extraction through a persistent user service or tmux, with commands,
logs and outputs outside the checkout as required by AGENTS.md. The bounded example
is not full world extraction.

Any acquisition, conversion, publication or final identity-check failure retains
the incomplete marker and receipts. Only complete selected producer execution
permits the component seal and final manifest. `resume` explicitly verifies the
same context before mutation:

```bash
"$BIN" resume --plan /absolute/private/plan.tsv \
  --client '/absolute/client' --output /absolute/private/incomplete-run
```

The plan fingerprint covers every canonical byte. The producer fingerprint hashes
the **loaded** Linux `/proc/self/exe`, so a concurrent build cannot substitute its
pathname. Retain that binary for restart. The source fingerprint is the versioned
logical build/config/locale/ROOT/ENCODING collection identity. It is not a physical
archive availability inventory or complete archive snapshot. Each read still
requires complete authenticated content bytes; unchanged metadata guards are checked
before each producer seal and final publication. A recovered missing physical file
of the same logical collection can therefore be read on an explicit new attempt.

Verified finished producers are skipped. Every unfinished producer reruns completely
and must replay all its retained files before sealing. Navigation computation is not
checkpointed between unfinished tiles. Resume refuses a changed context, changed
producer binary, unsupported state or already complete output. Never delete a marker
to make a failed run appear complete.

`verify` holds a shared package lock, validates the entire inventory and uses rooted
read-only descriptors for the bounded artifact checks. It rechecks state/namespace
after callbacks; ordinary `forever-package verify` preserves its prior behavior and
scope. Optional `--expect-context /absolute/private/context.json` independently binds
all fingerprints; that file uses exact `Context::json()` bytes with no final LF.
Fingerprints are integrity identifiers, not signatures/authenticated provenance.

Verification proves **artifact integrity and bounded intrinsic formats**, including
exact selected DB2/GT/camera sets, explicit terrain/navigation selectors, MAPS10
build checks, VMAP_4.E intrinsic framing and per-tree index bounds, and MMAP16/
Detour7 framing. It does not establish full geometry inventories, placement/ancestor
associations, Detour topology, original C++ parity for newly generated worlds, runtime
compatibility, or complete real-client extraction. All reports retain
`full_extractor_complete: false`; preserve the component oracle/reader evidence and
[remaining completion gates](forever-completion.md).

## Explicit supplied keys: product v2

To select v2, change the first line to `forever-product<TAB>2` and insert
`decryption<TAB>provided-salsa20-v1` immediately after `root`, before
`components`. All other lines, dependencies and limits retain their v1 rules.
There is no disabled or implicit v2 profile. V1 rejects the additional line
and any `--keys` option before opening its file.

```bash
"$BIN" inspect --plan /absolute/private/plan-v2.tsv
"$BIN" inspect --plan /absolute/private/plan-v2.tsv \
  --client '/absolute/client' --keys /absolute/private/provided.keys
"$BIN" extract --plan /absolute/private/plan-v2.tsv \
  --client '/absolute/client' --output /absolute/private/new-run-v2 \
  --keys /absolute/private/provided.keys
"$BIN" resume --plan /absolute/private/plan-v2.tsv \
  --client '/absolute/client' --output /absolute/private/incomplete-run-v2 \
  --keys /absolute/private/provided.keys
"$BIN" verify --plan /absolute/private/plan-v2.tsv \
  --output /absolute/private/new-run-v2
```

V2 client inspection, extraction and resume require `--keys`; plan-only
inspection needs neither keys nor client. Verification requires no keys and
rejects the option. Client inspection admits metadata without asserting the
availability of every selected payload or its key. Options/plan errors before
plan admission retain the generic v1 error envelope; admitted v2 operations
use `forever-product-v2`. Successful v2 command summaries include the fixed
`decryption` capability, without key material or metadata about the keyset.

The explicit key file uses the closed [supplied-key format](forever-local-casc.md#explicitly-supplied-salsa20-keys-library-api-2026-10-11):
1..4,096 sorted unique records, 128-bit values and at most 256 KiB. The Linux
loader opens a regular read-only file without following its final symlink or
blocking on a FIFO, bounds allocation/read length and checks file identity
before and after reading. The canonical key source is protected from output
creation/resume, including a source located inside the output. No environment,
client, parent-folder, built-in-key or network search occurs. Key memory is
not guaranteed to be securely erased.

The capability covers one encrypted BLTE E wrapper around N or strict Z,
Salsa20/20 with 128-bit keys, names of 0/8 bytes and IVs of 4/8 bytes. Complete
content MD5 verification is mandatory. This is integrity against the selected
CKey, without a MAC or trusted-provenance claim; it is not general cipher or
encrypted DB2-section support. All ordinary memory and input limits remain.
See the local CASC contract for frame ordinals, ciphertext checks and scratch
accounting.

The plan SHA binds the decryption capability. Key names, values, counts,
file paths and keyset hashes are excluded from the plan, JSON, context,
receipts and manifests. A keyset affects physical availability, so changing
it does not change logical source identity. Each resume must explicitly
admit its supplied file again; a different set may recover the same selected
complete CKey. Sealed components are verified and skipped. This neither
substitutes another logical source nor permits partial content. Existing
index-prefix candidate retry is not full EKey authentication or proof of
general ENCODING-map alternatives. A v1 plan cannot resume or verify a v2
package because its canonical plan fingerprint differs.

Missing/forbidden options produce `product_keys_required` /
`product_keys_forbidden` (exit 2). Static input diagnostics are
`product_keys_input_io`, `product_keys_input_type`, `product_keys_input_limit`,
`product_keys_input_changed` and `product_keys_allocation`; closed parser
diagnostics retain `tact_keys_format`, `tact_keys_order`, `tact_keys_limit` and
`tact_keys_allocation`. Input/parser/acquisition failures exit 9; an incomplete
package exits 8. These reports include no key or path details.

## Executed v1 integration evidence — 2026-10-11

Debug/release each pass 2,023 library tests, four example tests and three binary
tests (2,030 Rust tests), 72 contract checks, all-target Clippy, and 17 explicit
synthetic CLI scenarios. The last scenario supplements the original 16-case run
using the exact archived binaries. Six actual SIGKILL recoveries cover unfinished
producer replay and skipping a verified finished DB2 producer. The skip scenario
corrupts only a private synthetic archive region after sealing DB2; resume succeeds
without rereading that region and matches the uninterrupted package.

Each profile's six-component one-tile case publishes 31 files / 141,786 bytes,
including MAPS, VMaps and one navigation global/tile. Empty WDT geometry and a
complete empty Map world also finish explicitly. Producer outputs match across
profiles; no new C++ or RustyCore consumer execution is claimed.

The bounded real build-70205/esES plan publishes 33 files / 193,794 bytes, all
matching the retained accepted DB2/GT/camera artifact hashes. The known incomplete
Map rejects with `local_store_no_verified_encoding`, zero published payloads and
a retained marker; verification rejects the failed run. The base run records 68
empty network traces and four fsync/SIGKILL traces without network calls; the
supplement adds six empty traces and two fsync/SIGKILL traces without network calls.

Retained base report: `product-20261011T142606Z.json`, SHA-256
`7a3c61186179422e840afe1653e9a784d6018ab4d77426c302c755ea7cd0675a`, and
`product-20261011T142606Z-sealed-resume.json` in the external acceptance reports
directory. These are product integration/recovery results. Complete real geometry,
source acquisition, public runtime loaders/gameplay and distribution remain gates.

## Executed supplied-key product v2 evidence — 2026-10-11

Debug and release each pass 2,081 library, four example and three binary
tests (2,088 Rust tests/profile), all 76 contract tests and workspace Clippy.
Eighteen added Rust tests cover canonical v2 plans, bounded key input, driver
admission and keyless output verification with plan identity separation.
The existing 17 v1 CLI scenarios and 20 new v2 scenarios pass in each profile
using immutable archived binaries. The independent Python synthetic fixture
checks two public Salsa20 vectors, encrypted ENCODING/ROOT and selected
payloads, N/Z, IVs of 4/8 bytes and multiple frame ordinals.

The encrypted six-component actual-tile package has 31 files / 141,786 bytes
per profile, exactly matching its plaintext v1 inventory and bytes. Sixteen
actual SIGKILL interruptions across both profiles yield 14 recoveries and two
expected protected-key-source refusals. Recovery covers retained-file replay,
changed supplied sets, verified sealed-DB2 skip with its key removed, and
same-CKey index-prefix candidate retry. Missing/wrong keys, malformed/FIFO/
directory/symlink/oversize inputs, tampered ciphertext, capability mismatches
and a key source inside the output reject at their stated boundaries.
There are 164 empty runtime network traces and 16 interruption/fsync traces
without network calls under injected network denial.

A bounded build-70205/esES run in both v1 and v2 reproduces all 33 prior
accepted DB2/GT/camera artifacts / 193,794 bytes. V2 uses only an explicitly
authored nonsecret dummy zero key: these inputs already have N/Z acceptance,
so this establishes no real encrypted-data compatibility. Both runs retain
the same logical source identity and different canonical plan fingerprints.
The client build-info hash is unchanged. Real Map still fails with
`local_store_no_verified_encoding`, zero DB2 payloads and a retained incomplete
marker; keyless verification rejects that run with exit 8. No missing real
input, additional C++ world output or RustyCore runtime acceptance is claimed.

Retained report: `product-keys-20261011T152515Z.json`, SHA-256
`0280ee4a15fa06dc96b4c7e7e96aa0d71706157e1ec43165737e21c4871b802c`, in the external acceptance reports directory.
No dependency, existing acquisition profile or underlying decryption primitive
changed in this CLI increment. Full real extraction and distribution remain
open.

## Target DB2 metadata catalog: product v4

Product v4 is a separate plan identity for build 70205. It requires the line
`schema-catalog<TAB>wowdbdefs-70205-v1` immediately after `root`, then an explicit
`decryption<TAB>disabled` or `decryption<TAB>provided-salsa20-v1` line. It admits
up to 1,161 sorted DB2 entries and resolves each name and file ID against that
catalog. If leased disk collision storage is selected, its storage and seven
budgets follow the decryption line. Build 70170 and the legacy catalog remain on
the earlier plan profiles.

The [v4 contract](../contracts/forever-product-contract-v4.json) records the
target schema source and decoder boundary. One-section files use the existing
bounded complete decoder except for three explicitly allowlisted, high-volume
tables: `ManifestInterfaceData`, `ModelFileData`, and `WMOMinimapTexture`. Those
tables use a separate reader profile capped at 160,000 direct records, 100,000
copy pairs, 160,000 parent entries, 1,250,000 retained scalar values, 16 MiB
per file and 64 MiB of cumulative decoded UTF-8 text. Parent lookup is accepted
only when the table schema declares a parent field. This selection is exact-
catalog/name bound and does not change the generic reader or v1-v3 products.
The v4 path also routes multi-section files to the separate regular-section
reader, which admits at most 16 unencrypted regular sections for catalog shapes
it supports. That route still excludes parent lookup, TACT/catalog data,
unsupported field-compression shapes, and inline IDs. The single-section reader
also admits the bounded sparse subset described in the completion record,
including inline UTF-8 strings for external-ID, parent-free schemas. Failed or
unsupported files reject the producer and are not emitted as a successful DB2
batch.
Verification uses the same plan-bound catalog and reader selection. It does not establish all table
variants, typed RustyCore loader behavior, or full-client extraction.

The schema catalog line participates in the canonical plan hash, so a package
created under v1–v3 cannot be relabeled as v4. Existing accepted artifacts keep
their prior identities and claims. Full encrypted-data, geometry, runtime and
distribution acceptance remain separate gates.

### Unified product v4: bounded sparse text and empty column metadata — 2026-10-13

The release product binary
`f9ebb26f1561f1cf8a4d039028bfa4f4dcb568c71fce3f4718ae58cf2eb01768`
accepted two more real build-70205/esES DB2 variants. Its bounded sparse reader
supports external catalog IDs, no parent or inline ID, one unencrypted section,
numeric scalar/array values and inline UTF-8 string/string-array values, under
the existing 100,000-row/copy/value and 64 MiB decoded-text limits. It rejects
TACT, physical catalog, parent and multi-section sparse forms.

`SceneScriptText` produced one exact 401,058-byte file; its plan digest is
`51c77026acf0134fcaa9b686fa985d7b5a4d2273910ebbf065858729b46d4a4f` and the
artifact SHA-256 is
`2f987601bf8967c9c159586130726d90f852f0b6e7e04b873ce5343e0f16fc4c`. The
package-time producer and a separate `verify` both passed. The source DB2 is
byte-identical to the independent local acquisition. A structural scan found
147 rows, each with two NUL-terminated UTF-8 strings. No Python row-oracle
equality or RustyCore typed-loader/runtime acceptance is claimed.

The same binary admitted zero-row `SpellMissile`, whose sectionless WDC5 carries
complete per-field column metadata. The reader validates that metadata block
and exact EOF while requiring no string/common/pallet payload. Its one-file,
624-byte package passed extraction and separate verification; the packaged file
is byte-identical to the independent local acquisition. Plan digest:
`99b303ed289c5b4a763daf0285f19121929f447ca7cae45761dcfa77f0ccdfa8`; artifact
SHA-256: `bf4ab8a49d7f7fb8f118b3998f74b9a5cd3788ad120bc0676e5e706e48b4ed10`.

These are bounded product/DB2-reader acceptances for two inputs. They do not
change the other sparse rejections or establish all-catalog coverage, typed
loaders, runtime behavior, complete client extraction, or
`full_extractor_complete`.

A fresh 1,074-table product plan then admitted both files in the same batch,
passed extraction and separate verification, and retained all 1,072 prior
artifact path/size/hash entries unchanged. The plan SHA-256 is
`9f3cdca5c1773c906e16cbbf2f6a4816e386aec9029fa91ba62decb791aef9c1`; its
inventory SHA-256 is
`e63230fc1e784a5d8dacb6f38dd45e2d31d4a0c3e05610b2254df68766f97eb2`. The
private package is under
`/mnt/nvme2/datos/forever-product-runs/catalog-v4-high-volume-1074-20261013/output/`.

The fresh independent 89-plan matrix with this binary verifies the same two
tables and records 82 `local_store_no_verified_encoding`, three
`unsupported_record_variant` (the three CollectableSource sparse tables), one
`unsupported_table_schema` (`ExteriorComponentGroup`) and one
`invalid_record_string` (`SpellMissileMotion`). Retained summary:
`/mnt/nvme2/datos/forever-product-runs/catalog-v4-remaining-89-20261013-r3/summary.json`.

A follow-up matrix supplied the three matching public keys available in
`wowdev/TACTKeys` commit `f800811762b2e9ea638bf6dfdb4707382c497943`. It still
verified only the same two tables. `ItemDisplayInfoMaterialRes` advanced from
`local_store_no_verified_encoding` to `unsupported_record_variant`; aggregate
results are 81 missing verified encodings, four unsupported variants, one
unsupported schema and one invalid record string. The public snapshot covered
three of ten required frame-key names. Its temporary keyfile was removed, and
no key bytes are retained in the repository. Summary:
`/mnt/nvme2/datos/forever-product-runs/catalog-v4-remaining-89-keys-20261013/summary.json`.


## Explicit product collision disk source profile v9 (2026-10-12)

The mandatory corresponding-source profile is now
`forever-seven-crate-linux-product-collision-disk-source-v9`. It adds the explicit
product plan v3 parser, Linux GNU x86_64 borrowed-directory space observation,
guarded disk driver, CLI integration, synthetic checks, contract and scope
documentation. Seven crates, nine lock packages and two registry archives remain;
the extractor additionally has a local Linux platform edge. Historical v7/v8
profiles and their frozen verifiers retain their original claims.

The new plan selects `linux-leased-append-disk-world-v1`, seven downward byte
budgets and an explicit `--scratch` for extract/resume. Six fresh private leased
append owners finish the checked package before common product collision writes.
Unfinished collision resume uses a new scratch namespace; no archive adoption or
cleanup is implied. Physical preflight aggregates same-device allowances, but is
not an atomic reservation or capacity guarantee. Common Run live capacity is
reserved without raising conversion memory ceilings. Resident v1/v2 stays default.

Executable validation passes 2,643 ordinary Linux tests per debug/release
profile (41 targets, 26 ignored), four explicit ignored corpora per profile,
49 existing CLI cases per profile, all-target Linux/GNU-Windows Clippy, both
GNU-Windows compile profiles and scoped formatting. Final source association
reruns 187 contract and 112 distribution/sealer checks plus one added
cross-volume CLI case per profile against the exact full-run product binaries.
All Rust/Cargo and existing CLI-test source is unchanged from the full-run freeze.
This is synthetic GNU/Linux product/source acceptance; no new bundle,
real-client/native acceptance, Windows product or RustyCore runtime acceptance
is claimed. Complete encrypted inputs, real geometry, runtime/server integration
and reproducible distribution remain gates. The extractor is not complete. See
`docs/forever-product-disk.md` and the versioned disk contract.

## Target DB2 metadata catalog: product v5 — 2026-10-13

Product v5 is a separate plan and package identity for build 70338
(`1.60.1.70338`). It requires `schema-catalog<TAB>wowdbdefs-70338-v1` after the
root line and allows only the `db2` component. The catalog pins 1,161 table
definitions to WoWDBDefs commit
[`d6f50d4`](https://github.com/wowdev/WoWDBDefs/commit/d6f50d4f351193e84226aa62a36faa0f58528afa),
manifest blob `8a5d8122f312ae6a9564e8286fdc491831fecc22`, and source SHA-256
`4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265`. The
1,161 metadata shapes match the separately pinned 70205 catalog, while the
build target and provenance remain distinct. This is metadata coverage, not
proof that each table can be acquired or decoded.

The local 70338 client package was admitted with v5, extracted, and independently
verified for `TactKey` and `TactKeyLookup`: two artifacts / 4,292 bytes. The
canonical plan SHA-256 is
`a2f1fe241e22c2659a0606e546f8297959b144107c7e093ab2576f1c779e2c2d`; the
verified inventory SHA-256 is
`976cd6825f16ab4175668d4a9e3511f9c7b6b93411bc477e9177b50961fc9ddc`. Their
artifact SHA-256 values are respectively
`bdf268fa9eefda4ae23773599a69b542666fd9b7e1172fca135c183ab1b7b9e5` and
`5202ce0687062f29a7ba873b71b9986be7077c1a7cf47a70b01dee8e4a5c91eb`.

The two decoded tables contain 69 direct key records and 197 lookup records.
All 69 direct records join to lookup names and match the corresponding public
[`wowdev/TACTKeys` snapshot](https://github.com/wowdev/TACTKeys/tree/f800811762b2e9ea638bf6dfdb4707382c497943)
after the DB2 little-endian value interpretation. Of the other 128 lookup names,
47 have public supplemental keys in that snapshot; the client-derived and
supplemental set therefore contains 116 names. Key material is kept outside the
repository and is not included in the report.

A v5 `Map` probe with 69 client-derived keys and a second probe with the 116-name
enriched set both fail closed as `local_store_no_verified_encoding` for the
selected file (file data ID 1349477). With the enriched set, the failure
diagnostic counts zero missing or incomplete local files, one encrypted
encoding candidate, and zero corrupt or unsupported candidates. No successful
map payload or available-prefix result is produced. The v5 contract therefore
remains DB2-only, and `full_extractor_complete` remains false. It establishes
neither complete 70338
DB2 extraction, other product components, geometry, runtime compatibility, nor
full client extraction.

A fresh local survey of all 1,161 catalog entries acquired 1,078 complete
artifacts (29,213,383 bytes). Strict row decoding produced 1,074 tables /
1,160,115 logical rows. Raw-only catalog validation additionally accepted
`SpellMissileMotion` and the exact three-table sparse-parent allowlist, so all
1,078 acquired artifacts pass product DB2 validation. The regular string check
repeats framing, pointer, NUL-terminator and cumulative byte-budget validation
without converting the payload to UTF-8 or returning rows. Strict typed decoding
still rejects `SpellMissileMotion` because one payload byte (`0x97`) is outside
UTF-8. For sparse-parent files, bounded record framing and parent-block byte
length are checked, but no rows or parent associations are projected; the pinned
C++ sparse materializer still rejects those tables. Eighty-three files remain
encrypted local encoding candidates, so the full plan remains all-or-error and
emits no available prefix.

A one-table v5 package containing the real `SpellMissileMotion` file passed
extraction and separate verification: 1 file / 17,236 bytes. The packaged file
matches the separately acquired source byte for byte (SHA-256
`97bd53f4fd4ecd33c2f0169e7026501e70cce07ba7e444efa0a782af11b9ff1e`). Plan
SHA-256: `97ff5a9acd59650e9a562ff441ed6394b03a423339769350c5f6f5190750f6ea`;
inventory SHA-256: `ed10d1d0c755bd3f6f54ace1e4ad5cb18b27b3fa944823509e62961e4e4866a`.
Private plan, package and verification report are retained under
`/mnt/nvme2/datos/forever-product-runs/catalog-v5-spellmissile-opaque-text-20261013/`.
This validates raw artifact preservation, not typed UTF-8 projection, complete
DB2 extraction or overall extractor completion.

The same survey exposed an empty regular-table shape for `ExteriorComponentGroup`:
its zero-section header counts the appended parent in both field counts while
declaring no records. The reader now admits this exact catalog shape only when
there are no rows; nonempty files still require the physical column count. A
one-table v5 package for the real 216-byte file passed extraction and separate
verification. Its plan and inventory digests are recorded in the retained
private run at
`/mnt/nvme2/datos/forever-product-runs/catalog-v5-exterior-empty-parent-20261013/`.
Plan SHA-256: `d9e441c3cfd31641c998408b0906681cc07bf66987d06e0e1b1fbd312a5048b3`;
inventory SHA-256:
`7731349cecbd4f0a6f9481522f2132400b141d3df6b196f288324da9ba40c10e`.
This admits one empty file and does not establish populated appended-parent
compatibility or complete catalog extraction. The pinned reference checks the
layout and total-field relationship in
[`DB2FileLoader.cpp`](https://github.com/TrinityCore/TrinityCore/blob/02245dcd245e7433e524577656177723d3e4992e/src/common/DataStores/DB2FileLoader.cpp);
the evidence here does not include running that native loader on the client
file.


A fresh three-table v5 package admitted the raw-only sparse-parent profile and
passed extraction plus separate verification: `CollectableSourceEncounterSparse`
(212 bytes), `CollectableSourceQuestSparse` (220 bytes), and
`CollectableSourceVendorSparse` (192,664 bytes), 193,096 bytes total. All three
packaged DB2 files match their separately acquired source bytes exactly. Plan
SHA-256: `f9a5fd4e6ad2e0b8bdfd8458110d245131e4783a5bedbf2db84377f1f84546b4`;
inventory SHA-256: `b57435c960173ac5dce6c7734aa0860bd9f8aae7b7dde5b48ab7c3724fe8b61b`.
File SHA-256 values are `63d499404dd80f65b1f9f38b5c197ec092fe76f08623810ff4bd2c69856b6d3f`,
`c27193dca19a59fd10194e284688e65feaf1a80660466e29d1152b7b8daeb56a`, and
`a57d5e87e327559647ba684c37c104f5952c374a0c6e958a9ae299b7aae3dce3` in the
same order. Private plan, package and verification report:
`/mnt/nvme2/datos/forever-product-runs/catalog-v5-sparse-parent-raw-20261013/`.
This confirms bounded raw artifact admission only. Strict sparse row decoding,
pinned native parent materialization and parent-association parity remain
unsupported; the 83 encrypted candidates still block the full catalog plan.


## Shared execution boundary and workflow research — 2026-10-13

The operator's client-folder example is exploratory, not a path-only CLI
requirement. [Issue #28](https://github.com/alseif0x/rustycore-extractors/issues/28)
prioritizes common extraction execution before further OS-specific expansion.
The Windows leased-disk candidate in PR #27 remains separate and unaccepted;
its code and additional CI jobs are not dependencies of this increment.

Reviewed primary sources at TrinityCore Forever `8c5325b`, wow.tools.local
`cf33a49`, cascette-rs `2d1294b` and CascLib `38a3466` distinguish physical
storage access from format conversion. TrinityCore's
[orchestration script](https://github.com/advocaite/TrinityCore/blob/8c5325b37dfa155450a9e87f213965f6e6c00afe/contrib/extractor.sh)
runs map extraction, collision extraction, collision assembly, then navigation.
Its tools allow explicit paths or working-directory defaults. MMaps are generated
from extracted terrain/collision inputs. wow.tools.local separately configures
folder/product/locale and its storage/DB2 dependencies; cascette's README describes
Rust CASC libraries and planned/ported CLI tools, not a completed compatible
server extractor. The newer Trinity tree changes no acquisition implementation
relative to the pinned oracle `02245dcd245e7433e524577656177723d3e4992e`;
the added DB2 list entry is already present in our catalogs.

The candidate moves the existing six-producer loop into private
`driver::execution`, in DB2/GT/cameras/terrain/collision/navigation order.
Concrete producers, public API, plans v1-v6, hashes, build/key gates, resource
ceilings, output bytes and package formats retain their contracts. Linux owns
scratch preflight and collision storage selection. Windows retains its supplied-key
lease and every-return check. The common boundary has no OS selection or path
acquisition; it checks store then adapter before component/final seals and skips
completed stages. Eleven authored lifecycle tests cover order, skips and failures.
The existing three CI roles select its ten source-contract checks; corresponding
source automatically includes the new Rust module, JSON contract and Python tests.

Research report SHA-256:
`a10e9e8de8848375cbefa165d5cb6ad093388ebca0493e62a807cb2676ba4b1e`.
Evidence root:
`/mnt/nvme2/datos/forever-product-runs/common-execution-20261013-w9fqnb60/`.
See `contracts/forever-common-execution-contract-v1.json`. This private refactor
adds no automatic plan, new OS support, real-client or runtime acceptance, or
reproducible distribution. Final baseline/candidate and native acceptance are
recorded separately; the full extractor goal remains open.


### Subsequent common-execution local acceptance

The frozen implementation passes 2,754 ordinary workspace tests / 26 ignored
across 42 targets in each Linux profile, formatting and all-target Clippy.
Windows GNU product Clippy passes both default and explicit seal-v2 selections;
these are compile checks, not native execution. All 301 source-contract checks
pass; distribution source checks pass 162 tests with one platform skip, without
building a new reproducible bundle.

Independent baseline/candidate debug/release comparisons pass 28 authored
synthetic plan cases. Fourteen candidate cases match all six nonempty component
groups: 35 files / 142,622 bytes per case, with 490 direct artifact comparisons /
1,996,708 bytes. Six candidate SIGKILL/resume cases reproduce the entire clean
output inventory, bytes and manifest. Context fields match across binaries
except producer SHA-256. All 104 runtime traces contain no network calls;
84 are empty and the others contain only fsync/signals. Baseline 1,078 and
candidate 1,081 source paths, all four binaries and every plan/input remain
unchanged during the run. Later acceptance documentation is not rebuilt.

QA report SHA-256:
`ba753677b5abc8d6b78f801d915f005b2896bbae7f068f09e4f78ce6c7e70a58`.
Coordinator's independent complete-byte review SHA-256:
`d7a24b459587c84db8841b70c995fcbda0c70755829e2318da393e09db1d8ae0`.
Both are retained under the evidence root above. No native C++, real-client,
new native Windows, runtime, full-world or reproducible-distribution acceptance
is inferred; those gates and the full extractor goal remain open.


## Explicit Linux terrain/navigation output links — 2026-10-13

[Issue #30](https://github.com/alseif0x/rustycore-extractors/issues/30) adds the
explicit Linux `verify-links --plan PLAN --output OUTPUT [--expect-context CONTEXT]`
capability, profile `forever-terrain-navigation-links-v1`. It reuses complete
package integrity, plan/context binding and intrinsic format verification in
one package traversal. The existing `verify` invocation keeps its intrinsic-only
scope and behavior; plans v1-v6 and package formats retain their identities.

For all-map or single-map terrain selections, each observed map's whole tilelist
must equal its published MAPS coordinates. Missing lists, missing/extra MAPS and
MMap tiles without their map global fail explicitly. Empty lists and global-only
navigation are valid; tile subsets retain their no-whole-map-list rule. Facts
retain fixed bitsets and navigation presence, not artifact buffers. At most
8,192 observed terrain/navigation map IDs are admitted, charged at a conservative
4,096 logical metadata bytes each (32 MiB). This is bounded admission accounting,
not an exact allocator/RSS measurement; older package/file ceilings still apply.

The command reports the explicit profile and limited link scope, with
`full_extractor_complete=false`. It checks relationships among published files;
it does not prove omitted unobserved maps, complete CASC acquisition, VMap model
or parent associations, Detour topology or runtime acceptance. No client, key or
scratch input is required. Windows rejects this capability before input I/O.

The current installed client declares build 70338; the older fixture declares
70170 and links its Data directory. A complete immutable 70205 source has not
been re-established. The five historical 70205 DB2 acceptance followups and the
ItemDisplayInfoMaterialRes diagnostic remain separate pending real-input work;
70338 evidence is not transferred to 70205. No new client payload/key/memory read
was performed by this source/metadata audit.


### Linux output-links executed local acceptance

The frozen implementation passes 2,777 ordinary Linux workspace tests / 26
ignored across 42 targets in debug and release, with formatting and all-target
Clippy clean. The product has 226 passing tests per profile, including 23 new
unit cases. All 311 source contracts pass; distribution source tests pass 162
with one platform skip, without a new reproducible bundle.

Independent CLI QA passes 20 methods / 64 commands per profile, with 128 empty
denied-network traces. Its authored 35-file / 87,667-byte base packages match
between debug/release. Separately, 20 new-profile checks accept the preceding
35-file / 142,622-byte v1/v2/v3/v6 complete/recovered packages, with exact expected
contexts, unchanged outputs and 20 empty traces. These are separate fixture
inventories; no new geometry producer or real-client acceptance is inferred.
An independent review checks all 1,086 frozen source paths and retained evidence.
The subsequent acceptance documentation is not rebuilt into those binaries.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-output-links-20261013-pyw2o1xq/`.
QA report SHA-256:
`2f74ca579526efea9c520291fc41806a85a70ca3a8faceb67caf226b2b605d3b`.
Independent acceptance review SHA-256:
`087f3b1895bc08732674e1af5f8275f98ed46fed037dac29159bc8a1bdc2dcbc`.
The Linux real-input, full-world, VMap association, runtime and distribution
gates remain open. Windows development stays deferred.


## Linux selected map and ancestor domain v8 — 2026-10-13

The explicit [map-domain contract](../contracts/forever-linux-map-domain-contract-v1.json)
adds product plan v8 for Linux, builds 70170/70205 and the pinned metadata
catalog. It requires the existing leased disk collision profile, DB2/terrain/
collision components and a complete published `Map` DB2. V1–v6 canonical bytes,
hashes and execution semantics remain unchanged; v7 is reserved by the deferred
Windows draft. This v8 checkpoint leaves build 70338 DB2-only under v5;
the subsequent v9 profile below separately admits DB2/GT/cameras.

After the existing disk-budget fields, v8 declares these canonical fields:

```tsv
map-domain	map-and-ancestors-v1
map-id	7
gameobject-scope	global-startup-v1
placement-ids	selected-closure-serial-ids-v1
```

The ID above illustrates grammar, not a real-client map recommendation. Terrain
must select that whole map. Navigation is `none` or the same `map:<id>` request;
when selected it requires `LiquidType` and the explicit inline offmesh section,
including an empty section. Supplied connections remain scoped to the requested
map. Existing key and scratch requirements still apply.

Extract/resume authenticates and validates the complete Map before creating or
resuming package state. It rejects duplicate, cyclic, missing or copy-only graph
relationships and inactive selected members. The domain contains the requested
map and its actual effective ancestors: nonnegative ParentMapID takes precedence;
otherwise CosmeticParentMapID is used. Both branches are never combined. The
complete source graph and original physical ordinals survive collision planning,
but only domain WDTs and ADTs are acquired. Present dependency failures remain
terminal; an available prefix is not a successful file.

Terrain publishes whole-map MAPS/tilelists for the complete domain. Collision
publishes each domain tree and its tiles/indexes, selected VMOs and the global
GameObject dtree. GameObject startup remains global: its models are dependencies
even when they have no selected-map placement. Globals precede own ADTs, and one
serial allocation Session spans the domain. IDs are deterministic within that
domain; they need not equal a slice of a full-world extraction. Independently
extracted domains cannot generally be combined without an ID/rebinding policy.

Navigation also generates ancestor dependencies. A compact view retains the
original selected **direct** numeric Map rows, their order, flags and both parent
fields; it does not replay copies or fabricate SourceRecords. Discovery uses
that closed view and original own-candidate semantics. Each map with own MAPS or
payload `.vmtile` candidates generates navigation. A map with only an empty
tilelist and inherited `.vmtileidx` emits no invented global/tiles; its ancestors'
MMaps are included instead. A genuinely empty complete domain can therefore
finish with zero MMaps. Source graph/root-parameter validation remains mandatory;
not all navigation startup errors are admitted during the initial Map preflight.

The domain retains at most 1,024 members. Core selection reserves at most 64 KiB;
product domain plus the navigation numeric view share a separately checked
64 KiB ceiling. Their actual vector capacities reduce component budgets.
Complete Map input is capped at 16 MiB, with a conservative 690,028,544-byte
preparation reservation and 2 GiB combined admission. No component ceiling is
raised. These logical allowances do not measure allocator overhead or RSS.

Both v8 `verify` and `verify-links` always enforce domain/link checks over the
same verified artifact buffers. They derive the domain from the published Map,
require exactly its terrain tilelists and VMap trees, match published MAPS to
list bits, and require precisely the navigation globals justified by own MAPS/
`.vmtile` candidates. Inherited indexes do not justify a global. Five bounded
sorted fact vectors reserve 24 KiB separately from the source-domain allowance.
The checks do not prove expected WDT tile completeness, VMap associations,
Detour topology, publisher authenticity or runtime acceptance. V1–v6 intrinsic
verification retains its previous scope.

The implementation passes 2,832 Linux workspace tests / 26 ignored across 42
targets in each profile, all-target Clippy and complete formatting. Source
contracts pass 329 checks; distribution source tests pass 162 with one skip.
Independent integrated acceptance passes 29 methods / 116 commands per profile,
with exact original collision/terrain comparisons, ancestor navigation against
explicit v6 closure references, recovery equality and retained legacy golden
artifacts. Its guarded reports and unsuccessful earlier attempts are recorded
in [the completion evidence](forever-completion.md). No new distribution bundle
or real-client/full-world/runtime completion is established by this step.


## Linux build-70338 DB2/GT/camera product v9 — 2026-10-14

Issue #48 adds `PublicationProfile::Build70338Db2GtCamerasV1`, with explicit
plan/contract v9, build 70338, `wowdbdefs-70338-v1` and exactly
`components=db2,gt,cameras`. The canonical
[disabled-key example](../tests/contracts/fixtures/forever-product-v9/disabled.tsv)
selects public `CinematicCamera` FDID 1294214. The DB2 selection must be nonempty,
canonical, at most 1,161 entries, and include that source. GT selects the fixed
`gt-catalog-70338-v1` twenty-file catalog; cameras use `camera-raw-70338-v1`.
Locale, ROOT/index layouts and decryption policy remain explicit. Both existing
`disabled` and `provided-salsa20-v1` policies are admitted by source/tests;
the new real campaign uses **disabled only**. No automatic selection, geometry,
disk-world/map-domain/off-mesh profile, fallback or increased cap is admitted.

Execution is serial DB2 → GT → cameras. Before discovery or camera dependencies,
a bounded reread must match the already published, sealed DB2 source, including
when resume skips completed DB2. `Run::verify_sealed_artifact` checks the rooted
seal, receipt, hash, EOF, state and name identity without publishing or changing
transaction/counter state; failures poison the run. Publication through `put`
is not a read-only source proof. Package verification preflights the complete
canonical manifest/receipt/seal metadata and inventory budgets before the first
artifact payload read/hash/buffer, independent of path order.

DB2 admits at most 16 MiB per file and the existing 16 GiB total; GT at most
4 MiB per file / 80 MiB total. Camera discovery admits a 16 MiB source,
4,096 IDs, 16 MiB per camera and 256 MiB total including the source exactly once.
These are admission/work limits, not RSS or physical-I/O measurements.
Artifacts remain complete and nonempty. A complete nonempty DB2 with zero rows
can yield zero cameras without creating a camera directory; the standalone
empty-directory rule is not transferred. Final package manifest v1 and Linux
recovery state v2 are unchanged. Legacy v1–v6/v8 identities, canonical bytes,
errors and build gates remain; v5 is still 70338 DB2-only and v7 remains reserved
for its separate Windows work.

### Corrected producer and executed acceptance

The initial implementation `2e6c05eeff1a115f35e9fd19e49a597c4ed2d5ba` passed
its own synthetic and real campaigns; those immutable receipts remain historical.
The corrected functional commit is `6f39cc0f10c0b3c35cd824bb7d2035ebe7157343`,
tree `49d53dff59fe5bef0cd30561c74f00a602fb6fcd`. Its local binaries compiled base
`a4ad23af04b9d297f4da6d99ab5fb441169387bb` plus two reviewed source/test changes
before the corrected commit. Exact Git-blob binding matches **665 files /
15,532,624 bytes**, including the 453-file Rust/Cargo subset. This is source
equality, not recompilation at that commit. Metadata-only checkout commit
`2119a23024607c590ccd266a140d5a1571d24414` preserves all 665 Git entries/current
bytes and both binaries; subsequent three-file documentation changes do not
rebuild them. Fresh campaigns below bind the corrected binaries independently.

The first hosted head failed Windows compilation because the shared inspector
selected `File` while the Windows callback requires bounded `dyn Read`.
The correction selects the exact borrowed type/lifetime per platform; Linux
still uses `File`, with unchanged inspector body, calls, limits and format scope.
The next Windows read-consumer job failed nine canonical TSV fixture tests.
Local Git filtering with `core.autocrlf=true` reproduced 318 bytes / 16 CRLFs
instead of the canonical 302-byte LF fixture. Hosted fixture bytes were not
directly measured. Pinning `*.tsv text eol=lf` preserves parser strictness;
all six existing hosted checks pass at `2119a23`. Final documentation has its
own exact-head CI gate. This repairs inherited checks without extending Windows
product development.

The final complete eight-command validation passes debug and release workspace
runs, each **2,917 tests / 26 normally ignored / 45 targets**, all-target Clippy,
format, 347 source contracts, distribution checks (162 pass / one skip), and
both product builds. The focused package filter passes 127 tests per profile, with three historical
ignored tests, including 12 new passing cases; product tests number 281 library
plus five CLI, including 20 new library cases. Eight new formal contract methods are included. No new bundle was built
and normally ignored corpora were not rerun. The retained first workspace attempt
is globally failed: two old structural assertions failed after five commands;
distribution/final builds did not run. Two Python-test-only fixes preserve all
453 runtime files byte-for-byte at that repair; all eight commands then ran anew.
After the callback correction, the complete eight-command validation runs again
as `workspace-attempt3`, with new debug/release binaries and independent QA.

The corrected-binary synthetic campaign runs 78 commands: 30 successful, 44 expected
exit-9 negatives and four deliberate SIGKILL cuts. Its 32 owned output views
contain **eight positive complete packages and 24 negative copies**. Six positive
clean/resumed views contain 31 authored data files / 2,022 bytes each; two valid
empty-camera views contain 21 files. Both profiles cover clean extraction,
verification, GT/camera interrupted resume, sealed-source mismatches, wrong
plan/producer, complete/existing outputs and malformed/budget/inventory rejection.
All 2,463 frozen files / 269 directories and exact path sets pass the terminal
audit. The outer controller was captured only by the tool console, without
persisted raw outer stdout/stderr; no separate outer-stderr-zero claim is made.
All 78 child outputs/results/traces are retained. The terminal auditor has its
own private output capture; that is a separate invocation.

The corrected-binary real 70338/esES campaign runs **26 commands: 22 successful and four
intentional SIGKILL cuts**, without retry or recalibration. Debug and release each
produce a clean package, a GT-interrupted/resumed package and a camera-interrupted/
resumed package: **six complete packages, each 31 data files / 165,507 bytes**.
Each contains twenty GT files / 138,537 bytes, one CinematicCamera DB2 / 674 bytes
and ten opaque cameras / 26,296 bytes. All payload bytes and complete inventories
match the immutable previously accepted 70338 component oracles. Within each
profile final manifest bytes and DB2 seals are identical; across profiles the
manifest differs only in producer SHA. All 628 frozen files / 49 directories,
980 source/tool/evidence guards and 38 declared aliases pass independent terminal
review. Both raw outer streams were captured from process start: stdout 3,542
bytes, stderr zero. The final review accepts only this bounded real publication
scope. Source-only dependency admission has 962 guards / 38 aliases; the real
admission preserves them and adds 18 metadata guards, including bounded
`.build.info`. Root, prelaunch QA and terminal-auditor metadata reads have
separate scopes and are not all CLI/store reads.

The historical native whole-byte/CKey, numeric GT, public GT-loader and original
camera-discovery acceptance supplies the immutable oracles; those consumers were
**not rerun** by this product campaign. No client keys were read. IPv4/IPv6 denial
probes execute in child preexec before tracing; consumer traces show zero network
calls (synthetic: 72 empty plus six fsync/cut/calibration traces; real: 22 empty
plus four fsync/cut traces). This is not a global zero-socket claim. All owned
process groups are absent after completion. Metadata admission/audit reads and
published-artifact counts do not measure CASC requests, acquisitions or OS I/O.

Private evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70338-product-20261014-21a1dvjx/`.
Only aggregate results and receipt identities are public; client artifacts,
private camera identifiers, per-file content hashes and correlations stay local.

| Receipt (relative to evidence root) | SHA-256 |
| --- | --- |
| `impl/source-v4/freeze.json` | `e76378921f3183d72e68ac7f58c6fdf1cff3c0c0133a897eb1c3e1ff7d6967c4` |
| `impl/workspace-attempt1/validation-report.json` (failed) | `af352a6e19562f9720ccde83b63ef97d5edf99028fe9a3810094c4f0a1794ac5` |
| `impl/workspace-attempt3/validation-report.json` | `9b99c951f5decf55191ca5409e297e1a0ce1f1fd08c929c7be9d0730dc351a0d` |
| `impl/workspace-attempt3/producer.json` | `02b7dd360655d62ce76deaffdbf92cecee11dae6d35e7535a7ce108824b9f5c3` |
| `root/functional-binding-corrected-v1.json` | `7671830e85d5bb46da6c647fa380e4566cfc607b0b7fbb64dec77a393c60beb7` |
| `qa/review/workspace-validation-review-v2.json` | `88c615cab5e7dc91eca5f3a3809d214dc2549df72d80d1f6d424a41093942249` |
| `qa/review/functional-binding-review-v2.json` | `6c4a8fc60f50ca8c229cec31dc5b0585c80885e7d48dfe9c7d4acd6432c0c12a` |
| `root/checkout-policy-commit-binding-v1.json` | `f189173b613ad6ecd4acf44f412eca633a0a6fd0f714c8d1bfda50ce9d52dd63` |
| `synthetic-attempt2/acceptance.json` | `97cc91b21e80f71ed774566948390a8c2c1d932651b142ebffca8ebf4d9d3282` |
| `synthetic-attempt2/evidence-freeze.json` | `6f68154fe6c16f3a525797db3969d117bfbaa3c6bf7304c86f3b7137b4526c9a` |
| `qa/synthetic-terminal-audit-v2.json` | `dea41d7a9263a754e84184773057c29b21c1bdcd17eacbb082f32938314979a5` |
| `qa/review/synthetic-terminal-review-v2.json` | `2121bce370874e959ff0720ee833e1814c9d1f335440991f733bb212aab2e38e` |
| `real-attempt2/acceptance.json` | `a98f4e6912399bfaa11ca0b361f5d80564759cc95068478236577804d1ff2ab4` |
| `real-attempt2/evidence-freeze.json` | `5592709e6ef3a2b1547238b23f83ba8e643a07296e95d08c45c28b0c09908b06` |
| `qa/real-terminal-audit-v2.json` | `070921412b7b31081cb4255093ebe81655e286eb4b5e3446f404bea52ef0894d` |
| `qa/review/final-real-review-v2.json` | `7a9573cea434ef1fe61bd32d6359eed4023158c020e2118a321efd83745d22a2` |

This closes explicit Linux 70338 DB2/GT/raw-camera publication and recovery for
the bounded real selection. It does not accept the full catalog, geometry,
full-world resources, M2/waypoints/playback, server runtime, Windows product or
a new reproducible distribution. The historical 83 unavailable catalog entries
and Map/GameObject dependencies remain recorded gaps, without a new survey here.

The initial producer's accepted real review remains at
`qa/review/final-real-review-v1.json`, SHA-256
`54f0764d5040e78994645b0d6fb9a6b3247187d5d99680277b16d1999cd343c9`.
Its earlier campaigns and failed local/hosted attempts remain immutable; their
acceptance is not substituted for the corrected-producer evidence above.

## Public key refresh: negative historical coverage — 2026-10-14

Issue #52 authenticates two complete `wowdev/TACTKeys` `WoW.txt` snapshots
with exact length, SHA-256 and Git-framed blob SHA-1 before whole-file parsing.
Old commit: `f800811762b2e9ea638bf6dfdb4707382c497943`; new commit:
`a3da648c120465b01c6c65a2e33ebf515a8a342a`, tree
`a7281f1d14546d00fde1858ff8daa90376e18dcc`.

| Snapshot | Bytes | Git blob SHA-1 | SHA-256 |
| --- | ---: | --- | --- |
| Old | 983,950 | `11373e67eed8c487c175b2d53bb6acc3765608cd` | `52bdbe5f369a854a460056628d956708ffcf51cf1e1ef300f1713e25adab1cc2` |
| New | 984,250 | `6471af43e6c6eae4dfaf62601550f4060200bdbd` | `f6384c5b8d9dcd61bf8db37de691b68908915709df21f11650df292b7d44c8d4` |

The single actual comparison records 19,679→19,685 entries: six added, zero
removed and zero changed values. Against 83 historical associations / six
missing names, both snapshots match zero names and zero associations. No
subset is written; client/CASC reads and native/product/compiler executions are zero.
Twenty-four fake controls pass; source QA SHA-256: `e6b0996583ab12f7004a14639067a5e9874990e335380996ff3539d0ff290e98`.
Actual comparison receipt SHA-256: `c44c89681c4974551fac358d61908e639c351d44848366547f813a326a7563e0`.
Final independent QA: `passed-negative`, receipt SHA-256
`93bd2a54aea752ec8d14abb6f7b30ea07ba94c755990c9d3b5d5092264b81993`. Earlier `f800811` observations
retain their dated scope; this comparison reads no current client/ciphertext
and proves no decoded recovery. The 83 recorded failures, Map/GameObject,
full catalog, geometry and runtime gates remain unresolved. Gate B, another
client retry or full survey is not admitted. Key values, individual names and
per-file correlations stay outside model/Git/CI; Windows remains deferred.

Retained evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70338-key-refresh-20261014-oazqtiee/`.

## Explicit Linux 70374 DB2 product v10 — 2026-10-14

Issue #56 adds canonical product plan/contract v10 and
`Build70374Db2OnlyV1`: explicit Linux Rust-local acquisition, build 70374,
`wowdbdefs-70374-v1`, DB2-only component mask 1 and resident publication.
The commands are inspect, extract, resume, verify and verify-links. Both existing
key policies remain explicit; no new `--backend` option or default build
admission is introduced. Legacy Session/native/standalone profiles retain their
gates. Other components and Windows v10 execution reject.

The independent 1,161-table catalog pins WoWDBDefs commit
`464a25b3a7c02b0c74726be7a53a0b0ccf1a2d10`, with facts SHA-256
`cf9a5a549136090c3aba1dddccc76c4aec45eb8a30162f053860f2bed44d2faf`.
Its normalized metadata equals the older 70338 catalog; real acceptance is
established separately below. Product verification preflights the complete
selected DB2 inventory and its unchanged 16 MiB/file and 16 GiB aggregate
limits before payload processing. Off-mesh fields and geometry selections are
outside v10; its only terrain/navigation selection is `none`.

Final compiled source commit `52db8418587d6db2994bb91137aee11a67d5d6e8`
binds 708 selected files / 18,552,341 bytes; the separate Cargo/crate inventory
has 458 files / 9,881,390 bytes. Debug and release each pass 2,951 tests,
with 26 normally ignored tests across 46 targets. The nine-command validation
also passes Clippy, formatting, 355 contracts, 162 distribution checks plus
one skip, catalog regeneration and both product builds. Ignored tests are not
real acceptance. Independent review binds the logs and frozen producer copies.
Subsequent documentation does not rebuild those producers.

The authored CLI campaign passes 82 expected outcomes: 32 exit-0, 16 exit-2,
four exit-8, 26 exit-9 and four deliberate SIGKILLs. Both disabled and supplied
toy-key policies preserve two DB2 files / 922 bytes per complete view, with
exact same-profile clean/resumed manifests and bytes. Wrong contexts, existing
outputs, metadata limits, tampering, malformed second sources and pre-I/O gates
are covered. Incomplete selections remain failed. All 941 guards close and
82 owned groups are absent. Consumer traces have zero network calls: 74 empty
traces and eight classified fsync/cut traces; child-preexec denial probes are
separate.

The external four-table row adapters use the new catalog. Their qualification
passes 16 commands, ten authored Python methods, eight exact Rust/Python row
comparisons and four CLI negatives. The strict native oracle pins CascLib
`02245dcd245e7433e524577656177723d3e4992e`, complete EOF/CKey-MD5 checks
and 16 MiB/file / 32 MiB/selection. Native built-in keys differ from Rust's
disabled policy; matching bytes do not establish equal decryption capabilities.

The first whole real campaign remains failed at a trace-auditor defect: a
signal-only cut produced no INJECTED marker. V5 records the exact wrapper argv
and validates the calibrated fsync count plus SIGKILL; four authored audit
controls pass. One explicitly authorized fresh TACT campaign and one separate
startup diagnostic bound the native campaign to three invocations, including
the failed one, and at most 96 MiB of required source bytes. This changes no
producer/file/selection cap and does not measure total hashing, CASC or OS I/O.
The failed evidence remains unchanged; its historical external launcher guard
has a documented later source transition.

The fresh 70374/esES TACT campaign passes 25 expected outcomes, including two
calibrated cuts. Four complete clean/resumed views across debug/release each
contain `TactKey` and `TactKeyLookup`: two files / 4,292 bytes. All four private
Rust/Python row comparisons match exactly: 69 and 197 rows, 266 per profile.
Native source bytes, CKeys, published bytes, same-profile manifests, separate
verification and verify-links agree. Private keys, rows and per-file
correlations remain outside Git, CI and model context.

The separate startup diagnostic closes three failures: native reports
`native_complete_read_failed`; debug and release report
`local_store_no_verified_encoding`. The diagnostic stops at the first requested
table, `GameObjectDisplayInfo`; `Map` is not reached. Native creates no output,
and neither Rust output has a complete manifest. This accepts no prefix and
establishes no individual 70374 Map availability result.

Final independent real review: `passed-bounded-tact-positive-and-startup-negative`, with no findings.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70374-db2-product-20261014-s1erfa6i/`.

| Receipt relative to evidence root | SHA-256 |
| --- | --- |
| `impl/source-v3/source-freeze-v3.json` | `23f202c3a96198ee03c5e8b0594fdb895fee198da2f777acfe686adc1c48b510` |
| `root/full-validation-attempt1/report.json` | `1d11e9dcc53827010e7d8bc4043c84ef4cdb6030355aed084bfc60889bf48e9c` |
| `root/producers-v1/producer-freeze-v1.json` | `235f4fd3177fa8b7e83775cb58af3287f5351fe8d884cea01ee58068aa0ce913` |
| `qa/review/root-validation-review-v1.json` | `9632af49d03944ec717378933a9ae7bb9068b08c9568b74c8700dce243ad9d3e` |
| `synthetic-attempt1/acceptance.json` | `42791ab9fd1a7dafc296e7471113fb80db5ef8d236eb68f391f8e738d8ca7c66` |
| `qa/review/synthetic-terminal-independent-review-v1.json` | `b312a0083875e1402b801e92772fdf25e1875d86fa232a2ed754b1b5775fb213` |
| `real/qualification-attempt3/report.json` | `96ab0374ea5c8df0b94911457f6c3ca85a0b8bf3e5a280bab1811044b98ac133` |
| `real/revision5/real-tact-attempt2/acceptance.json` | `03033d55368573073fe2324797dae1da1c1aef940370eac5510dc8b1ea57f6a7` |
| `real/revision5/real-startup-diagnostic-attempt1/acceptance.json` | `8542c2acf52cdab4895d40cba138b2ccc062e424d8d2141a5ef72e3ff7569ca4` |
| `qa/review/real-terminal-independent-review-v1.json` | `fb1008dd5cc6c697b787c5f3d466ec07ee46cdfeabb45ff45017015e457d0b0c` |

This accepts the bounded Linux DB2 profile and TACT selection. Full catalog
acquisition, real geometry/world resources, game/PE authentication, RustyCore
runtime and a new reproducible Linux bundle remain gates. Existing 70338
evidence keeps its dated scope. Windows development/distribution remains
deferred, with draft PR #27 preserved. The full extractor goal remains active.

## Bounded 70374 key coverage closes negatively — 2026-10-14

Issue #58 follows merged PR #57 (`5aba789`) using the same frozen product
producers compiled from `52db8418587d6db2994bb91137aee11a67d5d6e8`.
Four independent Rust-local requests cover GameObjectDisplayInfo and Map in
esES and enUS: three encrypted results, one missing result and zero bytes.
This finite diagnostic does not survey the full catalog.

The corrected strict native observer also makes four independent requests.
It captures a TRUE-but-short read's error before another CascLib operation:
three read failures report error 1005 with causal missing-name observations,
representing two unique names; one size query fails with error 1007.
Zero files complete. These outcomes do not establish Rust/native error-category
parity. A causal getter exposes the last missing name in that fresh storage,
not every encrypted frame. Native built-in keys remain a separate baseline.
The earlier native actual attempt remains failed. Coverage V1 remains
unexecuted after a metadata mismatch in source review; V2 fixes that seam.

The fresh accepted TACT inputs have 69 direct rows and 197 Lookup rows.
Joining by ID and supplementing from the authenticated `a3da648` public snapshot
yields 116 names; all 69 direct pairs agree exactly with the public source.
The old header-declared 70338 cache supplies three direct TactKey candidates
joined to the fresh Lookup, all already public. Its separate BroadcastText-tail
profile yields zero candidates. Neither operation applies a hotfix overlay or
establishes 70374 cache compatibility.

Coverage V2 compares the base 116, public 19,685 and cache three entries with
only the two causally observed needed names. It covers zero of two, counts
119 identical source pairs collapsed and exports zero files. No supplied-key
retry is justified by this result. Key values, names and correlations stay
private; no positive decryption/availability acceptance or accepted prefix follows.

Final independent QA closes 2,097 software guards, two empty helper network
traces and absent owned groups. These helper traces are separate from native
and product traffic; counts are not OS I/O, RSS or an atomic-snapshot claim.
The retained failed and unexecuted evidence is preserved. At this checkpoint
official client refresh is ongoing; refreshed-client acceptance requires new
evidence. Production source/producers remain unchanged, with no new product
binary or bundle. Full catalog, geometry and runtime gates remain open;
Windows development/distribution stays deferred in draft PR #27.

Evidence root is the same 70374 DB2-product lab recorded above.

| Receipt relative to evidence root | SHA-256 |
| --- | --- |
| `root/next-availability-v1/actual-attempt1/receipt.json` | `81eb0fb79c104cced2e8bbbc932a4617c77f77ed0f436f31c316c1d912d1dfaa` |
| `root/next-availability-v1/native-key-actual-invocation-v2/receipt.json` | `6658a6701eafb37ec8288fc920c16030038c2a476045a21dd586ac1049264aa2` |
| `next-cache-tactkey-v1/actual-attempt1/acceptance.json` | `1cb0aee02daaf1e2b9a6243d071acf07475c69a4283d3cccf2a75e7ec7ae6ed9` |
| `next-key-coverage-v2/actual-attempt1/acceptance.json` | `a5c7fa8eb4d8a092df59015c6db990c0743459a62a54ef499c89589f4db5a924` |
| `qa/review/key-coverage-cache-terminal-independent-review-v1.json` | `ccf115f8f97deb0d4a234c8dfde428fb31a6009591f246144c9e4c76696f19c0` |

## Explicit Linux 70374 DB2/GT/raw-camera product v11 — 2026-10-14

Issue #60 adds canonical plan/contract v11 and `PublicationProfile::Build70374Db2GtCamerasV1`:
explicit Linux Rust-local build 70374, `wowdbdefs-70374-v1` and exactly
`components=db2,gt,cameras` (mask 7). The independent catalog contains 1,161 schemas. V10
remains DB2-only, v9 remains the explicit 70338 integration, and their evidence retains its
dated scope. Session, global native, standalone component and Windows build gates do not widen.

Execution remains serial DB2 → GT → cameras. CinematicCamera is mandatory in the explicit DB2
selection. The published sealed source must match the admitted source bytes before camera
discovery, including when resume skips completed DB2. Discovery validates the complete
direct-row source and returns the whole sorted camera set; it does not replay copies or parse
camera M2/playback data. Package verification preflights every artifact's metadata before
allocating payload buffers, including the source-inclusive camera total independently of
ordering. The limits remain DB2 16 MiB/file / 16 GiB total, GT 4 MiB/file / 80 MiB total, at
most 4,096 cameras / 16 MiB each / 256 MiB including source exactly once. The canonical plan,
package format, state format and supplied-key policies retain their existing bounded semantics.
No geometry or navigation component is selected.

Functional source `34d9e9c74e7973b5d6e1e0fd5f019aa0feb706ad` matches all 717 selected files /
18,625,984 bytes, including 465 Rust/Cargo files / 9,871,316 bytes. Compilation precedes that
Git commit; the binding proves exact source equality, not recompilation at the commit. This
documentation-only update changes the two selected product/completion documents: 715 of 717
selected source files remain unchanged, including all 465 Rust/Cargo files / 9,871,316 bytes.
AGENTS.md is outside that selected freeze. No new compilation is claimed for the documentation
commit; its exact binding is recorded separately. Debug/release each pass 2,987 workspace tests
with 26 historical ignores. Clippy, formatting, 363 source-contract checks, 162 distribution
checks with one platform skip, catalog regeneration and both product builds pass. Six
implementation-head hosted checks pass; final-head CI remains a separate publication gate. These
distribution checks do not build or accept a new reproducible bundle.

The first bounded real release probe, with supplemental keys disabled, produces 31 files /
165,507 bytes: one 674-byte source, twenty GT files / 138,537 bytes and ten opaque cameras /
26,296 bytes. Separate offline verification and verify-links pass. This one extraction and two
offline commands are outside the later 26-call lifecycle campaign and by themselves prove
neither native parity nor resume.

Fresh Phase A strict native source acquisition validates complete EOF and content-key MD5. Rust
debug/release, full-frame Python validation and original C++ memory discovery agree on ten
direct records, zero copies and the entire fresh ten-camera set. Four child commands pass; their
traces contain zero network calls and their owned groups are absent. The parent is untraced. The
first double-ptrace wrapper attempt remains failed before native source acquisition; the
separately reviewed direct-parent launcher fixes that tracing conflict. No historical camera set
substitutes for the fresh discovery proof.

The helper qualification preserves its first failed whole attempt: fifteen compiler/static
commands pass, then the first authored trace-control process fails strict trace acceptance after
denied AF_UNIX attempts. The compiled prefix remains a build-only receipt. A runtime-only
corrected qualification passes all 23 new authored child calls with zero new compiler commands:
one Python child / four trace methods, one native plan child / six checks, one two-policy
fixture child, fourteen C++ numeric children, two Rust projections and four pre-I/O native
rejections (1 + 1 + 1 + 14 + 2 + 4 = 23). Actual environment correction is recorded without
claiming a proven NSS cause. Original CascLib implementation units are reused, not rebuilt; no
client acquisition occurs in those authored qualifications. The accepted whole report does not
relabel the preceding failure or claim that both binaries were freshly compiled in the retry.

The complete synthetic campaign passes 56 outcomes: 44 exit0, eight calibrated SIGKILL cuts and
four expected pre-I/O key-policy rejections. Both producer profiles and both key policies yield
twelve complete 24-file / 1,632-byte views. Provided-key fixtures require the authored key
through DB2, GT and camera routes. Independent QA checks all child outcomes, raw traces,
immediate cut snapshots and the exact 1,104-file / 113-directory evidence freeze. Fresh cuts are
GT ordinal 27 and camera ordinal 170 for both producers and policies. DB2 remains sealed and
unchanged across each resume; camera cuts also retain the GT seal. These synthetic ordinals are
the only calibrated cuts admitted for the subsequent real phase.

Fresh native component acquisition completes all twenty GT files and all ten fresh cameras.
Together with Phase A's credited source, 31 artifacts / 165,507 bytes match complete Rust
publication bytes and content-key MD5. Source bytes are credited once rather than acquired
again. Fourteen authored source-style C++ numeric oracles and both Rust projections match 22,082
f32 values in 5,086 data rows. Six GT entries remain opaque. Neither original GameTables.cpp nor
either typed GT loader executes; this is not a new typed-loader acceptance claim. All 46 child
commands pass with zero observed child network calls and absent owned groups. The parent is
untraced, so no parent network-observation claim is made. Independent native QA accepts the
recorded metadata and trace scope.

The real disabled-key campaign closes all 26 outcomes: 22 exit0 and four synthetic-calibrated
SIGKILL cuts. Debug/release each produce clean, GT-resumed and camera-resumed packages: six
complete views, each 31 files / 165,507 bytes. All 186 complete output-to-oracle byte
comparisons pass, with exact inventories, same-profile manifests and unchanged sealed source.
All child groups are absent, child traces show zero network calls and the parent remains
untraced. Outer elapsed time is 241.235 seconds. The 602-file / 49-directory evidence freeze
excludes itself. No new compiler, original consumer or real calibration runs. Acceptance SHA256:
`323677ea23da96edb82a72c475ee189a7809c3e9e8064f085fa0efeee4553ffa`.

Final independent real QA accepts the bounded whole lifecycle, with no findings. Status:
`passed-bounded-real26-lifecycle`; receipt SHA256:
`968d047b2a48ba10f75a3cfb1e3c54d90c7651ff721f59601d0c5be7ee5e53ba`. No successful prefix
substitutes for the whole selection.

### Bounded v11 receipt identities

Only public aggregate results and complete receipt hashes are listed here. Private camera
identities, key material, CKeys, per-file hashes and correlations remain outside Git, CI and
model context.

| Receipt | SHA-256 |
| --- | --- |
| Compiled source freeze | `8bf8aef01e06ca21524164f1151d45f36e6fd9e2d3aa2123a081be210d2a5926` |
| Full validation | `12a4747d5dc56c5f83b507d91bd96c6fd64bf3173276eb78d7b6cc844d90dbf2` |
| Frozen producers | `5a12dc8d9ec2c7c5aee0ff656b756b7ed8c1cb061dcb942a38786755b6fada6f` |
| Independent producer QA | `179cb63ddd51fe38b1421806362327ddacede7dec4ddd32370087ec68c230bcb` |
| First publication probe and offline checks | `fb9a41a96bb55057c8e76e0cc364db32c7c3a410e4982bb97193b445b172da06` |
| Phase A source/discovery | `4cf9161574bb2ed62c8a1580aa059028aae76dd8da1876c57ce4136086c03212` |
| Independent Phase A QA | `09487e410adedc0726ea078f88d06c8fb022e84a5d33ba31b36c9432965c922f` |
| Corrected whole helper qualification | `edf5de35e67502688d0c462a34a1fa7099ae63796309478f4fe54e70ee47f7b3` |
| Independent helper QA | `364d7d43bef9e6e15422d7c325576b0cd03a7de15d228ac4598d76332e4d25df` |
| Synthetic56 acceptance | `09c0dae4c756c490db0ec2e522c147a3e9c3bc6976bacd42ee7131fb28b4eb9a` |
| Independent synthetic QA | `31f055335310384dd8fc217d03b1fd74890fd25100544740f0ca5cbe9bca3e64` |
| Native component terminal | `813697db3b556eaca86be1da091f5a34784f24b8d275f87f15805f2c63455922` |
| Complete byte-oracle metadata | `31821dd7ed39a94a6c81f50ba8d2789e8ec7f8fd21f5b1a50ed2dda5528e5bd0` |
| Independent native QA | `d94d2be86c56b2c774421c2153f3f1c6fba794d19740cc005fa9cc608eb4dc0c` |
| Real26 lifecycle acceptance | `323677ea23da96edb82a72c475ee189a7809c3e9e8064f085fa0efeee4553ffa` |
| Real26 evidence freeze | `07812f118bef0691d9615285d1e1c31999388ca5e43d476be631c13aba64a97b` |
| Independent real26 QA | `968d047b2a48ba10f75a3cfb1e3c54d90c7651ff721f59601d0c5be7ee5e53ba` |

These are bounded selected raw outputs. They establish neither a full 1,161-table catalog
acquisition nor Map/GameObject availability, real geometry/full-world resources, camera playback
or RustyCore runtime/server activation. Native built-in keys and Rust's disabled
supplemental-key policy are distinct baselines; matching bytes do not establish general
decryption parity. Counters record logical QA visits/outcomes, not all CASC requests, physical
archive I/O, RSS or atomic snapshots. A new reproducible v11 Linux bundle and any
release/publication remain separate gates. Prior bundles are unchanged; Windows
development/distribution stays deferred with draft PR #27 preserved. The full Linux extractor
remains incomplete.


## Complete 70374 DB2 catalog: first attempt rejected — 2026-10-14

Issue #64 follows merged PR #63 (`76f4009`) with the accepted shipped Linux
executable, without changing production source, admission, formats or dependencies.
The explicit v10 DB2-only plan contains all 1,161 `wowdbdefs-70374-v1` entries
(34,986 bytes), esES and disabled keys. Plan-only inspection passes without a
client argument and is separate from the three real/output-check calls below.
A persistent user service executes one bounded offline attempt; no retry follows.
Fresh client metadata declares 70374 and remains unchanged before/after those
calls. This is metadata evidence, without authenticating game execution.

Extraction exits 9 with `local_store_no_verified_encoding`. Its two provisional
DB2 files total 512 bytes; the incomplete marker remains, with no DB2 seal or
final manifest. Independent offline `verify` and `verify-links` each exit 8 with
`package_incomplete`. The full selection fails and no available prefix is
accepted. All three owned groups close, the three consumer network traces are
empty, and software guards remain unchanged. Independent terminal review passes
this negative closure without findings; it does not accept the full catalog.

The sequential public plan and exact provisional filename/length prefix identify
`Achievement` (public FDID 1260179), entry three, as the first failed acquisition.
No provisional DB2 contents are opened for this attribution. The failure precedes
DB2 schema validation. This generic acquisition error alone does not establish
encryption, a missing key or native error-category parity. The historical 70338
all-table survey and issue #58's Map/GameObject missing-name coverage do not
transfer to Achievement or this 70374 selection.

The next bounded diagnostic is a source-bound singleton native causal observer
for Achievement/esES, followed only by matching coverage of a causally observed
missing name against the retained 116-entry and public 19,685-entry sets. The
existing four-case observer cannot be relabeled or retargeted without a reviewed
authored-wrapper build. No complete-catalog retry or supplied-key retry follows
from this negative product result. Diagnostic source/qualification and actual
execution remain separate gates at this checkpoint.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70374-complete-catalog-20261014-q52765nk/`.
Only aggregate facts and overall receipt hashes are recorded here; client assets,
keys/names and private file correlations stay local. Map/GameObject acquisition,
real geometry/world resources, RustyCore runtime, full extraction and publication
remain open. Windows stays deferred in draft PR #27.

| Receipt relative to evidence root | SHA-256 |
| --- | --- |
| `root/initial-negative-v1.json` | `17121611e6f39d07edc8095f1c670031d3137fdfa258e45888e22ec3678c24d1` |
| `attempt1-invocation/receipt.json` | `fcf59971007b3b72e8047e5ec6d7ea4198519f5caf42427678c1490f8b63834c` |
| `verify-attempt1-invocation/receipt.json` | `ab3bfcc48a9f8728f27c5f42ddc47b580b4ebf4d2977d1717894a485839314d7` |
| `verify-links-attempt1-invocation/receipt.json` | `8bc2b5b027eeef21777001d3a53fe6b5186d0b2f9c9ec9e25437a3914294dc9e` |
| `qa/review/complete-catalog-attempt1-terminal-review-v1.json` | `e30de1a7399629055316b0e1d95ea9e545d750cb29da61e4fa221556fff03d17` |
