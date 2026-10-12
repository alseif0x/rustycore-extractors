# Local Forever Rust distribution candidate

The [versioned distribution contract](../contracts/forever-distribution-contract-v1.json)
covers a local Linux x86_64 candidate for the unified `forever-extractor`.
It does not authorize publication or complete the [Forever extraction goal](forever-completion.md).
Windows functionality, complete real all-component extraction, full-world
operation and RustyCore runtime acceptance remain open.

## Inputs and build

The packager uses Python 3 with `tomllib`, the already installed Rust **1.98.0**
toolchain, a native linker and the two exact dependency archives in an explicitly
selected existing Cargo cache. It performs no downloads or native-reference
build. Start from the project environment for external build/run locations:

```bash
source .opencode/environment.sh
python3 tools/package_forever.py build \
  --source "$PWD" \
  --output /absolute/external/new-distribution-run \
  --cargo-cache "$HOME/.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f" \
  --epoch 1791720000
```

The destination must be new and outside the source and every Git checkout.
Use an explicit fixed epoch for reproducibility. Do not use a client input,
reference output or existing successful run as the destination. Failed runs
are retained for diagnosis; the tool does not remove or overwrite older runs.

The current allowlisted source is copied, including uncommitted product code.
A HEAD-only archive would omit the current implementation. The source snapshot
includes exact Cargo manifests/lockfile/toolchain, production Rust sources,
required contracts, synthetic checks, packaging recipe and retained project
licenses/notices. Client assets, keys, CASC caches, generated extraction
outputs, secret environment files and the native reference checkout are not
package inputs. Links, special files and unreviewed binary extensions reject.
The selected source snapshot is checked for mutation.

The exact admitted registry closure is `miniz_oxide 0.8.9` and `adler2 2.0.1`.
Both archives must match the SHA-256 values in Cargo.lock and the
[recorded licensing closure](licensing/forever-rust-casc.md). Their original
source files and all notices are retained alongside the exact archives.
The source also retains the existing `THIRD_PARTY_NOTICES.md` and its three
referenced machine-readable `licensing/` inventories. Those inventories describe
legacy C++ provenance; the pure Rust binary closure is recorded separately in
the distribution manifest, without claiming native C++ dependencies are linked.
The pinned
Rust toolchain's `COPYRIGHT-library.html` and all 12 distributed license texts
are included under `toolchain-notices/`, with exact independent byte hashes.
This preserves standard-library attribution without asserting that every
component listed in those broad notices is linked. The initial build reads
them only from the known installed compiler sysroot; reconstruction uses
bundled copies, without requiring external toolchain documentation or a
download. Missing or changed notice inputs reject. No new compiled dependency
or upstream implementation is imported by copying these notices.

Generated Cargo directory-checksum files/configuration enable an isolated
`--offline --locked` build from the shipped source. Neither an external
Cargo cache nor a native C++ checkout is needed for reconstruction.

The frozen shipped source builds with:

```text
cargo build --offline --locked --release -p forever-extractor --no-default-features
```

The recipe records sanitized settings, fixed remapped paths, stripped debug
information and a single codegen unit. Cargo home and target directories are
isolated from normal workstation caches/builds. The shipped binary is produced
by this build; a separately supplied binary is not accepted as corresponding
to a source snapshot merely because its version string matches.

## Artifact and reconstruction

The deterministic tar.gz contains the product binary, corresponding source,
exact dependency archives/vendor sources, build instructions, toolchain record
and a closed versioned distribution inventory. Archive entries are sorted,
with fixed epoch/modes, zero uid/gid and empty owner/group names. Absolute
source/build paths and nondeterministic logs are excluded from the archive;
the private run retains build logs and failed attempts.

```bash
python3 tools/package_forever.py verify \
  --archive /absolute/external/distribution/forever-extractor-linux.tar.gz

# After unpacking the verified archive into a new external directory:
python3 /absolute/external/unpacked/source/tools/package_forever.py rebuild \
  --bundle /absolute/external/unpacked \
  --output /absolute/external/new-rebuild-run
```

