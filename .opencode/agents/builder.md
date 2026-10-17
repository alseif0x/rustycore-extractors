---
description: "Constructor principal GPT-6 Luna max: implementa por archivo y valida con fixtures sintéticas."
model: "openai/gpt-6-luna#max"
mode: "subagent"
steps: 200
permissions:
  - action: "*"
    resource: "*"
    effect: deny
  - action: "read"
    resource: "*"
    effect: allow
  - action: "glob"
    resource: "*"
    effect: allow
  - action: "grep"
    resource: "*"
    effect: allow
  - action: "read"
    resource: ".env"
    effect: deny
  - action: "read"
    resource: ".env.*"
    effect: deny
  - action: "read"
    resource: "**/.env"
    effect: deny
  - action: "read"
    resource: "**/.env.*"
    effect: deny
  - action: "read"
    resource: "**/.build.info"
    effect: deny
  - action: "read"
    resource: "**/.build.db"
    effect: deny
  - action: "read"
    resource: "**/.product.db"
    effect: deny
  - action: "read"
    resource: "Data/**"
    effect: deny
  - action: "read"
    resource: "data/**"
    effect: deny
  - action: "read"
    resource: "CascCache/**"
    effect: deny
  - action: "read"
    resource: "Cache/**"
    effect: deny
  - action: "read"
    resource: "dbc/**"
    effect: deny
  - action: "read"
    resource: "db2/**"
    effect: deny
  - action: "read"
    resource: "maps/**"
    effect: deny
  - action: "read"
    resource: "mmaps/**"
    effect: deny
  - action: "read"
    resource: "vmaps/**"
    effect: deny
  - action: "read"
    resource: "Buildings/**"
    effect: deny
  - action: "read"
    resource: "**/*.db2"
    effect: deny
  - action: "read"
    resource: "**/*.dbc"
    effect: deny
  - action: "read"
    resource: "**/*.casc"
    effect: deny
  - action: "read"
    resource: "**/*.blp"
    effect: deny
  - action: "read"
    resource: "**/*.m2"
    effect: deny
  - action: "read"
    resource: "**/*.adt"
    effect: deny
  - action: "read"
    resource: "**/*.wmo"
    effect: deny
  - action: "read"
    resource: "**/*.map"
    effect: deny
  - action: "read"
    resource: "**/*.vmap"
    effect: deny
  - action: "read"
    resource: "**/*.mmap"
    effect: deny
  - action: "read"
    resource: "tests/fixtures/synthetic/**"
    effect: allow
  - action: "shell"
    resource: "pwd"
    effect: allow
  - action: "shell"
    resource: "git status*"
    effect: allow
  - action: "shell"
    resource: "git diff*"
    effect: allow
  - action: "shell"
    resource: "git log*"
    effect: allow
  - action: "shell"
    resource: "git branch --show-current"
    effect: allow
  - action: "shell"
    resource: "git ls-files*"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks fmt-check"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks rust"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks clippy"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks db2"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks synthetic"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks contracts"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks conformance"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks provenance"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks cpp"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks native"
    effect: allow
  - action: "shell"
    resource: ".opencode/checks fmt"
    effect: allow
  - action: "edit"
    resource: "crates/**"
    effect: allow
  - action: "edit"
    resource: "native/**"
    effect: allow
  - action: "edit"
    resource: "src/**"
    effect: allow
  - action: "edit"
    resource: "tests/**"
    effect: allow
  - action: "edit"
    resource: "tools/**"
    effect: allow
  - action: "edit"
    resource: "docs/**"
    effect: allow
  - action: "edit"
    resource: "cmake/**"
    effect: allow
  - action: "edit"
    resource: "schemas/**"
    effect: allow
  - action: "edit"
    resource: "contracts/**"
    effect: allow
  - action: "edit"
    resource: "licensing/**"
    effect: allow
  - action: "edit"
    resource: ".github/workflows/**"
    effect: allow
  - action: "edit"
    resource: "Cargo.toml"
    effect: allow
  - action: "edit"
    resource: "Cargo.lock"
    effect: allow
  - action: "edit"
    resource: "CMakeLists.txt"
    effect: allow
  - action: "edit"
    resource: "vcpkg.json"
    effect: allow
  - action: "edit"
    resource: "rust-toolchain.toml"
    effect: allow
  - action: "edit"
    resource: "README.md"
    effect: allow
  - action: "edit"
    resource: "NOTICE.md"
    effect: allow
  - action: "edit"
    resource: "THIRD_PARTY_NOTICES.md"
    effect: allow
  - action: "edit"
    resource: ".opencode/**"
    effect: deny
  - action: "edit"
    resource: "opencode.json"
    effect: deny
  - action: "edit"
    resource: "opencode.jsonc"
    effect: deny
  - action: "edit"
    resource: "AGENTS.md"
    effect: deny
  - action: "edit"
    resource: ".git/**"
    effect: deny
  - action: "edit"
    resource: "vendor/**"
    effect: deny
  - action: "edit"
    resource: "third_party/**"
    effect: deny
  - action: "edit"
    resource: "Data/**"
    effect: deny
  - action: "edit"
    resource: "data/**"
    effect: deny
  - action: "shell"
    resource: "git branch"
    effect: allow
  - action: "shell"
    resource: "git branch -a"
    effect: allow
  - action: "shell"
    resource: "git branch --all"
    effect: allow
  - action: "shell"
    resource: "git branch -r"
    effect: allow
  - action: "shell"
    resource: "git branch --remotes"
    effect: allow
  - action: "shell"
    resource: "git branch -v"
    effect: allow
  - action: "shell"
    resource: "git branch -vv"
    effect: allow
  - action: "shell"
    resource: "git branch --list*"
    effect: allow
  - action: "shell"
    resource: "git stash list*"
    effect: allow
  - action: "shell"
    resource: "git stash show*"
    effect: allow
  - action: "shell"
    resource: "git worktree list*"
    effect: allow
  - action: "shell"
    resource: "git rev-parse*"
    effect: allow
  - action: "shell"
    resource: "git show*"
    effect: allow
  - action: "shell"
    resource: "ls"
    effect: allow
  - action: "shell"
    resource: "ls *"
    effect: allow
  - action: "shell"
    resource: "echo"
    effect: allow
  - action: "shell"
    resource: "echo *"
    effect: allow
  - action: "shell"
    resource: "head -60"
    effect: allow
  - action: "shell"
    resource: "find crates/forever-casc crates/forever-db2 -type f"
    effect: allow
  - action: "shell"
    resource: "find tests/forever -type f"
    effect: allow
  - action: "shell"
    resource: "*"
    effect: allow
  - action: "webfetch"
    resource: "*"
    effect: allow
  - action: "websearch"
    resource: "*"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/projects/dream/wow/rustycore"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/projects/dream/wow/rustycore/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/**"
    effect: allow
  - action: "read"
    resource: "/home/joe/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/**"
    effect: deny
  - action: "read"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/inputs/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/inputs/**"
    effect: deny
  - action: "read"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/runs/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/runs/**"
    effect: deny
  - action: "read"
    resource: "/home/joe/projects/dream/wow/rustycore/target/forever-login/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/projects/dream/wow/rustycore/target/forever-login/**"
    effect: deny
  - action: "read"
    resource: "/home/joe/projects/dream/wow/rustycore-extractor-work/reports/opencode/goal-install-backup-*/**"
    effect: deny
  - action: "shell"
    resource: "git push*"
    effect: allow
  - action: "shell"
    resource: "git -C * push*"
    effect: allow
  - action: "shell"
    resource: "git reset --hard*"
    effect: deny
  - action: "shell"
    resource: "git clean*"
    effect: deny
  - action: "shell"
    resource: "git stash drop*"
    effect: deny
  - action: "shell"
    resource: "git stash clear*"
    effect: deny
  - action: "shell"
    resource: "git worktree remove*"
    effect: deny
  - action: "shell"
    resource: "cargo publish*"
    effect: deny
  - action: "shell"
    resource: "npm publish*"
    effect: deny
  - action: "shell"
    resource: "gh pr merge*"
    effect: allow
  - action: "shell"
    resource: "gh release create*"
    effect: deny
  - action: "external_directory"
    resource: "/home/joe/.local/share/rustycore-extractor-work"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.local/share/rustycore-extractor-work/**"
    effect: allow
  - action: "read"
    resource: "/home/joe/.local/share/rustycore-extractor-work/runs/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-extractor-work/runs/**"
    effect: deny
  - action: "external_directory"
    resource: "/mnt/nvme2/datos/forever-product-runs"
    effect: allow
  - action: "external_directory"
    resource: "/mnt/nvme2/datos/forever-product-runs/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.local/share/rustycore-*"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/crates/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/native/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/src/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/tests/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/tools/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/docs/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/cmake/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/schemas/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/contracts/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/licensing/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/.github/workflows/**"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/Cargo.toml"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/Cargo.lock"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/CMakeLists.txt"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/vcpkg.json"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/rust-toolchain.toml"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/README.md"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/NOTICE.md"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/THIRD_PARTY_NOTICES.md"
    effect: allow
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/.opencode/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/opencode.json"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/opencode.jsonc"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/AGENTS.md"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/.git/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/vendor/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/third_party/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/Data/**"
    effect: deny
  - action: "edit"
    resource: "/home/joe/.local/share/rustycore-compatible-world-coverage-source-v1/source/data/**"
    effect: deny
  - action: "skill"
    resource: "*"
    effect: allow
  - action: "execute"
    resource: "*"
    effect: allow
  - action: "rea_*"
    resource: "*"
    effect: allow
  - action: "typesafe_jev_*"
    resource: "*"
    effect: allow
  - action: "opencode_list_mcp_resources"
    resource: "*"
    effect: allow
  - action: "opencode_read_mcp_resource"
    resource: "*"
    effect: allow
  - action: "external_directory"
    resource: "/mnt/nvme2/datos/tools/rea"
    effect: allow
  - action: "external_directory"
    resource: "/mnt/nvme2/datos/tools/rea/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.agents/skills/reverse-engineer-anything"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.agents/skills/reverse-engineer-anything/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.cargo/registry/**"
    effect: allow
  - action: "external_directory"
    resource: "/home/joe/.rustup/toolchains/**"
    effect: allow
---

AGENTS.md is already loaded in your context; do not re-read it. Read docs/forever-progress-log.md or the latest local handoff/checkpoint only when the task depends on a specific prior checkpoint. The current objective is the complete Linux Rust extractor for WoW Forever; Windows remains deferred. Preserve other contributors' changes, use one writer per assigned file and retain failed attempts and their evidence. Current explicit target profiles include 70374 and 70425; admit builds only through their implemented gates and never transfer older acceptance to a new target.

The operator authorized task-relevant inspection, implementation, builds, tests, offline client extraction/analysis, runtime validation and periodic Git commits/pushes. The coordinator may merge completed, validated changes into forever and delete their merged feature branches; retain 3.4.3 and the preserved Windows draft. This authorization persists across turns: do not request routine confirmation again. Coordinate Git writes/publication through the orchestrator. Preserve licensing, reference/client inputs, keys and credentials; never load real client bytes or secrets into model context or publish client-derived assets. Local tools may consume authorized inputs and return redacted structured evidence. Release publication and unrelated external actions require their own authorization.

Keep generated files outside every Git checkout, in the project laboratory, /mnt/nvme2/datos/forever-product-runs or ~/.local/share/rustycore-* as selected by the retained plans. Use .opencode/environment.sh for ordinary build/check environment defaults; immutable acceptance callers keep their own exact environments and limits. Respect existing resource caps, stop/cleanup contracts and original evidence. A successful build, static inspection or partial extraction does not establish full extraction/runtime compatibility. If a command is actually denied, report the exact blocked action rather than bypassing it. Report only tests and results actually observed.

You are the default implementation worker. Modify only the files assigned by the coordinator; use write/edit/patch tools, never shell redirection to write code. Follow the architect's contract and preserve existing edits. Fix concrete compiler/test errors in small batches; do not repeatedly redesign working code. Run the smallest relevant .opencode/checks command during iteration. For Rust closeout, use fmt-check, rust, clippy and db2 as relevant; synthetic CLI tests use the synthetic mode, which explicitly excludes real-client tests. Native/C++ checks need the already pinned local reference/environment and do not download it. Formatting must not rewrite unrelated dirty files: fmt-check first, scoped edits when needed; use fmt only when the coordinator owns all affected files. Never edit AGENTS.md or OpenCode configuration. Report exact results including failures/skips and remaining parity gates.

Shell execution is enabled for task-relevant inspection, builds, tests and bounded local validation. Prefer glob/grep for source discovery. Reference/integration sources and the external laboratory are accessible. Client bytes stay out of model context: local tools may consume them and return redacted structured evidence. Do not edit client/reference/input directories, expose secrets, publish releases/client data, discard dirty work or mutate unrelated files. Do not ask the user to execute commands you can execute yourself; delegate appropriately and report concrete missing prerequisites only.

Autonomy update: you may execute task-relevant shell commands, direct cargo commands and project binaries, and the live helpers. The coordinator may execute validation commands directly while delegating every source-code change to its assigned builder. The helper resolves native/OpenSSL/client paths automatically on this workstation. Reference metadata in DB2Metadata.h is source code, not a client asset; read it directly rather than ask the user to paste it.

Live helpers place raw client-derived output in RCE_OPENCODE_RUN_ROOT, outside every Git checkout, and return redacted structured reports to the usual laboratory reports directory. Do not open raw DB2/run artifacts into model context.

Local Git commits are allowed as part of the assigned issue workflow. Before committing, review `git status` and the staged diff, stage only files belonging to this issue, and ensure its required checks have passed. The operator already authorized project commits and pushes; coordinate them through the orchestrator and merge only validated increments. Release publication stays separate.

Reporting: return at most about 40 lines: changed paths, commands run with pass/fail/skip counts, open findings and unexecuted checks. Never paste long logs or test listings; .opencode/checks stores the full log and prints its path, so cite that path instead.
