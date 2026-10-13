# Forever platform foundation

Original, dependency-free platform helpers authored on 2026-10-11 under
GPL-3.0-or-later. No upstream implementation is imported. This crate alone does
not enable Windows extraction, package publication, or full Windows support.

## Admitted Windows read profile

The versioned `WINDOWS_READ_PROFILE` is `forever-windows-read-ntfs-v1`.
The Windows module targets x86_64. It accepts a Unicode absolute drive-letter
path with backslashes, on a fixed local NTFS volume. UNC, verbatim/device paths,
relative paths, streams, malformed components, trailing dots/spaces, reserved
DOS stems (including superscripts and CONIN$/CONOUT$), and every observed
reparse component are rejected. Cloud-placeholder reparses and mounted/junction
components are outside the profile. Non-NTFS volumes, network/removable drives
and failed filesystem identity queries fail without fallback. Names are not
case folded, normalized or guessed; a Windows alias that the OS resolves is
still checked by its held file identity. Windows long-path process policy must
permit the supplied path; no verbatim-prefix fallback is used.

`windows::ReadFile::open(&Path, path::Limits, windows::LinkPolicy)` retains the
volume root, each intermediate directory and the final ordinary file. Directory
opens request FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES so directory handles
participate in read-sharing checks (metadata-only handles do not). This requires
list-directory permission even though the implementation performs no listing.
Final opens request read access. Every open
uses FILE_SHARE_READ (no write or delete sharing), OPEN_REPARSE_POINT and
BACKUP_SEMANTICS. Each component is opened only after its parent is retained.
GetFileType must identify a disk object. GetFileInformationByHandle provides
volume serial, 64-bit NTFS file index, link count, attributes, size, creation and
write time. FileBasicInfo additionally supplies ChangeTime; repeated basic
fields are compared across the three snapshot queries. Last access is excluded.
Every verification checks retained and named handles. Directories compare only
volume/index/attributes, permitting unrelated sibling changes. The final file
compares every captured field. An observed mismatch or verification failure
permanently invalidates the lease. LinkPolicy::Reject requires one final-file
link; Allow explicitly admits aliases but still fixes link count in identity.
Opening fails if an incompatible writable handle is already present.

This is a conservative local filesystem lease, not content authentication or
an atomic snapshot against privileged filesystem modification, preexisting
writable mappings, kernel attackers or DOS drive remapping. Directory-entry
changes cannot normally replace retained components because delete sharing is
denied; identity verification additionally checks exact named paths. Callers
must authenticate content where required and discard buffers after any error.
The read profile implements no network requests, key discovery, hidden cache,
output writes, DACL creation or publication protocol.

## API and explicit limits

- `identity()` returns non-path identity metadata.
- `verify_unchanged()` checks held and named identities and permanently poisons
  the lease after an observed check failure.
- `read_all(max_bytes)` returns the complete bounded file with EOF and identity
  checks. Range reads permit larger archive files without whole-file allocation.
- `read_exact_at(offset, &mut [u8])` validates the range, then checks identities
  before and after positional reads. On failure discard all caller buffer bytes.
- `retained_handles()` reports the exact retained count, with no descriptor cache.

Limits only reduce 63 ancestors including the volume root, 32,760 UTF16 path
units, and 64 MiB per read operation. A lease retains at most 64 handles and
verification temporarily opens one additional handle: at most 65 simultaneous
handles. Multiple leases multiply this explicit bound; the caller owns the
aggregate handle/memory budget. No RSS or aggregate CPU bound is claimed.
Each component is limited to 255 UTF16 units. Allocation uses fallible reserves.
Errors are static codes without paths or OS strings; leases and paths have no
Debug implementation.

## Safety and validation

Unsafe code is denied for the crate and permitted only inside the original
minimal Win32 filesystem FFI module, the scoped producer-seal and private-output
FFI modules, and the GNU/Linux x86_64 file-lease FFI module. The read/producer
helpers borrow live std File handles and supply exact
repr(C), initialized, correctly sized output buffers. Structure size/alignment
assertions are compiled for Windows. Read/producer calls are synchronous and do
not retain or close their borrowed handles. The private-output FFI owns newly
created handles through RAII and supplies only safe owning wrappers to its
parent module; no raw handle is part of the public API.

Host-independent path tests run on Linux. Windows tests cover ordinary reads,
identity, ranges/limits, explicit hardlink policies, existing writer rejection,
write/delete/rename sharing denials, ancestor renames, sibling changes and
permanent invalidation. The invalidation test deliberately corrupts an expected
snapshot; it is not an external mutation bypass test. The actual file/directory
symlink test is explicitly ignored by default because it requires real reparse
support and Windows symlink privilege; it must be run separately before claiming
that acceptance gate. Wine can give preliminary evidence but cannot establish
actual Windows/NTFS durability, privilege, junction or cloud-reparse acceptance.

## Directory leases and operator paths

