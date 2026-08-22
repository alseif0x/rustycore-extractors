# Pinned C++ extractor source and dependency closure

This audit is based exclusively on Git objects at
`TrinityCoreLegacy/TrinityCore@92796557f9b0ba3d2d1c7c770f535153154cf83e`.
The local reference checkout was not switched or modified. Paths below are
repository-relative at that commit. This is an engineering provenance record,
not legal advice.

The exhaustive source/header allowlist is generated in
`licensing/source-files.json`; the selectors in this document explain that
file rather than replacing it.

## Executable roots

| Executable | Source selector at the pinned commit | Direct upstream targets |
|---|---|---|
| `mapextractor` | `src/tools/map_extractor/**` | `extractor_common` |
| `vmap4extractor` | `src/tools/vmap4_extractor/**` | `extractor_common` |
| `vmap4assembler` | `src/tools/vmap4_assembler/{TileAssembler.cpp,TileAssembler.h,VMapAssembler.cpp}` | `common`, `casc`, `zlib` |
| `mmaps_generator` | `src/tools/mmaps_generator/**` | `common`, `extractor_common`, `Recast`, `Detour`, `zlib` |
| shared extraction layer | `src/tools/extractor_common/**` | `casc`, `common` |

On Windows the original CMake files also append
`src/common/Debugging/WheatyExceptionReport.{cpp,h}` and
`src/common/WindowsSettings.manifest`. Those crash-reporting files are not
functionally required by extraction and will be substituted by normal platform
error handling rather than copied.

## Trinity source selected for the standalone common layer

The original `src/common/CMakeLists.txt` recursively compiles nearly all of
`src/common` and publicly links many server dependencies. The standalone suite
must not reproduce that accidental closure. Its bounded compatibility layer is:

- core types/version/banner: `src/common/{Banner.cpp,Banner.h,Common.cpp,Common.h,CompilerDefs.h,Define.h,GitRevision.cpp,GitRevision.h}` plus a standalone generated revision header;
- DB2 loading: every file under `src/common/DataStores/` at the fixed commit;
- assertions: `src/common/Debugging/{Errors.cpp,Errors.h}`;
- CASC network helper declarations: `src/common/Asio/{AsioHacksFwd.h,IoContext.h,Resolver.h}`; online use remains opt-in and is governed by the network contract;
- utility headers used by extractor code:
  `src/common/Utilities/{ByteConverter.h,Containers.h,Duration.h,EnumFlag.h,IteratorPair.h,Locales.h,Memory.h,Optional.h,StringFormat.h,Types.h}`,
  `src/common/Containers/Utilities/MapUtils.h`,
  `src/common/Threading/ProducerConsumerQueue.h`, and
  `src/common/Time/Timer.h`;
- collision/VMap reader:
  `src/common/Collision/{BoundingIntervalHierarchy.cpp,BoundingIntervalHierarchy.h,VMapDefinitions.h}`,
  `src/common/Collision/Management/{IVMapManager.h,VMapManager2.cpp,VMapManager2.h}`,
  `src/common/Collision/Maps/{MMapDefines.h,MapDefines.cpp,MapDefines.h,MapTree.cpp,MapTree.h}`,
  and `src/common/Collision/Models/{ModelIgnoreFlags.h,ModelInstance.cpp,ModelInstance.h,WorldModel.cpp,WorldModel.h}`.

Calls from these files into Trinity server logging, metrics, broad utility and
revision-generation infrastructure are substitution boundaries. Issue #3 must
provide small GPL-3.0-or-later standalone implementations; it must not widen
the import selector without first updating this inventory.

## Third-party compilation closure

- CascLib: the CascLib 3.0 MIT core and transitive headers allowlisted in the
  source manifest. Bundled MD5/SHA1 are replaced by OpenSSL 3; the Overwatch
  APM/CMF directory is excluded from the `wow_classic`-only build. Jenkins
  lookup3 remains as a separately inventoried public-domain component.
- G3D: the sources named by `dep/g3dlite/CMakeLists.txt` plus their transitive
  headers, except `g3dfnmatch.{cpp,h}`. Those advertising-clause files are
  replaced by a new portable filename-matching wrapper.
- Recast/Detour: exactly the source lists in
  `dep/recastnavigation/{Recast,Detour}/CMakeLists.txt` plus their `Include/`
  headers. Recast is needed only by `mmaps_generator`; Detour is needed by the
  generator and fixes the serialized MMap ABI.
- fmt: the 13 headers and two sources named by `dep/fmt/CMakeLists.txt`.
- Boost: the upstream CMake asks for system, filesystem, program_options,
  iostreams, regex and locale, but the selected extractor source only needs a
  narrow Filesystem surface after online CascHandles code is removed. Original
  GPL-3.0-or-later compatibility headers map that surface to C++20
  `std::filesystem`; no Boost header or library is compiled or distributed.
- zlib: version 1.3 is bundled at the fixed commit. The standalone target builds
  the pinned allowlisted static sources identically on Linux and Windows.
- OpenSSL: OpenSSL 3 `libcrypto` replaces CascLib's bundled MD5/SHA1. The
  standalone baseline is OpenSSL 3.0.13 under Apache-2.0; no TLS client and no
  old-OpenSSL-license source is selected.

## Excluded upstream target drag

`src/common/CMakeLists.txt` publicly links `argon2`, `sfmt`, `utf8cpp`,
`jemalloc`, `openssl_ed25519`, `short_alloc` and `valgrind`. Inspection of the
four tool roots and the selected common/DB2/collision layer found no extractor
function requiring those targets. They are recorded in the inventory with
`reference-only` or `substituted` treatment and must not appear in a v0.1
binary dependency or corresponding-source manifest.

## Evidence reviewed

The audit used `git show` for all target CMake files; root `COPYING` and
`AUTHORS`; `dep/CascLib/LICENSE`; `dep/recastnavigation/License.txt`; G3D
`Readme.txt`, headers and `source/license.cpp`; `dep/zlib/zlib.h`; dependency
license/readme files and representative file headers. Git history at the fixed
commit identifies CascLib upstream commit
`5c60050770767f2e606f6fec0c35beb8b9b00c60`, Recast base
`54bb0943e5174a71eeeca11919920f685760a4f0` followed by Trinity patches, G3D
9.0 r4036 followed by documented Trinity hotfixes, fmt 9.1.0, utf8cpp 3.2.3,
zlib 1.3 and jemalloc 5.2.1.

Per-file review additionally found public-domain Jenkins lookup3 inside
CascLib, old-OpenSSL and informal TACTLib material under its Overwatch
directory, GPLv2-or-unspecified-BSD SHA1 code, and BSD-4-Clause Berkeley
fnmatch code inside G3D. The allowlist retains lookup3 with its dedication and
excludes or substitutes every other problematic path.

## Gate for issue #3 and releases

The import is recorded by `licensing/import-manifest.json`. Every imported path
must remain in `licensing/source-files.json`, preserve its header and carry a
dated modification notice when changed. The `source-closure` CTest enforces the
exact 355-file set, all declared notices, excluded paths and the remote-CASC
lock. A missing grant, GPL-2.0-only header, incompatible term, or path outside
the selectors blocks the import. Before binary distribution, capture the actual
linker dependency list and compare it with this closure; any new component
reopens the licensing gate.