Verification checks safe names/types/topology, duplicates, the closed manifest,
exact inventory and every recorded length/hash/mode. It does not execute the
binary, establish cryptographic authenticity or prove client/runtime parity.
Input limits are 8,192 members, 32 MiB per selected source file, 64 MiB per
payload file, 4 MiB for the manifest and 256 MiB total payload bytes. These
are logical input/retained-payload limits, not an RSS bound; archive decoding
and retained payload buffers coexist. Unpacked verification admits count and
total size before each payload read and checks actual file modes. Unknown
files/directories, path-prefix collisions, noncanonical hidden tar headers
and additional gzip members reject.

Rebuilding requires the pinned compiler/linker and compatible system libraries;
this is not a container or a static executable. Actual ELF dependency/version
requirements are recorded by local acceptance. Do not infer compatibility with
all Linux distributions from a build on this workstation.

Product command and capability scope remains in [forever-product.md](forever-product.md).
A package verifier passing does not recover missing client inputs or validate
geometry inventories, placement associations, Detour topology or gameplay.

## Windows port gate

Current product dispatch, package storage, rooted VMap/MAPS sources and
navigation discovery have Linux-specific filesystem behavior. The read-only
portability survey identifies these source boundaries:

- `forever-extractor`: driver/verifier dispatch, loaded `/proc/self/exe`
  fingerprint and no-follow plan/context/key input;
- `forever-casc`: package staging/publication/recovery and LocalStore identity;
- `forever-vmap`: immutable rooted package source;
- `forever-navigation`: immutable terrain source and inventory discovery/run.

Pure table/geometry/navigation algorithms remain reusable. A Windows filesystem
profile must establish reliable file identity/link counts, restricted private
ACLs, rooted/no-reparse access, bounded held ancestor handles, case/device/ADS
path admission, locks, exclusive publication and directory-entry durability.
Hashing `current_exe()` by pathname alone does not establish the Linux loaded
executable guarantee. Removing platform guards would not implement these rules.

The active rustup installation remains Linux-only. Exact official Rust 1.98.0
Windows GNU std, Rust sources and Rust MinGW components are now staged in an
external sysroot, with SHA-256 checked against the installed channel manifest.
They support local cross compilation without modifying the active toolchain.
The new dependency-free platform crate uses original, scoped Win32 FFI because
Rust stable does not expose the required file identities/ChangeTime. Unsafe code
remains denied outside that module; no upstream implementation or cached
Windows registry package was imported. This foundation does not establish all
Windows filesystem guarantees. Supported Windows/NTFS runtime and failure
acceptance remain required before enabling that platform.

## Initial six-crate local acceptance — 2026-10-11

All 41 focused packaging tests and 82 contract tests pass. Independent review
rechecks 15 targeted regressions, including notice closure, moved rebuild,
output control-character injection and the archive/filesystem boundary cases.
All 13 notice pins independently match the installed toolchain.

Two fresh offline builds, including an output path with spaces, produce the
identical 2,673,443-byte archive and identical binary. A third build from the
moved/unpacked corresponding source with new Cargo home/target reproduces
that binary exactly. The inventory contains 532 files / 12,902,727 bytes,
including 475 selected project source files, exact crate archives/vendor
sources, project notices/provenance and 13 Rust notice documents (1,610,375
bytes). Counts exclude the inventory manifest itself.

- Archive SHA-256: `0fa6291bb31c15da4d1d1f3e508e9a676bec3292d76dcc9d988df0d3d978cce3`.
- Binary SHA-256: `90fc4b7f494465f7f1564195f50180e5a621063195e4a88c96116d6946177693`.

The shipped immutable binary passes 17 v1 and 20 v2 synthetic CLI scenarios.
Eight actual SIGKILL interruptions yield seven recoveries and one expected
protected-key-source refusal. Four synthetic packages retain all 110 artifact
hashes from the preceding accepted release-profile binary: 31 actual-tile,
28 empty-geometry, 26 empty-world and 25 build-70170/esES-subset artifacts.
This preserves prior bounded output evidence across the packaging compiler
settings without claiming new native/runtime execution.

