import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 30_000,
  outputDir: process.env.PLAYWRIGHT_OUTPUT_DIR ?? "test-results",
  use: {
    baseURL: process.env.BASE_URL ?? "http://127.0.0.1:8080",
    trace: "on",
  },
  reporter: "list",
});
