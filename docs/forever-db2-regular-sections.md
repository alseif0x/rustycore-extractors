# Explicit regular-section DB2 profiles

`forever-db2-regular-sections-numeric-v1` is a separate supplied-byte library
profile exposed by `DecodedTable::read_regular_sections(data, table,
max_sections)`. The default `DecodedTable::read`, component CLIs, sparse decoder
and closed terrain/GameObject adapters keep their existing admission policies.
The build-specific product v4/v5 profiles now route DB2 files with more than
one section through the metadata-catalog section profile described below;
product v1-v3 retain their previous path. That code routing is not real-file
acceptance.

## Admission

- Two through sixteen sections; callers may reduce the section ceiling to any
  value in that interval.
- Existing reviewed non-catalog schemas with numeric fields, external IDs and
  no parent field. Existing schema/layout-hash gates still apply.
- Regular WDC5 with complete primitive column metadata, existing column
  compression restrictions, no TACT declaration, strings or parent blocks.
- Both catalog count and catalog offset must be zero. The zero-offset rule is
  a conservative Rust profile restriction; original regular C++ ignores it.
- Complete contiguous physical sections, exact ID-table sizes, checked
  aggregate record counts, no overlaps, holes or trailing bytes.

The whole file retains the existing ceilings: 16 MiB input, 100,000 direct
records, 100,000 copy pairs and 1,000,000 retained scalar values. Section count
does not multiply those limits. These are logical work/admission limits, not an
exact heap or RSS measurement.

## Global semantics

All direct records are decoded before copies. Stored external IDs at or below
`MinId` are repaired using the global direct-record ordinal across sections.
Duplicate repaired/direct IDs and overflow reject the entire call.

Copy pairs are visited once in section order. Sources may reside in later
physical sections; earlier successful copies may feed later copies. Forward
copy references are not retried. Existing destinations are overwritten. Zero,
absent and out-of-extent sources are skipped. Applicable self-copies reject in
Rust and must not be executed by the native comparison: the original copy
implementation has undefined behavior for them. Successful results are ordered
by ID; any error returns no partial table.

Inline IDs are excluded because the original materializer rewrites their IDs
using physical offsets. This profile makes no general claim about those
offsets after decompression. Strings and parent translation require separate
reviewed increments.

## Evidence status

The implementation adds 32 authored synthetic unit cases. They have not yet
been executed at this checkpoint. A separate QA example and independent
synthetic corpus/native comparison are being prepared. No new passing
regression, native byte comparison or real-client compatibility is claimed by
this source checkpoint.

The native comparison must use unchanged original `DB2FileLoader` and
`SpellDurationLoadInfo` at commit
`02245dcd245e7433e524577656177723d3e4992e`, compare both direct getters and
materialized rows after `AutoProduceData` and `AutoProduceRecordCopies`, and
run only complete safe positives after Rust/independent fixture admission.
Malformed and policy cases belong to Rust-only rejection evidence.

This does not repair encrypted mandatory inputs, upgrade historical failed
batches, enable multisection production extraction or complete the extractor.

## Executed synthetic acceptance — 2026-10-12

The bounded implementation is now executed and independently reviewed at source
freeze `54ee22d44427caf6a188c270c3749be22455d13331c11b440dc2fddfa8d03cd6`
(1,038 source entries; 12 owned paths). Linux debug and release each pass 2,675
tests, zero failures and 26 existing ignored tests across 42 test summaries. The
32 regular-section unit cases pass in both profiles. GNU/Linux and GNU-Windows
Clippy, scoped rustfmt, both GNU-Windows compile-only profiles, 233 contract
checks and 112 distribution checks pass. Existing Linux executable regression
checks use actual observed, independently classified process traces.

Pinned-original native acceptance uses all 33 retained source/header/notice
entries from TrinityCore reference `02245dcd245e7433e524577656177723d3e4992e`,
including unchanged `SpellDurationLoadInfo` and `DB2FileLoader`. Actual compiler
depfiles resolve to 30 inputs within that inventory plus the QA wrapper; no
Boost or MySQL SDK inputs are used. Debug and release each pass 83 complete
synthetic positives, all 2..16 section counts and numeric compression modes
0..5, comparing direct signed32 getters and complete indexed rows after
`AutoProduceData` and `AutoProduceRecordCopies`. Each profile compares 634
direct rows, 107 copy pairs and 732 materialized rows, with 1,902 direct and
2,196 materialized scalar-bit checks. The profiles also pass 36 Rust-only
negative cases, five downward ceiling checks and two exclusive-output guards.
Each of the 209 actual CLI process network traces per profile is empty; the
outer Python acceptance driver is not syscall-traced.

The first full source-check run preserved an initial native-build preflight
failure: archive extraction had changed original source copies from read-only
mode. The original sources were unchanged and the C++ compiler was never run in
that attempt. A fresh native-only stage applied mode 0400 to the byte-verified
copies and passed all five native-build/example/debug-release acceptance steps.
The earlier nine Linux/GNU-Windows checks passed against the same frozen source.
The rejected attempt remains retained separately, not counted as passing native
work.

