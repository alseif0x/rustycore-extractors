# Explicit captured-volume collision storage

The optional Linux Spool exposes `create_with_profile` with explicit
`Profile::CapturedVolumesV2` (`linux-private-collision-spool-captured-volumes-v2`).
The original `create` always selects `SameMountV1`, preserving its source-to-output
mount equality requirement. V2 permits independently captured source and private
output mounts; it performs no mounts, copies or relocation.

Every source/key/output ancestor retains its original held descriptor, named
object identity and mount ID. Private output must still have a 0700 parent,
exclusive new namespace, no source/key containment or object alias, and records
on the captured destination mount. Every record retains full metadata, held /
named identity, SHA256 and EOF checks. All independent guards and sticky failure
semantics remain unchanged. Explicit protected inputs are caller assertions and
must include every supplied key file; source directories are identity anchors,
not authenticated membership or live-client atomic snapshots.

This is a storage capability only. Original source/read/model, disk, descriptor,
active-memory and reload ceilings all retain their downward-only v1 maxima.
Intermediate and native output formats, default product CLI, routing and v7
corresponding-source contract remain unchanged. Full-world scalable limits,
product integration/publication/resume, mandatory encrypted inputs, real geometry,
runtime and distribution remain separate completion gates.

## Evidence

Final source passes all 40 private Spool tests (nine new) and all 39 closed-package
cases in each Linux debug/release profile, CASC library/test Clippy, three scoped
formatting checks, 171 source/state contracts and 104 distribution/sealer checks.
The explicit second-filesystem test is ignored in ordinary runs, then executed
separately in both profiles with `RCE_SPOOL_CROSS_VOLUME_PARENT` pointing to a new
0700 parent on the external NVMe. It requires distinct source/output filesystem
devices, with source toy CASC on the workstation root; it is not a simulated mount
comparison or a real Blizzard client.

Each profile accepts four complete authored toy-CASC packages against resident
Rust: empty world, startup duplicates/texture omission with pre-ID-dedup M2 names,
cached and later-liquid WMO/doodads/globals, and parent-only/grandchild cache barriers.
The comparison checks complete inventory ordering/names/bytes, component counts
and recorded acquisition facts: 27 native files /
2684 bytes. A fifth whole-world case requires the exact
terminal Association error for a nearer mixed-parent payload. All six repositories
use the explicit v2 profile. All five cases separately check that default v1 rejects
before creating its cross-volume namespace. Completed map mutation remains fatal.

Unit cases separately retain every source/key/output ancestor mount, record mount,
source/key identity, alias/symlink/private-parent/existing-output rejection,
complete bytes and sticky quota guards. Original 31 Spool tests remain unchanged;
all original production text can be reconstructed by removing only the profile
declaration, wrapper/getter/field and one reviewed mount-admission dispatch.
Full path, alias, node/record/hash/EOF, resource-limit and final guard bodies are
byte-exact. No limit or parser admission has been relaxed.

Ten complete traces pass strict IPC-only audit. A live-held D-Bus unit reference
records loaded terminal success for invocation
`5e92817d3a46437d911a1cc2fa9ba48a`; source before/after and retained executable
hashes match the final frozen candidate. Attempt 1 rejects an unused test-helper
argument at Clippy, before test execution. Attempt 2 removes that argument and
reruns the entire final plan. Both frozen sources/reports/build targets remain
retained without relabeling.

| Evidence | SHA256 |
| --- | --- |
| Final source freeze, 992 entries / four owned paths | `8130ef3c871aa26e6c3c89ba79612a183c64a0990fd848cff05691d705be286e` |
| Ten-command final check report | `d488a21e146fa1659d993adab8577f9f16c033bf78b0b7c549906d0e781bf543` |
| Root saved-evidence acceptance | `8d152c36394377004156f12e6b9e28ddd1515f91c28fe0eb2dff967c83db2192` |

Private retained evidence is under
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate/collision-captured-volumes-20261012T041059Z`.
The accepted checker is `checks-attempt2.py`, with `source-freeze-attempt2.json`.
Publication qualifies documentation after acceptance; the three Rust paths remain
byte-identical to that freeze. No final full-workspace/full-collision local run,
new original C++/real-client comparison or distribution bundle is inferred.

Preceding canonical `2fe8cfb` independently passes hosted
[Rust CI 38202523503](https://github.com/alseif0x/rustycore-extractors/actions/runs/38202523503)
and [C++ CI 38202523528](https://github.com/alseif0x/rustycore-extractors/actions/runs/38202523528).
Linux debug/release each record 2,555 passed / zero failed / 22 ignored; Windows
debug records 2,116 passed / zero failed / 20 ignored. Retained logs contain 37
Linux-debug result summaries and 41 each for Linux-release/Windows; identical
summary counts are not inferred. These hosted results predate the v2 source;
hosted downloads remain separate from local IPC-only evidence.

The product remains resident. Scalable versioned source/read/disk/reload limits,
whole publication/resume with actual LocalStore recheck, complete mandatory
encrypted inputs, real geometry/runtime and reproducible Linux/Windows
distribution remain open. This storage capability does not complete extraction.
