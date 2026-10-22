import { expect, test } from "@playwright/test";

test("the sidebar resizes by dragging its edge, and double-click resets it", async ({ page }) => {
  await page.goto("/?mock");
  const nav = page.getByRole("navigation", { name: "Workspaces" });
  const handle = page.getByRole("separator", { name: "Resize sidebar" });
  const h = (await handle.boundingBox())!;
  await page.mouse.move(h.x + h.width / 2, h.y + 200);
  await page.mouse.down();
  await page.mouse.move(h.x + h.width / 2 + 120, h.y + 200, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => (await nav.boundingBox())!.width).toBeCloseTo(352, 0);

  await page.mouse.move(h.x + h.width / 2 + 120, h.y + 200);
  await page.mouse.down();
  await page.mouse.move(h.x - 400, h.y + 200, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => (await nav.boundingBox())!.width).toBeCloseTo(180, 0);

  await handle.dblclick();
  await expect.poll(async () => (await nav.boundingBox())!.width).toBeCloseTo(232, 0);
});
