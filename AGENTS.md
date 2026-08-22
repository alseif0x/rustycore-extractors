# AGENTS.md

Shared operating guide for AI agents and human contributors working in
`rustycore-extractors`. Keep it factual and update it when the repository contract changes.

## Project and source of truth

`rustycore-extractors` produces standalone, versioned client-data extraction tools for
RustyCore. The first delivery isolates the proven C++ tools; the second replaces them
incrementally with Rust behind the same compatibility contract.

- Repository checkout: `/home/server/rustycore-extractors`
- Private remote: `git@github.com:alseif0x/rustycore-extractors.git`
- Integration repository: `/home/server/rustycore`
- C++ reference repository: `/home/server/woltk-trinity-legacy`
- Fixed initial upstream: `TrinityCoreLegacy/TrinityCore`, branch `3.4.3`, commit
  `92796557f9b0ba3d2d1c7c770f535153154cf83e`
- Plan/index: `https://github.com/alseif0x/rustycore-extractors/issues/13`
- Default integration branch: `main`

Do not trust an extractor because it builds or finishes. TrinityCoreLegacy output plus
RustyCore reader/runtime acceptance is the compatibility authority.

## Fixed v0.1 target

- Product: `wow_classic`
- Client version/build: `3.4.3.51943`
- Map format: `MAPS`, version `10`
- VMap format: `VMAP_4.B` (`VMAP04B` raw intermediate)
- MMap format: `MMAP`, version `15`, with the exact supported Detour version

Other products, builds and expansions are out of scope until the v1.0 plan is complete.

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
2. Create the branch linked to the issue with `gh issue develop <N> --base main`.
3. One issue = one branch = one focused PR into `main` with `Closes #<N>`.
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
