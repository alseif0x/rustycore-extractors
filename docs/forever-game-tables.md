# Forever GameTable component

The default component acquires the six text files used by RustyCore's two public
Forever GameTable loaders. An explicit `--group catalog` selects the twenty
identities in the pinned C++ extractor. These are separate versioned contracts
within the [full completion plan](forever-completion.md). Neither acquisition
mode establishes full extractor or runtime compatibility.

## Identities and output

The source is `src/tools/map_extractor/System.cpp:1346-1367` at TrinityCore
`02245dcd245e7433e524577656177723d3e4992e`. Shapes come from that revision's
`GameTables.h`; traversal comes from `GameTables.cpp:44-100`.

| Group | Output path | FileDataID | Numeric columns |
| --- | --- | ---: | ---: |
| initial | `gt/BaseMp.txt` | 1391664 | 15 |
| initial | `gt/HpPerSta.txt` | 1391642 | 1 |
| initial | `gt/xp.txt` | 1391661 | 5 |
| spell-values | `gt/SpellScaling.txt` | 1391660 | 24 |
| spell-values | `gt/CombatRatingsMultByILvl.txt` | 1391670 | 4 |
| spell-values | `gt/StaminaMultByILvl.txt` | 1980632 | 4 |

`--group all` is the default closed six-file plan. Either named group selects
exactly its three files. Arbitrary names, IDs, output paths and network options
are not admitted. The target product is `wow_classic_beta`, build 70170 or
70205, with explicit enUS/esES storage selection. Client build is admitted
before output creation and before every read; native reads independently
confirm it. A bounded `.build.info` snapshot must remain byte-identical when
the batch finishes. These checks are not a filesystem-wide atomic snapshot.

```text
forever-gt-extract --client <root> --locale esES --output <new-root> --group all
forever-gt-verify --output <existing-root>
```

Extraction currently requires the explicit `native` feature and pinned
CascLib closure. The verifier has no native dependency. Both run offline;
neither tool executes the RustyCore typed loaders. Standard output is a single
redacted JSON result, except standalone `--help`.

## Parsing and preservation

Files are copied byte-for-byte. No line ending, label, decimal text or float
is rewritten. The pure Rust parser ignores the first column's label, uses
physical row order, and adds the source's unused positive-zero row zero.
Header token count, data column count, trailing tabs, first-CR truncation and
blank/one-token termination follow the source traversal. Early termination is
recorded; remaining file bytes are still hashed and preserved.

For known numeric schemas, admission requires UTF-8 without NUL and finite decimal cells. Numeric values
are parsed through f64 and narrowed to f32; invalid syntax, nonfinite values,
overflow and underflow to zero are rejected. Signed zero and representable
subnormals are retained. This is a conservative admission policy, not general
parity with the C++ Linux `std::stold` conversion and zero fallback. Decimal
rounding differences remain possible and require independent all-cell oracle
acceptance. The module exposes row bits for private comparison and deliberately
does not implement cell-bearing Debug output.

Limits are policy: 4 MiB per file, 24 MiB per v1 batch or 80 MiB per v2 catalog,
100,000 data rows and one million numeric cells per known-schema file, counting
the implicit zero row. Manifests are
limited to 64 KiB and JSON depth eight. Limits are checked before row allocation.

## Publication and verification

The [GT v1 contract](../contracts/forever-gt-contract-v1.json) and
[manifest schema](../schemas/forever-gt-manifest-v1.schema.json) are separate
from DB2 v2. Additional semantic validation enforces the exact plan order,
name/ID/path/column mapping, row/count relationships and inventory digest.

Only a new output directory is allowed. Outputs inside the client, its aliased
Data directory or a Git checkout are rejected. Unix directories are mode0700
and files mode0600. An incomplete marker is created before data files; each
file is fsynced and published exclusively by hard link from its `.part` file.
The manifest is written last. Only a completely successful group removes its
own incomplete marker. This provides atomic publication per file, **not an
atomic directory promotion**. Existing outputs are never replaced or cleaned.

A failed entry leaves the batch partial, even if its other files succeeded.
The verifier rejects the marker, partial manifests, pending files, unknown
paths, symlinks, wrong identities, size/hash differences and incorrect parsed
facts. It independently recomputes SHA-256, raw byte totals, the inventory in
relative-path order, and each file's numeric admission and row facts. Matching
file counts or FNV alone never establishes acceptance.

Exit0 means complete extraction or coherent verified output; exit2 is invalid
invocation/manifest, exit3 unsupported acquisition target, exit4 I/O/publication
failure, exit8 incomplete batch, and exit9 verification mismatch. A successful
verifier establishes output coherence, not build provenance from file contents,
source numeric parity or runtime compatibility. Catalog v2 completeness means
all twenty pinned identities have complete files; numeric facts apply only to
the fourteen source-backed schemas.
Per-file policy/read/parse failures are recorded as failed entries and make the
batch partial/exit8. A global admission, metadata-snapshot or publication error
returns exit4 and retains any incomplete output; it never becomes a successful
partial group. Initial unsupported build admission retains exit3.

