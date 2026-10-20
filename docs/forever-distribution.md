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
information, one Cargo build job and a single codegen unit. The build environment
sets `CARGO_BUILD_JOBS=1`, overriding inherited values. Cargo home and target
directories are isolated from normal workstation caches/builds. This build
produces the shipped binary; a separately supplied binary is not accepted as
corresponding to a source snapshot merely because its version string matches.

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


## Explicit product collision disk source profile v9 (2026-10-12)

The mandatory corresponding-source profile is now
`forever-seven-crate-linux-product-collision-disk-source-v9`. It adds the explicit
product plan v3 parser, Linux GNU x86_64 borrowed-directory space observation,
guarded disk driver, CLI integration, synthetic checks, contract and scope
documentation. Seven crates, nine lock packages and two registry archives remain;
the extractor additionally has a local Linux platform edge. Historical v7/v8
profiles and their frozen verifiers retain their original claims.

The new plan selects `linux-leased-append-disk-world-v1`, seven downward byte
budgets and an explicit `--scratch` for extract/resume. Six fresh private leased
append owners finish the checked package before common product collision writes.
Unfinished collision resume uses a new scratch namespace; no archive adoption or
cleanup is implied. Physical preflight aggregates same-device allowances, but is
not an atomic reservation or capacity guarantee. Common Run live capacity is
reserved without raising conversion memory ceilings. Resident v1/v2 stays default.

Executable validation passes 2,643 ordinary Linux tests per debug/release
profile (41 targets, 26 ignored), four explicit ignored corpora per profile,
49 existing CLI cases per profile, all-target Linux/GNU-Windows Clippy, both
GNU-Windows compile profiles and scoped formatting. Final source association
reruns 187 contract and 112 distribution/sealer checks plus one added
cross-volume CLI case per profile against the exact full-run product binaries.
All Rust/Cargo and existing CLI-test source is unchanged from the full-run freeze.
This is synthetic GNU/Linux product/source acceptance; no new bundle,
real-client/native acceptance, Windows product or RustyCore runtime acceptance
is claimed. Complete encrypted inputs, real geometry, runtime/server integration
and reproducible distribution remain gates. The extractor is not complete. See
`docs/forever-product-disk.md` and the versioned disk contract.


## Accepted Linux v9 bundle of b692ed3 — 2026-10-12

A subsequent terminal run closes the bounded Linux reproducibility gate for
commit `b692ed3eb189f790dbf79609f8cd0958411ffac6`, source freeze
`495d58c7ab7345b5c65461d235c66bb1efd7c3bc772aa8e3872c5b5d179f8358`.
Two fresh builds produce identical archives/manifests/binaries; a moved-source
offline rebuild reproduces the binary. All 50 shipped v1/v2/v3 synthetic CLI
cases pass, with 129 retained actual CLI traces and 33 complete package contexts
bound to the shipped binary. The outer Python CLI harness is not syscall traced.
Archive SHA-256 is
`a99989e79e956822fb80d19668890096e6b1879093eac71ecf82e7d3606f91ab`;
independent terminal review SHA-256 is
`53b660e03bed16187bf4580e1921fc18ba7587e49a89300ac7d085dd61336d5e`.
Complete hashes, runtime dependencies, resource observations and qualified scope
are recorded in `docs/forever-product-disk.md` under this same heading.

This is acceptance of the frozen `b692ed3` bundle. Its contract flags
`bundle_built=false`, `validation.v9_reproducible_bundle_accepted=false` and the
pending distribution record retain their preceding checkpoint values. The later
documentation commit is not rebuilt and replaces no accepted archive bytes.
Historical source profiles and reports retain their original scope. Real complete
inputs/geometry, full-world resources, RustyCore runtime and complete Windows
product/distribution acceptance remain gates. The extractor is not complete.


## Product v4/v5 DB2 catalog source profile v10 — 2026-10-13

The current mandatory corresponding-source profile is
`forever-seven-crate-linux-product-db2-catalog-source-v10`. The archive envelope
remains `forever-distribution-v1`; the seven workspace crates, nine lock packages,
two exact registry archives, thirteen toolchain notices and Linux build recipe
are unchanged. The accepted v9 bundle of `b692ed3` above remains historical,
verified with its own frozen tool. The v10 verifier does not admit its older
source profile or replace its archive bytes.

The source closure now explicitly requires both generated WoWDBDefs catalogs,
their provenance JSON, product v4/v5 contracts, generator, DB2 reader/validator
sources and tests, and the [regular-section scope](forever-db2-regular-sections.md).
Product v4 binds build 70205 to `wowdbdefs-70205-v1`; product v5 binds build 70338
to the separate `wowdbdefs-70338-v1` catalog and allows only DB2. See
[the product scope](forever-product.md). Raw-text and exact three-table
sparse-parent admission preserve complete source bytes without projecting the
affected rows or parent associations. No other build-70338 component or complete
catalog acceptance follows from shipping this source.

