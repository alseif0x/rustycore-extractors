# Required-input complete-span framing observations v1

Authored 2026-10-11. Profile `forever-local-required-framing-v1`.
This is an isolated library increment. There is no product CLI enablement,
key lookup, decoding, payload publication, repair or download operation.
Real-client and Windows runtime validation of this adapter are separate gates.

## Closed API

`LocalStore::diagnose_required_framing(framing::Limits)` returns exactly three
redacted `framing::Diagnostic` values, in Map / SkillLineAbility /
GameObjectDisplayInfo order (FDIDs 1349477 / 1266278 / 1266277).
The caller must already own an admitted key-free LocalStore. A store with a
keyring rejects before selection; the operation does not inspect that keyring.
Opening LocalStore separately reads authenticated config/index/ENCODING/ROOT
metadata under its existing budgets. This new API reads requested archive spans.

Selection preserves the existing metadata routing contract: count all matching
ROOT rows, apply the admitted locale/content flags, choose the first eligible
central-present CKey, then enumerate its full EKeys and all matching nine-byte
index-prefix collisions in their existing order. Archive availability never
changes the selected ROOT row. All candidates are reported independently; a
rejected candidate does not promote a later encoding to accepted content.

Each candidate contains only safe ordinals, record/archive byte sizes, the
existing `SpanState`, and `Inspection`:

- `NotRead`: missing index/archive, unsafe/unsupported archive, invalid/incomplete
  span, or unknown expected decoded size. These outcomes remain distinct through
  route/span fields. Unknown size permits metadata observation only.
- `Rejected(static_code)`: a complete record was read but its framing inspection
  rejected. No evidence from a successful prefix or earlier frames escapes.
- `Complete(Evidence)`: the entire supplied BLTE object passed the pure inspector.
  Only counts, declared sizes and `FramedMd5` / `HeaderlessUnhashed` are exposed.

Diagnostics/candidates have no Debug or raw-route accessor. Hashes, CKeys/EKeys,
archive index/offset/path, key identifiers, nonces and payload bytes are absent.
An observed read/source failure rejects the whole report, with no returned
prefix. No result asserts ciphertext interior, decoded CKey or plaintext validity.

## Admission and memory

Limits can only reduce the following closed ceilings:

| Limit | Maximum |
| --- | ---: |
| ROOT matching rows per input | 64 (visit at most 65 before rejecting) |
| Candidates per input / total | 64 / 192 |
| Complete record including prelude | 128 KiB |
| Sum of indexed record sizes | 256 KiB |
| Expected declared decoded size per input | 512 KiB |
| Frames per candidate BLTE object | 4096 |
| Returned report capacity | 128 KiB |

Candidate and ROOT caps also preserve the underlying store's downward limits.
Every index collision and each EKey with no index match counts. The conservative
byte sum includes all indexed candidates, even unknown-size, unavailable,
unsupported or incomplete candidates that will not be read. It counts repeated
references separately. There is no positive-prefix fallback when any cap fails.
All plans, byte totals and output capacities pass admission before any requested
payload read. Frame count is checked separately for each candidate BLTE object after the
complete bounded record has been read; exceeding it returns a static candidate
rejection. The 4096-frame ceiling is not an aggregate per-input limit. All
alternatives remain inspected; the 256 KiB conservative total record-byte cap
and 64/192 candidate caps bound aggregate work.

Private choice vectors and output candidate capacities are charged to existing
resident/combined budgets, including observed allocator excess. Report capacity
also includes the fixed three diagnostic structures. Path scratch reserves twice
the store path bound; Windows additionally reserves the existing 256 KiB logical
single-lease allowance. A record requests exact capacity fallibly and charges
observed capacity before resize/read. Record storage overlaps retained plans and
report capacities in the combined budget. One record is retained at a time.
No descriptor table, decoded payload or encryption scratch is allocated.
Caller serialization and retained prior results remain caller-owned. Fixed stack,
allocator bookkeeping and standard-library I/O internals follow the existing
logical-admission policy; these counters are not RSS or all CPU work.

## Exact-span source chain

Existing store guards, including exact admitted `.build.info` bytes, run before
and after the operation, even on failure. Every archive path uses the same
bounded decimal constructor as acquisition (`data.010`, `data.016`,
`data.4095`). Checked extent and byte limits precede allocation/read.

Linux opens a regular source using `O_NOFOLLOW | O_NONBLOCK`, compares named and
held identity, seeks to the selected offset, and reads exactly the planned record
under that held file. Named and held identity are checked afterward on successful
inspection, rejected framing, read failure and budget/allocation failure. Windows
uses the existing public platform readonly NTFS file/ancestor lease, bounded
read_range, and an explicit final lease verification on every result. Archive
leases drop before serial store guard checks; no handles persist in a report.
Unsupported hosts reject rather than use a weaker adapter.

The record may be inside a larger archive: this is exact **range** reading, not
an assertion that the record ends at archive EOF. An absent/incomplete record
never produces a partial buffer or manufactured zeros. Guard changes take
precedence over earlier candidate/error results and invalidate the entire report.
Linux parent traversal and metadata identities retain existing LocalStore policy:
this is not an anchored ancestor lease, atomic snapshot, full archive rehash or
proof against a privileged concurrent writer. Metadata observations across
archives are not mutually atomic. This increment adds no stronger guarantee.

Exactly 30 bytes of local prelude are removed. Its EKey, size and checksums remain
ignored, as in existing `archive_record`. Only the index's nine-byte EKey prefix
is checked. **The full EKey is not authenticated.** The entire remaining BLTE
slice must have exact framing/table extents, expected declared size and EOF.
Every framed MD5 is checked, including zero expected hashes and frames after an
outer E frame. Headerless input explicitly has no frame hash. N lengths are
checked; Z contents remain opaque; E validates only bounded outer name/nonce/
cipher/ciphertext extents, with no decryption. F/unknown modes reject explicitly.
Neither frame MD5 declarations nor prefix matches authenticate a publisher,
complete ciphertext interiors, encrypted inner modes, decoded CKey/plaintext,
a sufficient key, or a remote repair source.

## Validation and next gate

Tests use authored complete toy CASC inventories only. They cover encrypted-first
objects with later checksum/truncation failures, zero MD5, headerless evidence,
opaque Z/outer E, all candidate ordering and collision accounting, unavailable
and unknown-size routes, downward caps, report/resource boundaries and guard
error priority. The retained external source handoff supplies exact hashes and
Linux debug/release/Clippy evidence. No real-client or Wine execution is part of
this adapter's initial validation. A separate root review and bounded call must
precede any real observations. This does not complete the extractor.