## Reader acceptance requirement

`wow_data::forever_game_tables::InitialGameTables::load(root)` and
`SpellValueGameTables::load(root)` already expose the six projections; no
integration change is required. A separate bounded local probe must compare
every f32 bit and physical row, including row zero, against independent parsing
and the Linux source-style oracle. Report only counts, hashes, booleans and
static errors. Freeze sources/binaries, verify input hashes after loading,
and record whether both typed loaders actually executed.

### Accepted GT6 evidence

The local report `gt6-accept-20261011T003319Z-eca4882f.json` in the laboratory's
`reports/acceptance/` records six complete files, 85,794 raw bytes and 15,988
numeric cells including the six implicit zero rows. Both public typed loaders
executed; every numeric bit matched the independent Linux C++ `std::stold`
oracle and the typed projections. The verifier returned exit0, JSON Schema
validation passed, and the recorded source/input/output guards passed. This is
the accepted v1 six-table scope; it does not certify the added catalog v2 files.

## Opt-in pinned catalog v2

```text
forever-gt-extract --client <root> --locale esES --output <new-root> --group catalog
forever-gt-verify --output <existing-root>
```

`all`, `initial`, and `spell-values` retain their v1 meaning and byte format.
Catalog selection creates a manifest v2 with scope `pinned_gt_catalog`, plan
`catalog`, and the same `forever-gt-manifest.json` filename in a new output
root. The verifier dispatches on `manifest_version`, validates that version's
exact schema and inventory, and continues to emit the existing generic
`forever_gt_verification` report format. It never reinterprets a v1 plan as20.

The [catalog facts](../contracts/forever-gt-catalog-v1.json) record exact case,
FileDataIDs, column counts where known, field positions, source line/byte
offsets, the fixed C++ commit and source SHA-256 values. The
[v2 contract](../contracts/forever-gt-contract-v2.json) and
[v2 manifest schema](../schemas/forever-gt-manifest-v2.schema.json) bind that
facts file's SHA-256. Artifact order, identities, modes, paths and counts are
closed by the Rust validator; the JSON Schema also fixes all twenty positions.

| Additional identity | FileDataID | Numeric columns |
| --- | ---: | ---: |
| ArtifactKnowledgeMultiplier | 1582086 | 1 |
| ArtifactLevelXP | 1391662 | 2 |
| BarberShopCostBase | 1391663 | 1 |
| BaseProfessionRatings | 4494528 | unknown |
| BattlePetTypeDamageMod | 1391665 | unknown |
| BattlePetXP | 1391666 | 2 |
| CombatRatings | 1391669 | 32 |
| HonorLevel | 1391671 | unknown |
| ItemLevelByLevel | 2012881 | 1 |
| ItemLevelSquish | 1726830 | unknown |
| ItemSocketCostPerLevel | 1391643 | 1 |
| NPCManaCostScaler | 1391651 | 1 |
| ProfessionRatings | 4492239 | unknown |
| SandboxScaling | 1391659 | unknown |

The original six plus eight additional schemas contain only f32 fields in
`GameTables.h`. They share the unchanged bounded numeric policy, with maximum
column width32 for CombatRatings. The six unknown schemas have no struct or
`LOAD_GT` at the pin. They use `validation_mode: raw`: complete bytes, size and
SHA-256 are preserved, while columns and every row fact remain JSON null. No
UTF-8, numeric format, field type, row count, or typed-reader claim is made for
these files. Even float-looking raw bytes cannot obtain numeric facts through
`catalog::parse_numeric`; that API returns `gt_schema_unknown`.

All twenty files are required for successful catalog acquisition. Any missing,
incomplete, unreadable, oversized or invalid known-schema file makes the batch
partial/exit8 and keeps the incomplete marker; deleting that marker does not
make its partial manifest acceptable. There is no optional-table success,
prefix fallback, additional network path, resume or overwrite. The same
private publication, build admission and metadata-snapshot checks apply.

C++ acquisition uses `OpenFile(FileDataID, CASC_LOCALE_NONE)` at
`System.cpp:1373` after opening storage for the first installed locale. Its
loop logs missing files and continues. Separately, the C++ server requires all
fourteen `LOAD_GT` entries (`GameTables.cpp:120-144`). The v2 contract requires
all twenty complete files to avoid presenting a missing catalog member as
successful extraction. Source shapes are fixed at the pin; live build
availability and independent numeric acceptance remain separate evidence.

