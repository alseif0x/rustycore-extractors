# Private collision append archive — source profile v8

This is an authored intermediate library capability, not a product storage mode. It does not enable the
product CLI, publish a package, provide resume, authenticate complete client
inputs or establish native/runtime parity. Resource-profile acceptance remains
separate and incomplete. The current source selects mandatory corresponding-source profile
`forever-seven-crate-linux-collision-leased-archive-source-v8`; no v8 bundle is built or accepted.

## Admission and ownership

`spool::BackendProfile::FileSetV1` remains default; explicit `LeasedAppendV1`
requires the GNU/Linux x86_64 platform primitive and an actual kernel write
lease. Unsupported/unavailable leases reject; no advisory fallback is allowed.
Volume profile and cumulative resource profile are independent explicit choices.
All six actual owners (RAW models, WDTs, own caches, placements, VMOs and final
map/proof records) must agree on all three policies before acquisition/assembly.
A fresh read/write file description is private, never duplicated, exposed,
reopened for I/O or mapped. There is no public raw-record ordinal.

The platform helper blocks SIGIO in its own short-lived thread, targets that
thread before acquiring the lease, rejects pending SIGIO, clears the recipient,
and exits without restoring its mask. The caller's signal mask and process
signal dispositions are unchanged. Pending lease breaks reject immediately.
A lease is not a filesystem snapshot: name, mode, owner, links, full recorded
metadata, mount, namespace, protected input and content guards remain required.
Same-description duplicates and privileged debugger/process interference are
outside this private-description contract, rather than denied by the kernel.

## Private framing and verification

One exclusive `records.archive` regular file is created at mode 0600. A 32-byte
container header binds RCEARC1, version 1, header size and zero reserved bytes.
Each append has a 64-byte RCERCD1 header binding ordinal, complete body length,
SHA256 and zero reserved bytes. A write retains its full disk reservation on
failure, seals only its new header, syncs and rereads every new byte before
publishing its compact descriptor. Failed/caught-panicking owners cannot resume.
A reload checks the selected complete header, bounded extent and body digest
before invoking the callback. Typed causes survive independent final guards.
These bytes are private intermediates, not native RAW/VMO/VMAP wire formats.

Kernel protection plus fixed-size header/metadata checks replace scans over all
prior file records in this explicit backend. Descriptor growth is geometric with
checked old/new allocation overlap and fallback to exact admitted capacity.
The global guard count is constant in record count; ancestor depth and supplied
protected inputs retain their independent fixed ceilings. Selected-body hashing,
conversion and higher-level inventory/proof work remain bounded separately.
No claim of total linear whole-world CPU cost is made.

## Accounting and planned evidence

Disk accounting includes 32 container bytes and 64 framing bytes per record.
Reload-byte accounting includes selected record framing plus body; fixed global
container/metadata guard reads are not represented as all I/O/CPU counters.
Actual descriptor capacities and 256 KiB conservative helper admission remain
charged; helper requested stack is 128 KiB. Counters are not RSS or exact allocator
measurement. Existing active-body/memory ceilings are not raised by the backend.

Earlier frozen Rust focused evidence (accepted separately from current v8): four kernel tests (including 16 targeted SIGIO races), ten
ordinary archive cases, an explicitly executed 4,097-distinct-record write/read
corpus with exact 20,485 archive guards and bounded steady FDs, and one complete
six-owner toy-CASC package compared with the existing resident Rust assembler.
The earlier Python 28-case kernel report is primitive evidence only. Full
workspace, source-profile/distribution contracts, native/client/runtime,
scalable product publication/resume and encrypted mandatory inputs remain gates.


## Earlier accepted focused report

