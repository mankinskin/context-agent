import { test, expect, type Page, type TestInfo } from "@playwright/test";
import { primaryAction, checkLayout } from "./responsive";

async function configure(page: Page, info: TestInfo) {
  if (!await page.locator(".preamble-editor").getAttribute("open").then(value => value !== null))
    await page.getByText("Global preambles", { exact: true }).click();
  for (const id of ["PREAMBLE_ENABLED_29AD", "PREAMBLE_DISABLED_29AD"]) {
    await page.getByRole("textbox", { name: "Preamble id", exact: true }).fill(id);
    await page.getByRole("textbox", { name: "Preamble text", exact: true }).fill(`Synthetic instruction ${id}.`);
    await primaryAction(page.getByRole("button", { name: "Add preamble", exact: true }), info);
    await expect(page.getByRole("checkbox", { name: `Enable ${id}`, exact: true })).toBeVisible();
  }
  await page.getByRole("checkbox", { name: "Enable PREAMBLE_ENABLED_29AD", exact: true }).check();
  await expect(page.getByRole("checkbox", { name: "Enable PREAMBLE_ENABLED_29AD", exact: true })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Enable PREAMBLE_DISABLED_29AD", exact: true })).not.toBeChecked();
  // The subsequent ordinary reload also proves the completed PUT reached the store.
  await expect(page.getByRole("button", { name: "Add preamble", exact: true })).toBeEnabled();
}

test("preamble selection and genuine choice stay usable across media", async ({ page, browser }, info) => {
  const phase = process.env.CHOICE_PHASE ?? "offline";
  test.setTimeout(phase === "origin" ? 95_000 : 30_000);
  await info.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  let submits = 0;
  page.on("request", request => { if (request.method() === "POST" && /\/messages$/.test(request.url())) submits++; });
  await page.goto("/");
  if (phase !== "retained") {
    await configure(page, info);
    await primaryAction(page.getByRole("button", { name: "Start lookup choice", exact: true }), info);
  } else {
    await page.getByRole("button", { name: "Refresh sessions", exact: true }).click();
    const sessions = page.locator(".session-rail .session-tab");
    if ((page.viewportSize()?.width ?? 1440) <= 900) await page.getByRole("button", { name: "Show sessions" }).click();
    await expect(sessions).toHaveCount(1);
    await primaryAction(sessions.first(), info);
  }
  if (phase === "offline") {
    await expect(page.getByRole("alert")).toContainText("offline_ai_unavailable");
    await expect(page.locator(".choice-widget")).toHaveCount(0);
    await expect(page.locator(".tool-call")).toHaveCount(0);
  } else {
    const widget = page.locator(".choice-widget");
    await expect(widget).toBeVisible({ timeout: phase === "origin" ? 45_000 : 5_000 });
    const id = await widget.getAttribute("data-widget-id");
    expect(id).toMatch(/^choice-response-/);
    if (phase === "origin") {
      await expect(page.locator(".message-assistant .tool-call")).toContainText("workflow-proof-29ad8d96");
      await primaryAction(widget.getByText("lookup-a", { exact: true }), info);
      await primaryAction(page.getByRole("button", { name: "Submit choice", exact: true }), info);
    }
    await expect(page.getByText("Submitted: lookup-a", { exact: true })).toBeVisible({ timeout: 45_000 });
    await expect(page.locator(".choice-answer")).toHaveCount(1);
    await expect(page.locator(".message-assistant .tool-call")).toHaveCount(2, { timeout: 45_000 });
    await expect(page.getByRole("alert")).toHaveCount(0);
    for (const tool of await page.locator(".message-assistant .tool-call").all()) {
      await expect(tool).toContainText("workflow-proof-29ad8d96");
      await expect(tool).toContainText("completed");
    }
    await page.reload();
    await expect(page.locator(".choice-widget")).toHaveAttribute("data-widget-id", id!);
    await expect(page.getByText("Submitted: lookup-a", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Submit choice", exact: true })).toBeDisabled();
  }
  if (await page.locator(".preamble-editor").getAttribute("open") === null)
    await page.getByText("Global preambles", { exact: true }).click();
  await expect(page.getByRole("checkbox", { name: "Enable PREAMBLE_ENABLED_29AD", exact: true })).toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Enable PREAMBLE_DISABLED_29AD", exact: true })).not.toBeChecked();
  for (const width of [639, 640, 641, 899, 900, 901]) {
    await page.setViewportSize({ width, height: 844 });
    await page.getByRole("textbox", { name: "Message", exact: true }).fill("preserved unsent choice context");
    await checkLayout(page);
    await expect(page.getByRole("textbox", { name: "Message", exact: true })).toHaveValue("preserved unsent choice context");
  }
  await page.screenshot({ path: info.outputPath("choice-proof.png"), fullPage: true });
  expect(submits).toBe(phase === "origin" ? 2 : phase === "offline" ? 1 : 0);
});
