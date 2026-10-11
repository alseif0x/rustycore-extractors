# Forever Rust synthetic CI

Authored 2026-10-11. The proposed workflow is
[forever-rust.yml](../.github/workflows/forever-rust.yml). It is a source change;
no hosted run or publication has been performed by adding it.

## Jobs and admission scope

| Job | Commands and scope |
| --- | --- |
| Linux, debug and release | Locked offline tests for the complete workspace (six product/component crates plus platform primitives) and all targets; builds component/product binaries; exercises the metadata inspector with authored inputs in each profile |
| Linux, debug | Workspace Clippy with warnings denied; selected source contract checks and the synthetic Python DB2 oracle/catalog suites |
| Windows, debug | Locked offline all-target tests for the complete workspace, including guarded Windows CASC/input readers and unsupported output guards; builds all component/product binaries and exercises the metadata inspector; the same selected Python checks |

The Windows job now builds and tests the entire workspace. The new read branches
use `forever-platform` leases in CASC admission/local-store reads and explicit
product input reading. Navigation's rooted filesystem adapters and product
publication remain Linux capabilities. The compiled Windows product still
returns `product_platform`; Windows DB2/GT/camera output entrypoints explicitly
return `output_platform` before output I/O. Tests exercise these refusals without
pretending that compilation enables complete Windows extraction.

The [read foundation contract](../contracts/forever-windows-read-contract-v1.json)
and [read consumer contract](../contracts/forever-windows-consumers-contract-v1.json)
record directory/full-versus-object snapshots, handle and chunk budgets, retained
key leases and output restrictions. The versioned foundation profile and its
separate API profile literal retain their distinct identities. Passing the job
cannot establish real Windows/NTFS, private DACL, reparse privilege, publication,
recovery or complete Windows distribution acceptance. External Wine checks are
preliminary evidence; no hosted or real Windows result is claimed here.

Global `cargo fmt --all -- --check` is currently pending: the existing
`forever-vmap/src/placement.rs` module order disagrees with rustfmt. This workflow
does not edit that unrelated existing file or silently rewrite source in CI.
A strict workspace formatting gate should be added after its owner resolves the
recorded difference. Clippy and both Linux test profiles remain strict.

The jobs do not enable `native`, `--all-features`, or `--ignored`. External corpus
and configured-reference tests retain their normal skips. The selected Python
checks are limited to standard-library dependencies; JSON Schema validator
suites, local distribution reconstruction, explicit product kill/recovery
harnesses and native/runtime comparisons retain their separate local gates.
The metadata CLI executable is explicitly built and addressed through
`FOREVER_DB2_INSPECT`; this CLI check is not an omitted-binary skip.

## Versioned tools and downloads

Rust is fixed to `1.98.0`, matching the recorded local distribution toolchain.
Python is selected from the `3.11` line. GitHub runner images remain the named
`ubuntu-24.04` and `windows-2022` images; their changing image contents are not a
reproducibility guarantee.

Only two external actions are used, pinned by complete commit ID:

- Existing checkout pin:
  [`actions/checkout@11d5960a326750d5838078e36cf38b85af677262`](https://github.com/actions/checkout/commit/11d5960a326750d5838078e36cf38b85af677262).
- Python setup:
  [`actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065`](https://github.com/actions/setup-python/commit/a26af69be951a213d495a4c3e4e4022e16d87065).

Both commit identities were checked against their official repositories. There
is no Rust setup action or additional Rust crate dependency. Rustup explicitly
installs the named toolchain and verifies distribution checksums. Each job
prints the build-only download plan, then performs `cargo fetch --locked` for
the exact Cargo.lock registry closure. Subsequent Cargo builds, linting and tests
use `--offline --locked`; no `pip install` is used. This is an explicit online
build preparation followed by offline Cargo execution, not a claim that hosted
CI has no network access. GitHub action/Python setup and toolchain provisioning
also require their normal build-service downloads.

## Data and execution boundaries

Triggers are pull requests into `forever`, pushes to `forever`, and explicit
manual dispatch. There is no privileged pull-request trigger, self-hosted runner
or client mount. Token permissions are `contents: read`; checkout does not
persist credentials. No secret input, repository publishing, artifact upload,
release step or persistent cache action is configured. Only checked-in source
and authored fixtures are inputs. Build outputs go into the runner's temporary
directory and ordinary Rust/Python synthetic temporaries remain on that runner.
The runner context is used only in step environments, as required by
[GitHub context availability](https://docs.github.com/en/actions/reference/workflows-and-actions/contexts#context-availability);
it is not referenced from the workflow-level environment. No real client, keys,
CASC data, TACT service, listfile or native reference is
requested by the workflow.

Windows run blocks explicitly inspect `$LASTEXITCODE` after every external
command, so a failed early command cannot be hidden by a successful later one.
Each job has a twenty-minute limit, and newer runs cancel older runs of the same
workflow/ref. No tests modify source formatting.

The workflow uses JSON syntax, a YAML-compatible representation, allowing its
boundary tests to parse the exact document with Python's standard `json` module.
[The focused contract suite](../tests/contracts/test_forever_ci_contract.py)
checks branch/event/permission boundaries, exact action and Rust pins, whole-workspace Windows compilation scope, offline
Cargo execution, Windows failure propagation, explicit platform scope and CLI
configuration. Mutation cases reject widened permissions, moving action tags,
unlocked/network-enabled Cargo tests, ignored corpus execution and uploads.
These source checks do not substitute for a successful GitHub run. CI passing
also does not prove real full extraction, C++ parity, RustyCore runtime acceptance,
Windows product functionality or goal completion.


## Navigation readers corresponding source v7

The mandatory source profile is `forever-seven-crate-windows-navigation-readers-source-v7`.
It adds guarded Windows terrain/inventory and serial memory-sink reader source and
its local navigation-to-platform dependency edge, retaining the exact nine-package
closure and registry checksums. Current-profile static tests preserve the v4/v5/v6
historical contract/evidence references. No Windows product or output publication
is enabled. Synthetic Wine semantics passed 112 cases; overall acceptance remains
false because 783 denied NETLINK attempts and 40 unresolved syscall bodies prevent
strict-zero and complete trace claims. The original trace-completeness claim is
rejected by root. See `forever-distribution.md` and the navigation reader contract
for immutable report hashes and actual-NTFS/runtime qualifications. These source
checks do not replace actual NTFS, real geometry or product/runtime acceptance.