Frozen source `01ed12b3317061d2489a6618de36e11a17583a1abcff99ab11805d47a3874681`
passes 13 commands and 42 tests across debug/release, including both explicit
4,097-record corpora and both complete small six-owner packages. Report SHA256:
`b7229428caa69a2f66a89f08b92c9d12d50cd7dd134e793538ac690043d526cb`.
The local root replays all 13 observed IPC-only traces; source/tools are unchanged.
The initial mode-copy and observer-path launch failures remain retained.
This scope is not a final current-v8/full-workspace/native/client/product result.
Expanded M2/WMO/parent package, backend-mixing and corresponding-source closure
checks were pending at that earlier checkpoint. Complete current-v8
executable-source acceptance is recorded below.

## Subsequent cache admission correction (2026-10-12)

The expanded package matrix exposed a conservative admission conflict: two
64 MiB descriptor ceilings plus the 512 KiB guard allowance leave less than
128 MiB inside the disk profile's 256 MiB inventory ceiling. Reserving the
previous 128 MiB cache parser allowance rejected even a small own-tile cache.
The disk profile now defaults to a reduced 64 MiB cache parser allowance.
No memory, active-body, parser or cumulative ceiling is raised. Earlier
resource evidence remains historical; the later report below accepts this correction.

The full-workspace attempt also retained a directory-metadata test failure:
a newly created name can share the captured timestamp. That test now forces
a changed timestamp. LocalStore's production directory guard remains
metadata-only and does not authenticate a full directory-name snapshot.

A subsequent complete-workspace attempt also exposed a same-length MAPS
fixture rewrite with unchanged metadata timestamps. The Linux MAPS fixture and
its equivalent VMap fixture now force a distinct mtime before testing metadata
verification. Their production readers and post-read metadata-only scope are
unchanged. The earlier failed workspace reports remain retained.

## Package lock owner lifetime correction

A later complete-workspace attempt retained one Busy failure during recovery.
A deterministic private duplicate-descriptor regression reproduces the lock
lifetime problem: closing only one description does not release the lock while
an inherited duplicate exists. Package locks now explicitly unlock on owner
drop, then close their descriptor. This covers incomplete Run drop, completed
Run drop and temporary verifier owners, without changing formats or Busy
admission while an actual owner remains. Drop cannot return unlock errors;
closing retains the previous fallback if explicit unlock fails. The regression
checks both dropped and completed owners while a private duplicate is held.
No actual fork sequence or inherited-descriptor access policy is claimed.

The next whole-workspace attempt retained two more same-clock fixture failures
in MAPS-root and navigation inventory metadata verification. Those fixtures
now force distinct file/directory mtimes, including child inventory directories.
Production metadata-only checks and source/reader limitations are unchanged.

A subsequent debug pass was complete; release retained one child-directory
metadata fixture failure in navigation run tests. That case and its sibling
new-entry callback now force distinct directory mtimes. A complete navigation
release precheck is included before the whole-workspace matrix.

## Complete current-v8 executable-source acceptance

The current mandatory corresponding-source profile is
`forever-seven-crate-linux-collision-leased-archive-source-v8`. GNU/Linux x86_64
write leases protect one private append archive per owner. All six actual
RAW/WDT/cache/placement/VMO/map owners bind the same backend, volume and resource
policies. FileSetV1 remains default. Complete framing/digests, source/namespace /
mount guards and terminal poisoning remain. Geometric descriptors and constant
archive guard counts do not claim whole-world linear work or throughput.

The disk-profile cache parser allowance is reduced to 64 MiB in the inventory
and world runner; no memory ceiling is raised. Metadata-only fixtures
force distinct UNIX timestamps. Production reader behavior is unchanged.
The complete captured root plus helper reservation is admitted before archive
creation; a downward-budget regression retains only the empty private namespace.

Frozen executable/test source passes 2,599 ordinary tests across 41 targets per
Linux debug/release profile (26 ignored), four explicitly executed ignored
corpora per profile, 180 contracts and 108 distribution/sealer checks. Workspace
all-target Linux/GNU-Windows Clippy, both Windows compile profiles and scoped
formatting pass. Each 4,097-record corpus performs 20,485 archive guards with
bounded steady FDs. Four complete six-owner M2/WMO/parent packages match 27
resident-Rust artifacts / 2,684 bytes; one exact terminal nearer-parent
Association is checked, also on a second filesystem. All 27 command traces,
immutable source/tools and live terminal observations pass independent review.
Earlier failed reports remain retained and rejected.

