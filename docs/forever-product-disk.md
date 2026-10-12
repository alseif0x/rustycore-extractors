# Explicit product collision disk plan v3

Authored 2026-10-12. Contract: `forever-product-collision-disk-v1` in
`contracts/forever-product-disk-contract-v1.json`. This checkpoint records the
implementation and its required corresponding source. Synthetic GNU/Linux CLI and complete workspace validation are accepted below. This document supplies no new real-client, native or runtime acceptance.

## Selection and admission

Canonical `forever-product 3` plans explicitly select
`collision-storage linux-leased-append-disk-world-v1` and seven decimal byte
budgets, in RAW, WDT, cache, placement, VMO, map and final-output order. The actual
wire syntax uses a single TAB between each key and value and one final LF. All
six archive budgets are at least 32 bytes and at most 128 GiB each; final output
is at least one byte and at most 64 GiB. The output budget limits the whole common
package across DB2, GT, cameras, maps, VMAPs and MMAPs, including the selected
collision artifacts. Offline verification enforces the same whole-package byte
cap without requiring scratch or keys. These only reduce the disk-world and common-package ceilings.
The collision component is required. Decryption remains independently explicit:
`disabled` or `provided-salsa20-v1`. Paths, key identifiers and key material do
not enter the plan. The canonical plan hash binds storage, budgets and all prior
options in the common package context. Existing v1/v2 grammar and resident
assembly remain unchanged; the disk backend is never selected automatically.

`extract` and `resume` require `--scratch` for a v3 plan and reject it for a
resident plan. Plan inspection and offline verification need no scratch path.
Disk execution is limited to Linux GNU x86_64. Inspecting the canonical plan
on another target does not grant an execution capability. Unsupported execution
must fail before client acquisition or output creation.

## Private phases and publication

The driver holds rooted source, output-parent and scratch-parent guards. Their
namespaces are disjoint; symlink, mount and identity changes reject. Paths are absolute and
bounded to 4,096 bytes and 32 components; preflight reserves namespace and owner
path capacity before acquisition; scratch must be an existing directory
with mode 0700 owned by the effective UID. The held mount namespace is checked. A fresh
exclusive private namespace contains six independently guarded leased append
owners using `CapturedVolumesV2`, `DiskWorldV1` and `LeasedAppendV1`. It connects
startup, world traversal, placement staging, VMO preparation and final package
assembly through the same library APIs. Archive budgets include private framing.
The complete checked package exists before the first collision artifact enters
the common product `Run`. Files retain normal `vmaps/` names and the existing
transaction, receipt, seal and final-manifest protocol.

An unfinished collision component is recomputed into a **new scratch namespace**
on resume. Retained scratch archives are neither adopted nor removed; they are
private failed-attempt evidence, not a resumable cache. An already verified sealed
component follows the existing product skip policy. Resume binds the same source,
producer and canonical plan. Any acquisition, guard, conversion or publication
failure is terminal. An available prefix never accepts a failed selection.

## Physical space and memory

Preflight observes free bytes and, where reported, free inodes through borrowed
open directory descriptors before client acquisition or output creation. Byte
arithmetic is checked. Budgets sharing a device are aggregated, including output;
separate filesystems retain separate allowances. File rounding, receipt/directory
allowances and 64 MiB free headroom per endpoint are conservative admission
policy. Same-device aggregation retains both endpoint allowances.

This is **not an atomic reservation**, filesystem snapshot or guaranteed capacity.
It does not measure all metadata, journaling, copy-on-write, quota usage, allocator
RSS or all CPU work. Concurrent consumers can exhaust space after admission;
every later write error remains terminal. Source and scratch guards do not turn
that observation into a storage reservation. Logical conversion limits stay
bounded: the 2 GiB combined logical ceiling reserves 258 MiB for the common
product `Run` and 16 MiB for guards, reducing conversion admission accordingly.
No library memory ceiling is raised by this CLI selection.

## Retained synthetic integration evidence

The independent integration run accepts twelve directed v3 CLI cases against
complete authored toy-CASC inputs: all six product components with actual
navigation tiles, plaintext and supplied synthetic Salsa20 keys, exact resident
artifact comparison, admission/alias/key-path failures, whole-package output
budget failure, and a real injected SIGKILL followed by resume. The interrupted
collision attempt retains its private namespace; resume uses a new one and
produces the exact baseline final manifest. No real key material is used.
There are 38 retained actual CLI traces; these contain no network attempts.
The outer Python CLI harness is not syscall traced and has no network-absence
claim. Separately traced contract and distribution commands retain their own
strictly classified traces.

