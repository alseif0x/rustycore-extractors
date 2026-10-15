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

## Selected-terrain source candidate

Issue #74 adds a separate Linux-only `terrain::available_70374::run_selected`
capability for at most two explicit tiles on one map. Available Map coverage
remains separate from complete liquid/WDT/ADT source facts. Both requested
tiles must exist; missing or incomplete dependencies, sink errors and source
changes reject the whole selection. Callbacks remain provisional until success.
No tile lists are emitted. The pure 70374 conversion/assembly/verification
entry points share the existing kernels while preserving older API build gates.

The private `terrain_available_70374` example fixes Map 0 tiles (31,31)/(31,32)
and creates output only after successful conversion of both. Its explicit
`--native-inputs` option additionally stages the same complete ADT buffers,
complete typed liquid lookups and contexts for an original-converter comparison.
These private artifacts are not a product format. A separate 64 MiB caller
reservation reduces the runner's memory ceiling; no extra acquisition occurs.
No raw Map/DB2 buffers or inaccessible spans are exported.

Focused debug/release checks each pass 226 tests with one historical corpus
ignore: terrain 156, CASC terrain 57, availability 9 and the example 4. Selected
all-target Clippy and formatting pass. The initial catalog-gate and duplicate
fixture-module failures remain failed and retained. Source freeze:
`aea7e2da9fdd37ec53036d930629282ff80b836e76a89903ae39f16c136608e6`;
validation receipt: `6be70b30725e6bc5a5efa8139eaa098d5efdc92a751e1bc6e0b36241984dfa4b`.
Independent source QA passes without findings:
`e4497532b5144fa50861bc7b6225990637a4630dd8eaf7241cecd1f59795315f`.

The fresh immutable whole-workspace qualification binds the 729 committed
files at `ffdec1aa86fc4a266f6da323fde6ba5ca0e521c4`, plus 41 pinned vendor/config
files. All six commands pass: debug/release each pass 3,061 tests with the same
26 historical ignores, both example builds, all-target Clippy and formatting.
Independent terminal QA checks the source snapshot, software guards, streams,
owned groups, compiler traces and two literal producer copies without findings.
The two workspace test commands retain `network_not_observed`; four compiler
commands have separately classified local IPC traces. Qualification receipt:
`66d83d46834c61fec363305a28551167cb1ebe539300b3ada7ca273fb6bc6405`;
independent QA: `46145b344ab5d013420a8ad50ef9a8e4489eea2d6736a366f749e0f3c10cf702`.

The current-original supplied-input converter builds separately against the
pinned upstream `8a864593bb4d566153e89058b33793752ff81c57` sources. Static
inspection retains the four required parser/converter functions and no forbidden
acquisition/exporter functions. An initial overly broad symbol rule rejected
two inert, identical weak shared-pointer destructors; that attempt remains
failed. A narrow exact-symbol rule and ten passing controls precede the new
successful static inspection. Neither inspection executes the native program.
Build receipt: `31fa1e0a66b01c8c2e479f4a3c1372db78d475bcbc964781da7f0fa385ac8d2d`;
static receipt: `2364318ae6b98c1beb8627706a13c345f8dd8c6e9883e0609754e72b6683cf3a`.

The subsequent four authored complete ADTs compare flat/ramped heights with
packing disabled/enabled. All 15 commands pass, including four Rust conversions
with strict 70374 verification followed by four original parser/converter calls.
All four complete MAPS files match byte for byte (200,470 bytes total).
Campaign receipt: `86afa9f62a434f9750780bf3b0bbc7efea64a0f14982be6afe5ed5b7d2d008a7`.
Independent terminal review passes without findings:
`c5e06a553ff9774addc33df3aeb8a94017791abea3d31fa243dad129006066eb`.
All 15 groups are absent; fourteen runtime traces are empty and the compiler
trace passes its local IPC policy. This comparison reads no client and executes
no native CASC or complete exporter.

## First complete real selected-terrain run

