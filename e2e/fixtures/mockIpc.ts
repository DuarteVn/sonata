// In-memory fake of the Sonata core for Playwright runs against the Vite dev server.
// Loaded by ui/src/main.tsx only in dev builds when the URL contains `?mock`.
// Every call is recorded in `window.__sonataMock.calls` for assertions.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Bootstrap, CanvasNode, Workspace, WorkspaceSummary } from "../../ui/src/ipc/types";

type Call = { cmd: string; args: Record<string, unknown> };

export type MockState = {
  calls: Call[];
  workspaces: Workspace[];
  failNext: Record<string, { code: string; message: string }>;
};

const colors = ["#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#14B8A6", "#F97316"];

function summary(ws: Workspace): WorkspaceSummary {
  return {
    id: ws.id,
    name: ws.name,
    icon: ws.icon,
    color: ws.color,
    workingDirectory: ws.workingDirectory,
    workingDirectoryMissing: false,
    isPinned: ws.isPinned,
    isLoaded: true,
    busyTerminalCount: 0,
  };
}

export function installMockIpc() {
  const state: MockState = { calls: [], workspaces: [], failNext: {} };
  (window as unknown as { __sonataMock: MockState }).__sonataMock = state;
  let seq = 0;
  const id = () => `id-${++seq}`;
  const find = (wsId: unknown) => state.workspaces.find((w) => w.id === wsId);

  mockIPC((cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    state.calls.push({ cmd, args });
    const failure = state.failNext[cmd];
    if (failure) {
      delete state.failNext[cmd];
      throw failure;
    }
    switch (cmd) {
      case "app_bootstrap": {
        const boot: Bootstrap = {
          workspaces: state.workspaces.map(summary),
          loadFailures: [],
          activeWorkspaceId: null,
          crashGuardTripped: false,
          recentWorkspaceIds: [],
          preferences: { autosaveIntervalSeconds: 30, terminalFontSize: 13, historyMaxRows: 10000 },
          warnings: [],
        };
        return boot;
      }
      case "workspace_create": {
        const name = String(args.name ?? "").trim();
        if (!name) throw { code: "NameEmpty", message: "Enter a workspace name." };
        if (state.workspaces.some((w) => w.name.toLowerCase() === name.toLowerCase()))
          throw { code: "NameTaken", message: `A workspace named "${name}" already exists.` };
        if (!String(args.workingDirectory ?? "").startsWith("/"))
          throw { code: "DirectoryMissing", message: `The folder ${args.workingDirectory} does not exist.` };
        const now = new Date().toISOString();
        const ws: Workspace = {
          id: id(),
          name,
          icon: "rocket",
          color: colors[state.workspaces.length % colors.length],
          workingDirectory: String(args.workingDirectory),
          isPinned: false,
          createdAt: now,
          lastOpenedAt: now,
          camera: { x: 0, y: 0, zoom: 1 },
          nodes: [],
        };
        state.workspaces.push(ws);
        return summary(ws);
      }
      case "workspace_open": {
        const ws = find(args.id);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        return structuredClone(ws);
      }
      default:
        return null;
    }
  });
  return state;
}

export type { CanvasNode };
