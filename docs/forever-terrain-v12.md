# Linux full-map terrain CLI candidate

[Issue #76](https://github.com/alseif0x/rustycore-extractors/issues/76)
adds explicit `forever-product-v12` for build 70374 on Linux. This candidate
connects available Map rows to complete terrain acquisition and ordinary
product publication. Whole software qualification and the first real Map0
extraction with both verifiers pass. Independent terminal review passes; CLI
recovery acceptance remains pending. The preceding two-tile proof is recorded in
[the compatibility report](forever-linux-compatibility.md).

## Plans and commands

Use the authored [Map 0 plan](../tests/contracts/fixtures/forever-product-v12/map0.tsv)
or [all available maps plan](../tests/contracts/fixtures/forever-product-v12/available.tsv).
Both specify esES, contiguous indexes and Header50893V2 ROOT. Select the
matching explicit locale/layouts for the client; the tool does not guess them.
The plan admits build 70374 only. Outputs must be outside the client tree.

Example command shape with a qualified `forever-extractor` binary:

```bash
forever-extractor inspect --plan tests/contracts/fixtures/forever-product-v12/map0.tsv
forever-extractor extract --plan tests/contracts/fixtures/forever-product-v12/map0.tsv --client /path/to/client --output /path/to/new-output
forever-extractor verify --plan tests/contracts/fixtures/forever-product-v12/map0.tsv --output /path/to/new-output
forever-extractor verify-links --plan tests/contracts/fixtures/forever-product-v12/map0.tsv --output /path/to/new-output
```

Use `resume` with the same plan, client and incomplete output after an
interruption. It repeats terrain production and compares every retained
artifact, including a previously sealed terrain stage. Complete packages
refuse further writes. Preserve failed output/evidence; use a fresh directory
for a new independent run. Run long extractions in a persistent user service
or tmux session as described in AGENTS.md.

## Output and coverage

The package contains complete `.map` files, one whole-map `.tilelist` per
selected map, `maps/selected-terrain-coverage-v1.tsv` and the common manifest.
Absent WDT MAIN coordinates are normal. Every known-present root ADT must
convert successfully before the selection can complete. A dependency or
budget failure retains an incomplete package and fails the selection.

The Map table may have unknown encrypted sections. Their bytes remain
inaccessible; explicit coverage records admitted/omitted sections, row counts
and independent source authentication flags. `all-available-maps` means the
entire admitted Map domain, not all encrypted client maps. Liquid tables,
WDTs and requested ADTs still require complete authenticated source reads.
There are no supplied keys, native fallback or network requests.

Both verification commands enforce exact coverage/list/tile inventory,
70374 binary formats and complete tile/list bitset equality. Coverage is
producer-reported and bound to source/plan/producer identities; offline
verification does not independently authenticate the original client selection.

The [v12 contract](../contracts/forever-product-contract-v12.json) records
limits and scope. The CLI reserves package and sink memory before entering
the bounded core; logical counters are not RSS measurements. Cap failures
reject, without truncating a map or accepting an output prefix.

## Acceptance boundaries

The new full-map API has independent source review and focused synthetic
checks. The product candidate's first review found an inherited CLI test
using now-supported version 12 as an unknown version. Its correction uses
version 13; joint immutable qualification must execute that historical test
alongside the whole workspace and the new product tests.

Initial PR #77 CI also exposes a common-execution boundary regression: the
candidate placed a Linux conditional in the shared lifecycle. The correction
moves terrain dispatch/context to the existing platform adapter; the generic
adapter defaults to the older terrain producer. The operating-system boundary
guard is retained. Its ten source-contract checks and the five v12 public
contract checks pass locally; fresh joint qualification remains pending.

The first actual joint qualification retains a whole failure after five
commands: both Rust profiles pass 3,100 tests / 26 historical ignores and both
product builds succeed, but the CI source-contract set rejects three outdated
verification anchors across two Python files. The correction explicitly checks
v12 metadata preflight/coverage dispatch while retaining the older intrinsic
and v8 checks. All 421 selected CI methods then pass locally with two existing
skips. The failed joint attempt is not accepted as a prefix; a fresh complete
qualification is required before real product execution. Production Rust and
Cargo inputs are unchanged by this test-only correction.

Independent negative review additionally finds three omitted public test
dependencies in the first immutable snapshot: the catalog generator and the
Windows output/membership documents. Its actual selected suite runs 388
methods with three failures, three errors and one skip; the metadata catalog
module does not execute. The new qualification snapshot must include those
three exact committed inputs. Live 421-method success does not qualify that
incomplete snapshot or its retained producers.

The original extractor may continue after failed terrain loads/conversions;
v12 deliberately rejects a known-present failure. No native failure-admission
parity is claimed. This component does not deliver DB2, GT, cameras, collision,
navigation, real full-world/runtime acceptance or a reproducible release.
Windows remains deferred. Earlier accepted bundles keep their own frozen
source/producer identities; this candidate has not replaced them.

## Executed Map0 checkpoint — 2026-10-15

The second whole immutable qualification passes all eight commands. Debug and
release each pass 3,100 tests with 26 historical ignores; both product builds,
421 selected source checks with two expected skips, 163 distribution checks with
one platform skip, all-target Clippy and formatting pass. Independent terminal
QA has no findings; its receipt SHA-256 is
`706835989b069897c7e26a0084b42b02b7eeab678a53b03ea633ca148b0fedea`.
All six GitHub checks pass at compiled source commit
`b40bacf35f5a30b7c3cd0e5a0156ef4714d83680`. The immutable source contains 741
committed public inputs and 41 offline vendor/config files. Later documentation
does not rebuild or replace the qualified producers.

The actual qualified release extracts the requested Map0 with disabled keys
in 28.568 seconds. All 736 present tiles convert; 3,360 MAIN positions are absent.
The completed package has 738 artifacts / 57,334,413 bytes: 736 MAPS, one
whole-map tilelist and canonical coverage. Separate offline `verify` and
`verify-links` calls pass in 5.211 and 5.229 seconds. All three children and
persistent parents exit successfully; child traces are empty, owned groups
are absent and there are zero automatic retries. Parents are untraced.

The private closing validator matches the entire captured output inventory,
source identity and bounded build metadata. It also compares both previously
native-compared MAPS in full: two matches / 142,084 bytes. No original converter
or new CASC acquisition runs during those offline comparisons. Closing receipt
SHA-256 is `8b839f0fe0ee65be213e2e8cde3b8676077ff7ebd33e1350f32a0fc8c0e24fe9`.
Independent review of this actual terminal result passes all 43 checks without
findings; QA receipt SHA-256 is
`e514da54d82e8a8b457ecaad6ff2f11872e5367f10e2898f1f0902be9dbe0afa`.
It checks all 1,481 private product files and their captured identities. Its
first audit remains failed: it compared the qualification's compact inventory
directly with a later inventory containing identities. The reviewed correction
compares exactly the qualification's recorded bytes/modes/hashes and directory
modes, while retaining separate current-source and product identity checks.
It does not infer an inode snapshot from qualification metadata.

Coverage retains 71 direct/materialized Map rows, one admitted section and
three omitted encrypted sections. The encoded source authenticates; its full
decoded CKey does not. Completion applies to the requested Map0 outputs. It
does not imply extraction of omitted maps, an all-available-maps run, recovery,
runtime acceptance or the complete stable suite.

The first post-extract preparation helper fails before private preparation:
its configuration omits the already-pinned static ELF inspector target from
the guard map. It creates neither inputs nor a receipt. The corrected
configuration adds that exact expected guard and a fresh receipt destination;
the unchanged helper then succeeds. The failure is retained, and the actual
extractor is not rerun. The earlier failed qualification and source-only
closing proposals retain their recorded scopes.

## Executed recovery lifecycle — 2026-10-15

The qualified debug and release producers complete the separate authored
48-call lifecycle: 30 successful calls, four forced process cuts and fourteen
expected runtime rejections (ten exit-9 and four exit-8). Both profiles recover
after the first MAPS pending-file sync and after the maps seal, before manifest
publication. Completed-package resume, a missing present ADT, a corrupted
source during sealed replay and tampered coverage reject as specified.

Two strict Rust conversions precede two original C++ conversions of the same
authored inputs. Both whole outputs match, totaling 101,343 bytes; independent
oracle QA passes 65 checks without findings. The lifecycle compares sixteen
whole MAPS files across eight complete package views with that separate oracle.
Clean/resumed manifest equality is an additional recovery check. The run records
48 closed child groups, accepted network/fsync traces and seccomp proofs, one
successful numeric parent terminal and zero retries. Its 4,060 software/input
guards and 48 aliases close unchanged. This is process recovery, not power-loss
durability or a real-client recovery run.

Lifecycle receipt SHA-256:
`0217a63051f17c258c4a843fa626f8696d4d50001936a8564b8d070d54a7c75a`.
Independent terminal QA accepts all 48 calls, 336 leaf evidence files and fourteen
retained output roots, without findings:
`3f95af7b7c06b1b498e270478f1e01c520891cb111f57657a47c649a334826b2`.

The first unexecuted lifecycle controller expected exit-2 for runtime failures.
Its replacement derives exit-8/9 from the frozen CLI and checks each exact
failure cause; fourteen pure controls pass. The unexecuted first terminal
auditor also expected failed replay to retain the pending manifest. Its reviewed
replacement permits only removal of that exact canonical transient, preserving
every artifact, seal and receipt. Fourteen auditor controls pass before its one
actual review. Earlier metadata/auditor failures remain retained. These changes
leave the qualified producer binaries unchanged; the 48-call stage executes
once, without producer retries.

The real Map0 and authored recovery gates are now closed for this terrain
component. All-available-maps acceptance, build-70374 collision/navigation
product integration, runtime acceptance and reproducible stable distribution
remain separate work. No complete stable extraction-suite claim follows.

## Failed all-available-maps attempt — 2026-10-15

After PR #77 merges, the same qualified release makes one actual
`all-available-maps` attempt with disabled keys. It fails after 54.131 seconds
with `file_unavailable` / component `maps`, child exit 9 and parent exit 1.
There are 1,724 provisional MAPS and two provisional tile lists, totaling
1,726 artifacts / 128,123,861 bytes. None is accepted as a successful prefix.
Coverage, a complete manifest and the maps seal are absent; no snapshot,
verifier, resume or retry follows.

The separately reviewed launcher declares 21,600 wall seconds and 14,400 CPU
seconds per inherited process, preserving the other resource and network
controls. This failure occurs before either deadline. Fifty-one pure controls
pass for the new streaming inventory/closing helpers; they do not validate this
failed product. Two preparation failures remain recorded: the helper directory
needed its existing frozen files sealed under a read-only directory, and the
preparer needed the already-pinned resolved ELF inspector guard. These fixes
change neither the qualified producer nor the failed attempt's interpretation.

Independent negative terminal QA closes all 4,010 guards / 48 aliases, the 782
qualified source files and bounded build metadata unchanged. Child network
trace is empty, the recorded child group and parent cgroup are empty, and there
are zero restarts. The private incomplete inventory has 3,456 physical files /
five directories / 128,364,056 bytes. Failure QA receipt SHA-256 is
`f83372bc6a2d8057a8fde443f64a246240677d190f690f9e7cafbd2dcb4ac55f`.
The numeric parent proof is retained before removing only its terminal service;
all failed outputs remain private and preserved.

The generic error establishes neither the causal request nor encryption or
native-reader failure. Source inspection shows that the original map loop
continues after unsuccessful WDT loading and writes an empty tile list; an
unsuccessful ADT conversion leaves that tile's bit clear. Its geometry reader
passes `CASC_LOCALE_ALL_WOW` to the file-open API. Subsequent source review
confirms that the pinned CascLib ignores that argument: ROOT locale selection
occurs when opening storage. The native comparison therefore uses fresh esES
storage for both observations; changing the ignored file-open argument cannot
establish locale recovery. Native built-in keys also remain distinct from the
Rust disabled-key policy. No same-request native comparison or original complete
map-loop execution has yet established the cause. [Issue #78](https://github.com/alseif0x/rustycore-extractors/issues/78)
tracks a bounded private diagnosis before any separately versioned policy
change. Strict v12 behavior and the earlier accepted Map0 package remain intact.

## First failed request reproduced — 2026-10-15

A separately qualified, diagnostic-only Rust caller reconstructs the failed
traversal from private retained metadata. Its one actual run reproduces
`file_unavailable` at the first unfinished WDT, with two source-read attempts
and zero conversions. The diagnostic closes successfully in 33.704 seconds;
that success records the failure and accepts neither extraction nor a prefix.
It neither repeats preceding conversions nor resumes the failed product.

Independent terminal QA closes 4,102 guards / 49 aliases, the 782 qualified
workspace files, bounded client metadata, the retained failed inventory and
private prefix unchanged. All seven child evidence files, an empty network
trace, absent child group and numeric parent exit zero are retained; there are
zero retries. Closing receipt is
`53caed424668b418fd4229106035f07aa1e5d78e10d8ea9d23be524932b6fb66`;
independent QA passes without findings
(`2c24ff5bfb511cf5ca3b437f4ac1d173fb7ec77cd24ae6c0b890d9503ce86ea5`).
Request identifiers, source correlations and per-asset hashes remain private.
The observed Rust error alone still does not establish physical absence,
encryption or native-reader failure. The same-request native comparison is
the next gate. A separate compatible-mode source candidate is being prepared;
it has no product, real-client or native-loop acceptance at this checkpoint.
