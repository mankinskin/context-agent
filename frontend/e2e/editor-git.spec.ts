import { test, expect } from "@playwright/test";
import { primaryAction, checkLayout } from "./responsive";

test("rooted editor saves real Git changes and preserves responsive draft and focus", async ({ page, browser }, info) => {
  await info.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  let messages = 0;
  page.on("request", request => {
    if (request.method() === "POST" && /\/messages$/.test(request.url())) messages++;
  });
  await page.goto("/");
  await page.getByText("Rooted file editor", { exact: true }).click();
  const path = page.getByRole("textbox", { name: "File path", exact: true });
  const content = page.getByRole("textbox", { name: "File content", exact: true });
  const open = page.getByRole("button", { name: "Open file", exact: true });
  const save = page.getByRole("button", { name: "Save file", exact: true });
  const diff = page.getByRole("region", { name: "Git diff", exact: true });
  await expect(path).toHaveValue("notes/demo.txt");
  await primaryAction(open, info);
  await expect(content).toHaveValue(/original fixture text/);
  const draft = "original fixture text\nEDITOR_PROOF_29AD\n" + "long-line-".repeat(180) + "\n";
  await content.fill(draft);
  await page.getByRole("textbox", { name: "Message", exact: true }).fill("unsent editor context");
  await content.focus();
  const original = page.viewportSize()!;
  for (const viewport of [
    { width: original.height, height: original.width },
    ...[639, 640, 641, 899, 900, 901].map(width => ({ width, height: 844 })),
    original,
  ]) {
    await page.setViewportSize(viewport);
    await expect(content).toHaveValue(draft);
    await expect(content).toBeFocused();
    await expect(page.getByRole("textbox", { name: "Message", exact: true })).toHaveValue("unsent editor context");
    await checkLayout(page);
  }
  await primaryAction(save, info);
  await expect(page.getByText("Saved notes/demo.txt", { exact: true })).toBeVisible();
  await expect(diff).toContainText("+EDITOR_PROOF_29AD");
  await expect(diff).toContainText("notes/demo.txt");
  await path.fill("../outside/demo.txt");
  await primaryAction(open, info);
  await expect(page.getByRole("alert")).toContainText("editor_path_rejected");
  await expect(content).toHaveValue(draft);
  await path.fill("notes/demo.txt");
  await primaryAction(open, info);
  await expect(content).toHaveValue(draft);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.reload();
  await page.getByText("Rooted file editor", { exact: true }).click();
  await primaryAction(page.getByRole("button", { name: "Open file", exact: true }), info);
  await expect(page.getByRole("textbox", { name: "File content", exact: true })).toHaveValue(draft);
  await expect(page.getByRole("region", { name: "Git diff", exact: true })).toContainText("+EDITOR_PROOF_29AD");
  await checkLayout(page);
  await page.screenshot({ path: info.outputPath("editor-git-proof.png"), fullPage: true });
  expect(messages).toBe(0);
});
