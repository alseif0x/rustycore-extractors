# Forever local CASC backend in Rust

`forever_casc::local_store` is a read-only library backend for the admitted
`wow_classic_beta` versions `1.60.1.70170` and `1.60.1.70205`. It opens local
build metadata, version 7 indexes, ENCODING, ROOT and complete archive records
using Rust. The existing native backend and extraction CLI defaults are
unchanged. This API does not establish full extractor or runtime compatibility.

The format and selection reference is TrinityCore commit
`02245dcd245e7433e524577656177723d3e4992e`, principally `CascFiles.cpp`,
`CascOpenStorage.cpp`, `CascRootFile_WoW.cpp`, `CascReadFile.cpp` and
`CascStructs.h` in its CascLib dependency. The store adapter is original code.
The dependency and notice closure is recorded in
[the Rust CASC licensing document](licensing/forever-rust-casc.md).

## API

The caller supplies the client root, locale and all physical layout choices.
There is no layout autodetection or retry with another layout after a failure.
For example, a caller that has independently established contiguous indexes and
ROOT 50893 version 2 can construct:

```rust
use forever_casc::{
    Locale,
    local_index,
    local_store::{Limits, LocalStore, OpenOptions},
    root,
};

let options = OpenOptions {
    locale: Locale::EsEs,
    index_layouts: [local_index::Layout::Contiguous; 16],
    root_layout: root::Layout::Header50893V2,
    limits: Limits {
        max_decoded_file_bytes: 16 * 1024 * 1024,
        ..Limits::default()
    },
};
let mut store = LocalStore::open(client_root, options)?;
let verified = store.read_file_data_id(file_data_id)?;
```

`OpenOptions.index_layouts` permits a separate reviewed layout for each bucket;
`Contiguous` and `Paged` use their respective checked parsers. `root_layout`
selects one of the four explicit ROOT variants. The caller must establish these
choices for its input; supplying a matching header name alone is not evidence.

| Method or result | Meaning |
| --- | --- |
| `build()` | Owned admission metadata: product, exact version, build number, Build Key, CDN Key, tags and optional config build UID/name |
| `stats()` | Index row count, ENCODING row and EKey counts, ROOT row count and logical retained-byte charge |
| `read_file_data_id(id)` | One complete file, subject to the configured per-file ceiling |
| `read_file_data_id_with_limit(id, bytes)` | The same read with a ceiling that can only tighten the store policy |
| `VerifiedFile` | FDID, selected full CKey, owned complete bytes and number of candidate attempts |

FDID zero and `0xffffffff` are rejected. Returned bytes belong to the caller
and remain valid after the store is dropped. Results must be retained only in
private, authorized output locations; they may contain client assets. Do not
log a whole `VerifiedFile` or upload raw results.

## Admission and metadata

`.build.info` must contain exactly one active row for the admitted product and
one unique instance of each required typed column: Product, Active, Version,
Build Key, CDN Key and Tags. Row widths, active markers, exact hash lengths,
text constraints, row count and byte count are checked. Unknown columns are
not interpreted. A build-config name cannot authorize another product or build.

The Build Key identifies `Data/config/aa/bb/<full-key>`; the complete config
must match that MD5 before parsing. ROOT's CKey and ENCODING's CKey plus EKey
are mandatory. Optional decimal sizes retain the pinned unknown-size sentinel
as `None`, distinct from a declared zero.

Central CKey presence includes all ENCODING rows and the reviewed config
references, with original insertion priority: ENCODING's own config reference,
ENCODING rows, download, install, patch, root, size, vfs-root and numerically
ordered positive `vfs-N` entries. Exact names and case aliases are checked;
duplicate relevant fields reject. Unknown assignments and `*-config` metadata
are counted but do not create routes. A single-hash PATCH is EKey-only and
creates no central CKey. Auxiliary tags with sizes but no CKey likewise do not
create a CKey.

