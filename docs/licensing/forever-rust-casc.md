# Pure Rust BLTE dependency proposal — 2026-10-11

**Status: recorded before dependency integration.** The operator authorized
publication of the exact closure, which was recorded in
[issue #1](https://github.com/alseif0x/rustycore-extractors/issues/1#issuecomment-6103725010).
This permits the bounded dependency integration described here; it does not
claim that a Rust CASC backend is complete.

The existing [native CASC closure](forever-casc-prototype.md) covers the pinned
CascLib, existing zlib and native adapters. It does not cover these Rust crates.
AGENTS.md requires the exact dependency/license closure in issue #1 before new
upstream code is imported. That gate is satisfied by the authorized record
above. No package download was needed.

## Original project code already permitted

`crates/forever-casc/src/blte.rs` and its private `blte/md5.rs` implement bounded
BLTE `N`/`Z` validation and MD5 checks as original GPL-3.0-or-later project code,
using the unmodified `miniz_oxide` dependency for zlib decompression.
No upstream implementation is copied or translated. BLTE wire-format facts
were checked against `dep/CascLib/src/CascReadFile.cpp` and `CascStructs.h` at
TrinityCore commit `02245dcd245e7433e524577656177723d3e4992e`; MD5 follows the
algorithm and constants specified in RFC 1321. The files carry authorship dates.
Tests use synthetic data, standard digest vectors and Python `hashlib` goldens.

The original default entry point rejects encrypted `E`, recursive `F` and
unknown modes. The later explicit supplied-key API is described below.
Neither path imports the CascLib key table or accesses the network.

## Exact registry closure

Source registry: `registry+https://github.com/rust-lang/crates.io-index`.
The following complete `.crate` archives already exist in the local Cargo
cache. Their SHA-256 values were computed locally. Every regular file in their
extracted source directories was compared byte for byte with its archive:
23 files for `miniz_oxide`, 13 for `adler2`, with no differences. No download was
performed for this review.

| Package | Exact version | Archive SHA-256 | Declared SPDX choices | Selected terms |
| --- | --- | --- | --- | --- |
| miniz_oxide | 0.8.9 | `1fa76a2c86f704bdb222d66965fb3d63269ce38518b83cb0575fca855ebb6316` | `MIT OR Zlib OR Apache-2.0` | MIT |
| adler2 | 2.0.1 | `320119579fcad9c21884f5c4861d16174d0e06250625266f50fe6898340abefa` | `0BSD OR MIT OR Apache-2.0` | MIT |

Direct dependency after the gate:

```toml
miniz_oxide = { version = "=0.8.9", default-features = false }
```

`miniz_oxide` has one mandatory dependency, `adler2`, declared with requirement
`2.0` and `default-features = false`; `Cargo.lock` must pin it to `2.0.1` with
the archive checksum above. No default or optional features are enabled for
either package. No build dependency, native library, assembly feature,
`simd-adler32`, serde, rustc workspace dependency, compression wrapper or
RustCrypto crate is in this closure. MD5 remains original project code.

The intended API is `miniz_oxide::inflate::core::decompress`, using a caller
bounded output buffer and the zlib-header/checksum flags. No allocation
feature is needed from the crate. The adapter must check stream completion,
exact consumed input, exact produced output and Adler32; it must reject extra
data, truncation, wrong declared lengths and policy-limit overruns. The pinned
CascLib can zero-pad short `Z` output; the Rust path must reject that case and
record this deliberate stricter behavior in compatibility evidence.

## Retained notices and source obligations

`miniz_oxide` includes `LICENSE` and `LICENSE-MIT.md`, retaining these notices:

- Copyright 2013–2014 RAD Game Tools and Valve Software.
- Copyright 2010–2014 Rich Geldreich and Tenacious Software LLC.
- Copyright (c) 2017 Frommi.
- Copyright (c) 2017–2024 oyvindln.

The package also retains `LICENSE-APACHE.md` and `LICENSE-ZLIB.md`.
`adler2` includes `LICENSE-MIT`, `LICENSE-APACHE` and `LICENSE-0BSD`; the last
preserves the Jonas Schievink copyright attribution. Keep every original
license and attribution file with the unmodified source archives even though
MIT is selected here. Do not remove notices, claim authorship of these crates
or copy their source into the project without a separately recorded reason.

The selected MIT terms are compatible with GPL-3.0-or-later. A later binary
distribution must provide the corresponding exact crate sources and retained
notices alongside the project source, lockfile and build instructions. A
cache path or this proposal alone is not corresponding source. No release,
remote request, key download or client-data publication is authorized here.

## Acceptance after the licensing record

1. Add the exact dependency and offline lockfile resolution; verify that the
   closure contains only the two packages above and matches both checksums.
2. Implement the bounded `Z` adapter without changing `N` validation or the
   caller's size/frame/encoded-byte limits.
3. Compare synthetic zlib vectors with an independent standard implementation;
   cover empty, stored/fixed/dynamic DEFLATE, frame mixtures, checksum errors,
   truncation, tails, bombs and wrong lengths. Preserve all-or-error output.
4. Review the locked dependency/features and run the focused local checks.
   This proves a BLTE primitive only; local indexes, ENCODING, ROOT, optional
   explicit keys, full CASC parity and reader/runtime acceptance remain separate.

## Original explicitly supplied Salsa20 support — 2026-10-11

`keys.rs`, `keys/salsa20.rs` and their tests are original GPL-3.0-or-later
project code. The implementation is authored from algorithm facts in
[D. J. Bernstein’s Salsa20 specification](https://cr.yp.to/snuffle/spec.pdf),
sections 3–10. No implementation source is copied, translated or imported.
Tests use the specification’s public quarterround/128-bit expansion vectors
and public eSTREAM 128-bit stream vectors recorded in
[Crypto++’s vector file](https://raw.githubusercontent.com/weidai11/cryptopp/master/TestVectors/salsa.txt);
no Crypto++ implementation or dependency is imported.

BLTE header facts are checked against the already pinned native closure’s
`CascDecrypt.cpp` decryption function and `CascReadFile.cpp` operation loop.
The existing native source/notice obligations are preserved. Its built-in
TACT key table is not copied into Rust or fixtures. The synthetic oracle links
the already accepted original native library but never calls its key loaders;
its fresh key map contains only keys supplied by the synthetic fixture. No new registry package, native dependency, download or
Cargo lock change is introduced.

The bounded profile is Salsa20/20 with 16-byte explicitly supplied keys only.
The existing default N/Z API and versioned acquisition/product CLI contracts
retain their scope. Every admitted encrypted result requires complete CKey
verification; secure erasure and adversarial provenance are not established.
See [the supplied-key API contract](../forever-local-casc.md#explicitly-supplied-salsa20-keys-library-api-2026-10-11)
for bounds and independent acceptance scope. This increment does not authorize
key discovery, client-data publication or a release.


## Navigation reader source profile v7

The Windows navigation readers, sequential caller-owned sink adapter and synthetic
tests are original Rust project source under their retained GPL-3.0-or-later notices.
The added `cfg(windows)` dependency is the existing local `forever-platform` crate;
it adds no registry package or external native source. The exact dependency closure
remains seven project crates plus `adler2` 2.0.1 and `miniz_oxide` 0.8.9, with unchanged
registry checksums and retained notices. Legacy import/reference inventories and
their original provenance remain unchanged; they are not evidence of new native
linkage. Historical v4/v5/v6 source and evidence references are retained. Source
inclusion does not establish Windows product, actual NTFS or runtime acceptance.
