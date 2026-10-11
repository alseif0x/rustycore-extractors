# Forever Rust foundation — metadata inspection

Issue [#18](https://github.com/alseif0x/rustycore-extractors/issues/18) introduces
the first Rust code in this suite. It is independent of RustyCore's `wow-world`
refactor and the paused character/world integration.

## What exists

`forever-db2` is a dependency-free, safe-Rust library with one diagnostic binary,
`forever-db2-inspect`. It reads an explicitly supplied local DB2 **as metadata**:

- WDC5 signature and version 5;
- the 204-byte packed header, preserving signed `IndexField`;
- the ordered 40-byte section headers; and
- the source's maximum-one parent lookup condition.

An inspector-specific maximum of 1024 sections bounds reads to **41,164 bytes**.
That policy is not a claimed native format limit. Large declared record/field
counts never trigger record allocations. A nonzero TactId is reported only as
`tact_declared`; neither the identifier, encryption keys, opaque Schema bytes,
client paths nor record bytes are emitted. It does not resolve keys or invent
readability for encrypted sections.

Success means **metadata readable**, NOT a complete file, exact table layout,
client/build identity, decodable record body or server compatibility. A locally
acquired `.available.db2` prefix may have readable metadata but unavailable
record bodies. The report always sets `client_build_verified`,
`table_schema_validated` and `record_data_validated` to `false`.

This metadata inspector implements no CASC opening, decompression, key download,
record decoding, maps, VMaps or MMaps. The separate optional CASC read prototype
below is a later continuation. Existing C++ tools and their
3.4.3/51943 acceptance stay unchanged. Legacy issue #8 is not closed by this.

## Build and use

Rust **1.98.0**, pinned in `rust-toolchain.toml`. No Cargo dependencies or build
script; normal builds require no network once that toolchain is installed.
Keep compilation outputs outside this checkout and the client:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$(pwd)/../rustycore-extractor-work/builds/rust-forever"
cargo build --offline --locked --bin forever-db2-inspect

"$CARGO_TARGET_DIR/debug/forever-db2-inspect" --file /path/to/local/table.db2 --json
```

The command never creates output files. It opens the input read-only and makes
no network calls. Human mode prints a short summary; JSON mode prints exactly
one path-free JSON document with an empty stderr for handled errors.

This preview report has `report_version: 1` and is **not** the legacy v1
`command-result` or `data-manifest` contract. No unified `extract` command or
promotion manifest is implied. Its exits are 0 (metadata inspection success),
2 (invocation), 3 (unsupported signature/version), 4 (I/O/truncation/parent
metadata), and 6 (section policy limit).

## Evidence and provenance

Reviewed reference: TrinityCore
`02245dcd245e7433e524577656177723d3e4992e`,
`src/common/DataStores/DB2FileLoader.h:31-67` (packed `DB2Header` and
`DB2SectionHeader`) and `DB2FileLoader.cpp:1726-1793::LoadHeaders` (endianness,
signature/version and parent lookup checks). WDC4 offsets are not reused.
Table-specific `DB2Meta` conditions and the remainder of `LoadHeaders` are
outside this metadata-only implementation.

The new Rust implementation uses the documented wire structure; it imports no
upstream source or new dependency. Existing TrinityCore/third-party notices and
the legacy source-closure audit remain intact. This does not extend that audit
to a future CASC dependency, which needs its own license/offline/byte-parity
decision before integration.

Acceptance commands, sequential with one Cargo job:

```bash
cargo fmt --all -- --check
cargo test --offline --locked --workspace --all-targets
cargo build --offline --locked --bin forever-db2-inspect
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
FOREVER_DB2_INSPECT="$CARGO_TARGET_DIR/debug/forever-db2-inspect" \
  python3 -m unittest discover -s tests/forever -p 'test_*.py' -v
git diff --check
```

Synthetic tests cover every header/section offset, signedness, ordered sections,
all truncated prefixes, rejected signature/version/parent count, exact/exceeded
section bounds, no body reads, redacted I/O and CLI/report behavior. Real local
comparison evidence must retain source/command/SHA and scope; never copy client
files into the checkout or claim this is a full extraction acceptance.

### Executed acceptance — 2026-10-03

Tested code SHA: `b1cb582a249c6efc3aaea345108c0120ad2e061b`, authored by
`alseif0x`. Local x86_64 Linux, Rust 1.98.0, one Cargo job; target directory
outside both checkouts under `rustycore-extractor-work/builds/rust-forever`.
The measured campaign was **14:11:56–14:12:42 UTC (46 seconds)** including
the initial tiny-crate compilation, checks below and local comparisons. This
is this tool's acceptance, not a RustyCore warm-workspace benchmark.

- `cargo fmt --all -- --check`: pass.
- `cargo test --offline --locked --workspace --all-targets`: 7 library tests
  pass; the binary unit target has zero tests and contributes no test evidence.
- `cargo build --offline --locked --bin forever-db2-inspect`: pass.
- `cargo clippy --offline --locked --workspace --all-targets -- -D warnings`: pass.
- `tests/forever/test_metadata_cli.py`: 5 tests pass against the real binary.
- Legacy `tests/contracts`: 10 tests pass; no legacy C++ code/contract changed.
- `compare_local_metadata.py --binary <binary> --file <local-input>` repeated
  for `ChrClasses`, `ChrRaces`, `Map.available`, `Achievement.available` and
  `SpellEffect.available`: all 5 pass against independent Python packed-header
  unpacking. Inputs were prior private build-70170 acquisitions, read in place;
  no client data was copied into this repository. This is metadata comparison,
  NOT a C++ runtime oracle, record/body acceptance or fresh CASC acquisition.
- Linux `strace -f -e trace=network -e inject=socket:error=EPERM` around the
  five CLI tests: all pass, zero network syscalls observed. Trace stays outside
  Git under `rustycore-extractor-work/reports/forever-db2-18`. This proves those
  Linux scenarios, not untested platforms or OS-mounted remote filesystems.
- `git diff --check`: pass.

No CASC extraction, Windows acceptance, full DB2 record reader, server
installation/restart or original character/world-goal resumption occurred.
Subsequent evidence closeout is documentation-only; do not relabel the tested
SHA above as a later build.

## Next dependency

Select and prove a **local-only CASC access path** for `wow_classic_beta` build
70170, including product/build admission, locale selection, FileDataID bytes,
BLTE and encrypted-file behavior. A Rust wrapper around C/C++ is a possible
candidate, not a pure-Rust CASC claim. Typed DB2 decoding and server-reader
conformance follow that proof; maps/navigation are later responsibilities.

## Optional local CASC read prototype — continuation

The operator authorized continuing after the metadata delivery. `forever-casc`
adds a Rust-owned local read path behind explicit feature `native`; without it,
the workspace does not compile/link a native backend. It is **not pure-Rust CASC**.
Only Linux native-host builds are admitted; no Windows/cross-build claim.
The [separate licensing/build closure](licensing/forever-casc-prototype.md)
records the pinned external dependency and why the legacy backend is not reused.

Rust first checks exactly one active `wow_classic_beta` row with version
`1.60.1.70170` or `1.60.1.70205` in the explicitly supplied installation root's `.build.info`.
The private native ABI v2 receives that selected build and independently matches
the storage's product/build before any
FileDataID read. Locale is explicit (`enUS` or `esES`) and selected at storage
opening, not by the ignored native file-open locale argument. This is admission
of installed CASC metadata, not signature verification of a game executable.

The native backend has no socket transport, uses a download-cancelling progress
callback and opens storage offline. No key file, listfile or remote option is
accepted. File opens use strict integrity checks and **never**
`CASC_OVERCOME_ENCRYPTED`. Missing keys/local blocks produce errors; partial
bytes are not exposed as successful files. The 128 MiB maximum budget is a
prototype policy, not a CASC format limit. A complete read plus optional byte
comparison still does not decode/validate DB2 records or a table-specific schema.

```bash
# External, unchanged checkout at 02245dcd245e7433e524577656177723d3e4992e.
# No implicit download occurs. Compiler inputs are checked against Git blobs.
export RCE_FOREVER_CASC_REFERENCE=/path/to/pinned/TrinityCore
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$(pwd)/../rustycore-extractor-work/builds/rust-forever"
cargo build --offline --locked -p forever-casc --features native

"$CARGO_TARGET_DIR/debug/forever-casc-probe" \
  --client /path/to/installation-root --locale esES --file-id 1361031 \
  --compare /path/to/prior/ChrClasses.db2 --json
```

Build prerequisites: CMake 3.25+, C/C++20 compiler and OpenSSL 3 development
files. Native compilation uses the retained zlib vendor but never builds the
legacy extractor executables. Input/build/output directories stay separate.
The CLI writes only a single redacted JSON diagnostic (even without `--json`),
never extracted files or client bytes. `reference_bytes_equal: null` means no
comparison was requested, not a parity pass. Exit 9 marks unequal comparison
bytes. Other handled exits use the existing preview meanings (2/3/4/6).

Acceptance adds native-feature tests/build/clippy and `test_casc_cli.py`, then
explicit real-client byte comparisons and missing-ID/key/budget scenarios under
network syscall tracing/denial. Executed results are recorded below when known;
source existence alone does not establish acceptance. Maps/VMaps/MMaps, typed
record decoding, complete category extraction/manifests/resume and reader/runtime
conformance remain outstanding. No server-world integration is resumed.

### CASC executed evidence and target-change boundary — 2026-10-03

Tested repaired code SHA: `3a84773f37cb4ec1f52ad55c67a8b559d129f103`.
The optional native Linux build/test campaign uses Rust 1.98.0, GNU C/C++ 15.2,
one job, the same external target directory, and the pinned modern reference.
No source/fixture changes occurred during that candidate's checks.

- Format, native-feature workspace tests/build and Clippy with `-D warnings`:
  pass. **14 Rust tests** execute (6 CASC library, 1 CASC CLI options, 7 DB2).
- Real-binary synthetic CLI tests: **10 pass** (5 CASC, 5 DB2).
- Legacy contract tests: **10 pass**. The source closure check verifies all
  **355 imported files / 10 modification notices**. Legacy vendor/CMake/adapters
  and source/import manifests are byte-identical to published `09e70951`.
- Network-denied `strace` of those 10 synthetic CLI tests: pass, zero network
  syscalls observed. Trace stays in the external work reports, not Git. This
  covers synthetic admission/missing-storage cases, not a successful CASC read.

Failures remain recorded: candidate `40e673b` first failed configuration because
OpenSSL development headers were absent. A local, non-system bootstrap used
Ubuntu `libssl-dev_3.5.5-1ubuntu3.7_amd64.deb`, 2,936,724 bytes, SHA256
`d1cf6cafb141490b7020e392964b1f901835d60cb88c3a68ef0f95ef5e930de4`, downloaded
explicitly from the Ubuntu security repository over HTTPS and extracted outside
both checkouts. It links the installed OpenSSL 3.5.5 libcrypto. No system package
was installed. Nonstandard header locations can use `RCE_OPENSSL_INCLUDE`,
`RCE_OPENSSL_ARCH_INCLUDE` and `RCE_OPENSSL_CRYPTO`; normal development hosts
can use CMake's discovery. Dependency bootstrap is not extractor runtime network.

Candidate `5bcfe36` compiled/tested Rust but failed the incomplete `.build.info`
CLI case (no JSON report). Source review found that
`CascFiles.cpp:607-726::ParseFile_BuildInfo` dereferences exact typed Active
columns without checking null. `3a84773` adds pre-FFI typed-column, nonempty
Active/product, hexadecimal config identifier and Tags bounds validation plus
regression coverage. This is a bounded guard for a trusted local installation,
not a claim that native CascLib is memory-safe for all adversarial client data.

**Real target acceptance is NOT passing:** during this turn the normal client's
active Beta row changed from `1.60.1.70170` to `1.60.1.70205`. The explicit
`--locale esES --file-id 1361031 --compare <prior-ChrClasses.db2>` command rejects
it with exit **3**, `unsupported_client`, before native access. A pre-existing
private 70170 metadata snapshot was also inspected; its Data is a symlink to the
updated installation, not an independent archived client. The same probe against
that snapshot exits **4**, `native_read_failed`. No old client reconstruction,
metadata spoof, source/build gate bypass, client downgrade or new-build support
is performed. No successful BLTE/FileDataID byte-parity, encryption failure or
real-data budget acceptance is inferred from synthetic checks.

Campaign wall-clock checkpoint: **18:17:52–18:29:25 UTC, 693 seconds**, including
the two failed attempts, dependency bootstrap, error repair, final local checks,
real probes and investigation. The 600-second target is **not met**; cold native
bootstrap and repair are included here, not mislabeled as a warm benchmark.
The later documentation-only closeout extends that elapsed checkpoint; prior
tested SHAs are not relabeled. Detailed pure coding/repair duration is unknown.

The operator subsequently authorized adding 70205 alongside 70170; this resolves
the target choice for the bounded read prototype only. The original server character/world objective stays
paused. This foundation code is local, unpushed, and the complete extractor is
unfinished; no release/manual-test readiness is claimed.

## Build 70205 read admission — 2026-10-03

The operator approved 70205 support alongside 70170. Production candidate
`312c654` admits only the two exact versions. `admit` returns the selected build;
private C ABI v2 takes that expected build, rejects any other build before storage
access, and matches `CascStorageProduct` against it before opening a FileDataID.
Reports carry the matched build rather than a fixed 70170 constant. This retains
the existing pinned source/licensing closure, offline policy and strict reads.
It does not change the legacy suite or the paused RustyCore server.

Acceptance on Linux x86_64 used the existing external warm target and one Cargo
job. From 21:44:18 to 21:44:21 UTC, `cargo fmt --all -- --check`, offline locked
workspace/all-target tests with `forever-casc/native` (15 Rust tests), workspace
native build, native all-target Clippy with `-D warnings`, ten synthetic Python
CLI checks, source closure (355 paths / 10 notices), ten legacy contract checks
and `git diff --check` passed at `312c654`.

Test-only candidate `998858fda71d0514ca8fd667a50cab1cb3aadda8` adds opt-in
`tests/forever/test_live_casc_cli.py`; production/build inputs are unchanged from
the Cargo-tested candidate. All 13 Python CLI checks, including the three live
checks, passed at this candidate in 9.008 seconds. Every live subprocess ran
under strace network-syscall error injection and the trace showed no network
attempts. No client files were written and no private rows became fixtures.

Current installed 70205, explicit `esES`:

| FileDataID | Table | Complete bytes | WDC5 records / fields | Result |
| --- | --- | --- | --- | --- |
| 1361031 | ChrClasses | 6200 | 9 / 43 | matched build 70205 |
| 1305311 | ChrRaces | 22095 | 58 / 51 | matched build 70205 |

An additional full-byte comparison of current ChrClasses against the prior
private **70170** acquisition returned equality. This is cross-build unchanged
table evidence, not an independent newly acquired 70205 oracle. The one-byte
budget returns exit 6 / `read_policy_limit`, and absent FileDataID 4294967294
returns exit 4 / `file_unavailable`; neither exposes partial success.

The complete acceptance interval, including smoke-test authoring, repeated
Python acceptance, real reads and comparisons, was 21:44:18–21:46:22 UTC
(124 seconds, within 600 seconds). The earlier failed cold campaign remains
recorded above. Documentation closeout is outside this measured interval.

70170 remains unit/CLI admitted, but its stale local snapshot still cannot prove
a current complete 70170 live read. Encrypted sections, typed row decoding,
table-specific schema conformance and full extraction remain unproven here.
Changes are committed locally, not pushed or released.

## Implementation levels, CLI and report semantics — Rust preview

The Forever Rust preview is deliberately split into three separate levels. Do
not read success at one level as evidence for a later one.

1. **Header/metadata inspection** — `forever-db2-inspect`. Reads the 204-byte
   packed header and the ordered 40-byte section headers of one explicit local
   WDC5 file. It does **not** decode records, column data or strings, and it
   makes no CASC, key or network access. Output scope is `metadata_only`;
   `client_build_verified`, `table_schema_validated` and `record_data_decoded`
   are always `false`.
2. **Bounded record decoding** — `forever-db2` `DecodedTable`, used by the
   `forever-db2-extract` batch CLI. As of 2026-10-11 it admits **56 reviewed
   schemas**, including name/category, appearance, item specialization, Birth,
   initialization and scalar spell dependencies. Local acceptance covers 54
   complete artifacts; incomplete `SkillLineAbility` and `Map` inputs block real typed
   Birth and initialization acceptance respectively.
   `ChrClasses` (1361031) and `ChrRaces` (1305311) remain the default plan; other tables
   require an explicit file list. Supports an empty table or one unencrypted
   regular section, with reviewed inline/separate IDs, arrays, copy rows and
   the documented parent-lookup variants. Offline, no keys, per-file budget
   16 MiB and per-batch budget 256 MiB, at most 256 explicit entries. Sparse,
   encrypted and multi-section record variants are rejected explicitly.
   Unreviewed tables may be acquired as metadata-only artifacts but receive
   no schema or record-decoding claim.
   There is **no** encrypted decoding, no listfile discovery and no complete
   catalog.
3. **Full extraction** — **NOT implemented**. There is no map/VMap/MMap output,
   no complete table catalog and no global v1 `data-manifest`. Level 3 remains
   future work and must not be inferred from levels 1–2.

### Report semantics

The batch report keeps three independent facts per file, and they are not
interchangeable:

- `table_schema_validated` — `true` only when the file's `table_hash`,
  `layout_hash` and `field_count`/`total_field_count` match the reviewed schema
  identity. It is computed independently of body decoding. The header's signed
  `IndexField` is **not** part of this check: the pinned reference only
  endian-converts it (`DB2FileLoader.cpp:1746`) and decides inline vs. id-table
  from the reviewed metadata `Meta->HasIndexFieldInData()`
  (`DB2Meta.cpp:22-25`, `Meta->IndexField != -1`), which already disagrees with
  the real 70205 ChrRaces header word (`0` vs. the reviewed `-1`).
- `record_data_decoded` — `true` only when every row of a reviewed table decoded
  successfully (with `decoded_rows` as the count). Unreviewed names are `false`.
- `declared_records` — the header's declared record count, **not** a verified or
  fully decoded count.

The metadata-inspector JSON uses the same `record_data_decoded` key for
consistency; there it is always `false`. Note: `forever-casc-probe` carries the
build matched by the native storage independently of any fixed constant.

When a file is fully read and its metadata parsed but a reviewed table's
`DecodedTable::read` fails, the batch emits a `status:error` row that still
carries `code` **plus** `table_schema_validated`, `declared_records` and `bytes`,
so schema identity and body decoding stay independent observable facts. The
partial `*.db2` is not published.

### CLI usage and exit codes

Build and outputs stay outside this checkout and outside the client. Replace the
`<...>` placeholders with your own local paths; no client path is ever written
into a report:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$(pwd)/../rustycore-extractor-work/builds/rust-forever"
cargo build --offline --locked -p forever-db2 --bin forever-db2-inspect
cargo build --offline --locked -p forever-casc --features native \
  --bin forever-db2-extract

"$CARGO_TARGET_DIR/debug/forever-db2-inspect" --file <local.db2> --json
"$CARGO_TARGET_DIR/debug/forever-db2-extract" \
  --client <client-root> --locale enUS \
  --output ../rustycore-extractor-work/runs/candidate/<run-id> \
  [--manifest <name-tab-id.tsv>]
```

`--manifest` is an explicit `name<TAB>decimal FileDataID` text file (≤ 64 KiB,
≤ 256 entries, unique non-zero ids, reviewed-name/id consistency); without it the
default two character tables are used. `forever-db2-extract` prints exactly one
redacted JSON document (`report_version: 1`). Handled codes/exits:

| Exit | Codes |
| --- | --- |
| 0 | run complete |
| 2 | `invalid_invocation`, `invalid_manifest`, `manifest_policy_limit`, `unsupported_locale` |
| 3 | `unsupported_client` |
| 4 | `input_io`, `manifest_input_io`, `native_read_failed`, `file_unavailable`, `encrypted_key_unavailable`, `local_file_incomplete` |
| 6 | `read_policy_limit` |
| 8 | `incomplete`: finished but at least one entry failed |

Refusal to create output (`unsafe_output_path`, `output_exists_or_unavailable`)
never leaves a silently partial directory: no directory is created, or the run
fails before publishing. If the final `report.json` cannot be published, the run
returns an error and any already-published `*.db2` files remain without a report
(an explicit, detectable failure, not a success).

### Forever v2 output contract and manifest

The batch publishes a **separate version-2 output contract line** for the Forever
target, kept distinct from the legacy 3.4.3/51943 v1 contracts and never
re-labelling them:

- `contracts/forever-contract-v2.json` — the Forever contract: only `wow_classic_beta`
  builds `70170`/`70205`, locales `enUS`/`esES`, `scope: explicit_db2_batch`,
  `full_client_extraction: false`, `WDC5` version 5, local-offline network and the
  shared inventory/privacy rules.
- `schemas/forever-manifest-v2.schema.json` — the JSON Schema that rejects the v1
  shape, so the two lines cannot be relabelled into each other. Its synthetic
  fixtures are checked by `tests/contracts/test_forever_contract.py`, and each
  schema is asserted to reject the other version line.
- `forever-manifest.json` — the manifest the `forever-db2-extract` batch now emits
  **alongside** `<Name>.db2` and `report.json`. It records the admitted build and
  locale, the reviewed per-table identity read from the real WDC5 header
  (`table_hash`, `layout_hash`, `field_count`, `meta_index_field`), each artifact's
  independent `table_schema_validated`/`record_data_decoded`/`decoded_rows`
  facts, a sorted inventory with its SHA-256 digest and the local/redacted
  constants. Identity fields with no runtime source (`run_id`, install
  fingerprint, git commit) stay optional and are omitted rather than invented.

This is a second output contract for the local batch; it is **not** the legacy v1
`data-manifest` nor a unified `extract` command/promotion manifest, and it does not
change the legacy suite.

### Output verification — `forever-db2-verify`

`forever-db2-verify --output <existing-directory>` is a dependency-free,
offline, read-only check of a directory already published by the batch. It opens
**no** client, CASC or remote listfile and never writes to the output; it only
reads the published `forever-manifest.json` and `.db2` files and recomputes them.
For a complete run it validates, in order (first failure wins):

1. the manifest exists, is a real non-symlink file, parses and satisfies the v2
   schema, and its admitted build is one of the supported clients;
2. the run is complete — no `.part` file, no incomplete marker and no
   `incomplete`/`failed` status;
3. every published relative path is a simple name and never a symlink;
4. the recomputed per-file `sha256`, `bytes`, sorted inventory digest,
   `file_count`/`total_bytes` and the set of published `.db2` files match the
   manifest's `integrity`;
5. each `.db2` re-reads as WDC5 and its header identity matches the pinned table
   entry (and the reviewed `ChrClasses`/`ChrRaces` identity);
6. any claimed `record_data_decoded` body re-decodes and the row count equals
   `decoded_rows`;
7. the declared network/privacy/scope constants, and the companion `report.json`
   still declares `full_client_extraction: false`.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$(pwd)/../rustycore-extractor-work/builds/rust-forever"
cargo build --offline --locked -p forever-casc --features native --bin forever-db2-verify

"$CARGO_TARGET_DIR/debug/forever-db2-verify" \
  --output ../rustycore-extractor-work/runs/candidate/<run-id>/tables
```

It prints exactly one path-free JSON document
(`scope: forever_output_verification`, `offline: true`, `read_only: true`). Exit
codes and their stable failure codes:

| Exit | Codes |
| --- | --- |
| 0 | `verified` |
| 2 | `invalid_invocation`, `MANIFEST_SCHEMA_INVALID` |
| 3 | `unsupported_client` |
| 4 | `input_io` |
| 8 | `INCOMPLETE_RUN` |
| 9 | `INVENTORY_MISMATCH`, `TABLE_IDENTITY_MISMATCH`, `UNSAFE_PATH`, `ROW_DECODE_MISMATCH` |
| 10 | `internal` |

A green exit proves the published directory is **self-consistent with the v2
contract**. It is **not** format parity, a table-schema oracle, a RustyCore
reader/runtime acceptance, or proof that the bytes match an independent
extractor; see the scope limits in the acceptance below.

### Verification status

- Schema field counts are transcribed and asserted by synthetic tests, now
  verified against the pinned C++ reference at `02245dcd`:
  `ChrRacesMeta` (`DB2Metadata.h:4182`) declares **51** fields and
  `ChrClassesMeta` (`DB2Metadata.h:3455`) **43**; the transcribed lists match
  exactly. The earlier **OPEN DISCREPANCY 50 vs 51** is resolved: the missing
  final `FT_BYTE` signed field was added. A second transcription fix corrected
  the `ChrClasses` INT block to nine signed 32-bit fields (18-26); an earlier
  over-counted tenth `Signed(32,1)` widened the meta `IndexField` 29 from
  `FT_BYTE` (8 bits) to 16 bits and read 6/9 real ids past `max_id`
  (`invalid_record_ids`). The transcribed index field is now 8 bits, pinned by
  a synthetic width test. This is schema transcription verification, not
  format/reader parity.
- Id handling follows the reference (`DB2FileLoader.cpp`): an inline index field
  is returned raw (`:747-760`, no repair), while the separate id table alone
  receives the `id <= MinId -> MinId + record` repair (`:1958-1967`). Record
  copies skip `source == 0`, a source outside the ID-index extent, or a missing
  source without error (`:613-634`), overwriting a present destination and
  rewriting only an inline id. This extent is `GetMaxId() + 1`, computed from
  direct IDs and all copy destinations (`:394-398`, `:955-977`), **not** the
  header's direct-record count. The string lookup
  `record_start + fieldOffset + stored_offset` (`:798-806`) is unchanged.
- The header `IndexField` word is **not** authoritative: only the reviewed
  schema's `index` decides inline vs. id-table, and it is not part of table
  matching. The sole flag validation is sparse rejection, `_header.Flags & 0x1`
  (`DB2FileLoader.cpp:1852`); other flag bits are ignored as the reference
  ignores them, so real `Flags == 4` metadata is accepted and decoded.
- Building, a green synthetic suite or a successful exit is **not** format
  parity, schema acceptance or a real-client oracle.
- Build **70170** compatibility is **not** claimed. Its old local snapshot points
  at the updated installation and cannot serve as evidence; 70170 remains only
  unit/CLI-admitted, and each report carries the build actually matched.
- Maps, VMaps, MMaps, encrypted sections, complete catalogs and reader/runtime
  conformance remain outstanding.

### Executed acceptance — 2026-10-04

Status: **committed locally** as `2898fc5` on branch
`18-forever-rust-wdc5-metadata-inspector-foundation` (local, **not pushed**).
Subsequent commits only document or record this accepted content; no acceptance
figure or run ID changes here.

Contract verification against the pinned reference
`02245dcd245e7433e524577656177723d3e4992e` used `git show` (the on-disk
reference file is a stub), so it reads the pinned Git object rather than the
checkout:

- `DB2Metadata.h:3455` `ChrClassesMeta` declares **43** fields (10 `FT_STRING`,
  17 `FT_INT`, two `FT_SHORT` at 27-28 and fourteen `FT_BYTE` at 29-42;
  `IndexField = 29` is an 8-bit `FT_BYTE`).
- `DB2Metadata.h:4182` `ChrRacesMeta` declares **51** fields.
- `DB2FileLoader.cpp:1746` only endian-converts the header `IndexField` (it is
  not a gate); `:1852` validates only `Flags & 0x1` (sparse); `:747-760`
  returns the inline id raw; `:1958-1967` applies the `<= MinId` repair only to
  the separate id table; `:798-806` resolves a string as
  `record + fieldOffset + stringOffset`; `:613-634` handles record copies.

Synthetic acceptance (real measured output; no client bytes):

- `fmt-check`: pass.
- `rust`: pass — `forever-db2` **43** tests, `forever-casc` **22** tests in the
  default (no `native`) build, and the `forever-db2-inspect` binary target
  **0** tests. The `forever-casc` `ffi` module's **2** tests compile only with
  `native`; they are measured as **24** in the `native` row below.
- `clippy`: pass.
- `db2`: pass.
- `synthetic` CLI tests: **18** collected, `OK`; **5** executed (the metadata
  inspector CLI) and **13** skipped because the native/live binaries and client
  selectors are unset. A skipped test is not evidence; this mode excludes
  real-client tests.
- `contracts`: **10** pass.
- `provenance`: **355** imported files / **10** modification notices.
- `native`: **24** `forever-casc` tests plus **3** probe-binary tests (**0**
  extract-binary tests); the Python CLI suite collects **18**, executes **14**
  and skips **4**, returning `OK`.

Bounded real 70205 acceptance, locale `esES`, offline and without modifying the
client:

- `live-extract` run_id `20261004T080604Z-bde949ae`: exit 0 in **4.334 s**,
  `network_attempted=false`, `client_metadata_unchanged=true`; `ChrClasses`
  (1361031) **6200 B** / **9** decoded rows and `ChrRaces` (1305311)
  **22095 B** / **58** decoded rows; `full_client_extraction=false`.
- `live` run_id `20261004T094947Z-d8c600da`: **24** Python CLI tests
  collected, `OK (skipped=1)` (the skipped comparator is the gated real-dump
  oracle), including the network-denied cases. Earlier run
  `20261004T080617Z-b9fabf75` measured **18** tests in **13.084 s**.

Forever v2 output contract and verification (local, **not pushed**): commit
`52ae57c` emits the v2 manifest from the batch and commit `85e31e5` adds
`forever-db2-verify`. Measured on build **70205**, locale `esES`, offline, with
the client left unmodified and `forever-manifest.json` published alongside the
two artifacts and `report.json`:

- batch `forever-db2-extract` exit **0**; `ChrClasses` (1361031) **9** decoded
  rows and `ChrRaces` (1305311) **58** decoded rows — the same two tables as the
  literal evidence above.
- `forever-db2-verify --output <run-root>/<run-id>/tables` (the batch was run
  with `--client <client-root>`) exit
  **0**, `status: verified`, `code: verified`, `build: 70205`,
  `artifacts_verified: 2`, `offline: true`, `read_only: true`.
- manifest `integrity`: `file_count: 2`, `total_bytes: 28295` (6200 + 22095),
  `algorithm: sha256`; `scope: explicit_db2_batch`, `full_client_extraction:
  false`, `paths_redacted: true`.
- Persisted redacted report of the batch+verify integration:
  `rustycore-extractor-work/reports/acceptance/manifest-v2-verify-manifest-v2-accept-1791107644.json`.
  `extract_exit: 0`, `verify_exit: 0`, `network_attempted: false`,
  `paths_redacted: true`, `full_client_extraction: false`. The batch reports
  `ChrClasses` **6200 B** / **9** rows and `ChrRaces` **22095 B** / **58** rows,
  each with `table_schema_validated: true` and `record_data_decoded: true`. Its
  `verify_report` carries `status: "verified"`, `artifacts_verified: 2`,
  `build: 70205`. This confirms `forever-db2-verify` was executed against the
  real batch output and returned exit **0**; per the scope limits below it proves
  **output coherence only**, not complete format parity or reader/runtime
  acceptance.

Scope limits, stated explicitly: this covers **2 tables** and one **regular
unencrypted section**, with **no maps/VMaps/MMaps**, **no 70170 compatibility
claim** (its local snapshot points at the updated installation) and no complete
table catalog. Reports and the manifest are redacted; raw client-derived outputs
stay outside Git. `forever-db2-verify` succeeds only by confirming **output
coherence** — the published manifest, inventory digests and WDC5 identities agree
with the bytes on disk — and a green exit is **not** complete format parity,
table-schema conformance, a RustyCore reader/runtime acceptance, or proof of
equality against an independent extractor. The three implementation levels above
are unchanged and remain the only source of scope.

### Independent dump oracle — value comparison only

Commit `e4d0a41` (local, **not pushed**) adds a **canonical decoded dump** and an
**independent oracle** that reproduces it without importing Rust:

- `crates/forever-db2/examples/db2_dump.rs` writes the deterministic dump
  `forever-db2-dump v1 <table>`; rows are emitted in ascending id order, fields
  are tab-separated, array elements are comma-joined, and every value is tagged
  `u:`/`i:`/`f:0x########`/`t:` with `\\ \t \r \n` escaping.
- `tests/forever/db2_oracle.py` is a from-scratch Python decoder of the same wire
  layout (it imports nothing from the Rust crate and never touches CASC, the
  network or a client directory), and `tests/forever/test_db2_oracle.py` carries
  the synthetic suite plus a comparator gated by `FOREVER_DB2_FILE`,
  `FOREVER_DB2_TABLE` and `FOREVER_DB2_RUST_DUMP`. The comparator reports only
  equality or the index of the first differing line — never any value.

Measured result on build **70205**, locale `esES`, offline, client left
unmodified: the Rust dump and the oracle's dump are **byte-for-byte identical**
for `ChrClasses` (**10** lines) and `ChrRaces` (**59** lines). This is a
comparison of **decoder values** on the two authorized tables. It is **not**
RustyCore reader/runtime acceptance and **not** complete format parity.

Reproduction (replace the placeholders with your own local paths/run id; no
client path is written into a report or the checkpoint):

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$(pwd)/../rustycore-extractor-work/builds/rust-forever"
cargo build --offline --locked -p forever-db2 --example db2_dump

RUN="../rustycore-extractor-work/runs/candidate/<run-id>"
mkdir -p "$RUN"
for table in ChrClasses ChrRaces; do
  "$CARGO_TARGET_DIR/debug/examples/db2_dump" \
    --file <client-root>/<table>.db2 --table "$table" > "$RUN/$table.rust.dump"
  FOREVER_DB2_FILE=<client-root>/<table>.db2 \
  FOREVER_DB2_TABLE="$table" \
  FOREVER_DB2_RUST_DUMP="$RUN/$table.rust.dump" \
    python3 -m unittest tests.forever.test_db2_oracle -v
done
```

The dumps are client-derived and stay inside the external run directory; only
equality/line-index evidence is reported.

## Soporte de parent lookup y evidencia real

El decoder admite como máximo un bloque de parent lookup. La comprobación sigue
a `DB2FileLoader.cpp:1767,1775`: `ParentLookupCount` debe ser `1` o `0`, y un fichero
que declare un bloque parent mientras el esquema revisado no tiene campo de
índice parent se rechaza como `Schema`. Un esquema con parent sí puede omitir
el bloque: conserva el valor dentro del registro o deja en cero el campo añadido.
El incremento de 44 tablas corrigió la exigencia anterior de un bloque obligatorio.

En este primer caso, el parent es un **campo extra al final**. El array de
campos que se publica tiene `FieldCount` posiciones del esquema total, pero solo
`FileFieldCount` columnas se leen del fichero:

- `fields` son las columnas reales de fichero (`FileFieldCount`);
- `parent` es el índice dentro del array total (`ParentIndexField`), que es
  exactamente una posición más allá de las columnas decodificadas y **se rellena
  desde el bloque parent**, no desde column meta.

El caso revisado inicialmente es `ChrClassesXPowerTypes`: `FieldCount 2`,
`FileFieldCount 1`, `ParentIndexField 1` y `LayoutHash 0x70DA1F8C`
(`DB2Metadata.h::ChrClassesXPowerTypesMeta`). El valor parent se escribe al
final, después de que existan todas las filas y antes de la copy-table, con
entradas `{ParentId, RecordIndex}` de 8 bytes LE; un duplicado posterior para el
mismo `RecordIndex` gana.

El manifiesto v2 toma `meta_index_field` del **esquema revisado**, no del header
crudo (`IndexField`). En ficheros reales los dos discrepan: el header declara `0`
mientras la meta revisada es `-1`, por lo que el word del header nunca se publica
como `meta_index_field`.

### Evidencia real persistida — `xpower-accept-1791109553`

Reporte redactado
`rustycore-extractor-work/reports/acceptance/xpower-parent-lookup-xpower-accept-1791109553.json`:
`extract_exit 0`, `verify_exit 0`, `network_attempted: false`, `paths_redacted:
true`; el cliente no fue modificado (se conserva su `client_metadata_sha256`).
`ChrClassesXPowerTypes` (FileDataID 1121420) decodifica **15 filas**
(`meta_index_field: -1`, `field_count: 1`, `table_schema_validated: true`,
`record_data_decoded: true`). La paridad Rust vs oráculo independiente es
`equal: true` sobre **16 líneas** (una cabecera más 15 filas), y `verify` reporta
`status: verified` con `artifacts_verified: 1`.

Límites: una sección **no cifrada**, sin maps/VMaps/MMaps y **sin claim de
70170**. Esta evidencia demuestra coherencia de salida y comparación de valores
del decoder, **NO paridad de formato completa** ni aceptación reader/runtime de
RustyCore. Reproducción con placeholders propios (`<client-root>`, `<run-id>`);
ninguna ruta de cliente real se escribe en un reporte.

### Evidencia real persistida — `anim-tables-anim3-accept-1791114040`

Reporte redactado
`rustycore-extractor-work/reports/acceptance/anim-tables-anim3-accept-1791114040.json`:
`extract_exit 0`, `network_attempted: false`, `paths_redacted: true`,
`client_metadata_sha256_after` registrado (el cliente queda intacto) y
`full_client_extraction: false`. El reporte histórico registró coincidencia
byte a byte entre Rust y Python (`oracle_equal: true`):

| Tabla | FileDataID | Filas | Paridad |
| --- | --- | --- | --- |
| `AnimKitBoneSet` | 1375433 | 18 | `oracle_equal: true` |
| `AnimationData` | 1375431 | 1806 | `oracle_equal: true` |
| `AnimKitConfig` | 1300872 | 40 | `oracle_equal: true` |

**Corrección de la interpretación — 2026-10-11.** Esa coincidencia no demuestra
la semántica de copias: ambos decoders comparaban erróneamente los IDs con
`header.record_count` y descartaban copias válidas. Se retira la aceptación de
las **40 filas de `AnimKitConfig`** y la afirmación de que solo seis de sus 170
entradas de copia fueran válidas. El reporte original se conserva como registro
histórico; los conteos de tablas con copias necesitan nueva aceptación.

**Regla de copias (`AutoProduceRecordCopies`).** En el commit C++ fijado
`02245dcd245e7433e524577656177723d3e4992e`, `AutoProduceData` calcula
`indexTableSize = GetMaxId() + 1` (`DB2FileLoader.cpp:394-398`). `GetMaxId()`
incluye los IDs directos y **todos los destinos de copia**, y comprueba que no
superen `header.MaxId` (`:955-977`). `DB2Store.cpp:101-107` pasa ese tamaño a
`AutoProduceRecordCopies`; no pasa `header.RecordCount`.

Las entradas `{NewRowId, SourceRowId}` se aplican en orden: se omiten fuentes
cero, fuera del índice o ausentes. Una copia puede servir como fuente a una
entrada posterior. El destino sobrescribe una fila existente y se reescribe
su campo de índice inline cuando lo tiene (`DB2FileLoader.cpp:613-634`).
El conteo `decoded_rows` representa las filas únicas tras aplicar las copias;
`declared_records` conserva el conteo de registros directos del header.

El decoder rechaza destinos que superen `header.MaxId` y el desbordamiento de
`max_id + 1` con `invalid_record_ids`. No asigna un array proporcional al ID
máximo. El límite de un millón de valores cuenta los elementos escalares de
arrays, el padre anexado y las filas nuevas creadas por copias; las entradas
omitidas y las sobrescrituras no añaden filas al presupuesto. No es un límite
de bytes de memoria.

Dos variantes con comportamiento inseguro en la referencia se rechazan como
`unsupported_record_variant`: una autocopia cuya fuente existe y una copia
aplicable con campo de índice inline de ancho distinto de 32 bits (por ejemplo,
el índice de un byte de `ChrClasses`). El C++ reemplaza el puntero destino antes
de leer el fuente y escribe siempre un `uint32` como ID, respectivamente. No
se intenta reproducir la lectura sin inicializar ni la alteración de campos
adyacentes. Las entradas con fuente cero o ausente siguen omitiéndose antes de
comprobar estas variantes.

Límites: es comparación de valores del decoder sobre tres tablas, **NO paridad de
formato completa** ni aceptación reader/runtime de RustyCore. Sin rutas de cliente
en el reporte; los artefactos crudos derivados del cliente quedan fuera de Git.

### Revalidación de copias — 2026-10-11

La corrección se validó localmente sobre build **70205**, locale **esES**,
con un lote nuevo de las mismas tres tablas. Reporte redactado:
`rustycore-extractor-work/reports/acceptance/copy-accept-20261010T232418Z-fa13ff78.json`
(hora del identificador en UTC). Registra `extract_exit: 0`, `verify_exit: 0`,
`artifacts_verified: 3`, `network_attempted: false` y
`client_metadata_unchanged: true`.

| Tabla | Registros directos | Filas tras copias | Rust/Python |
| --- | ---: | ---: | --- |
| `AnimKitBoneSet` | 18 | 18 | Iguales byte a byte |
| `AnimationData` | 1806 | 1806 | Iguales byte a byte |
| `AnimKitConfig` | 34 | **204** | Iguales byte a byte |

El nuevo resultado de `AnimKitConfig` incorpora **170 copias** y recupera las
**164 filas** que el cálculo anterior descartaba. Sustituye la interpretación
del reporte histórico de 40 filas. La igualdad de dumps se acompaña de la
revisión de la cadena de llamadas del C++ fijado y de goldens sintéticos con
IDs dispersos; no se ejecutó aquí el decoder C++ completo ni el runtime de
RustyCore.

El reporte identifica el HEAD base `fa85668397f6f8562025e6112aecf2a3ed9c5331`,
los hashes de fuentes modificadas sin commit, los binarios y el driver de
aceptación. El driver, plan, comandos exactos, trazas y dumps quedan en el run
privado bajo `~/.local/share/rustycore-extractor-work/runs/candidate/`, fuera de
Git. Cada proceso de extracción, dump y verificación se ejecutó con
`strace -e trace=network -e inject=network:error=EPERM`.

Comprobaciones locales de esta corrección mediante `.opencode/checks`:

| Modo | Resultado |
| --- | --- |
| `rust` | 150 tests de `forever-db2` y 43 de `forever-casc`, todos correctos |
| `synthetic` | 70 tests recogidos: 56 correctos, 14 omitidos por selección explícita de binarios/cliente |
| `native` | 45 tests de biblioteca + 3 del probe; Python: 65 correctos, 5 omitidos |
| `contracts` | 34 correctos |
| `provenance` | 355 rutas importadas y 10 avisos verificados |
| `fmt-check`, `clippy` | Correctos; Clippy con `-D warnings` |

También pasó `.opencode/checks live-extract` para el plan predeterminado:
`ChrClasses` conserva 9 filas y `ChrRaces` 58. Reporte
`rustycore-extractor-work/reports/acceptance/opencode-live-extract-20261010T232432Z-b55e4fc0.json`,
build 70205/esES, sin intentos de red y con metadatos del cliente sin cambios.

Las regresiones cubren copias encadenadas, sobrescrituras, fuentes ausentes/cero,
padres heredados, overflow, variantes no admitidas y presupuesto de valores.
El oráculo también interpreta correctamente padres signed32 con el bit alto
activado. La prueba de lote confirma que un manifiesto antiguo con conteo
insuficiente se rechaza como `ROW_DECODE_MISMATCH`, aunque el hash del DB2
siga siendo válido. Se conservan los runs anteriores; no se reescriben sus
manifiestos para ocultar esa discrepancia.

El primer intento de `rust` detectó que los temporales del helper estaban bajo
el Git contenedor `wow`. Se corrigió `TMPDIR` a
`~/.local/share/rustycore-extractor-work/tmp/opencode/` y se retiró la dependencia
del fixture de `/tmp/opencode`. Las pruebas completas pasaron con la protección
de rutas de salida activa.

## RustyCore reader conformance (bounded)

AGENTS.md names **TrinityCoreLegacy output plus RustyCore reader/runtime
acceptance** as the compatibility authority. This records the first bounded
execution of the RustyCore reader half of that gate, run offline and without
modifying RustyCore.

**Scope (not a broader acceptance).** The gate opens each produced `.db2` with
RustyCore's own reader and compares two independent things: (1) header facts and
id presence — `format_version`, `table_hash`, `layout_hash`, `field_count`,
`record_count`, `inline_id_field` and whether the reviewed ids are present; and
(2) per-id record-byte digests (FNV over each record body) against our decoder.
It covers **2 tables** only. It is **not** a 70170 acceptance, **not** full
extraction, and **not** complete format parity; it exercises no maps/VMaps/MMaps,
no encrypted section and no complete table catalog.

**How it is run.** The probe is a standalone lab crate that lives **outside** this
repository at
`rustycore-extractor-work/tools/rustycore-reader-check/` and depends on RustyCore
by path (`wow-data = { path = "../../../rustycore/crates/wow-data" }`), using
`wow_data::wdc4::Wdc4Reader`. RustyCore is opened **read-only and is never
modified**. Replace `<rustycore>` and `<run-root>` with your own local paths; no
client path is written into the report:

```bash
cd ../rustycore-extractor-work/tools/rustycore-reader-check
CARGO_TARGET_DIR=<run-root>/build cargo run --offline --locked -- \
  --file <run-root>/<run-id>/tables/ChrClasses.db2 --id <id> [--id <id> ...]
```

**Measured results (redacted; hashes/counts only, never raw bytes or values):**

| Table | `table_hash` | `layout_hash` | `field_count` | `inline_id_field` | rows | digest match |
| --- | --- | --- | --- | --- | --- | --- |
| ChrClasses | `0xF5889D8C` | `0xAFC9B0C2` | 43 | `Some(29)` | 9 | 9/9 |
| ChrRaces | `0x53F1783C` | `0x4F44C796` | 51 | `None` | 58 | 58/58 |

RustyCore **accepts both** files. Positional **semantic conformance** — the FNV
digest of each record's bytes by id — matches between RustyCore and our decoder
**9/9** for `ChrClasses` and **58/58** for `ChrRaces`.

**Persisted evidence.**
`rustycore-extractor-work/reports/acceptance/rustycore-reader-conformance-20261004T112024Z.json`
records this run (`rustycore_checkout_modified: false`, `network_attempted:
false`, `client_bytes_in_report: false`, `hashes_not_values: true`,
`full_client_extraction: false`). Only hashes and counts are compared and
persisted; **no raw record bytes or client values are ever written into the
report**. A green result here is bounded reader conformance on **2 tables** and
is not complete format parity or a full reader/runtime acceptance.

**Real typed initialization remains blocked — verified boundary.** The above covers
RustyCore's **generic reader** (`wow_data::wdc4::Wdc4Reader`). RustyCore's
**typed** creation path is `wow_data::forever_initialization::InitializationRecords::load`,
the only public API that loads all seven creation tables together, and it cannot
run against our current 70205 output. The concrete, reviewed reasons:

- `MapBaseline::AvailablePrefix` routes to `CreationDb2::open_initial_map_prefix`
  → `Wdc4Reader::open_available_initial_map`, whose `validate_initial_map`
  (`crates/wow-data/src/wdc4/available.rs`) demands an **exact 70170/esES pin**:
  `format_version` 5, `table_hash` `0xBD84CD62`, `layout_hash` `0xD43AFAC3`,
  `field_count`/`total_field_count` 26, `record_size` 48, `record_count` 79,
  `flags` 4, `id_index` 0, `parent_lookup_count` 0, `section_count` **4**,
  `bytes == 9574` exactly, and per-section `tact_key_hash`/`record_count`/`file_offset`/`string_table_size`
  (`71,2,5,1` / `2292,9574,9720,10103` / `3590,42,123,26`) with zero copy/relationship/offset-map counts.
- Our extractor does not produce that artifact: the `Map` 70205 acquisition is
  `local_file_incomplete` and its `.available` variant does not match this pin,
  so the typed load fails closed.
- Even isolating ChrClasses/ChrRaces is not publicly available:
  `CreationDb2` is `pub(crate)` (`crates/wow-data/src/wdc4/creation.rs`) and
  `identity::load` is `pub(super)` (`crates/wow-data/src/forever_initialization/identity.rs`),
  so there is no public entry point to validate only those two tables.

This is a **scope limit imposed by RustyCore's exact-baseline gate** for
`InitializationRecords`: the generic-reader conformance above is persisted
(**9/9** and **58/58** digests, redacted hashes/counts only). Until that exact
baseline is reproducible, **no typed initialization acceptance and no full
RustyCore runtime acceptance is claimed** for Forever. The independent names
loader described below does not require `Map` or `InitializationRecords`.
The later initialization increment catalogs the three remaining schemas and
validates the public seven-table loader with synthetic complete inputs. Its
real-input acceptance remains blocked by the incomplete local `Map` file.

## Name/category schemas and typed names loader — 2026-10-11

This increment expanded the reviewed decoder catalog to **36 schemas**. Its four additions serve
RustyCore's public `wow_data::forever_names::NameRecords::load` API, which is
called by the Forever world bootstrap to load character-name rule inputs:

| Table | FileDataID | LayoutHash | Fields in file order |
| --- | ---: | --- | --- |
| `NamesProfanity` | 1117086 | `0xF227E638` | string, i8 |
| `NamesReserved` | 1117085 | `0x2B2D5D97` | string |
| `NamesReservedLocale` | 1117087 | `0x7B9823D4` | string, u8 |
| `Cfg_Categories` | 1068162 | `0x8710BE94` | string, u16, u8, u8, i32, i8 |

All four use a separate ID table (`IndexField = -1`), no parent
(`ParentIndexField = -1`) and equal field/file-field counts. Facts were
independently transcribed from `DB2Metadata.h` at the pinned C++ commit
`02245dcd245e7433e524577656177723d3e4992e`, lines 15780, 15800, 15819 and 2747.
The first three strings are `FT_STRING_NOT_LOCALIZED`; the category string is
`FT_STRING`. The decoder returns the selected file's text, without merging
locales. The metadata supplies no table hash, so none was invented for these
schemas. RustyCore's own reader applies its independently pinned table hashes.

An explicit four-table plan can be authored without a client listfile:

```text
NamesProfanity\t1117086
NamesReserved\t1117085
NamesReservedLocale\t1117087
Cfg_Categories\t1068162
```

Use actual tab separators with `forever-db2-extract --manifest <plan.tsv>`.
The default selection remains `ChrClasses` and `ChrRaces`.

### Typed comparison boundary

The isolated local probe lives at
`rustycore-extractor-work/tools/rustycore-names-check/`. It depends on RustyCore
and `forever-db2` by path, builds offline, and runs as:

```bash
<external-build>/debug/rustycore-names-check --directory <run>/tables
```

It calls the public `NameRecords::load`, then compares its typed fields against
`DecodedTable` using exact canonical bytes sorted by ID. Strings carry byte
lengths, integers carry explicit types, and the table name provides a domain
prefix. The report exposes counts and FNV-1a digests; **exact equality, not
digest equality, is the acceptance gate**. No names, row values, client paths
or native error details enter the report. A 16 MiB per-file preflight runs
before either reader.

The typed projections are ID/text/language for `NamesProfanity`, ID/text for
`NamesReserved`, ID/text/locale-mask for `NamesReservedLocale`, and
**only ID/creation-charset (field 2)** for `Cfg_Categories`. The independent
Python oracle covers every field of all four tables. The probe's typed API
does not validate build or locale; those facts must come from CASC admission
and the batch manifest. It exercises no name-policy decision, SQL/hotfix
application, complete character creation, `Map` load or server startup.

### Executed 36-table and typed names acceptance

The definitive post-change run is
`names36-accept-20261010T232906Z-9eef32bb` (2026-10-11 in Europe/Madrid).
Its redacted report is retained at
`rustycore-extractor-work/reports/acceptance/names36-accept-20261010T232906Z-9eef32bb.json`.
The explicit plan requested exactly the 36 schemas implemented at that point,
from the locally admitted **build 70205 / esES** client. It completed with:

- `extract_exit: 0`, `verify_exit: 0`, `artifacts_verified: 36`;
- **36/36 full dumps equal** between Rust and the Python oracle, totalling
  **127,697 decoded rows**, including copied rows;
- `typed_names_exit: 0`, with all four typed projections equal;
- `network_attempted: false`, `client_metadata_unchanged: true`,
  `acceptance_inputs_unchanged: true`, `rustycore_worktree_unchanged: true`.

The typed names results are:

| Table | Direct records | Decoded / typed rows | Typed projection |
| --- | ---: | ---: | --- |
| `NamesProfanity` | 6595 | 6595 / 6595 | ID, text, language: equal |
| `NamesReserved` | 2559 | 2559 / 2559 | ID, text: equal |
| `NamesReservedLocale` | 2 | 2 / 2 | ID, text, locale mask: equal |
| `Cfg_Categories` | 91 | 100 / 100 | ID, creation charset: equal |

The 36-table result includes six empty tables: `ChrClassTitle`,
`ChrClassUIDisplay`, `ChrClassUIChrModelInfo`, `ChrClassVillain`,
`ContentRestrictionRuleSet` and `DataTagInfo`. Their live evidence exercises the
empty-table variant; populated decoding for them remains synthetic evidence.

The run also supersedes earlier incorrect copy-row counts throughout the
reviewed catalog. In particular the float-table counts are now
`ArmorLocation: 23`, `BoneWindModifiers: 54`, `AreaTriggerSphere: 8` and
`DurabilityQuality: 18`. Representative other results are
`ChrCustomizationGeoset: 4832`, `ChrCustomizationMaterial: 51686`,
`ChrCustomizationReq: 492`, `AnimReplacementSet: 71` and
`ItemExtendedCost: 1101`. The report retains the complete per-table inventory,
direct/decoded counts, artifact hashes and full-dump hashes. Prior runs are
preserved as historical evidence, not silently amended to these counts.

Source/binary fingerprints were captured before the run and checked again at
the end. The report identifies extractor base HEAD
`fa85668397f6f8562025e6112aecf2a3ed9c5331` plus the uncommitted changed-source
hashes, and RustyCore HEAD `544721324cf0a47553caae1ea0299a0ff2917ce7` plus reader
source/probe hashes. Every extraction, dump, verification and typed-reader
process ran with syscall-level network denial. Exact commands, driver, TSV
plan, traces and raw dumps remain in the private external run directory under
`~/.local/share/rustycore-extractor-work/runs/candidate/`. The report contains
no client values. RustyCore was not edited for this acceptance.

Local checks passed through `.opencode/checks`:

| Check | Observed result |
| --- | --- |
| `rust` | 158 DB2 tests + 43 CASC tests passed |
| `native` | 45 library tests + 3 probe tests passed; Python 81 passed, 5 gated tests skipped |
| `contracts` | 34 passed |
| `provenance` | 355 imported paths and 10 modification notices verified |
| `fmt-check`, `clippy` | Passed; Clippy uses `-D warnings` |

This increment adds eight Rust tests for the new schemas and sixteen Python
tests: eight for names/categories and eight closing the independent float-table
golden coverage gap. Their fixtures cover strings/UTF-8/escaping, signedness,
arrays, exact IEEE-754 bit patterns including NaN payloads, copies, empty tables
and malformed layouts or data. No upstream code or client fixture was imported.

This is acceptance of the explicit reviewed subset and the typed names loader.
It establishes no complete DB2 catalog, build-70170 live acceptance,
multi-section/encrypted decoding, full C++ decoder parity, map/VMap/MMap output
or full RustyCore runtime compatibility.

## Appearance schemas and typed appearance loader — 2026-10-11

This increment expanded the catalog from 36 to **41** schemas. Together with
the existing `ChrRaces` and `ChrCustomizationReq`, they supply all seven files
read by RustyCore's public `wow_data::forever_appearance::AppearanceRecords::load`.

| Table | FileDataID | LayoutHash | File / total fields | ID index | Parent index |
| --- | ---: | --- | --- | ---: | ---: |
| `ChrModel` | 3384313 | `0x03FAB755` | 17 / 17 | 2 | 4 |
| `ChrRaceXChrModel` | 3490304 | `0xA203BC29` | 4 / 4 | -1 | 0 |
| `ChrCustomizationOption` | 3384247 | `0xDCC2A86E` | 13 / 13 | 1 | 4 |
| `ChrCustomizationChoice` | 3450554 | `0x9559C358` | 11 / 11 | 1 | 2 |
| `ChrCustomizationReqChoice` | 3580359 | `0xF925BC6F` | 1 / 2 | -1 | 1 |

The facts and complete field types were independently transcribed into Rust
and Python from `DB2Metadata.h` at pinned C++ commit
`02245dcd245e7433e524577656177723d3e4992e`, lines 4049, 4160, 3909, 3715 and
3967 respectively. No upstream implementation or client fixture was imported.
That metadata supplies no table hash; RustyCore's separate typed reader checks
its own pinned table hashes. Localized strings use the selected file's text.

All five declare a parent field. When a parent lookup block is present,
the first four write their parent into an existing field;
`ChrCustomizationReqChoice` appends a signed 32-bit parent after its single
file field. The `ChrRaceXChrModel` parent occupies a signed byte, with incoming
parent bits restricted to its underlying unsigned byte width. The existing
decoder applies parent entries by direct-record ordinal, with later entries
winning, before applying copies. This increment added schema data and tests
without changing that behavior. The later 44-table increment corrected the
old requirement for a lookup block: without one, existing fields retain their
values and appended parents remain zero. The recorded 41-table run used the
populated lookup-bearing variant for these five tables.

The twelve new tests in each of Rust and Python cover independent synthetic
fixtures for inline/separate IDs, parent defaults and overwrites, appended
signed parents, copies and ID repair, byte/short/int limits, strings and UTF-8,
signed arrays, float vectors and exact IEEE-754 bits. Negative cases cover
invalid parent counts/ordinals/widths, strings and truncated data. More than
one parent block fails during metadata parsing, before the record variant
gate. Empty-table fixtures exercise the separate zero-section path.

### Typed appearance comparison boundary

The isolated offline probe is retained at
`rustycore-extractor-work/tools/rustycore-appearance-check/`. It depends by path
on RustyCore and `forever-db2` and runs as:

```bash
<external-build>/debug/rustycore-appearance-check --directory <run>/tables
```

It checks the 16 MiB limit and decodes all seven files before invoking the
public `AppearanceRecords::load` API. It compares exact canonical bytes sorted
by row ID, with explicit numeric types and table domains; duplicate IDs fail.
Only counts, equality flags and summary digests leave the private run.

| Table | Compared typed projection (plus row ID) |
| --- | --- |
| `ChrModel` | Display ID from field 4 |
| `ChrRaces` | Visual parent from field 38 |
| `ChrRaceXChrModel` | Race, model and sex from fields 0, 1 and 2 |
| `ChrCustomizationOption` | Model and requirement from fields 4 and 10 |
| `ChrCustomizationChoice` | Option and requirement from fields 2 and 3 |
| `ChrCustomizationReq` | Flags, class mask, two-word race mask, achievement, quest and item appearance |
| `ChrCustomizationReqChoice` | Choice and appended requirement from fields 0 and 1 |

Signed integers projected into unsigned reader fields preserve their bits.
The race byte is first interpreted as unsigned before widening; sex remains
signed. Full Rust/Python dump equality covers the fields omitted from these
typed projections, including strings and floats.

The loader does not establish build or locale; CASC admission and the manifest
provide that evidence. The probe does not call `AppearanceRecords::finish`,
apply SQL/hotfix overlays or validate character creation or world startup.

### Executed 41-table and typed appearance acceptance

The definitive run is `appearance41-accept-20261010T234000Z-0792c7be`,
2026-10-11 in Europe/Madrid. Its redacted report is retained at
`rustycore-extractor-work/reports/acceptance/appearance41-accept-20261010T234000Z-0792c7be.json`.
The explicit 41-table plan used the admitted local **build 70205 / esES** client:

- Extraction and output verification exited zero; **41 artifacts verified**.
- All **41 full Rust/Python dumps matched**, totalling **140,006 decoded rows**.
- The typed names loader again matched four tables and **9,256 rows**, with
  the category projection limited as documented above.
- The typed appearance loader matched all seven input tables and **12,859 rows**.
- `network_attempted: false`, `client_metadata_unchanged: true`,
  `acceptance_inputs_unchanged: true`, `rustycore_worktree_unchanged: true`,
  `rustycore_reader_inputs_unchanged: true`, `probe_sources_unchanged: true`.

| Appearance table | Direct records | Decoded / typed rows | Projection equal |
| --- | ---: | ---: | --- |
| `ChrModel` | 127 | 127 / 127 | Yes |
| `ChrRaces` | 58 | 58 / 58 | Yes |
| `ChrRaceXChrModel` | 116 | 116 / 116 | Yes |
| `ChrCustomizationOption` | 1173 | 1173 / 1173 | Yes |
| `ChrCustomizationChoice` | 10447 | 10447 / 10447 | Yes |
| `ChrCustomizationReq` | 48 | 492 / 492 | Yes |
| `ChrCustomizationReqChoice` | 446 | 446 / 446 | Yes |

The six empty tables identified in the 36-table run remain empty. All five
new appearance tables are populated in this client. The 36 previously reviewed
tables retain the same counts; this increment adds 12,309 decoded rows.

The report fingerprints the changed extractor sources, all executed binaries,
both probe projects, both RustyCore loader module trees and the WDC4 reader
tree before execution and checks them after both typed probes complete. It
records extractor base HEAD `fa85668397f6f8562025e6112aecf2a3ed9c5331` and
RustyCore HEAD `544721324cf0a47553caae1ea0299a0ff2917ce7`. RustyCore was not
edited. Every extraction, dump, verification and typed-loader process ran
under syscall-level network denial. Exact commands, plan, driver, traces and
raw artifacts remain in the private external run directory; only counts,
hashes and structured diagnostics enter this report.

The earlier successful run `appearance41-accept-20261010T233806Z-f6898433`
is preserved. It omitted `forever_names/load.rs` from its source fingerprints
and checked worktree status before the appearance probe. The definitive run
above repeats the acceptance with both provenance gaps corrected.

Local checks passed through `.opencode/checks`:

| Check | Observed result |
| --- | --- |
| `rust` | 170 DB2 tests + 43 CASC tests passed |
| `native` | 45 library tests + 3 probe tests passed; Python 93 passed, 5 gated tests skipped |
| `contracts` | 34 passed |
| `provenance` | 355 imported paths and 10 modification notices verified |
| `fmt-check`, `clippy` | Passed; Clippy uses `-D warnings` |

This establishes the reviewed 41-table subset and the stated typed loader
projections for build 70205/esES. Complete DB2 extraction, build-70170 live
acceptance, multi-section/encrypted decoding, full C++ decoder parity,
map/VMap/MMap output and full RustyCore runtime compatibility remain pending.

## Item specialization and gem schemas — 2026-10-11

Three additions expand the reviewed catalog from 41 to **44** schemas and
supply the files required by RustyCore's public
`wow_data::forever_birth::item_specs::ItemSpecRecords::load` API:

| Table | FileDataID | LayoutHash | File / total fields | Parent | Fields in file order |
| --- | ---: | --- | --- | ---: | --- |
| `ItemSpec` | 1135120 | `0x83F3D113` | 6 / 6 | 2 | u8, u8, u8, u8, u8, u16 |
| `ItemSpecOverride` | 1134576 | `0xB292998C` | 1 / 2 | 1 | u16; appended parent i32 |
| `GemProperties` | 1343604 | `0x86487AD2` | 2 / 2 | -1 | u16, i32 |

All three have separate IDs (`IndexField = -1`). The Rust and Python schema
transcriptions were checked independently against `DB2Metadata.h` at pinned
commit `02245dcd245e7433e524577656177723d3e4992e`, lines 12929, 12953 and 9802.
The metadata supplies no table hash. RustyCore separately pins reader hashes
`0x08DA6E2A`, `0x149AAE79` and `0x9C00EA6D`, respectively.

### Corrected parent and empty-table admission

This increment corrects two assumptions shared by the decoder and Python oracle:

1. **Parent lookup is optional even when the schema has a parent field.**
   `DB2FileLoader.cpp:1775` rejects a lookup block only if the schema has no
   parent; it does not require the reverse. With no block, the reference leaves
   its lookup vector empty (`1866–1874`), preserves existing fields and
   initializes appended fields to zero (`480–510`). Rust and Python now follow
   that rule, including subsequent copies. A declared lookup for a schema with
   no parent still fails; undeclared lookup bytes, invalid ordinals and values
   outside the parent width still fail. Scalar limits still charge appended
   parents when their value is the default zero.
2. **Zero sections still require complete primitive field metadata.**
   The reference reads four bytes per file field (`1782–1784`) and checks the
   complete length (`1903–1912`). The previous early return could accept a
   truncated header-only file. The supported minimal empty variant now needs
   exactly `204 + 4 * FileFieldCount` bytes and zero declared records, strings,
   column-storage metadata, common data and pallet data. Other empty payload
   variants are explicitly unsupported. Existing sparse/schema gates remain;
   this rule does not invent a global flags-4 or record-size-0 restriction.

For the three new tables, minimal complete empty sizes are **228, 208 and 212
bytes**. Synthetic helpers now emit the primitive metadata instead of treating
a 204-byte header as a complete table. This tightens record decoding and output
verification; the separate metadata-only inspector keeps its bounded header
inspection contract.

RustyCore deliberately has narrower physical table admission: populated
`ItemSpec` uses no lookup, `ItemSpecOverride` uses one and `GemProperties` has
no parent. Its dedicated complete-empty variant has additional constraints.
Decoder acceptance alone therefore does not imply acceptance by that reader.

### Typed item-specialization comparison boundary

The isolated probe lives at
`rustycore-extractor-work/tools/rustycore-item-spec-check/` and runs as:

```bash
<external-build>/debug/rustycore-item-spec-check --directory <run>/tables
```

It bounds each file to 16 MiB and decodes all three before calling
`ItemSpecRecords::load`. It compares every field exposed by this loader,
including row IDs, as exact typed canonical bytes sorted by ID:

- `ItemSpec`: minimum/maximum level, item type, primary/secondary statistics
  and specialization, in file order.
- `ItemSpecOverride`: specialization and appended item ID. The source signed
  parent is reinterpreted as an unsigned 32-bit bit pattern.
- `GemProperties`: enchantment and signed kind.

Only counts, equality flags and digest summaries are reported. This exercises
no `finish`, overlays, item-template construction, inventory or server startup.
Build and locale come from CASC admission and the manifest, not the typed API.

The additional synthetic probe run
`itemspec-probe-synthetic-20261010T234641Z-9d3a5d52` passed all four checks:
populated inputs (three rows per table, including copies and numeric boundary
values), complete empty inputs, rejection of truncated empty metadata, and
rejection by RustyCore of a populated `ItemSpec` with an unexpected lookup.
The latter is valid for the decoder but outside the typed reader's admitted
physical variant. Its source-only fixtures, driver and redacted report are
retained outside the checkout; no client fixture was used.

### Executed 44-table and typed item-specialization acceptance

The run `itemspec44-accept-20261010T234645Z-18dd0a49` (2026-10-11 in
Europe/Madrid) requested exactly the 44 reviewed schemas from the admitted
local **build 70205 / esES** client. The redacted report is retained at
`rustycore-extractor-work/reports/acceptance/itemspec44-accept-20261010T234645Z-18dd0a49.json`.

- Extraction and verification exited zero; **44 artifacts verified**.
- **44/44 full Rust/Python dumps matched**, totalling **140,015 decoded rows**.
- RustyCore's typed item-specialization loader accepted all three files and
  matched the decoded projections.
- Typed names and appearance checks passed again: four tables / **9,256 rows**
  and seven tables / **12,859 rows**, within their documented projections.
- All 41 previous artifact hashes, dump hashes and row counts remained equal
  to the definitive 41-table run.

| New table | Bytes | Direct records | Decoded / typed rows |
| --- | ---: | ---: | ---: |
| `ItemSpec` | 228 | 0 | 0 / 0 |
| `ItemSpecOverride` | 412 | 9 | 9 / 9 |
| `GemProperties` | 212 | 0 | 0 / 0 |

The two new empty tables bring the empty total to **eight**. Their live evidence
proves complete-empty admission, not populated decoding; populated cases are
covered by the independent golden tests and the synthetic typed probe above.

All subprocesses reading client artifacts ran with syscall-level network
denial. The report records `network_attempted: false` and true for
`client_metadata_unchanged`, `acceptance_inputs_unchanged`,
`rustycore_worktree_unchanged`, `rustycore_reader_inputs_unchanged` and
`probe_sources_unchanged`. It fingerprints Rust sources in both extractor
crates, Cargo manifests/lock, oracle/tests, all executed binaries, all three
probe projects, and the relevant RustyCore loader/reader source trees.
Extractor base HEAD remains `fa85668397f6f8562025e6112aecf2a3ed9c5331`;
RustyCore HEAD remains `544721324cf0a47553caae1ea0299a0ff2917ce7`, with no
integration edits. Exact commands, plan, driver, traces and raw outputs remain
in the private run under `~/.local/share/rustycore-extractor-work/runs/candidate/`.

Local checks passed through `.opencode/checks`:

| Check | Observed result |
| --- | --- |
| `rust` | 180 DB2 tests + 43 CASC tests passed |
| `native` | 45 library tests + 3 probe tests passed; Python 113 passed, 5 gated tests skipped |
| `contracts` | 34 passed |
| `provenance` | 355 imported paths and 10 modification notices verified |
| `fmt-check`, `clippy` | Passed; Clippy uses `-D warnings` |

This increment adds ten Rust tests and twenty Python tests, plus the four
local synthetic probe cases. It establishes the explicit 44-table subset and
the documented loader projections for 70205/esES. It does not establish a
complete DB2 catalog, build-70170 live compatibility, encrypted/multi-section
decoding, full C++ decoder parity, map/VMap/MMap output or server startup.

## Birth schemas and the incomplete ability table — 2026-10-11

Five schema additions expand the reviewed catalog from 44 to **49**. They
describe the inputs of RustyCore's public
`wow_data::forever_birth::BirthRecords::load` API. Four are available as complete
regular files in the local 70205/esES client; `SkillLineAbility` fails strict
local acquisition. A reviewed schema is not evidence that its client artifact
is available or that the complete typed loader has accepted real inputs.

| Table | FileDataID | LayoutHash | File / total fields | ID index | Parent index |
| --- | ---: | --- | --- | ---: | ---: |
| `SkillLine` | 1240935 | `0x6763217C` | 15 / 15 | 5 | -1 |
| `SkillRaceClassInfo` | 1240406 | `0x24277B48` | 7 / 7 | -1 | 0 |
| `SkillLineAbility` | 1266278 | `0x224F7EA0` | 18 / 18 | 2 | 3 |
| `CharacterLoadout` | 1344281 | `0x713CE8BB` | 5 / 5 | -1 | -1 |
| `CharacterLoadoutItem` | 1302846 | `0x0C7A1862` | 2 / 2 | -1 | 0 |

Rust and Python were independently checked against `DB2Metadata.h` at
`02245dcd245e7433e524577656177723d3e4992e`, lines 19625, 19715, 19658, 3206 and
3229. In file order (`string` uses the selected file's locale):

- `SkillLine`: five strings, u32, i8, i32, i8, u32, then five i32 fields.
  The first four strings are localized; the fifth is nonlocalized.
- `SkillRaceClassInfo`: i16, i32, i32, i32, i8, i16, i32[2].
- `SkillLineAbility`: two localized strings, u32, i16, i32, i16, i32, i32,
  i32, i16, i16, i32, i8, i16, i16, i16, i32[2], i32[2].
- `CharacterLoadout`: i8, i32, u8, i32, i32[2].
- `CharacterLoadoutItem`: u16, u32.

No new field type or decoder behavior is introduced. All parents are inside
the file fields. Two parent fields are signed 16-bit values; the loadout-item
parent is unsigned 16-bit. Their raw parent values must fit 16 bits before
signed interpretation. Race masks retain two signed 32-bit source words in
their original order. No table hash was inferred from `DB2Meta`.

### Strict acquisition and typed-loader boundary

The local metadata preflight
`birth-preflight-20261010T235031Z-691e6e8a` used an explicit five-table plan,
with network syscalls denied. It admitted build 70205, preserved client metadata
and obtained four complete, single-section files without TACT or copy rows:
`SkillLine` (154 direct records), `SkillRaceClassInfo` (186),
`CharacterLoadout` (114) and `CharacterLoadoutItem` (846). At that point these
were unreviewed acquisitions with no schema or record-decoding claim.

`SkillLineAbility` returned `local_file_incomplete`, leaving the batch
incomplete with exit 8. No complete artifact was created for it. This evidence
does not establish its full 70205 header, section layout or decoded records.
The older RustyCore diagnostic of a six-section 70170 available prefix does
not change that result and is not promoted to a complete 70205 input.

RustyCore's normal `BirthRecords::load(..., AbilityBaseline::Complete)` needs
all five ordinary `.db2` files. Its populated table reader additionally pins
hashes `B53DC9D6`, `06ADE420`, `FF4446F6`, `E00A47FB`, `65C39BA7`, and lookup
counts 0, 1, 1, 0, 1 respectively. No available-prefix fallback, key download,
zero-filled section or synthetic replacement is used for local acceptance.

### Synthetic typed birth comparison

The isolated probe lives at `rustycore-extractor-work/tools/rustycore-birth-check/`:

```bash
<external-build>/debug/rustycore-birth-check --directory <run>/tables
```

It enforces the 16 MiB file bound and decodes all five files before invoking
the public typed API with `AbilityBaseline::Complete`. A missing file fails
this preflight before the typed loader executes. Successful calls must return
`unknown_ability_records == 0`. There is no `finish`, overlay application,
learned/equipped character or server-startup check.

It compares every numeric field exposed by `BirthRecords`, together with IDs,
using exact canonical bytes sorted by ID. Signed shorts used as skill IDs
preserve their bits as u16. Race masks combine low/high u32 bit patterns into
u64, and the separate two-element ability field remains two signed i32 values.
Strings excluded by the typed API are covered by the full Rust/Python dump
tests. Reports expose only counts, equality flags, hashes and static errors.

The source-generated run `birth-probe-synthetic-20261010T235419Z-9aa05352`
passed six checks:

- Complete populated fixtures: three rows per table, including copies and
  signed/unsigned numeric boundaries; all five typed projections equal.
- Complete empty regular-section fixtures: all five typed projections empty.
- Missing ability file: `input_io` before invoking the typed loader.
- Declared encrypted section: `unsupported_record_variant`.
- Truncated ability data: `record_data_bounds`.
- A decoder-valid ability file with no parent block: rejected by the narrower
  typed reader as `typed_birth_load_failed`.

The preceding synthetic run `birth-probe-synthetic-20261010T235340Z-bfc43cde`
is retained as failed evidence: its inline-ID fixtures omitted the explicit
32-bit storage width required by RustyCore. Correcting the fixture metadata
made the positive cases pass without changing either reader.

### Executed 48-table acceptance and Birth blocker

The run `birth49-accept-20261010T235434Z-608feeeb` (2026-10-11 in
Europe/Madrid) requested the 48 complete tables available from the admitted
70205/esES client. The redacted report is
`rustycore-extractor-work/reports/acceptance/birth49-accept-20261010T235434Z-608feeeb.json`.
Its status is `available_subset_passed_birth_blocked`, explicitly separate
from complete Birth or full-extractor acceptance.

- Extraction and verification exited zero; **48 artifacts verified**.
- **48/48 full Rust/Python dumps matched**, totalling **141,315 decoded rows**.
- The prior 44 tables retain identical artifact/dump hashes and row counts
  relative to `itemspec44-accept-20261010T234645Z-18dd0a49`.
- Eight tables remain empty. The four new complete Birth tables are populated:

| Table | Declared / decoded rows | Full Rust/Python dump equal |
| --- | ---: | --- |
| `SkillLine` | 154 / 154 | Yes |
| `SkillRaceClassInfo` | 186 / 186 | Yes |
| `CharacterLoadout` | 114 / 114 | Yes |
| `CharacterLoadoutItem` | 846 / 846 | Yes |

The same run repeated all three previously accepted public typed loaders:
names (four tables, 9,256 rows), appearance (seven tables, 12,859 rows), and
item specialization (three tables, nine rows plus the two empty inputs).
Every compared projection matched. The existing limits on category fields,
`load` versus `finish`, overlays and runtime acceptance still apply.

A separate strict single-table acquisition of `SkillLineAbility` returned
exit 8 / `local_file_incomplete`. Its verifier returned exit 8 /
`INCOMPLETE_RUN`. Neither output directory contains a `SkillLineAbility.db2`
artifact. The Birth probe against the 48-table output returned exit 2 /
`input_io` during its preflight, **before the typed Birth loader executed**.
These expected failures were required by the acceptance driver; they are not
reported as successful real Birth decoding.

All commands ran with network syscalls denied, with no network attempt.
Client metadata, extractor sources/binaries, all four probe sources, and the
relevant RustyCore reader sources stayed unchanged during the run. RustyCore
HEAD remains `544721324cf0a47553caae1ea0299a0ff2917ce7`; its worktree was also
unchanged. The report fingerprints the uncommitted extractor sources, external
probes, binaries and driver. Raw files, full dumps, command logs and manifests
remain under the private run root outside Git; the report contains no client
record values. Previous runs are retained.

This increment adds eleven Rust and fourteen Python tests for the Birth
schemas, including strings, signed/unsigned 16-bit parents and overflow,
two-word masks, parent blocks, copies, schema mismatches and empty inputs.
The completed checks are: **191 DB2 + 43 CASC** Rust tests; native checks
**45 library + 3 probe** tests; Python **127 passed, 5 gated skips**;
**34 contract tests**; provenance **355 paths / 10 notices**; formatting,
`git diff --check`, and Clippy with warnings denied. The six synthetic typed
Birth checks above also passed.

This establishes the 48-table subset for 70205/esES and synthetic coverage
for all five Birth schemas. A complete local `SkillLineAbility` input is still
needed for real Birth acceptance. It does not establish build-70170 live
compatibility, encrypted/multi-section decoding, full C++ decoder parity,
complete initialization, map/VMap/MMap output or server startup.

## Initialization schemas and the incomplete Map table — 2026-10-11

Three schema additions expand the reviewed catalog from 49 to **52**. They
complete the schema inventory required by RustyCore's seven-table
`InitializationRecords::load` API. The two new complete local artifacts are
`PowerType` and `Movie`; `Map` remains unavailable as a complete strict read.
The decoder's supported record variants and acquisition policy are unchanged.

| Table | FileDataID | LayoutHash | File / total fields | ID index | Parent index |
| --- | ---: | --- | --- | ---: | ---: |
| `Map` | 1349477 | `0xD43AFAC3` | 26 / 26 | -1 | -1 |
| `PowerType` | 1266022 | `0x14BBEEA1` | 13 / 13 | 2 | -1 |
| `Movie` | 1332556 | `0xF53888FA` | 6 / 6 | -1 | -1 |

Rust and Python were independently checked against `DB2Metadata.h` at
`02245dcd245e7433e524577656177723d3e4992e`, lines 14538, 17452 and 15329.
The field types in file order are:

- `Map`: six strings, f32[2], u8, i8, u8, u16, four i16 fields, u8, f32,
  i16, u8, i16, five i32 fields, i32[3]. Its first string is nonlocalized;
  the remaining five are localized.
- `PowerType`: two nonlocalized strings, u32, i8, six i32 fields, two f32
  fields, i32. Field 2 is the inline ID.
- `Movie`: one localized string, two u8 fields, three u32 fields.

These use existing primitives, with no parent lookup or appended field.
Table hashes remain unspecified in the extractor schemas because the pinned
`DB2Meta` does not supply them. RustyCore's independent reader pins table
hashes `BD84CD62`, `8D899A57` and `032DFA13` respectively. Empty record decoding
requires complete primitive metadata: minimum sizes are 308, 256 and 228 bytes.

### Strict local acquisition diagnostics

The esES preflight `next49-preflight-20261010T235939Z-33a0d715` used the
previous 49-schema binary. It obtained `PowerType` (six direct records,
890 bytes) and `Movie` (three direct records, 470 bytes), both single-section
regular files without TACT or copies. They were metadata-only acquisitions
at this stage. `Map` returned `local_file_incomplete`, with no complete
artifact or decoded-record claim.

A separate explicitly selected enUS diagnostic,
`blocked-enUS-preflight-20261011T000050Z-7e975c90`, attempted strict reads of
`Map` and `SkillLineAbility`. Both returned `native_read_failed`; neither
produced an artifact. This does not identify a more specific failure cause
or establish enUS compatibility. It supplies no replacement for esES inputs.
Both preflights denied network syscalls, observed no network attempt and
preserved client metadata. Their redacted reports are under the external
laboratory's `reports/acceptance/`; raw diagnostics remain private.

### Synthetic seven-table typed initialization

The external probe is
`rustycore-extractor-work/tools/rustycore-initialization-check/`:

```bash
<external-build>/debug/rustycore-initialization-check --directory <run>/tables
```

It first bounds and decodes all seven files (`Map`, `PowerType`, `Movie`,
`ChrSpecialization`, `ChrClassesXPowerTypes`, `ChrClasses`, `ChrRaces`), then
calls `InitializationRecords::load(..., MapBaseline::Complete)` and requires
`unknown_map_records == 0`. It compares every numeric field exposed by those
seven public record types as exact canonical bytes sorted by ID. Float
comparisons preserve raw bits, including NaN, infinities and negative zero.
Strings absent from the typed API remain covered by full Rust/Python dumps.
No `finish`, effective catalog, overlay, player creation or startup is exercised.

The final synthetic run
`initialization-probe-synthetic-20261011T000207Z-1537ca9f` passed eight cases:

- Populated inputs: all seven projections equal (20 rows total), including
  copy rows and signed/unsigned boundaries. `ChrClasses` has two direct rows
  without copies because narrow inline-ID copies remain unsupported.
- Empty regular-section inputs: all seven projections empty.
- Missing Map: `input_io`, before the typed loader executes.
- Encrypted Map section: `unsupported_record_variant`.
- Truncated Movie: `record_data_bounds`.
- Wrong PowerType table hash: `typed_initialization_load_failed`.
- A decoder-valid specialization with a parent block: rejected by the typed
  reader, which pins physical lookup count zero for this table.
- A decoder-valid class-power parent above 255: rejected by the typed reader's
  byte-width contract, despite its public class field being u32.

The last three cases fail at the narrower typed-reader boundary. Their static
error is `typed_initialization_load_failed`; they do not relax the reader.
The report fingerprints the generator, both Python fixture/oracle sources,
probe sources and binary. The earlier successful synthetic run
`initialization-probe-synthetic-20261011T000137Z-55ef717d` is retained; the final
run adds explicit NaN coverage in the typed PowerType projection and fixture
source fingerprints.

### Executed 50-table acceptance and two blocked loaders

The run `initialization52-accept-20261011T000250Z-de5b0e39` requested exactly
the 50 complete tables available from the admitted build-70205/esES client.
Its redacted report is
`rustycore-extractor-work/reports/acceptance/initialization52-accept-20261011T000250Z-de5b0e39.json`.
The status is `available_subset_passed_birth_and_initialization_blocked`.

- Extraction and output verification exited zero; **50 artifacts verified**.
- **50/50 full Rust/Python dumps matched**, totalling **141,324 decoded rows**.
- `PowerType` has six declared/decoded rows and `Movie` has three; both have
  validated schemas and complete record decoding.
- The previous 48 tables have identical artifact/dump hashes and row counts
  relative to `birth49-accept-20261010T235434Z-608feeeb`. Eight remain empty.
- Typed names, appearance and item specialization checks passed again, with
  respectively 9,256, 12,859 and nine rows across four, seven and three tables.

Separate strict single-table attempts for `Map` and `SkillLineAbility` each
returned exit 8 / `local_file_incomplete`. Their independent output verifiers
returned exit 8 / `INCOMPLETE_RUN`, and neither produced a complete artifact.
The initialization and Birth probes against the 50-table output each returned
exit 2 / `input_io` before executing their public typed loader. These expected
failures are required gates for this bounded report; neither loader receives
real complete-input acceptance.

Network syscalls were denied with no observed network attempt. Client metadata,
all recorded extractor sources/binaries, all five probe sources, relevant
RustyCore reader sources and its worktree remained unchanged during the run.
The source fingerprints now additionally include the native bridge/CMake/hash
adapter files and both disabled-network compatibility sources. RustyCore HEAD
remains `544721324cf0a47553caae1ea0299a0ff2917ce7`. The decoder core and earlier
batch tests also match the snapshot recorded before this increment.

Raw outputs, full dumps and exact commands are retained in the private run
root outside Git. Reports contain only redacted structured diagnostics, counts
and hashes. All prior successful runs remain available.

This increment adds **eight Rust and twelve Python tests**. Completed checks:
**199 DB2 + 43 CASC** Rust tests; native **45 library + 3 probe** tests;
Python **139 passed, 5 gated skips**; **34 contract tests**; provenance
**355 paths / 10 notices**; formatting, `git diff --check`, and workspace
Clippy with warnings denied. The external initialization probe also passed
Clippy, and all eight synthetic typed initialization cases passed.

The 52 reviewed schemas and 50 accepted real tables are separate counts.
Complete local inputs for both blocked tables are still required, and must
also satisfy the decoder and reader variant gates. No encrypted/multi-section
decoding, complete DB2 catalog, full C++ parity, build-70170 live acceptance,
map/VMap/MMap extraction or server startup is established by this increment.

## Scalar spell dependencies and generic-reader comparison — 2026-10-11

Four additions expand the reviewed schema catalog from 52 to **56**. They
provide spell casting time, duration, radius and proc-rate data. This increment
uses the existing scalar decoder and introduces no new record variant.

| Table | FileDataID | LayoutHash | File / total fields | Field types |
| --- | ---: | --- | --- | --- |
| `SpellCastTimes` | 1134089 | `0x75B6BD3A` | 2 / 2 | i32, i32 |
| `SpellDuration` | 1137828 | `0xA931BD2B` | 3 / 3 | i32, i32, i32 |
| `SpellRadius` | 1134584 | `0xF9A913EE` | 4 / 4 | f32, f32, f32, f32 |
| `SpellProcsPerMinute` | 1133526 | `0x2227014B` | 2 / 2 | f32, i32 |

All four use separate IDs (`IndexField = -1`), no parent, and no strings or
arrays. Radius has four separate float fields. These facts were independently
checked in Rust and Python against `DB2Metadata.h` at
`02245dcd245e7433e524577656177723d3e4992e`, lines 20706, 21120, 21807 and 21764.
Extractor table hashes remain unspecified because `DB2Meta` supplies none.
Minimal complete empty files require 212, 216, 220 and 212 bytes respectively.

### Reader scope and independent comparison

RustyCore's public `SpellRecords::load` loads 49 tables together. The partial
dependency loader and its checked `CreationDb2` API are private. The older
public spell Stores have different projections for several of these tables;
they are not substitutes for the Forever loader.

The new external probe is
`rustycore-extractor-work/tools/rustycore-spell-dependency-reader-check/`:

```bash
<external-build>/debug/rustycore-spell-dependency-reader-check --directory <run>/tables
```

It reads each file with a 4 MiB limit, fully decodes it using the extractor,
then opens the same immutable path through the public `Wdc4Reader`. It applies
explicit header gates from RustyCore's pinned `SpellSchema`: table hashes
`FDBA5A66`, `B423FA7A`, `AB7E4841`, `D0768C6C`, the layouts/counts above,
flags 4, physical header index 0, locale mask `FFFFFFFF`, and no parent lookup.
For an empty table it also requires the source's zero-section, zero-size and
zero-payload header shape. These are probe gates; the generic reader does not
itself perform full typed spell admission.

Every field of every row is compared as exact canonical bytes sorted by ID.
The probe calls the public `iter_records`, `get_field_i32` and `get_field_f32`
APIs, preserving float bits via `to_bits()`. It rejects duplicate IDs rather
than silently overwriting them. It does not repair copied rows, normalize the
input for the reader, or replace reader values with decoder values. A bounded
second read detects an input change across the path-based API calls. Reports
contain counts, static codes and digest summaries; equality uses full bytes.

This is **generic-reader comparison**, with
`typed_spell_loader_executed: false`. It does not execute `SpellRecords::load`,
effective composition, spell evaluation or server startup.

### Synthetic comparison and known generic-reader differences

The run `spell-reader-synthetic-20261011T001239Z-71fa3a83` passed 15 cases:

- Exact agreement for populated scalar inputs, common/default/override fields,
  pallet fields and minimal complete empty files, across all four schemas.
  Populated cases include a direct-source copy, signed boundaries and float
  NaN, infinities and negative zero.
- Expected rejection for a missing table, wrong hash/locale/flags, an empty
  regular-section shape excluded by the spell header contract, a declared
  encrypted section and truncation.
- Expected disagreement for a copy chain: the generic iterator returns three
  rows while the decoder materializes four.
- Expected duplicate-ID rejection for a copy overwriting an existing ID.
- Expected disagreement for copying from source ID zero: the generic iterator
  returns three rows while the decoder's reference-compatible skip leaves two.
- Expected value disagreement when a synthetic packed column's legacy storage
  offset differs from the packed-data offset used by the checked decoder.

These differences are retained as limits of the generic reader. Positive
results on the real files do not establish compatibility with every copy or
compression variant. The synthetic report fingerprints its generator,
fixture/oracle sources, probe sources and binary; network syscalls were denied.
The earlier successful run `spell-reader-synthetic-20261011T001009Z-15dd14b0`
is retained. The final run uses the binaries rebuilt after correcting the
crate's outdated introductory documentation; decoder behavior was unchanged.

### Local metadata and preliminary reader evidence

The metadata-only preflight `next52-preflight-20261011T000703Z-b8af7867` used
the previous 52-schema binary and obtained four complete 70205/esES files:

| Table | Direct records | Copy entries | Bytes |
| --- | ---: | ---: | ---: |
| `SpellCastTimes` | 68 | 4 | 1014 |
| `SpellDuration` | 123 | 11 | 2140 |
| `SpellRadius` | 49 | 4 | 1051 |
| `SpellProcsPerMinute` | 11 | 0 | 401 |

All were single regular sections without TACT. The preliminary generic-reader
run `spell-reader-preflight-20261011T000915Z-96381c08` subsequently matched all
270 decoded rows across these four artifacts. It predates the probe's extra
empty-header gate; final acceptance below records the completed implementation.
Both runs were offline with no observed network attempt. Redacted reports
remain under the external laboratory's `reports/acceptance/`; client assets
and raw diagnostics remain in the private run root.

### Executed 54-table and generic spell-reader acceptance

The run `spell56-accept-20261011T001242Z-3960661a` requested exactly the 54
complete tables available from the admitted build-70205/esES client. The
redacted report is
`rustycore-extractor-work/reports/acceptance/spell56-accept-20261011T001242Z-3960661a.json`.
Its status remains `available_subset_passed_birth_and_initialization_blocked`.

- Extraction and output verification exited zero; **54 artifacts verified**.
- **54/54 full Rust/Python dumps matched**, totalling **141,594 decoded rows**.
- The previous 50 tables retain identical artifact/dump hashes and row counts
  relative to `initialization52-accept-20261011T000250Z-de5b0e39`.
- Eight tables remain empty. All four new spell tables are populated and have
  validated schemas and complete record decoding.

| Table | Declared rows | Copy entries | Decoded / generic-reader rows | All fields equal |
| --- | ---: | ---: | ---: | --- |
| `SpellCastTimes` | 68 | 4 | 72 / 72 | Yes |
| `SpellDuration` | 123 | 11 | 134 / 134 | Yes |
| `SpellRadius` | 49 | 4 | 53 / 53 | Yes |
| `SpellProcsPerMinute` | 11 | 0 | 11 / 11 | Yes |

The driver requires the four exact table identities, successful generic-reader
comparison, every field equal, and reader counts equal to both decoder counts
and the independent full-dump counts. The 270-row result includes the 19 copy
entries present in these files. `typed_spell_loader_executed` remains false.

The existing typed names, appearance and item specialization checks passed
again. Separate strict `Map` and `SkillLineAbility` attempts each returned
exit 8 / `local_file_incomplete`; their output verifiers returned exit 8 /
`INCOMPLETE_RUN`, with no complete artifact. The initialization and Birth
probes returned `input_io` before executing either public typed loader.

The run denied network syscalls and observed no network attempt. Client
metadata, recorded extractor sources/binaries, all six probe sources, relevant
RustyCore sources and its worktree stayed unchanged. The report additionally
fingerprints the `forever_spells` sources used to establish the reader contract.
RustyCore HEAD remains `544721324cf0a47553caae1ea0299a0ff2917ce7`.
The decoder core and previous batch tests match the pre-increment snapshot.
The only additional crate change is its introductory documentation, corrected
to describe reviewed schemas and parent support without a stale table count.

This increment adds **eight Rust and seventeen Python tests**. Completed
checks: **207 DB2 + 43 CASC** Rust tests; native **45 library + 3 probe** tests;
Python **156 passed, 5 gated skips**; **34 contract tests**; provenance
**355 paths / 10 notices**; formatting, `git diff --check`, workspace Clippy
and the new probe's Clippy with warnings denied. All 15 synthetic comparison
cases passed, including the expected generic-reader disagreements.

Raw files, full dumps and commands remain in the private run root outside Git;
redacted reports contain no client record values. Prior runs remain available.
This establishes the explicit 54-table subset and the four observed generic
reader comparisons. Complete typed spell loading, the two blocked loaders,
build-70170 live compatibility, full C++ parity, maps/VMaps/MMaps and server
startup remain outside this acceptance.
