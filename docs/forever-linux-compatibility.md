# Functional Linux extraction

Issue: [#72](https://github.com/alseif0x/rustycore-extractors/issues/72).

## Delivery scope

On 2026-10-15 the operator selected stable, usable Linux extraction matching
the Forever TrinityCore tools as the next milestone. Broader coverage follows;
Windows delivery remains deferred. The current implementation is a source
candidate, not an accepted extraction CLI or release.

The upstream setup guide reports a working Linux server using data extracted
on Windows, with navigation disabled for that test. Its README declares build
70338; our current development input is 70374. Comparing upstream commits
`8c5325b37dfa155450a9e87f213965f6e6c00afe` and
`8a864593bb4d566153e89058b33793752ff81c57` finds identical blobs, modes and
types for all 125 relevant extractor, CascLib and DB2 paths. Upstream's DB2
reader permits zero-filled encrypted spans and skips sections without a key;
Map consumers omit the resulting unavailable records. These are source facts
and an upstream operational report, not our real-input acceptance.

References: [upstream setup](https://github.com/advocaite/TrinityCore/blob/8a864593bb4d566153e89058b33793752ff81c57/doc/LinuxServerSetup.md),
[DB2 source](https://github.com/advocaite/TrinityCore/blob/8a864593bb4d566153e89058b33793752ff81c57/src/tools/extractor_common/DB2CascFileSource.cpp).
The earlier review remains in `forever-completion.md`.

## Separate availability capability

Existing complete-file/CKey APIs, strict profiles and failed batches retain
their contracts. The new capability must keep unavailable spans inaccessible
and retain explicit coverage. It cannot convert a partial result into
`VerifiedFile` or claim complete plaintext authentication.

The initial CASC profile is Linux, build 70374 and disabled keys. A complete
read uses the existing strict path. An encrypted-only failure may use a
separate encoded-source check: the independently selected full EKey binds
the BLTE header, or the whole headerless object, and every frame hash, extent
and EOF must validate. N/Z ranges decode exactly; encrypted ranges remain
inaccessible. Different authenticated partial encodings are ambiguous and
reject. Candidate counts include the preceding strict pass; decoded extents
include unavailable spans. These are logical budgets, not RSS or total CPU
measurements.

The pure DB2 interface borrows only fully available ranges and proves no
cryptographic identity itself. Explicit current-catalog Map and
GameObjectDisplayInfo projections return a distinct snapshot with section
coverage. The initial disabled-key policy admits zero-Tact sections and omits
all nonzero-Tact sections without reading their payloads. Missing bytes in an
admitted section or its required string dependency are fatal. Declared rows
and copies in omitted sections still count against limits. Physical order,
direct IDs and raw copy facts remain distinct; terrain copy materialization
uses admitted direct sources only. GODI does not replay copies.

## Acceptance sequence

1. Qualify the new source and authored fixtures, then compare supplied positive
   inputs against the identified original C++ readers. Native malformed-input
   behavior is not a safety oracle.
2. Probe real available Map/GODI inputs under an explicit new admission and
   record actual admitted and omitted coverage. No private rows, key names,
   keys or asset correlations belong in Git or CI.
3. Prove complete requested terrain tiles using available Map rows and complete
   liquid/WDT/ADT inputs. Missing requested maps, required ancestors or present
   tiles reject that selection.
4. Integrate a versioned product CLI with sealed coverage evidence and matching
   verification/resume rules. Add collision and navigation after their own
   selected-input and output comparisons.

Global source coverage and requested output coverage are separate. A complete
selected terrain map may use a globally incomplete Map table when its required
row and all terrain inputs are available. That does not establish complete
global DB2 data, collision models, all-world extraction or runtime acceptance.
Existing v11 and v8 product promises are not widened by this candidate.

## First bounded real reader checkpoint — 2026-10-15

The source-bound Linux diagnostic compiled from `2273d4f45b3ad865873e2b48ef681bdd789c0f69`
passes its first build-70374/esES run with disabled keys:

| Table | Available direct rows | Copy pairs | Admitted sections | Omitted sections |
| --- | ---: | ---: | ---: | ---: |
| Map | 71 | 0 | 1 | 3 |
| GameObjectDisplayInfo | 3,195 | 20 | 1 | 1 |

Map materialization retains 71 rows. GODI copies remain inspection facts and
are not replayed. Both sources have authenticated encoded keys and checked
frame coverage; neither has complete decoded CKey verification. Map has 607
unavailable bytes out of 10,191, and GODI has 360 out of 116,270. No rows or raw
files are exported. The two-request diagnostic has zero retries, an empty child
network trace and a closed process group. Client metadata and the private
logical source identity match the preceding accepted liquid input.

The corrected whole offline qualification passes all six commands: debug and
release each pass 3,043 tests with 26 historical ignores across 48 targets,
including eight diagnostic tests; both example builds, Clippy and format pass.
Separate checks pass 363 contract tests and 162 distribution tests with one
platform skip. The earlier traced timeout and module-order formatting failure
remain failed. The later two private-helper Linux `cfg` attributes repair
non-Linux linting; the recorded diagnostic retains its original producer binding.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-functional-db2-20261015-39o2hd2n`.
Whole qualification SHA-256: `7f921223a3da5693ec17deaeb48c95181d06fa4980e5f9431587543560d19ff7`.
Independent qualification QA: `33ce92a71c9d2d9eb4b909fed1e7617523da3d70e4030dd46d9a754c460e6716`.
Aggregate real closing receipt: `332c352719ff5ad36723ef5f625651d27facac57cd5b2833488eae2da70eb6f6`.
Independent real QA: `76ae874da2a5ad72aa4baf9a45d21a8dc9eda43e9b5a85ff1121301f3c5b367c`.

This establishes available-row acquisition and projection. No terrain, model
conversion or runtime consumer executes in this diagnostic.

## Original available-section getter comparison

The separately qualified original C++ loader/getters match Rust on all eight
authored positive fixtures: 14 direct records and 16 copy pairs across complete,
mixed, all-unknown and empty-middle section cases for both tables. Authored Map
materialization separately yields eight rows; original `ReadMapDBC` is not run.
All 42 build/comparison commands exit zero with closed groups. Six original
translation units and one authored adapter compile; full-system dependencies
are admitted before compilation and checked again afterward. Sixteen compiler
traces pass their reviewed local IPC policy; 26 comparison traces are empty.
All committed DB2 files at `65d0342` match the companion's immutable dependency.

Campaign receipt: `d1556305e7f9f0d08c54d545245fa9e3ee777577d6645c375f55605ee1548853`.
Independent terminal QA: `e8ad4c0f70d642e54fae12644dd20315ac666afa502e351829f06582448e0b5f`.
The initial execution admission omitted two already-guarded runtime libraries;
its failed review is preserved. The corrected admission adds only those loader
bindings and passes independent review before the single campaign execution.
There is no native CASC, real-input native comparison or full extractor claim.

PR #73 merged as `4b9a67767ef382e5a7accdf53ecbaf8f1663d491` after all six
CI jobs passed on its exact head. Its implementation branch is deleted.
[Issue #74](https://github.com/alseif0x/rustycore-extractors/issues/74) now tracks
selected real terrain through a separate 70374 API. Map 0 tiles (31,31) and
(31,32) are the fixed proposed request, not established available inputs.
Missing maps/tiles or incomplete dependencies must fail the whole selection.
The selected terrain, versioned product CLI and complete extraction remain open.
