# Windows explicit private membership foundation v1

Authored 2026-10-12 under GPL-3.0-or-later. This is a separate opt-in capability
on a live `windows::output::PrivateRoot`. At the authored source checkpoint, native Windows/NTFS execution acceptance
for this increment was pending; later executed acceptance is recorded below. Source/contract checks and planned CI commands
are not execution evidence.

The earlier ordinary private-output acceptance belongs to functional commit
`a220596da8a29f9f3a867dc7f2bd016d01cd3558`; its later documentation checkpoint and
fourteen native cases retain their scope. It did not enumerate directory
membership. This increment does not relabel that evidence or the Linux v9 bundle.

## Explicit API and observed membership

The standalone `output::membership::verify_membership(&PrivateRoot, Limits)`
returns `Result<Summary, Error>`. The root and its parent lease must remain alive.
`Limits` has `max_directories`, `max_records` and `max_query_calls`; `Summary`
reports `directories`, `records`, `children` and `query_calls`. The separate
profile is `forever-windows-private-membership-v1`. The check compares the complete observed direct
children of every owned directory with the owner's current entries, including
their current names after file rename. Retained and named identity, attributes,
owner/DACL, case-sensitivity and parent guards remain required. Complete
transient snapshots of all owned directories bracket the scan as additional
observations; they do not turn it into a snapshot.

Unknown entries are errors; this is not adoption, cleanup or recursive traversal
through an unknown directory. Missing, duplicate, wrong-name, wrong-type or
wrong-identity observations cannot become successful partial verification.
An observed scan/guard failure permanently poisons the owner, including after
an external actor repairs the directory.

The existing `verify_unchanged` remains the original identity/security check;
it does not silently become a membership scan. Ordinary append/flush/rename do
not automatically provide this separate membership result. The caller must
choose when to invoke it and how to use that result in a future protocol.

## Enumeration boundary

The native implementation uses synchronous `NtQueryDirectoryFile`,
`FileIdFullDirectoryInformation` (class 38), a fixed 640-byte buffer aligned to
eight bytes, single-entry
queries and explicit completion at EOF. The record reads `FileId` at offset 72
and the variable name at offset 80; the buffer admits complete 255-UTF16-unit
names without reallocating; the accepted child-name policy remains lowercase
ASCII and at most 64 bytes. The parser accepts exact record extent or opaque
alignment padding of at most seven bytes and requires complete framing;
it does not accept truncated or partial names, a zero-byte successful record or
an unexpected status as successful exhaustion. Directory names are compared
with the bounded output-name policy; observed IDs are checked against the held
objects rather than trusting the enumeration order or NTFS `FileIndex`.
Enumeration checks FileId, file/directory type and reparse rejection; the existing
held/named guards separately check admitted attributes and security. Dot entries
are optional; when supplied they require unique occurrence, directory type and
self/parent identity. They count as records and are excluded from child totals.

This is complete **observed** membership for a bounded pass, not a filesystem
snapshot. Microsoft documents that directory changes during multiple query
calls may or may not appear in enumeration, depending on timing and filesystem
behavior. Pre/post guards cannot prove that a transient insertion/removal never
occurred. The result does not stop a same-SID process or administrator from
changing the directory after the pass.

## Limits and failure scope

Separate downward-only membership ceilings bound 128 directories, 512 admitted /
decoded records (including dot entries) and 640 native query calls across the
whole pass. At the record cap a query lookahead, charged to the global query
budget, can establish EOF. If it returns data, the pass rejects `Budget` without
decoding that extra record and permanently poisons the owner. Thus a failed pass
may receive one extra native record; a successful pass never exceeds the
admitted/decoded-record cap. They do not raise
the original private-output object/depth/name/file/write limits. The fixed query
buffer does not grow after overflow, malformed data or failure. Counters are
logical admission/work bounds, not process RSS, all CPU work or physical storage
reservation. The 640-query cap counts enumeration calls; existing root-guard
I/O is separate. Zero or above-cap `Limits` reject before filesystem I/O without
poisoning the root. Repeated caller-initiated verification requires caller budgeting.

