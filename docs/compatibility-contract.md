# Compatibility, manifest and network contract v1

This document freezes the public v0.1 contract. It is normative together with
[`contracts/extractor-contract-v1.json`](../contracts/extractor-contract-v1.json), the JSON
Schemas under [`schemas/`](../schemas), and their synthetic fixtures. If prose and a
machine-readable constraint disagree, the stricter constraint applies until the discrepancy is
reviewed.

This is an engineering compatibility contract, not a claim that Blizzard client data may be
redistributed. Extracted data remains local to the user and must not enter Git, CI caches or
release artifacts.

## Fixed compatibility target

| Property | Required value | Pinned evidence |
|---|---|---|
| CASC product | `wow_classic` | Extractor defaults at TrinityCoreLegacy commit `92796557f9b0ba3d2d1c7c770f535153154cf83e` |
| Client | `3.4.3.51943` | Project v0.1 target |
| Map | ASCII magic `MAPS`, version `10` | `src/common/Collision/Maps/MapDefines.cpp` |
| VMap | ASCII magic `VMAP_4.B`; raw intermediate `VMAP04B` | `src/common/Collision/VMapDefinitions.h` |
| MMap | ASCII magic `MMAP`, version `15` | `src/common/Collision/Maps/MMapDefines.h` |
| Detour | navmesh version `7`, 64-bit polygon references | `dep/recastnavigation/Detour/Include/DetourNavMesh.h` |

The files above were inspected with `git show` at the pinned commit. The initial implementation
must also use that exact patched Recast/Detour closure; matching only MMap version `15` is not
enough.

Build `51943` is supported. Any other build is rejected with exit code `3` before output is
created unless the caller passes `--experimental-unsupported-build`. Experimental runs retain
the target formats but set `compatibility.support` to `experimental`, record the detected build
and a reason, and must never be described as compatible.

The supported locale identifiers are enumerated in the machine-readable contract. At least one
installed locale is required. Locale-specific DB2 files are written under `dbc/<locale>/`; the
historical directory name remains `dbc` even though the files for this client are `.db2`.

## Output tree

A complete output root may contain only the requested categories below plus
`data-manifest.json`:

```text
data-manifest.json
dbc/<locale>/*.db2
gt/*.txt
cameras/FILE????????.xxx
maps/<map>_<grid-x>_<grid-y>.map
maps/<map>.tilelist
vmaps/<map>.vmtree
vmaps/<map>_<tile-y>_<tile-x>.vmtile
vmaps/GameObjectModels.dtree
mmaps/<map>.mmap
mmaps/<map><tile-y><tile-x>.mmtile
```

`Buildings/`, raw `VMAP04B` intermediates, meshes, logs, CASC caches and temporary files are not
final artifacts. Cameras are optional when the client has no referenced camera files.
`GameObjectModels.dtree` is required whenever VMaps are requested and game-object model input is
present. MMaps require complete Maps and VMaps produced under the same manifest contract.

The existing `/home/server/woltk-server-core/Data` tree confirms the directory and filename
shapes but is not conformance evidence because its producing extractor version and options are
not recorded. RustyCore currently consumes DB2 data from `dbc/<locale>`, GT tables from `gt`, Map
tiles from `maps`, and navigation data from `mmaps`; reader/runtime acceptance remains mandatory
in the later conformance issue.

## `data-manifest.json`

Every promoted output has a manifest conforming to
[`data-manifest-v1.schema.json`](../schemas/data-manifest-v1.schema.json). Its stable top-level
sections are:

- detected client product, version, build, locales and a path-free installation fingerprint;
- extractor version, Git commit, implementation kind and pinned compatibility-oracle commit;
- all format magic/version values, including the Detour version and polygon-reference width;
- effective locales, requested components and format-affecting options;
- network mode, authorization state, lock digest and downloaded byte count;
- per-category file counts, byte counts, completion state and integrity digest;
- run status and timestamps; and
- explicit confirmation that paths are redacted and no client data is embedded.

The installation fingerprint is SHA-256 over canonical JSON containing only product, build,
sorted locale identifiers and CASC build/config content identifiers. It never contains the
installation path, account data or client file bytes.

Integrity inventories are UTF-8 text with one line per regular output file:

```text
<lowercase-sha256><two ASCII spaces><POSIX relative path><LF>
```

Paths are sorted by their UTF-8 byte sequence. They must be relative, normalized, contain no
`..`, and remain inside the output root after symlink resolution. `inventory_sha256` is SHA-256
of those exact lines. The overall inventory excludes `data-manifest.json` and any incomplete
marker so the manifest does not hash itself. Counts and byte totals cover the same files as the
inventory.

The repository fixtures are synthetic examples only. They deliberately contain no Blizzard data
and no server paths.

## Stable command-line interface

The release executable is `rustycore-extractors` and exposes these commands:

```text
rustycore-extractors inspect --client <dir> [--json]
rustycore-extractors extract --client <dir> --output <dir>
  [--locale <locale>]... [--components <csv>] [--json]
  [--experimental-unsupported-build] [--resume <run-id>]
  [--allow-network [--remote-casc]]
rustycore-extractors verify --output <dir> [--json]
```

