// SPDX-License-Identifier: GPL-3.0-or-later
// OpenCode V2 shell defaults for this project's shared and private servers.
import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const workRoot = resolve(projectRoot, "../rustycore-extractor-work");

export default {
  id: "rustycore.project-environment",
  async setup(ctx) {
    await ctx.shell.hook("create.before", (event) => {
      const env = event.env;
      const cargoBin = join(homedir(), ".cargo/bin");
      const path = env.PATH ?? process.env.PATH ?? "/usr/bin:/bin";
      env.PATH = [cargoBin, ...path.split(":").filter((p) => p && p !== cargoBin)].join(":");

      // Explicit caller environments and immutable acceptance commands keep
      // their selected paths. These defaults support ordinary development.
      env.CARGO_TARGET_DIR ??= join(workRoot, "builds/cargo-opencode");
      env.TMPDIR ??= join(homedir(), ".local/share/rustycore-extractor-work/tmp/opencode");
      env.PYTHONPYCACHEPREFIX ??= join(workRoot, "cache/python-opencode");
      env.CARGO_BUILD_JOBS ??= "4";
      for (const path of [env.CARGO_TARGET_DIR, env.TMPDIR, env.PYTHONPYCACHEPREFIX]) {
        mkdirSync(path, { recursive: true, mode: 0o700 });
      }
    });
  },
};