`catalog::Table::ALL` exposes source order; `value_columns()` distinguishes
known from unknown schemas. `catalog::parse_numeric()` returns bounded
`NumericTable` facts and private all-cell bits for an independent fourteen-table
oracle comparison. It runs no integration loader. The legacy CombatRatings
reader's column count matches32 but its named indices retain legacy meanings;
it is not used as Forever acceptance. The current local oracle must explicitly
admit32 columns before comparing the additional CombatRatings schema. Its
Linux `std::stold` conversion does not establish Windows `std::from_chars`
parity. The additional eight schemas have no new typed-loader acceptance claim.

### Accepted catalog20 evidence

The local report `gt20-accept-20261011T004112Z-6ad435fc.json` in the laboratory's
`reports/acceptance/` records all twenty complete files. Every one of 22,082
numeric cells across the fourteen known f32 schemas matched the independent
C++ oracle exactly. Both existing typed GT6 loaders also passed their complete
projections. Extraction, verification and schema validation passed, together
with all recorded guards. The six raw-only entries retain their explicit lack
of numeric schema and typed-reader claims; catalog acceptance does not imply
full client extraction or runtime compatibility.

## Explicit Rust local acquisition (GT manifest v3)

`forever-gt-extract` accepts `--backend native|rust-local`, defaulting to native.
Native GT v1 (six tables, or a closed three-table group) and v2 (twenty-table
catalog) retain their existing manifest and report bytes. A binary built without
`native` reports `backend_unavailable` for the default selection before opening
client or output paths.

Rust local is an explicit choice. It requires both `--index-layout
contiguous|paged` (uniform across sixteen buckets) and `--root-layout
headerless-18125|header-30080|header-50893-v1|header-50893-v2`. An optional
`--max-file-bytes` reduces the per-file limit from 4 MiB; zero and increases are
rejected. Native rejects all three profile options. There is no automatic layout
selection, native fallback, network access or prefix-read fallback.

The CLI opens one `acquisition::Session` before creating output and reuses it for
the entire plan. GT parsing, raw-byte preservation, total budgets and output
publication remain the existing contracts. `InputChanged` aborts the whole batch,
retains the incomplete marker, publishes no manifest and prevents further reads.
Bootstrap failures precede output creation. Per-file failures retain the existing
partial/exit-8 behavior; global post-bootstrap failures use exit 4.

Rust local emits manifest version **3** with required `acquisition` provenance.
The existing scopes discriminate `explicit_gt_batch` (all/initial/spell-values)
from `pinned_gt_catalog` (exactly twenty identities); the plan is never inferred
from success counts. The provenance records `local-v7-ckey-root-nz-v1`, layouts,
all effective logical budgets, full CKey MD5 verification and EKey **prefix9**
validation. Top-level `max_file_bytes` is the component's 4 MiB ceiling;
`acquisition.limits.file_bytes` is the effective, possibly smaller limit.

`forever-gt-verify` accepts v1/v2/v3 offline. New provenance must match the exact
common profile contract, and every complete artifact must fit its effective
file limit. Verification then applies the unchanged, version-specific identity,
path, inventory, SHA and numeric reparse rules. This checks the recorded profile's
shape; it cannot authenticate the historical source of coherently replaced raw
bytes without reopening the client. The schema is self-contained and performs no
remote reference resolution. Its embedded common profile is identified by hash
in `contracts/forever-gt-contract-v3.json`.

The injected Rust API is `batch::extract_with_profile` and
`catalog_batch::extract_with_profile`, taking `Option<acquisition::Profile>` before
the reader closure. `None` preserves the native contract. The caller is responsible
for supplying a reader that actually enforces its declared profile; the CLI uses
`Session` for this purpose. New synthetic cases cover both plans, native byte
preservation, profile tampering, reduced caps, missing provenance and global
input-change termination. Real Rust-backend acceptance is recorded separately.

### Accepted Rust local CLI evidence

The local report `rust-cli-accept-20261011T012203Z-2aa558be.json` in the
laboratory's `reports/acceptance/` records successful GT v3 acquisition for both
`all` (six files) and `catalog` (twenty files). Every artifact matched the
previously accepted complete pinned C++ bytes. Schema validation and the
offline verifier passed for both scopes. The independent comparison of all
22,082 f32 cells across fourteen known schemas and both complete typed GT6
loaders were rerun and passed. Raw-only catalog members retain their explicit
lack of numeric-schema and typed-reader claims.

For each scope, a one-byte acquisition cap produced exit8 and a valid partial
manifest without acquired assets. Three profile-tampering cases per scope were
rejected. Native-backend unavailability and invalid layout/cap options failed
before client/output I/O. Source, binary, client and reference guards passed;
the bounded acceptance recorded no network access. This establishes the
recorded offline Rust acquisition scopes, not full extraction or additional
runtime compatibility.