Unknown flags and missing values are usage errors. Repeated `--locale` values are deduplicated and
sorted in the manifest. If no locale is given, all installed supported locales are selected.
Default components are `dbc,gt,cameras,maps,vmaps,mmaps`. The v1 interface does not silently read
the current directory as a client or output location.

Before `extract` writes anything, it emits a plan containing detected product/build/locales,
selected components, staging/final destinations, estimated disk requirement, duration and work
units, and network requests/download bytes. Unknown estimates are displayed as `unknown`, never
as zero.
The plan is informational; the explicit command flags are the authorization.

Human mode writes the plan/progress/summary to standard output and diagnostics to standard error.
With `--json`, standard output is exactly one JSON document conforming to
[`command-result-v1.schema.json`](../schemas/command-result-v1.schema.json); progress and all
diagnostics are embedded in that document, and normal operation leaves standard error empty.
Diagnostic codes, JSON field names and exit codes are stable within contract version 1. Human
wording is not an API. The `details` object always has stable `client`, `plan` and `output` slots;
commands set inapplicable slots to JSON `null` rather than omitting them. Unknown numeric plan
estimates are also `null`.

| Exit | Meaning |
|---:|---|
| `0` | Requested command completed successfully |
| `2` | Invalid invocation or option |
| `3` | Unsupported product/build without experimental opt-in |
| `4` | Client missing, unreadable or structurally invalid |
| `5` | Network request forbidden, unpinned or checksum-invalid |
| `6` | Insufficient disk, memory or another preflight resource |
| `7` | Extraction stage failed after a run began |
| `8` | Output conflict or incomplete run requiring explicit resume |
| `9` | Verification or compatibility check failed |
| `10` | Internal invariant failure |
| `130` | Interrupted run |

An interrupted or partially successful extraction never exits `0` and never reports JSON status
`success`.

## Offline and network contract

`inspect`, `extract` and `verify` are offline by default. In default mode they perform no DNS
lookup, socket connection, telemetry, update check, TACT-key fetch, listfile fetch or remote CASC
access. Proxy environment variables do not change this rule. No mode uploads paths, manifests,
hashes or client content.

`--allow-network` authorizes only resources present in the shipped
[`network-lock-v1.json`](../contracts/network-lock-v1.json). Each metadata resource requires an
HTTPS URL, exact byte size, SHA-256 digest and deterministic cache key. A cache hit is verified
before use; a mismatch is deleted from the tool-owned cache and fails with exit `5` unless a
fresh, locked download is explicitly allowed.

Remote CASC additionally requires `--remote-casc` and a locked profile containing the exact build
and CDN configuration hashes. The preflight plan must display the region, content identifiers,
cache destination, estimated download and the warning that client data will be downloaded from
Blizzard infrastructure. `--remote-casc` without `--allow-network` is a usage error.

The v1 network lock intentionally contains no resources or remote profiles. Therefore current v1
commands have no permitted network destination even when `--allow-network` is present, and remote
CASC fails with exit `5`. Adding a resource or profile is a reviewed contract change; a moving
`latest` URL or an entry without a checksum is invalid.

## Atomic output, failure and resume

Extraction uses a sibling directory on the same filesystem named
`.rustycore-extractors.<run-id>.staging`. Before the first artifact write it atomically creates
`.rustycore-extractors-incomplete.json` inside staging with manifest status `incomplete` and
fsyncs the marker and directory.

Artifacts are written to temporary names, fsynced, validated and renamed into staging. The final
manifest is written last. A complete run then removes the incomplete marker, fsyncs staging and
atomically renames staging to the requested output. The final output path must not already exist;
v1 has no implicit merge, cleanup or replacement behavior.

Failure or interruption leaves staging and updates the marker to `incomplete` or `failed` when
possible. It never promotes staging or changes a pre-existing final output. `--resume <run-id>`
accepts only the exact sibling staging directory whose client fingerprint, extractor commit,
effective options, network lock and target output match. Verified completed files may be reused;
mismatches fail with exit `8`. Resume never searches arbitrary directories and never follows a
staging symlink.

`verify` rejects an incomplete marker, schema errors, unknown files, unsafe paths, count/hash
mismatches, format/header mismatches and missing requested categories. It does not repair output.

## Redaction and diagnostics

Human output may show a user-supplied path only in the initial local plan. Persisted manifests,
JSON results, logs and diagnostics use role labels such as `<client>`, `<output>` and `<cache>` or
paths relative to those roots. Query strings, authorization headers, environment values, account
identifiers and encryption keys are never printed. Errors expose a stable code and safe field
name, not raw library exceptions containing absolute paths.

## Contract validation

Run the focused, non-Rust checks with:

```bash
python3 -m unittest discover -s tests/contracts -p 'test_*.py' -v
```

The tests validate the schemas and positive fixtures, reject unsupported-build and incomplete
promotion cases, enforce the paired network flags/lock, confirm that every command defaults to
offline, and scan fixtures for local path disclosure. Runtime implementations must add a socket-
denial test proving that each default command makes zero external requests; the machine-readable
contract and empty lock make that future test deterministic.
