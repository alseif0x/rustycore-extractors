# Forever product interface v1 / v2

`forever-extractor` connects the existing Rust-local acquisition, DB2 catalog,
20 game tables, camera discovery, terrain conversion, collision assembly and
sequential navigation producers. The [v1 contract](../contracts/forever-product-contract-v1.json)
uses an explicit canonical TSV plan and the existing [common package](forever-package.md)
final manifest v1 / internal recovery state v2. Linux x86_64 has local evidence;
Windows distribution and full real extraction remain open.
The separate [v2 contract](../contracts/forever-product-contract-v2.json) admits
explicitly supplied Salsa20 keys under the bounded profile described below.

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
Plans are at most 128 KiB / 1,126 DB2 entries. Only catalog-backed complete record
decoding is admitted; this product CLI does not issue metadata-only DB2 success.
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
its defaults, 55/55 and false booleans. Off-mesh connections remain unsupported
in product v1 and v2 and are always explicitly empty.

Selecting maps/collision enables the existing bounded local ROOT-name capability.
This is a local logical-name index, without a listfile, network or OS-path fallback.
Collision runs the complete existing startup/shared-world assembler, which retains
its bounded in-memory staging policy. Navigation runs fresh sequential tile contexts
against the sealed maps/vmaps folders. Complete empty Map direct rows use the
validated empty pure navigation plan for `all`; explicit Map/Tile requests still
reject. Other empty geometry can legitimately produce globals without tile bodies,
or no navigation outputs, as admitted by the component planner.

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
