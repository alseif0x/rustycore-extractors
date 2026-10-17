# Explicit Linux70425 foundation

Issue #86 adds an explicit foundation for WoW `1.60.1.70425`. It does not
complete extraction or admit a70425 product/component profile. Linux remains
the delivery priority; Windows work is deferred.

## API and catalog

- `forever_casc::admit_db2_70425` selects exact active, typed metadata.
- `LocalStore::open_db2_70425` opens the existing guarded local backend with
  supplied keys disabled. Complete reads retain their CKey verification and
  downward resource limits. There is no implicit network request.
- `wowdbdefs-70425-v1` registers 1,161 public table declarations with distinct
  catalog handles and provenance. Generic/older selectors and the default
  reviewed-schema lookup retain their existing contracts.
- Registration does not extend raw-text, sparse-parent or high-volume reader
  allowlists, or any terrain, collision, navigation or product profile.

Public WoWDBDefs source is pinned to commit
`996e53f029c0213b223aecb94d754f6b4b9efb49`, manifest blob
`f95528066d44e1b70ea6cd124a35a5a63685d384`, manifest SHA256
`f7b752a991d6a0321d07ecb7f06e62cd94694c1594d2fe5ce0cb7b243c71559a`
and generated JSON facts SHA256
`2150cbfc6a431931d410c3368e5e7b8de2c66b6b6a798d0052886f948624488a`.
Independent source-artifact review compares all 1,346 definitions, including
the 185 definitions outside this target, and every generated JSON/Rust fact.
That review is `4f82ddfca8c186f1d9640abb6f6604acda6f7a9323b85782e18f17ba908a10ca`.

## Software evidence

### Complete liquid reader APIs — 2026-10-17

`forever_db2::terrain_sources::read_liquid_materials_70425` and
`read_liquid_types_70425` select the fixed `wowdbdefs-70425-v1` catalog,
including its table-hash and layout gates. They reuse the complete-source
decoder, numeric projections and ordered copy replay. Existing generic readers
keep their original selection and behavior. Identical wire schemas cannot
identify a client build; acquisition must still admit the intended collection.

Seventeen new synthetic tests pass in both debug and release. Each full DB2
library run passes 441 tests with one historical ignore; all seventeen new
methods execute with no ignores. Package all-target Clippy and formatting of
the three changed Rust files also pass. Checks cover catalog identity, numeric
extremes, duplicate and repaired IDs, copy semantics, complete empty sources,
hash/layout rejection, malformed extents, unselected fields and existing limits.
The first focused run failed an incorrect test expectation for an invalid
parent-count header. The corrected expectation matches the existing parser
and legacy test; production rejection behavior was unchanged.

This is library software evidence. No new client read or native comparison was
performed. Complete liquid acquisition, same-owner compatible world traversal,
final VMaps, a 70425 product profile and runtime acceptance remain pending.
These source changes are separate from the frozen GNU15 comparison candidate
and earlier whole-workspace qualifications below.

### Foundation qualification

Functional commit `7c744d5ec8e61d3b02867d68b69a3256ec378bc9` matches all 769
selected Git sources in the frozen810-file/208-directory candidate. The other
41 inputs are the unchanged offline build closure. Commit binding is
`93348c703271a87aeadba1c5d1519f241f78703aad1a9133ded57bb48f6d925b`;
the candidate was uncommitted when frozen. Later documentation is not rebuilt
into those functional binaries.

The corrected whole software run exits zero on all eight commands:

| Check | Observed result |
| --- | --- |
| Workspace debug | 3,290 passed, 26 historical ignores, 52 target summaries |
| Workspace release | 3,290 passed, 26 historical ignores, 52 target summaries |
| Product debug/release builds | Both pass |
| Source contracts/oracle subset | 424 passed, two skips |
| Distribution tests | 165 passed, one skip |
| Clippy and workspace format | Both pass |

Each workspace profile observes all 16 new CASC bootstrap tests and five new
catalog tests. These cover exact/cross-build selection, typed metadata, guards,
resource limits, separate catalog/provenance, old default lookup and retained
reader restrictions using authored inputs. They establish no actual-client
reader acceptance.

Frozen candidate SHA256 is
`18545c8b2f255853eedfac203ebd08df4f3505d855b5c9ec9aaf07429e2c513a`;
whole software report SHA256 is
`ca5ae874ccb7bcf8ac426993c62ce02a459b271b8d02a449a67a5e17c18dffd7`.
The parent exits zero, with its numeric terminal/UUID/empty owned group captured
before stopping the unit. Independent terminal closing passes without findings:
`b7279b991d2f4543d6ffb37648a3bcac6f1719958b36ed2fa4afd031748d8587`.
All 3,997 guarded files, 48 aliases and 321 directory identities close unchanged;
both retained product executables match their actual target bytes. Runtime and
Python stages are untraced; compiler stages use the retained compiler-only
classification. No empty-network observation is claimed for untraced stages.

## Retained failures and remaining work

The first generator child exits zero but fails strict trace closing because of
Git-child SIGCHLD notifications. Separate source-artifact validation admits its
public facts; the original failed invocation is not relabeled.

The first focused runtime campaign records 20 passes and one failed new test.
The test incorrectly assumed TactKey/TactKeyLookup were among the 56 reviewed
names. Its correction checks their existing default catalog fallback; production
default lookup is unchanged. The failed campaign retains no prefix acceptance.

