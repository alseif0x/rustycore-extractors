# Output conformance harness

The v1 harness compares a pinned TrinityCoreLegacy reference output with a
candidate output at three levels: relative file set, parsed binary structures,
and exact bytes. It emits only relative names, sizes, SHA-256 digests and
structural facts. It does not embed client payloads or absolute input paths.

This is engineering compatibility evidence, not legal advice and not
permission to redistribute extracted data.

## Run locally

Keep both output trees and the report outside the checkout:

```bash
python3 tools/conformance/compare_outputs.py \
  --reference /home/server/rustycore-extractor-work/runs/reference/<run-id> \
  --candidate /home/server/rustycore-extractor-work/runs/candidate/<run-id> \
  --reference-version-json /home/server/rustycore-extractor-work/reports/<run-id>-reference-version.json \
  --candidate-version-json /home/server/rustycore-extractor-work/reports/<run-id>-candidate-version.json \
  --report /home/server/rustycore-extractor-work/reports/<run-id>-conformance.json
```

Exit `0` means conformance passed, exit `9` means a compatibility mismatch,
and exit `2` means the harness itself could not run. Input roots are read-only.
The report is written last and may be retained because it contains hashes and
structured evidence rather than extracted payloads.

## Comparisons

- The manifest layer reports missing and unexpected relative files.
- MAP parsing validates `MAPS` version 10, build, ordered AREA/MHGT/MLIQ
  chunks, terrain encoding, liquid dimensions and holes.
- WDC4 parsing records table/layout identity, locale, field layout and section
  metadata for locale-specific DB2 output.
- VMap parsing validates `VMAP_4.B`, map BIH/spawn indices, WMO/M2 spawn
  transforms and bounds, `.vmo` group/vertex/triangle/liquid structure, and
  the game-object model index.
- MMap parsing validates version 15, Detour version 7, 64-bit polygon-reference
  policy, navmesh parameters, initialized padding and tile mesh counts.
- Files match byte-for-byte unless the reviewed profile explicitly assigns a
  path to semantic-only comparison. The v1 profile has no exceptions.

Malformed structures fail independently of byte comparison, so matching a
candidate against an equally truncated or corrupted copy is not a pass.
Symlinks and incomplete-run markers are rejected.

## Synthetic CI coverage

`tests/conformance/test_harness.py` creates reproducible, non-Blizzard binary
fixtures in a temporary directory. It proves detection of changed headers,
field ordering, omitted files, truncated data and incomplete outputs. Nothing
with a client-data suffix is stored in Git.

## RustyCore reader boundary

At the 2026-08-23 audit of `/home/server/rustycore`, `wow-map::GridMap` parses
MAPS/10 terrain and `wow-recastdetour` parses MMAP/15 plus Detour 7 using
64-bit polygon references. Their focused synthetic tests are local reader
acceptance evidence. RustyCore's `wow-map::vmap` currently defines a static LOS
provider interface but does not load `.vmtree`, `.vmtile` or `.vmo`; therefore
full RustyCore VMap reader acceptance cannot honestly be claimed yet. The
harness uses the pinned C++ readers/writers as the structural authority and
keeps this missing Rust reader visible as a release risk.

Real-client closeout must use newly produced reference and candidate runs with
recorded commands and tool metadata. The pre-existing
`/home/server/woltk-server-core/Data` tree is useful for parser smoke tests but
is not final conformance evidence because its producing versions are unknown.