Seven crates, nine packages and two exact registry archives remain. A Linux-local
CASC-to-platform edge and 28 mandatory source paths enter v8. Historical v7
contracts/import pins retain their scope. No new bundle/rebuild, Windows runtime,
native/client/runtime or product acceptance is established. The product still
uses resident assembly. Physical-disk preflight, publication/resume integration,
encrypted mandatory complete inputs, real geometry/runtime and full distribution
remain gates. The extractor is not complete. See
`docs/forever-collision-leased-archive.md` for hashes and retained history.

Retained identities:

- source_freeze_sha256: `b022e4d2d072334ce9cd4fa8a196786aada270c481f6538f6531c759345c1d58`.
- report_sha256: `7d1e36098e355e79086cbc0a035ebf6f6a8114ac772bdd6be7a9b83eddb3c831`.
- root_review_sha256: `09c4a565c667d3708796c352aed40b92a4833141b0e355bf2c9a4b67f693778a`.
- observation_sha256: `8f67af39a0f6e1311d1840f2e04aa7622ee3912453a934d6e0d156df6975ab9e`.

The accepted freeze has 1,003 public entries. Final publication changes only
documentation, JSON evidence and static assertions. Executable Rust and Rust
test bytes remain identical; final 180/108 static checks are recorded separately.

### Retained failed workspace reports

- `459e0f539f5d35ef20ec19fd40591f33548b8ede83dc05faaee41e2527d53404`: cache reservation conflict and a same-timestamp directory fixture. Rejected and retained.
- `f5abe0f905d66f0b6018c1de079a9ae0b07f364c5b4ac52930b9129fc0d93a48`: inventory cache reduction not yet propagated to the world runner. Rejected and retained.
- `9d2c6486b025b6e30cda835f0dac99418364b79acc55b40bcda43002420f1a1a`: complete CASC passed; a same-length MAPS fixture retained metadata timestamps. Rejected and retained.
- `505deee0e8e2f9abf6c45d1985c566764c80c6bcce2316faaf8351ed5e80a611`: one recovery attempt returned Busy after the preceding owner was dropped. Rejected and retained.
- `311e31aff022b4096f583732210c1620b020363510442a5abc7ecf443374562b`: deterministic duplicate-description regression reproduced Busy before explicit owner unlock. Rejected and retained.
- `dffa03da766a34eb9d9e76e89550c1ef833a5c50a02206cd619d255f3195082c`: complete CASC and owner-release regression passed; two navigation metadata fixtures retained timestamps. Rejected and retained.
- `f7b2f95b53c07de74091799f003446b2594c1d30b307ae73a96c4b1318cffcb6`: whole workspace debug passed; release retained a child-directory timestamp fixture failure. Rejected and retained.
- `574e0092c58c83aa81767824542fee01531ca83cf1919738fcbc78a355eae939`: all 26 nested navigation cases passed; the checker expected 23 and rejected the run before the complete matrix. Rejected and retained.
- `72f60eaa7f077113ac57e3a77c00af0adfe553418a5864f5011662ebca84ebcc`: the controlled earlier Spool creates records.archive before rejecting the root-plus-helper budget; the current source rejects before archive creation. Rejected and retained.

The earlier complete v8g report (`48ed51a66d11819f405f007db02ee50f2bf1f6fb94dfd48ecf4d7ffd82566899`) remains accepted only for its exact frozen source. Final review then found the helper-admission boundary above; the current accepted source adds the complete root-plus-helper reservation before archive creation. The unchanged 11-test archive inventory now includes that boundary in its framing test.

All preceding launch/resource failures remain unaccepted. No malformed native
execution or stronger production reader claim follows from fixture corrections.
