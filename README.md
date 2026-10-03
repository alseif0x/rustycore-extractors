<h1 align="center">RustyCore Extractors · Forever</h1>

<p align="center">
  Local client data. Versioned tools. Reproducible evidence.<br>
  Preparing the extraction tooling for <strong>WoW Forever 1.60.1 · build 70170 · Beta x64</strong>.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="GPL-3.0-or-later"></a>
  <img src="https://img.shields.io/badge/default%20branch-forever-6f42c1" alt="Default branch: forever">
  <img src="https://img.shields.io/badge/current%20tools-C%2B%2B20-orange" alt="Current tools: C++20">
  <img src="https://img.shields.io/badge/70170-support%20pending-e09f3e" alt="Build 70170 support pending">
</p>

<p align="center">
  <a href="#current-status">Status</a> ·
  <a href="#build-the-existing-suite">Build</a> ·
  <a href="#forever-development-direction">Forever</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#data-stays-local">Data policy</a>
</p>

## What this repository is

Standalone, versioned client-data extraction tools for
[RustyCore](https://github.com/alseif0x/rustycore). The tools read a locally
installed WoW client and produce data for server readers without requiring a
TrinityCore checkout to build the standalone suite.

**`forever` is the default development branch.** The previous `main` branch
was renamed to [`3.4.3`](https://github.com/alseif0x/rustycore-extractors/tree/3.4.3),
preserving the WotLK Classic baseline as a separate version line.

> **Development preview, not a 70170-ready extractor.** Changing the branch
> does not port the implementation. The code currently builds the existing
> **C++ suite for WoW Classic 3.4.3, build 51943**. Its version metadata,
> formats and compatibility contract still describe that baseline.
> **Neither a Rust-native replacement nor Forever/70170 compatibility is
> implemented or validated in this repository yet.**

## Current status

| Area | What exists today |
| --- | --- |
| Version lines | `forever` for 1.60.1.70170 development; `3.4.3` for the legacy baseline |
| Implementation | Standalone C++20 suite; Rust-native replacement remains planned |
| Existing client contract | `wow_classic`, 3.4.3.51943 — not a Forever product/build claim |
| Existing tools | `mapextractor`, `vmap4extractor`, `vmap4assembler`, `mmaps_generator` |
| Conformance | Synthetic fixtures and recorded build-51943 reference comparisons; no 70170 acceptance |
| Client assets | Never distributed through Git, CI artifacts or releases |

The current tools cover map/client-data extraction, collision geometry and
navigation generation. They are distinct from the target-specific Rust DB2
readers and diagnostic probes currently developed in RustyCore itself; those
components have **not** been migrated into this repository as a native
extractor suite.

The historical conformance evidence is scoped: the recorded build-51943 run
compares 17,391 files, but MMap generation covers one deterministic map-0 tile.
The recorded RustyCore VMap-reader boundary remains unsupported at that audit.
See [the conformance guide](docs/conformance-harness.md) for the exact coverage
and limitations, rather than treating these results as full runtime support.

## Build the existing suite

Requirements: **CMake 3.25+**, a **C++20 compiler**, **Python 3** and
**OpenSSL 3 development files**. Other compiled dependencies are pinned in
the audited source closure.

Keep builds, installations and extraction outputs outside this checkout and
outside the client directory:

```bash
git clone --branch forever https://github.com/alseif0x/rustycore-extractors.git
cd rustycore-extractors

cmake -S . -B ../rustycore-extractor-work/builds/standalone \
  -DCMAKE_BUILD_TYPE=Release
cmake --build ../rustycore-extractor-work/builds/standalone --parallel 1
ctest --test-dir ../rustycore-extractor-work/builds/standalone --output-on-failure
cmake --install ../rustycore-extractor-work/builds/standalone \
  --prefix ../rustycore-extractor-work/artifacts/standalone
```

All four binaries provide `--version` and `--version-json`. The latter reports
the existing product/build, upstream commit and output-format versions;
**expect the 3.4.3/51943 baseline even when building this branch**.

For Windows build instructions and safe real-client invocation, use the
[standalone suite guide](docs/standalone-cpp-suite.md). This README does not
recommend running the existing suite against a 70170 client or changing a
build-number constant to bypass compatibility checks.

## Forever development direction

The next work must establish the actual Forever contract before declaring
support:

1. Identify the target client product, CASC inputs, DB2 layouts and required
   outputs from build-70170 evidence.
2. Reuse existing C++ components only where target evidence supports them.
3. Introduce Rust-native extraction stages with explicit inputs, diagnostics
   and synthetic positive/negative coverage.
4. Compare outputs with an appropriate pinned reference and exercise the
   corresponding Forever readers in RustyCore.

These are development goals, **not completed features**. Preserve the existing
3.4.3 contracts as versioned evidence; do not silently relabel MAPS/10,
VMAP_4.B or MMAP/15 as validated Forever formats.

The [original delivery plan](https://github.com/alseif0x/rustycore-extractors/issues/13)
describes the **3.4.3 programme**. Its old `main` references and build-51943
milestones are historical context, not proof of Forever readiness.

## Data stays local

- Current extraction is **local-CASC-only**. Remote CASC and implicit key
  downloads are disabled; changing branches does not enable network access.
- Treat the client installation as read-only. Use explicit paths and isolated
  run directories; the MMap generator works relative to its working directory.
- Never commit or upload client-derived DB2, maps, VMaps, MMaps, models,
  cameras, game tables, CASC caches, keys or intermediate files.
- Use synthetic fixtures and reviewed hash/structured reports for evidence.
  A successful process exit alone does not establish compatibility.

## Documentation

The following guides describe the **existing 3.4.3 suite**, unless explicitly
marked otherwise:

- [Development environment](docs/development-environment.md)
- [Standalone C++ suite](docs/standalone-cpp-suite.md)
- [Output conformance harness](docs/conformance-harness.md)
- [Versioned compatibility and network contract](docs/compatibility-contract.md)
- [Source-closure audit](docs/licensing/source-closure.md)
- [Machine-readable dependency inventory](licensing/inventory.json)

## License and attribution

Project code is licensed under **GPL-3.0-or-later**. Preserve the original
TrinityCore and third-party notices. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and
the [license directory](LICENSES).

World of Warcraft and its related trademarks belong to Blizzard
Entertainment. This project is not affiliated with or endorsed by Blizzard.
No Blizzard client data is stored or distributed by this repository.
