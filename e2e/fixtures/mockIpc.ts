// In-memory fake of the Sonata core for Playwright runs against the Vite dev server.
// Loaded by ui/src/main.tsx only in dev builds when the URL contains `?mock`.
// Every call is recorded in `window.__sonataMock.calls` for assertions.
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Bootstrap, CanvasNode, Link, Role, Workspace, WorkspaceSummary } from "../../ui/src/ipc/types";

type Call = { cmd: string; args: Record<string, unknown> };

export type MockState = {
  calls: Call[];
  /** What `app_quit_check` returns. */
  busy: unknown[];
  emit: (name: string, payload: unknown) => Promise<void>;
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
  const state: MockState = { calls: [], busy: [], emit: (n, p) => emit(n, p), workspaces: [], failNext: {} };
  (window as unknown as { __sonataMock: MockState }).__sonataMock = state;
  let seq = 0;
  const id = () => `id-${++seq}`;
  const noteTexts: Record<string, string> = {};
  let roles: Role[] = [];
  let seededActive: string | null = null;
  const find = (wsId: unknown) => state.workspaces.find((w) => w.id === wsId);

  mockIPC(
    (cmd, payload) => {
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
          activeWorkspaceId: seededActive,
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
          links: [],
        };
        state.workspaces.push(ws);
        return summary(ws);
      }
      case "workspace_open": {
        const ws = find(args.id);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        return structuredClone(ws);
      }
      case "workspace_rename": {
        const ws = find(args.id);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        ws.name = String(args.name).trim();
        return summary(ws);
      }
      case "workspace_set_pinned": {
        const ws = find(args.id);
        if (ws) ws.isPinned = Boolean(args.pinned);
        return null;
      }
      case "workspace_reorder": {
        const order = args.orderedIds as string[];
        state.workspaces.sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id));
        return null;
      }
      case "workspace_delete":
        state.workspaces = state.workspaces.filter((w) => w.id !== args.id);
        return null;
      case "app_quit_check":
        return state.busy;
      case "app_quit":
        return args.confirmed || state.busy.length === 0 ? [] : state.busy;
      case "workspace_set_camera": {
        const ws = find(args.id);
        if (ws) ws.camera = { x: Number(args.x), y: Number(args.y), zoom: Number(args.zoom) };
        return ws?.camera ?? null;
      }
      case "node_create_terminal": {
        const ws = find(args.workspaceId);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        const now = new Date().toISOString();
        const count = ws.nodes.filter((n) => "terminal" in n.content).length;
        const frame = args.frame as CanvasNode["frame"];
        const node: CanvasNode = {
          id: id(),
          frame: { ...frame, width: Math.max(240, frame.width), height: Math.max(120, frame.height) },
          z: Math.max(0, ...ws.nodes.map((n) => n.z)) + 1,
          createdAt: now,
          modifiedAt: now,
          content: {
            terminal: {
              name: (args.name as string) || `Terminal ${count + 1}`,
              launchCommand: (args.launchCommand as string) || undefined,
              saveHistory: true,
              monitorActivity: true,
              orchestrator: false,
              ...Object.fromEntries(
                Object.entries((args.settings as Record<string, unknown>) ?? {}).filter(([, v]) => v !== null && v !== undefined),
              ),
            },
          },
        };
        ws.nodes.push(node);
        return node;
      }
      case "node_update_frame": {
        const node = find(args.workspaceId)?.nodes.find((n) => n.id === args.nodeId);
        if (node) node.frame = args.frame as CanvasNode["frame"];
        return node?.frame ?? null;
      }
      case "node_bring_to_front": {
        const ws = find(args.workspaceId);
        const node = ws?.nodes.find((n) => n.id === args.nodeId);
        if (!ws || !node) return null;
        node.z = Math.max(...ws.nodes.map((n) => n.z)) + 1;
        return { z: node.z };
      }
      case "node_delete": {
        const ws = find(args.workspaceId);
        if (ws) {
          ws.nodes = ws.nodes.filter((n) => n.id !== args.nodeId);
          ws.links = ws.links.filter((l) => l.a !== args.nodeId && l.b !== args.nodeId);
        }
        return null;
      }
      case "node_create_note": {
        const ws = find(args.workspaceId);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        const now = new Date().toISOString();
        const frame = args.frame as CanvasNode["frame"];
        const node: CanvasNode = {
          id: id(),
          frame: { ...frame, width: Math.max(120, frame.width), height: Math.max(80, frame.height) },
          z: Math.max(0, ...ws.nodes.map((n) => n.z)) + 1,
          createdAt: now,
          modifiedAt: now,
          content: { note: { color: "yellow", textSize: 3 } },
        };
        ws.nodes.push(node);
        noteTexts[node.id] = "";
        return node;
      }
      case "note_read":
        return { text: noteTexts[String(args.nodeId)] ?? "", sync: "clean", assetBase: "/mock/notes" };
      case "note_write":
        noteTexts[String(args.nodeId)] = String(args.text);
        return null;
      case "link_create": {
        const ws = find(args.workspaceId);
        const [a, b] = [String(args.a), String(args.b)];
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        if (a === b) throw { code: "LinkSelf", message: "A node cannot be linked to itself." };
        if (ws.links.some((l) => (l.a === a && l.b === b) || (l.a === b && l.b === a)))
          throw { code: "LinkExists", message: "These two are already linked." };
        const link: Link = { id: id(), a: a < b ? a : b, b: a < b ? b : a, createdAt: new Date().toISOString() };
        ws.links.push(link);
        return link;
      }
      case "link_delete": {
        const ws = find(args.workspaceId);
        if (ws) ws.links = ws.links.filter((l) => l.id !== args.linkId);
        return null;
      }
      case "presets_detect":
        return ["claude", "codex", "opencode", "gemini", "shell"].map((p) => ({ id: p, found: p !== "opencode" }));
      case "roles_list":
        return roles;
      case "role_save": {
        const role: Role = { id: id(), name: String(args.name), text: String(args.text), createdAt: "" };
        roles.push(role);
        return role;
      }
      case "role_delete":
        roles = roles.filter((r) => r.id !== args.id);
        return null;
      case "terminal_update_settings": {
        const node = state.workspaces.flatMap((w) => w.nodes).find((n) => n.id === args.nodeId);
        if (!node || !("terminal" in node.content)) throw { code: "NodeNotFound", message: "No node." };
        const s = args.settings as Record<string, unknown>;
        const t = node.content.terminal as Record<string, unknown>;
        for (const [k, v] of Object.entries(s)) if (v !== undefined) t[k] = v === null ? undefined : v;
        return { node, appliesOnRestart: "launchCommand" in s };
      }
      case "terminal_restart":
        return null;
      case "note_set_style": {
        const node = state.workspaces.flatMap((w) => w.nodes).find((n) => n.id === args.nodeId);
        if (node) node.content = { note: args.style as never };
        return null;
      }
      case "note_insert_image":
        return { markdown: "![](../files/mock.png)" };
      case "node_create_browser": {
        const ws = find(args.workspaceId);
        if (!ws) throw { code: "WorkspaceNotFound", message: "No workspace." };
        const now = new Date().toISOString();
        const node: CanvasNode = {
          id: id(),
          frame: args.frame as CanvasNode["frame"],
          z: Math.max(0, ...ws.nodes.map((n) => n.z)) + 1,
          createdAt: now,
          modifiedAt: now,
          content: { browser: { url: "about:blank" } },
        };
        ws.nodes.push(node);
        return node;
      }
      case "browser_navigate":
        return String(args.input).startsWith("localhost") ? `http://${args.input}` : `https://${args.input}`;
      case "browser_action":
      case "browser_layout":
        return null;
      case "exchange_log":
        return [];
      case "terminal_rename":
        return String(args.name).trim();
      default:
        return null;
    }
    },
    { shouldMockEvents: true },
  );
  // `?mock&seed=N`: start with one open workspace holding N terminals in a grid.
  const seed = Number(new URLSearchParams(location.search).get("seed") ?? 0);
  if (seed > 0) {
    const now = new Date().toISOString();
    const nodes: CanvasNode[] = Array.from({ length: seed }, (_, i) => ({
      id: `seed-${i}`,
      frame: { x: (i % 6) * 680, y: Math.floor(i / 6) * 440, width: 640, height: 400 },
      z: i + 1,
      createdAt: now,
      modifiedAt: now,
      content: { terminal: { name: `Agent ${i + 1}`, saveHistory: true } },
    }));
    state.workspaces.push({
      id: "seeded",
      name: "Seeded",
      icon: "bot",
      color: colors[0],
      workingDirectory: "/projects/seeded",
      isPinned: false,
      createdAt: now,
      lastOpenedAt: now,
      camera: { x: 0, y: 0, zoom: 1 },
      nodes,
      links: [],
    });
    seededActive = "seeded";
  }
  return state;
}

export type { CanvasNode };
