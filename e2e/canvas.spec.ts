import { expect, test, type Page } from "@playwright/test";

type Call = { cmd: string; args: Record<string, unknown> };

async function calls(page: Page, cmd: string): Promise<Call[]> {
  return page.evaluate(
    (c) => (window as unknown as { __sonataMock: { calls: Call[] } }).__sonataMock.calls.filter((x) => x.cmd === c),
    cmd,
  );
}

async function createWorkspace(page: Page) {
  await page.goto("/?mock");
  await page.getByRole("button", { name: "Create workspace" }).click();
  await page.getByLabel("Name").fill("Demo");
  await page.getByLabel("Folder").fill("/projects/demo");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByRole("application", { name: "Canvas" })).toBeVisible();
}

test("add, move, resize, rename, front, pan and zoom", async ({ page }) => {
  await createWorkspace(page);

  await page.getByRole("button", { name: "Add terminal" }).click();
  await page.getByLabel("Launch command").fill("echo hi");
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await page.getByRole("button", { name: "Add terminal" }).click();
  await page.getByRole("button", { name: "Add", exact: true }).click();
  expect(await calls(page, "node_create_terminal")).toHaveLength(2);

  const first = page.locator("[data-node-id]").first();
  const title = first.locator(".node-title");
  const box = (await title.boundingBox())!;
  await page.mouse.move(box.x + 60, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 160, box.y + box.height / 2 + 40, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => (await calls(page, "node_update_frame")).length).toBeGreaterThan(0);
  expect(await calls(page, "node_bring_to_front")).not.toHaveLength(0);

  const handle = first.locator(".resize-handle");
  const hb = (await handle.boundingBox())!;
  await page.mouse.move(hb.x + 4, hb.y + 4);
  await page.mouse.down();
  await page.mouse.move(hb.x + 80, hb.y + 60, { steps: 5 });
  await page.mouse.up();
  await expect.poll(async () => (await calls(page, "node_update_frame")).length).toBeGreaterThan(1);

  await title.dblclick();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Agent");
  await page.keyboard.press("Enter");
  await expect.poll(async () => (await calls(page, "terminal_rename")).map((c) => c.args.name)).toContain("Agent");

  const canvas = page.getByRole("application", { name: "Canvas" });
  const cb = (await canvas.boundingBox())!;
  await page.mouse.move(cb.x + cb.width - 40, cb.y + cb.height - 40);
  await page.mouse.wheel(0, 120);
  await page.keyboard.down("Control");
  await page.mouse.wheel(0, -50);
  await page.keyboard.up("Control");
  await expect
    .poll(async () => (await calls(page, "workspace_set_camera")).length, { timeout: 3000 })
    .toBeGreaterThan(0);
});
