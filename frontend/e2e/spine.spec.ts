import { test, expect } from "@playwright/test";
import { primaryAction, openSessions, checkLayout } from "./responsive";

test("genuine Copilot get_ticket proof is visible and reconnectable", async ({ page, browser }, info) => {
  test.setTimeout(95_000);
  await info.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  await info.attach("media", { body: JSON.stringify(info.project.metadata), contentType: "application/json" });
  let submissions = 0;
  page.on("request", request => {
    if (request.method() === "POST" && /\/messages$/.test(request.url())) submissions++;
  });
  await page.goto("/");
  if (process.env.SPINE_PHASE === "origin") {
    await openSessions(page);
    await primaryAction(page.getByRole("button", { name: "New session", exact: true }), info);
    if ((page.viewportSize()?.width ?? 1440) <= 900) await page.getByRole("button", { name: "Close sessions" }).click();
    await page.getByRole("textbox", { name: "Message", exact: true }).fill(
      "Read the authorized Workflow proof ticket using get_ticket. Report its proof marker exactly, using the tool result, not memory.");
    await primaryAction(page.getByRole("button", { name: /^Send/ }), info);
  } else if (process.env.SPINE_PHASE === "retained") {
    await page.getByRole("button", { name: "Refresh sessions", exact: true }).click();
    await openSessions(page);
    const sessions = page.locator(".session-rail .session-tab");
    await expect(sessions).toHaveCount(1);
    await primaryAction(sessions.first(), info);
  } else {
    throw new Error("owned runner must choose origin or retained");
  }
  const tool = page.locator(".message-assistant .tool-call").first();
  const failure = page.locator(".error-banner");
  await expect(tool.or(failure)).toBeVisible({ timeout: process.env.SPINE_PHASE === "origin" ? 85_000 : 5_000 });
  await expect(failure).toHaveCount(0);
  await expect(tool).toContainText("get_ticket", { timeout: process.env.SPINE_PHASE === "origin" ? 85_000 : 5_000 });
  await expect(tool).toContainText("workflow-proof-29ad8d96");
  await expect(tool.locator(".tool-call-id")).toContainText(/-tool-\d+/);
  await expect(tool).toContainText("completed");
  await checkLayout(page);
  await page.screenshot({ path: info.outputPath("spine-proof.png"), fullPage: true });
  expect(submissions).toBe(process.env.SPINE_PHASE === "origin" ? 1 : 0);
});
