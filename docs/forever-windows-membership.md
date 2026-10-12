# Windows explicit private membership foundation v1

Authored 2026-10-12 under GPL-3.0-or-later. This is a separate opt-in capability
on a live `windows::output::PrivateRoot`. Native Windows/NTFS execution acceptance
for this increment is pending. Source/contract checks and planned CI commands
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