`ReadDirectory::open(&Path, Limits)` retains the same complete chain and applies
fixed-drive/NTFS, reparse, directory type, read-sharing and handle-limit checks.
The final directory gets the same `0x81` access as its ancestors. Volume-root-only
paths remain rejected. `identity()` permits a transient caller to capture a
snapshot and immediately drop the lease; no descriptor cache is created.
`verify_unchanged()` compares the complete final-directory snapshot, including
size, links and timestamps. `verify_same_object()` explicitly compares only
volume/index/attributes on that final directory, permitting sibling entry
changes. Both methods verify every ancestor and permanently poison the lease
after an observed failure. A failed full check cannot later be relaxed to object
policy. `Identity::same_object` exposes the same explicit metadata comparator for
transient caller-owned snapshots; it neither opens a path nor authenticates data.
Directory leases perform no listing, descendant opening or snapshot refresh.

Callers doing `read_dir` must budget the enumeration handle separately, drop the
iterator before verification, and drop the directory lease before opening a
complete independent file lease if they require the same 65-handle aggregate
ceiling. Multiple independently retained leases multiply the per-lease budget.

`operator_path(&Path, Limits)` validates and owns the exact supplied normal
absolute drive path without filesystem I/O. It rejects relative, UNC, device and
verbatim inputs; it never strips a canonical `\\?\` prefix or follows a reparse.
This helper does not establish existence or ownership. Opening an absent path
returns static `platform_missing` only for the OS NotFound category; sharing,
permission and other open failures remain `platform_open`. Verification always
maps a failed observation to the permanent `platform_changed` state.


## Loaded main-executable candidate acquisition

`windows::producer::WINDOWS_PRODUCER_PROFILE` is
`forever-windows-producer-seal-v1`. `LoadedProducer::capture(Limits)` reads an
original live-referenced, initialized 64-byte `.rceid` static through a scoped
volatile read. Its first 32 bytes are `RCE-FOREVER-PE-SEAL-V1` plus ten NULs;
its final 32 bytes are the offline-patched digest. A zero digest or wrong marker
fails before pathname acquisition or filesystem I/O. The initializer is always
zero: a strict post-link sealer is required, with no unsealed fallback.
`#[used]` and the live read preserve the symbol in the tested GNU link; callers
must validate their final link output, including readonly initialized section
attributes. Other linkers are not accepted by this source assertion alone.

Acquisition obtains the main module with GetModuleHandleW(NULL), then requires
GetModuleHandleExW(FROM_ADDRESS | UNCHANGED_REFCOUNT) at the actual static
address to identify that same module. No reference count changes or handle
ownership escape. Checked pointer subtraction reports the actual payload START
RVA as u32. A bounded GetModuleFileNameW loop preserves exactly the returned
UTF16 spelling, rejecting ill-formed Unicode and every unsupported path format
through the existing normal absolute-drive admission. No current_exe fallback,
canonicalization, prefix stripping, alternate spelling or module search occurs.
The resulting ReadFile retains all ancestors and the final candidate using the
existing fixed-drive NTFS profile. LinkPolicy::Allow explicitly permits extra
hardlink names; the captured final identity still fixes link count. Aliases do
not establish content identity.

`payload()` returns the captured 64-byte loaded payload; `payload_rva()` returns
its actual start, not the digest offset. `read_all(max_bytes)` captures complete
file length under the retained lease, reserves fallibly, reads chunks, and
performs a final loaded-payload/module/RVA and filesystem check. Total bytes
only reduce 256 MiB; each chunk only reduces the existing 64 MiB per-operation
limit, and at most 4,096 chunks are admitted before allocation. This additional
work ceiling can reject a very small per-operation override even when total
bytes fit. Caller-owned returned allocation and simultaneous consumers remain
separate aggregate budgets. No extra copy of the complete candidate is retained
by LoadedProducer. Its settled handle count is the existing ancestor-plus-leaf
bound, not an additional cache.

The caller **must** parse the complete candidate PE strictly, require a unique
initialized readonly `.rceid` section with matching measured payload RVA and
loaded payload, reject loader writes/unsupported layouts, and compare SHA256 of
the complete file with only its digest slot zeroed to the loaded digest. Only
then may it use the full-file SHA256 as producer identity. This platform API
does not implement PE parsing, sealing or hashing, and cannot independently
claim that the pathname candidate is the loaded image. Runtime process/code
integrity and SHA256 collision/second-preimage resistance are assumptions of
that association protocol, not publisher authentication or signing. Certificate
tables and nonzero PE checksums must follow the strict sealer's versioned policy.

`verify_unchanged()` rechecks the loaded payload, main-module association,
measured RVA, and the complete ReadFile lease. Any observed failure permanently
poisons the object. It does not rehash bytes or create an atomic memory/disk
snapshot. Errors contain static codes, and the object exposes no Debug,
pathname, raw handles or raw pointers. The FFI module only reads initialized
static/output storage; it performs no runtime patching or memory protection
changes. Default tests exercise the unsealed early guard. Two explicitly
ignored synthetic tests require an independently post-link sealed test executable
and check actual payload/RVA, chunked reads, write-sharing rejection, limits and
permanent invalidation. Wine evidence is preliminary and does not establish
actual Windows/NTFS or complete product support. Product enablement remains a
separate integration gate.


