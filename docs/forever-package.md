# Forever package integrity v1

`forever_casc::package` adds a common disk stage and offline verifier for explicit
Forever build 70170/70205 plans. The current disk profile has Linux x86_64 evidence; other architectures are unaccepted. The
[contract](../contracts/forever-package-contract-v1.json) and
[manifest schema](../schemas/forever-package-v1.schema.json) are versioned.

The final manifest remains **v1**. Newly initialized stages use the separate
[internal state v2 contract](../contracts/forever-package-state-v2.json) and
[state schema](../schemas/forever-package-state-v2.schema.json), adding explicit
same-context recovery. Completed legacy v1 packages stay read-only verifiable;
incomplete v1 stages are not migrated.

## What completion means

The context binds the build, locale, selected component mask, and caller-supplied
SHA-256 identities for the complete source inventory/acquisition, selected plan
and options, and producer. These fingerprints are not signatures or evidence
that a caller supplied correct provenance. Pin an independently expected context
when accepting someone else's output.

`scope: artifact_integrity` proves the observed files match the producer's byte
inventory and all explicitly selected producer completion declarations. The API
can package arbitrary bytes. Its verifier does not decode DB2, GT, cameras, MAPS,
VMAP or MMAP and cannot establish reader/runtime compatibility. Preserve those
component verifiers and the recorded C++/RustyCore acceptance separately.

Component bits are DB2=1, GT=2, cameras=4, maps=8, vmaps=16 and mmaps=32. Mask 63
selects all six. Empty components still need an explicit completion declaration.
Selecting only mmaps never declares the full extractor complete. The full goal's
[completion requirements](forever-completion.md) still apply.

The [product interface v1](forever-product.md) now supplies the explicit plan,
logical collection identity, loaded producer binary identity and actual selected
Rust producers. It requires its plan again for verification and adds bounded
intrinsic format/selection checks. Its source identity deliberately describes
the authenticated logical collection, not physical archive availability or a full
archive snapshot. This does not change the generic package's caller trust model.

`verify_with` additionally hands rooted read-only artifact descriptors to caller
checks while holding the shared lock. Hash checks bind each descriptor, and
receipt/seal/manifest/namespace checks repeat after callbacks. Callback results
remain provisional until the complete call returns; caller buffers/semantic work
are outside the inventory's logical memory budget. The original `verify` path
retains its previous integrity-only behavior. Neither API supplies an atomic
snapshot against arbitrary concurrent external filesystem mutation.

## Producer integration

```rust,ignore
let mut run = package::Run::create(output, context, protected_inputs, limits)?;
// Provisional callbacks: preserve the producer's final bytes exactly.
run.put(package::Component::Navigation, "mmaps/0001.mmap", global_bytes)?;
// Only after the corresponding producer returns its complete result:
run.finish_component(package::Component::Navigation)?;
let summary = run.finish()?;
```

`create` requires a new absolute output directory outside Git and outside all
caller-supplied protected inputs. Its parent must exist. It creates private
0700 directories and 0600 files, a durable incomplete marker and an OS managed
exclusive writer lock. Supply every client/reference/input root as protected;
the API does not discover these environment-specific paths automatically.

`put` publishes a complete file without overwrite using a same-directory pending
file, fsync, exclusive hard link, pending unlink and directory fsync. A durable
`.state/transaction.json` binds exact expected artifact facts before
publication. After recording canonical per-file facts in `.state/files/`, the
writer removes the transaction and syncs the state directory. Identical retained
writes are idempotent, including replay of a sealed component's existing files.
New writes to sealed components, conflicting writes or any mutation error poison
the active run. Dropped, interrupted and failed runs retain their marker and
cannot verify as complete. `create` never reuses an existing stage; explicit
state-v2 recovery follows the rules below. Deleting a marker is not recovery.

`finish_component` rehashes its files and records a seal. `finish` requires every
selected seal, validates the full receipt inventory, hashes all payloads, rejects
unknown files/directories and writes the canonical manifest. Only then is the
incomplete marker removed and the root synced. Component callbacks remain
provisional until successful `finish`.

Payload names use `dbc/<locale>/`, `gt/`, `cameras/`, `maps/`, `vmaps/` and `mmaps/`.
The bounded Linux profile permits printable ASCII including spaces and ordinary
punctuation, up to 512 path bytes, with no quotes, backslashes, `:*?<>|`, hidden,
empty, dot, dot-dot or trailing dot/space segments. Filesystem basename limits
still apply. File and directory case aliases reject. Names are never silently
normalized or renamed. This is not a Windows filename acceptance result.

