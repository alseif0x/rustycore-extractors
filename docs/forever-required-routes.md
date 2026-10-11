# Required-input offline route diagnostic v1

Authored 2026-10-11. Isolated synthetic implementation; no current real-input
acceptance, production CLI enablement, repair/download plan or key-only claim.

## API and closed scope

`LocalStore::diagnose_required_routes(routes::Limits)` returns exactly three
`routes::Diagnostic` records, in this order:

| Input | FDID |
| --- | --- |
| Map | 1349477 |
| SkillLineAbility | 1266278 |
| GameObjectDisplayInfo | 1266277 |

Profile: `forever-local-required-routes-v1`. There is no arbitrary-FDID route
argument. A key-bearing store rejects with
`route_diagnostic_requires_key_free_store`; this operation never accesses a
keyring or decrypts. Metadata already admitted by `LocalStore::open` is reused.
Opening a store may read its admitted config/index/ENCODING/ROOT; this diagnostic
does not reopen or acquire those objects. Existing guards still recheck metadata
and the exact admitted build-info bytes before and after every operation,
including failures. No CLI, native backend or acquisition default changes.

## Evidence and omissions

- ROOT matching rows and eligible rows are counted separately. `RootAbsent`
  means no row for the FDID; `RootIneligible` means rows exist but none pass the
  admitted locale/content flags. The shared ROOT resolver preserves audio and
  source ordinal order. The first eligible central-present CKey is selected;
  missing archive bytes never select a later ROOT row.
- `CentralAbsent`, `UnknownDecodedSize`, `EncodingAbsent` and `Routed` describe
  admitted metadata, not successful content acquisition. A known decoded size
  is the central metadata value, even when it exceeds the caller's decoded-file
  acquisition budget. Unknown size remains unknown; no extent guesses fill it.
- Candidates preserve full-EKey alternative order internally and every matching
  index collision. Only safe ordinals, encoded-record size, observed archive
  length and span classification are projected. No CKey/EKey, raw path/offset,
  key ID, content checksum or payload is exposed, including through `Debug`.
  There is no private raw-identity accessor in this version.
- `IndexAbsent` is distinct from `ArchiveAbsent`, `ArchiveRejected`,
  `UnsupportedArchive`, `InvalidExtent` and `IncompleteSpan`. `ArchiveRejected`
  means an archive cannot be admitted as a regular safe readonly source; it does
  not imply absence. Other OS I/O failures remain errors rather than invented
  route diagnoses. Unsupported hosts reject without a filesystem fallback.
- `FitsObservedLength` means only checked index offset plus encoded-record size
  fits the held regular file's observed length. It does not read that range,
  validate the 30-byte prelude, BLTE framing or frame hashes, authenticate the
  complete EKey, or establish complete ciphertext/plaintext. Payload and
  ciphertext are always `NotInspected`, including E-marked synthetic payloads.
- Historical `encrypted=1` decoder failures are external historical evidence,
  never injected into a current diagnostic. The decoder may stop at the first
  E frame before validating later frames. These reports cannot prove that an
  operator-supplied key alone is sufficient or that bytes must be downloaded.

## Bounds and source observations

Limits may only reduce 64 matching ROOT rows per input, 64 candidate attempts
per input and 192 total candidates. The per-input candidate cap also cannot
exceed the store's acquisition-attempt cap. Every index-prefix collision and
every EKey lacking an index match counts. At most 65 ROOT rows are visited per
input before rejection; no full large duplicate range is counted. Exceeding a
cap returns no partial report. All returned vector capacities, resident store
bytes and bounded path/lease scratch are charged to existing logical budgets.
These are admission allowances, not exact allocator/RSS or all-CPU bounds.
The existing archive-span budget applies; no archive-sized allocation occurs.

Acquisition and diagnostics share one bounded decimal archive-path constructor:
index 10 is `data.010`, 16 is `data.016`, and 4095 is `data.4095`.
The fixed basename uses stack storage; the exact joined path is checked before
fallible path allocation, with its observed capacity also bounded. Separators
are inserted once, including an already separator-terminated directory.

One archive at a time is opened readonly, observed, verified and dropped.
Windows retains the existing NTFS leaf/ancestor lease and rejects unsupported
path/reparse/sharing conditions; metadata source guards are reacquired serially.
Linux uses a held regular file, `O_NOFOLLOW | O_NONBLOCK`, named/held identity
comparison and final metadata verification. Leaf symlinks and directories reject;
a replaced FIFO cannot block the open. Linux parent traversal retains existing
LocalStore path/guard policy; this is not an anchored ancestor lease, atomic
snapshot, full archive rehash or proof against a privileged concurrent writer.
Archive metadata observations occur at different times and are not mutually
atomic. Any observed guard change rejects the whole operation, including when
an earlier diagnostic step had already failed.

## Proposed QA call after separate authorization

1. Record source/tool hashes and a new private report destination. Use explicit
   admitted locale and index/ROOT layouts; unset all live test selectors.
2. Open the explicitly supplied local client using **key-free** `LocalStore::open`.
   If admission fails, retain its static error and stop. Do not bootstrap/recover
   metadata, discover keys, probe alternative clients or contact a network.
3. Call `diagnose_required_routes(routes::Limits::default())`; serialize only its
   redacted public scalar fields/states. Recheck `verify_unchanged` before closing.
   Retain actual timestamp/source hashes separately from historical evidence.
4. Review whether the gate is absent authenticated routing, unknown expected
   size, absent index/span, unsafe archive or a fitting but uninspected span.
   No result authorizes a key lookup or request.

No such real-input call has been executed for this implementation.

## Separate next framing step (not implemented here)

The independent proposed pure API is
`blte::framing::inspect(encoded, expected_content_size, blte::Limits)` returning
only copied framing counts, declared sizes and outer-frame evidence. A future
explicitly authorized QA adapter must privately retain the exact selected route,
validate/read its complete local span under one source lease and existing byte/
allocation limits, remove exactly the known 30-byte local prelude, call that
inspector on the entire BLTE slice, then perform final lease and store checks on
success **and error**. This route API exposes no raw route for that purpose yet.
The inspector can check all framed MD5s and admitted outer E headers without
keys; headerless objects have no frame MD5. Neither outer framing/E checks nor
an EKey-prefix match authenticates plaintext, decrypted inner Z/E structure,
complete CKey, an external publisher or a remote repair source.