The fixed Map 0 request for (31,31)/(31,32) completes on the same admitted
70374/esES source with disabled keys. Rust emits two MAPS files / 142,084 bytes
from 1,253,174 logical source bytes, including the declared Map size. Map retains
71 direct/materialized rows, one admitted section and three omitted sections.
Both requested ADTs and all liquid/WDT inputs use the complete-source path.
There are no tile lists or automatic retries. The owned group closes, the child
network trace is empty, and software/client metadata checks pass.

The optional private witness contains the same two complete ADTs, contexts and
one materialized liquid lookup. The closing controller checks their CKeys,
coordinates, private source identity and exact owned output inventory. It
checks MAPS framing and extents, not an independent full MAPS interpretation.
No raw DB2, key material or client asset is committed or published.
Invocation receipt: `9b8681cf9e50d473929bf1623cc5b470d3795b44ca86fe358e47628fc5657446`.
Private completion-report receipt:
`a944d8abe9a4d988656fe46afcfb8434a5310d15b9d80745659d75e3c2ae363e`.

Independent terminal review passes without findings:
`34ed46c7cd581f63d59cf6ba95a015b0c84167679baa5353dcbfa764636a36da`.
It independently checks all 3,956 software guards / 48 aliases, the single
invocation and eight private files, without another acquisition or producer run.
Map's complete decoded CKey remains unverified; the requested terrain inputs
have complete-source verification. This two-tile check does not establish a stable
whole-map extractor, product publication, collision/navigation or runtime
acceptance. The next product milestone must expand usable extraction coverage;
the two-tile proof is not the final delivery requested by the operator.

## Current-original comparison on those real inputs

The separately qualified Rust conversion caller replays each same-buffer ADT,
liquid lookup and context, executes strict 70374 MAPS verification, and matches
the complete acquired MAPS bytes before the native call is admitted. The
current-original parser/converter then runs once per tile. Both original outputs
match the acquired files byte for byte: two files / 142,084 bytes, with four
complete comparisons including the Rust replays. All four calls exit zero,
groups close and child network traces are empty. No new CASC acquisition occurs.

The pure final closing binds the original real extraction and independent QA,
the authored producer qualification, all input CKeys/contexts, per-call guards,
prechecks, streams and exact private output inventory. An initial source-only
comparison proposal missed final dynamic evidence guards; it remains unexecuted
with its finding retained. The corrected proposal closes each complete per-call
guard map and passes five controls before the four actual calls.
Closing receipt: `af4cf9efcfda48a52224579adeb5c7a76bd9005325ef8236040a9536b1485e70`.
Independent terminal review passes without findings:
`e78aca423acfc6d946b914914c0fcb6091d5a4b6fdb17de1483c688e35b0d629`.
It independently rechecks all base/dynamic guards, 78 aliases, four current
absent groups, input/source bindings, full bytes and 51 owned evidence nodes.

This compares supplied complete inputs with the current-original converter;
it does not execute native CASC, the complete original `ExtractMaps`, a whole
map or a runtime consumer. The versioned CLI and scalable map traversal are
the next implementation milestone.

## Full-map product candidate

[Issue #76](https://github.com/alseif0x/rustycore-extractors/issues/76)
introduces `run_full_maps` and explicit Linux70374 terrain-only product v12.
It traverses every coordinate of selected complete maps, streams verified
MAPS and whole-map tilelists through the existing package lifecycle and
retains available Map coverage. Known-present failures reject the selection.
Both offline verification commands check exact tile/list/coverage associations;
sealed-stage resume replays and compares the retained artifacts.

The six-file API source review passes; the twelve-file product review found
one inherited unknown-version test that still uses newly admitted version 12.
The root correction changes that fixture to version 13. Focused product
checks alone do not establish historical CLI regression or real full-map
acceptance. Joint immutable qualification and the first actual full-map
product attempt remain pending. Public plans, command shapes and boundaries
are in [the v12 runbook](forever-terrain-v12.md).
