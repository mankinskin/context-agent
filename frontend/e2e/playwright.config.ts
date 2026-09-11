import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  timeout: 30_000,
  use: {
    baseURL: process.env.BASE_URL ?? "http://127.0.0.1:8080",
  },
  reporter: "list",
});
