# Pinned Forever metadata catalog

The optional `pinned-metadata-v1` catalog records all 1,126 declarations in
TrinityCore `DB2Metadata.h` at
`02245dcd245e7433e524577656177723d3e4992e`. It records facts about table identities
and field shapes. It does not establish CASC availability, extraction parity or
RustyCore reader/runtime acceptance.

The existing 56 reviewed table mappings and the default two-table extraction
plan are unchanged. New tables require explicit API, diagnostic or batch opt-in.
The batch selector publishes a separate v3 contract; ordinary runs retain v2.

## Source and regeneration

- Source: `src/server/game/DataStores/DB2Metadata.h`.
- Git blob: `d06b97bad72ca2aecad3987ce9c348ac6055e0db`.
- SHA-256: `897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def`.
- Source length: 1,014,924 bytes.
- Attribution: TrinityCore contributors; upstream declarations are under
  GPL-2.0-or-later. The independently written generator and API are under the
  repository's GPL-3.0-or-later license. No upstream implementation is imported.
- Outputs: `contracts/forever-metadata-v1.json` and
  `crates/forever-db2/src/records/catalog_generated.rs`.

Run against an existing local reference repository:

```sh
python3 -B tools/generate_forever_catalog.py --reference /path/to/reference
python3 -B tools/generate_forever_catalog.py --reference /path/to/reference --check
```

The generator reads the immutable Git object, verifies its blob and SHA-256,
parses the entire declaration grammar, and rejects unparsed remainder, malformed
fields and duplicate table names or file IDs. It makes no network request and
does not read client files. Both output files are deterministic; no source path
or generation timestamp from the local machine is embedded.

The JSON preserves all source field types, localization distinctions, array
counts and signedness, including the parent declaration. The Rust mapping uses
only the first `FileFieldCount` fields as physical columns. Appended byte, short
and int parents occupy 32-bit record storage, as specified by pinned
`DB2Meta.cpp::GetRecordSize` and `DB2FileLoader::FillParentLookup`. The five signed
byte parent declarations retain their raw metadata and use the existing signed
32-bit appended value representation.

## API and limits

```rust
use forever_db2::{catalog, CharacterTable, DecodedTable};

let metadata = catalog::find("ItemSparse").expect("pinned metadata");
assert!(metadata.shape_compatible());
let table = CharacterTable::from_catalog("ItemSparse")?
    .expect("known table");
let records = DecodedTable::read(&synthetic_or_local_bytes, table)?;
```

The example assumes an enclosing function with compatible error handling.
`CharacterTable::named` continues to resolve only the existing 56 reviewed
tables. `from_catalog` preserves those variants and their identity gates,
including the two known table hashes. Direct construction through a public
catalog handle also preserves those gates. Unknown names return `Ok(None)`;
known unsupported shapes return `unsupported_catalog_schema` with explicit
reasons. They cannot silently become metadata-only success.

`DecodedTable::read_with_direct_u32_values(data, table, physical_field)` returns
the usual final decoded table together with a sorted, deduplicated `BTreeSet<u32>`
from the original direct records. The selector must identify a physical scalar
unsigned 32-bit field; other types, arrays and appended parents are rejected,
including for empty files. Zero is retained. Values are captured before parent
lookup, copies and inline-ID rewriting, so a copy that replaces a direct row
cannot erase its original dependency. The set is bounded by the existing
100,000-direct-record limit. Both results are returned only after every field
and the complete file pass the normal decoder checks; an error yields neither
partial rows nor a partial set. `read` and `rows` retain their existing behavior.

For the pinned `CinematicCamera` metadata, FileDataID is physical field 3
(flattened value slot 5). This matches the direct-record scan and `std::set`
in `map_extractor/System.cpp:394-414`. The API does not acquire camera files or
establish camera-output compatibility by itself.

The catalog exposes 1,126 shape-compatible entries. Five declarations exercise
the additional representations completed after the first 1,121-shape foundation:

