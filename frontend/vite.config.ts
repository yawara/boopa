import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import { loadEnv } from "vite";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const backendTarget = env.BOOPA_DEV_BACKEND ?? "http://127.0.0.1:8080";

  return {
    plugins: [react()],
    server: {
      proxy: {
        "/api": {
          target: backendTarget,
          changeOrigin: true,
        },
        "/boot": {
          target: backendTarget,
          changeOrigin: true,
        },
      },
    },
    test: {
      include: ["src/**/*.{test,spec}.{ts,tsx}"],
      exclude: ["e2e/**"],
      environment: "jsdom",
      globals: true,
      setupFiles: "./src/test/setup.ts",
    },
  };
});
