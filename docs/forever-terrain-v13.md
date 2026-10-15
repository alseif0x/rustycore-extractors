# Linux70374 compatible terrain product v13 — candidate

This explicit candidate preserves strict v12 and adds a separately selected
available-map traversal policy. It has no real v13 extraction, original-loop
comparison, whole-world, runtime, distribution or release acceptance yet.
The existing v12 evidence retains its original scope.

## Explicit plan and commands

Use the canonical examples in `tests/contracts/fixtures/forever-product-v13`.
V13 requires Linux, Rust-local build70374, the current wowdbdefs-70374-v1 catalog,
disabled keys, resident maps-only component mask8, and `map:<u32>` or
`all-available-maps`. Tile subsets, supplied keys, native backend, scratch,
DB2/GT/camera outputs, collision and navigation are rejected.

The commands remain inspect, extract, resume, verify and verify-links. V13 uses
`Build70374CompatibleTerrainV1`, the separate
`linux70374-compatible-full-maps-available-map-v1` source profile, and coveragev2.
Catalog/build equality does not opt older plans into the new policy.

## Explicit missing-input policy

Map's authenticated available sections and all three complete liquid startup
tables remain mandatory. Global Map coverage keeps admitted/omitted sections,
declared/admitted row counts, and independent full-CKey/EKey authentication
flags. Unknown encrypted sections are not plaintext or a complete Map claim.

Only `LocalStore::MissingFile` while reading a WDT or known-present root ADT is
an admitted omission. Other store errors, encrypted-only candidates, corrupt or
incomplete encodings, unsupported formats, invalid routing, malformed WDT/ADT,
conversion errors, resource limits, source changes and output failures remain
terminal. Every omission is followed by source closure before traversal
continues. Reads and outputs retain the existing logical budgets.

An unavailable WDT emits a zero tilelist and reports MAIN as **unknown**. Its
4096 coordinates are unknown, not known absent. A loaded WDT reports known MAIN
present/absent coordinates. A MissingFile root ADT emits no MAPS and leaves its
list bit clear while traversal continues. Converted and missing root counts sum
to that map's known-present count. A successfully loaded empty MAIN remains
distinct from an unavailable WDT.

Source traversal preserves the materialized Map order and each WDT's row then
column order. Every selected map, including unavailable/empty maps, emits a
whole-map tilelist. Invalid dependencies do not become omissions. The source
policy follows the original successful-output/list-bit behavior for these
explicit unavailable inputs; it does not claim general native failure parity.
Original output-open failure also returns false, while product publication
failure deliberately stays terminal.

## Coverage and verification

`maps/compatible-terrain-coverage-v2.tsv` is canonical bounded ASCII TSV. It binds
the six-key Context (source/plan/producer/build/locale/component mask), selection
and packing. Per-map rows retain plan index, materialized ordinal, full-u32 map
ID, loaded/unavailable WDT disposition, known present count or unknown, converted
count, MissingFile root ADT count and exact tilelist association. The finish row
separates known present, known absent, unknown, converted and omitted counts.
Transient candidate-work counters do not change canonical replay bytes.

Both offline verifiers preflight inventory metadata before payload allocation,
validate strict70374 MAPS and tilelists, require every declared map's list, and
check exact successful MAPS/list bitsets and counts. Extra/missing/foreign outputs,
wrong context, false counts and malformed coverage reject. The verification
profile is `linux70374-compatible-available-map-terrain-product-v1`, with scope
`artifact_integrity_intrinsic_formats_and_declared_available_map_omission_coverage_links`.
Coverage remains producer-reported; offline verification does not independently
authenticate unobserved client MAIN, Map sections or omission causes.

A completed selected output can therefore contain explicitly admitted omissions.
It does not assert that all client maps or all known-present inputs were acquired.

## Publication, resume and resources

Incomplete same-context output can resume. Even a sealed terrain stage is
reacquired and replayed through existing Run receipts, including coverage and
omission bytes. A changed disposition/coverage rejects; complete packages reject
resume without modification. Any fatal error leaves an incomplete product that
both verifiers reject. No provisional prefix is accepted.