| Table | FileDataId | Additional value representation |
| --- | ---: | --- |
| BattlePetEffectProperties | 801580 | Six string pointers |
| InitiativeReward | 6880280 | Signed 64-bit integer |
| LiquidType | 1371380 | Six string pointers |
| TraitCondAccountElement | 5890759 | Signed 64-bit integer |
| UiWidgetVisTypeDataReq | 2740907 | Unsigned appended 32-bit parent |

`RecordGetString` at pinned `DB2FileLoader.cpp:800` makes each string-array
pointer relative to its own four-byte slot. Raw 64-bit signed integers are
preserved as `Value::Signed64`; the canonical diagnostic text still uses `i:`.
Both signed and unsigned 64-bit CommonData are rejected: the native code copies
only four bytes into an uninitialized eight-byte value, leaving no defined
upper half. Other compressed 64-bit modes remain outside the bounded decoder.
The unsigned byte parent declaration retains its native 32-bit appended storage
and is represented by an unsigned value, including values above 255.

The local diagnostic and independent Python oracle accept the same explicit
option, in either argument order:

```sh
db2_dump --schema-catalog pinned-metadata-v1 --table LiquidType --file /private/run/synthetic.db2
python3 -B tests/forever/db2_oracle.py --schema-catalog pinned-metadata-v1 --table LiquidType --file /private/run/synthetic.db2
```

The oracle verifies the SHA-256 of the complete factual JSON before constructing
any additional schema. It maps source types independently of the generated Rust
Kind slices. These commands produce decoded values: keep real-client invocations
and their output in the private external run directory, outside Git and model
context.

Compatibility here covers value shapes only. The existing WDC5 decoder gates
remain: bounded minimal empty files or one unencrypted section;
regular files and the separate sparse subset below use distinct record paths;
16 MiB input limit; checked columns, IDs, strings, copies and parent blocks.
Each decode also has a fixed **64 MiB cumulative UTF-8 payload budget**, shared
by all direct string fields, string-array elements and applicable record copies.
Repeated pointers consume their full string length each time. Copies that
overwrite an existing row consume budget without refunds; omitted copies with
zero or missing sources consume none. The check precedes string allocation or
row cloning and fails with `record_policy_limit`. This is a resource policy,
not a WDC5 format constraint or a bound on total process memory. Results below
the budget are unchanged. The independent Python oracle charges the same UTF-8
byte lengths, including logical copies of its immutable strings; it searches
for terminators in the original buffer without copying the remaining block.
Sparse variants outside the subset below, multiple sections, encrypted sections,
incomplete local files, and unsupported compression/field combinations still fail. Narrow inline IDs
retain their existing copy restriction. No keys, network fallback, prefix
fallback or general runtime-acceptance claim is introduced.

## First numeric sparse subset

The decoder has a separate sparse path selected by header flag bit 0, as in
pinned `DB2FileLoader.cpp:1852`. It admits only scalar unsigned 8/16/32-bit and
signed 32-bit fields, separate catalog IDs, no parent, and zero or one
unencrypted section. Strings, floats, 64-bit fields, arrays, signed bytes or
shorts, inline IDs and every parent variant remain excluded from this path.
The regular decoder and the existing table-identity checks are unchanged.

The sparse path follows the reviewed `DB2FileLoaderSparseImpl` value and copy
semantics and adds conservative validation:

- Primitive `UnusedBits` determines sequential field sizes; primitive `Offset`
  and regular column bit offsets do not determine sparse values. The subset
  requires exact one/two-byte slots for byte/short fields, and supports checked
  1–32 meaningful bits for integer fields (`RecordGetVarInt`, `GetFieldSize`,
  `CalculateAndStoreFieldOffsets`, lines 1487–1570).
- Catalog IDs precede copies and packed six-byte offset/size entries. The later
  conventional ID table is consumed completely but ignored for logical IDs,
  matching `LoadCatalogData` and `SetAdditionalData` (1020–1050). Catalog IDs
  below `MinId`, including zero, are not repaired. Duplicate catalog IDs fail
  closed here; the C++ implementation overwrites the earlier index entry. IDs
  beyond the declared maximum also fail closed.
