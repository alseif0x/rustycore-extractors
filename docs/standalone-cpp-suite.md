# Standalone C++ extractor suite

The v0.1 compatibility suite isolates the four fixed TrinityCoreLegacy 3.4.3
extractors from all server targets. It imports only the 355 paths audited in
`licensing/source-files.json` from commit
`92796557f9b0ba3d2d1c7c770f535153154cf83e`. The precise import and nine
isolation edits are recorded in `licensing/import-manifest.json`.

This document describes engineering evidence and distribution mechanics; it is
not legal advice.

## Dependency boundary

| Target | Standalone libraries | External runtime |
|---|---|---|
| `mapextractor` | extractor common, common/DB2/collision, CascLib, G3D, fmt, zlib, Detour | OpenSSL 3 `libcrypto` |
| `vmap4extractor` | extractor common, common/DB2/collision, CascLib, G3D, fmt, zlib, Detour | OpenSSL 3 `libcrypto` |
| `vmap4assembler` | common/collision, CascLib, G3D, fmt, zlib, Detour | none beyond platform C/C++ runtime |
| `mmaps_generator` | extractor common, common/collision, CascLib, G3D, fmt, zlib, Recast, Detour | none beyond platform C/C++ runtime |

The common target contains no world/auth server, database client, Argon2,
jemalloc, SFMT, utf8cpp or Valgrind dependency. Boost.Filesystem is replaced by
a narrow adapter backed by C++20 `std::filesystem`. CascLib MD5/SHA1 calls use
OpenSSL 3 EVP. The blocked bundled hash implementations, G3D Berkeley fnmatch
and CascLib Overwatch handlers are absent.

## Linux build

Install a C++20 compiler, CMake 3.25+, Python 3 and OpenSSL 3 development
headers. Keep the build and installation outside the checkout:

```bash
cmake -S /home/server/rustycore-extractors \
  -B /home/server/rustycore-extractor-work/builds/standalone-linux \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX=/home/server/rustycore-extractor-work/artifacts/standalone-linux
cmake --build /home/server/rustycore-extractor-work/builds/standalone-linux --parallel
ctest --test-dir /home/server/rustycore-extractor-work/builds/standalone-linux --output-on-failure
cmake --install /home/server/rustycore-extractor-work/builds/standalone-linux
```

## Windows x86_64 build

The clean CI job uses Visual Studio 2022 and the GitHub-hosted runner's vcpkg
checkout. `vcpkg.json` pins the registry baseline and installs only OpenSSL.
The equivalent Developer PowerShell commands are:

```powershell
cmake -S . -B build -G "Visual Studio 17 2022" -A x64 `
  -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_INSTALLATION_ROOT/scripts/buildsystems/vcpkg.cmake" `
  -DVCPKG_TARGET_TRIPLET=x64-windows-static-md
cmake --build build --config Release --parallel 2
ctest --test-dir build -C Release --output-on-failure
cmake --install build --config Release --prefix artifacts
```

## Metadata and offline boundary

All four programs accept `--version` and `--version-json` without opening CASC
or creating output. The JSON identifies this project version and commit, the
fixed TrinityCoreLegacy commit, `wow_classic` build 51943, `MAPS` version 10,
`VMAP_4.B`/`VMAP04B`, and `MMAP` version 15 with Detour version 7 and 64-bit
poly references.

`CASC::Storage::Open` passes only the local install path to CascLib.
`CASC::Storage::OpenRemote` always returns `ERROR_NOT_SUPPORTED` while
`contracts/network-lock-v1.json` contains no pinned remote profile. There is no
HTTP, TLS, resolver or key-download implementation in the executable closure.

Never run extraction in the client directory. Use explicit input/output paths
and an isolated directory beneath `/home/server/rustycore-extractor-work`.
`mmaps_generator` consumes and writes relative to its working directory, so its
working directory must be its dedicated run directory.

## Validation evidence

CTest verifies the exact imported path set, modification notices, excluded
sources, offline lock, replacement hash/fnmatch behavior and metadata from all
four binaries. Real-client outputs are intentionally excluded from Git and CI.
Local closeout evidence belongs under `/home/server/rustycore-extractor-work`.
