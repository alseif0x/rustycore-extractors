# Required-input framing acceptance and source v6

Authored 2026-10-11. This supplements the byte-exact
[pre-live implementation snapshot](forever-required-framing.md). That document's
pending real-client wording describes its initial source validation. The later
bounded Linux observations below supersede that pending status only. The
[metadata routes design](forever-required-routes.md) and v5 contract likewise
retain their historical scopes. No imported implementation or old report is
rewritten to imply a later success.

## Distribution source boundary

The current source profile is
`forever-seven-crate-windows-vmap-required-framing-source-v6`. It retains envelope
`forever-distribution-v1`, seven workspace crates, nine lock packages, the same
two pinned registry archives/notices and the v5 local dependency graph, including
Windows-only `forever-vmap -> forever-platform`. It mandates the separate
`LocalStore::diagnose_required_framing(framing::Limits)` API, exact-span reader,
23 synthetic cases, exported modules, both framing documents, source contract
and static consistency checks. Missing required source, contract, tests or
documentation rejects before compilation; omitted mandatory manifest inventory
rejects verification. Generic crate Markdown and unrelated
live helpers remain excluded. Historical bundles for prior source profiles retain their bundled frozen
verifiers; the v5 source merge did not build a v5 bundle. This source increment has not built a v6 bundle.

The adapter is key-free and closed to Map 1349477, SkillLineAbility 1266278 and
GameObjectDisplayInfo 1266277. It preserves first eligible central-present ROOT
selection, every encoding alternative and every nine-byte index collision; no
archive-availability fallback promotes a different route. It reads one entire
bounded record at a time and ignores exactly 30 local-prelude bytes. The full
EKey and that prelude are not authenticated. All plans, candidate counts, total
indexed bytes and report reservations precede requested payload reads. Limits
only reduce 128 KiB per record, 256 KiB total indexed records, 512 KiB expected
decoded bytes per input, 64/192 candidates and 128 KiB retained report. The
4096-frame limit is **per candidate
BLTE object**, checked after its complete read; it is not an aggregate FDID
frame limit. Existing store memory and final identity guards remain mandatory
on success and failure. Linux metadata observations are not an atomic snapshot
or an anchored ancestor lease. Budgets are logical admission, not RSS/all CPU.

## Source validation

The accepted source handoff is
`required-framing-adapter-20261011T191915Z/handoff-frame-cap-precision.json`,
SHA256 `8a7a97a89d51fda8b95014a4d529499689a505f4069c6bc27290da47a4e6ac6e`.
Its freeze SHA256 is
`1bc8a2b2f34e3141e17d8372297d89dc8c8ee549d8eed64e9d44345c85b09605`.
Linux debug/release each pass 111 LocalStore and 80 BLTE tests, including 23 new adapter
cases, with no ignored cases. Linux and Windows GNU all-target Clippy pass.
Independent review/supplement SHA256 is
`62fc00718406fa9e379420e22776be5c69f720e3a864c23dc4190354d5ffbc4f`.
Windows compilation does not establish executed Windows/NTFS acceptance.
Static contract tests are consistency checks, not runtime proof.

## Later bounded real observations — build 70205/esES

The key-free acquisition observation
`required-acquisition-live-20261011T1930Z-attempt2/live-report.json`, SHA256
`d1979922ac941a5c7363f7e014f023f47cffd8372523df49d4d35bd6c5c42536`,
is accepted/terminal as a diagnostic. All three reads still reject with
`local_store_no_verified_encoding`: each records encrypted 1, missing 0,
incomplete 0, corrupt 0 and unsupported 0. This first-E rejection alone does not
inspect later frames or establish complete ciphertext. No decoded input was
acquired and no typed loader was invoked by this observation.

The separate complete-span adapter run
`required-framing-live-20261011T1940Z-attempt2/live-report.json`, SHA256
`43f2ccf6a92a4d9e33239be3cc3e0e3de87bb43d6b69d7e9baea93c515ed3f01`,
is accepted/terminal. Each selected input has one `Complete(FramedMd5)` candidate:

| Input | Record bytes | Declared decoded bytes | Frames / verified MD5 | Z | E |
| --- | ---: | ---: | ---: | ---: | ---: |
| Map | 5,042 | 10,181 | 5 / 5 | 2 | 3 |
| SkillLineAbility | 94,617 | 347,118 | 7 / 7 | 2 | 5 |
| GameObjectDisplayInfo | 74,592 | 116,054 | 3 / 3 | 2 | 1 |
| Total | 174,251 | 473,353 | 15 / 15 | 6 | 9 |

This checks the entire admitted BLTE framing/table/extents/EOF, declared-size
sums, all 15 declared encoded-frame MD5s and outer N/Z/E wrappers. Z is not inflated;
E ciphertext interiors and inner modes are not parsed or decrypted. This is
not full EKey/CKey or publisher authentication, plaintext integrity, complete
ciphertext-interior validity, or evidence that keys are the only missing
requirement. Headerless evidence remains explicitly unhashed.

Root rehash report SHA256 is
`ebe4ad0cfb130f5c0f6b9d900d4afbc703ace676dd8636249a20d477548fa108`;
agent closeout SHA256 is
`dfc93865746aa683cbfeca5e6f48c5c49074a99dca42b8edf2321d1ce125a966`.
Source 932, canonical 921, build-info and prior binaries remain unchanged during
that run. The runtime filtered network trace is empty under AF_UNIX-only seccomp.
This describes observed calls, not absence of denied attempts or individual exit
of every descendant. No raw client assets, keys or payload outputs are retained.
The original preparation failed its 256 MiB growth cap and remains failed; root
separately authorized 320 MiB total growth with a 3 GiB free floor, reusing the exact
built binary without compilation. This does not relabel the original report.

## Historical failures and remaining gates

Earlier `local_file_incomplete` outcomes for these inputs remain historical
results of those native/batch/locale attempts. They are not the current
key-free acquisition category: current diagnostics report encrypted rejection,
followed separately by admitted whole-span framing. No old failed batch is
accepted or repaired. The Birth, initialization and GameObject typed loaders
have not been rerun successfully with decoded versions of these three inputs.
Their complete real acceptance, terrain/world acquisition, full RustyCore
runtime acceptance, Windows/NTFS behavior, enabled Windows product, product
publication and full extractor completion remain open. There is no key lookup,
provisioning, download, repair or decryption in this increment.
