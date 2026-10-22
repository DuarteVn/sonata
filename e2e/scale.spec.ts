import { expect, test } from "@playwright/test";

test("30 terminals: previews when zoomed out, live when zoomed in, WebGL bounded", async ({ page }) => {
  await page.goto("/?mock&seed=30");
  const canvas = page.getByRole("application", { name: "Canvas" });
  await expect(page.locator(".terminal-node")).toHaveCount(30);
  const box = (await canvas.boundingBox())!;

  // Zoom out until text is too small: no live terminals, only previews.
  await page.mouse.move(box.x + 50, box.y + 50);
  await page.keyboard.down("Control");
  for (let i = 0; i < 12; i++) await page.mouse.wheel(0, 60);
  await page.keyboard.up("Control");
  await expect(page.locator(".live-terminal")).toHaveCount(0, { timeout: 3000 });
  await expect(page.locator(".terminal-preview").first()).toBeVisible();

  // Zoom back in on the first terminal: it becomes live.
  await page.keyboard.down("Control");
  for (let i = 0; i < 12; i++) await page.mouse.wheel(0, -60);
  await page.keyboard.up("Control");
  await expect(page.locator(".live-terminal").first()).toBeVisible({ timeout: 3000 });
  const webgl = await page.locator('.live-terminal[data-renderer="webgl"]').count();
  expect(webgl).toBeLessThanOrEqual(12);

  const subs = await page.evaluate(
    () =>
      (window as unknown as { __sonataMock: { calls: { cmd: string }[] } }).__sonataMock.calls.filter(
        (c) => c.cmd === "terminal_preview_subscribe",
      ).length,
  );
  expect(subs).toBeGreaterThan(0);
});
