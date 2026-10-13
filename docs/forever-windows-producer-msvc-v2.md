# Explicit MSVC producer seal v2 candidate

`forever-windows-producer-seal-msvc-v2` is a separate, bounded profile for the
observed x64 MSVC product. Enable `windows-msvc-seal-v2` on forever-extractor;
it enables `producer-msvc-v2` on forever-platform. The initialized payload uses
`RCE-FOREVER-PE-SEAL-V2` with ten NUL bytes, followed by the 32-byte digest.
The feature affects the payload only for MSVC. V1 remains the default and keeps
its own marker, parsing rules and rejection of directory 10. Old bundles and
their feature-free recipes are not migrated or accepted as v2.

## Binding and host boundary

V2 retains the same complete-file SHA256 binding: only the 32-byte digest slot
is zeroed in the canonical hashing view. Every other byte, including headers,
metadata, padding and overlays, remains covered. Verification requires the
payload and its start RVA measured independently from the actual main module,
plus the held candidate file and ancestor identities. No profile fallback or
disk-derived replacement for loaded evidence is admitted.

Each v2 loaded snapshot requires a successful `IsWow64Process2` result with
process machine UNKNOWN (0) and native machine AMD64 (`0x8664`) before pathname
or filesystem acquisition. An error or emulated process rejects. This does not
accept ARM64 emulation or validate that platform's volatile-metadata consumer.
The borrowed current-process pseudo handle is not an owned filesystem handle.

## Closed observed metadata shape

The separate [v2 contract](../contracts/forever-windows-producer-msvc-contract-v2.json)
admits a complete 320-byte load-config directory with GuardFlags `0x100`, nine
explicit VA fields and zero unknown fields. It validates the cookie and seven
guard slots as conservative eight-byte destinations, disjoint from the seal and
parsed controls. Six initial slots contain executable code VAs; CastGuard is
zero. The admitted DllCharacteristics value is exactly `0x8160`; read-only
metadata/slots require `.rdata` / `0x40000040`, executable targets require
`.text` / `0x60000020`, and the cookie requires `.data` / `0xc0000040`.
Every config VA and nonzero slot VA requires one exact DIR64 fixup.
Duplicate, partial or untyped control fixups reject. This preserves ASLR.

The volatile object is an empirical 88-byte shape: a 24-byte header, forty bytes
containing ten unique sorted access RVAs, and twenty-four bytes containing three
nonoverlapping code-range pairs. The second header word must be exactly
`0x80038003`; it is not labelled an official version or decoded into invented
flags. The complete object is contiguous, all accesses and ranges are mapped
executable code, and every access belongs to a declared range. Other shapes
remain unsupported. This is not general volatile-metadata ABI support.

The relocation/import controls remain protected. V2 also protects load-config,
the volatile object, and the complete TLS directory/callback table against
untyped writes. Standard nonzero TLS VA fields and callback entries have exact,
single DIR64 exceptions; IAT and TLS-index writes cannot overwrite those
controls. The profile does not enumerate all Windows loader or CRT writes,
validate arbitrary startup code, authenticate a publisher or measure every
loaded code page. Logical limits are not an RSS or all-CPU-work measurement.

Files/images are capped at 256 MiB; relocation and import directories at 1 MiB
each. Relocation entries and cumulative import-thunk visits are separately
capped at 65,536. TLS terminates within 1,024 slots (at most 1,023 callbacks).
Rust's fallibly reserved vectors have separate ceilings of 65,536 relocation
writes, 65,536 IAT writes, 65,568 protected controls and 1,043 typed fixups,
with reservation chunks of at most 256 entries and no recursive parser stack.
These are conservative parser storage/work bounds, not exact heap or RSS bounds.

## Offline tool and native acceptance

The sealer requires explicit `--profile forever-windows-producer-seal-msvc-v2`
for v2; its default remains v1. Inputs must carry the corresponding exact
marker. Only the existing explicit `--zero-checksum` transformation is allowed;
source bytes are preserved and the new output is exclusive.

Native acceptance builds debug/release with the explicit feature, seals both
under v2, then runs all fifteen resident-product CLI cases per profile. The
test assets are authored synthetic data; no real client or key is uploaded.
Its source guards, complete command reports and artifact comparisons are
separate evidence from pure parser tests and this source contract.

