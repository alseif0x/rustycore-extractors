# Forever CASC prototype dependency closure — 2026-10-03

Recorded in [licensing issue #1](https://github.com/alseif0x/rustycore-extractors/issues/1#issuecomment-5971981717)
before compilation. Separate from the legacy 355-file source import. No legacy
vendor source is replaced, and no new upstream source is stored in Git.

## Exact external compiler inputs

TrinityCore Git commit `02245dcd245e7433e524577656177723d3e4992e`, selectors
relative to that immutable tree:

- `dep/CascLib/LICENSE` (MIT, Ladislav Zezula copyright notice);
- `dep/CascLib/src/Casc*.cpp` and `Casc*.h`;
- `dep/CascLib/src/common/*.cpp` and `*.h`, excluding `Sockets.cpp`;
- `dep/CascLib/src/jenkins/lookup3.c` and `lookup.h` (Bob Jenkins public-domain
  dedication retained).

The pin, selectors and per-file Git-blob verification are enforced by
`native/forever-casc/CMakeLists.txt`. Modified/missing compiler inputs and source
symlinks reject; the build does not execute upstream CMake. Only verified files
are copied, unchanged with notices, into the external Cargo OUT_DIR. The copied
MIT license remains beside them. No client data enters that source/build tree.

The whole upstream `hashes` directory is excluded (including its potentially
incompatible SHA1 header). New project GPL-3.0-or-later adapters implement the
required hash ABI with OpenSSL 3 EVP, preserving the modern SHA1 Final argument
order. OpenSSL failures are errors, not synthetic zero hashes. The complete
Overwatch implementation and socket transport are also excluded. Existing
GPL-3.0-or-later deny-all Overwatch/socket adapters are compiled against the
modern headers instead. No TACTLib/AES/APM/CMF code or remote downloader is added.

Decompression uses the already audited, unchanged zlib 1.3 vendor sources
(Zlib license). Dynamic OpenSSL 3 libcrypto uses its Apache-2.0 terms. The native
ABI links the host C++ runtime and pthread; this is a Linux development prototype,
not a binary release or Windows/linker-closure acceptance. The existing MIT,
Apache, Zlib and project license notices still apply. Any binary distribution
must supply corresponding source including this exact external CascLib closure,
adapters, build scripts and retained notices; an environment path alone is not
corresponding source. No release is authorized or claimed by these checks.

## Why this candidate

The legacy audited CascLib rejects ROOT version 2 in
`CascRootFile_WoW.cpp::CaptureRootHeader_50893`; the modern pin admits versions
1/2 and has the v2 group parser. Its progress callback ABI also differs.
Substituting the legacy library under the modern ABI is not safe.

The architecture skill guided a separate optional native build boundary:
`forever-db2` remains safe, dependency-free Rust; `forever-casc` owns admission,
budgets and its single-threaded RAII handle. Its private FFI is the only Rust
unsafe module. The C ABI contains fixed integers, buffers and one opaque owned
storage+file object, never C++ layouts or callbacks into Rust. The native object
closes file before storage; exceptions never cross the ABI. No server runtime,
world API, shared cache/lock, duplicated client state or production gameplay
dependency is introduced.

This is a **Rust-owned prototype using C/C++ CascLib**, not pure-Rust CASC. A
pure-Rust replacement still requires its own license, offline, BLTE/encryption
and exact-byte proof; no third-party Rust candidate has been selected here.
