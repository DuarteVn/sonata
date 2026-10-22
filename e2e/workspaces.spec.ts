import { expect, test, type Page } from "@playwright/test";

type Call = { cmd: string; args: Record<string, unknown> };
type Mock = { calls: Call[]; busy: unknown[]; emit: (name: string, payload: unknown) => Promise<void> };

const mock = (page: Page) => page.evaluateHandle(() => (window as unknown as { __sonataMock: Mock }).__sonataMock);
const calls = (page: Page, cmd: string) =>
  page.evaluate((c) => (window as unknown as { __sonataMock: Mock }).__sonataMock.calls.filter((x) => x.cmd === c), cmd);

async function create(page: Page, name: string, folder = "/projects/x") {
  await page.getByRole("button", { name: "New workspace" }).click();
  await page.getByLabel("Name").fill(name);
  await page.getByLabel("Folder").fill(folder);
  await page.getByRole("button", { name: "Create", exact: true }).click();
}

const sidebarNames = (page: Page) => page.locator(".ws-item .ws-name").allTextContents();

test("manage many workspaces", async ({ page }) => {
  await page.goto("/?mock");
  for (const n of ["One", "Two", "Three", "Four", "Five"]) await create(page, n);
  await expect(page.locator(".ws-item")).toHaveCount(5);

  // Three distinct validation errors; nothing created.
  await page.getByRole("button", { name: "New workspace" }).click();
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("Enter a workspace name.");
  await page.getByLabel("Name").fill("one");
  await page.getByLabel("Folder").fill("/projects/x");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText('A workspace named "one" already exists.');
  await page.getByLabel("Name").fill("Six");
  await page.getByLabel("Folder").fill("relative");
  await page.getByRole("button", { name: "Create", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("does not exist");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.locator(".ws-item")).toHaveCount(5);

  // Pin moves to the top.
  await page.getByRole("button", { name: "Pin Four" }).click();
  await expect.poll(() => sidebarNames(page)).toEqual(["Four", "One", "Two", "Three", "Five"]);

  // Drag "Five" above "One".
  const five = page.locator(".ws-item", { hasText: "Five" });
  const one = page.locator(".ws-item", { hasText: "One" });
  const from = (await five.boundingBox())!;
  const to = (await one.boundingBox())!;
  await page.mouse.move(from.x + 40, from.y + from.height / 2);
  await page.mouse.down();
  await page.mouse.move(from.x + 40, to.y + 2, { steps: 8 });
  await page.mouse.up();
  await expect.poll(async () => (await calls(page, "workspace_reorder")).length).toBe(1);
  await expect.poll(() => sidebarNames(page)).toEqual(["Four", "Five", "One", "Two", "Three"]);

  // Rename inline.
  await page.locator(".ws-item", { hasText: "Two" }).locator(".ws-name").dblclick();
  await page.getByLabel("Workspace name").fill("Deux");
  await page.keyboard.press("Enter");
  await expect(page.locator(".ws-item", { hasText: "Deux" })).toBeVisible();

  // Switch.
  await page.locator(".ws-item", { hasText: "One" }).locator(".ws-open").click();
  await expect.poll(async () => (await calls(page, "workspace_open")).at(-1)?.args.id).toBeTruthy();

  // Delete: cancel, then confirm. The dialog names the workspace.
  await page.getByRole("button", { name: "Delete Three" }).click();
  await expect(page.getByRole("dialog")).toContainText("Three");
  await page.getByRole("button", { name: "Cancel" }).click();
  await expect(page.locator(".ws-item")).toHaveCount(5);
  await page.getByRole("button", { name: "Delete Three" }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();
  await expect(page.locator(".ws-item")).toHaveCount(4);
  expect((await calls(page, "workspace_delete"))[0].args.confirmed).toBe(true);
});

test("quit dialog lists busy terminals per workspace", async ({ page }) => {
  await page.goto("/?mock");
  await create(page, "Agents");
  const m = await mock(page);
  await m.evaluate((x) => {
    x.busy = [{ workspaceId: "id-1", workspaceName: "Agents", count: 2 }];
  });
  await m.evaluate((x) => x.emit("app://quit-requested", null));
  const dialog = page.getByRole("dialog", { name: "Quit Sonata?" });
  await expect(dialog).toContainText("Agents");
  await expect(dialog).toContainText("2 terminals");
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toBeHidden();
  expect(await calls(page, "app_quit")).toHaveLength(0);

  await m.evaluate((x) => x.emit("app://quit-requested", null));
  await page.getByRole("button", { name: "Quit", exact: true }).click();
  await expect.poll(async () => (await calls(page, "app_quit")).at(-1)?.args.confirmed).toBe(true);
});

test("quits without asking when nothing runs", async ({ page }) => {
  await page.goto("/?mock");
  await create(page, "Idle");
  const m = await mock(page);
  await m.evaluate((x) => x.emit("app://quit-requested", null));
  await expect.poll(async () => (await calls(page, "app_quit")).length).toBe(1);
  await expect(page.getByRole("dialog")).toHaveCount(0);
});
