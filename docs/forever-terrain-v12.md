# Linux full-map terrain CLI candidate

[Issue #76](https://github.com/alseif0x/rustycore-extractors/issues/76)
adds explicit `forever-product-v12` for build 70374 on Linux. This candidate
connects available Map rows to complete terrain acquisition and ordinary
product publication. Whole qualification and real full-map acceptance are
pending. The preceding two-tile proof is recorded in
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

The original extractor may continue after failed terrain loads/conversions;
v12 deliberately rejects a known-present failure. No native failure-admission
parity is claimed. This component does not deliver DB2, GT, cameras, collision,
navigation, real full-world/runtime acceptance or a reproducible release.
Windows remains deferred. Earlier accepted bundles keep their own frozen
source/producer identities; this candidate has not replaced them.
