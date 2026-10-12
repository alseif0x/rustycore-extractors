# Private disk WDT/cache shared runner

`linux-private-disk-wdt-cache-shared-run-v1` is an explicit optional Linux
runner with scoped synthetic acceptance. It consumes two fresh typed inventories and
uses the same successful GameObject startup/model collector for all complete
WDT and own Obj0ADT acquisition. The Map graph and lookup provenance remain
caller-supplied here. The separate consuming authenticated Map/liquid wrapper
is described in the world companion; product integration remains required.

All WDTs are acquired in physical preload order and immediately stored, with
source Vec storage released only after dropping it. Globals follow physical
order. Maps then follow depth/physical order and x/y tiles. Successful own
parent tiles are normalized and stored, including empty own tiles. Missing
own tiles ask the immediate parent lazily; inherited tiles never become own
caches and globals are never inherited. Absent ranges ask for sorted existing
parent keys; they do not recheck every absent cell. Lazy inactive-parent failure
is still evaluated at the first missing cell. Requested read failures are terminal.
The existing ID Session and request-window projector are reused.

The new explicit combined logical ceiling is 2 GiB, downward-only. Before
run acquisition, the complete established collector memory ceiling and both
complete inventory heap ceilings must fit together. Each inventory must also
fit the previous run's 256 MiB WDT/cache memory cap, with exact reduced parser
and cache policies. Components retain all original limits. The bridge owns
and precharges user Sink/IDs; both inventory owners live outside it and are
borrowed by callbacks, with complete ceilings separately reserved for the
whole run. No arbitrary external release or callback-allowance reinterpretation
occurs. Returned replay Vec storage has a separate full output allowance
charged to the collector before reload, held through user callback and drop.
Borrowed source bytes may be conservatively covered in both component admission
allowances. These reservations are not RSS, exact portable allocator usage or
all CPU work. Caller Map rows/graph/lookup residency remains separate here.

Every exit destroys the owned Sink before independently checking actual
LocalStore/model spool and both inventory spools. Observed change has diagnostic
priority and the typed operation/callback cause remains available. Outputs are
provisional; there is no partial completion or publication token on failure.
Protected directory identity is not authentication of unread contents. Actual
acquired-source verification remains the established collector's responsibility.

The resident and earlier spooled runners, formats, product driver and v7 source
profile remain unchanged. The 4,096 cumulative source-read ceiling, same-mount
repository boundary, full-world policies, selected VMO/map/proof staging,
publication/resume, complete real mandatory inputs, runtime activation and
Linux/Windows distribution remain gates. This step adds no native or real
client acceptance.

## Retained acceptance — 2026-10-12

The preceding exact runner source passes 26 focused and 438 complete collision
tests per Linux debug/release profile, CASC library/test Clippy, five scoped
formatting checks, 171 source/state contracts and 104 distribution/sealer
checks. All eight retained traces admit only local IPC. The explicit supplied
Map graph remains separate from the later closed-source world owner.

Run `collision-disk-run-20261012T0126Z` attempt-3 source freeze:
`7c472a6d238056e743c07e5f7d1cb4fc070a5e632427d60a571b7696ef1b9e25`.
Completed report:
`6d9b0745af27636b6b8525da37a94e19f2d0a7f5e4b9de382f82e35cde6fde0f`.
Root evidence replay:
`50c7c77338243f408460bc4686fe14ffae4357862777c8c07f6f092451913383`.
The observer retained the loaded invocation through terminal exit. Attempt 1
was deliberately stopped to replace per-absent-cell guards with sorted range
lookup. Attempt 2 passed the focused profiles but its full debug run failed
two existing fixtures because the copied new helper shared their directory
prefix. Only the new fixture prefix changed. Both source/binary histories and
all failed/interrupted logs remain preserved.

The later [authenticated world](forever-collision-disk-world.md) adds a private
exact precharge transfer and rechecks all 26 public runner cases in both
profiles. The 438-case result belongs to the preceding source, not that later
complete source. `Report`'s WDT/cache resident-byte fields describe conservative
descriptor reservations, including the repository allowance; they are not
measured allocated residency. Detailed owner peaks are in its WDT/cache stats.
This acceptance text was qualified after the frozen checks.