The bounded real build-70205/esES DB2/GT/camera package reproduces all 33
previously accepted artifacts / 193,794 bytes; client build-info is unchanged
and no real encrypted key is supplied. There are 86 empty runtime traces and
eight interruption/fsync traces without network calls. Three compiler traces
each contain 13 matched local AF_UNIX socketpair/recvfrom exchanges, audited
separately rather than counted as empty runtime network evidence.

ELF NEEDED records are `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` and
`ld-linux-x86-64.so.2`; the recorded symbol-version set includes GLIBC_2.34.
This is a dynamically linked local candidate, without universal Linux ABI or
Windows compatibility. Production Rust and the Cargo dependency closure are
unchanged. No new C++ world-output, RustyCore runtime or full-client acceptance
is claimed, and nothing is published.

Retained final report: `distribution-20261011T160155Z.json`,
SHA-256 `a72e17bf3c056d3f721a46fba95c8f9daa9f6a497a73762f49c85ecd9a3b3d4c`, in the external acceptance reports directory.
Superseded prototypes/observation errors and the earlier candidate missing
required notices remain retained. This final documentation is recorded after
the immutable bundle's source snapshot and is not substituted into its archive.

## Seven-crate source profile and Windows foundation — 2026-10-11

The previous Linux corresponding-source recipe required
`forever-seven-crate-ci-source-v2`, including the dependency-free
`forever-platform` source, its README, the exact synthetic CI workflow, its
contract test and [CI documentation](forever-ci.md). The archive envelope remains
`forever-distribution-v1`. This source-profile change is explicit; earlier
six-crate archives use their bundled frozen verifier and are historical
acceptance, not inputs admitted by the new source profile. No registry package,
license pin, Linux build flag or existing production dependency edge is added.

`forever-platform` supplies the separate Windows NTFS read-only profile
`forever-windows-read-ntfs-v1`. See its
[API and safety limits](../crates/forever-platform/README.md). At this checkpoint it
was not consumed by the existing producers. Windows output/DACL, publication/durability,
recovery, discovery and loaded-executable identity remain unresolved; the
Windows product stays disabled. Wine is preliminary execution evidence and
cannot establish real Windows/NTFS or actual reparse/junction acceptance.

## Seven-crate acceptance — 2026-10-11

The retained `distribution-seven-20261011T162620Z-completed.json` report accepts
two identical archives and a moved-source offline rebuild. The binary remains
exactly `90fc4b7f494465f7f1564195f50180e5a621063195e4a88c96116d6946177693`,
matching the prior accepted six-component product. The archive contains 544
inventoried files and 487 selected project-source files. All 45 packaging and
109 contract checks pass; the packaged binary passes the existing 17 v1 plus
20 v2 CLI scenarios and reproduces the prior 33 real DB2/GT/camera artifacts.
Closing verification accepts 23 valid packages, rejects one deliberately altered
payload and validates 24 manifests/30 state protocols. Four prior synthetic
packages retain all 110 artifact inventories.

Build A was retained after an orchestration parser rejected an unfinished
`socketpair` trace line. The continuation normalizes unfinished/resumed calls
and independently admits all three compiler traces as 13 local IPC pairs each;
it does not rerun or relabel a failed compiler. Build A's process exited zero.
The initial parser-failure report and exact continuation are retained.

The frozen bundle contains the documentation snapshot used for those builds.
This executed-evidence append and updated toolchain-availability paragraph are
post-acceptance documentation updates, recorded separately from that bundle.
No publication or complete Windows distribution acceptance is implied.

## Windows consumer source profile v3 — 2026-10-11

The earlier v3 recipe requires `forever-seven-crate-windows-input-source-v3`.
The archive envelope remains `forever-distribution-v1`, with the same seven
workspace crates, two registry archives, license pins and Linux build recipe.
The exact lock graph now includes the Windows-only `forever-platform` dependency
from both `forever-casc` and `forever-extractor`. Missing or extra internal edges
reject even if the package-name inventory is otherwise unchanged.

The closed source inventory explicitly requires the platform path/lease/FFI
implementation, local-store and admission consumers, product input adapter, and
their Windows synthetic tests. These files must be shipped even though the
distribution product remains Linux. The new verifier rejects the earlier v2
source profile; the retained v2 archives continue to use their own frozen
verifier and their recorded acceptance is not relabeled as v3.