- Every record range stays inside the section payload. Per-record catalog sizes
  govern reads; trailing record padding is allowed. This subset requires
  `section.file_offset == metadata_end` and an exact file end after the complete
  catalog and both ID tables. These are stricter sparse acceptance conditions:
  the pinned C++ whole-file size gate applies only to its regular path. All
  these checks complete before any result is returned.
- Copy bounds use the sparse `Header.MaxId + 1` formula from `AutoProduceData`
  (1064) and `AutoProduceRecordCopies` (1297–1320). This implementation rejects
  `MinId > MaxId` and overflow of `MaxId + 1` as conservative bounds checks; it
  does not reproduce native overflow or unchecked indexing. Ordered copies skip source
  zero, missing sources and either ID outside that extent; existing self-copies
  fail closed. A copied row retains every physical field because its synthetic
  ID is not a file field. Direct-record collection occurs before copies and
  is returned only after full validation.
- Record and catalog counts must agree. Primitive metadata is complete; storage
  metadata is either absent or one complete `None` entry per file field without
  additional payload. Common/pallet/string/parent payloads are excluded. The
  existing 16 MiB input, 100,000 direct records, 100,000 copies and 1,000,000
  retained scalar-value limits apply. This subset contains no decoded strings.

Synthetic fixtures independently transcribe `ConversationLineMeta` (FDID
1260801, layout `0xAA1E7988`, nine scalar fields) and test numeric limits,
non-byte integer widths, ordered copies, skipped IDs, zero/near-maximum IDs,
padding, incomplete ranges, full-file validation and the explicit exclusions.
The Python oracle implements this path independently; generated Rust field
slices are not used to construct its fixtures or goldens.

This does not resolve schema mismatch in `ChrRacesCreateScreenIcon`, nor the
three `CollectableSource*Sparse` tables with appended parents: pinned sparse
`AutoProduceData:1053–1057` explicitly rejects that parent shape even when
empty. It does not add incomplete-file recovery, encryption or multiple
sections. RustyCore's current `wow-data/src/wdc4/reader.rs` rejects sparse WDC5
at its header gate, so decoder comparison or v3 extract/verify evidence does
not establish generic or typed RustyCore reader/runtime acceptance. No change
to that integration reader is part of this work.

A current-source diagnostic recheck on 2026-10-12 confirms that the
`ChrRacesCreateScreenIcon` mismatch is still present in this checkout, rather
than only in the frozen 1,126-table survey binary. The complete retained
build-70205/esES file has five physical fields and a different layout hash from
the pinned catalog declaration, which has four physical fields; the current
`db2_dump` returns `unsupported_table_schema`. The same decoder recheck still
returns `unsupported_record_variant` for the three `CollectableSource*Sparse`
tables, while it accepts the retained `ConversationLine` input. The private,
payload-free summary is
`current-db2-dump-failure-spotcheck-20261012T2005.json` in the external
laboratory's `reports/acceptance` directory. This rechecks one of eleven
schema failures and four of six variant failures; the other frozen-survey
failures have not all been replayed against this source revision.

## Explicit Rust-local acquisition and DB2 v4

`forever-db2-extract` keeps native acquisition as its default. Native reviewed
and catalog output remains manifest v2/v3 with report version 1. A binary built
without the native feature rejects that default with `backend_unavailable`
before opening the client, manifest or output; it does not change backends.

The explicit Rust backend uses one `acquisition::Session` for the whole batch:

```text
forever-db2-extract --client <client-root> --locale enUS \
  --output <new-private-directory> --backend rust-local \
  --index-layout contiguous --root-layout header-50893-v2 \
  --manifest <name-tab-id.tsv> --schema-catalog pinned-metadata-v1
```