## Explicit state-v2 resume

```rust,ignore
// Recompute the exact complete source/plan/producer fingerprints first.
let mut run = package::Run::resume(output, expected_context, protected_inputs, limits)?;
let completed = run.completed_components()?;
// Skip only fully sealed selected components. Replay every unfinished producer
// to its complete result, emitting all retained files with the same exact bytes.
```

Recovery requires an initialized state-v2 protocol, canonical incomplete marker,
matching caller-expected context, exclusive OS writer lock and the same protected
input/Git boundaries. Every committed file/receipt and optional seal is checked,
and the observed namespace must close, before any deletion or publication.
Declared pending artifacts reserve their full file/byte/metadata budgets first.
An unsealed component cannot complete until all of its retained artifacts have
been re-emitted exactly; duplicate callbacks never cover a different file.

Only explicitly identified tool-owned pending files may be discarded. A
standalone unpublished pending file must be regular, single-link and within its
declared size; its partial bytes are never trusted. A final/pending publication
pair must be the same inode with exactly two links. Conflicting pairs, unrelated
links, unknown files, symlinks and nonregular files reject without cleanup.
A final artifact with a durable intent is rehashed before a missing receipt is
reconstructed. The journal is removed only after that receipt is durable.

An unresolved intent remains mandatory for exact producer replay. Published
seals are retained after full validation; unpublished seal prefixes are discarded
and require renewed producer completion. An unpublished manifest is regenerated
only after every selected seal. A complete matching published manifest surviving
beside the incomplete marker is reused without rewriting its bytes/inode.
The marker remains throughout recovery and is removed only by successful finish.

A bounded sorted digest index tracks retained-file replay coverage. Its actual
Vec capacity is charged in addition to retained fact metadata before recovery
mutates the stage. Later inventories subtract the index reservation; complete
coverage releases it. Source/payload buffers and allocator overhead remain
outside this logical ledger.

Preflight errors preserve all stage bytes and names. I/O failure or interruption
while applying validated repairs can leave some safe steps applied; the marker
remains and a later explicit resume validates again. This is not an atomic
filesystem repair transaction or power-cut durability certification. Completed
markerless output refuses write access. Incomplete legacy v1 and initial create
failures before durable context/protocol/lock initialization remain preserved and
unresumable (`package_state_version` or initialization/path errors).

The API does not discover or authenticate the caller's source fingerprints and
does not checkpoint Recast/CASC computation. Unfinished producers replay their
work; completed component seals permit skipping whole completed components.
A unified production planner/driver still needs its own integration.

## Offline verification

```bash
forever-package verify --output /absolute/complete-output \
  --expect-context /absolute/canonical-context.json
```

The optional context file contains exactly `Context::json()` without a trailing
newline. Without it the verifier checks internal consistency against the stored
context. The command reads only the package and optional explicit context; it
has no client, CASC, key, listfile or network path. It does not write outputs.
Diagnostics are one structured JSON line without filesystem paths. Exit codes:
0 verified, 2 invocation/platform, 4 I/O, 8 incomplete, 9 integrity/context/schema/
path/lock/resource rejection. Active writers and replaced lock files reject.

The verifier reads bounded canonical lines, rehashes manifest/inventory/payloads,
compares every receipt and seal, and closes the namespace. Missing, extra,
changed, symlinked, hard-linked and nonregular payloads fail. Header/footer and
array punctuation are compared against the canonical renderer. JSON Schema
alone does not check these byte-level rules or actual payloads.

Retained directories/files are opened with no-follow flags and reobserved by
identity around reads. Directory enumeration also rechecks identities. These
checks are observations, not an atomic filesystem snapshot against hostile
concurrent mutation. Advisory locks coordinate cooperating producers/verifiers;
SHA-256 consistency is not authenticity. Completed directories should remain
immutable while used. Completion durability depends on Linux filesystem fsync
and hard-link semantics.

Limits only reduce 262,144 artifacts/directories, 128 MiB per artifact, 1 TiB total
payload and 96 MiB manifest. Lines are at most 1,024 bytes and depth is at most 16.
The 256 MiB logical metadata ledger charges actual retained Vec/String capacities,
old/new Vec growth overlap and a fixed 128 KiB scratch allowance. It excludes
caller-owned payload/source buffers, allocator overhead and kernel cache; it is
not a portable RSS bound. Payload/receipt lookups use sorted binary search and
exact cardinality rather than quadratic inventory scans.

