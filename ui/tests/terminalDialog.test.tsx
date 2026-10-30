import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  presetsDetect: vi.fn(),
  rolesList: vi.fn(),
  roleSave: vi.fn(),
  roleDelete: vi.fn(async () => {}),
  nodeCreateTerminal: vi.fn(),
  terminalUpdateSettings: vi.fn(),
  laneCheck: vi.fn(),
  terminalShells: vi.fn(),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => "/picked/dir") }));

import { TerminalDialog } from "../src/dialogs/TerminalDialog";
import { useCanvas } from "../src/state/canvas";
import { useWorkspaces } from "../src/state/workspaces";

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
  useCanvas.setState({ workspaceId: "w1" });
  useWorkspaces.setState({
    active: {
      id: "w1", name: "W", icon: "code", color: "#3B82F6", workingDirectory: "/projects/w",
      isPinned: false, createdAt: "", lastOpenedAt: "", camera: { x: 0, y: 0, zoom: 1 }, nodes: [], links: [],
    },
  });
  ipc.presetsDetect.mockResolvedValue([
    { id: "claude", found: true },
    { id: "codex", found: false },
    { id: "opencode", found: false },
    { id: "gemini", found: true },
    { id: "kimi", found: true },
    { id: "shell", found: true },
  ]);
  ipc.laneCheck.mockResolvedValue({ available: true, baseBranch: "main" });
  ipc.terminalShells.mockResolvedValue([]);
  ipc.rolesList.mockResolvedValue([{ id: "r1", name: "Reviewer", text: "Review diffs.", createdAt: "" }]);
  ipc.nodeCreateTerminal.mockImplementation(async (_ws, frame, opts) => ({
    id: "t1", frame, z: 1, createdAt: "", modifiedAt: "",
    content: { terminal: { name: opts.name ?? "Terminal 1", saveHistory: true, monitorActivity: true, orchestrator: false } },
  }));
});

describe("New Terminal dialog", () => {
  it("offers Kimi Code, which gets instructions at launch like Claude Code", async () => {
    render(<TerminalDialog mode={{ kind: "create", at: { x: 10, y: 20 } }} onClose={() => {}} />);
    await userEvent.click(await screen.findByRole("button", { name: /Kimi Code/ }));
    expect(screen.getByLabelText("Command")).toHaveValue("kimi");
    expect(screen.getByLabelText("Name")).toHaveValue("Kimi");
    expect(screen.getByLabelText(/Type Sonata instructions on start/)).not.toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: /Gemini CLI/ }));
    expect(screen.getByLabelText(/Type Sonata instructions on start/)).toBeChecked();
  });

  it("presets fill name and command; missing ones are marked but selectable", async () => {
    render(<TerminalDialog mode={{ kind: "create", at: { x: 10, y: 20 } }} onClose={() => {}} />);
    const codex = await screen.findByRole("button", { name: /Codex/ });
    await waitFor(() => expect(codex).toHaveTextContent(/not found/i));
    await userEvent.click(codex);
    expect(screen.getByLabelText("Name")).toHaveValue("Codex");
    expect(screen.getByLabelText("Command")).toHaveValue("codex");
    await userEvent.click(screen.getByRole("button", { name: /Shell/ }));
    expect(screen.getByLabelText("Command")).toHaveValue("");
  });

  it("creates with details, appearance, and a library role at the clicked point", async () => {
    const onClose = vi.fn();
    render(<TerminalDialog mode={{ kind: "create", at: { x: 10, y: 20 } }} onClose={onClose} />);
    await userEvent.click(await screen.findByRole("button", { name: /Claude Code/ }));
    await userEvent.click(screen.getByRole("button", { name: "Browse…" }));
    expect(screen.getByLabelText("Working folder")).toHaveValue("/picked/dir");
    await userEvent.click(screen.getByLabelText(/Orchestrator/));
    await userEvent.click(screen.getByRole("tab", { name: "Appearance" }));
    await userEvent.click(screen.getByRole("radio", { name: "#8B5CF6" }));
    await userEvent.clear(screen.getByLabelText("Font size"));
    await userEvent.type(screen.getByLabelText("Font size"), "16");
    await userEvent.click(screen.getByRole("tab", { name: "Role" }));
    await userEvent.click(await screen.findByRole("button", { name: "Use Reviewer" }));
    expect(screen.getByLabelText("Role text")).toHaveValue("Review diffs.");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() => expect(ipc.nodeCreateTerminal).toHaveBeenCalled());
    const [ws, frame, opts] = ipc.nodeCreateTerminal.mock.calls[0];
    expect(ws).toBe("w1");
    expect(frame).toMatchObject({ x: 10, y: 20 });
    expect(opts).toMatchObject({
      name: "Claude Code",
      launchCommand: "claude",
      workingDirectory: "/picked/dir",
      settings: {
        accentColor: "#8B5CF6",
        fontSize: 16,
        orchestrator: true,
        monitorActivity: true,
        role: { name: "Reviewer", text: "Review diffs." },
      },
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("saves a role to the library", async () => {
    ipc.roleSave.mockResolvedValue({ id: "r2", name: "Lead", text: "Coordinate.", createdAt: "" });
    render(<TerminalDialog mode={{ kind: "create" }} onClose={() => {}} />);
    await userEvent.click(screen.getByRole("tab", { name: "Role" }));
    await userEvent.type(screen.getByLabelText("Role text"), "Coordinate.");
    await userEvent.type(screen.getByLabelText("Role name"), "Lead");
    await userEvent.click(screen.getByRole("button", { name: "Save to library" }));
    expect(ipc.roleSave).toHaveBeenCalledWith("Lead", "Coordinate.");
    expect(await screen.findByRole("button", { name: "Use Lead" })).toBeInTheDocument();
  });

  it("edit mode pre-fills and reports that command changes apply on restart", async () => {
    const node: CanvasNode = {
      id: "t9", frame: { x: 0, y: 0, width: 600, height: 400 }, z: 1, createdAt: "", modifiedAt: "",
      content: { terminal: { name: "Lead", launchCommand: "claude", saveHistory: true, monitorActivity: true, orchestrator: true, accentColor: "#8B5CF6" } },
    };
    useCanvas.setState({ nodes: [node] });
    ipc.terminalUpdateSettings.mockResolvedValue({ node, appliesOnRestart: true });
    render(<TerminalDialog mode={{ kind: "edit", nodeId: "t9" }} onClose={() => {}} />);
    expect(screen.getByLabelText("Name")).toHaveValue("Lead");
    expect(screen.getByLabelText(/Orchestrator/)).toBeChecked();
    await userEvent.clear(screen.getByLabelText("Command"));
    await userEvent.type(screen.getByLabelText("Command"), "codex");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(ipc.terminalUpdateSettings).toHaveBeenCalledWith("t9", expect.objectContaining({ launchCommand: "codex" })));
    expect(await screen.findByRole("status")).toHaveTextContent(/restart/i);
    within(screen.getByRole("dialog"));
  });
});