The following records the authored contract checkpoint: implementation, pure
tests and native product acceptance were pending. The original v1 CI failures remain failed and retained in
[the resident product report](forever-windows-product.md). The metadata
observation from run 38345728238 is not a successful v2 execution. Reproducible
Windows distribution, real geometry/runtime and full extractor completion
remain separate gates. No release is authorized by this candidate.

## References and observation limits

Microsoft defines the public
[load-config fields](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_load_config_directory64)
and explains
[CFG metadata and slot writes](https://learn.microsoft.com/en-us/windows/win32/secbp/pe-metadata).
The public
[volatile compiler option](https://learn.microsoft.com/en-us/cpp/build/reference/volatile)
describes an emulation purpose, without establishing the observed private
header's general ABI. Its closed shape here comes from retained compiler bytes,
not a claim that those documents specify `0x80038003`. The
[machine query](https://learn.microsoft.com/en-us/windows/win32/api/wow64apiset/nf-wow64apiset-iswow64process2)
supplies the explicit runtime host boundary.

## Subsequent local source acceptance — 2026-10-13

The completed implementation passes 58 focused Rust producer tests in both
profiles (37 retained v1 and 21 new v2) and 81 Python sealer tests, with one
actual-Win32 case skipped on Linux. Independent read-only review found no
remaining blocker for this bounded profile. All-target feature-enabled Linux
and Windows GNU lint/compile checks pass; GNU remains compile-only and retains
the v1 payload under its target environment.

The coordinator's final frozen-source run passes 2,743 workspace tests / 26
ignored across 42 targets per Linux debug/release profile, 280 contracts and
158 distribution tests / one Windows-only skip, plus all-target Linux Clippy
and complete formatting. All 1,078 source files remain unchanged during that
run. The earlier attempt is retained as a failed source-freeze check: all six
commands passed, but one Clippy-equivalent alignment expression changed during
the run. Only the subsequent unchanged-source run is accepted.

External retained evidence under
`/mnt/nvme2/datos/forever-product-runs/windows-product-20261013-bc74jzeo/`:

- `root-v2-acceptance-final/local-validation.json`:
  `9ae4a47bc03a387df25ca527b8bff97857d0ca38c927710559cd2a52a1ec2b9c`.
- `pe-v2-worker-validation.json`:
  `d33cc8c6ff9f4762d1fc49ccbe196299afd87d05ce75c2c948f7040f1a19aae5`.
- `lab/package/reports/pe-v2-negative-matrix.md`:
  `b5e602621136b91bdffc47ab3b948724a45327b33f79bf027e999b6e509e6930`.

The authored contract's pending statuses retain their original checkpoint;
this section supplies subsequent local evidence. Documentation of these
results is later than the frozen run and changes no Rust/Cargo or Python source.
Native MSVC product execution, Windows distribution and complete real-client /
runtime extraction remain pending. No full extractor or release acceptance is
established by these local checks.


## Subsequent native preflight — product import still fails

Run 38348243944 / Windows job 115134527122 on functional commit
`5440090015fc618ad39ff3326eed0b1c61a87a67` passes both v2 offline seal
preflights, explicit-feature MSVC lint and both native machine-query tests.
The complete Windows workspace and focused namespace/package tests also pass.
The product-case import then fails on Python-3.11-incompatible authored fixture
syntax; zero actual product CLI cases execute. This is retained failed native
product evidence, not successful loaded-v2 binding or full-product acceptance.
See [the complete retained run](forever-windows-product.md) for counts, hashes
and scope. No Rust parser or payload change is required by that import failure.

## Subsequent native debug execution — incomplete suite

Run 38349096456 / Windows job 115137168501, functional commit
`3770dc89491d8b0299c3403b26ca7ac71595a30e`, passes the corrected Python-3.11
import and fourteen actual debug product methods using the loaded sealed v2
producer. Both build/locale all-component goldens, verification, toy-key
decoding and interruption/resume have separate positive evidence. One method
errors because its Python call to `locate` still selects the default v1
marker; it never executes its modified-seal candidate. The suite fails and
release is not reached. The fix is an explicit v2 selection in that test,
with no Rust parser, payload or default-profile change.

The next CI plan gives the complete two-profile product helper a dedicated
20-minute job and logs its run/checkout identity. Existing workspace/preflight
and lint checks remain. See [the retained failed attempt](forever-windows-product.md)
for exact binary, source and report hashes. Partial debug success is not full
native product, Windows distribution, real-client or runtime acceptance.
