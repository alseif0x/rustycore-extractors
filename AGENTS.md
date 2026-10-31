# AGENTS.md

Shared operating guide for AI agents and human contributors working in
`rustycore-extractors`. Keep it factual and update it when the repository contract changes.

## Project and source of truth

`rustycore-extractors` produces standalone, versioned client-data extraction tools for
RustyCore. The first delivery isolates the proven C++ tools; the second replaces them
incrementally with Rust behind the same compatibility contract.

- Repository checkout: `/home/server/rustycore-extractors`
- Public remote: `https://github.com/alseif0x/rustycore-extractors.git`
- Integration repository: `/home/server/rustycore`
- C++ reference repository: `/home/server/woltk-trinity-legacy`
- Fixed initial upstream: `TrinityCoreLegacy/TrinityCore`, branch `3.4.3`, commit
  `92796557f9b0ba3d2d1c7c770f535153154cf83e`
- Plan/index: `https://github.com/alseif0x/rustycore-extractors/issues/13`
- Default development branch: `forever`; legacy version line: `3.4.3`

### Current delivery priority — 2026-10-13

Complete the Linux extractor first: real acquisition, complete component outputs,
full-world resources, runtime acceptance and a reproducible Linux package.
Windows development and distribution are deferred until that Linux delivery is
complete. PR #27 remains a preserved, unaccepted draft; existing platform code
and historical evidence retain their recorded scopes.

Use one focused active implementation branch. Merge each completed and validated
implementation into `forever` and delete its merged local/remote branch.
GitHub automatically deletes merged feature branches. Preserve uncommitted work
and pending implementations; retain the `3.4.3` version line.

## Progress and evidence log

Dated implementation checkpoints, acceptance evidence, receipt hashes and
retained failures live in `docs/forever-progress-log.md` (moved verbatim from
this file on 2026-10-17) and in the component documents under `docs/`.
Read them when a task depends on a specific prior checkpoint; they are not
needed for every task. Add new dated entries there, not here: this file is
injected into every agent request, so keep it to stable rules and the current
priority.

Detailed key-source investigation, REA operation, client internals and repeat
procedures belong only in the operator’s external private laboratory.
`docs/forever-key-investigation.md` carries only a general public status.
Do not publish investigation details, client data or key material in Git or issues.

Forever summary: the operator made this repository public, renamed `main` to
`3.4.3`, and selected `forever` as default on 2026-10-03. Forever's development
target is modern WoW `1.60.1` Beta x64. Admit each build only through its own
implemented, versioned gates; never transfer older-build acceptance to a new
target. Full Rust extraction, complete real geometry/world resources, RustyCore
runtime acceptance and the reproducible Linux distribution remain open goals.

## Local Forever distribution boundary

The versioned `forever-distribution-v1` recipe produces local Linux x86_64
candidates only. It builds the frozen shipped source offline with Rust 1.98.0,
including current uncommitted product code; HEAD-only archives are insufficient.
Only the two already admitted exact registry packages may be vendored, with
their archives, checksums, original sources and all notices. Use new external
output directories, retain failed/successful runs, and never package client
assets, keys, extraction outputs, caches, secrets or native reference trees.
Archive integrity and reproducibility do not establish real extraction,
runtime acceptance, Windows support or release authorization. Windows needs
a versioned filesystem backend/acceptance and any new native dependency's
recorded license closure. See `docs/forever-distribution.md` and
`contracts/forever-distribution-contract-v1.json`.
The candidate must also retain `THIRD_PARTY_NOTICES.md` and the exact pinned
Rust standard-library copyright index/license texts under toolchain-notices;
these are notice documents, without a claim that every listed component links.
The current source profile is
`forever-seven-crate-linux-product-db2-gt-cameras-available-navigation57-consumer-source-v15`;
its explicit v23/v56/v57 producer/consumer source, test, contract and redacted-evidence closure is mandatory.
V15 has available real v57 extraction, 69 planned-map native queries, matching fresh/offline-rebuild
archives and packaged real `inspect`/`verify`/`verify-links` acceptance. Its frozen archive
retains earlier pending-status prose. V14 retains its frozen inputs and scoped v56 acceptance.
V13 retains its frozen inputs, matching fresh/offline-rebuild archives, 47 packaged CLI cases and real nine-payload parity run.
No profile establishes full server/full-world acceptance. V12 retains its frozen inputs.
The build
environment fixes one Cargo job. Qualification belongs to each frozen source
snapshot, archive and executable; recorded earlier test counts and artifact
comparisons retain their original inputs and scopes. Current qualification and
retained failures live in `docs/forever-distribution.md` and its structured
reports. Windows, full-world and real complete geometry/runtime acceptance
stay open. No publication is authorized.

