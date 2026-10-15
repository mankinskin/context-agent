import { defineConfig } from "@playwright/test";

const media = process.env.E2E_MEDIA
  ? JSON.parse(process.env.E2E_MEDIA)
  : { name: "desktop", viewport: { width: 1440, height: 900 }, hasTouch: false, isMobile: false };
export default defineConfig({
  testDir: ".",
  timeout: 30_000,
  outputDir: process.env.PLAYWRIGHT_OUTPUT_DIR ?? "test-results",
  use: {
    baseURL: process.env.BASE_URL ?? "http://127.0.0.1:8080",
    trace: "on",
  },
  reporter: "list",
  workers: 1,
  retries: 0,
  projects: [{
    name: media.name,
    use: { browserName: "chromium", viewport: media.viewport, hasTouch: media.hasTouch, isMobile: media.isMobile },
    metadata: { ...media, emulation: "Chromium; not physical device or iOS" },
  }],
});
