# Windows VMap source capability v1

Authored 2026-10-11. Original bounded read adapter; the Windows product remains
disabled. Source profile `forever-seven-crate-windows-vmap-diagnostics-source-v5`
ships this capability in the existing Linux distribution envelope v1.

`forever_vmap::store::windows::{RootedSource, FsLimits, FsStats}` implements the
existing static-package Source contract. Each operation acquires fresh readonly
platform leases, retains admitted ancestors while checking, then drops all
handles. Root/map directories and final files retain full identities; legitimate
ancestor churn uses the existing object-only ancestor checks. Exact artifact
spelling, bounded enumeration, normal absolute drive paths and the platform's
fixed NTFS/no-reparse policy apply. No aliases are normalized into an admitted
name. Every successful complete read hashes all supplied bytes. Observed input
change permanently poisons the source. Explicit verification observes metadata;
it is neither an atomic snapshot nor authentication. Linux RootedSource is
unchanged.

The added safe `ReadFile::verify_eof()` performs a one-byte probe at captured EOF,
with checks before and after, without returning payload bytes. Observed read or
extent failure permanently invalidates the lease. A real open failure takes
precedence over a later verification-budget admission error.

## Downward limits

| Policy | Ceiling |
| --- | ---: |
| Chunk | 2 MiB |
| Complete file | 128 MiB |
| Source metadata | 128 MiB |
| Root bytes | 4,096 |
| One lease/scratch allowance | 256 KiB |
| Peak OS handles | 65 |
| Handles retained between operations | 0 |
| Snapshot entries / entries per directory scan | 65,536 |
| Verification work per operation | 262,144 |

Each platform call charges `32 * (path_components + 2)` work units; bounded
enumeration consumes additional units. Capacity/growth overlap and chunk scratch
are charged. These are logical admission bounds, not allocator/kernel/RSS or
all-CPU measurements. `read_bytes` counts successfully returned chunks, including
chunks preceding a later error; failed kernel partial reads and EOF probe bytes
are not reported in that counter.

## Preliminary evidence and remaining gates

`windows-vmap-source-20261011T184046Z/attempt-2/closeout.json` is accepted and
terminal (SHA256 `a61eb502cd46d22c51277aae5f8e05d1dd75d72c50345d921d51d2f262ba80cc`).
Runtime report SHA256 is
`2af82d155107c24af577a36638528eb400207b4847a37137c6e8d554c0147b6a`.
Linux passes 477 VMap tests per profile. Linux and Windows GNU VMap/platform
all-target Clippy pass. Each Wine profile passes 28 focused cases: 24 VMap,
three safe EOF and one existing read_all case. Attempt 1 remains historical;
corrected attempt 2 accepts the actual open-failure precedence regression.

This is synthetic Wine evidence. Actual Windows/NTFS, real junction/reparse/ACL
behavior, private output and durability, product activation, real-client VMap
acquisition and RustyCore runtime acceptance remain separate gates. Filtered
traces describe observed calls; denied attempts may be absent and individual
exit of every descendant is not established. Static source-contract checks are
consistency checks, without runtime proof. No v5 package has been built by this
source composition, and no publication is authorized.
