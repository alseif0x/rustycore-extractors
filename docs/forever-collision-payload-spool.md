<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Private collision payload spool, first increment

The explicit Linux API `collision::payload::SpoolingExtractor` uses profile
`linux-private-collision-payload-spool-v1`. It is separate from the existing
resident `Extractor`; its original public Error enum remains exact, and the
new wrapper exposes its own `CollectorError::{Collision, Payload}`. The existing
product driver and owned collector remain resident and retain their existing cumulative ceilings. No product plan,
manifest, recovery protocol or Windows backend is enabled by this increment.

## Storage and acquisition

The new wrapper uses the same acquisition/conversion engine, stable insertion
handles, immutable source class/name/origin binding, GameObject magic and shared
lookup transition. At EACH completed model boundary, including recursive doodad
children, it streams RAW and all large WMO metadata to a private ordinal record:
complete root bytes, bindings, selected references and group source facts. Set
ranges are read from that same complete root on reload. The vectors are dropped
immediately and their residency charge released. Source facts and compact
name/origin/handle descriptors remain resident. There is no collect-all/drain
phase and no hidden global RAW/WMO cache.

A WMO conversion still admits its complete root, required group buffers and
conversion scratch for that operation. Multiple active dependencies can overlap
that root; the API does not promise one buffer or exact RSS. One subsequent
`with_payload` callback admits a complete record as a single bounded buffer;
metadata accessors are borrowed views/iterators without another owned metadata
copy. Any returned owned callback data belongs to the caller and is outside the
collector's budget. A callback may not retain borrowed payload views.

`SpoolingExtraction` keeps only compact collector descriptors and repository
facts after final store verification. It continues checking the spool on reload;
it does not retain or recheck the former LocalStore after finish. That store's
last resident-memory reservation remains conservatively charged on reload.

## Explicit ownership and filesystem policy

`spool::ProtectedInputs::capture(source_root, key_files)` is an explicit caller
ownership assertion: source_root must be the root of the supplied LocalStore,
and every supplied key-file path must be declared. The API cannot infer those
paths from LocalStore's private state. It reads metadata, never key-file bytes.
It retains held/named ancestor/leaf identities, rejects symlink traversal and
checks source-root containment and directory-object aliases in the destination
ancestor chain. Existing ordinal files cannot be overwritten; file hardlinks
are rejected on record verification. Protection does not depend solely on a
canonical pathname. This Linux profile requires procfs and rejects source/destination
mount differences. Each captured descriptor retains its `mnt_id`; boundary
checks reread held and freshly opened nofollow named descriptors so replacing a
path with a bind mount of the same device/inode is also rejected. `/proc/self/fdinfo`
reads are capped at 256 bytes and require complete ASCII records with exactly one
nonzero decimal mount ID. Missing, duplicate, malformed, overflowing or truncated
records reject. Cross-mount operation is unsupported. The explicit input ownership
assertion and stable same-mount path checks remain necessary; this is not a universal
alias inventory or an atomic snapshot. No key discovery or network source exists.

`Spool::create` requires an existing private mode-0700 parent and creates a NEW
mode-0700 namespace. Records have internal ordinal names, are created exclusively
at mode0600 and become mode0400 after complete write. Retained root/ancestor
handles anchor I/O. Completed writes are reread with fixed 64KiB stack scratch to check actual
on-disk SHA-256 and EOF before accepting a record descriptor. Every reload checks
held/named full file identity, complete SHA-256 and exact EOF; checks retain the
descriptor through allocation refusal and callback failure. Partial writer errors
also check the created held/named object, while allowing the writer's own append
and permission transition.
Namespace/protected-input checks and final LocalStore verification also run on
errors. An observed error poisons the new collector; no fallback or earlier
payload prefix is exposed afterward. Metadata verification is not an atomic
snapshot, exclusion of a same-user writer or publisher authentication.

The namespace is private intermediate state, NOT a product artifact inventory.
There is no implicit Drop cleanup, pathname unlink, overwrite, process resume,
or publication. Created partial files remain after failure for explicit caller
handling. File fsync does not establish publication or directory durability.
The caller owns the namespace lifecycle after success/failure.

## Separate downward ceilings

All old collector limits remain downward-only and cumulative: source reads,
requests (including cache hits), dependency requests, acquired bytes and RAW
bytes are never refunded by eviction. The new spool independently limits:

- 4096 records and 512MiB cumulative disk payload bytes;
- 256MiB active conversion/reload capacity and 16MiB repository plus compact
  collector descriptor capacities;
- 16384 reload attempts and 1GiB cumulative attempted reload bytes.

Every ceiling may only be reduced. Repository/compact capacities and current
payload/scratch additionally share the existing collector global memory ceiling
with LocalStore residency. Descriptor slots are reserved before source reads, including GameObject's
unknown-class source. The new route conservatively requires an available model
slot before it can discover a texture; on texture omission the model is removed
but slot capacities stay reserved, and read/request/source counters stay charged.
Every growing repository/collector vector admits its FULL new capacity while the
old allocation remains live, then checks the actual returned capacity. This
transient allocation overlap is charged against global and relevant descriptor
or active budgets. The old resident allocation policy remains unchanged. Rooted path/guard
state is included; stack, allocator bookkeeping, caller-owned lookup/callback
results and all process RSS are not. Statistics distinguish observed payload
capacities at persistence checkpoints from the configured active ceiling; they
do not claim converter scratch peak or exact RSS. Metadata guard loops are
bounded by admitted descriptor/request/reload counts, not a measured CPU budget.

## Remaining full-world gates

This first increment does not connect GameObject startup, placement/world stage,
assembly or the product driver to the new API. The old path still retains all
WDT/parent caches/dir_bin and completed map/VMO outputs. Subsequent work must
externalize those states and parent-association proof payloads, preserve one
serial ID session and first-present effective-parent matching, and emit final
outputs provisionally to the package sink. Only complete selection, association
validation and final source guards may seal the component. New work-unit recovery
or changed cumulative policies require separate versioned protocols/profiles.

Validation is synthetic and offline only. No client bytes, actual supplied key,
native extraction, runtime geometry, Windows spool or full-world acceptance is
claimed here.