`contracts/WOWDBDEFS-LICENSE.md` is the sole admitted Markdown file under
`contracts/`; other Markdown files there still reject. Its attribution includes
both source commits and is mandatory, alongside
`tools/generate_wowdbdefs_catalog.py`. The previously JSON-only directory rule
would reject this existing attribution document before compilation. Frozen
Rust/catalog facts are sufficient for the shipped offline build. Regenerating
metadata separately requires the already pinned external WoWDBDefs checkout;
the packager neither acquires nor bundles that checkout. Original project and
Rust notices remain required, including the complete existing CC-BY-SA-4.0 text
under `toolchain-notices/licenses/`.

The source-only checkpoint repaired corresponding-source selection and
versioning. Its authored contract retains the pre-execution acceptance flags;
the subsequent executed evidence is recorded below. Historical v1-v3 cases
remain required.
The 83 encrypted build-70338 candidates, including Map, still block complete
catalog/full-client acceptance; full geometry and RustyCore runtime acceptance,
Windows product distribution and publication authorization remain separate gates.

### Executed local Linux v10 acceptance — 2026-10-13

Two fresh offline builds, including an output path with spaces, produce the
same 3,667,182-byte archive and 3,320,032-byte binary. Both archives pass the
closed verifier. A third offline build from the verified, moved/unpacked bundle
uses its own shipped helper/source/vendor archives and reproduces that binary.
The exact source freeze contains 616 selected files / 15,136,381 bytes; the
complete distribution inventory contains 673 files / 20,474,652 bytes. Source
identity is unchanged before/after all builds and executable checks.

- Archive SHA-256: `8ddf5b58580b8a87ceda6f81253d88ec36a3044d1c0e5192d00ae28db29fad80`.
- Binary SHA-256: `ae9f4a984246e935dc983e050c9a6eb5326f4fe571b9e165986c004a54ce4ee5`.
- Selected-source SHA-256: `ef3eeb8812ba95822c9b56d103ebbd483802506245ae76d572df180bbff50bdf`.

The shipped binary passes 64 synthetic test methods: fourteen v4/v5 catalog
fixture/CLI checks, seventeen v1, twenty v2 and thirteen v3. The new catalog
suite writes its WDC5 bytes with an independent literal writer and reuses only
the existing toy CASC container writer; that helper's transitively imported
DB2 writer does not generate these fixtures. It verifies exact numeric/text
multisection source bytes, schema/build/FDID/hash gates and all-or-error rejection
of compressed text, including empty sections. Its four complete synthetic
packages and 37 actual CLI traces are separate from real-client acceptance.

All 166 runtime traces contain no network syscalls: 155 are empty and eleven
contain only expected fsync/SIGKILL interruption events. Each compiler trace
contains fourteen matched AF_UNIX socket pairs and fourteen empty receptions
bound by PID/descriptor/inodes, with local signals classified separately.
These compiler traces are not counted as empty network traces. Three initial
classifier rejections for line reconstruction, compiler signals and interrupted
fsync remain retained; the original builds and suites all exited successfully
and were reused without rewriting their evidence.

At the same source freeze, Linux workspace debug passes 2,691 tests with 26
existing ignored cases across 42 summaries; workspace all-target Clippy with
warnings denied and formatting pass. All 248 contracts, 118 distribution/sealer
checks and 185 Python oracle cases pass, with one oracle case skipped. Earlier
shell-PATH and Clippy failures remain retained separately. No new native or
Windows execution follows from these Linux checks.

The binary retains the recorded dynamic NEEDED closure (`libgcc_s.so.1`,
`libm.so.6`, `libc.so.6`, `ld-linux-x86-64.so.2`), including GLIBC_2.34 symbol
requirements. This is a local Linux candidate, with no universal ABI, full
extraction or gameplay/runtime claim. Nothing is published. Later documentation
updates are not part of the compiled source snapshot above.

Retained evidence root:
`/mnt/nvme2/datos/forever-product-runs/distribution-v10-root-oxp5imur/`.
Bundle acceptance report: `bundle-evidence/acceptance.json`, SHA-256
`d90e92159445eb2543be19f37673dba47a92237e823a1ce761773e6e1d79c243`.
Independent coordinator review: `root-bundle-review.json`, SHA-256
`78a157f5ac2942923bb59dd3563a8ec786572ca2efa01d617f9180a79cdaa77d`.
Source validation report: `core-validation.json`, SHA-256
`7178fbc2ef9962950794b22aecf93ccd09c48dcdec4d1abed3bc891606037bff`.

