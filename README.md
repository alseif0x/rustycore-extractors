# rustycore-extractors

Standalone, versioned client-data extraction tools for RustyCore 3.4.3.

The standalone C++ compatibility suite builds `mapextractor`, `vmap4extractor`,
`vmap4assembler` and `mmaps_generator` without a TrinityCore checkout. Follow the pinned
[delivery plan](https://github.com/alseif0x/rustycore-extractors/issues/13) and the
[development environment](docs/development-environment.md). No Blizzard client-derived data is
stored or distributed by this repository.

## Build

Requirements are CMake 3.25 or newer, a C++20 compiler, Python 3 and OpenSSL 3
development files. All other compiled dependencies are pinned in the audited
source closure.

```bash
cmake -S . -B /path/outside/repository/build -DCMAKE_BUILD_TYPE=Release
cmake --build /path/outside/repository/build --parallel
ctest --test-dir /path/outside/repository/build --output-on-failure
cmake --install /path/outside/repository/build --prefix /path/to/install
```

Use `--version` for human-readable metadata or `--version-json` for the fixed
product, client build, upstream commit and output-format versions. Extraction
is local-CASC-only: remote CASC and implicit key downloads are disabled.

See [the standalone suite guide](docs/standalone-cpp-suite.md) for Windows,
dependency boundaries and safe real-client execution.

Use the [output conformance harness](docs/conformance-harness.md) to compare a
pinned reference run with a candidate by file set, parsed format semantics and
deterministic bytes. Its synthetic CI fixtures contain no Blizzard data.

The project is licensed under GPL-3.0-or-later. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), the
[source-closure audit](docs/licensing/source-closure.md), and the
[machine-readable inventory](licensing/inventory.json).

The versioned [compatibility, manifest and network contract](docs/compatibility-contract.md)
defines the fixed build-51943 formats, stable CLI/JSON behavior, offline boundary, atomic output
rules and `data-manifest.json` schema. Contract examples under `tests/contracts/fixtures/` are
synthetic and contain no client data.
