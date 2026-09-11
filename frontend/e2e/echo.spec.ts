import { test, expect } from "@playwright/test";

// Generic browser smoke: type a message, submit it to /api/echo through the
// real UI, and confirm the echoed result renders. Generalized for any app
// built from this template that keeps the same single-input echo flow;
// replace this spec when the app grows real UI/endpoints.
test("echoes a submitted message through the UI", async ({ page }) => {
  await page.goto("/");
  await page.locator("input").fill("smoke-test");
  await page.getByRole("button", { name: "Send to /api/echo" }).click();
  await expect(page.getByText("Echoed: smoke-test")).toBeVisible();
});