Accepted integration report SHA-256:
`f06d71ca87468bca9e0f8f4205920be292e34370f280b044addf6e254d5fda3a`.
Live terminal observer SHA-256:
`a8bad08dd4379b3482fe1b1dd42a943fce0bda178db163353de2d1cf8fb5e3b9`.
Immutable executable/test source inventory SHA-256:
`44575e7be4feed983ac8866b3e8d300dbd20649d515aabe2a623574653446351`.
Retained evidence archive SHA-256:
`b9517ee467f38344d4121aa379a171a384eb3aac50b605ecb8c28e21c231bc66`.
All are retained outside Git under the product-collision-disk
`20261012T103428Z` candidate run. The observer holds the exact live systemd
invocation through its terminal exit; source/tools and raw traces are reviewed
independently. This preliminary run also passes 187 contract and 112
distribution/sealer checks. It does not substitute for final complete validation
or establish a new reproducible bundle.

## Replaying authored CLI checks

The opt-in `tests/forever/test_product_disk_cli.py` suite requires an exact
`RCE_PRODUCT_BINARY` and a fresh external `RCE_PRODUCT_DISK_CASES` path.
`RCE_SPOOL_CROSS_VOLUME_PARENT` must name an existing private directory on a
second filesystem for the cross-volume case. The harness retains authored
clients, outputs, scratch and actual CLI traces outside Git; it never substitutes
real client data or production keys. Missing second-filesystem evidence fails the
explicit case rather than counting it as a passing skipped test.

## Complete source validation and final association

The frozen implementation and existing tests pass 2,643 ordinary tests across
41 targets in each Linux debug/release profile (26 ignored), plus four explicitly
executed ignored archive/resource/cross-volume corpora per profile. All-target
Linux and GNU-Windows Clippy, both GNU-Windows compile profiles and scoped
formatting pass. Windows binaries are compiled, not executed by this evidence.
Each profile passes the complete 49-case v1/v2/v3 CLI suite; the twelve directed
v3 cases above are included, not additional cases. The final association runs
one additional cross-filesystem scratch case per profile against these exact
product binaries, comparing complete artifact inventories and final manifests.
That gives 50 distinct CLI cases per profile. Final source/contract association
also reruns all 187 contract and 112 distribution/sealer checks. The new case
is the sole CLI test addition; all Rust, Cargo and existing CLI-test source
remains identical to the full-run inventory. Documentation, mandatory-source
closure and static assertions are separately frozen for final publication.

Full source-validation report SHA-256:
`e170a44cb1ca1aa86e07510fdef7dd20ee255fa404e072d6ceaa38548954f114`.
Live terminal observer SHA-256:
`4514e6e40a24be2f1ffe9373fe97151455d53a22f0caee3f62c9a0a9b24902fe`.
Retained evidence archive SHA-256:
`b8a84a6d778e4744debf23ef48dcc26ac24d1e97d091a0fa3a01dd7ab0c2fce5`.
The final source-association report and hashes are retained in the same external
candidate run. The accepted full run retains all actual CLI traces and traced
compiler/test/contract commands; raw network traces are independently classified.
The CLI Python harness itself remains outside syscall-trace coverage. Final
allocated log accounting includes retained per-case logs as well as command
logs; the older sampler covered command logs only. Neither observation is a
measurement of allocator RSS or all filesystem activity.

These are offline synthetic source/product checks with immutable tools and exact
live invocation observations, under 2 GiB service memory, zero swap, 900-second
command and 3,600-second service ceilings. No available-prefix fallback, native
CASC-routing parity, real-world resource throughput or server activation is
established. A reproducible v9 source/binary bundle is a separate gate.

## Corresponding source and open gates

Source profile `forever-seven-crate-linux-product-collision-disk-source-v9`
mandates the v3 parser, Linux space FFI, driver, CLI, synthetic checks, contract
and this document. Seven crates, nine lock packages and two registry archives
remain; only the local Linux extractor-to-platform edge is added. Historical v7
and v8 contracts, bundles, frozen verifiers and acceptance retain their original
scope. In particular, the v8 library archive acceptance does not validate this
new product driver. No new reproducible bundle or moved-source rebuild is claimed.

Complete real `Map`, `SkillLineAbility` and `GameObjectDisplayInfo` inputs remain
unavailable under the retained accepted evidence. Real geometry and full-world
resource acceptance, RustyCore server activation/gameplay, complete Windows
product execution and reproducible distribution remain gates. The extractor is
**not complete**.
