# Forever product interface v1–v6

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
for builds 70170/70205. It does not extend build 70338 beyond v5's DB2 scope.

## Build and inspect

```bash
source .opencode/environment.sh
cargo build --offline --locked --release -p forever-extractor
"$CARGO_TARGET_DIR/release/forever-extractor" inspect --plan /absolute/private/plan.tsv
```

No selected file is read by `inspect --plan`. Adding `--client` admits the client
and explicitly specified ROOT/index layouts, builds the local metadata store
and returns its context. This is not an availability survey. `--help` documents
the four commands; every ordinary result/error is path-free JSON.

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
subsequence explicitly. GT selects all 20 files (14 numeric and six raw-only).
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
