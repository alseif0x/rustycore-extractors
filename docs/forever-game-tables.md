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


## Explicit build-70338 GT catalog (manifest v4)

The standalone GT CLI admits build 70338 only with the explicit
`--profile gt-catalog-70338-v1 --group catalog --backend rust-local` selection
and both acquisition layouts. The profile pins the existing twenty-file
catalog and its fingerprint; it adds no discovery or automatic layout choice.
The default remains six files with the native backend for builds 70170/70205.
Legacy manifest versions 1–3 now explicitly reject 70338 rather than inheriting
backend-wide build admission. Their admitted-build output bytes remain covered
by authored golden vectors and comparison of the unchanged renderer functions.
These checks do not claim execution of an older binary.

The new `catalog_batch::extract_with_contract` API requires a typed acquisition
profile. Its injected reader remains the caller's responsibility; provenance
alone does not authenticate a supplied closure. Component admission precedes
callbacks and output creation. The CLI first checks backend availability, then
bounded build metadata and the component contract, then opens one Session. A
changed Session build is fatal before output creation.

Successful extraction requires all twenty complete files: fourteen numeric
schemas and six raw-only entries. Limits remain 4 MiB per file, 80 MiB per
catalog, 100,000 data rows and one million numeric values including row zero
per numeric file, and a 64 KiB/depth-eight manifest. File-budget overrides only reduce limits.
Missing or invalid files retain partial/exit-8 semantics; observed input changes
are globally fatal. No resume, overwrite, available-prefix success, network
option or directory-wide atomic promotion is added.

The independent verifier accepts version 4 only with build 70338, the exact
component profile, catalog and Rust-local acquisition shape. It checks the full
output tree, hashes and numeric facts offline. Coherent replacement of data and
manifest cannot be detected as a change of historical client origin without
reopening that client. During Rust-local acquisition, CKey verification uses
full MD5 and EKey validation remains prefix9. Six raw-only files gain no numeric or typed-reader claim.

### Source and synthetic validation

Functional commit `70f4b03caafb8349ae74bf45da9628066bebf8e8` contains exactly the
reviewed source bytes. Each Linux debug/release workspace run passes 2,865 tests
with 26 ignored across 43 targets, including 62 GT and four CLI cases. Four
formal JSON cases, 333 source-contract methods, 162 distribution methods/one
platform skip, all-target Clippy and formatting pass. Distribution checks do
not create a new bundle. The initial fixture-location rejection and setup typo
remain failed attempts; the corrected validation is separately recorded.

The four producer/verifier binaries were compiled against baseline `a2ef480`
plus the inventoried source delta before the later functional commit; their
binding to that commit is exact source-byte equality, not a claim that Cargo
observed the later commit. Validation is source/synthetic only at this checkpoint;
real build-70338 catalog and typed-loader evidence require a separate campaign.
Product build-70338 admission remains DB2-only. This component increment does
not establish full Linux extraction, geometry, installed runtime or a new
reproducible package. Windows development remains deferred.

### Fresh build-70338/esES real acceptance — 2026-10-14

The single frozen real campaign acquires all twenty complete files / 138,537
bytes (maximum 41,518). Raw public-Session acquisition and all twenty strict
pinned native CASC reads match complete bytes and native CKey MD5 before any
GT numeric consumer. Both fresh production CLI profiles extract the same
complete catalog; separate offline verification, self-contained v4 JSON Schema
validation and complete output inventories pass.

The fourteen numeric tables contain 5,086 physical data rows, plus fourteen
implicit zero rows. All 22,082 f32 cells match the independent authored C++
Linux `std::stold` source-style oracle in both profiles. This executes neither
original `GameTables.cpp` nor its full server initialization. Both actual public
RustyCore loaders execute their complete six-file projections, including row
zero and one-past-end checks. The production loader modules compile in a minimal external crate with an
explicit shim. All four retained production/test files match integration commit
`544721324cf0a47553caae1ea0299a0ff2917ce7`; copied test-file provenance does not
imply execution of those unit tests. This is not execution of the whole
`wow-data` crate or server. Six raw-only members retain byte-identity
coverage without invented numeric schemas.

These are newly measured 70338 results. Older 70205 artifacts, expected totals
and acceptance are not campaign inputs. There are 80 successful client-source
visits of twenty identities: twenty raw Rust, twenty native and forty across
the two production profiles. This is not physical archive-I/O accounting;
private comparison, consumer and review visits are separate.