All CKey rows, EKey alternatives and index-prefix collisions are preserved in
owned vectors with ordinal lookup indexes. A known content size is the first
value from the first inserted row. If it is unknown, only a later build-config
reference may fill it; duplicate ENCODING rows cannot fill or overwrite it.
Build-config references likewise never overwrite a known size. This preserves the reference's insertion
semantics while retaining alternatives that its maps may collapse.

The backend does not load DOWNLOAD to fill unresolved sizes or implement its
EKey-only availability catalog. A selected content key with no known content
size returns `local_store_unknown_content_size`. CDN Key and build tags are
retained as metadata; no CDN configuration, encryption key or listfile is
fetched or discovered.

## Selection and complete reads

The backend requires all sixteen index buckets. It chooses the highest numeric
generation in each bucket; duplicate filename aliases reject. A bad selected
newest file does not fall back to an older generation. Every selected index
uses its declared offset-bit geometry, checked guards and the explicit layout.
Unknown directory entries still count toward the scan bound.

ROOT selection uses central CKey presence, independently of physical archive
availability. It applies the pinned audio-pass ordering, content exclusions,
physical order and locale mask. Neutral blocks are eligible. esES never falls
back to enUS. Once a ROOT row is selected, a failed local read cannot select a
later ROOT row.

For that selected CKey only, each complete EKey and each matching nine-byte
index prefix is considered in preserved order. An attempt means either an
EKey with no index match or a specific matching index row. The combined count
is bounded. Local records come from `Data/data/data.NNN`; this profile rejects
archive numbers at or above the pinned `0x1000` ceiling.

A candidate must fit the actual archive file, have its exact encoded span read,
and decode to the exact known content size. The 30-byte archive prelude is
removed explicitly; its EKey and checksum fields are ignored, as in the pinned
reader. The index checks only the EKey's nine-byte prefix. **The remaining
EKey bytes are not authenticated by this adapter.** Complete CKey MD5 is
mandatory; BLTE checks its frame checksums and zlib checks as applicable.

Only BLTE N and Z are supported. E, F and unknown modes are explicit candidate
rejections. No missing spans are padded with zeros and no prefix is returned.
An empty file also needs a present, complete BLTE record that verifies against
its CKey; an empty-content MD5 alone does not prove local availability.

## Default policy limits

Limits can be reduced or explicitly increased by the caller. They are policy,
not declarations of general CASC compatibility. Bound failures do not silently
truncate scans, routes, metadata or files.

| Resource | Default |
| --- | ---: |
| Logical retained payload | 768 MiB |
| Logical current scratch payload | 512 MiB |
| Retained plus current scratch payload | 1 GiB |
| `.build.info` | 256 KiB; 4,096 nonempty rows; at most 64 columns |
| Path storage bound | 4,096 bytes, including checked suffix allowance |
| Build config | 256 KiB; 8,192 physical lines; 4,096 assignments |
| Directory scan / index filenames | 65,536 entries / 4,096 index files |
| One index / all selected index rows | 64 MiB / 2,000,000 rows |
| Paged index pages per file | 131,072 |
| ENCODING | 256 MiB; 65,536 pages; 64 KiB per page |
| ENCODING ESpec / opaque tail | 16 MiB / 128 MiB |
| ENCODING rows / EKeys | 4,000,000 / 8,000,000 |
| ROOT | 256 MiB; 4,096-byte header; 65,536 blocks; 4,000,000 rows |
| One archive span | 256 MiB, including the prelude |
| One ordinary decoded file | 64 MiB |
| Candidate attempts / BLTE frames | 64 / 65,536 |

These are **logical payload budgets, not a process RSS limit**. Requested
Vec/String/Path storage, retained admission bytes and conservative overlap
between decoded metadata and parser/owned vectors are charged. Allocations use
checked arithmetic and fallible exact reservations. Ordinary allocation
bookkeeping, fixed stack frames, standard-library IO internals and previously
returned files retained by the caller are outside the accounting. Bounds are
checked before large phase allocations; conservative conversion overlap can
reject a run whose eventual retained representation would fit. Decoded
ENCODING and ROOT buffers are released after conversion; there are no pointers
into the store's own backing buffers.

## Input guards and diagnostics