## Acceptance scope

The authored `tests/forever/package_navigation.rs` adapter connects the actual
sequential navigation producer to `Run::put`, seals only after `Completed`, then
runs common verification. It accepts only the explicit complete synthetic plane
fixture profile (empty liquid/offmesh inputs), not real acquisition. An injected
sink failure after the first global leaves one provisional file, no seal, no
manifest and a rejected incomplete stage. External acceptance compares final
files with the prior original C++/RustyCore accepted hashes and independently
recomputes manifest/receipt/seal facts using Python hashlib. Debug and release
are separate evidence. ### Initial manifest-v1 acceptance (2026-10-11)

Recorded acceptance passes **three synthetic packages per debug/release profile**,
with **18 final files / 1,433,160 bytes per profile** (four globals and fourteen
tiles). Their complete inventories and hashes preserve the preceding pinned
C++/RustyCore accepted bytes. Each profile rejects eight independently altered
packages, invalid invocation and a producer interrupted after its first global.
All **36 network-denied runtime traces** are empty. This increment reuses prior
native/reader evidence; it does not execute C++ or RustyCore again.

The separate synthetic scale adapter `tests/forever/package_inventory.rs` writes
**1,024 files / 4,096 bytes across all six components** in forward and reverse
order. Both manifests and inventory digests are identical, every payload matches
independent Python hashlib, and both network-denied traces are empty. This is
bounded inventory evidence, not a performance or full-scale client claim.

Final checks pass **28 new package tests** in debug and release, **two new
streaming SHA tests**, **three CLI argument tests**, all **1,950 Rust tests**
(1,943 library, four example, three CLI; 22 ignored corpora), all-target Clippy
with warnings denied, **59 contract tests** including ten new package cases,
and formatting of the changed/new Rust modules. JSON Schema additionally accepts
all eight generated complete package manifests.

Retained external reports:

- `package-20261011T130738Z-f393441e.json` (SHA-256 `73dabf0843cf761f9ca330bbf87afedb3bbc59ca917c013f5faffd50c7def069`);
- `package-scale-20261011T131343Z-2418f8fe.json` (SHA-256 `26a867d76f1124c527e1f0ac38998cfacba8d8df20542057019b324d2806d8e9`).

Raw synthetic outputs, logs and helper builds remain outside the checkout.

### State-v2 restart acceptance (2026-10-11)

The final implementation passes **31 new recovery tests** (**59 package tests**)
in debug and release. Tests cover torn and complete unpublished writes, exact
two-link publication pairs, receipt reconstruction, component and manifest
interruption, context/lock/legacy rejection, preflight preservation, resource
limits, protected paths and mandatory retained-file replay coverage. Repeated
callbacks cannot hide an omitted retained artifact, and the replay index is
charged and released separately.

External acceptance genuinely terminates **22 producer processes with SIGKILL**:
**eight fsync boundaries per debug/release profile** for an authored 32-file,
six-component inventory, plus **three complete synthetic navigation packages per
profile** interrupted after first-global publication. Every recovered inventory
has exactly the same manifest bytes as its uninterrupted reference. Navigation
replay preserves **18 final files / 1,433,160 bytes per profile**, matching the
prior native/reader accepted artifacts. These are process-kill tests on the
current Linux filesystem, not power-loss/device tests.

All **24 newly completed packages** pass independent Python payload/inventory
hashes and manifest-v1/state-v2 schema checks. **Ten verifier invocations** accept
the **eight distinct previous complete v1 packages** without migration. All
**62 network-only traces are empty**; the **22 mixed fsync/SIGKILL traces** contain
no network syscall. Workspace checks pass **1,981 Rust tests** (1,974 library,
four example, three CLI; 22 ignored corpora), all-target Clippy with warnings
denied, **69 contract tests** and formatting. Source/fixture and prior-output
hash guards pass.

Retained report: `package-resume-20261011T134153Z-78675c37.json` (SHA-256 `4ce539aac412f31896c0fe305b2d153db428668830e18c54288bd19234aecbcb`).
An earlier experiment was deliberately stopped after audit found missing replay
coverage. Its incomplete outputs and original binaries remain retained; the
named acceptance uses the corrected implementation. No new C++/RustyCore run,
real-client read, network acquisition or publication occurred.

Unified inspect/extract/verify orchestration, complete dependency plans, efficient
per-task computational skipping, Windows packages, complete real inputs and full runtime acceptance remain
open. This foundation does not change component serializers or their contracts.
