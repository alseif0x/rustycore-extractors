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