Both layout options are required for `rust-local`. Index layout is `contiguous`
or `paged`, applied explicitly to all 16 buckets. ROOT layout is one of
`headerless-18125`, `header-30080`, `header-50893-v1`, or `header-50893-v2`.
The example's layouts are explicit operator choices; the CLI does not detect or
retry another layout. Optional `--max-file-bytes` accepts decimal 1 through
16,777,216 and only lowers the effective per-file ceiling. Native rejects
layout and file-limit options. The existing batch limits remain 256 entries
and 256 MiB, with each read also bounded by its remaining batch budget.

The backend does not change schema selection. Without a catalog selector the
default is still ChrClasses/ChrRaces; an explicit list retains the previous
reviewed-schema or metadata-only policy. An unknown table can therefore publish
complete bytes and metadata with `table_schema_validated=false`,
`record_data_decoded=false`, and `decoded_rows=null`. Catalog selection still
requires an explicit manifest, exact names/IDs, fixed catalog provenance and
full schema/record decoding for every successful artifact.

Rust-local output uses the separate
`schemas/forever-manifest-v4.schema.json` and
`contracts/forever-contract-v4.json`. `schema_selection` distinguishes
`reviewed` from `catalog`; mandatory `acquisition` records
`local-v7-ckey-root-nz-v1`, every effective store limit, explicit layouts,
offline mode and complete CKey MD5 validation. EKey validation is honestly
recorded as `prefix9`. Output does not claim authentication of its remaining
seven bytes. The reusable acquisition schema is
`schemas/forever-acquisition-v1.schema.json`.

The session opens before output creation. A real incomplete marker is published
before the first read and removed only after all files, report and manifest
publish successfully. An input-change error or changed observed build aborts
immediately, leaves the marker and existing artifacts, and issues no subsequent
reads. Other entry failures may continue but leave an incomplete run and the
marker. Rust-local stdout/`report.json` uses report version 2, including its
acquisition profile; global errors after valid selection also disclose that
profile without client paths. Failed bootstrap creates no output directory.

`forever-db2-verify` dispatches v2/v3/v4 explicitly. V4 validates the closed
acquisition profile, compares its file ceiling to options, checks artifact and
batch limits, and requires the companion report's profile, version, build and
locale to agree. It performs the existing hash, metadata and claimed-row checks
without opening CASC or the client. Historical metadata-only claims are not
upgraded into record claims. Its own result remains report version 1 because
that response shape is unchanged. The v4 JSON Schema embeds its acquisition
and catalog identity definitions and uses only local fragment references; it
needs no schema registry or network retrieval. Drift tests compare those
definitions with the repository source schemas. The Rust verifier uses its
bounded in-tree validator and performs no schema fetch. Both manifest and
companion report remain limited to 4 MiB; the shared JSON parser rejects more
than 64 nested arrays/objects in a linear scan before recursive parsing.

## Evidence and source-list difference

### Typed DB2 sources for the terrain consumer

`forever_db2::terrain_sources` supplies four closed map-extractor readers:
`read_map`, `read_liquid_types`, `read_liquid_materials`, and the subsequently
added `read_liquid_objects`. They return immutable
`SourceRecords<T>` snapshots with `direct_records()` and `copies()` in physical
order. Each direct record has an ID and typed fields; each copy has `new_id`
and `source_id`. No callback or partial snapshot is exposed during validation.

| Reader | Pinned identity | Physical projection |
| --- | --- | --- |
| Map | 1349477 / D43AFAC3 | 0 Directory, 1 MapName, 21 signed WdtFileDataID |
| LiquidType | 1371380 / D1ECEEC9 | 3 unsigned SoundBank, 14 unsigned MaterialID |
| LiquidMaterial | 1132538 / 98E5D7AA | 0 signed32 Flags bits, 1 signed8 LVF |
| LiquidObject | 1308058 / CB0D39E8 | 2 signed16 LiquidTypeID, retained as u16 bits |

