# Windows private-output foundation v1

Authored 2026-10-12. This is an original GPL-3.0-or-later library foundation in
`forever-platform`, separate from the Windows read and loaded-producer profiles.
Ordinary synthetic native Windows x86_64/fixed local NTFS acceptance is recorded
for functional commit `a220596da8a29f9f3a867dc7f2bd016d01cd3558` on 2026-10-12.
Source contract checks are consistency checks; the terminal native CI evidence
and its precise scope are recorded below. Full Windows product acceptance
remains open.

The module does not enable the Windows product driver, package publication,
recovery or distribution. The Linux hardlink-based package/recovery protocol is
unchanged and is not implemented by this module. Full Forever extraction,
real-client geometry and RustyCore runtime acceptance remain separate gates.

## API and admission

`windows::output` creates a new private directory below a retained
`windows::ReadDirectory` on a fixed local NTFS volume, on Windows x86_64.
The parent retains the existing read profile's admitted normal absolute path,
ancestor/reparse guards and read-sharing restrictions. Parent and owned
directories must report zero flags for `FileCaseSensitiveInformation` (class
71); case-sensitive directories reject. No existing child is adopted, opened
as an output or overwritten.

The safe owning `PrivateRoot::create(&ReadDirectory, name, Limits)` returns an
owner with opaque, copyable `Directory` and `OutputFile` tokens. `root()` returns
the root directory token; `create_directory` and `create_file` add children.
`append`, `flush` and `rename` operate on tokens belonging to this same owner.
`verify_unchanged()` checks retained and relative named objects; `stats()` reports
owned objects/handles, charged bytes and operations. The root borrows its parent
lease for its lifetime, owns all created handles and is not `Sync`. Tokens expose
neither raw handles nor paths. Errors are static `windows_output_*` codes.

Creation uses a single relative name and a retained directory handle as
`OBJECT_ATTRIBUTES.RootDirectory`, with `NtCreateFile` disposition `FILE_CREATE`.
This makes an existing name a failure. It does not canonicalize a new absolute
output pathname or infer source/key/output separation for a caller.

The caller owns the relationship between this private root, protected inputs and
the eventual package protocol. A private output handle is not a product artifact,
manifest receipt, completed package or authorization to publish.

## Security boundary

Creation supplies an explicit protected DACL and the process token's user SID as
owner. The admitted DACL has one non-inheriting ALLOW ACE for that same SID with
`FILE_ALL_ACCESS`; it does not copy a permissive parent ACL. Resulting handle
security is checked. Filesystem operations reject thread impersonation rather
than silently switching which identity is admitted.

This is discretionary access control for the current process user. Other
processes with that same SID have the same ACL rights; administrators, backup /
restore privileges, kernel access and raw-volume access are outside the boundary.
The module does not authenticate a caller or defend against arbitrary code in
the same process. Handle sharing and identity checks have their own stated
scope; an ACL is not a filesystem snapshot.

## Handle sharing and observed changes

A directory keeper requests `FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES |
READ_CONTROL | SYNCHRONIZE`, shares read/write and denies delete sharing. Its
relative target/probe handle requests `FILE_TRAVERSE | FILE_READ_ATTRIBUTES |
READ_CONTROL | SYNCHRONIZE` and shares read/write/delete, for compatibility with
Windows' internal relative rename target open. Directory write sharing admits
write, reparse and metadata access: this is not directory write exclusion.
Pre/post handle identity, attributes and security checks detect observed changes
and permanently poison the owner. They do not establish an atomic namespace
against another process with the same SID or an administrator.

A file writer requests `FILE_READ_DATA | FILE_WRITE_DATA | DELETE |
FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE` and shares read only. Its
probe requests `FILE_READ_ATTRIBUTES | READ_CONTROL | SYNCHRONIZE` and shares
all three modes.
The writer denies competing data-write/delete opens while allowing readers
whose share modes are compatible in both directions. `WRITE_ATTRIBUTES`,
`WRITE_DAC` and `WRITE_OWNER` are not data-write sharing access; same-SID callers
can still mutate metadata/security. Files also have pre/post identity, attributes
and security checks with permanent poison after an observed failure. This is
not an atomic snapshot. These file sharing guarantees must not be applied to
directories. Untracked child creation is not rejected by inventory enumeration:
there is no closed-directory membership proof.

## Resource policy

Limits only reduce 128 objects, directory depth 16, 64 MiB per file, 256 MiB
total appended bytes, 1 MiB per append operation and 16,384 operations. Full
parent/root/relative paths remain bounded by 32,760 UTF16 units, checked before
creation or rename. Names are bounded single components, at most 64 bytes, in
the module's lowercase ASCII grammar. Limits
include the private root among objects and assign it depth zero. Every child
edge increases depth, including files and a renamed file's destination. Root
creation starts the operation counter at one; child creation, append, flush and
rename charge before mutation. Explicit metadata verification does not consume
this counter. Each append charges the whole admitted request before writing,
never refunds a failed partial write, and flushes the file before success.

