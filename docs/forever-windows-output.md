# Windows private-output foundation v1

Authored 2026-10-12. This is an original GPL-3.0-or-later library foundation in
`forever-platform`, separate from the Windows read and loaded-producer profiles.
Native Windows/NTFS execution acceptance is pending. Source contract checks and
the planned CI commands are not execution evidence.

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
Its Python checks compare source, API limits and planned workflow boundaries;
they do not execute Windows calls. Native tests require an actual Windows x86_64
process and a fixed local NTFS temporary directory. Wine and GNU-Windows cross
compilation cannot establish this gate.

The Windows job in `.github/workflows/forever-rust.yml` retains its workspace
debug checks and adds focused private-output debug/release execution, with
locked/offline Cargo and exact filtered-test inventory/result checks: fourteen
ordinary `windows::output::tests::` cases per profile, with zero ignored cases.
The ordinary inventory includes an independent PowerShell `Get-Acl` /
`SecurityIdentifier` oracle for the directory/file owner and protected single
explicit, non-inheriting FullControl ALLOW ACE. Its child stdout/stderr are
suppressed; no SID or fixture path is printed. A separate ordinary case uses
`ImpersonateSelf` with RAII `RevertToSelf` to check rejection before creation
and permanent poisoning of an existing root. These are planned executable
checks, with native results still pending.

The separate `windows::output::privileged_tests::` reparse case remains ignored
by default and needs explicit native Windows symlink privilege/NTFS execution;
ordinary CI does not close that privileged gate. Each focused step logs the
Windows version and successful fixed-NTFS admission without paths or SIDs.
The job's toolchain/registry
fetch is a separate build prerequisite. `--offline` controls
Cargo dependency acquisition; it is not a claim that the complete runner or
test harness has been independently observed without network activity.

No native acceptance report or hosted-run result is claimed at this checkpoint.
Execution evidence must identify the frozen sources, runner/filesystem, exact
commands, selected test count and terminal result before this gate is closed.

## Primary API references

- [NtCreateFile](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntcreatefile): handle-relative names, explicit creation security and `FILE_CREATE` semantics.
- [FILE_RENAME_INFORMATION](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information): relative destination and no-replacement rename semantics.
- [SetSecurityDescriptorControl](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-setsecuritydescriptorcontrol): protected DACL control.
- [AddAccessAllowedAceEx](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-addaccessallowedaceex): explicit ALLOW ACE.
- [OpenThreadToken](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-openthreadtoken): thread-token detection.
