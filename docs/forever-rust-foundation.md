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

No CASC opening, decompression, key download, record decoding, maps, VMaps or
MMaps are implemented in this Rust delivery. Existing C++ tools and their
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