This acceptance covers `SpellDuration`'s three signed32 numeric fields. The API's
other reviewed numeric schemas have unit coverage, but this native comparison
makes no float, array, 64-bit, real-client, full-extraction or runtime claim.
The authored contract's `source_checkpoint` keeps its accurate pre-execution
values; this section records the later executed evidence. The raw reports, full
traces, exact terminal observers and binaries remain in the external
`db2-regular-sections-20261012T152238Z` evidence directory. Independent root
review SHA-256:
`896155d17bd458790873f6c87b2dc1d06f980b6183251e4617bcb0c2f0b46f1a`.

## Separate text profile — 2026-10-12

`forever-db2-regular-sections-text-v1` is an additional supplied-byte API:
`DecodedTable::read_regular_sections_with_text(data, table, max_sections)`. It
does not change the numeric v1 API or production `DecodedTable::read`.

The admission boundary remains 2–16 unencrypted regular sections, reviewed
non-catalog schemas, external IDs, no parent blocks, complete contiguous
section layout, and exact EOF. This profile additionally requires at least one
reviewed `String`/`StringArray` field; numeric schemas remain on numeric v1.
String columns must use compression 0. Per-section bytes are interpreted as
records, that section's string bytes, IDs, and copies. The section string sizes
must sum to the header string-table size. The decoder concatenates those pools
in section order and resolves each nonzero field-relative pointer against the
flattened records followed by that pool. A zero pointer yields an empty Rust
string. Strings must be NUL-terminated and valid UTF-8. Copies preserve decoded
text and are charged against the cumulative text budget when applied.

The reference basis is pinned `DB2FileLoader.cpp` at
`02245dcd245e7433e524577656177723d3e4992e`: `LoadTableData` flattens each
section's records and appends its string bytes at the cumulative offsets, while
`RecordGetString` adds the column offset to the stored field-relative pointer.
This source evidence establishes the layout rule, not Rust/native output parity.

The implementation retains the 16 MiB file, 100,000 direct-record, 100,000
copy, 1,000,000 scalar-value, and 64 MiB cumulative UTF-8 allocation/copy
ceilings. They are Rust admission limits, not native format limits or exact
RSS bounds. Catalogs, encrypted/TACT sections, compressed strings, inline IDs,
parent lookups, and malformed-input parity remain outside this profile.

The three added text-focused authored cases pass in the focused Rust run
(`cargo test -p forever-db2 records::regular_sections::tests --lib`: 35 passed,
0 failed, 300 filtered). This run covers section-ordered pointers and copies,
null strings, UTF-8 byte charging, budget boundaries, malformed offsets,
terminators, and aggregate string sizes. It does not compare against the native
loader or real Forever client files. No text-profile native or production
acceptance is claimed.

## Metadata-catalog section profile — 2026-10-12

`forever-db2-regular-sections-metadata-v1` adds
`DecodedTable::read_regular_sections_from_catalog(data, catalog_id, table_name,
max_sections)`. It resolves a table through the explicit pinned metadata catalog
or either build-specific `wowdbdefs-70205-v1` or `wowdbdefs-70338-v1` catalog,
then uses the metadata's table/layout identity and reviewed primitive shape.
String schemas select the same bounded text resolver; numeric schemas use
numeric field decoding. This makes the opt-in section reader available to
shape-compatible catalog entries without widening the existing numeric-v1 or
text-v1 APIs.

This path retains the same 2..=16 sections, 16 MiB input, 100,000 direct-row,
100,000 copy, 1,000,000 scalar-value and 64 MiB cumulative UTF-8 policy. It
requires external IDs and no parent; inline-ID schemas reject. TACT/encrypted
data, sparse records, physical catalog data, nonzero catalog offsets, compressed
text and noncontiguous/trailing data remain outside the profile. The default
`DecodedTable::read` is unchanged. Product v4/v5 target-catalog plans call this
reader from `driver::validate_db2` when a DB2 header declares more than one
section; product v1-v3 keep their previous validation path. This activation is
bounded to the profile above and does not admit TACT/encrypted, sparse or
parent-bearing sections. A successful decode is not evidence of native or
real-client parity. No real Forever multisection file has yet been accepted
through this product route, and the text/catalog extension has no pinned native
parity result.

## Empty-text admission regression — 2026-10-13

String-column compression is now checked before the row loop, so zero-record
sections cannot bypass the compression-0 requirement. Private policy structs
also group decoder options without widening public limits or profiles.

The final focused source passes 346 `forever-db2` all-target tests with one
existing ignored case, 41 regular-section tests in release, sixteen static
contract checks and all-target DB2 Clippy with warnings denied. The six new
authored cases cover empty text/compression, numeric and text catalog routes,
global IDs/copies and schema/hash/inline-ID/parent gates. This is Rust synthetic
regression evidence; the earlier numeric native comparison is unchanged, and
text/catalog native or real-client acceptance is still pending.

Retained report: external run `db2-multisection-fix-20261013-nm2hB7/summary.json`,
SHA-256 `7cb27928895da0472771330c8814d9fb5da6ec036288541ee1571fa4788687d0`.
Its earlier static-check and Clippy failures remain retained as failed attempts.
