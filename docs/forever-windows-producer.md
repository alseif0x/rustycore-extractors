# Windows loaded-producer source candidate

The separate [producer seal contract](../contracts/forever-windows-producer-contract-v1.json)
is `forever-windows-producer-seal-v1`. It supplies original bounded acquisition,
a pure PE verifier and an offline sealing tool. At that foundation checkpoint,
the Windows product driver, output publication and distribution were disabled;
preliminary production-API execution under Wine did not supply native acceptance.
The later [resident Windows product](forever-windows-product.md) uses this seal
with its separate NTFS backend and records actual native execution independently.
Neither checkpoint completes the extractor or a Windows distribution.

## Loaded evidence and candidate bytes

`windows::producer::LoadedProducer::capture(path::Limits)` reads the initialized
64-byte seal from the actual running module with a volatile operation. An
all-zero digest rejects before a module pathname query or file acquisition.
GetModuleHandleExW with FROM_ADDRESS binds that payload pointer to the actual
main module returned by GetModuleHandleW(NULL). Checked pointer subtraction
measures the payload start RVA. A disk offset, digest-slot RVA or a value
supplied by the file cannot replace that measurement.

GetModuleFileNameW supplies a candidate path using its exact spelling. The
existing read profile admits only bounded normal absolute drive paths on fixed
local NTFS: no UNC/device/verbatim prefix stripping, relative resolution or
observed reparse fallback. One ReadFile with explicit LinkPolicy::Allow retains
ancestors and the final file. Hardlink aliases remain subject to complete
identity checks and the loaded seal's content binding. No pathname alone
establishes the producer identity.

The acquisition API exposes copied payload bytes, the measured start RVA,
bounded complete candidate reads and `verify_unchanged`; it exposes no raw
handle or pathname. Rechecks compare the actual loaded payload/RVA and retained
held/named identities. Any observed recheck or source-read failure permanently
poisons the acquisition. Allocation and limit refusals do not establish a file
mutation. This remains metadata observation, not an atomic snapshot or
a defense against privileged modification of process memory.

`producer::verify_pe(data, loaded_payload, measured_payload_rva)` is pure Rust
and accepts caller-supplied evidence. `producer::loaded_identity()` on Windows
connects the actual acquisition, complete candidate read, pure verification and
final identity recheck. The isolated example exercises that helper; the product
CLI does not dispatch to it. Callers of the pure verifier must obtain actual
loaded evidence independently before claiming a loaded producer identity.
The safe QA helper `loaded_identity_evidence()` returns `VerifiedProducer` with
copied complete-file SHA-256 and measured payload RVA. Both helper paths perform
the final recheck even when PE verification fails.

## Strict PE and seal profile

The admitted image is unsigned x64 PE32+ with console subsystem 3, a 240-byte
optional header, exactly 16 data directories and 1..96 sections. PE checksum
must be zero. File/image size is bounded; raw and virtual sections have checked,
monotonically nonoverlapping spans. Section alignment is a power of two from
4,096 to 65,536; file alignment is a power of two from 512 through section
alignment. Headers must fit and obey file alignment.

Exactly one `.rceid` section has 64 virtual bytes, at least 64 initialized raw
bytes, zero raw padding and exact flags `0x40000040` (readable initialized data).
The payload begins with the ASCII marker `RCE-FOREVER-PE-SEAL-V1`, padded with
ten NUL bytes to 32 bytes, followed by the 32-byte digest **D**.

The verifier requires all three bindings:

1. The measured loaded payload **start** RVA equals `.rceid`'s payload RVA.
2. The file's digest slot equals the nonzero digest D from the loaded payload.
3. SHA-256 of the **entire file**, with only that 32-byte slot replaced by zeros,
   equals D.

Success returns SHA-256 of the complete sealed file, including the actual D.
The canonical hashing view does not omit overlays, padding, headers or other
file bytes. This is an integrity binding to the admitted loaded seal; there is
no publisher signature, trust root or complete measurement of loaded code pages.

Certificate/security, architecture, global pointer, load configuration, bound
import, delay import, CLR and reserved directories are rejected (indexes
4, 7, 8, 10, 11, 13, 14 and 15). Every admitted data directory excludes the seal.
Bounded relocation, import and TLS parsing additionally prevents loader-write
targets from overlapping it. Relocations admit ABSOLUTE and DIR64 only; imports
check descriptors, complete lookup/destination thunks and terminators. TLS has
an exact 40-byte directory, an excluded index write and bounded callbacks into
executable sections. This is an explicit subset of PE, without general signed
or unsigned image compatibility. These checks inspect declared static loader
metadata; they do not validate complete Windows loader safety or arbitrary
startup behavior.

