import { expect, test, type Page } from "@playwright/test";

type Call = { cmd: string; args: Record<string, unknown> };
const calls = (page: Page, cmd: string) =>
  page.evaluate((c) => (window as unknown as { __sonataMock: { calls: Call[] } }).__sonataMock.calls.filter((x) => x.cmd === c), cmd);

async function open(page: Page) {
  await page.goto("/?mock");
  await page.getByRole("button", { name: "Create workspace" }).click();
  await page.getByLabel("Name").fill("Demo");
  await page.getByLabel("Folder").fill("/projects/demo");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  const canvas = page.getByRole("application", { name: "Canvas" });
  await expect(canvas).toBeVisible();
  return (await canvas.boundingBox())!;
}

test("tool bar places terminals and notes where clicked", async ({ page }) => {
  const box = await open(page);
  const tools = page.getByRole("toolbar", { name: "Tools" });
  await expect(tools.getByRole("button", { name: "Select tool" })).toHaveAttribute("aria-pressed", "true");

  await page.keyboard.press("t");
  await expect(tools.getByRole("button", { name: "Terminal tool" })).toHaveAttribute("aria-pressed", "true");
  await page.mouse.click(box.x + 200, box.y + 150);
  const dialog = page.getByRole("dialog", { name: "New terminal" });
  await expect(dialog.getByRole("button", { name: /OpenCode/ })).toContainText("not found");
  await dialog.getByRole("button", { name: /Claude Code/ }).click();
  await dialog.getByRole("button", { name: "Create", exact: true }).click();
  const created = (await calls(page, "node_create_terminal"))[0];
  expect(created.args.launchCommand).toBe("claude");
  expect((created.args.frame as { x: number }).x).toBeCloseTo(200, 0);
  await expect(tools.getByRole("button", { name: "Select tool" })).toHaveAttribute("aria-pressed", "true");

  // The new terminal has keyboard focus; click empty canvas so shortcuts reach the canvas.
  await page.mouse.click(box.x + box.width - 20, box.y + box.height - 120);
  await page.keyboard.press("n");
  await page.mouse.click(box.x + 900, box.y + 120);
  await expect(page.locator(".note-node")).toHaveCount(1);
  await expect.poll(async () => (await calls(page, "node_create_note")).length).toBe(1);
});

test("terminal toolbar: edit, link, restart, delete", async ({ page }) => {
  const box = await open(page);
  await page.keyboard.press("t");
  await page.mouse.click(box.x + 100, box.y + 100);
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await page.locator(".terminal-node .node-title").first().click({ position: { x: 120, y: 10 } });
  const bar = page.getByRole("toolbar", { name: "Terminal actions" });
  await expect(bar).toBeVisible();
  await expect(bar.getByRole("button", { name: "Edit terminal" })).toBeVisible();
  await expect(bar.getByRole("button", { name: "Link to another item" })).toBeVisible();
  await bar.getByRole("button", { name: "Restart terminal" }).click();
  await expect.poll(async () => (await calls(page, "terminal_restart")).length).toBe(1);
  await bar.getByRole("button", { name: "Edit terminal" }).click();
  await expect(page.getByRole("dialog", { name: "Edit terminal" })).toBeVisible();
  await page.getByRole("button", { name: "Cancel" }).click();
  await bar.getByRole("button", { name: "Delete terminal" }).click();
  await expect(page.locator(".terminal-node")).toHaveCount(0);
});

test("note toolbar formats text and styles the note", async ({ page }) => {
  const box = await open(page);
  await page.keyboard.press("n");
  await page.mouse.click(box.x + 300, box.y + 200);
  const bar = page.getByRole("toolbar", { name: "Note actions" });
  await expect(bar).toBeVisible();
  await bar.getByRole("button", { name: "Checklist" }).click();
  await page.keyboard.type("ship it");
  await bar.getByRole("button", { name: "Bold" }).click();
  await page.keyboard.type("now");
  await page.getByRole("application", { name: "Canvas" }).click({ position: { x: 20, y: 20 } });
  await expect.poll(async () => (await calls(page, "note_write")).at(-1)?.args.text).toBe("- [ ] ship it**now**\n".trimEnd());
  await page.locator(".note-node").click({ position: { x: 100, y: 10 } });
  await bar.getByRole("button", { name: "Note color" }).click();
  await page.getByRole("radio", { name: "blue" }).click();
  await bar.getByRole("button", { name: "Larger text" }).click();
  await expect.poll(async () => (await calls(page, "note_set_style")).at(-1)?.args.style).toEqual({ color: "blue", textSize: 4 });
});
