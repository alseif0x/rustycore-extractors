# rustycore-extractors

Standalone, versioned client-data extraction tools for RustyCore 3.4.3

The project is currently in private planning/bootstrap. Follow the pinned
[delivery plan](https://github.com/alseif0x/rustycore-extractors/issues/13) and the
[development environment](docs/development-environment.md). No Blizzard client-derived data is
stored or distributed by this repository.

The project is licensed under GPL-3.0-or-later. See [LICENSE](LICENSE),
[NOTICE.md](NOTICE.md), [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), the
[source-closure audit](docs/licensing/source-closure.md), and the
[machine-readable inventory](licensing/inventory.json).

The versioned [compatibility, manifest and network contract](docs/compatibility-contract.md)
defines the fixed build-51943 formats, stable CLI/JSON behavior, offline boundary, atomic output
rules and `data-manifest.json` schema. Contract examples under `tests/contracts/fixtures/` are
synthetic and contain no client data.