The authored campaign separately passes 45 commands on twenty toy sources /
2,008 bytes, including 282 exact numeric cells, both loaders, required-file
failures, tampering and existing-output guards. The real campaign passes all
43 commands. Its 1,555 known source/tool/input/output guards remain unchanged;
all owned groups are absent. All 45 synthetic and 43 real consumer network
traces are empty. Deliberately denied controller/pre-exec policy probes are
outside those traces; no global zero-attempt claim is made.

Outer client evidence is bounded build-metadata reads with descriptor/EOF
checks, public Session authentication per read and complete native-source
comparison. It is not an atomic or whole-client snapshot. Caller/source Vec
capacity, Session logical limits, staged output traffic and process AS/CPU/file
limits have separate scopes; no combined heap or RSS bound is claimed.

Failed source-validation and tooling attempts remain retained: the fixture
location rejection, setup typo, two pre-codegen missing-header failures, Rust
compiler-trace classification failure and first audit's directory-mapping
error. The original runtime proposal remains unexecuted; its receipt-shape and
closure fixes are frozen as v2. Rust attempt1 did not serialize its process
exit, so its `Finished` log is not exit-0 evidence. Accepted receipts bind only
the corrected source validation, native attempt3, Rust attempt2, runtime v2 and
one synthetic/one real campaign. Later acceptance Markdown is not rebuilt into
any executable or new bundle.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70338-gt-20261014-wjtuvnvd/`.

Independent final acceptance review passes with the exact receipts below. Its
initial alias/payload classification error is retained separately; correcting
that audit reader changes no campaign input or consumer execution.


| Retained receipt | SHA-256 |
| --- | --- |
| `impl/source-review-freeze-v2.json` | `90a73d19ecc9d95da3c1e1ca26be3f0f6b68dadcdc561867d11a05555e1a84d8` |
| `impl/validation-attempt2/validation-report.json` | `c4cfd89c87b69035294fd4ba899c0f32e929ba6c4ce5e95beb96952a01566199` |
| `root/functional-binding.json` | `38527c76acda90e9b2c8ee1bd788fe3a309bf9b762554b28305691567d436447` |
| `qa/review/source-validation-review-v1.json` | `39c8626c5c5f56cac268838537c34dc3cb33a4878201823842584dd2515e06cf` |
| `tooling/native-build-attempt3/build-report.json` | `e1f3fe48aafb7af9cc2a7349a801c361c6fa9aa9464b5f0071d939a13bc8b131` |
| `tooling/rust-build-attempt2/build-report.json` | `4b128d44950259582abfba7ff28bc0d8e2f30aa2399ae02c30e2f585baa461f5` |
| `qa/review/helper-build-receipts-review-v1.json` | `b2ed1d8b659387bed1033d97a09efcbc7170fbbd1e6f167b303a8e16facc3c84` |
| `tooling/runtime-source-freeze-v2.json` | `8352cd05a7357b725044b1a55e642eaab77eea65aecbe84b0c085fc0f25af552` |
| `qa/review/runtime-source-review-v1.json` | `36951c403766e904426414a6608afa19d5c802e3438abacf19a2dabf75614641` |
| `tooling/synthetic-attempt1/acceptance.json` | `cfaeaac3116d25fc5df44f1d2e7153fb8680f88e45104a3e29c55d332b6360fd` |
| `qa/review/synthetic-acceptance-review-v1.json` | `0adb1c04d6d9042aa67494dc4c18a1e7e84a6f2361b2257f1ef19df2fd4ae2ce` |
| `tooling/real-attempt1/acceptance.json` | `a442d91ba1c8e56471a279a7c116b1e120f51b592852a21be65bb711cc6567f1` |
| `tooling/real-attempt1/evidence-freeze.json` | `b72020526abfba467fe74449c26be91d64eef4d5e90d5fdcf618ea826350bbd3` |
| `tooling/real-audit-v1.json` | `243e7917f51af967f07957950faae53bb20ac04917dc0488d91d3876ced5d4b1` |
| `qa/review/final-real-review-v1.json` | `9afec6c982183667b5d3c8323e38bda81d21f840131cd9da0e4e1848dad036cd` |