Directory write sharing and ACL limitations retain the prior output contract.
There is no directory write exclusion, same-user isolation, atomic namespace,
crash durability or content authentication. Membership observations do not
establish package completeness, receipts/sealing, restart recovery or a Windows
product driver. The Linux hardlink/recovery protocol is not migrated.

## Validation gate

The new membership contract and static checks describe source consistency only.
Native tests require an actual Windows x86_64 process and a fixed local NTFS
temporary directory. Wine and GNU-Windows compilation cannot close that gate.
The planned Windows workflow adds separate focused membership debug/release
steps, comparing an exact 24-case contract inventory with `--list` and requiring
the same passing count with zero failed/ignored tests. The inventory comprises
13 real temporary-NTFS fixture cases and 11 pure decoder/matcher/status/limit
cases; listing those sources is not execution evidence. The original fourteen
private-output tests and their focused steps are preserved.

Each focused step records OS version and fixed-NTFS fixture admission without
SID or fixture path output. Locked/offline Cargo describes dependency resolution;
it is not a claim of global runner/harness network exclusion. No native
membership run or new reproducible bundle is accepted at this authored checkpoint.
Full real inputs/geometry, RustyCore server/runtime, Windows package publication /
recovery and complete extraction remain open.

## Primary API references

- [NtQueryDirectoryFile](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntquerydirectoryfile): handle-based enumeration, single-entry/restart behavior, completion and concurrent-change limits.
- [FILE_ID_FULL_DIR_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_id_full_dir_information): record fields, variable name storage, 8-byte alignment, `FileId` and the distinct `FileIndex` field.

## Executed ordinary membership acceptance — 2026-10-12

Functional commit `433d5ce02c5fa541ca9340a20de21889c10216bd`, tree
`7fee5b9f1abbfdc24c3832034ca0c1691b45f154`, passes
[synthetic CI run 38261424386](https://github.com/alseif0x/rustycore-extractors/actions/runs/38261424386).
Each actual job checked out `f4b8a1006d6317d87fd14235439008866840701c`;
its tree equals the functional tree. The Windows Server 2022 job
(`windows-2022`, reported OS `Microsoft Windows NT 10.0.20348.0`) admitted
fixed local NTFS and listed then passed all 24 exact membership cases separately
in debug and release, with zero failed or ignored cases. Thirteen cases use
real temporary NTFS fixtures; eleven exercise pure decoding, matching, status
and limits. The unchanged fourteen private-output cases also pass separately
in both profiles. The privileged reparse case remains ignored and unaccepted.

The full Windows workspace reports 2,175 passed, zero failed and 21 ignored
across 41 summaries. Linux debug and release each report 2,643 passed, zero
failed and 26 ignored across 41 summaries. The separate isolated remote
`compile3` validation passes Linux/GNU-Windows all-target Clippy, scoped format,
both Linux workspace profiles, 219 contracts, 112 distribution cases and both
GNU-Windows compile-only profiles. Its exact terminal invocation, unchanged
source/tools/binaries and independently classified network traces are retained;
GNU-Windows compilation does not supply the native execution evidence.

Retained review: `native-ci1-root-review.json`, SHA-256
`d3ff10adcfa4ec9254e1a54c15e863369c4431906cb2fe1cdad3f49d994231d2`. Source freeze:
`b192bebc605ef7daa039e86cd4bb4df515d02ec19fc49f8b34ab8004acb02713`.
Remote review: `compile3-root-review.json`, SHA-256
`feab62b4ea4537d54d84f2550685ab30bd421843edf52fe490f5aafbd38f3985`.
Reports reside under the external `windows-membership-20261012T133305Z`
acceptance directory. Compile1's permission preflight and compile2's two
Clippy style failures remain failed attempts; compile3 reran all nine commands
after the reviewed style correction. No native membership attempt failed.

This later acceptance does not rewrite the authored contract's pending/native
false flags. It accepts bounded complete observed membership, not an atomic
snapshot or same-SID isolation. Windows adoption, removal, locks, package
publication, restart recovery, product enablement and reproducible Windows
distribution remain open, alongside full real inputs/geometry and RustyCore
runtime acceptance. No new binary bundle or global hosted-runner network
exclusion is claimed. Documentation-only follow-up validation does not rebuild
the functional binaries or establish execution of its later document commit.
