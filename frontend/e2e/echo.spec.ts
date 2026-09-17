import { test, expect } from "@playwright/test";

test("renders a rich weather response in the tiled editor", async ({ page }) => {
  const initialSessions = await page.request.get("/api/sessions");
  const initialSessionCount = (await initialSessions.json()).length;

  await page.goto("/");
  await expect(page.getByRole("button", { name: "New session" })).toBeVisible();

  await page.getByRole("textbox", { name: "Message" }).fill("How is the weather in Münster today?");
  await page.getByRole("button", { name: /Send/ }).click({ force: true });

  await expect(page.getByRole("heading", { name: "Weather in Münster" })).toBeVisible();
  await expect(page.getByText(/feels like/i)).toBeVisible();
  await expect(page.getByText(/Precipitation:/i)).toBeVisible();
  await expect(page.getByText(/Wind:/i)).toBeVisible();
  await expect(page.locator(".tool-call code", { hasText: "fetch_weather" })).toBeVisible();
  await expect(page.locator(".message-stream .message")).toHaveCount(2);
  await expect(page.locator(".editor-tabs .session-tab")).toHaveCount(initialSessionCount + 1);
  await expect(page.locator(".editor-tabs .session-tab.is-active .active-indicator")).toBeVisible();
  await expect(page.locator(".editor-tabs")).toHaveCSS("overflow-y", "hidden");
  await expect(page.locator(".workspace")).toHaveCSS("display", "grid");

  await page.getByRole("button", { name: "Create parallel session" }).click({ force: true });
  await expect(page.locator(".editor-tabs .session-tab")).toHaveCount(initialSessionCount + 2);
  await expect(page.locator(".editor-tabs .session-tab.is-active")).toHaveCount(1);
  await expect(page.locator(".editor-tabs .session-tab").first()).toHaveClass(/is-active/);
  await expect(page.locator(".editor-tabs .session-tab-label").first()).toHaveText(/^Session \d+$/);
});