The product input adapter reads explicit normal absolute drive paths, holds
ancestor leases while reading, and rejects final hardlinks for both keys and
plans. Key input retains a read lease until consumed or dropped; a future
Windows output protocol still needs file-identity protection against case and
8.3 aliases. Large explicit inputs are read in chunks within the platform's
per-operation limit without reducing the caller's existing total limit.
Windows product publication, private DACLs, output durability/recovery, loaded
executable identity and complete extraction remain unaccepted. This source
profile change alone is not a fresh bundle/reproducibility acceptance.


### Corrected consumer source v3 Linux bundle

The later `distribution-v3-consumer-closeout-20261011T180333Z.json` (SHA-256
`7e3e61553a18d5050738b41e717a6b342836e1af3a192c8f67bea9f0caa66f75`)
is accepted and terminal. Two fresh archives and manifests match; the moved
source rebuild reproduces the same binary. The archive has 550 inventory files,
including 493 selected project source files, and is 2,721,409 bytes.

- Archive SHA-256: `4ab8fffa9b118e0d13fd6663950c83b272d2a0b69462b01cb87d6216a538855d`.
- Binary SHA-256: `ca528b696128622a0c9b624b351e0a18b34ece9abd43cb154821cf20201f27d5`.

This binary differs from the preceding `b8050261...` candidate. That closeout
records 17 v1 plus 20 v2 packaged synthetic CLI cases, eight actual interruptions,
seven recoveries and one expected protected-key-source refusal. Its inputs are
synthetic; it does not establish a new 33-file real-client result. Older real
acceptance remains tied to its own frozen binary and report. This v3 bundle does
not contain or accept the later producer source v4 increment below.

The subsequent `distribution-v3-bounded-real-20261011T180333Z.json` (SHA-256
`89a1a05a7acb41043a77875389d7194f1e9a10e293d0e4e1ce67e1854748685b`)
accepts 33 bounded DB2/GT/camera artifacts / 193,794 bytes with the corrected
`ca528b69...` binary. This is the new-binary real supplement absent from the
synthetic closeout itself. It does not build v4 or establish full extraction,
Windows or RustyCore runtime acceptance.

## Windows producer source profile v4 — 2026-10-11

The v4 recipe introduced `forever-seven-crate-windows-producer-source-v4`.
The envelope remains `forever-distribution-v1`, with the same seven workspace
crates, exact nine-package lock graph, two admitted registry archives/notices
and Linux build target. Existing Windows-only platform dependency edges remain
unchanged. V1/v2/v3 source archives retain their own frozen verifier and evidence;
the v4 verifier rejects a v3 source profile.

Mandatory source now also includes the original platform loaded-producer API
and scoped FFI/tests, pure extractor PE verifier/tests and loaded-helper example,
the exact `tools/seal_forever_windows.py` tool and its synthetic tests, and the
[producer contract and scope](forever-windows-producer.md). Missing source,
sealer, tests, contract or documentation rejects before compilation; missing
mandatory inventory rejects verification. No unrelated tool becomes allowlisted.

The separate producer profile binds a strict unsigned x64 PE32+ console file to
the actual running main module's volatile 64-byte `.rceid` payload and measured
payload start RVA. Only the digest slot is zeroed for canonical whole-file
SHA-256; success returns the complete sealed file SHA-256. Protected read leases
and explicit acquisition/work limits remain mandatory. The sealer's supplied
payload check is artifact integrity only; production loaded evidence comes from
the platform API and verifier helper.

The Linux packaging recipe ships this corresponding source but does not seal a
Windows executable or enable Windows product dispatch. Original source introduces
no new registry/native dependency. The historical external producer prototype's
29 checks and Wine observations do not accept this production integration.
Focused candidate checks pass 139 contract tests and 94 distribution tests
(53 packaging guards and 41 sealer cases), with all 921 source files unchanged.
These checks create no v4 bundle and execute no loaded Windows helper.
The corrected production helper separately passes six debug/release Wine cases,
with 37 Rust tests/profile, 41 Python cases and Windows all-target Clippy.
That is preliminary production-API execution, without actual Windows/NTFS or
enabled-product acceptance; report hashes and scope are in the producer doc.
A fresh reproducible v4 bundle was pending at that source checkpoint; the later
accepted bundle is recorded below.
Real Windows/NTFS, private output
DACL, publication/durability/recovery, complete real extraction and RustyCore
runtime acceptance remain open. No publication is authorized.

