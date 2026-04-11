import { defineConfig } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { e2eBackendOrigin, e2eBaseUrl } from "./scripts/e2e-config.mjs";

const frontendRoot = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: e2eBaseUrl,
    headless: true,
    trace: "on-first-retry",
  },
  webServer: [
    {
      command: "node ./scripts/run-boopa-e2e-backend.mjs",
      cwd: frontendRoot,
      timeout: 120_000,
      reuseExistingServer: false,
      url: `${e2eBackendOrigin}/api/health`,
    },
    {
      command: "node ./scripts/run-vite-e2e.mjs",
      cwd: frontendRoot,
      timeout: 120_000,
      reuseExistingServer: false,
      url: e2eBaseUrl,
    },
  ],
});