## Fixed v0.1 target

- Product: `wow_classic`
- Client version/build: `3.4.3.51943`
- Map format: `MAPS`, version `10`
- VMap format: `VMAP_4.B` (`VMAP04B` raw intermediate)
- MMap format: `MMAP`, version `15`, with the exact supported Detour version

On the legacy `3.4.3` line, other products/builds remain outside this fixed contract.
The separately authorized Forever target does not change that legacy contract.

## Current-server paths and data boundary

The following inputs are **read-only references**. Do not edit, clean, move, patch, stage,
or write generated directories beneath them:

- WoW client: `/home/server/wow-client/World of Warcraft`
- C++ reference: `/home/server/woltk-trinity-legacy`
- Existing extracted reference: `/home/server/woltk-server-core/Data`

All builds, downloads, temporary files, logs, comparisons and generated outputs belong
outside the Git checkout under:

```text
/home/server/rustycore-extractor-work/
  artifacts/
  builds/
  cache/tact/
  inputs/                 # read-only symlinks to canonical inputs
  logs/
  reports/
  runs/reference/
  runs/candidate/
  worktrees/
```

Never run a legacy script whose cleanup/output behavior targets the client directory.
Invoke tools with explicit input/output paths or from an isolated run directory. The legacy
MMap generator uses its current working directory, so run it only inside its run directory.

## Copyright and licensing gate

- Project license target: GPL-3.0-or-later.
- Preserve all TrinityCore copyrights and GPL-2.0-or-later headers.
- Keep MIT, Zlib, BSD and other third-party notices with their source.
- Do not import upstream code until issue #1 records the exact dependency/license closure.
- Mark derived files with a prominent modification notice and date.
- Never claim ownership of upstream work or remove an upstream author's notice.

No Blizzard client file or client-derived `dbc`, DB2, map, VMap, MMap, model, camera, GT,
CASC cache or intermediate output may be committed, uploaded to CI, or attached to a release.
Only synthetic fixtures and hash/structured reports are allowed in Git.

## Network policy

- Local CASC and offline operation are the default.
- No hidden TACT-key, listfile, CDN or remote-CASC request.
- Network access requires an explicit option, a pinned source and checksum, and a displayed
  download plan.
- Tools never upload client paths or data.

## Working an issue

Follow the numbered order in issue #13 and respect dependency gates.

1. Read this file, the index and the selected issue.
2. Create issue branches from the version line: `forever` for Forever, `3.4.3` for legacy work.
3. One issue = one branch = one focused PR into that version line with `Closes #<N>`.
4. Compare with the fixed C++ reference before changing behavior or formats.
5. Add focused positive/negative tests and structured diagnostic output.
6. Run the smallest relevant local check during iteration.
7. Run the issue's compatibility acceptance before push.
8. Do not push, publish a release or make the repository public without explicit approval.

PR checks stay lightweight: formatting, synthetic tests and platform builds. Real-client full
extraction is local evidence because its inputs and outputs cannot be placed in CI.

## Long-running work

Do not attach a full extraction or MMap generation to an AI-agent process. Use a persistent
user service or `tmux`, write logs and manifests inside the run directory, and record the exact
command. Example shape:

```bash
systemd-run --user \
  --unit rustycore-extract-<run-id> \
  --working-directory /home/server/rustycore-extractor-work/runs/candidate/<run-id> \
  --collect \
  <command>
```

