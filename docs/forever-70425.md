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