Opening captures guards for `.build.info`, build config, the data directory and
each selected index. A guard checks length, modification time and, on Unix,
device/inode identity. `.build.info` additionally retains its bounded original
bytes and compares them exactly when checking the guard, including same-length,
same-mtime rewrites. Guards are checked before and after data reads; archive
file identity is checked before and after a candidate read. Initial config,
index, ENCODING and ROOT parsing validates their checksums independently.

These checks are not a filesystem-wide atomic snapshot. Build-config and index
guards do not repeatedly hash their entire files, and a coordinated same-size,
same-mtime in-place rewrite can evade their later stat checks. Every successful
payload still has to match its selected complete CKey. Files and directories
are opened or inspected read-only; no network request, client write, cache
population or remote fallback occurs.

`Error::code()` and Display return static diagnostics with no client paths or
keys. `NoLocalEncoding` carries aggregate counts for missing, incomplete,
encrypted, corrupt and unsupported candidates. Input mutation, malformed build
metadata, ambiguous admission, missing ROOT selection, unknown size, incomplete
index sets and policy limits are separately reported. Nested parser errors keep
their specific codes. No error contains partial bytes.

## API evidence

The coordinator executed the synthetic CASC, DB2 and terrain suites, native
checks and Clippy successfully on 2026-10-11. Local-store fixtures cover both
admitted build strings, N/Z, alternative encodings, prefix collisions,
unknown/conflicting sizes, config-only central presence, locale/audio ordering,
empty files, unsupported modes, missing/truncated/corrupt records, budgets,
newest-index policy and exact build-info mutation guards. Independent review
closed three findings with regression coverage: only build references may fill
a duplicate ENCODING row's unknown size, input mutation takes precedence over
EOF, and index inventory paths charge their actual reserved capacity.

The bounded live API acceptance report
`local-store-accept-20261011T011040Z-c84f6f43.json` records **57 complete files,
528,633 bytes**, all byte-identical to the pinned C++ results, and two known
incomplete files rejected without partial data. The observed logical retained
charge was 341,831,934 bytes. Source/input guards passed and no network attempt
occurred. This report covers the build-70205 API. The production CLI evidence
is recorded below. Previous ENCODING and ROOT bootstrap reports separately establish
complete metadata-object equality with the pinned reader.

## Shared CLI acquisition

`acquisition::OptionsBuilder` parses only the shared acquisition pairs, leaving
component-specific options to each command. Its `finish(component_cap)` returns
`Selection::Native` by default. `--backend rust-local` requires both
`--index-layout contiguous|paged` and an explicit `--root-layout` from
`headerless-18125`, `header-30080`, `header-50893-v1` or `header-50893-v2`.
The CLI profile applies the chosen index layout uniformly to all sixteen
buckets; the lower-level store API can represent mixed choices. Native rejects
layout and size overrides. There is no `auto` selection.

`--max-file-bytes` is a positive decimal integer that can only reduce the
component ceiling: 16 MiB for DB2/cameras or 4 MiB for GT. It does not reduce the
separate bootstrap metadata bounds. Unsupported native capability returns
`backend_unavailable` before client/output IO; a binary built without the
native feature never changes the default to Rust automatically.

`acquisition::Session::open` opens one backend before output creation. A batch
reuses `read(id, budget)` with its remaining per-file/batch budget; Rust uses
the smaller of that budget and the declared profile ceiling. Build metadata and
indexes are not reloaded per file. `build()` supplies the admitted number;
`profile()` is `None` for native and `Some(Profile)` for Rust. A detected
`InputChanged` poisons the session, so subsequent reads fail without backend IO.
The component batch integrations also abort immediately and retain their
incomplete marker.

The closed [shared acquisition schema](../schemas/forever-acquisition-v1.schema.json)
records backend, format profile, all sixteen index layouts, ROOT layout,
offline operation, full CKey checking, EKey-prefix checking and all 29 policy
bounds. `Profile::render` emits that deterministic JSON object;
`Profile::parse` and the crate's `Profile::decode` reject missing, duplicate,
unknown or inconsistent properties. The parent component must enforce its
smaller per-file ceiling. Paths, source keys and raw client bytes are excluded.

