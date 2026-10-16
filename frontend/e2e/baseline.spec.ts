import { test, expect } from "@playwright/test";
import { openSessions, primaryAction, checkLayout } from "./responsive";

test("offline responsive session selection preserves draft and focus", async ({ page, browser }, info) => {
  await info.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  await info.attach("media", { body: JSON.stringify(info.project.metadata), contentType: "application/json" });
  await page.goto("/");
  await openSessions(page);
  await primaryAction(page.getByRole("button", { name: "New session", exact: true }), info);
  if ((page.viewportSize()?.width ?? 1440) <= 900) await page.getByRole("button", { name: "Close sessions" }).click();
  const tabs = page.locator(".editor-tabs .session-tab");
  await expect(tabs).toHaveCount(1);
  const first = await tabs.first().getAttribute("title");
  await primaryAction(page.getByRole("button", { name: "Create parallel session" }), info);
  await expect(tabs).toHaveCount(2);
  await info.attach("sessions-observed", {
    body: JSON.stringify({ session_ids: await Promise.all((await tabs.all()).map(async tab => {
      const title = await tab.getAttribute("title");
      const match = title?.match(/[\da-f]{8}-(?:[\da-f]{4}-){3}[\da-f]{12}$/i);
      expect(match).not.toBeNull();
      return match![0];
    })) }),
    contentType: "application/json",
  });
  const message = page.getByRole("textbox", { name: "Message", exact: true });
  await message.fill("responsive draft");
  await message.focus();
  for (const width of [639, 640, 641, 899, 900, 901, info.project.use.viewport!.width]) {
    await page.setViewportSize({ width, height: info.project.use.viewport!.height });
    await expect(message).toHaveValue("responsive draft");
    await expect(message).toBeFocused();
    await checkLayout(page);
  }
  if (info.project.use.hasTouch) {
    const { width, height } = info.project.use.viewport!;
    await page.setViewportSize({ width: height, height: width });
    await expect(message).toHaveValue("responsive draft");
    await expect(message).toBeFocused();
    await checkLayout(page);
    await page.setViewportSize({ width, height });
  }
  await openSessions(page);
  await primaryAction(page.locator(".session-rail").getByTitle(first!, { exact: true }), info);
  await expect(tabs.filter({ has: page.getByText("Active", { exact: true }) })).toHaveCount(1);
  await expect(message).toHaveValue("responsive draft");
  if ((page.viewportSize()?.width ?? 1440) <= 900) await primaryAction(page.getByRole("button", { name: "Show inspector" }), info);
  await expect(page.getByRole("complementary", { name: "Run inspector" })).toBeVisible();
  await primaryAction(page.getByRole("button", { name: "Load credits" }), info);
  await expect(page.getByText(/cache_read_tokens/)).toBeVisible();
  await page.screenshot({ path: info.outputPath("baseline-responsive.png"), fullPage: true });
});