## Canonical producer closeout and Linux v4 bundle — 2026-10-11

The later accepted/terminal `windows-producer-closeout-20261011T182651Z.json`
(SHA256 `ae9e3761d6d1ab6848dec6c54f4739c53181eb47ba3f0334d528e900fcb2ddf6`)
keeps all 921 canonical sources unchanged. Debug/release each pass 2,130 Linux
tests (22 ignored, 41 targets) and 1,925 Windows GNU tests under Wine (20 ignored,
41 executables). Each Wine profile reuses 24 exactly SHA256-identical executables
with validated prior evidence and runs 17 fresh executables. Linux/Windows
workspace all-target Clippy, 139 contracts and 94 distribution checks pass.
Independent audit SHA256 is
`82891385f5903f74f3c6c462ffd45e24e2d8ac8f19770782861441d4222d2d78`.

`distribution-v4-producer-closeout-20261011T182651Z.json` (SHA256
`9f48f5537f3c8521f4b597cdc2fe208e582094e7028a1182859b91f5e2d62ad5`)
separately accepts two identical archives/manifests and the moved-source rebuild:

- Archive SHA256: `ab2649ea72fd6d6605b4377dd484e87128a89becec06a69c3fe9bbaa94aac919`.
- Binary SHA256: `42f1ed4fcae892af7cb8f22cb3ada0ad7165395ae500feefa483a45a59c6060b`.

Its 37 packaged synthetic CLI cases and eight interruptions preserve seven
recoveries plus one protected-source refusal. The subsequent accepted/terminal
`distribution-v4-bounded-real-20261011T182651Z.json` (SHA256
`a0e7a62e5cdfb54f0efb869f2f36898b653900cfb4437b2f1a6d34f807796613`)
accepts 33 bounded DB2/GT/camera artifact records / 193,794 bytes with that v4
binary, identical to the prior bounded artifacts. Build-info and 921 sources
remain unchanged. Earlier v3 bundles and failed/pre-correction reports retain
their original binaries, source snapshots and scope.

These results do not accept actual Windows/NTFS, enable the Windows product,
add real encrypted-key/native/runtime evidence or complete extraction. Filtered
seccomp traces record observed calls; denied attempts may be absent. Natural
launcher exit does not prove individual exit of every descendant.

## Windows VMap and input diagnostics source v5 — 2026-10-11

At the v5 source checkpoint the profile was
`forever-seven-crate-windows-vmap-diagnostics-source-v5`. The archive envelope
remains v1 with seven workspace crates, nine lock packages and the same two
registry archives/notices. The sole new dependency edge is Windows-only
`forever-vmap -> forever-platform`. Prior v1/v2/v3/v4 archives retain their
bundled frozen verifiers; the v5 verifier rejects their older source profiles.

The mandatory closure adds [Windows VMap read source](forever-windows-vmap.md),
[whole-span pure BLTE framing](forever-blte-framing.md), fixed required-input
route metadata, and their source/tests/contracts/docs. The route document is
relocated byte-for-byte from the worker's new crate-doc path to
`docs/forever-required-routes.md`, retaining author/date. Generic crate Markdown
and unrelated tools remain outside the allowlist. Omitted mandatory source or
inventory rejects; the exact VMap platform dependency edge is required.

Framing validates all framed encoded MD5s and outer N/Z/E structure, with
explicitly unhashed headerless evidence; it does not decrypt, inflate or prove
plaintext/CKey/EKey validity. Routes report metadata observations only, with
payload and ciphertext NotInspected. There is no route-to-byte adapter or key-only
sufficiency claim. The first routes Linux handoff is superseded after an archive
filename-format defect. Corrected decimal naming shares the acquisition path
constructor and passes 88 Linux tests/profile, Linux/Windows Clippy and Windows
cross-build. Accepted Linux handoff SHA256 is
`879b0be0fe82874f5d57a91722103114177f5e006ecfb73196d15fa7c55a80c4`;
Wine route runtime remains pending; the subsequent metadata-only real QA is
recorded below.
VMap's 28 Wine cases/profile remain preliminary, not actual NTFS/product proof.
No v5 bundle, complete real-input payload diagnosis, full extraction or
publication is accepted by this isolated source composition.