Map fields are `wdt_file_data_id: i32`, `name: String`, and `directory: String`.
Liquid type fields are `sound_bank: u8` and `material_id: u8`; material fields
are `flags_bits: u32` and `lvf: i8`. The latter preserves the source's assignment
of `GetUInt8("LVF")` into an `int8`. The authoritative facts are pinned
`System.cpp:54-79,275-389`, `DB2Metadata.h:13983-14069,14538-14589`,
`DB2LoadInfo.h:3778-3859,3934-3972`, and
`extractor_common/ExtractorDB2LoadInfo.h:109-136`.
LiquidObject follows `System.cpp::ReadLiquidObjectTable` and
`DB2Metadata.h::LiquidObjectMeta` at the same pin. All five physical fields are
validated; unused float bit patterns need not be finite. Its ordered liquid
copy replay includes missing-source insertion, self-copies, chains and zero.
The terrain runner loads and replays this required startup dependency, then
releases its unused lookup: the pinned ADT converter does not consult it.
Two new Rust tests cover signed bits/copy replay and empty/complete-file policy;
the existing identity/variant/tail matrix also includes this reader. The 296-test
DB2 library suite passes. This adds no populated real LiquidObject acceptance.

The internal regular scan validates every physical field, including unused
strings, array elements and numeric storage. Its metadata, exact file extent,
compression, UTF-8 and resource checks are shared with the general decoder.
The closed readers admit only the existing minimal empty profile or one
unencrypted regular section, with no parent. They do not add sparse, encrypted,
multiple-section or extended empty-file support.

Map's selected Directory and MapName pointers must be nonzero, including rows
whose WDT ID is zero. Pinned `DB2FileLoader.cpp:798-803` returns `nullptr` for a
zero string offset, while `System.cpp:294-295` assigns both selected strings
directly to `std::string` before WDT filtering. That undefined native case is
rejected as `invalid_record_string`. A nonzero pointer to a NUL terminator is a
valid empty string. Unselected strings, the generic decoder and camera API keep
their existing null-to-empty behavior.

`materialize(self)` consumes the snapshot and applies the specific consumer:

- **Map:** preserve all direct rows and their order. An ID lookup points to the
  last direct row with that ID. Copies append rows using only this fixed lookup;
  copy-created IDs never become sources. Existing destinations, self-copies and
  a present source zero are valid. Finally remove rows with WDT exactly zero;
  negative signed WDT values remain intact.
- **Liquids:** direct duplicates overwrite earlier values. Each copy first
  inserts a zero-valued source when absent, then assigns that value to its
  destination. The inserted source remains present. Zero, missing sources,
  self-copies, chains and overwrites therefore follow `map[new] = map[source]`.
  The returned `BTreeMap` provides deterministic key order.

These are `Load/GetRecordCopy` consumers, not `AutoProduceData` consumers.
They retain duplicate direct IDs and full-u32 copy operands without allocating
by ID. They do not enforce the latter materializer's header-MaxId index extent,
zero-source skip or self-copy rejection. Separate ID-table repair remains
`MinId + ordinal` with checked overflow. The generic `DecodedTable::read`,
`rows()` and direct-camera API retain all their previous policies.

Input limits remain 16 MiB, 100,000 direct records, 100,000 copy operations and
one million decoded direct scalar values. Map replay is bounded to 200,000 rows
before filtering, and liquid replay to 300,000 keys, counting both a missing
source and a new destination. The 64 MiB cumulative UTF-8 budget includes all
decoded strings, even unselected fields, then every Map copy's name/directory.
Filtering or replacement never refunds bytes. `decoded_utf8_bytes()` reports
the initial charge. Every error returns no partial replay result.

For the original Map/LiquidType/LiquidMaterial group, the independent Python oracle exposes `read_terrain_sources` and
`materialize_terrain`, using immutable tuples and separately transcribed
operations. This API adds no DB2 acquisition, CLI selection, MAPS container,
terrain integration or runtime acceptance claim.

