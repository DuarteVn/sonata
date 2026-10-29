import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Bootstrap, CanvasNode } from "../src/ipc/types";
import { revealLabel } from "../src/platform";

const ipc = vi.hoisted(() => ({
  appBootstrap: vi.fn(),
  workspaceOpen: vi.fn(),
  appRevealPath: vi.fn(),
  terminalSetSaveHistory: vi.fn(async () => {}),
  terminalClearHistory: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { App } from "../src/App";
import { TerminalMenu } from "../src/nodes/TerminalMenu";
import { resetInitForTests, useWorkspaces } from "../src/state/workspaces";

const boot: Bootstrap = {
  workspaces: [],
  loadFailures: [
    {
      folderName: "6f1c0000-0000-4000-8000-000000000000",
      path: "/Users/me/.sonata/workspaces/6f1c0000-0000-4000-8000-000000000000/workspace.json",
      reason: { kind: "newerVersion", found: 3, supported: 1 },
    },
  ],
  activeWorkspaceId: null,
  crashGuardTripped: true,
  recentWorkspaceIds: [],
  preferences: { autosaveIntervalSeconds: 30, terminalFontSize: 13, historyMaxRows: 10000 },
  warnings: ["Sonata crashed twice right after starting, so it did not reopen your last workspace."],
};

beforeEach(() => {
  vi.clearAllMocks();
  useWorkspaces.setState(useWorkspaces.getInitialState(), true);
  resetInitForTests();
  ipc.appBootstrap.mockResolvedValue(boot);
});

describe("recovery UI", () => {
  it("shows a load failure with reason and path, and reveals it", async () => {
    render(<App />);
    const item = await screen.findByRole("listitem", { name: /could not be loaded/i });
    expect(item).toHaveTextContent("newer version");
    expect(item).toHaveTextContent("workspace.json");
    await userEvent.click(within(item).getByRole("button", { name: revealLabel() }));
    expect(ipc.appRevealPath).toHaveBeenCalledWith(boot.loadFailures[0].path);
  });

  it("shows the crash-guard and persistence warnings as dismissible banners", async () => {
    render(<App />);
    const banner = await screen.findByRole("status", { name: /warning/i });
    expect(banner).toHaveTextContent("crashed twice");
    useWorkspaces.getState().addWarning("Could not save: disk full");
    expect(await screen.findByText("Could not save: disk full")).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole("button", { name: "Dismiss" })[0]);
    expect(screen.queryByText(/crashed twice/)).not.toBeInTheDocument();
  });

  it("terminal menu toggles history saving and clears it", async () => {
    const node = {
      id: "t1",
      frame: { x: 0, y: 0, width: 400, height: 300 },
      z: 1,
      createdAt: "",
      modifiedAt: "",
      content: { terminal: { name: "T", saveHistory: true } },
    } satisfies CanvasNode;
    render(<TerminalMenu node={node} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    // Regression: the menu must render outside the canvas layers so live terminals cannot cover it.
    expect(screen.getByRole("menu").parentElement).toBe(document.body);
    const toggle = screen.getByRole("menuitemcheckbox", { name: "Save history" });
    expect(toggle).toHaveAttribute("aria-checked", "true");
    await userEvent.click(toggle);
    expect(ipc.terminalSetSaveHistory).toHaveBeenCalledWith("t1", false);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Clear saved history" }));
    expect(ipc.terminalClearHistory).toHaveBeenCalledWith("t1");
  });
});
