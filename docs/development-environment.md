# Development environment

## Current server layout

The source checkout is deliberately separated from large or copyrighted inputs and outputs:

| Role | Path | Policy |
|---|---|---|
| Extractor source | `/home/server/rustycore-extractors` | Git-tracked source only |
| RustyCore integration | `/home/server/rustycore` | Separate repository; no extractor build output |
| WoW 3.4.3.51943 client | `/home/server/wow-client/World of Warcraft` | Read-only input |
| TrinityCoreLegacy reference | `/home/server/woltk-trinity-legacy` | Read-only reference |
| Existing extracted data | `/home/server/woltk-server-core/Data` | Read-only preliminary reference |
| Extractor laboratory | `/home/server/rustycore-extractor-work` | Untracked builds, runs, caches and reports |

The installed client has been inspected as `wow_classic` version `3.4.3.51943`. The existing
reference data is about 3.1 GiB, including roughly 197 MiB DBC/DB2, 332 MiB maps, 608 MiB
VMaps and 2.0 GiB MMaps. Keep ample additional space for reference and candidate runs.

## Laboratory layout

```text
/home/server/rustycore-extractor-work/
  artifacts/             release candidates, never Git input
  builds/                out-of-tree CMake/Cargo builds
  cache/tact/             explicitly pinned local metadata cache
  inputs/                 convenience read-only symlinks
  logs/                   persistent-process logs
  reports/                structured compatibility reports
  runs/reference/         isolated C++ oracle runs
  runs/candidate/         isolated candidate runs
  worktrees/              per-issue Git worktrees
```

Every real extraction receives a unique run ID. Never point an output at the client, reference
checkout, existing Data directory or Git checkout.

Example candidate path:

```text
/home/server/rustycore-extractor-work/runs/candidate/2026-08-22-map-smoke
```

## Local machine capabilities

The current server has CMake 3.28, GCC 13, Rust/Cargo 1.88, `tmux` and a running systemd user
manager. There is no native Linux extractor build or Wine installation yet. Windows reference
binaries exist beside the local client, but they are not the Linux build/test path.

Full MMap generation is a persistent background/release validation task. Normal development
uses one representative map or tile and stores commands/results in `reports/`.