The first whole run passes debug but fails its metadata guard after an auditor
repeats chmod on a published QA receipt. All hashes remain unchanged; only that
receipt's ctime changes. The corrected run uses a final immutable root-owned
copy in a distinct runner/admission/attempt. The failed first run remains failed.

No fresh70425 client acquisition, available Map/GameObjectDisplayInfo projection,
complete geometry package, pathfinding/runtime use or reproducible full-suite
Linux distribution is accepted here. Those are subsequent gates of
[the complete extractor goal](forever-completion.md).

## Issue #88 — explicit available source and readers

This is a separate checkpoint from issue #86 above. Functional commit
`157a32e5d23b1afcd24624a609b8eb0b724c666a` matches all 771 selected Git inputs
in frozen C3: 812 files / 208 directories, including 41 unchanged offline
build inputs. Later documentation is not rebuilt into these functional binaries.

### API and software evidence

Issue #88 adds separate availability methods for build 70425:
`LocalStore::read_file_data_id_available_70425_with_limit` and its internal
external-budget counterpart, plus `read_map_70425`, `read_vmap_maps_70425` and
`read_gameobject_display_info_70425`. The source profile is
`linux70425-disabled-key-available-source-v1`; the reader profile is
`wowdbdefs-70425-available-sections-disabled-v1`. Closed target dispatch selects
`WOWDBDEFS_70425_CATALOG_ID` through the shared bounded parser. Existing 70374
methods/profiles, generic and older selectors, component/product admissions and
reader allowlists retain their contracts. Equal physical table layouts do not
transfer client-build admission or earlier acceptance.

All eight C3 software commands pass:

| Check | Observed result |
| --- | --- |
| Workspace debug | 3,318 passed, 26 historical ignores, 52 target summaries |
| Workspace release | 3,318 passed, 26 historical ignores, 52 target summaries |
| Explicit 70425 methods in each profile | 49: issue #86's 21 plus 28 new methods (12 CASC, 16 DB2) |
| Product debug/release builds | Both pass |
| Source contracts/oracle subset | 424 passed, two skips |
| Distribution tests | 165 passed, one skip |
| Clippy and workspace format | Both pass |

Software report SHA256 is
`ef292078faaf0ee7a73b0ed9701b8ac721b8f64179decca28c83590235308717`;
independent software QA is
`491d55811a9be92088a93ff39b8eb9f4c531d052aa576ebe936883e2c2552892`.
The functional/source binding is
`7d92470e61a1577ecb7b8a7f1bb463d137f3911e5837064f846a4b10975bcf73`.

A separate external C4 caller adds one example to C3; it is not a Git example
or a product CLI. Its five qualification commands pass: eight authored unit
methods in debug and release, release build, example Clippy and formatting.
Qualification QA is
`03cb63917e33e5af2a396096e7ae2c65bfd545eb30e4d99cdae1853fcf88d1ef`.
Static software review qualifies four ELF objects/eight dependency edges:
`5bfb19bda123806cc160f8a31c4b52f992190160cef58ba0e3a46b2bf65df8ff`.
These software facts do not themselves establish real reader acceptance.

### Actual available-reader acceptance

The single actual Rust invocation passes two fixed source requests and three
reader views: terrain Map, VMap Map and direct GODI. Post-probe metadata closing
also passes. Independent final QA passes without findings:
`2de2624726ab2b8212303f68e00ffdcc82a21fba312b09e68030698e9a8d14d2`.
It checks the retained producer aggregates, private identity binding, original
source/software facts and complete invocation namespaces; it performs no
additional client reads or stat calls.

| Source | Direct records | Copy pairs | Terrain materialized | VMap materialized | Admitted sections | Omitted sections |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Map | 73 | 0 | 71 | 73 | 1 | 3 |
| GameObjectDisplayInfo | 3,223 | 20, unreplayed | — | — | 1 | 2 |

Both actual Rust and post-metadata parents and children exit zero, with empty
strict network traces and numeric terminal/UUID/owned-group closure captured
before stopping their units. The original 4,010 Rust guard facts, 6,830 closing
metadata guard facts, 48 aliases in each role and 319 input directories remain
unchanged. The original 3,167 installed-software facts are also checked, including
13 resources checked separately from the metadata runner's original 3,154.
This is bounded available-reader acceptance, without a new native value/parity
comparison or full source completeness claim.

Omitted sections remain unknown. Available encoded sources authenticate their
EKey/encoded frames; holes do not establish full plaintext CKey verification.
No missing bytes, omitted rows or unknown sections become a complete table.
VMap materialization retains WDT-zero rows and its existing source-direct copy
and parent semantics; this diagnostic does not validate the parent graph.
No model, placement, collision, navigation, package or runtime acceptance follows.

The accepted metadata controls have four content reads across two new
observations and two post-probe closing reads: six accepted-scope content reads.
An earlier metadata invocation completed four reads but failed strict network
trace closing after Python startup made two AF_UNIX socket calls and two nscd
connect attempts returning ENOENT; no INET calls were observed. That invocation
remains failed and unaccepted. Its omitted Python `-S` flag was corrected in a
separate explicit `-I -S -B` version, whose strict trace is empty. There is no
automatic retry or IPC policy waiver. The campaign retains ten external metadata
content reads in total, including the four failed-attempt reads; it does not
relabel them as accepted observations.

Full catalog coverage, compatible missing-file handling, final disk collision
and product integration, navigation, RustyCore runtime and a reproducible full
Linux suite remain gates. The issue #86 evidence and historical failures above
retain their original scopes.