The private diagnostic example `terrain_sources` accepts exactly
`<Map|LiquidType|LiquidMaterial> <input.db2> <output.json>`. It reads a regular
file of at most 16 MiB, checks its length and identity before and after reading,
and completes decoding and replay before creating the exclusive output file.
Its JSON contains the ordered direct records, raw copies and materialized
records. Stdout reports only status, counts or a static error code. The output
contains client-derived data and must stay in the private external run directory.

`terrain-sources-20261011T013829Z-455396ac.json` records **89 synthetic positive
cases and 36 negatives**, plus two complete local files. LiquidType had 53
direct records, no copies and 53 materialized records; LiquidMaterial had four
direct records, one copy and five materialized records. Rust and Python agreed
on every projected direct value, copy operand and materialized value. Source,
executable and prior-input guards passed; no network was attempted. The real
Map file was incomplete and was not invoked. The selected-string null policy
above is covered synthetically; a valid pointer to an empty string is accepted.
This comparison did not execute a C++ typed loader: the consumer semantics
were checked against pinned source, with independent Python replay. It does
not establish terrain integration or runtime acceptance.

### Bounded CLI acquisition evidence and survey exclusions

`rust-cli-accept-20261011T012203Z-2aa558be.json` records five bounded plans and
61 native/Rust-local artifact comparisons: reviewed DB2 (2), catalog DB2 (22),
GT (6 and 20), and the camera bundle (11). Cap, incomplete-input, early-failure
and profile-tamper negatives passed; source guards stayed true and no network
was used. This acceptance predates the terrain source API above.

Private triage in `spell-missile-motion-string-diagnostic-20261011.json` found
one non-truncated invalid UTF-8 value in SpellMissileMotion's second string
field among 66 records. Its first field had 66 valid UTF-8 values, the second
65; bounds, terminators and the C++ relative-offset assertion had no failures.
The current UTF-8 rejection is retained: no lossy or alternate-encoding fallback
and no schema change is inferred.

SceneScriptText is a sparse file with two string fields and remains outside
the numeric sparse profile. SpellMissile is a regular empty file with zero
sections but 60 bytes of primitive metadata and 360 bytes of column metadata
(624 bytes total). Its extended empty layout remains outside the current
minimal empty profile. These are explicit exclusions, not claims that the
pinned schema identities are incorrect.

### Completed catalog survey with frozen executables

The offline survey examined all **1,126 catalog entries** in 36 bounded
batches. **1,023 files completed and matched the independent Python oracle**,
covering **719,466 decoded rows**. The other 103 entries remained errors:

| Error category | Entries |
| --- | ---: |
| Local file incomplete | 82 |
| Record policy limit | 3 |
| Unsupported record variant | 6 |
| Unsupported table schema | 11 |
| Invalid record string | 1 |

The aggregate redacted report is
`catalog1126-survey-20261011T004624Z-a6ba728c.json` in the external laboratory's
`reports/acceptance` directory. Its persistent service finished successfully
and became inactive; client guards stayed true and no network was used. Service
completion means that the survey finished recording its results. Failed
entries and their incomplete batch results remain failed.

The survey used frozen executables from **before the numeric sparse support
and cumulative decoded-text budget changes**. Its coverage is not retroactively
increased by later fixes or by the separate ConversationLine acceptance below;
nor does it validate the newer text budget or terrain source readers. It is
bounded acquisition and decoder comparison evidence, not typed-loader or
runtime acceptance, full client extraction, or admission of unsupported inputs.

The coordinator's offline comparison of 21 additional locally complete DB2s
matched all canonical Rust/Python output bytes: 5,302 decoded rows, including
six empty tables. The redacted report is
`catalog21-accept-20261011T003802Z-8d87411a.json` in the external laboratory.
Source, executable and input hashes remained stable, with no network activity.
This is local decoder comparison evidence. It did not upgrade previous
metadata-only manifests and does not establish typed-loader/runtime acceptance.

