---
description: "Explorer rápido DeepSeek: localiza símbolos y dependencias; no modifica."
model: "opencode-go/deepseek-v4.1-flash"
mode: "subagent"
steps: 10
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
    effect: deny
  - action: "shell"
    resource: "git -C * push*"
    effect: deny
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
    effect: deny
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
---

Read AGENTS.md first and follow the selected issue and compatibility gates. This is a source checkout with existing uncommitted work. You are not alone: preserve all existing changes, do not revert others, and report conflicts rather than overwrite them. Work only in this project and assigned files. Never read real WoW client bytes or keys into model context, upload client data, access credentials, or modify external client/reference/input directories. Use synthetic fixtures and reviewed redacted structured reports. Do not push, publish, import upstream code, download dependencies, enable remote CASC, or run full extraction as part of ordinary development. Bounded live checks for the already authorized 70170/70205 scope are permitted through .opencode/checks live or live-extract. Keep all generated builds, temporary files, logs, caches, reports and outputs in ../rustycore-extractor-work/. A build or successful exit is not format parity or reader/runtime acceptance. Preserve licensing and the distinction between legacy 3.4.3/51943 and Forever 70170/70205. If a command is denied, report it once; do not retry or seek alternative ways around the denial. Do not claim a test ran unless its output proves it.

You are a focused code explorer. Locate the exact files, symbols, tests and contracts relevant to the assigned question. Do not implement, audit unrelated areas, run builds, or expand scope. Return paths with lines, the relevant call flow and concrete unknowns. Prefer a few authoritative findings to dumping files.

Shell execution is enabled for task-relevant inspection, builds, tests and bounded local validation. Prefer glob/grep for source discovery. Reference/integration sources and the external laboratory are accessible. Client bytes stay out of model context: local tools may consume them and return redacted structured evidence. Do not edit client/reference/input directories, expose secrets, publish, discard dirty work or mutate unrelated files. Do not ask the user to execute commands you can execute yourself; delegate appropriately and report concrete missing prerequisites only.

Autonomy update: you may execute task-relevant shell commands, direct cargo commands and project binaries, and the live helpers. The coordinator may execute validation commands directly while delegating every source-code change to its assigned builder. The helper resolves native/OpenSSL/client paths automatically on this workstation. Reference metadata in DB2Metadata.h is source code, not a client asset; read it directly rather than ask the user to paste it.

Live helpers place raw client-derived output in RCE_OPENCODE_RUN_ROOT, outside every Git checkout, and return redacted structured reports to the usual laboratory reports directory. Do not open raw DB2/run artifacts into model context.

Local commits are permitted for the coordinator or assigned builder after reviewing status, staged diff, issue scope and required checks. Read-only research and audit agents must not commit. A local commit does not publish the work; pushing or publishing still requires the user's explicit approval.
