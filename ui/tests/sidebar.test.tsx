import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Bootstrap, WorkspaceSummary } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  appBootstrap: vi.fn(),
  workspaceCreate: vi.fn(),
  workspaceOpen: vi.fn(),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => "/picked/folder") }));

import { App } from "../src/App";
import { resetInitForTests, useWorkspaces } from "../src/state/workspaces";

const emptyBoot: Bootstrap = {
  workspaces: [],
  loadFailures: [],
  activeWorkspaceId: null,
  crashGuardTripped: false,
  recentWorkspaceIds: [],
  preferences: { autosaveIntervalSeconds: 30, terminalFontSize: 13, historyMaxRows: 10000 },
  warnings: [],
};

const summary: WorkspaceSummary = {
  id: "w1",
  name: "Demo",
  icon: "rocket",
  color: "#3B82F6",
  workingDirectory: "/projects/demo",
  workingDirectoryMissing: false,
  isPinned: false,
  isLoaded: false,
  busyTerminalCount: 0,
};

beforeEach(() => {
  vi.clearAllMocks();
  useWorkspaces.setState(useWorkspaces.getInitialState(), true);
  resetInitForTests();
  ipc.appBootstrap.mockResolvedValue(emptyBoot);
});

async function openDialog() {
  render(<App />);
  const button = await screen.findByRole("button", { name: "Create workspace" });
  await userEvent.click(button);
  return screen.getByRole("dialog", { name: "New workspace" });
}

describe("sidebar and create dialog", () => {
  it("shows the empty state", async () => {
    render(<App />);
    expect(await screen.findByRole("button", { name: "Create workspace" })).toBeInTheDocument();
    expect(screen.getByText(/no workspaces yet/i)).toBeInTheDocument();
  });

  it.each([
    ["NameEmpty", "Enter a workspace name."],
    ["NameTaken", 'A workspace named "Demo" already exists.'],
    ["DirectoryMissing", "The folder /nope does not exist."],
    ["NotADirectory", "/file.txt is not a folder."],
  ])("shows the %s error message", async (code, message) => {
    ipc.workspaceCreate.mockRejectedValue({ code, message });
    await openDialog();
    await userEvent.type(screen.getByLabelText("Name"), "Demo");
    await userEvent.type(screen.getByLabelText("Folder"), "/nope");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(message);
  });

  it("adds the created workspace with its icon and color", async () => {
    ipc.workspaceCreate.mockResolvedValue(summary);
    ipc.workspaceOpen.mockResolvedValue({
      ...summary,
      createdAt: "",
      lastOpenedAt: "",
      camera: { x: 0, y: 0, zoom: 1 },
      nodes: [],
    });
    await openDialog();
    await userEvent.type(screen.getByLabelText("Name"), "Demo");
    await userEvent.click(screen.getByRole("button", { name: "Choose…" }));
    expect(screen.getByLabelText("Folder")).toHaveValue("/picked/folder");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(ipc.workspaceCreate).toHaveBeenCalledWith("Demo", "/picked/folder"));
    const item = await screen.findByRole("button", { name: "Demo" });
    expect(item.querySelector("[data-icon='rocket']")).not.toBeNull();
    expect(item.querySelector("[data-color='#3B82F6']")).not.toBeNull();
  });
});