The subsequent explicit catalog v3 batch passed offline extraction, manifest
schema validation and verification for the same 21 files and 5,302 rows. Input
hashes and canonical decoded-output hashes matched the earlier diagnostic run;
all recorded guards passed. The redacted report is
`catalog-v3-accept-20261011T004108Z-aa77d0a1.json` under the external laboratory's
`reports/acceptance` directory. This validates that bounded v3 batch and its
independent artifact verification; it does not extend the evidence to other
catalog entries, typed loaders or runtime behavior.

The bounded sparse ConversationLine run passed local extraction, v3 output
verification and the independent Python comparison for **558 decoded rows**.
The redacted report is `sparse-conversation-accept-20261011T011040Z-24845f24.json`.
This evidence applies to that complete sparse input and does not change the
RustyCore reader exclusion documented above. Separate local-store acceptance
reported 57 verified complete files and two explicit incomplete local inputs
in `local-store-accept-20261011T011040Z-c84f6f43.json`; incomplete inputs were
not treated as successful reads. These reports precede the CLI v4 integration
and do not by themselves establish v4 CLI or runtime acceptance.

Synthetic checks exercise all 1,126 admitted shapes with complete empty files,
regular rows, raw float bits, numeric limits, UTF-8 text, separate/inline IDs,
applicable chained copies and optional parents. Every physical field of all 56
reviewed schemas is compared in Rust and against the independent Python oracle.
Parser tests cover malformed input, duplicate identity, unsupported shapes,
remainder rejection and reproduction from the verified local Git object.
Existing hand-authored decoder goldens remain the independent behavioral checks.
Passing these checks is synthetic evidence, not real-data or server acceptance.

The same pinned `DBFilesClientList.h` contains 1,132 entries. The difference is
seven list entries without metadata and one metadata entry absent from the list;
it is not six missing metadata declarations. All shared names have matching IDs.

| Only in DBFilesClientList | FileDataId |
| --- | ---: |
| Cfg_DatacenterLocality | 7928017 |
| Cfg_SuperDistrict | 7929271 |
| AvailableSuperDistrict | 8208798 |
| SuperDistrictSet | 8279681 |
| SuperDistrictSetCollection | 8279682 |
| SuperDistrictTransfer | 8279874 |
| SuperDistrictSetXAvailableSD | 8281618 |

`CooldownViewerItemSpell` (7939216) is present only in metadata. No schema is
invented for any of the seven missing declarations.

## Explicit catalog batch contract v3

The optional batch mode requires both arguments:

```sh
forever-db2-extract --client /local/client --locale enUS --output /private/new-run \
  --manifest /private/plan.tsv --schema-catalog pinned-metadata-v1
forever-db2-verify --output /private/new-run
```

Each plan line is an exact, case-sensitive catalog name, a tab, and its decimal
FileDataId. Unknown names, incorrect IDs, duplicate entries and oversized plans
are rejected before creating the output directory. A catalog entry must pass
schema validation and record decoding before its `.db2` file is published.
There is no metadata-only success path in this mode.

`contracts/forever-contract-v3.json` and
`schemas/forever-manifest-v3.schema.json` define the separate manifest version.
Its scope is `explicit_db2_catalog_batch`. The top-level `schema_catalog` records
the fixed ID, source commit, Git blob, source SHA-256 and complete factual JSON
SHA-256; `options.schema_catalog` records the selected option. Entry count,
format list and artifacts must agree. Names and IDs must belong to the pinned
catalog, and each complete artifact must claim validated schema and decoded
records. The existing limits remain 256 entries, 16 MiB per file and 256 MiB per
batch; verifier loading observes the same v3 byte budget.

The verifier dispatches on manifest version before interpreting the contract.
Version 2 retains its prior semantics, including metadata-only artifacts and the
ordinary two-table default. Version 3 requires exact provenance and re-decodes
every complete artifact using explicit catalog resolution, preserving the 56
reviewed identity gates. Unknown future versions fail with
`MANIFEST_SCHEMA_INVALID`. Previous v2 reports are never automatically promoted.

