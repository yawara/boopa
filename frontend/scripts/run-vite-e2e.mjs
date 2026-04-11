import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { buildViteEnv, e2eVitePort } from "./e2e-config.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const frontendRoot = path.resolve(scriptDir, "..");
const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";

const child = spawn(
  npmCommand,
  ["run", "dev", "--", "--host", "127.0.0.1", "--port", String(e2eVitePort)],
  {
    cwd: frontendRoot,
    env: {
      ...process.env,
      ...buildViteEnv(),
    },
    stdio: "inherit",
  },
);

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
