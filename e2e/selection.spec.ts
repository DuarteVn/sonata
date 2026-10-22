import { expect, test, type Page } from "@playwright/test";

type Call = { cmd: string; args: Record<string, unknown> };
const calls = (page: Page, cmd: string) =>
  page.evaluate((c) => (window as unknown as { __sonataMock: { calls: Call[] } }).__sonataMock.calls.filter((x) => x.cmd === c), cmd);

async function openWithNotes(page: Page) {
  await page.goto("/?mock");
  await page.getByRole("button", { name: "Create workspace" }).click();
  await page.getByLabel("Name").fill("Demo");
  await page.getByLabel("Folder").fill("/projects/demo");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  const canvas = page.getByRole("application", { name: "Canvas" });
  await expect(canvas).toBeVisible();
  const box = (await canvas.boundingBox())!;
  for (const [x, y] of [[100, 100], [420, 140], [100, 420]]) {
    await page.keyboard.press("n");
    await page.mouse.click(box.x + x, box.y + y);
    await expect.poll(async () => (await calls(page, "node_create_note")).length).toBeGreaterThan(0);
    // Leave the new note's editor so shortcuts reach the canvas.
    await page.mouse.click(box.x + box.width - 20, box.y + box.height - 140);
  }
  await expect(page.locator(".note-node")).toHaveCount(3);
  return box;
}

test("a selection rectangle selects several items that move and delete together", async ({ page }) => {
  const box = await openWithNotes(page);

  // Drag across the two top notes from empty canvas.
  await page.mouse.move(box.x + 60, box.y + 60);
  await page.mouse.down();
  await page.mouse.move(box.x + 700, box.y + 200, { steps: 6 });
  await expect(page.locator(".marquee")).toBeVisible();
  await page.mouse.up();
  await expect(page.locator(".marquee")).toHaveCount(0);
  await expect(page.locator(".note-node.selected")).toHaveCount(2);

  // Dragging one selected note moves both.
  const title = page.locator(".note-node.selected .note-title").first();
  const t = (await title.boundingBox())!;
  await page.mouse.move(t.x + 40, t.y + t.height / 2);
  await page.mouse.down();
  await page.mouse.move(t.x + 140, t.y + t.height / 2 + 30, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => new Set((await calls(page, "node_update_frame")).map((c) => c.args.nodeId)).size).toBe(2);
  await expect(page.locator(".note-node.selected")).toHaveCount(2);

  // Shift+click adds the third note; Delete removes all three.
  const third = page.locator(".note-node:not(.selected) .note-title");
  await third.click({ modifiers: ["Shift"] });
  await expect(page.locator(".note-node.selected")).toHaveCount(3);
  await page.keyboard.press("Delete");
  await expect.poll(async () => (await calls(page, "node_delete")).length).toBe(3);
  await expect(page.locator(".note-node")).toHaveCount(0);
});
