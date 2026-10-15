import { expect, type Locator, type Page, type TestInfo } from "@playwright/test";

export async function primaryAction(control: Locator, info: TestInfo) {
  await expect(control).toBeVisible();
  if (info.project.use.hasTouch) {
    const bounds = await control.boundingBox();
    expect(bounds?.width).toBeGreaterThanOrEqual(44);
    expect(bounds?.height).toBeGreaterThanOrEqual(44);
    await control.tap();
  } else {
    await control.click();
  }
}

export async function openSessions(page: Page, info?: TestInfo) {
  if ((page.viewportSize()?.width ?? 1440) <= 900) {
    const toggle = page.getByRole("button", { name: "Show sessions" });
    if (await toggle.getAttribute("aria-expanded") !== "true") {
      if (info) await primaryAction(toggle, info);
      else await toggle.click();
    }
  }
}

export async function checkLayout(page: Page) {
  const width = page.viewportSize()!.width;
  for (const selector of [".app-shell", ".editor-pane", ".composer", ".send-button"]) {
    const box = await page.locator(selector).boundingBox();
    expect(box, selector).not.toBeNull();
    expect(box!.x, selector).toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width, selector).toBeLessThanOrEqual(width + 1);
  }
  await expect(page.getByRole("button", { name: /^Send/ })).toBeVisible();
  if (width >= 901) {
    await expect(page.getByRole("complementary", { name: "Session navigation" })).toBeVisible();
    await expect(page.getByRole("complementary", { name: "Run inspector" })).toBeVisible();
  } else {
    await expect(page.getByRole("button", { name: "Show sessions" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Show inspector" })).toBeVisible();
  }
}