Existing DB2 v2/v3, GT v1/v2 and camera v1 manifests have closed key sets and
remain native-default contracts. New acquisition provenance belongs to new
DB2 v4, GT v3 and camera v2 contracts. These versions are emitted by the
explicit Rust-local component integrations and checked by their corresponding
verifiers and closed schemas. The default selection remains **Native**.

## Production CLI acceptance

The coordinator's build-70205 report
`rust-cli-accept-20261011T012203Z-2aa558be.json` records five successful plans
through the production extraction commands, using a binary snapshot built
without the native feature. The plans account for **61 artifact comparisons**;
this is not a count of unique files because plans overlap.

| Plan | Manifest | Artifact comparisons |
| --- | --- | ---: |
| DB2 reviewed default | v4, reviewed scope | 2 |
| DB2 catalog selection, including sparse ConversationLine | v4, catalog scope | 22 |
| GT default set | v3, all six default tables | 6 |
| GT catalog selection | v3, catalog scope | 20 |
| Cameras | v2, source plus ten camera files | 11 |

All compared bytes matched the prior accepted C++ outputs. Every successful
manifest passed its schema and production verifier. The GT checks also passed
22,082 f32 comparisons and re-executed two public RustyCore loaders successfully.
The guarded source/input checks all passed and no network request occurred.
This exercises the explicit local N/Z profile and its recorded policy bounds.

The same report records the following negative cases:

- Incomplete Map and SkillLineAbility reads, plus a one-byte per-file ceiling
  for each of DB2, GT and cameras, returned exit 8 with schema-valid failure
  manifests and no extracted client assets.
- Fifteen early option, ceiling and unavailable-native cases rejected before
  client or output IO. Native remained the default even in this build without
  native support.
- Two incorrect explicit-layout bootstrap attempts failed before output
  creation, without layout fallback.
- All fifteen acquisition-profile tampering cases were rejected by the
  production verifiers.

The corresponding validation snapshot passed 340 CASC tests, 248 DB2 tests,
two DB2 example tests and 65 terrain tests, together with Clippy. Native
validation separately passed 341 CASC tests and three probe tests; Python
validation passed 226 tests with six skips, and all 49 contract checks passed.
These native checks are separate from the production Rust CLI snapshot above.

This evidence covers the stated plans and build. Explicit encryption keys,
other physical profiles, build-70170 CLI acceptance and full extraction/runtime
acceptance remain separate gates in the [completion plan](forever-completion.md).
Later terrain kernel and complete ADT comparisons are recorded separately in
[the terrain contract](forever-terrain.md); they are outside this CASC/CLI snapshot.

## Opt-in local name reads

`root::name::NameIndex` and `LocalStore::enable_names` now implement the
conservative `linux-ascii-stable-root-names-v1` capability. After explicit
activation, `read_name` / `read_name_with_limit` resolve a complete logical name
and read the retained CKey directly. Existing FDID behavior is preserved.
This is an API increment: `acquisition::Session`, the native product bridge,
CLI selection and manifest provenance still expose their existing FDID
contract. The `local-v7-ckey-root-nz-v1` acquisition profile does not silently
include name activation.

### Materialization before lookup

At the same pin `02245dcd245e7433e524577656177723d3e4992e`,
`dep/CascLib/src/CascRootFile_WoW.cpp:334–405,432–546` processes eligible ROOT
rows in audio-pass/physical order, applying the storage locale and content
exclusions. Only rows whose CKey exists in the central CKey map participate.
Local archive availability must not decide which row wins.

`common/FileTree.cpp:370–419` creates one node per FDID. Its first admitted
row fixes the CKey and flags, but a later nonzero name hash overwrites that
same node's hash. An absent or zero hash does not clear it. Thus these rows,
with both CKeys present centrally, make name hash H select **CKey A**:

```text
FDID 7, CKey A, no name hash
FDID 7, CKey B, name hash H
```