### Packaged real v5 selection — 2026-10-13

The immutable shipped binary above also passes plan inspection, real extraction
and separate verification for a new explicit build-70338/esES selection of all
1,078 files acquired completely in the fresh census. All 1,078 output paths,
sizes and SHA-256 digests exactly match that census: 29,213,383 bytes, with a
maximum input of 4,844,598 bytes. Product limits are unchanged, decryption is
disabled, and all three actual process network traces are empty under syscall
denial. The manifest binds the exact shipped producer and plan; binary,
catalog and client build-info hashes are identical before/after the run.

This independently selected plan does not include the 83 failed acquisitions.
The 1,161-table plan and its failures remain separate. The run performs no new
row-oracle comparison, accepts no real multisection input and supplies no
geometry or runtime evidence. Outputs remain private and are absent from the
distribution and Git.

Retained real report:
`/mnt/nvme2/datos/forever-product-runs/catalog-v5-packaged-70338-20261013-q52a5tcu/report.json`,
SHA-256 `b56e7724ef2335876069a0588312553ed8ea555d731082dc3ca4eb644bd7f7ec`.
Independent output review: `real-v5-root-review.json` in the distribution
evidence root, SHA-256
`9167a8a204cf5ffd014ebe11f0924c7b1a351df9015077e6acf9eb1fe6ef9afd`.

### Executed v6 product bundle with the v10 source recipe — 2026-10-13

The frozen functional commit `6f1a6fa2f17fb174ec74cbd7b9440b026b66e973`
adds the explicit v6 inline navigation plan. PR #21 merged it into `forever`
as `62093c477bfd72a1c12ea71db425912f3383838a`; both commits have Git tree
`0a8363e0ae1a426bae9ad1307e6a8cbab3129a78`. The unchanged
`forever-seven-crate-linux-product-db2-catalog-source-v10` recipe already
selects the nine new shipped source/contract/fixture paths. No dependency,
toolchain, license or source-profile version changes are needed for this exact
snapshot. This does not make those paths mandatory in older v10 snapshots.

Two fresh offline builds, including a path with spaces, produce identical
3,697,053-byte archives and 3,330,656-byte binaries. Both archives pass the
closed verifier. A moved-source offline rebuild reproduces the binary exactly.
The complete inventory contains 682 files; selected source contains 625 files /
15,234,939 bytes and all thirteen pinned toolchain notice files are checked.
The unrelated untracked configuration backup is excluded.

- Archive SHA-256: `258c8fe40b799048b8684fe68548c91df9aabdbcd7b4dcc76a87d74f66adeb6e`.
- Binary SHA-256: `a318428a0ef70eb3fa36ab6a34325a2ba9bf495f137a6797e4cde8f3b4d9fdbf`.
- Selected-source SHA-256: `6afeac8b76a9e9fda02bcf47dd53ae3b84e32b301abf16cb4dd8df4b2fa3f281`.
- Source-freeze report SHA-256: `80ef60f24e38a3c4cab594e9eebeb293a89409fe970879ac8d9bed8717c121d6`.

The immutable shipped binary passes 72 synthetic CLI methods without skips:
seventeen v1, twenty v2, thirteen v3, fourteen v4/v5 and eight v6. The v6
methods execute eight original Detour consumer calls / sixteen routes and one
exact original TileBuilder comparison, including provided-key and leased-disk
modalities and interrupted recovery. The 371 retained native source, object,
report and executable guards remain unchanged; fixture inputs have separate
per-case guards. The previously failed flat
fixture and its routability limitation remain recorded in the product evidence;
this new run does not repair or supersede that negative result.

There are 228 runtime traces including help: 215 empty, twelve expected SIGKILL
traces and one baseline fsync trace, with no network calls. Compiler traces
separately retain fourteen matched local AF_UNIX pairs and fourteen empty
receptions per build, with local signals classified; they are not empty network
traces. The outer Python harness is outside the syscall-traced runtime scope.
The dynamic NEEDED closure remains `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`
and `ld-linux-x86-64.so.2`, with maximum observed GLIBC symbol version 2.34;
no universal Linux ABI claim follows.

An independent coordinator review checks both archives, the shipped and rebuilt
binaries, all 625 current source files, eight v6 case-report hashes, 228 runtime
traces and all 371 native guards. It reuses retained evidence without repeating
builds or native execution. Evidence root:
`/mnt/nvme2/datos/forever-product-runs/distribution-v6-6f1a6fa-8vfh8hk3/`.

| Report | SHA-256 |
| --- | --- |
| `acceptance.json` | `cdf2d4a0c66f95509e1770a5c3bbc6e6557c298c3dacae4494f7fbcdbd8fedeb` |
| `final-review.json` | `aa2eed54489876e01265026be8be46505b6d550eb11801a4c95a4507fbdcd290` |
| `root-review.json` | `c03acb6055ec0feea3c206f60e421d87b165eceb62906e1a88ca878b1412d51a` |

