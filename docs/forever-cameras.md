# Forever complete camera files

This component preserves the complete `CinematicCamera.db2` and each camera
file it references. Its separate v1 contract is
[`forever-camera-contract-v1.json`](../contracts/forever-camera-contract-v1.json),
with a strict [manifest schema](../schemas/forever-camera-manifest-v1.schema.json).
The admitted product is `wow_classic_beta`, builds 70170 and 70205, using the
existing `enUS` or `esES` storage root selection.

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