Filtering ROOT rows by H would incorrectly choose B. A simple final-hash map
is insufficient for all cases too: NameMap stores pointers to mutable nodes;
duplicate hash insertion does not replace an existing match. Rebuilding the
map reinserts nodes in creation order, and its timing can depend on whether
`realloc` moves NodeTable. See `common/FileTree.cpp:93–125,201–233`,
`common/Map.h:135–195` and `common/Array.h:193–221`.

The optional index processes all eligible rows and retains the first CKey per
FDID. A first nonzero hash may arrive later, but different nonzero hashes for
one FDID or a nonzero hash shared by different FDIDs reject the entire name
index. A failed activation leaves FDID reads and resident accounting intact.
This excludes ROOTs containing renames and is not general CascLib name
support. Accepted lookups resolve hash to node to its retained CKey, then use
the existing complete-read rules and EKey alternatives for that CKey alone.

### Normalization and excluded effects

`common/Common.cpp:418–460` normalizes ASCII lowercase to uppercase and `/`
to backslash, without collapsing separators, `..`, spaces or colons. Its Jenkins
`hashlittle2` call uses two zero seeds and combines the returned words as
`(pc << 32) | pb`; the final words are `pc=c`, `pb=b`. The algorithm reference
is `jenkins/lookup3.c:470–642`; u32 arithmetic wraps and input words are little
endian.

The input domain is nonempty ASCII without NUL. The pin indexes the
normalization table with plain `char`, without an unsigned conversion, so
high-bit bytes cannot establish safe parity on a signed-char build. A planner
may retain such a logical name while its acquisition remains unsupported.
`CascPort.h:185–186` defines non-Windows `MAX_PATH` as **1024 bytes**. The
Linux profile admits exactly 1024 and rejects longer names; it does not
copy the native hash's prefix truncation or assume the same Windows limit.

Storage opening subsequently inserts `ENCODING`, `DOWNLOAD`, `INSTALL`,
`PATCH`, `ROOT` and `SIZE` names and can trigger another rebuild
(`CascOpenStorage.cpp:806–841,1239–1248`). The capability
rejects queries whose hashes match those internal names. Listfile search can
also materialize names and folder nodes; it remains excluded. Missing names
do not fall back to textual FDIDs, CKeys/EKeys, INSTALL, another locale,
another ROOT row, a different backend or network access.

FDID zero needs particular care: native nodes can use it and be opened by name,
whereas the current FDID read API rejects it. The name API reads the selected
CKey directly, without routing through the FDID API or inventing an ID. Native
`CascGetFileInfo` also cannot prove the matched FDID when several nodes share
a CKey: `common/RootHandler.cpp:106–118` uses the first matching
node found by `common/FileTree.cpp:520–531`.

### Bounds and activation

Default name-index limits are 65,536 blocks, 4,000,000 rows/nodes/names,
256 MiB simultaneous vector capacity and 1,024 name bytes; each is reducible.
The builder validates the complete block partition, block indexes, per-block
ID order, hash-array presence and the invalid-ID sentinel before invoking its
central-presence callback. Ordinal scratch and retained routes are reserved
fallibly and charged by actual capacities. No allocation is sized by maximum
FDID. Fixed stack storage, borrowed inputs and allocator bookkeeping are not
part of the vector budget.

`enable_names` builds after decoded ROOT bytes have been released. Before any
reservation its peak allowance is reduced to the minimum of the requested
limit, store scratch allowance, remaining combined budget and remaining
resident allowance. Charging peak construction against free resident space is
conservative; the final resident charge includes only the retained capacity.
`names_stats`, `names_limits` and `names_capability` expose the result.

Activation is atomic and surrounded by input guards. Repeating exactly the
same requested limits is idempotent with fresh guards; changing them after
success rejects explicitly. Name reads share complete MD5, same-CKey EKey
alternatives, read ceilings and post-read input guards with FDID reads.
`InputChanged` remains an unwrapped store error. Session-level name activation
and poisoning, native name operations and versioned product provenance remain
future integration work.

### Executed synthetic evidence — 2026-10-11