This accepts a local Linux candidate for the frozen functional source. Later
documentation is not rebuilt and replaces none of its bytes. The real 1,078-table
v5 selection above remains bound to its older `ae9f4a98…` binary and is not
transferred to this new binary. Build 70338 remains DB2-only; v6 geometry stays
restricted to builds 70170/70205. Real complete geometry/runtime, the 83 encrypted
70338 inputs, Windows product/distribution and full extraction remain open.
No client assets or keys are packaged and nothing is published.


### Executed current Linux v8 bundle with the v10 recipe — 2026-10-13

Issue #36 accepts the local Linux package of functional commit
`2452a7e16fade9a08cd71b6a901f83ab488b1a28`, tree
`abbc124bb137fff493801cb7b06bc110de83afb6`, under unchanged source profile
`forever-seven-crate-linux-product-db2-catalog-source-v10`. The existing recipe
selects current v8/product-links sources and the liquid diagnostic without a
profile, dependency, toolchain or production-format change. The focused QA
change accepts `RCE_MAP_DOMAIN_GRID_SOURCE` as an explicit absolute GridMap
source path whose SHA-256 must match the retained original receipt. The absent
variable keeps the historical default; explicit invalid input has no fallback.
This makes the shipped suite executable from a moved source tree.

Two fresh offline builds, including a path with spaces, produce identical
3,879,295-byte archives, manifests and 3,368,520-byte binaries. Both archives
pass the closed verifier. A moved extracted-source offline rebuild with isolated
HOME/CARGO_HOME/target reproduces the binary exactly. The inventory contains 720
payloads; selected source contains 663 files / 15,928,610 bytes, each byte equal
to the committed snapshot. The seven-crate closure retains two registry
archives, 38 vendor files and thirteen checked toolchain notices. The unrelated
untracked configuration backup is excluded.

| Artifact | SHA-256 |
| --- | --- |
| Archive | `10c241fbb74fca7209c69fa27b461fe2ee75e59ecb8317b672592d6daf79dd00` |
| Shipped and moved-rebuilt binary | `0ea8fc2b4e0b73be63f099343b8e162c8feafa8b41526e25e950e78bd8dc5470` |
| Selected source | `abfe4131f32e5f7b8297f2c4f23a8aeb16aee19d74a4d0d45b84073d1533648a` |
| Source-freeze report | `d73492db8f6c54881901d97d79eed31b5c64445f71dae55d1f6757764a9c5767` |

The immutable shipped suites pass 129 methods without skips: seventeen v1,
twenty v2, thirteen v3, fourteen v4/v5, eight v6, twenty output-links, twenty-nine
v8 and eight GridSource guards. Ten are pure fixture/path guards (two catalog
and eight GridSource); 129 is not a CLI-process count. The v8 execution includes
exact SIGKILL/resume, selected domain/ancestor outputs, strict association,
unrelated-source exclusion and verification. Its results match the immutable
PR #33 receipt in all 29 cases and 33 authenticated manifest inventories,
with 741 artifact visits / 5,181,615 exact bytes; producer SHA-256 is the only
context difference. Rehashed rejected negative inventories are included, so
these are not 33 accepted packages. Fresh and resumed manifests/artifacts agree
byte for byte.

Original supplied-input comparison executes six collision assembly cases /
72 exact files / 8,468 bytes, one original ConvertADT output / 49 actual GridMap
queries, eight original Detour consumer calls / sixteen routes and one exact
original TileBuilder comparison. Retained authored synthetic inputs and memory
adapters preserve their scopes: no native CASC routing, native map selection,
malformed-native parity or complete original C++ navigation pipeline is implied.
All 823 native/dependency/report guards, selected sources, binaries and separate
input guards remain unchanged.

The complete runtime trace inventory has 407 entries: 393 empty, thirteen
expected SIGKILL and one fsync baseline; full-line review finds zero network
events or unknown bodies. Each of the three compiler traces separately retains
fourteen same-PID AF_UNIX pairs and fourteen empty receptions, with local
signals classified. These are local compiler IPC, not empty runtime traces.
Outer Python orchestration and ELF inspection are outside traced child scope.