V13 retains v12 ceilings: 64GiB logical source/output, 262144 package files,
4096 maps, 96MiB manifest metadata, 1MiB coverage, 256MiB package-metadata
reservation, and the 8MiB sink reservation subtracted from the 1GiB combined core
memory ceiling. Sink state and outcome capacities are charged; MAPS is streamed
through Run::put and never retained as a whole selection. These are logical
budgets, not exact RSS, physical I/O or global process-tree CPU measurements.
Caps fail without truncating the requested domain.

## Authored acceptance scope

Focused Rust source tests cover plan admission, canonical coverage, unknown MAIN,
omission accounting, source order, output associations, malformed inputs and
sealed replay. CLI tests use only authored CASC/DB2/WDT/ADT bytes, including a
correctly framed synthetic encrypted omitted Map section. They exercise selected/all acquisition,
MissingFile WDT/root behavior, strict-v12 negatives and both verifiers.
Recovery tests construct deterministic empty/partial/sealed Run states, then run
the actual CLI resume and compare complete output bytes. They are not killed-process
or fsync-crash tests. Clean/recovered equality is a lifecycle regression, not an
independent native conversion oracle. Original-loop authored comparison and a
fresh immutable whole qualification were separate gates at that checkpoint.
The subsequent qualification below closes the software gate only.

The focused candidate validation passes 389 extractor tests per debug/release
profile (358 library and 31 integration), including 32 new authored methods.
The exact source-contract subset passes 424 tests with two historical skips;
distribution passes 162 with one historical platform skip. Extractor all-target
Clippy and scoped formatting pass. Validation receipt is
`2f138088e8c15aee3e8b885db4ee1919c6103855b8d2370cc80666f706b49b8b`.
Three initial source-anchor failures are retained; only the anchors for the
shared terrain gate and explicit v13/v12 metadata dispatch are corrected.
Separate earlier library validation passes 65 tests before formatting; its
receipt remains tied to that earlier source. None is whole-workspace or
real-client acceptance for the current committed producer.

## Committed Linux software and forced-interruption checkpoint

Source commit `3eb45c1c734b6f950303ac1f26587e27b3b3cdbd` is frozen with
795 files and 202 directories. Its eight-command offline qualification passes
3,147 tests / 26 historical ignores in both debug and release, 424 source
contracts / two skips, 165 distribution tests / one skip, workspace all-target
Clippy and full formatting. Both executable copies match their actual build
outputs. All 3,970 software guards / 48 aliases and the source inventory close
unchanged. Qualification report:
`751c1854296f681f7d791a25637d81a6918c759f0729fddc928aa54171a24993`;
independent terminal review:
`fff748597289d44d43c3957398f22182e45482c908acb6e2f708591845ed79e5`.
The packaging correction includes five existing CI inputs and passes 84 focused
synthetic packaging tests; it does not establish reproducible distribution.

A separate release campaign executes eleven authored CLI calls: a clean triplet
and two actual fsync-injected SIGKILL cuts followed by resume and both verifiers.
One cut interrupts the first MAPS transaction; the other occurs after the maps
seal and before manifest publication. All three completed views match the
independent authored MAPS oracle, exact tilelists, coverage, manifests and state
receipts: six MAPS, six lists and three coverage files across those views.
Independent review passes 251 checks, with 4,024 guards / 48 aliases, exact cut
snapshots, traces, absent owned groups and numeric parent exit zero. Report:
`5778a4fcd90f6f303fd15d5eb58e94a334b75c7385eed6c869668dcbf9315243`;
independent review:
`b54fb176f3c584d62df4cdbcb0b06191e3352160aff3ce22afa6c5a9de4a2093`.
Two auditor-only scope/mode assumptions were corrected in retained attempts;
the producer was not changed or rerun. Interrupted fsync is not successful fsync,
and these cuts do not establish power-loss or crash durability.

This checkpoint executes no real-client extraction or original native loop.
The separately prepared eleven-case original-loop comparison, real all-map
extraction with both verifiers, stable distribution and remaining component and
runtime gates stay open. Historical v12 failures and contracts remain intact.
