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
