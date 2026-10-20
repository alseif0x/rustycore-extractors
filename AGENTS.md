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
`forever-seven-crate-linux-product-db2-gt-cameras-selected-navigation-consumer-source-v13`;
its explicit v23/shared-consumer source/test/contract/document closure is mandatory.
V13 has bounded component/real-worker acceptance and matching fresh/offline-rebuild
archives, 47 packaged CLI cases and a real nine-payload parity run. RustyCore publication and
server/full-world acceptance remain open. V12 retains its frozen inputs.
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

## OpenCode orchestration on Joe's workstation (2026-10-04)

The local checkout is `/home/joe/projects/dream/wow/rustycore-extractors`.
The `/home/server/...` paths above describe the reference-server layout; they are
not local paths to invent or automatically remap. Workstation generated files
belong in `/home/joe/projects/dream/wow/rustycore-extractor-work/`.

The project-local OpenCode configuration is `opencode.json`, with profiles in
`.opencode/agents/`. Start through `.opencode/start` so a private server inherits
the external Cargo build and temporary paths. See `docs/opencode-orchestration.md`.

- `orchestrator`: DeepSeek V4.1 Flash; plans and delegates, never implements.
- `general`: DeepSeek V4 Pro; architecture and compatibility contracts, read-only.
- `scout`: DeepSeek V4.1 Flash; focused source discovery, read-only.
- `builder`: GPT-6 Luna max (OpenAI/Codex account); implementation worker, owns assigned files.
- `explore`: DeepSeek V4.1 Flash high; independent audit and checks, never implements.
- `local-coder`: disabled (its Strata provider was removed).

DeepSeek agents use OpenCode Go by default. When the Go quota is exhausted the
operator may switch them to the direct DeepSeek API with
`.opencode/model-route api` (and back with `go`); no other automatic fallback.
Cloud-agent input is restricted to source code, synthetic fixtures and reviewed
redacted reports. Never place real client assets, keys or secret environment
values in model context. Starting a session does not automatically start extraction. Explicit bounded live acceptance for the already authorized 70170/70205 task may run through the configured helpers.

Before any task, record the current dirty files, assign non-overlapping ownership,
and preserve existing work. Do not change the active branch merely to initialize
these profiles. Use `.opencode/checks` for bounded validation; native checks require
an explicitly configured, already pinned local reference. Formatting unrelated
dirty files is prohibited. An explicit permission rejection is final for that
attempt: report it rather than seek another command to bypass it.

Persistent goal mode is installed globally through OpenCode Goal Plugin 0.1.64.
The orchestrator owns its lifecycle tools and must create/replace goals only
when explicitly requested by the user. Preserve the goal across delegation;
wait for active children, then require concrete acceptance evidence before
completion. Honor pause/stop and retain every project/data/license boundary.

### Authorized agent autonomy and local acceptance

The operator explicitly authorized the agents to run the task circuit and the
commands required for inspection, compilation, testing and local acceptance.
All profiles now permit task-relevant shell execution and scoped external
reference/laboratory access. The coordinator may validate directly and still
delegates implementation; do not ask the operator to execute available
commands or paste reference metadata. This does not authorize publishing,
client-data exposure, changing the task scope or discarding other work.

The helpers automatically select the pinned local reference and existing
OpenSSL development headers. `checks live` enables the local CLI tests;
`checks live-extract` executes the bounded two-table default plan offline and
returns only redacted structured evidence. Neither is full extraction.
Raw real-client outputs and live temporaries use RCE_OPENCODE_RUN_ROOT under
`/home/joe/.local/share/rustycore-extractor-work/runs/candidate`, because the
neighboring laboratory is itself beneath the enclosing wow Git repository.
Retain previous outputs; reports remain in the usual external laboratory.
Read client data only inside local tools, never into model context.