DIR64 and IAT destination eight-byte ranges, including terminating IAT thunks,
also reject intersection with the declared relocation or import directory.
An original-thunk-zero import may still use its current IAT slot as lookup.

## Bounds and unsafe-code ownership

- Candidate and image: at most 256 MiB.
- A read chunk: at most the existing 64 MiB primitive limit; at most 4,096 chunks.
- Sections: at most 96, in fixed parser storage.
- Relocation and import directories: at most 1 MiB each, with at most 65,536
  entries/thunks for each work budget.
- TLS callbacks: at most 1,024.
- Existing read lease: at most 64 retained filesystem handles and a peak of 65.

Caller limits only reduce these ceilings. Captured size and chunk count are
admitted before a fallible complete-file allocation. Rust canonical hashing streams
one zero-slot view rather than making another complete file copy. These are
logical input/work/handle limits, without allocator, kernel-memory or RSS proof.
The original loaded-module FFI stays scoped inside forever-platform. The
extractor retains its unsafe-code prohibition. No upstream implementation or
new registry dependency is imported.

## Offline artifact sealing

The authored tool preserves the input and writes a new exclusive output:

```text
python3 tools/seal_forever_windows.py seal --input FILE --output NEW
python3 tools/seal_forever_windows.py seal --input FILE --output NEW --zero-checksum
python3 tools/seal_forever_windows.py verify --input FILE --loaded-payload HEX128 --payload-rva INT
```

Python canonical hashing also uses memoryview slices around the zero digest
slot; creating the sealed output still allocates a complete new file buffer.
The explicit `--zero-checksum` option permits zeroing the PE checksum in the new
output before canonical hashing. The verifier always requires a zero checksum;
it never silently normalizes it. Sealing does not rewrite the source in place,
download inputs, contact a network or publish an artifact.
Failed new outputs remain for inspection. The tool does not unlink an output by
pathname after an error. A file fsync supplies no parent-directory durability or
Windows crash-publication proof.

Tool verification uses an operator-supplied 64-byte payload and start RVA. Its
result is artifact integrity with supplied seal values. Only the loaded helper
acquires evidence from the actual running process. Neither command enables a
Windows extraction or publication backend.

## Evidence and remaining gates

The external `windows-producer-prototype-20261011T174041Z.json` (SHA-256
`32de82efc07e3b2e04af58be89233d4d538f7e1f1640e141554088fb6d635cff`)
records 29 prototype cases and preliminary Wine evidence. Its accompanying
design report is `windows-producer-design-20261011T174041Z.json` (SHA-256
`c1223456703299e7c9cbf04623536809f2ce9c0c3e631270dfc9b06e19296e5b`).
Those reports predate this production source integration; they remain historical
prototype evidence and do not accept the production implementation.

The integrated candidate's focused run passes 139 contract tests, including
14 new static producer checks, and 94 distribution tests: 53 packaging guards
and 41 sealer cases. Both suites exit zero with all 921 source files unchanged;
temporary files and Python caches remain external. Static contract checks are
source/profile consistency only, without loader or filesystem execution proof.
These runs do not create a v4 bundle or execute the loaded Windows helper.

The corrected production helper closeout is accepted and terminal at
`windows-producer-consumer-20261011T180421Z/final-control-check/closeout.json`
(SHA-256 `e086d4459640c898fa0fc7e63c9c6d788a7c230c0aa6bd3688bd4532e9dcdbd1`);
its runtime report SHA-256 is
`ef7e8b62a50d34629d5d687437c838120c7b86b578ef25c8d9eb46c63f9878df`.
The final unchanged consumer source passes 37 Rust tests in each profile,
41 Python cases and Windows workspace all-target Clippy. Six actual helper
cases under Wine cover debug/release sealed success, `producer_unsealed` and
post-seal tampering rejected as `pe_digest_mismatch`. Measured payload RVAs
are 5,009,408 and 2,412,544 respectively and match the admitted file layouts.
Earlier pre-correction probes remain historical, superseded by this corrected
source execution. This is preliminary Wine evidence for the production API,
without actual Windows/NTFS or enabled product acceptance.

Filtered network traces describe observed calls; denied attempts may be absent,
and individual exit of every Wine descendant remains unproven. Complete
corresponding-source v4 bundle acceptance remains pending.
Actual Windows/NTFS, real reparses, output DACL,
publication/durability/recovery and complete Windows distribution remain gates.
There is no new real-client, RustyCore runtime, hosted CI or release acceptance.

Primary API/format references, without imported implementation:

- [Microsoft PE format](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format)
- [GetModuleHandleExW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulehandleexw)
- [GetModuleFileNameW](https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getmodulefilenamew)