### Corrected required-route real metadata observation

The accepted/terminal `required-route-live-20261011T1901Z-attempt2.json` (SHA256
`83b4ad2ba3a939207f194dc3b1f287b6307c33cffcb304dc500b9abfd7ad6ec1`)
executes the corrected fixed-three-FDID diagnostic on Linux build-70205/esES.
Each input has one `Routed` candidate and `FitsObservedLength`:

| Input | Encoded record bytes | Declared decoded bytes |
| --- | ---: | ---: |
| Map | 5,042 | 10,181 |
| SkillLineAbility | 94,617 | 347,118 |
| GameObjectDisplayInfo | 74,592 | 116,054 |

Payload and ciphertext remain `NotInspected`. This observes admitted routing and
held-file extents; it does not read requested archive spans, inspect BLTE, consult
keys or establish complete ciphertext/plaintext. It cannot establish that a key
is the only missing requirement. Build-info, corrected source and canonical
sources remain unchanged. Attempt 1 failed in the external harness's toolchain
pin/lock setup before client execution and remains preserved. The relocated
route design document retains its original pre-QA author/date/content; this
later record supplies its first real metadata observation. This is not an
implemented route-to-framing adapter, repair plan or v5 package acceptance.

The isolated 935-source composition passes 156 static contract checks and 97
synthetic distribution checks (56 packaging guards plus 41 sealer cases), with
all source hashes unchanged. Its selected package-source closure contains 518
files. These checks perform no Rust integration runtime acceptance or v5 build.


## Required-input complete-span framing source v6 — 2026-10-11

The historical v6 mandatory profile was
`forever-seven-crate-windows-vmap-required-framing-source-v6`, retaining the v1
envelope and the v5 graph of seven crates, nine packages and two registry
archives. It adds the separate fixed-three-input key-free complete-span framing
API and its exact source, tests, contract and documentation. The product CLI
and Windows product remain disabled. Historical v4/v5 contracts and imported
design documents retain their initial scopes. The
[framing acceptance supplement](forever-required-framing-acceptance.md) records
the later real Linux observation and exact report hashes. No v6 bundle has been
built by this source increment.

The current key-free acquisition diagnostic for Map, SkillLineAbility and
GameObjectDisplayInfo rejects all three with `local_store_no_verified_encoding`,
each encrypted 1 and missing/incomplete/corrupt/unsupported 0 (report SHA256
`d1979922ac941a5c7363f7e014f023f47cffd8372523df49d4d35bd6c5c42536`).
A separate accepted/terminal framing run (SHA256
`43f2ccf6a92a4d9e33239be3cc3e0e3de87bb43d6b69d7e9baea93c515ed3f01`)
returns one `Complete(FramedMd5)` candidate each: 15 frames, 6 Z, 9 E and 15
verified encoded-frame MD5s. The bounded adapter reads complete records, ignores
30 prelude bytes, checks only the nine-byte index prefix, and retains final
guards on all results. The 4096-frame cap is per candidate BLTE object, not
aggregated per FDID. No full EKey/CKey, Z interior, E inner mode, ciphertext
interior or plaintext validity is established. Key sufficiency remains unknown.

Earlier `local_file_incomplete` statements for these three inputs describe
their historical attempts; they are not the current acquisition category. Old
failed manifests remain failed. No newly decoded input or typed-loader success
is established. Complete Birth/initialization/GameObject real acceptance, full
geometry/runtime, actual Windows/NTFS and product completion remain pending.
The 935-source v5 canonical merge is accepted as source integration only
(closeout SHA256
`dc55520190ea017eedda82436469cd8199f2079d07a95fb28a4a4d04c63be77f`).
This adds no v5/v6 bundle, publication or hosted CI result.


## Windows navigation reader source profile v7 (2026-10-11)