## Build 70205 definitions catalog

`wowdbdefs-70205-v1` is a second, explicitly selected catalog for the exact
WoW Classic Beta client `1.60.1.70205`. It contains 1,161 DBD table declarations
from `wowdev/WoWDBDefs` commit
`674636522d51c010f250ad1b5fe549ef6bc8eb0a`, pinned by manifest blob
`8a5d8122f312ae6a9564e8286fdc491831fecc22` and SHA-256
`4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265`.
Attribution and the CC BY-SA 4.0 terms are recorded in
`contracts/WOWDBDEFS-LICENSE.md`. The generator is
`tools/generate_wowdbdefs_catalog.py`; its checked-in outputs are
`contracts/wowdbdefs-70205-v1.json` and
`crates/forever-db2/src/records/wowdbdefs_70205_generated.rs`.
Regenerate or check these files against an existing pinned checkout with:

```sh
python3 -B tools/generate_wowdbdefs_catalog.py --reference /path/to/WoWDBDefs --check
```

The independent Python DB2 oracle can select this catalog for local byte-for-
byte decoder comparisons:

```sh
python3 -B tests/forever/db2_oracle.py --schema-catalog wowdbdefs-70205-v1 \
  --table ChrRacesCreateScreenIcon --file /local/ChrRacesCreateScreenIcon.db2
```

The oracle loads the pinned JSON facts directly, normalizes field types in
Python, and gates the target table hash plus every declared layout hash. This
adds a comparison path; it does not itself constitute decoder acceptance.

This catalog does not replace `pinned-metadata-v1`, change the default plan, or
authorize use with build 70170. Use it only with an explicit manifest and the
matching selector:

```sh
forever-db2-extract --client /local/client --locale enUS --output /private/new-run \
  --manifest /private/plan.tsv --schema-catalog wowdbdefs-70205-v1
forever-db2-verify --output /private/new-run
```

Before this catalog was added, the bounded build-70205 local survey had 1,023
complete DB2 artifacts. Their parsed headers matched the corresponding target
DBD declarations in all 1,023 cases: no missing definitions and no table-hash,
layout-hash, or field-count mismatches. This supports the identity of the
target catalog against those local headers. It does not establish record
decoding parity for every declaration, make incomplete inputs complete, run
typed RustyCore loaders, or establish extraction/runtime compatibility. The
catalog's Rust verifier binds manifests to the exact catalog provenance and
re-decodes complete artifacts using only that catalog's field facts; it also
rejects build 70170.

### Local decoder comparison — 2026-10-12

The independent Python oracle now reads the catalog's explicit `decode_kinds`
for scalar widths and array shapes. Treating every DBD `Int` as 32 bits caused
a real `ChrRaces` mismatch because the selected DBD definition includes
explicit 8-bit fields. With that correction, Python and Rust's standalone
`db2_dump` produce byte-identical canonical dumps for three complete local
build-70205/esES files: `ChrClasses` (9 rows), `ChrRaces` (58 rows), and
`CinematicCamera` (10 rows). Dumps remain in a private external run directory
and are not checked into Git.

The Rust-local v4 extractor also acquired the same three files from the
installed 70205/esES client using explicit contiguous indexes and ROOT
50893-v2. All three schemas and record sets were accepted (9, 58, and 10 rows);
the offline v4 verifier accepted all three artifacts, and their raw bytes match
the earlier accepted live package. The path-free run is retained under the
external private run name `db2-catalog-extract-70205-esES-three-20261012`.

This acceptance covers only those three unencrypted DB2 files. It does not
rewrite the earlier failed native `ChrClasses` manifest, invoke RustyCore typed
loaders, or establish all-schema, encrypted, multi-section, sparse, or runtime
compatibility. The required encrypted `Map`, `SkillLineAbility`, and
`GameObjectDisplayInfo` files remain unavailable without their target-build
TACT keys.
