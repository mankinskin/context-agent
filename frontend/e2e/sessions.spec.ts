import { test, expect } from "@playwright/test";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { checkLayout, openSessions, primaryAction } from "./responsive";

test("offline nested navigation survives reload, media changes and owned restart", async ({ page, browser }, info) => {
  await info.attach("browser-version", { body: browser.version(), contentType: "text/plain" });
  await info.attach("media", { body: JSON.stringify(info.project.metadata), contentType: "application/json" });
  const rail = page.locator(".session-rail");
  const tabs = rail.locator(".session-tab");
  const message = page.getByRole("textbox", { name: "Message", exact: true });
  let proofWritten = false;
  const activeId = async () => {
    const title = await rail.locator(".session-tab[aria-pressed=true]").getAttribute("title");
    return title!.split(" · ").at(-1)!;
  };
  const select = async (id: string) => {
    await openSessions(page, info);
    await primaryAction(rail.getByTitle(`browser-default · ${id}`, { exact: true }), info);
    await expect(rail.getByTitle(`browser-default · ${id}`, { exact: true })).toHaveAttribute("aria-pressed", "true");
    if (proofWritten && id === grandchildA) {
      await expect(page.getByText("offline nested branch proof", { exact: true })).toBeVisible();
    } else {
      await expect(page.getByText("No messages in this session yet.", { exact: true })).toBeVisible();
    }
  };
  const create = async (name: string) => {
    const previous = await rail.locator(".session-tab[aria-pressed=true]").count() ? await activeId() : "";
    await primaryAction(page.getByRole("button", { name, exact: true }), info);
    await expect.poll(activeId).not.toBe(previous);
    return activeId();
  };
  await page.goto("/");
  await openSessions(page, info);
  const rootA = await create("New session");
  await expect(tabs).toHaveCount(1);
  const childA = await create("New child session");
  await expect(tabs).toHaveCount(2);
  const grandchildA = await create("New child session");
  await expect(tabs).toHaveCount(3);
  const rootB = await create("New session");
  await expect(tabs).toHaveCount(4);
  expect(new Set([rootA, childA, grandchildA, rootB]).size).toBe(4);
  const treeOrder = [rootB, rootA, childA, grandchildA];
  const assertTree = async () => {
    await openSessions(page, info);
    await expect(tabs).toHaveCount(4);
    expect(await Promise.all((await tabs.all()).map(tab => tab.getAttribute("title"))))
      .toEqual(treeOrder.map(id => `browser-default · ${id}`));
    for (const [id, level] of [[rootA, "1"], [childA, "2"], [grandchildA, "3"], [rootB, "1"]]) {
      await expect(rail.getByTitle(`browser-default · ${id}`, { exact: true })).toHaveAttribute("aria-level", level);
    }
    if (info.project.use.viewport!.width <= 900) {
      await primaryAction(page.getByRole("button", { name: "Close sessions" }), info);
    }
  };
  await assertTree();
  await select(grandchildA);
  await message.fill("offline nested branch proof");
  await primaryAction(page.getByRole("button", { name: /^Send/ }), info);
  await expect(page.getByText(/offline_ai_unavailable/)).toBeVisible();
  proofWritten = true;
  for (const id of [rootA, childA, rootB, grandchildA]) await select(id);
  await expect(page.getByText("offline nested branch proof", { exact: true })).toBeVisible();
  await message.fill("nested draft");
  await message.focus();
  const original = info.project.use.viewport!;
  const sizes = [639, 640, 641, 899, 900, 901].map(width => ({ width, height: original.height }));
  if (info.project.use.hasTouch) sizes.push({ width: original.height, height: original.width });
  sizes.push(original);
  for (const size of sizes) {
    await page.setViewportSize(size);
    await expect(message).toHaveValue("nested draft");
    await expect(message).toBeFocused();
    await expect(rail.getByTitle(`browser-default · ${grandchildA}`, { exact: true })).toHaveAttribute("aria-pressed", "true");
    await checkLayout(page);
  }
  await page.reload();
  await expect(page.getByText("offline nested branch proof", { exact: true })).toBeVisible();
  await expect(rail.getByTitle(`browser-default · ${grandchildA}`, { exact: true })).toHaveAttribute("aria-pressed", "true");
  await assertTree();
  await page.screenshot({ path: info.outputPath("sessions-before.png"), fullPage: true });
  await writeFile(join(process.env.SESSIONS_CONTROL_ROOT!, "sessions-ready.json"), JSON.stringify({
    rootA, childA, grandchildA, rootB, selected: grandchildA, url: page.url(), tree_order: treeOrder,
  }));
  await expect.poll(async () => {
    try { return JSON.parse(await readFile(join(process.env.SESSIONS_CONTROL_ROOT!, "sessions-restarted.json"), "utf8")).ready; }
    catch (error) { if ((error as NodeJS.ErrnoException).code === "ENOENT") return false; throw error; }
  }, { timeout: 20_000 }).toBe(true);
  await page.reload();
  await expect(page.getByText("offline nested branch proof", { exact: true })).toBeVisible();
  await expect(rail.getByTitle(`browser-default · ${grandchildA}`, { exact: true })).toHaveAttribute("aria-pressed", "true");
  await assertTree();
  for (const id of [rootA, childA, rootB, grandchildA]) await select(id);
  await expect(page.getByText("offline nested branch proof", { exact: true })).toBeVisible();
  await checkLayout(page);
  await page.screenshot({ path: info.outputPath("sessions-after.png"), fullPage: true });
});