Never delete an older successful run until its report identifies it and another verified run
supersedes it. Clean only explicit tool-owned directories.

## Validation policy

- Fast iteration uses synthetic inputs or one representative map/tile.
- The compatibility harness compares file sets, parsed semantics and deterministic bytes.
- A successful process exit is not a parity claim.
- Full build-51943 extraction is reserved for issue closeout and release candidates.
- Existing `/home/server/woltk-server-core/Data` is convenient reference material, not final
  proof until its producing extractor version and options are recorded.

## Kimi Code orchestration on Joe's workstation

The local checkout is `/home/joe/projects/dream/wow/rustycore-extractors`.
The `/home/server/...` paths above describe the reference-server layout; they are
not local paths to invent or automatically remap. Workstation generated files
belong in `/home/joe/projects/dream/wow/rustycore-extractor-work/`.

The user-level Kimi model pool is available in this checkout without storing
provider credentials in the repository. K3.1 Pro High performs all implementation,
validation and closeout directly with the operator; no subagents or orchestrated
workers are used. There is one writer for the active version and no project
launcher; source `tools/environment.sh` and use `tools/checks` for bounded
validation.

Use one active product version and one bounded real failure per development
session. Before implementation, record the dirty files, preserve existing work,
assign ownership and write down the scope, focused checks, real acceptance gate
and closeout requirements. Do not change the active branch merely to initialize
Kimi. Native checks require an explicitly configured, already pinned local
reference. Formatting unrelated dirty files is prohibited. An explicit
permission rejection is final for that attempt: report it rather than seek
another command to bypass it.

The development circuit is mandatory:

1. Implement the scoped change with focused tests. Use `tools/checks test`
   for Cargo test selection and keep iteration out of the full workspace suite.
2. Run the authorized bounded real attempt as soon as the focused checks pass.
   Record its actual result, including a new blocker; synthetic success is not
   real acceptance.
3. Correct only failures demonstrated within the current scope. Adjacent cases
   belong to a later version, not to an expanding implementation or review.
4. When focused and real-attempt evidence is complete, run
   `tools/checks closeout` once. Any later code change invalidates that run.
5. Review the final diff directly. Only demonstrated correctness, security,
   data-integrity or false-acceptance defects reopen the circuit; record minor
   observations as follow-ups without expanding scope.
6. Integrate the validated increment under the existing Git authorization and
   record changed files, checks, real result, omissions and the next concrete
   blocker. Start the next version with a fresh session.

If the same real blocker shows no new real progress after two complete
correction cycles, stop the autonomous circuit and report the evidence,
attempted fixes and remaining decision to the operator. Do not continue with
more versions, reviews or autonomous goal turns until the operator chooses the
next scope. A new diagnostic or a later real failure is progress; another
synthetic-only pass is not.

Do not start the next version until the current real attempt or its blocker is
documented, closeout is complete and integration is done. Continue directly on
only the current defect, not across a series of product versions.

Cloud-agent input is restricted to source code, synthetic fixtures and reviewed
redacted reports. Never place real client assets, keys, secret environment values
or unredacted laboratory data in model context. Starting a session does not
automatically start extraction. Only explicitly authorized bounded live
acceptance for the selected build/profile may run through configured helpers.

### Authorized agent autonomy and local acceptance

The operator explicitly authorized the agents to run the task circuit and the
commands required for inspection, compilation, testing and local acceptance.
The coordinator may validate directly and delegates substantial implementation;
do not ask the operator to execute available commands or paste reference
metadata. This does not authorize publishing, client-data exposure, changing
the task scope or discarding other work.

The helpers automatically select the pinned local reference and existing OpenSSL
development headers. `tools/checks live` enables the local CLI tests;
`tools/checks live-extract` executes the bounded two-table default plan offline
and returns only redacted structured evidence. Neither is full extraction.
Raw real-client outputs and live temporaries use `RCE_KIMI_RUN_ROOT` under
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate`, because the
neighboring laboratory is itself beneath the enclosing wow Git repository.
Retain previous outputs; reports remain in the usual external laboratory.
Read client data only inside local tools, never into model context.