describe("Run in a lane", () => {
  it("is offered with the base branch, locks the folder, and creates a lane terminal", async () => {
    render(<TerminalDialog mode={{ kind: "create", at: { x: 0, y: 0 } }} onClose={() => {}} />);
    const lane = await screen.findByRole("checkbox", { name: /Run in a lane/ });
    await waitFor(() => expect(lane).toBeEnabled());
    expect(screen.getByText(/from main/)).toBeInTheDocument();
    expect(ipc.laneCheck).toHaveBeenCalledWith("w1");
    await userEvent.click(lane);
    expect(screen.getByLabelText("Working folder")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Browse…" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(ipc.nodeCreateTerminal).toHaveBeenCalled());
    const [, , opts] = ipc.nodeCreateTerminal.mock.calls[0];
    expect(opts.lane).toBe(true);
    expect(opts.workingDirectory).toBeUndefined();
  });

  it("is disabled with the reason when the folder cannot host lanes", async () => {
    ipc.laneCheck.mockResolvedValue({ available: false, reason: "The workspace folder is not inside a git repository." });
    render(<TerminalDialog mode={{ kind: "create", at: { x: 0, y: 0 } }} onClose={() => {}} />);
    expect(await screen.findByText(/not inside a git repository/)).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /Run in a lane/ })).toBeDisabled();
  });

  it("is not shown when editing a terminal", async () => {
    const node: CanvasNode = {
      id: "t9", frame: { x: 0, y: 0, width: 640, height: 400 }, z: 1, createdAt: "", modifiedAt: "",
      content: { terminal: { name: "T", saveHistory: true } },
    };
    useCanvas.setState({ nodes: [node] });
    render(<TerminalDialog mode={{ kind: "edit", nodeId: "t9" }} onClose={() => {}} />);
    await screen.findByLabelText("Name");
    expect(screen.queryByRole("checkbox", { name: /Run in a lane/ })).not.toBeInTheDocument();
    expect(ipc.laneCheck).not.toHaveBeenCalled();
  });
});

describe("Shell picker (Windows, feature 012)", () => {
  const shells = [
    { kind: "pwsh", label: "PowerShell 7", path: "C:\\pwsh.exe", default: true },
    { kind: "cmd", label: "Command Prompt", path: "C:\\cmd.exe", default: false },
    { kind: "wsl", distro: "Ubuntu", label: "Ubuntu (WSL)", path: "C:\\wsl.exe", default: false },
  ];

  it("is hidden when the system has no shell list (macOS)", async () => {
    render(<TerminalDialog mode={{ kind: "create", at: { x: 0, y: 0 } }} onClose={() => {}} />);
    await screen.findByRole("button", { name: /Claude Code/ });
    expect(screen.queryByLabelText("Shell")).toBeNull();
  });

  it("preselects the default and omits it on create", async () => {
    ipc.terminalShells.mockResolvedValue(shells);
    render(<TerminalDialog mode={{ kind: "create", at: { x: 0, y: 0 } }} onClose={() => {}} />);
    const picker = await screen.findByLabelText("Shell");
    expect(picker).toHaveDisplayValue("PowerShell 7 (default)");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(ipc.nodeCreateTerminal).toHaveBeenCalled());
    expect(ipc.nodeCreateTerminal.mock.calls[0][2].settings.shell).toBeUndefined();
  });

  it("sends the chosen shell with its WSL distribution", async () => {
    ipc.terminalShells.mockResolvedValue(shells);
    render(<TerminalDialog mode={{ kind: "create", at: { x: 0, y: 0 } }} onClose={() => {}} />);
    await userEvent.selectOptions(await screen.findByLabelText("Shell"), "Ubuntu (WSL)");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => expect(ipc.nodeCreateTerminal).toHaveBeenCalled());
    expect(ipc.nodeCreateTerminal.mock.calls[0][2].settings.shell).toEqual({ kind: "wsl", distro: "Ubuntu" });
  });
});
