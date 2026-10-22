import { expect, test, type Page } from "@playwright/test";

type Call = { cmd: string; args: Record<string, unknown> };
const calls = (page: Page, cmd: string) =>
  page.evaluate((c) => (window as unknown as { __sonataMock: { calls: Call[] } }).__sonataMock.calls.filter((x) => x.cmd === c), cmd);

async function setup(page: Page) {
  await page.goto("/?mock");
  await page.getByRole("button", { name: "Create workspace" }).click();
  await page.getByLabel("Name").fill("Demo");
  await page.getByLabel("Folder").fill("/projects/demo");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  const box = (await page.getByRole("application", { name: "Canvas" }).boundingBox())!;
  const blur = () => page.mouse.click(box.x + box.width - 20, box.y + box.height - 120);
  for (const [x, y] of [[40, 60], [760, 60]]) {
    await blur();
    await page.keyboard.press("t");
    await page.mouse.click(box.x + x, box.y + y);
    await page.getByRole("button", { name: "Create", exact: true }).click();
  }
  await blur();
  await page.keyboard.press("n");
  await page.mouse.click(box.x + 400, box.y + 560);
  return box;
}

const node = (page: Page, i: number) => page.locator("[data-node-id].node").nth(i);

test("link by drag, refuse duplicates, link by toolbar, delete", async ({ page }) => {
  await setup(page);
  await expect(page.locator(".node")).toHaveCount(3);

  // Select the first terminal to reveal its link handle, then drag onto the note.
  await node(page, 0).locator(".node-title").click({ position: { x: 150, y: 10 } });
  const handle = node(page, 0).locator(".link-handle");
  const h = (await handle.boundingBox())!;
  const target = (await node(page, 2).boundingBox())!;
  await page.mouse.move(h.x + 5, h.y + 5);
  await page.mouse.down();
  await page.mouse.move(target.x + 60, target.y + 60, { steps: 8 });
  await page.mouse.up();
  await expect.poll(async () => (await calls(page, "link_create")).length).toBe(1);
  await expect(page.locator(".link-layer .link")).toHaveCount(1);

  // Dropping on empty canvas creates nothing.
  await page.mouse.move(h.x + 5, h.y + 5);
  await page.mouse.down();
  await page.mouse.move(h.x + 5, h.y + 300, { steps: 5 });
  await page.mouse.up();
  await expect(page.locator(".link-layer .link")).toHaveCount(1);

  // Link from the toolbar: first terminal → second terminal.
  await page.getByRole("toolbar", { name: "Terminal actions" }).getByRole("button", { name: "Link to another item" }).click();
  await node(page, 1).locator(".node-title").click({ position: { x: 150, y: 10 } });
  await expect(page.locator(".link-layer .link")).toHaveCount(2);
  await expect(node(page, 0).locator(".link-count")).toHaveText("2");

  // Same pair again in reverse: refused.
  await page.getByRole("toolbar", { name: "Terminal actions" }).getByRole("button", { name: "Link to another item" }).click();
  await node(page, 0).locator(".node-title").click({ position: { x: 150, y: 10 } });
  await expect(page.locator(".link-layer .link")).toHaveCount(2);
  await page.keyboard.press("Escape");

  // Select a link and delete it with the key.
  await page.locator(".link-hit").first().click({ force: true });
  await page.keyboard.press("Delete");
  await expect(page.locator(".link-layer .link")).toHaveCount(1);

  // Deleting a node removes its links.
  await node(page, 0).locator(".node-title").click({ position: { x: 150, y: 10 } });
  await page.keyboard.press("Delete");
  await expect(page.locator(".link-layer .link")).toHaveCount(0);
});