Thirty-two pure name-index tests and fourteen complete synthetic LocalStore
cases pass, within 408 CASC library tests. They cover original lookup3 hash
vectors around 12-byte boundaries and 1,024 bytes, input validation and limits,
late names, first-CKey retention, zero hashes, missing central routes,
audio/locale/content ordering, ID zero, aliases, internal names, global
conflicts, atomic failed activation, changed inputs and complete-read failures.
The selected incomplete CKey never falls through to a later ROOT row.
Independent source audits found no outstanding defect in either component.

`name-index-20261011T021550Z-2ee1ce9d.json` records 135 accepted synthetic
cases and 3,468 exact hash/FDID/selected-CKey query comparisons against the
original pinned `CASC_FILE_TREE` and `CalcFileNameHash`. Cases include node
counts around 16,384 and up to 25,000, late names, central-presence filtering,
locale/audio ordering, CKey aliases and insertion of the six internal names.
Two ambiguous-name cases reject only through Rust; no native rejection
parity is claimed. Source, original-source and binary guards pass. The C++
adapter supplies eligible rows; it does not execute the original ROOT parser,
storage acquisition or a real client. The authored adapter is
`tests/forever/native_name_index.cpp`; original sources remain outside Git.

### Local activation and negative lookup evidence

`local-name-accept-20261011T021802Z-675f74b5.json` compares 52 explicitly
supplied candidate names on build-70205/esES against a freshly built pinned
offline CascLib caller. These include candidate GT/DB2/WDT names, normalized
variants and an intentionally absent synthetic name. **All 52 are absent in
both implementations; zero complete named files are read.** A failed name
lookup does not establish that its corresponding FDID or content is absent.
This is negative lookup evidence, not positive real name-read acceptance.

The real name-index activation succeeds over 2,747,538 ROOT rows. It observes
1,524,678 eligible rows, 1,524,668 centrally present rows, 1,425,393 nodes and
137,388 named nodes. Retained vector capacity is 5,495,520 bytes and peak
construction capacity is 11,594,232 bytes; these are logical payload sizes,
not RSS. Source, binary, client metadata and original-source guards pass, with
no network attempt under syscall denial. No raw output is created for a
missing name. The earlier 26-query diagnostic is retained separately and is
not added to these counts.

This does not execute terrain discovery, recover incomplete Map/SkillLineAbility,
change a product manifest or establish general CascLib filename compatibility.

## Explicitly supplied Salsa20 keys (library API, 2026-10-11)