The actual ELF requires `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` and
`ld-linux-x86-64.so.2`, with interpreter `/lib64/ld-linux-x86-64.so.2` and maximum
observed GLIBC symbol version 2.34. Compatible external linker/runtime is still
required; no universal Linux ABI or static-linkage acceptance is claimed.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-current-bundle-20261013-27rcxr5j/`.

| Report | SHA-256 |
| --- | --- |
| `qa/attempt1/acceptance.json` | `63ddf83a328d64ad10e28b1e6dfaf03b9ae93bc03bfdbabefea6fe76ce4985fa` |
| `qa/review/final-review.json` | `366efe53c99addd2a8012a17166474f545ccc0a515ebf58b8fb0404b3d67765f` |
| `qa/attempt1/pr33-byte-context-resume-comparison.json` | `32e058135dc518123e50c0e1a284ce35df286db2581a24c45f17383fae2bba95` |

The independent final review checks both closed archives, full source bytes,
the moved rebuild, all runtime/compiler traces and external/native receipts
without repeating builds or native execution. This accepts bounded current
Linux distribution for the frozen source. The subsequent three-file Markdown
delta (`AGENTS.md`, `docs/forever-completion.md`, this document) is not rebuilt
and does not replace any compiled/package bytes. The historical real 1,078-table
v5 acceptance remains bound to its prior binary and is not transferred here.
Build 70338 stays DB2-only; v8 geometry stays restricted to 70170/70205. Real
complete geometry, full-world resources, RustyCore/server runtime and complete
extraction remain open. Windows development/distribution stays deferred. No
client assets or keys are packaged and nothing is published.


### Current Linux product v9 local bundle — 2026-10-14

Issue #50 uses the unchanged
`forever-seven-crate-linux-product-db2-catalog-source-v10` recipe for functional
commit `0ad65fe5fe0eeccc17e7b896572212299ac53b8e`, tree
`6825f32ff1b21048390547a32f5661f54606f84d`. The corresponding selected
source is 698 files / 16,238,802 bytes; the complete packaged source is
739 files, including 41 vendor/configuration/archive additions. This source
selection is distinct from the preceding product's 665 compiled-source guard
map. Historical source proofs remain bound to their Git snapshots and are
not merged into the changed live checkout.

Six packaging commands perform two isolated fresh offline builds (one under
a path with spaces), verify both archives, verify the moved extraction and
rebuild it offline with fresh Cargo home/target paths. All three Cargo builds
produce the same 3,409,472-byte binary; the two archives/manifests are exact,
and the moved-source reconstruction reproduces that binary. Each archive
is 3,959,339 bytes with 755 payloads / 756 total members. The existing pinned
registry, toolchain/license/notices and dynamic Linux linkage requirements
retain their scopes; no client files, keys or native reference sources are
added to the distribution. This is a local candidate, not publication.

Static inspection of this binary records interpreter `/lib64/ld-linux-x86-64.so.2`,
needed `libgcc_s.so.1`, `libm.so.6`, `libc.so.6` and `ld-linux-x86-64.so.2`,
and maximum GLIBC symbol version 2.34. This remains dynamic Linux linkage,
without universal Linux ABI or atomic loader-binding acceptance.

| Artifact or report | SHA-256 |
| --- | --- |
| Archive | `275d90cf04ceab2a27e2f63e709b51063672903446a79e8a7c6c7d58221599da` |
| Shipped and moved-rebuilt binary | `3e9c98e37d3ea98a246b225d6cde6a831464254523a4fa04308c93419f867aea` |
| Independent packaging review | `72a9f95487a5bf0303c5e0aec6fefd581cfebd1ddc19549c76adfdfb6e2122a5` |
| Independent shipped-runtime review | `697fa300db20a4d1a966576d0b2eedd7d57d59bdac52b354fd21679a17633065` |
| Packaged real acceptance | `b13c8ef6d58cef3e6110a7c37e2836aeaef29b94f349147d470c8a5c0fb06daf` |
| Independent final REAL review | `6e5d0cad73c14ac6749d972626bd20795497157a507259da2b5c60638751484c` |

Both actual roles, **shipped** and **moved-rebuilt**, execute all 129 existing
methods and the new v9 method: 130 per role, 260 across eighteen stages,
zero skips. The 892 measured consumer calls comprise 814 legacy calls and
78 new v9 calls. Each new method has 39 outcomes: fifteen exit0, twenty-two
expected exit9 and two intentional kills; four complete positive packages
and twelve negative copies are retained per role. These roles identify shipped and
moved/rebuilt package locations and bind the current frozen binary. Historical
binary acceptance retains its recorded scope.

The complete consumer trace inventory contains 858 empty and 34 fully
classified nonempty traces, including thirty expected kills, with zero
network calls. Compiler local IPC and outer orchestration are separate scopes.
Sixteen deliberate special nodes remain in owned evidence: regular files are
hashed, special-node metadata and bounded symlink target text are retained
without following links or opening FIFO/device/socket payloads. A reviewed
evidence-only profile admits 65,536 nodes per root and 262,144 retained nodes
globally; 66,546 were retained. Rechecks reuse each original quota without
double charging. One comparison tree can overlap retention, giving a declared
maximum of 327,680 node records, not a heap/RSS or cumulative-visit bound.
The per-root allowance increases eightfold from 8,192; the global retained bound reduces
from the prior theoretical 294,912. Strict source admission remains 8,192
nodes per root; all source/product byte caps remain unchanged. Both earlier
failed runtime attempts are preserved, without accepted prefixes.

The subsequent real campaign completed 26 expected outcomes (22 exit0 and
four intentional SIGKILLs) across both roles. Six clean/GT-resumed/camera-resumed
views each contain the same 31 files / 165,507 bytes: twenty GT, CinematicCamera
DB2 and ten opaque cameras. Their complete bytes match the frozen accepted
component oracles. Disabled-key build-70338/esES explicit selection remains
the real scope; original/native consumers, compilers and real calibration
are not rerun. Final independent REAL review status: `passed`.
The independent real audit checks twenty-two empty and four classified kill
traces, with no consumer network calls. These output counts are not measured CASC acquisitions, physical I/O or RSS.

Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-70338-bundle-20261014-djny1lvy/`.
The later documentation-only update is not rebuilt and substitutes no bytes
into the immutable archive. Full catalog acquisition, complete Map/GameObject
and real geometry inputs, full-world operation and RustyCore/server runtime
remain separate Linux gates. Windows development/distribution stays deferred.
No release or publication is made, and the extractor is not complete.


