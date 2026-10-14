# Forever complete camera files

This component preserves the complete `CinematicCamera.db2` and each camera
file it references. Its separate v1 contract is
[`forever-camera-contract-v1.json`](../contracts/forever-camera-contract-v1.json),
with a strict [manifest schema](../schemas/forever-camera-manifest-v1.schema.json).
The admitted product is `wow_classic_beta`, builds 70170 and 70205, using the
existing `enUS` or `esES` storage root selection.

Linux build 70338 has a separate explicit
[manifest-v3 profile](#explicit-build-70338-raw-cameras-manifest-v3), with fresh
local acceptance below. The native default retains its legacy contract.

## Commands

```text
forever-camera-extract --client <client-root> --locale <enUS|esES> --output <new-output-root>
forever-camera-verify --output <existing-output-root>
```

Extraction uses the explicitly enabled native offline backend. Verification
needs neither a client nor the native feature. There are no arbitrary FDID,
source-file, network, key-download, prefix, resume or overwrite options.
Every discovered ID is required for a complete result. A missing, encrypted,
incomplete or oversized source/camera produces a partial result and retains
the incomplete marker. A valid empty source produces a complete source-only
result; a missing source cannot substitute for it.

## Source and discovery contract

The reference commit is
`02245dcd245e7433e524577656177723d3e4992e`. Paths below are relative to that
C++ repository, read through pinned Git objects:

| Fact | Source |
| --- | --- |
| Direct-record scan and `std::set<uint32>` | `src/tools/map_extractor/System.cpp:82,394-414` |
| CinematicCamera layout and types | `src/server/game/DataStores/DB2Metadata.h:4410-4430` |
| Field names | `src/server/game/DataStores/DB2LoadInfo.h:1297-1311` |
| Raw extraction and output filename | `src/tools/map_extractor/System.cpp:1147-1187,1303-1334` |
| Storage locale selection | `src/tools/map_extractor/System.cpp:1555-1560` |

`CinematicCamera.db2` has FileDataID **1294214**, layout **0x8BFD752B**,
external IDs, no parent, and these five physical fields:

| Physical index | Type | Field |
| --- | --- | --- |
| 0 | f32[3] | Origin |
| 1 | u32 | SoundID |
| 2 | f32 | OriginFacing |
| 3 | u32 | FileDataID |
| 4 | u32 | ConversationID |

`CinematicSequences` is a different table and does not supply these file IDs.
Discovery uses `DecodedTable::read_with_direct_u32_values(..., 3)`: it collects
the physical field from direct records before parent or copy application, then
returns the collection only after the complete DB2 passes decoding, copy,
extent and budget validation. It retains zero, deduplicates, and orders IDs as
unsigned u32 values. This matters when a copy overwrites an existing direct
row: its original dependency must still be extracted. No projection escapes
after a rejected tail, encryption declaration or invalid copy.

The current decoder's supported single-section and minimal-empty layouts
remain the discovery boundary. Unsupported shapes fail explicitly. This does
not emulate the C++ loader's ability to skip encrypted sections. The metadata
source SHA-256 is pinned in the contract and manifest.

C++ opens each camera with `CASC_LOCALE_NONE`; its storage was opened for a
selected installed locale. The existing native bridge already calls
`CascOpenFile(..., 0, ...)` and explains that the file locale parameter is
ignored (`native/forever-casc/bridge.cpp:44-53`). The Rust API's `Locale`
continues to select the storage root. This component introduces no extra
locale or FFI behavior.

## Bytes, limits and publication

The preserved source is `dbc/<locale>/CinematicCamera.db2`. Camera paths are
exactly `cameras/FILE<8 uppercase hexadecimal digits>.xxx`, matching the C++
filename contract. Complete bytes are retained without reserialization or
extension-based transformations. Zero remains a discovered ID; if the native
reader rejects it, that entry is an error and the run is partial.

| Limit | Bound |
| --- | ---: |
| Source DB2 | 16 MiB |
| Each camera | 16 MiB |
| Distinct camera IDs | 4,096 |
| Total acquired bytes, including source | 256 MiB |
| Direct source records | 100,000 |
| Decoded scalar values | 1,000,000 |
| Manifest | 2 MiB, depth 8 |

Every read repeats build admission. The reported build must also match, and a
final bounded `.build.info` snapshot comparison detects metadata changes.
Neither a complete prefix nor a partial native read can satisfy a file.

The destination must be new, outside the client and its resolved `Data` tree,
and outside Git trees. Traversal and existing destinations are rejected.
Directories and files are private on Unix (0700 and 0600). Each file is staged,
synced and published through an exclusive hard link, then its stage is removed;
existing names are never overwritten. The manifest is published last. Only a
complete batch removes its incomplete marker. This is atomic publication per
file with a final marker, not an atomic directory promotion. Existing runs are
never cleaned or merged.

## Manifest and independent verification

The private manifest records source identity, source layout, direct record
count, discovered ID count and set hash, plus complete sizes and SHA-256 hashes
for the source and each camera. Entries include explicit null facts for
failures. The set hash covers sorted little-endian u32 IDs without separators;
the inventory hash covers path-sorted UTF-8 lines of `sha256`, two spaces,
relative path and LF. Stdout reports expose only counts, fixed policy facts
and error codes; discovered IDs and bytes remain in the private artifact tree.

The verifier bounds and checks the manifest, rejects duplicate JSON keys and
unknown properties, enforces identities, filenames, ordering, limits and all
claims, then reads and decodes the preserved source independently. Its actual
discovery set and facts must match the manifest exactly. Every expected file
must exist with the recorded complete length and SHA-256. The entire output
tree must match; extra files, missing files, wrong case, symlinks and pending
stages fail. A partial manifest stays incomplete even if its marker is removed.
Verification never repairs or changes outputs.

| Exit | Meaning |
| --- | --- |
| 0 | Complete extraction or verified complete artifact |
| 2 | Invalid invocation or manifest |
| 3 | Unsupported initial client |
| 4 | Input, metadata guard or publication failure outside a file result |
| 8 | Partial acquisition or incomplete output |
| 9 | Unsafe path, inventory or discovery mismatch |

Per-file read, policy or publication errors are recorded as partial/8. A
global metadata or output failure returns 4 and retains any incomplete output;
there is no successful manifest for that failure.

## Acceptance boundary

Synthetic tests cover physical versus copied-row discovery, zero, duplicate
IDs, strict layout and whole-file validation, empty versus missing source,
exact raw preservation, bounds, changed builds and metadata, publication
guards, manifest mutations and offline rediscovery after a coherently forged
manifest. Native workspace checks and the independent overflow review passed.

The local report `camera-accept-20261011T005622Z-9564f358.json` records
build-70205/esES acceptance of ten cameras and the source DB2, eleven files
and 26,962 bytes in total. Extraction, manifest schema and output verification
pass. An independent Python projection of the ten direct source records
(no copies in this input) reproduces the discovered set, and all filenames
match the C++ rule. Every artifact matches a complete pinned C++ CascLib
read byte for byte. Source, binary and client metadata guards passed and
network syscall denial recorded no attempt. The report is in the external
acceptance laboratory; all raw artifacts remain outside Git in the private
run directory. Synthetic tests separately cover direct/copy differences.

This component does not parse M2 files, generate camera waypoints or establish
RustyCore camera playback compatibility. Its manifests and reports explicitly
set `m2_parsed`, `waypoints_generated`, `runtime_accepted`, and
`full_client_extraction` to false. Those later contracts need separate checked
reader and runtime evidence.

## Explicit Rust local acquisition (camera manifest v2)

The camera CLI accepts `--backend native|rust-local`, with native as the default.
Native output retains the camera v1 manifest/report contract. Without the optional
native build feature, default selection fails `backend_unavailable` before client
or output access.

Rust local requires explicit `--index-layout contiguous|paged` and
`--root-layout headerless-18125|header-30080|header-50893-v1|header-50893-v2`.
`--max-file-bytes` may reduce the 16 MiB file ceiling; its effective limit applies
to both the discovery DB2 and each camera. Native rejects layout/limit extras.
No automatic layout selection, backend fallback or network access occurs.

One `acquisition::Session` opens before output creation. The same session reads
the complete discovery DB2 and all discovered IDs, including zero. Input change
is globally fatal: stop immediately, retain the incomplete marker and previously
published files, publish no manifest, and perform no further reads. This global
post-bootstrap failure exits 4. Ordinary missing/incomplete files preserve the
partial/exit-8 behavior. Bootstrap errors create no output.

Rust local emits camera manifest **v2**, requiring the strict common acquisition
profile, complete CKey MD5 and EKey prefix9 declarations, explicit layouts, and
all effective budgets. Top-level source/file ceilings remain 16 MiB; the effective
limit is `acquisition.limits.file_bytes`. The offline verifier accepts old v1 and
new v2, enforces that limit on the source and every complete camera, redecodes the
preserved DB2 and requires the exact discovered file set. Schemas are self-contained;
the embedded common profile hash is in `contracts/forever-camera-contract-v2.json`.

The injected API `batch::extract_with_profile` takes
`Option<acquisition::Profile>` before its reader closure. Its caller must use a
reader matching the declared profile; the production CLI uses the common session.
Verification checks stored bytes, inventory, discovery and provenance structure.
It does not reauthenticate historical acquisition from the client, nor does it
parse M2, generate waypoints or establish runtime compatibility. New synthetic
cases cover native byte preservation, exact provenance, effective source/camera
budgets, zero IDs and immediate termination on changed input. Live backend
acceptance is recorded separately.

### Accepted Rust local CLI evidence

The local report `rust-cli-accept-20261011T012203Z-2aa558be.json` in the
laboratory's `reports/acceptance/` records successful camera v2 acquisition of
the complete discovery DB2 and ten cameras. All eleven artifacts matched the
previously accepted pinned C++ bytes. Reference filenames, rediscovered IDs,
manifest schema validation and the offline verifier passed.

A one-byte acquisition cap returned exit8 with a valid partial manifest and no
acquired assets. Three acquisition-profile mutations were rejected. Native
backend unavailability and invalid layout/cap options failed before client or
output I/O. Source, binary, client and reference guards passed, with no network
access recorded. These results establish raw acquisition and discovery for the
recorded scope; M2 parsing, waypoint generation and runtime acceptance remain
outside this evidence.

## Explicit build-70338 raw cameras (manifest v3)

Issue #46 adds `--profile camera-raw-70338-v1`, requiring explicit
`--backend rust-local` and both layouts. It binds target metadata from
`wowdbdefs-70338-v1` at WoWDBDefs commit
`d6f50d4f351193e84226aa62a36faa0f58528afa`, through the
[v3 contract](../contracts/forever-camera-contract-v3.json) and
[self-contained manifest schema](../schemas/forever-camera-manifest-v3.schema.json).

```text
forever-camera-extract --client <client-root> --locale esES --backend rust-local --index-layout contiguous --root-layout header-50893-v2 --profile camera-raw-70338-v1 --output <new-output-root>
forever-camera-verify --output <existing-output-root>
```

Legacy v1/v2 batch APIs, CLI routes and manifest decoders retain their
70170/70205 meanings and reject build 70338. Supplied bytes alone do not
authenticate a build; bare legacy discovery retains its layout-only contract.
The default remains native. Availability and bounded component/build admission
precede Session bootstrap and output; the opened Session must report the
admitted build. The new public selection is
`CameraContract::{Legacy, Build70338V1}`, with `discover_with_contract` and
`batch::extract_with_contract`; injected readers must implement the declared
acquisition profile.

### Target metadata and source semantics

The source remains FDID 1294214, requiring table hash `C2CEFDF9` and layout
`8BFD752B`, external IDs, five physical fields and seven scalar values.
[The exact public definition](https://raw.githubusercontent.com/wowdev/WoWDBDefs/d6f50d4f351193e84226aa62a36faa0f58528afa/definitions/CinematicCamera.dbd)
assigns physical field 3 / scalar index 5 excluding ID to `FileDataID<u32>`.
Accessor slot 6 including ID is a derived position, not a record length.
The definition has 25,678 bytes, Git blob
`3676f4e583d5c53c4eda065db9c322f9d45921f0`, SHA-256
`21f07cf5a2cdba30bec06895cb68e7467b58ab20cbee474f256473d2f70ca9d6`.
Both enclosing Git trees were independently reconstructed against the pinned
commit. The DBD does not declare the FDID; the target catalog/descriptor supplies
it. Definition semantics, catalog facts and legacy C++-header provenance retain
their distinct meanings.

Discovery captures direct dependencies before copy overwrites, exposing them
only after complete source/copy/tail validation. Zero survives, duplicates
collapse and unsigned IDs sort. Offline v3 verification rediscovers through the
same explicit contract and exact catalog/profile fields. Verification checks
stored bytes, inventory and provenance structure, without authenticating
historical client origin. Acquisition uses complete CKey MD5 and EKey prefix9.

The existing 16 MiB source/file, 4,096-ID, 256 MiB total including source,
100,000-direct-record, one-million-scalar and 2 MiB/depth-eight manifest limits
remain; file overrides only reduce limits. File-byte budgets do not bound total
Session residency or RSS. All dependencies are required; a valid empty source
is complete source-only output, while a missing source cannot substitute.
Individual missing/incomplete files remain partial; observed input changes are
globally fatal. Publication is exclusive per file with the manifest last and
an incomplete marker on failure, without atomic directory promotion.

### Frozen implementation and independent acceptance — 2026-10-14

Functional commit `bea89cecf979f0359a88faf3705cf76e3f17ae81` has tree
`f98a513f7d17a70707c636249bddf58c49139fd2`. The four production extract/verify
binaries were compiled from baseline `57ff6eb` plus the frozen delta before
that commit; the subsequent binding proves source equality. Each debug/release
profile passes 2,880 ordinary workspace tests / 26 ignored across 44 targets,
37 focused camera tests and four metadata-only CLI cases. Six formal JSON
methods, 339 source contracts, 162 distribution checks / one platform skip,
formatting and all-target Clippy pass. Focused suites overlap workspace tests;
ignored tests were not executed. These build/test commands have no syscall-trace
network claim. The 446-entry Rust/Cargo inventory overlaps the 654 source guards.

The authored campaign passes 58 runtime commands plus two in-process late-file/
late-empty-directory rejection controls. Three original DB2 consumers receive
only complete Rust/Python-admitted positives: copy overwrite, valid empty source
and zero/MAX discovery. Malformed DB2/package fixtures never reach the original
consumer. Ordinary and source-only packages pass both production profiles,
offline verification and formal v3 validation. Toy CASC does not execute positive
native CASC routing. All 58 consumer traces are empty.

Fresh current build-70338/esES Phase A passes seven commands: complete 674-byte
source, ten direct rows and no copies. Both raw Rust profiles match strict native
CASC whole bytes/CKey before any original DB2 consumer. Rust, independent Python
and original `DB2FileLoader` getters plus unchanged `ReadCinematicCameraDBC`/
`std::set` agree on the complete private ten-ID set. The original memory adapter
opens the accepted source twice; these are not extra CASC acquisitions. Phase A
acquires no camera dependency.

Under a distinct admission Phase B passes sixteen commands. All ten cameras are
complete: 26,296 bytes, maximum 3,820; with the source this yields eleven data
artifacts / **26,970 bytes** per production profile, excluding the manifest.
Both raw profiles, ten strict native whole-byte/CKey checks, both production
CLIs, separate offline verifiers, formal v3 validation, exact C++ filename rules
and complete inventories pass. The private FDID union is measured as eleven.
Phase B has 52 successful complete acquisitions / 132,828 bytes; with Phase A,
55 / 134,850 bytes. These count whole-file visits, not archive I/O or metadata
guard reads. The 23 real consumer traces are empty; denied controller/pre-exec
policy probes are separate, without a global zero-attempt claim.

Independent final review checks 202 Phase-B and 76 Phase-A evidence files,
2,062 guards, 35 aliases, six current/eight inherited exact stage inventories
and cleanup. Outer checks combine bounded metadata with per-read content
authentication and native comparisons, without an atomic whole-client snapshot.
No native malformed-input/failure-admission parity is inferred. The fresh
26,970-byte result does not reuse the historical 70205 total of 26,962 bytes.

The original pinned DB2/discovery closure executes through an immutable memory
adapter. This does not execute the original full map extractor or camera export
loop. Camera bytes stay opaque; M2 parsing, waypoints, playback/server runtime,
full extraction and a new bundle remain separate gates. Product 70338 stays
DB2-only pending versioned integration. Windows remains deferred.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70338-cameras-20261014-vlkbpbj6/`.
Relative receipt paths and SHA-256:

| Receipt | SHA-256 |
| --- | --- |
| `impl/source-v1/freeze.json` | `5c512f73f15e8244ecf50e523d3ca0999a51153a1d3e2039ece5be937bdc172a` |
| `impl/validation-attempt1/validation-report.json` | `005fe7854435528f247a924f39684805952a8b171b578349d69e3dbf47e97b22` |
| `qa/review/core-validation-review-v1.json` | `744eca904ba8a283c66723af4fc4b67cc641395a54b2aa5bb9f4a632fd8ddbd2` |
| `root/functional-binding.json` | `06261f2671df29ae11957330c105506621575a79c8f42d1157d81a6621c7761c` |
| `tooling/native-build-attempt1/build-report.json` | `a11d0692872008bcff5d07a962de35f6ae3af639f48e9b24c055ef6d7b3fee32` |
| `tooling/rust-build-attempt1/build-report.json` | `7d8dcdb561b4c614f02027507d25451a7c2e73a9856f1e4725bbb9d8619a0d50` |
| `tooling/runtime-freeze-v4.json` | `175efa1b11704e8948f8fbea863126d1b7db4210c8c16f55a089c67d3851a08e` |
| `tooling/synthetic-attempt1/acceptance.json` | `e531848215ae7162cf27231617b1b6a0b75a5b2515493d2e6c98aff350b72afe` |
| `qa/review/synthetic-terminal-review-v1.json` | `1b73107867864e9f9fa60634d8824a36770ffaf225f8e153d4a37d5447d25f3b` |
| `tooling/phase-a-attempt1/acceptance.json` | `73f05ce0bccf49c9a486f423635c77a5a14b77903e72df55548173b8e3d8feeb` |
| `tooling/phase-b-attempt1/acceptance.json` | `120d139e3ecf00dbe055cd5a8e4578aa6323ec78fd138e67823f2195382b883a` |
| `tooling/phase-b-attempt1/evidence-freeze.json` | `c939a85a67d73946277ff413241dbcb0309cee6c8db2f933cde64377f72c8f16` |
| `tooling/phase-b-audit-v1.json` | `769df3438d726c7f97495cd50c3f133b8e987632994972f362341b228673043f` |
| `qa/review/final-real-review-v1.json` | `d43f6584cdf92797b94e4b36bb059bbd6ef1dfb96decac6f77a3a6a188d89713` |

Unexecuted controller versions remain retained. Reviews corrected relative
guard normalization, exact file/directory inventories including an empty camera
directory, source rechecks before the original consumer and explicit Phase-A
schema/profile authentication in Phase B. The auditor's initial inventory-shape
assumption was repaired without rerunning producers. Product/helper sources
and binaries stayed unchanged. The later three-file Markdown update is not
rebuilt into those binaries or a new distribution bundle. Raw files, rows,
IDs, content correlations and keys remain private outside Git/CI.