The separate opt-in APIs are `keys::Keyring::parse`, `blte::decode_with_keys`,
`archive_record::decode_span_with_keys` and `LocalStore::open_with_keys`.
`LocalStore::open`, the existing acquisition profiles and the v1 product plan
continue to use the N/Z profile. The initial library increment supplied no
production CLI key option. The subsequent explicit [product v2 contract](forever-product.md#explicit-supplied-keys-product-v2)
adds `--keys` only for its admitted client commands. Providing keys to a
library caller does not silently broaden an existing manifest or acquisition
contract; the earlier component CLIs retain their previous profiles.

The caller supplies the keyring bytes explicitly. The module performs no I/O,
search, network request or built-in-key lookup. The closed ASCII format is:

```text
forever-tact-keys<TAB>1<LF>
<16 lowercase hex digits naming a u64><TAB><32 lowercase hex key digits><LF>
```

Records must be strictly sorted by numeric name and unique, with 1..4,096
records and at most 256 KiB. Comments, empty records, CR, extra fields,
uppercase hex, incomplete records and missing final LF reject. Key values are
16 bytes; the key name is written in conventional most-significant-digit-first
hex and decoded to the little-endian u64 carried in the BLTE header. Neither
keyring nor entries implement Debug, Clone or serialization; errors contain
only static codes, with no key name, key value, offending line or path.
Caller-owned input and retained key memory are not guaranteed to be securely
erased. Only deliberately synthetic keys belong in fixtures or reports.

The bounded wire profile admits one encrypted E wrapper around a complete N
or strict Z frame. Key-name length is zero (name zero) or eight bytes; nonce
length is four or eight bytes, padded to eight. Only the S cipher is admitted:
Salsa20/20 with a 128-bit key and initial 64-bit block counter zero. The
little-endian frame ordinal is XORed into the first four nonce bytes. A, nested
E, F and unknown modes reject. This is not general CascLib cipher support.

Ciphertext frame MD5 is checked before key lookup/decryption. The complete
expected content MD5 is mandatory when an encrypted frame is admitted, including
headerless objects. No wrong/missing key, incomplete record or checksum error
returns plaintext bytes. This checks the recovered content against the supplied
CKey; it is not a MAC or a proof of trusted provenance. Strict zlib length,
terminator, consumed-input and Adler32 checks remain in effect; the pinned
native decoder's short-output zero fill is still deliberately rejected.

Keyring capacity is reserved once after complete syntax/order validation.
Its actual Vec capacity counts against LocalStore's resident budget. BLTE
preflight checks/decrypts one frame at a time before allocating the final
output; the decode pass decrypts again into one temporary buffer whose actual
capacity is at most the encoded frame length. LocalStore conservatively
reserves two complete encoded spans plus the decoded output for the opt-in
path, even if an individual object is unencrypted. Limits are not raised.
No-key opening retains its previous reservation policy.

Keys change available encodings of the selected content, without changing
logical collection identity. ROOT selection, same-CKey alternatives, exact
size, complete content MD5 and metadata guards retain their existing rules.
Missing keys count as encrypted candidates, unsupported ciphers as unsupported,
and malformed encrypted records as corrupt. A different encoding may succeed
only for the same selected CKey. Encrypted ENCODING and ROOT can be opened with
explicit keys; this is synthetic coverage, not evidence of recovering either
metadata object in a real client.

Original Salsa20 code follows [Bernstein's specification](https://cr.yp.to/snuffle/spec.pdf),
with public [eSTREAM vectors recorded by Crypto++](https://raw.githubusercontent.com/weidai11/cryptopp/master/TestVectors/salsa.txt).
There are 17 key/parser/primitive tests (12 published vector cases), 13 encrypted
BLTE tests, two archive adapter tests and eight synthetic LocalStore tests.
These include encrypted metadata, missing/wrong keys, strict headers, frame
ordinals, checksums, block boundaries, fallback, memory limits and incomplete
spans. No actual client keys or built-in key table are accessed. See the
[completion gates](forever-completion.md); incomplete real inputs and runtime
acceptance remain open.

Workspace regression acceptance passes in both debug and release: 2,063 library
plus four example and three binary tests per profile (2,070 total), formatting,
workspace Clippy and 72 contract tests. All 17 existing product CLI scenarios
per profile pass with the default N/Z capability. Six actual SIGKILL recoveries
remain accepted, including the verified finished-producer skip. Seventy CLI
network traces are empty; the other six contain the injected interruption and
filesystem synchronization, with no network call. This is synthetic regression
coverage; no new real client or secret key was supplied in this run.

The original pinned `CascDecrypt` additionally produces ciphertext for 272
authored synthetic cases (544 encrypt/decrypt calls). Python standard-library
zlib/hashlib constructs the independent inner-frame/plaintext/CKey oracle;
the public Rust `decode_with_keys` decodes 488 complete BLTE objects per debug
and release profile, matching every byte (89,120 decoded bytes/profile).
Each encrypted object is also rejected by the default no-key API. The corpus
contains 208 headerless, 208 framed, 64 indexed and eight mixed-frame objects,
with key-name lengths 0/8, IV lengths 4/8 and actual frame ordinals
0/1/3/255/256. This compares the decryption wire profile; the native CASC routing,
LocalStore and real clients are not executed in this oracle. The separate eight
LocalStore tests supply synthetic integration evidence.

The original native adapter, both Rust runtime invocations and the native
compiler have four empty traces under injected network denial. Rust compiler
traces are recorded separately: their only network-category syscalls are
matching local AF_UNIX linker IPC socketpair/recvfrom calls. No Internet
request is made; compiler IPC is not counted as an empty runtime trace. Earlier
adapter-build/trace-validation attempts are retained as superseded evidence.
