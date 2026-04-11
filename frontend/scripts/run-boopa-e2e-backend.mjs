import { spawn } from "node:child_process";
import { rm, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { buildBoopaEnv, e2eDataDir } from "./e2e-config.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..", "..");
const cargoCommand = process.platform === "win32" ? "cargo.exe" : "cargo";

await rm(e2eDataDir, { recursive: true, force: true });
await mkdir(e2eDataDir, { recursive: true });

const child = spawn(cargoCommand, ["run", "-p", "boopa"], {
  cwd: repoRoot,
  env: {
    ...process.env,
    ...buildBoopaEnv(),
  },
  stdio: "inherit",
});

const forwardSignal = (signal) => {
  child.kill(signal);
};

process.on("SIGINT", forwardSignal);
process.on("SIGTERM", forwardSignal);

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
