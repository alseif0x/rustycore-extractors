# Selected navigation package consumer

## Implementation scope — 2026-10-20

Issue #104 carries the Linux build-70450 selected-navigation consumer. Its
shared package API, checked native ingress and optional RustyCore manager/worker
have passed bounded local synthetic and real-package acceptance. The RustyCore
implementation is published in [RustyCore PR #1454](https://github.com/alseif0x/rustycore/pull/1454).
Its merge remains pending. The general final still rejects inherited
architecture/session baseline drift; publication uses only the existing
experimental exception for that unchanged debt. Server activation and gameplay
acceptance remain open.

`forever_navigation::package_v23` owns the canonical V23 coverage and source
identity codec. The extractor CLI delegates output-package verification to
that API. `forever_casc::package` retains common manifest, receipt, seal,
inventory, rooted-file and shared-lock verification. There is one codec for
the component's stored schema.

The API admits the existing `linux70450-selected-navigation-v1` profile:
one selected map, MMap16 containers, Detour7 payload framing and zero global
off-mesh connections. It preserves the existing source coverage, ancestry,
terrain/VMap projection counts and unknown-source boundaries. It does not
authenticate the producer or prove native payload topology.

## Verification and owned reads

`verify(root, expected)` compares the selected map, locale and canonical
producer-plan digest, plus an optional exact common package context. The
RustyCore consumer must supply that context and retain the returned complete
summary/inventory for later comparisons. Public metadata structs are not
authority tokens; their codec validates canonical encoding and consistency.

`read_batch(root, expected, requested_tiles, limits)` verifies the complete
package and retains its global container and only the requested tile
containers. The package engine hashes and inspects the same held descriptors.
Nothing is returned until complete verification succeeds. There is no caller
callback that could publish native state before that success.

The default request ceiling is 64 tiles and the retained payload-buffer ceiling
is 128 MiB. Per-file reads retain the existing MMap limit. Overrides can reduce
these ceilings. Duplicate coordinates, coordinates outside `0..63` and excessive
request counts reject before payload work. Requested declared sizes are admitted
in inventory preflight before payload hashing; actual retained capacities are
also checked. Metadata and transient file-read allocations remain distinct
bounded costs. These are logical limits, not RSS measurements.

Each requested result is either a verified written container or an explicitly
recorded empty candidate. A coordinate absent from coverage is unavailable;
it is never manufactured as empty geometry. Request ordering is preserved.

Verification describes the package during that call. It does not hold a lock
after return or certify future pathname reads. A consumer must reverify on a
later batch and compare the original context and summary/inventory. Returned
owned bytes can then be checked and copied into native ownership.

## Native and RustyCore boundary

Container framing and hashes do not validate every index consumed by native
Detour. The checked native ingress validates its exact ABI, layout,
polygon/detail/BV/portal references, numerical domain and resource limits
before adding a tile; the bounded acceptance below qualifies this selected profile. Its borrowed
checked token is constructed only after parsing, and the copy uses the existing
native add path. C++ static assertions bind the layout assumptions to the vendored
64-bit ABI. Default ceilings include 64 MiB per payload, 100,000 polygons,
250,000 vertices, one million links/detail triangles/BV nodes, and 4,096 native
tile slots. Caller overrides may only reduce those limits. An instance query
retains its node budget in `4..=65535`; a different budget requires unloading
that query before reloading it.

Original MMap16 headers and underscore-separated names
remain intact; the consumer must not fabricate a legacy v15 container.

RustyCore's existing MMap manager remains the native mesh/query owner. Queries
must stay on its dedicated worker thread and be destroyed before their mesh.
Configured source/scope/integrity errors must remain failures; legacy missing
mesh fallback is not acceptance of the selected modern profile. Partial native
status remains visible. Older terrain-library acceptance does not admit
build70450, and this operation does not activate the server or world entry.

The implemented consumer exposes explicit package construction, tile
preparation/unload, instance-query preparation/unload and raw corridor requests
through `WorldMMapPathfinderWorkerLikeCpp`. Its selected constructor reports
initialization success or failure before returning a worker. The owner retains
the original package context and summary; subsequent tile reads compare both.
Loaded tiles and queries remain in the existing manager, with 128 MiB of loaded
artifact bytes, 64 instance queries and 4,096 nodes per query as default ceilings.
The node budget matches the previous bounded SDK probe. The consumer permits at most
256 corridor references per request. The lower-level SDK admits up to 4,096;
that SDK ceiling does not increase the consumer limit. Caller-reduced requests
remain supported.

The source profile
`forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-consumer-source-v13`
adds the shared schema, public consumer tests and contract to mandatory
corresponding source. Its Linux navigation-to-CASC path edge is explicit.
The seven workspace crates and the two pinned registry packages are unchanged.
The v13 frozen source produced two identical fresh archives and binaries and
a matching moved-source offline rebuild. The packaged executable passes 47 synthetic CLI/fixture cases and a fresh
real four-command run with nine exact baseline payloads. The accepted v12 archives retain their original source and evidence.

See the [consumer contract](../contracts/forever-rustycore-navigation-consumer-contract-v1.json),
[V23 product](forever-product-v23.md) and
[distribution evidence](forever-distribution.md).

## Bounded acceptance — 2026-10-20

The source qualification combines 77 SDK tests, ten public worker tests and one
feature-disabled regression with 17 shared package tests. The extractor sweep
completes 3,941 tests with 26 explicit ignores through a retained full attempt,
one corrected diagnostic-assertion rerun and continuation of the remaining
crates. It is not a single green workspace invocation. Clippy and formatting
pass; 436 Forever contract tests and 89 packaging tests pass. Ordinary component
receipt: `573971d643fb25428760380a71329c335ec37f1aea69253687f332cc8e88b2e6`.

The actual public worker admitted the eight unchanged real ground containers,
prepared an instance, resolved 98 proximity queries, ran 256 cross-tile queries,
then unloaded the instance and every tile. The final loaded-tile and query
counts are zero. Six corridors reach their target and 250 retain Detour's
partial-result bit; none report out-of-nodes or buffer-too-small. All 256
ordinal/status/length/target facts match the earlier raw SDK replay exactly.
Receipt: `8551686bc18b2c411ca6ed4e6504ee686f8a3c3a3a6abf643f3e63340b7c4901`.

The standalone harness build took 201.550s and its actual worker operation
3.321s; the complete guarded run took 205.691s. This establishes the selected
library/worker operation. The cause of the partial routes remains unestablished.
The ordinary acceptance campaign exceeded its 600-second target, including
retained failed attempts; that performance target is not met.

The qualification freezes implementation sources before later status prose.
The consumer request limit is 256; earlier draft prose listed the SDK's 4,096
ceiling as the consumer limit. This correction changes documentation and its
contract assertion, with no Rust behavior change. No whole-world, gameplay,
Windows or release acceptance is implied.

## Frozen v13 package qualification

The later core publication changes no shipped extractor Rust, dependency,
manifest, lock or packaging recipe. Its status update does not requalify a new
archive or transfer the frozen v13 qualification to a later Git SHA.

The frozen corresponding-source digest is
`577d46cd2952cf306c5fa404ba87bd5881e1e6701e14b4f6f56f3ea2cabbf85b`.
Two fresh archives and binaries match; a moved-source offline rebuild matches
the same binary. Archive: `14e7d7d95f5bc4390837453eb26ef0bac9ae349ba4a2a3f20dc63db7f45adfbe`.
Binary: `1c391fe7dea024c16c1080244b497bc5c97d0067c095f3f577ff1df6590bcdb0`.
The three-build campaign took 174.512s. The archive retains the earlier
definition-stage status prose and the draft SDK/consumer-limit wording; the
subsequent documentation correction does not change product Rust or its build
graph. Qualification belongs to this exact snapshot, not a later Git SHA.

The extracted executable passes 37 synthetic product cases and ten v23
CLI/fixture cases in 45.277s. The ten v23 cases were traced with network syscalls
forced to fail: zero network calls were attempted. The 37 other cases were not
traced, so this is not a network observation over all 47. A fresh real run took
54.740s and passed inspect, extraction, verification and linked verification.
Its eight tile payloads and global are byte-identical to the retained baseline:
nine payloads / 3,782,524 bytes. Input trees and the package remained unchanged.
See the exact receipt hashes in the [contract](../contracts/forever-rustycore-navigation-consumer-contract-v1.json).

## Isolated core qualification and publication — 2026-10-20

Core code `1bf5ee4b48d8f729e806f7acddfdebad01025b58` passes workspace
all-target compilation, 5,933 tests across four affected libraries (three
ignored), ten public consumer cases, one feature-disabled regression, focused
dependency checks, formatting and SDK Clippy. The 5,944 passes compose retained
unchanged-candidate evidence and a resumed invocation with the already pinned
SQL reference. A missing-reference failure remains retained; no source was
changed to bypass its comparisons.

The initial isolated candidate also exposed a new SDK physical-size failure
(220 versus 218). Moving the three synthetic-fixture FFI declarations into a
private module preserves cfgs, symbols and callers and repairs that failure
without changing a ceiling. Seven reviewed preexisting test-only repairs are
included; original pending terrain, configuration and source files are preserved.

Publication commit `803449bd030ca68d48b906880b78ca8a31810751` adds only the two
owning core checkpoint documents after code qualification. Its final reproduces
exactly the same inherited logical/Session rejection and no new failure. The
existing exception authorizes experimental branch publication; it does not
authorize merging or activating the server. The core PR and issue #104 remain
open. These isolated attempts and closing validation total 1,397.524s; they
do not reset the earlier failed ordinary 600-second target.

Code qualification receipt:
`fe9f725ff4de50441ace06613be4b028bb723cbab4a1cbde9cdbc75107ea8e41`.
Closing publication review:
`c22c427eb9bb2ccb5b14863c819be8f03fd70db762dd5d6d262f00546b2834f1`.
These receipts qualify the stated component operation; complete real geometry,
full-world resources and server/gameplay acceptance remain open.

## Local core bootstrap amendment — 2026-10-20

Core candidate `c89279417bc39a97c703c73cc485099a852640af` records the pinned
source bootstrap for the adjacent extractor checkout at commit
`3baac307329bd0391cd7f083a55e03b982081b27` and lock SHA-256
`8afafe1f5f8b9c98f53ed55155160b2a6d21f6efb5edf1d1c7f0f6e60819af27`. It is
unpublished: GitHub rejected the push for missing OAuth workflow scope, so PR
#1454 remains at `803449bd`; no retry or bypass was attempted.

Bootstrap evidence includes actionlint 1.7.12, Bash syntax, 16 synthetic cases,
the exact public fetch/checksum and offline locked metadata for 38 packages with
the feature disabled. ShellCheck was unavailable. The initial PATH diagnostic
is retained; resuming with pinned Rust 1.98.0 did not change sources or download
again, so the steps are not one green campaign. Focused and live receipt hashes,
and the 52.636-second core final receipt, are in the contract. That final has
the same inherited rejection and no new failure waived. The historical 1bf5
code qualification and 803449 publication remain unchanged; no new general final
or server/runtime acceptance is claimed. Rust, manifests, lock and dependency
policy are unchanged by this amendment.