### Current Linux product v11 local packaging — 2026-10-14

Issue #62 follows merged product #60 using the existing `forever-distribution-v1`
wire format and `forever-seven-crate-linux-product-db2-gt-cameras-source-v11`
recipe. Baseline `fdba4b2a64d1a72ae4f24fd86649cf5449de88be`, tree
`2d2a7cd865b0f85c9b6519ebf9b40887ee84bb98`, selects 717 committed files /
18,639,856 bytes. The complete packaged source contains 758 files, including
41 existing vendor/configuration/archive additions. All four copied source
trees preserve the selected baseline; the protected unrelated backup is neither
opened nor packaged.

Six packaging operations complete three isolated offline Cargo builds. Two fresh
builds, including a path with spaces, produce identical archives, manifests and
binaries. Verified extraction to a moved directory and reconstruction with fresh
Cargo home/target directories reproduce the binary using bundled corresponding
source/dependencies and the installed pinned toolchain/system linker. The archive
contains 774 payloads / 775 members. No client assets, keys, extraction outputs or
native reference sources are packaged; no dependency or output format changes.

| Artifact or receipt | SHA-256 |
| --- | --- |
| Archive, 4,103,587 bytes | `cdea14babeca67b92513acf48c90bceeff0306d35042124348029c432500f2da` |
| Shipped/rebuilt executable, 3,418,192 bytes | `cc4b171521ecff81b634873530ac0a51259baa7653cea4fb895f519a5d83571f` |
| Selected-source freeze | `315567cb37c5de32f5d8deb291e4f7b9b3ee9a5d99434dc87590fa80f2d366e9` |
| Packaging acceptance | `6bc48cff13fc9e25b9d5dfee3f0abc7dd6740ee579fbd00f8a3e4bc40e2c4e89` |
| Independent packaging review | `593c72617dab08c60540b10d5d36bb70670154aff5d0cceb6c84a00d57ac02cf` |
| Actual packaged producer freeze | `8506695ea5e18cf9b190b20b5c741bb34c500d847eb4a0bbba35bacd6f57602e` |