Names use only ASCII `a-z`, `0-9`, `.`, `_` and `-`. Empty names, `.`, `..`,
`.git`, trailing dots, DOS reserved stems (including extensions), uppercase,
non-ASCII, whitespace, separators and stream syntax reject.

These are admission counters, not an allocator, process RSS, all CPU or physical
free-space guarantee. Partial I/O and full disks can still fail after admission.

## Publication and failure scope

Handle-relative no-overwrite rename applies only to `OutputFile`; directory
rename is not exposed. The successful file token retains its binding with the
new name and parent. This is a library operation. It supplies neither
the Linux package protocol nor a Windows replacement for its receipts, sealing,
restart recovery and durability model. File flushing does not establish durable
directory entries, an atomic whole-package commit or crash recovery. There is no
pathname cleanup or adoption of an abandoned private root. Invalid arguments,
foreign tokens, names, limits and collisions already known to the owner reject
without poisoning. I/O or observed guard failure irreversibly poisons the owner;
no rollback of a partial write is promised.

## Validation gate

The companion contract is `contracts/forever-windows-output-contract-v1.json`.
Its Python checks compare source, API limits and workflow boundaries; they do
not execute Windows calls. Native tests require an actual Windows x86_64
process and a fixed local NTFS temporary directory. Wine and GNU-Windows cross
compilation cannot establish this gate.

### Accepted ordinary native foundation — 2026-10-12

The terminal [CI run 38254527798, attempt 1](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798)
and [Windows job 114829043981](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798/job/114829043981)
pass on Windows Server 2022 (`windows-2022`), OS `10.0.20348.0`, with fixed local
NTFS explicitly admitted for the temporary fixtures. Both focused steps log the
OS version and filesystem admission without printing SID or fixture paths.
The exact listed and executed `windows::output::tests::` inventory contains
fourteen ordinary cases in each debug/release profile, with zero failures and
zero ignored cases. The complete Windows workspace debug run also includes
these fourteen cases.

| Execution | Passed | Failed | Ignored | Workspace result summaries |
| --- | ---: | ---: | ---: | ---: |
| Windows workspace debug | 2,151 | 0 | 21 | 41 |
| Windows private-output focused debug | 14 | 0 | 0 | — |
| Windows private-output focused release | 14 | 0 | 0 | — |
| [Linux workspace debug](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798/job/114829044031) | 2,643 | 0 | 26 | 41 |
| [Linux workspace release](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254527798/job/114829044069) | 2,643 | 0 | 26 | 41 |