The current corresponding-source profile is
`forever-seven-crate-windows-navigation-readers-source-v7`. Its exact nine-package
closure adds only the local Windows `forever-navigation` to `forever-platform`
edge. The v4/v5/v6 source profiles and their frozen verifiers remain historical;
this verifier requires v7 and all mandatory navigation source, synthetic tests,
contract and scope documentation. The Linux build/publication recipe is unchanged.

The frozen implementation document `docs/forever-windows-navigation-readers.md`
retains its pre-execution evidence wording. The subsequent corrected NAV-1/NAV-2
source passed 52 focused Linux tests (three ignored) per profile and GNU all-target
Clippy. The finite-server Wine diagnostic passed all 56 synthetic cases in each
profile (112 total), with no timeout or NotRun. These are synthetic reader and
sequential memory-sink semantics; no real geometry or Windows product is enabled.

Overall runtime acceptance remains **false**: 783 nonlocal NETLINK attempts were
visible and denied by the unchanged AF_UNIX filter, and an independent audit found
40 unresolved `???(...)` syscall bodies across all 225 traces. The original report's
`trace_evidence_complete=true` is rejected by root and is preserved historically,
with sidecar qualification; zero recognized IP attempts is not complete zero-network
attempt evidence. No unknown syscall is assumed safe or nonnetwork. Actual NTFS,
Windows product/runtime, full extraction and publication remain unaccepted.

The current contract `contracts/forever-windows-navigation-contract-v1.json` pins
the original report `52b351af1de73424d3a8fac6420d540b9356cf4f62a49a53af656db35b612edb`,
root semantic review `faa6af302012c98a517e52f730522760df97467054f3d901561f834681742041`,
root trace audit `6568c256083139c7a392755b28a8115e6c49639f77878781dd7a830fc4919da3`
and independent qualification `5e3c359460ce4dc83c47deb8d0f04bbbdd4405ea633e520fb9d0b65f6b83f508`.
Static contract checks establish source consistency only. No new v7 bundle or
rebuild has been executed at this source-union checkpoint.


## Linux private collision archive source profile v8 (2026-10-12)

The current mandatory corresponding-source profile is
`forever-seven-crate-linux-collision-leased-archive-source-v8`. GNU/Linux x86_64
write leases protect one private append archive per owner. All six actual
RAW/WDT/cache/placement/VMO/map owners bind the same backend, volume and resource
policies. FileSetV1 remains default. Complete framing/digests, source/namespace /
mount guards and terminal poisoning remain. Geometric descriptors and constant
archive guard counts do not claim whole-world linear work or throughput.

The disk-profile cache parser allowance is reduced to 64 MiB in the inventory
and world runner; no memory ceiling is raised. Metadata-only fixtures
force distinct UNIX timestamps. Production reader behavior is unchanged.
The complete captured root plus helper reservation is admitted before archive
creation; a downward-budget regression retains only the empty private namespace.

Frozen executable/test source passes 2,599 ordinary tests across 41 targets per
Linux debug/release profile (26 ignored), four explicitly executed ignored
corpora per profile, 180 contracts and 108 distribution/sealer checks. Workspace
all-target Linux/GNU-Windows Clippy, both Windows compile profiles and scoped
formatting pass. Each 4,097-record corpus performs 20,485 archive guards with
bounded steady FDs. Four complete six-owner M2/WMO/parent packages match 27
resident-Rust artifacts / 2,684 bytes; one exact terminal nearer-parent
Association is checked, also on a second filesystem. All 27 command traces,
immutable source/tools and live terminal observations pass independent review.
Earlier failed reports remain retained and rejected.

Seven crates, nine packages and two exact registry archives remain. A Linux-local
CASC-to-platform edge and 28 mandatory source paths enter v8. Historical v7
contracts/import pins retain their scope. No new bundle/rebuild, Windows runtime,
native/client/runtime or product acceptance is established. The product still
uses resident assembly. Physical-disk preflight, publication/resume integration,
encrypted mandatory complete inputs, real geometry/runtime and full distribution
remain gates. The extractor is not complete. See
`docs/forever-collision-leased-archive.md` for hashes and retained history.