Static ELF inspection records `/lib64/ld-linux-x86-64.so.2`, needed
`libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, `ld-linux-x86-64.so.2`, and maximum
GLIBC symbol version 2.34. This remains a dynamically linked local Linux
candidate, without universal Linux ABI acceptance. Each compiler trace contains
fourteen matched local Unix socket exchanges; the two archive-verifier traces
are empty. Unpacked verification API and outer orchestration are untraced.
Compiler IPC is separate from subsequent consumer network observations.

Packaging is independently accepted. Current packaged synthetic regressions and
v11 clean/resume/verification campaigns are pending at this packaging checkpoint;
historical binaries retain their own acceptance. The later documentation update
does not rebuild or substitute bytes into this immutable archive. Evidence root:
`/mnt/nvme2/datos/forever-product-runs/linux-v11-distribution-20261014-l3ak9p_u/`.
Full catalog, Map/GameObject inputs, real geometry/world resources, RustyCore
runtime and publication remain separate gates. Windows stays deferred in draft
PR #27; this does not complete the extractor.


#### Subsequent packaged v11 lifecycle acceptance

The unchanged package above now passes two separate current-producer campaigns
against its actual **shipped** and **moved-rebuilt** executable paths. The
external v11 controller is source-reviewed; five reused support/fixture modules
remain byte-identical to issue #60. Its first unexecuted proposal had an invalid
full-fact metadata reference; revision2 projects the strict path/SHA reference.
No failed product run is hidden or promoted by that source correction.

The synthetic campaign closes 56 outcomes: 44 exit0, eight deliberate SIGKILLs
and four expected pre-I/O key-policy rejections. Both key policies and both
package roles pass, with twelve complete authored views of 24 files / 1,632
bytes. Each role/policy has fresh synthetic calibration, observed at GT ordinal
27 and camera ordinal 170. Immediate cut snapshots retain the sealed DB2 source,
and camera cuts also retain the sealed GT component. Clean/resumed manifests
match within and across the two byte-identical executable roles.

A separately admitted real campaign first checks fresh bounded client metadata
against the original 70374 Phase A bytes/path; it does not rebase those facts.
Thirty-two explicitly typed retained inputs comprise 31 artifact oracles plus
the private ten-camera selection. Their existing per-file/selection caps stay
separate from software guards and QA hash-visit bounds. Disabled-key
70374/esES extraction closes 26 expected outcomes, including four calibrated
cuts, and six complete clean/GT-resumed/camera-resumed views. Each has 31 files /
165,507 bytes: one DB2 source, twenty GT files and ten opaque cameras. The
controller checks 186 complete output-to-oracle byte comparisons; independent
QA separately checks 186 hash/length comparisons. It also checks exact
manifests, seals, traces, captures, source/producer guards and owned groups.
No native/original consumer, compiler or real-calibration rerun occurs.

Both whole campaigns pass independent terminal QA, without accepted prefixes.
Their child traces have zero network calls; direct parent orchestration is
untraced. Evidence freezes retain 1,104 files / 113 directories for synthetic
and 602 files / 49 directories for real, excluding each freeze itself. Historical
candidate text marked pending remains preserved beside the actual root approvals.
Private IDs, per-file correlations and key material are not public evidence.

| Campaign or review | SHA-256 |
| --- | --- |
| Packaged v11 synthetic acceptance | `ed97c2b4bca79266e50e4f7e1c91ab6b8b7e435da2afd08b2c56ef9b33c4a0a9` |
| Independent synthetic terminal QA | `81b274b4bac481ff4451e321b23ed5d8c00a7dc50534d19b009a50e48aaacc23` |
| Packaged v11 real acceptance | `c82510185b8a34e9002a061e631c0c9847e6fbe8ce7e8075bc739da6b5041349` |
| Independent real terminal QA | `5b2ba77568e420b7ae3645928504ba601cac4d8a298a293720a3df0764f6af72` |

The retained legacy packaged regression matrix remains a separate gate at this
checkpoint. Neither campaign admits the complete 1,161-table catalog, real
Map/GameObject resources, geometry/world extraction, RustyCore runtime or Windows.
The binary/archive remain frozen at `fdba4b2`; later Markdown does not rebuild them.


#### Retained packaged regression closeout

Both current package roles subsequently close all 130 retained test methods:
260 methods / eighteen stages, zero skips. The measured 892 consumer calls
comprise 814 earlier-profile calls and 78 product-v9 calls. All 892 owned groups
close; 858 traces are empty and 34 are strictly classified nonempty, including
thirty expected kills, with zero consumer network calls. The existing six native
synthetic consumer binaries, original-source snapshots and derived reports are
rechecked and reused without compilation. These native calls belong to the
retained regressions, separately from the v11 campaigns' no-native-rerun scope.
Parent capture is console tool output only; each actual child's raw captures,
generation, result and trace remain retained locally. The parent is untraced.

Thirty-six stage/cross-volume inventories retain 66,546 nodes under the existing
65,536-per-root / 262,144-retained evidence profile, without changing strict
8,192-node source admission or product caps. Observed comparison peak is 85,208
nodes, reserved peak 132,082, beneath the 327,680 comparison ceiling. These are
node-record bounds, not heap/RSS or cumulative visits. Deliberate special nodes
are recorded without opening payloads. The first metadata-only preparation is
failed at its cross-volume path-depth check, before any consumer; V2 adds the
required fresh `/runtime` subdirectory without changing admission policy. Its
partial metadata remains failed and is not an accepted prefix.

Regression acceptance SHA-256:
`528403734db4c4df23acf44f5b08629d1d02e78049a604ef104ae75c56e48e26`.
Independent regression terminal review passes without findings, SHA-256:
`819efeff700a5e03f361e52b75f7ee490b891d6305f9eae2dad1d87f1411280e`.
It checks 36 native synthetic consumer calls within the 892 total, 3,453 guarded
files / 39 declared aliases / five strict source pathsets, and ten FIFOs / six
symlinks recorded as metadata only.

The reproducible package and both v11 campaigns retain their original closed
producer/source bindings. Later documentation does not change any frozen
archive, executable or accepted input. Full catalog, Map/GameObject sources,
real geometry/world resources, RustyCore runtime and publication remain open;
Windows and draft PR #27 stay deferred.


## V23 selected-navigation corresponding-source profile v12 — 2026-10-20

Issue #102 advances the current source profile to
`forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-source-v12`.
The explicit required closure includes the v23 selected-navigation implementation
and synthetic plan/API/CLI tests, its canonical product contract and fixture, and
the product guide plus these three documentation/evidence files:

- `docs/forever-navigation.md`
- `docs/acceptance/forever-v23-selected-navigation-70450-20261020.json`
- `docs/acceptance/forever-v23-selected-navigation-ground-70450-20261020.json`

This is an unconditional current-profile source requirement; removing the product
contract does not disable the gate. Shared extractor entry points remain required
by the existing source profile.

Earlier v11 bundles remain historical with their bundled verifier and recorded
bytes. The v12 profile does not revise those artifacts. It also does not turn
the separately recorded v23 real CLI run into packaged-binary evidence. At the initial profile-definition
checkpoint the v23-specific source closure was defined, but no fresh distribution
candidate, two-build reproducibility result, moved-source rebuild, or packaged
v23 CLI acceptance was claimed. The subsequent qualification below records those
executed results. The archive schema, Linux x86_64 scope, Rust
1.98.0 toolchain, two-package registry closure, notices, and offline build recipe
remain unchanged; full extraction, runtime acceptance and publication remain
open.

## V12 local candidate qualification — 2026-10-20

The frozen candidate from commit `719faee8392a1ce2d1c3193a4c6eea025b9dfca3`
passes local v12 distribution acceptance. Before freezing it, 87 packaging tests
and 439 contracts passed with all 1,344 source files unchanged. Two fresh offline
builds, including an output path with spaces, produced identical archives and
binaries. The trusted verifier accepted both archives; a new moved-source offline
rebuild using the bundled helper/vendor reproduced the binary. The three-build
campaign took 171.276 seconds under observed 1,800s / 3-GiB / one-CPU / 64-task
limits. The bundle has 905 selected source files and 962 inventory entries.

| Identity | SHA-256 |
| --- | --- |
| Archive | `a3ff0d9d91bcbe10df88b8193b02c31f96b83f34b33146b810e5551c808455e3` |
| Binary | `afbc1618f3fad9c420d0ee0726005b6246e9b1aa9edac50e37e1f3e42de387aa` |
| Selected source snapshot | `0e71112571234dc244f4dfe4399daf873bf49cd676ce4839416259da93e19578` |
| Distribution receipt | `feddacb4e6affcaeb8eb6d9a3b8732af2403833b8d3b6073dc6272ab292687a1` |

The exact archive-extracted executable then passed 37 synthetic product/key cases
and ten v23 CLI/fixture cases, including the connected terrain/geometry/navigation
workflow. Syscall denial remained enabled. Its parent trace contains only 27
`SIGCHLD` exit notifications and zero network syscall lines. The original wrapper
incorrectly expected an empty parent trace; the failed wrapper is retained beside
a separate root review validating every line, successful case, bundle and source
guard. No successful test was repeated to replace that finding. A preceding
preflight correctly refused verification of a build workspace containing extra
build files; that attempt made no product call.

Finally, the extracted candidate passed plan-only inspect, real v23 extraction,
offline verify and verify-links in 54.066 seconds under observed 900s / 3-GiB /
one-CPU / 64-task limits. It admitted six terrain tiles and produced eight
nonempty MMap tiles, three empty outcomes, a global and coverage: ten artifacts /
3,787,736 bytes. All nine navigation payload paths, lengths and hashes match the
previous accepted ground package byte-for-byte (3,782,524 bytes); metadata has its
new producer identity. Input packages, plans, source, binary, baseline and client
root identity stayed stable. Receipt:
`4e695fcfb45767f42a72a62e39739b83c559aa81f6a777bbf7a4b4b599109c13`.

The immutable archive retains its initial profile-definition checkpoint, where
acceptance was still pending. This later qualification does not rewrite it or
change Rust/dependency bytes. Original notices and pinned Rust notices remain
included. ELF inspection records external system libraries and GLIBC symbols up
to `GLIBC_2.34`; no static or universal Linux ABI claim is made. This is a local
Linux candidate for bounded extraction. Complete geometry/source/world coverage,
production manager/server acceptance, Windows and release publication remain open.
See the [redacted qualification report](acceptance/forever-v23-distribution-v12-20261020.json).