The separate [C++ CI run 38254528030](https://github.com/alseif0x/rustycore-extractors/actions/runs/38254528030)
builds the exact same functional commit successfully with Linux Ninja and
Windows Visual Studio 17. This is build evidence; it does not execute extraction
or establish C++/RustyCore runtime compatibility.

The workflow selects the platform library with `--offline --locked`,
`--no-default-features` and filter `windows::output::tests::`; each profile first
checks `--list` against the fourteen literal contract names and then executes
with `--test-threads=1`. Release adds `--release`. These counts come from the
reviewed hosted logs, separately from the local compile/source-check counts.

The independent PowerShell `Get-Acl` / `SecurityIdentifier` oracle verifies
**persisted** directory/file ownership and DACL after `root.verify_unchanged()`,
`drop(root)` and `drop(parent)`: exact process user SID, protected DACL, one
explicit FullControl ALLOW ACE, no inherited ACE and no inheritance/propagation
flags. Its path probe while leases are held is diagnostic only, without an
acceptance assertion or an inferred sharing/NTFS cause. Child stderr is
suppressed; stdout is parsed only when at most 128 bytes and exactly two `i64`
exception codes are present. Raw child bytes, SID and paths are never formatted.
The child alone removes `PSModulePath` before Windows PowerShell starts, following
the documented [PowerShell module-path inheritance rule](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_psmodulepath?view=powershell-7.6).

The ordinary DACL-change/restore case uses test-only `NtSetSecurityObject` to
restore the original descriptor control; production owner/DACL checks stay
strict and the already observed failure remains poisoned. The ordinary
`ImpersonateSelf`/RAII `RevertToSelf` case passes rejection before creation and
permanent poisoning of an existing root. Compatible owner reads while file
keepers remain held have separate ordinary test coverage; the persisted ACL
oracle does not establish live path-reader compatibility.

### Source association and documentary checkpoint

| Identity | Value |
| --- | --- |
| Functional commit accepted | `a220596da8a29f9f3a867dc7f2bd016d01cd3558` |
| Functional Git tree | `5014b5f12020fc5324c6d4deca1d2335fa9077b4` |
| Actual CI checkout merge | `8a208b5d01c06dccea92f82cb5cca5822eb13a45` |
| Actual checkout tree | `5014b5f12020fc5324c6d4deca1d2335fa9077b4` — exact functional-tree equality reviewed |
| Compile6 source freeze SHA-256 | `1de7e5ae036d61f5fbabea51fdf2a152486a6622cd9ba6381b0958fc900479cf` / 1,023 files |

This acceptance belongs to that functional source and the reviewed equal-tree
CI checkout. The later commit recording this documentary closeout was not
rebuilt or executed by this run. No accepted binary is replaced and no new
Linux or Windows bundle is built by these documentation changes. The v9 source
profile and historical accepted `b692ed3` Linux bundle retain their scope.

The frozen contract's `validation.native_windows_ntfs_accepted=false`,
`validation.hosted_ci_executed=false`, pending `status`, and
`validation.independent_acl_oracle` description are the authored checkpoint
before this native closeout. In particular, its earlier oracle description
predates the persisted-oracle scope and numeric diagnostic harness above.
They are not retroactively rewritten. Actual functional source and the later
terminal report establish the current persisted-oracle execution scope.
Product/package/recovery/publication flags remain false because those features
are still outside the accepted foundation.

### Retained reports and earlier attempts

Evidence is retained outside Git under
`/mnt/nvme2/datos/rustycore-extractor-work/runs/candidate/windows-private-output-20261012T115925Z`.

| Evidence | SHA-256 |
| --- | --- |
| `native-ci4-root-review.json` — accepted terminal native scope | `353c2a48b8094eb4d2dbc5ba42bcc7fd61b505117cf3484f715edb60a757afda` |
| CI metadata | `2426a37fe19c707b2eb9d832cb160823082d32a2400af8c51dd14ba950d4fd09` |
| CI API run record | `6ac7b377dc8eec78cd74092403095a0928021a3dcbb5de94434ac7d41c5f04a9` |
| Windows job log | `962c0fe700c4f1e57e3eb77ad0a372db82d6dc168b2b699ce8680ab1aa42bb81` |
| Linux debug job log | `81fb072d5f3c7acfdb18676aa180f06a1ae50ec5f23866c5caa43e0c0c2b7ca8` |
| Linux release job log | `0466c2e94d5f789c78e40ae0e063d4548423c270ee999cd7eaed35d0a41a10b9` |
| `compile2-root-review.json` — scoped source/compile checks only | `44679f5dec31d6831e7f104c2539177a1103889be920c5c16a22ac640fcde486` |
| `compile3-root-review.json` — scoped source/compile/lint checks only | `5051af2996004247d85cbd1c1e8ee6013d2e3f4dea845f1808056aa265f54396` |
| `compile5-root-review.json` — scoped source/compile/lint checks only | `4dd2ff3b08fe03b35066d308b5118fb3ea5f33f2653a3016fce353ccb53116b3` |
| `compile6-root-review.json` — local source/compile checks only | `63297be4032a25e53ab03e49445a92d6878290b5f4b70931b606fbf61d0f1ecf` |
| Compile6 remote command report | `f9ee1e7ad3148ca16043a78d64d2eadf1a0c57dfee457061c6d30d78d9417eaa` |
| Compile6 terminal observer | `228433509a1211bbd5524b78f33cde581e40a4685334391e5518bc2a43ef3672` |

The accepted `compile2-root-review.json` covers scoped formatting, 203 static
contracts, 112 distribution/sealer checks and GNU-Windows debug/release
compilation. Accepted compile3, compile5 and compile6 additionally cover Linux
and GNU-Windows Clippy, with those same source-check and compile categories.
Each report retains its own source freeze; none executes native Windows.
Their acceptance does not relabel earlier snapshots as native-tested.
Native CI1/CI2/CI3 failures and static compile1/compile4 failures remain retained
and unaccepted; CI4 is the later ordinary native closeout.

### Gates still open

The separate `windows::output::privileged_tests::` reparse case remains ignored
by default and needs explicit native Windows symlink privilege/NTFS execution;
ordinary CI does not close that privileged gate. No junction/cloud-reparse or
general privileged mutation acceptance follows. The foundation does not enable
the Windows driver, package protocol/receipts/sealing, recovery, durable directory
entries, crash durability or a reproducible Windows product distribution.
The Linux hardlink-based recovery protocol is not migrated or relabeled.
Real complete inputs/geometry, full-world operation, RustyCore runtime/server
activation and full extractor completion remain open.

The workflow's toolchain/registry fetch is a separate build prerequisite.
`--offline` controls Cargo dependency acquisition; it is not a claim that the
complete runner, test harness or PowerShell oracle was independently observed
without network activity. Directory write sharing, same-SID/administrator access
and the metadata/security checks retain their limitations above; no atomic
namespace, closed membership inventory or same-user isolation is established.

## Primary API references

- [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile): handle-relative names, explicit creation security and `FILE_CREATE` semantics.
- [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information): relative destination and no-replacement rename semantics.
- [SetSecurityDescriptorControl](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-setsecuritydescriptorcontrol): protected DACL control.
- [AddAccessAllowedAceEx](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-addaccessallowedaceex): explicit ALLOW ACE.
- [OpenThreadToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openthreadtoken): thread-token detection.
