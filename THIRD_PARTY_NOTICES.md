# Third-party notices

This file summarizes the components in the v0.1 extractor closure. Exact path
selectors, commits, use decisions and evidence are in
`licensing/inventory.json`, `licensing/source-files.json` and
`docs/licensing/source-closure.md`.

| Component | Copyright | License | Planned treatment |
|---|---|---|---|
| TrinityCoreLegacy extractor/common/DB2/collision source | TrinityCore, MaNGOS and earlier contributors; per-file history applies | GPL-2.0-or-later | Copy and modify while preserving headers |
| CascLib 3.0 | Copyright (c) 2014 Ladislav Zezula; later headers identify Ladislav Zezula | MIT | Copy the pinned vendored subset; preserve notice |
| CascLib lookup3 | Bob Jenkins, public-domain dedication | LicenseRef-Public-Domain | Copy the two pinned files and preserve the dedication |
| Recast and Detour | Copyright (c) 2009-2010 Mikko Mononen | Zlib | Copy pinned, locally patched sources; mark later changes |
| G3D 9.0 r4036, TrinityCore-modified | Copyright (c) 2000-2013 Morgan McGuire; embedded code copyright (c) 2001 Erik de Castro Lopo | LicenseRef-G3D-2000 AND HPND-sell-variant | Copy allowlisted sources/headers and reproduce both notices |
| Boost 1.83.0 | Boost contributors; file-level notices apply | BSL-1.0 | Reference-only; replace used Filesystem API with original `std::filesystem` adapter |
| zlib 1.3 | Copyright (c) 1995-2023 Jean-loup Gailly and Mark Adler | Zlib | Build the pinned static source subset on all supported platforms |
| {fmt} 9.1.0 | Copyright (c) 2012-present Victor Zverovich | MIT | Copy the pinned target used by Trinity string formatting |
| OpenSSL 3.0.13 baseline | OpenSSL Project Authors and contributors | Apache-2.0 | Link `libcrypto` for CascLib MD5/SHA1 adapters |

The G3D notice at the fixed commit is not byte-equivalent to the standard
BSD-3-Clause license: its third condition says the author's name *may* be used
for endorsement. It is therefore represented by the resolved custom SPDX
identifier `LicenseRef-G3D-2000`, not by `BSD-3-Clause` and not by an unknown
value.

The G3D copy excludes `g3dfnmatch.{cpp,h}` because those files carry the
four-clause Berkeley advertising license; a new portable wrapper replaces
them. The CascLib copy excludes its entire Overwatch directory, whose AES
files use the old OpenSSL license and whose generated CMF table cites only
informal TACTLib permission. CascLib's bundled MD5/SHA1 files are also
replaced with OpenSSL 3 APIs because the SHA1 header offers GPLv2 without an
“or later” grant or unspecified BSD terms. None of those excluded files is in
the distribution allowlist.

The upstream monolithic `common` target also names Argon2, SFMT, utf8cpp,
jemalloc, an OpenSSL-derived Ed25519 shim, short_alloc and a Valgrind interface.
They are not used by the four extractor functions and are excluded by replacing
that target with an extractor-focused common library. Their evidence and
licenses remain recorded as reference-only entries so a later build change
cannot silently pull them into a release.

Source redistributions retain the license headers in every third-party file.
Binary redistributions reproduce MIT, Zlib, G3D and other required
notices in accompanying documentation and satisfy the GPL corresponding-source
requirements described in `NOTICE.md`.