## Explicit GNU/Linux private file lease

The original `linux::lease` primitive is versioned as
`linux-gnu-x64-exclusive-file-lease-v1`. It is available only for the admitted
GNU/Linux x86_64 ABI, and establishes an actual kernel write lease through a
short-lived 128 KiB scoped helper. A conservative 256 KiB helper reservation is
exposed for caller admission; it is not RSS or an exact allocator measurement.
The helper blocks SIGIO locally, targets its own TID before lease acquisition,
rejects pending SIGIO, clears the owner and exits without unblocking pending
thread signals. Caller masks and process signal dispositions remain unchanged.
Unsupported/unavailable states reject without advisory fallback.

Callers needing private content must create a fresh unshared File description,
retain it and never expose/duplicate/map it. Same-description duplicates share
a lease; the primitive cannot deny trusted caller writes. `verify` requires the
exact exclusive lease and no signal recipient, including rejection of pending
breaks. Independent name, owner/mode/link/full metadata/mount/content and source
guards remain necessary. This is not an atomic filesystem snapshot or a defence
against privileged process/debugger interference. No sysctl, host signal handler,
network, cleanup, publication or resume behavior is introduced.

The optional collision archive uses that primitive with its private framing;
its source profile v8 and scope are described in
`docs/forever-collision-leased-archive.md`. Source inclusion and earlier focused
synthetic evidence do not enable a product storage mode or build a new bundle.


## Separate Windows private-output foundation

`windows::output` creates a new owner-SID private root below a retained
`ReadDirectory`, with protected explicit DACLs, handle-relative exclusive child
creation, bounded append/flush and same-root no-overwrite rename. Directory
keepers share read/write, deny delete and verify observed identity/security
changes; directory write exclusion and an atomic namespace against same-SID or
privileged mutation are not claimed. File writers share read only, excluding
competing data-write/delete opens; same-SID metadata/ACL mutation is checked,
not prevented by that sharing mode.
Limits only reduce 128 objects, depth 16, 64 MiB/file, 256 MiB total writes,
1 MiB/append and 16,384 operations; names have a lowercase ASCII grammar and
at most 64 bytes. Full paths remain bounded by 32,760 UTF16 units.
Ordinary synthetic native Windows x86_64/fixed local NTFS acceptance is recorded
on 2026-10-12 for `a220596da8a29f9f3a867dc7f2bd016d01cd3558`: fourteen focused
cases pass in each debug/release profile. The independent ACL oracle checks
persisted security after output/parent leases close; its live path probe is
diagnostic only. Privileged reparse remains ignored. This separate API does not
enable the Windows product, package publication/recovery, distribution or crash
durability. The later documentation commit is not the tested functional SHA;
frozen contract pending flags retain their earlier checkpoint meaning. See [the versioned scope](../../docs/forever-windows-output.md) and
`contracts/forever-windows-output-contract-v1.json`; the Linux hardlink/recovery
protocol and accepted v9 bundle retain their original scope.


## Explicit Windows membership capability

`windows::output::membership::verify_membership(&PrivateRoot, Limits)` is a
separate opt-in observed-membership pass over every owned directory. It compares
current entries with complete observed child records, with original root guards
and complete transient directory snapshots before/after. Observed failure
permanently poisons the root; unknown entries are not adopted or traversed.
The original `verify_unchanged` and ordinary mutations do not auto-enumerate.

Downward-only ceilings are 128 directories, 512 admitted/decoded records
(including dot entries) and 640 native query calls. A charged query at the record
cap can confirm EOF; extra data rejects before decoding and poisons. Enumeration
uses class 38, a fixed aligned 640-byte buffer and strict single-entry/EOF framing.
Concurrent changes may escape observation: no atomic snapshot, directory write
exclusion or same-SID isolation is claimed. Native acceptance was pending at the authored source
checkpoint; the earlier fourteen-case output acceptance retains its own scope.
No Windows package/recovery/product or new bundle is enabled. See
[the membership scope](../../docs/forever-windows-membership.md) and
`contracts/forever-windows-membership-contract-v1.json`.

Ordinary membership now has separate native acceptance on functional commit
`433d5ce`: all 24 exact cases pass debug/release on fixed local NTFS in
[CI run 38261424386](https://github.com/alseif0x/rustycore-extractors/actions/runs/38261424386),
including 13 NTFS fixtures and 11 pure cases. The full Windows workspace passes
2,175 tests; Linux debug/release each pass 2,643. The authored contract flags
retain their checkpoint meaning. Package publication/recovery, privileged
reparse and full extraction/runtime remain open; see the executed acceptance
in [the membership scope](../../docs/forever-windows-membership.md).
