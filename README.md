<h1 align="center">RustyCore Extractors</h1>

<p align="center">
  <strong>Local client data → versioned server resources</strong><br>
  Standalone C++ tools and a growing Rust extraction pipeline for RustyCore.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="GPL-3.0-or-later"></a>
  <a href="https://github.com/alseif0x/rustycore-extractors/tree/forever"><img src="https://img.shields.io/badge/development-forever-6f42c1" alt="Development branch: forever"></a>
  <img src="https://img.shields.io/badge/priority-Linux-1793d1" alt="Delivery priority: Linux">
  <img src="https://img.shields.io/badge/tools-Rust%20%2B%20C%2B%2B20-orange" alt="Tools: Rust and C++20">
</p>

<p align="center">
  <a href="#current-status">Status</a> ·
  <a href="#build-the-rust-product">Rust quick start</a> ·
  <a href="#build-the-existing-suite">C++ suite</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#contributing">Contributing</a>
</p>

> [!IMPORTANT]
> **Forever is in development.** Bounded Linux extraction and reader checks
> exist, but complete client extraction, full world resources and RustyCore
> runtime acceptance remain open. Windows delivery is deferred.

## What this repository is

Standalone, versioned extraction tools for
[RustyCore](https://github.com/alseif0x/rustycore). They turn locally installed
client data into tables, cameras, terrain, collision and navigation resources,
with explicit input contracts and separate output verification.

There are two distinct version lines:

| Version line | Purpose | Compatibility boundary |
| --- | --- | --- |
| [`3.4.3`](https://github.com/alseif0x/rustycore-extractors/tree/3.4.3) | Preserved WotLK Classic C++ suite | `wow_classic` 3.4.3, build 51943 |
| [`forever`](https://github.com/alseif0x/rustycore-extractors/tree/forever) | Default branch; Rust development for WoW Forever 1.60.1 Beta x64 | Explicit build and component profiles; full extraction remains unfinished |

The standalone C++ targets retain their **3.4.3/51943** contract when built
from `forever`. Selecting a branch does not change their supported client or
output formats. The Rust product is the separate `forever-extractor` binary.

## Current status

**Linux comes first:** complete acquisition, component output, full world
operation, runtime integration and reproducible distribution are the delivery
goals. The [completion checklist](docs/forever-completion.md) is the detailed
source of truth; the table below summarizes the boundaries.

| Component | Implemented and evidenced scope | Still required for complete Forever delivery |
| --- | --- | --- |
| Local acquisition | Bounded Rust CASC reads, metadata authentication and explicit available-section APIs | Complete required client resources for each admitted collection |
| DB2 | Build-specific catalogs, decoding, batch extraction and verification; selected real data comparisons | Complete catalog acquisition and the required server consumers |
| Game tables & cameras | Linux product integration and bounded real extraction; cameras are preserved as raw files | Full target coverage; camera playback is outside raw extraction |
| Terrain / MAPS | ADT conversion, assembly, readers and bounded local execution; separate available-Map terrain profiles | Complete real terrain selections and world/runtime acceptance |
| Collision / VMaps | M2/WMO conversion, placements, assembly, queries and resident/leased-disk library pipelines | Complete current-client acquisition, final product integration and real runtime acceptance |
| Navigation / MMaps | Rust Recast/Detour stages, tile generation and supplied-input reader/query comparisons | Complete real world generation, product integration and runtime acceptance |
| Packaging | Versioned manifests, component seals, resume and offline verification; reproducible bounded Linux bundle | Complete extractor release and distribution |
| Windows | Platform work and historical scoped evidence are retained | Development and delivery resume after the Linux milestone |

### Current build-specific scope

| Build | What the evidence establishes | Boundary |
| --- | --- | --- |
| **70374** | Linux product v10 selects DB2; v11 selects DB2, game tables and raw cameras. Bounded real lifecycle and local packaging evidence exist. Separate available-source collision and terrain work is documented. | Full catalog, complete world resources and runtime support remain open. |
| **70425** | Explicit disabled-key bootstrap, a separate catalog and available-section Map/GameObjectDisplayInfo readers; a bounded real source-reader probe is recorded. | No 70425 product profile, full component extraction or runtime acceptance follows. |

Older builds retain their own recorded results. Evidence is tied to the
specific source collection, tool and profile: a matching build number alone
does not transfer acceptance to a changed installation.

See [product scope](docs/forever-product.md),
[70425 source-reader scope](docs/forever-70425.md) and
[Linux compatibility](docs/forever-linux-compatibility.md).

### Active milestone: collision placement compatibility

The current implementation is tracked in
[issue #92](https://github.com/alseif0x/rustycore-extractors/issues/92).

- [x] Implement serial placements over the compatible model collector.
- [x] Pass the eight software qualification commands, including 3,388 tests
  in each debug/release profile and 31 new placement tests per profile.
- [ ] Complete comparison with the original C++ extractor.
- [ ] Integrate world traversal, parent replay and final VMap output.
- [ ] Validate real resources, product integration and RustyCore runtime use.

This checkpoint covers library software. The
[collision guide](docs/forever-collision.md#compatible-serial-placement-software-checkpoint--2026-10-17)
records the tested source and remaining acceptance gates.

## Build the Rust product

The workspace pins **Rust 1.98.0** in [rust-toolchain.toml](rust-toolchain.toml).
Keep build files and extraction output outside the checkout and the client.

```bash
git clone --branch forever https://github.com/alseif0x/rustycore-extractors.git
cd rustycore-extractors

WORK_DIR="$PWD/../rustycore-extractor-work"
export CARGO_TARGET_DIR="$WORK_DIR/builds/rust"

# Build preparation: fetch only the locked Rust dependency closure.
cargo fetch --locked

# Compile the Rust product with offline dependency resolution.
cargo build --offline --locked --release \
  -p forever-extractor --no-default-features

BIN="$CARGO_TARGET_DIR/release/forever-extractor"
"$BIN" --help
```

Provision the pinned toolchain before an offline build. The fetch step requires
network access unless the locked dependencies are already cached; client
acquisition remains local. This build selects the Rust product, without the
optional native acquisition prototype.

### Inspect a plan and verify output

The product uses an **explicit canonical TSV plan**. Choose the documented
version for the intended build and components; see the
[plan format and commands](docs/forever-product.md#build-and-inspect).

```bash
# Supply your own plan and a previously produced package outside the checkout.
PLAN=/absolute/path/to/plan.tsv
OUTPUT=/absolute/path/to/package

# Parse the plan without opening a client or reading selected assets.
"$BIN" inspect --plan "$PLAN"

# Verify a package against that plan; these commands need no client or keys.
"$BIN" verify --plan "$PLAN" --output "$OUTPUT"
"$BIN" verify-links --plan "$PLAN" --output "$OUTPUT"
```

On Linux, `verify-links` additionally checks the published terrain tile sets
and navigation tile/global presence. Verification covers artifact integrity and
the admitted intrinsic formats; it does not establish complete world geometry
or server runtime compatibility.

`--help` also describes `extract` and `resume`, their explicit client/output
arguments, and the scratch requirements for leased-disk plans. Use the
[product guide](docs/forever-product.md) before a real run. Missing required
inputs reject the selection; an incomplete output is not a usable full package.

## Build the existing suite

The preserved standalone suite provides:

| Tool | Role |
| --- | --- |
| `mapextractor` | Client tables and map extraction |
| `vmap4extractor` | Raw collision extraction |
| `vmap4assembler` | Collision assembly |
| `mmaps_generator` | Navigation generation |

Requirements: **CMake 3.25+**, a **C++20 compiler**, **Python 3** and
**OpenSSL 3 development files**. From the repository root:

```bash
cmake -S . -B ../rustycore-extractor-work/builds/standalone \
  -DCMAKE_BUILD_TYPE=Release
cmake --build ../rustycore-extractor-work/builds/standalone --parallel 1
cmake --install ../rustycore-extractor-work/builds/standalone \
  --prefix ../rustycore-extractor-work/artifacts/standalone
```

All four binaries provide `--version` and `--version-json`. Their metadata
continues to describe **3.4.3/51943**, including when built on `forever`.
Consult the [standalone guide](docs/standalone-cpp-suite.md) for invocation,
dependencies and legacy platform instructions.

## Forever development direction

The delivery path is to complete the Linux pipeline while keeping each
versioned compatibility boundary explicit:

1. Acquire every required input for the admitted client collection.
2. Produce and verify complete selected component outputs.
3. Compare with the pinned reference and exercise the corresponding server readers.
4. Validate complete world/runtime operation and publish a reproducible package.

Library checks, synthetic comparisons, real component samples and complete
runtime acceptance are separate milestones. Follow the
[completion gates](docs/forever-completion.md) for progress and blockers.
The [original delivery plan](https://github.com/alseif0x/rustycore-extractors/issues/13)
describes the legacy 3.4.3 programme; its historical milestones retain that scope.

## Documentation

### Start here

- [Completion checklist](docs/forever-completion.md) — Linux delivery gates and remaining work
- [Product interface](docs/forever-product.md) — plans, extraction, resume and verification
- [Linux distribution](docs/forever-distribution.md) — local packaging and reproducibility scope
- [Standalone C++ suite](docs/standalone-cpp-suite.md) — the preserved legacy tools

### Components and formats

- [Rust DB2 foundation](docs/forever-rust-foundation.md) · [Metadata catalogs](docs/forever-metadata-catalog.md)
- [Local CASC](docs/forever-local-casc.md) · [70425 foundation and readers](docs/forever-70425.md)
- [Game tables](docs/forever-game-tables.md) · [Raw cameras](docs/forever-cameras.md)
- [Terrain](docs/forever-terrain.md) · [Available-Map terrain](docs/forever-terrain-v12.md) · [Explicit omissions](docs/forever-terrain-v13.md)
- [Collision](docs/forever-collision.md) · [Leased-disk world pipeline](docs/forever-collision-spooled-world.md)
- [Navigation](docs/forever-navigation.md) · [Package integrity](docs/forever-package.md)
- [Disk product contract](docs/forever-product-disk.md) · [Deferred Windows scope](docs/forever-windows-product.md)

### Engineering and provenance

- [Contributor operating guide](AGENTS.md) · [Development environment](docs/development-environment.md)
- [Compatibility contract](docs/compatibility-contract.md) · [Linux compatibility](docs/forever-linux-compatibility.md)
- [Conformance harness](docs/conformance-harness.md) · [CI scope](docs/forever-ci.md)
- [Source-closure audit](docs/licensing/source-closure.md) · [Dependency inventory](licensing/inventory.json)

Detailed guides retain the historical measurements, exact contracts and
limitations behind the summaries above.

## Contributing

Read [AGENTS.md](AGENTS.md) before changing implementation or acceptance claims.
Target `forever` for modern development and keep the `3.4.3` version line intact.
Use focused changes and synthetic fixtures; document the exact build, profile,
inputs and reader scope behind a compatibility claim.

For bug reports, share tool versions, sanitized diagnostics and reproduction
steps. Keep client-derived files, identifiers, keys and private local paths out
of issues and pull requests. The [issue tracker](https://github.com/alseif0x/rustycore-extractors/issues)
is the place to discuss bugs and missing capabilities.

## Data stays local

- Client acquisition is local CASC only. Remote CASC and implicit key downloads are disabled.
- Treat client installations as read-only; use isolated build, scratch and output directories.
- Never commit or upload client-derived DB2, maps, VMaps, MMaps, models, cameras, game tables, CASC caches, keys or intermediate files.
- Supplied-key profiles are explicit; keys are never discovered, downloaded or included in output metadata.
- Use synthetic fixtures and sanitized reports for shared evidence. A successful process exit alone does not establish compatibility.

## License and attribution

Project code is licensed under **GPL-3.0-or-later**. Preserve the original
TrinityCore and third-party notices. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
the [license directory](LICENSES).

World of Warcraft and its related trademarks belong to Blizzard
Entertainment. This project is not affiliated with or endorsed by Blizzard.
No Blizzard client data is stored or distributed by this repository.
