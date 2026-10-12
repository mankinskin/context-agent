import { test, expect } from "@playwright/test";

test("creates and selects sessions through the offline desktop and narrow UI", async ({ page, browser }, testInfo) => {
  await testInfo.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  await page.goto("/");
  const editorTabs = page.locator(".editor-tabs");
  const tabs = editorTabs.getByRole("button", { name: /^Session / });
  await expect(page.getByRole("button", { name: "New session", exact: true })).toBeVisible();

  await page.getByRole("button", { name: "New session", exact: true }).click();
  await expect(tabs).toHaveCount(1);
  const firstTitle = await tabs.first().getAttribute("title");
  if (!firstTitle) throw new Error("The first session must expose its visible identity");
  await expect(tabs.first()).toHaveAttribute("aria-pressed", "true");

  await page.getByRole("button", { name: "Create parallel session", exact: true }).click();
  await expect(tabs).toHaveCount(2);
  const secondTitle = await tabs.filter({ has: page.getByText("Active", { exact: true }) }).getAttribute("title");
  if (!secondTitle) throw new Error("The second session must expose its visible identity");
  expect(secondTitle).not.toBe(firstTitle);

  for (const [name, width, height] of [
    ["desktop", 1440, 900],
    ["narrow", 390, 844],
  ] as const) {
    await page.setViewportSize({ width, height });
    const first = editorTabs.getByTitle(firstTitle, { exact: true });
    const second = editorTabs.getByTitle(secondTitle, { exact: true });
    await first.click();
    await expect(first).toHaveAttribute("aria-pressed", "true");
    await expect(second).toHaveAttribute("aria-pressed", "false");
    await second.click();
    await expect(second).toHaveAttribute("aria-pressed", "true");
    await expect(first).toHaveAttribute("aria-pressed", "false");
    await expect(page.getByRole("textbox", { name: "Message", exact: true })).toBeVisible();
    await page.screenshot({ path: testInfo.outputPath(`baseline-${name}.png`), fullPage: true });
  }
});
