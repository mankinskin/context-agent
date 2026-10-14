import { test, expect } from "@playwright/test";

test("native session and submitted input survive an owned backend restart", async ({ page, browser }, testInfo) => {
  await testInfo.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  await page.goto("/");
  const tabs = page.locator(".editor-tabs").getByRole("button", { name: /^Session / });
  if (process.env.BRIDGE_PHASE === "before") {
    await page.getByRole("button", { name: "New session", exact: true }).click();
    await expect(tabs).toHaveCount(1);
    await page.getByRole("textbox", { name: "Message", exact: true }).fill("offline native persistence proof");
    await page.getByRole("button", { name: /^Send/ }).click();
    await expect(page.getByText(/offline_ai_unavailable/)).toBeVisible();
    await expect(page.getByText(/correlation_id/)).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath("bridge-before.png"), fullPage: true });
  } else if (process.env.BRIDGE_PHASE === "after") {
    await page.getByRole("button", { name: "Refresh sessions", exact: true }).click();
    await expect(tabs).toHaveCount(1);
    await tabs.first().click();
    await expect(page.getByText("offline native persistence proof", { exact: true })).toBeVisible();
    await expect(tabs.first()).toHaveAttribute("aria-pressed", "true");
    await page.screenshot({ path: testInfo.outputPath("bridge-after.png"), fullPage: true });
  } else {
    throw new Error("The owned runner must choose before or after the real backend restart");
  }
});
