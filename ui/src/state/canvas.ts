import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type {
  AskEvent,
  Camera,
  CanvasNode,
  Frame,
  LaneAction,
  LanePreview,
  Link,
  NoteStyle,
  NoteSync,
  TerminalSettings,
  TerminalStateEvent,
  Workspace,
} from "../ipc/types";

export type Tool = "select" | "terminal" | "note" | "browser";
export type Selection = { kind: "node" | "link"; id: string } | null;
/** A land or remove confirmation (feature 008); `request` is set when an agent asked for it. */
export type LaneDialogState = {
  nodeId: string;
  action: LaneAction;
  request?: { requestId: string; requestedBy: string; preview: LanePreview; deleteBranch: boolean };
};

export type TerminalDialogMode = { kind: "create"; at?: { x: number; y: number } } | { kind: "edit"; nodeId: string };

export type NoteState = { text: string; sync: NoteSync; conflictPath?: string; assetBase?: string; loaded: boolean };

type CanvasState = {
  workspaceId: string | null;
  camera: Camera;
  nodes: CanvasNode[];
  focusedId: string | null;
  /** True while a pan/zoom gesture is in progress (live terminals use a CSS transform). */
  gesturing: boolean;
  terminalStates: Record<string, TerminalStateEvent>;
  tool: Tool;
  selection: Selection;
  /** Nodes selected together (two or more), from a selection rectangle or Shift+click. */
  group: string[];
  setGroup: (ids: string[]) => void;
  links: Link[];
  attention: Set<string>;
  rawNotes: Set<string>;
  /** Source node of a link started from a toolbar; the next node click completes it. */
  linkingFrom: string | null;
  terminalDialog: TerminalDialogMode | null;
  setTool: (tool: Tool) => void;
  select: (selection: Selection) => void;
  createLink: (a: string, b: string) => Promise<void>;
  deleteLink: (id: string) => Promise<void>;
  startLinking: (from: string) => void;
  cancelLinking: () => void;
  toggleRaw: (id: string) => void;
  openTerminalDialog: (mode: TerminalDialogMode | null) => void;
  laneDialog: LaneDialogState | null;
  openLaneDialog: (state: LaneDialogState | null) => void;
  replaceNode: (node: CanvasNode) => void;
  setNoteStyle: (id: string, style: NoteStyle) => Promise<void>;
  /** Terminal waiting for "close anyway?" confirmation. */
  confirmClose: string | null;
  /**
   * A press on a node: completes a pending link, else selects (and focuses terminals). With
   * `additive` (Shift), adds the node to the group or removes it. Pressing a grouped node keeps
   * the group so it can be dragged together.
   */
  pressNode: (id: string, kind: "terminal" | "note", additive?: boolean) => boolean;
  /** Deletes the selected node, group, or link (Delete key, Cmd+W, toolbar). */
  deleteSelection: () => Promise<void>;
  setConfirmClose: (id: string | null) => void;
  /** Terminals of a group deletion still running a program, waiting for confirmation. */
  confirmCloseMany: string[] | null;
  setConfirmCloseMany: (ids: string[] | null) => void;
  /** Asks in flight (queued or waiting), by ask id (003 FR-027). */
  asks: Record<string, { from: string; to: string }>;
  browserStates: Record<string, import("../ipc/types").BrowserState>;
  setBrowserState: (s: import("../ipc/types").BrowserState) => void;
  addBrowser: (frame: Frame) => Promise<CanvasNode>;
  setAsk: (e: AskEvent) => void;
  applyNodesChanged: (workspaceId: string, nodes: CanvasNode[], links: Link[]) => void;
  notes: Record<string, NoteState>;
  previews: Record<string, string[]>;
  liveIds: Set<string>;
  webglLost: Set<string>;
  setPreview: (id: string, lines: string[]) => void;
  setLiveIds: (ids: Set<string>) => void;
  markWebglLost: (id: string) => void;
  addNote: (frame: Frame) => Promise<CanvasNode>;
  loadNote: (id: string) => Promise<void>;
  setNoteText: (id: string, text: string, sync: NoteSync) => void;
  setNoteConflict: (id: string, conflictPath: string) => void;
  load: (ws: Workspace | null) => void;
  setCamera: (camera: Camera) => void;
  setGesturing: (on: boolean) => void;
  setFocus: (id: string | null) => void;
  previewFrame: (id: string, frame: Frame) => void;
  previewFrames: (frames: Record<string, Frame>) => void;
  commitFrame: (id: string, frame: Frame) => Promise<void>;
  bringToFront: (id: string) => Promise<void>;
  addTerminal: (
    frame: Frame,
    opts: { name?: string; launchCommand?: string; workingDirectory?: string; settings?: TerminalSettings; lane?: boolean },
  ) => Promise<CanvasNode>;
  removeNode: (id: string, confirmed: boolean) => Promise<void>;
  renameTerminal: (id: string, name: string) => Promise<void>;
  setTerminalState: (e: TerminalStateEvent) => void;
};

const CAMERA_SAVE_DELAY_MS = 250;
let cameraTimer: ReturnType<typeof setTimeout> | undefined;

function replaceNode(nodes: CanvasNode[], id: string, patch: (n: CanvasNode) => CanvasNode) {
  return nodes.map((n) => (n.id === id ? patch(n) : n));
}

export const useCanvas = create<CanvasState>((set, get) => ({
  workspaceId: null,
  camera: { x: 0, y: 0, zoom: 1 },
  nodes: [],
  focusedId: null,
  gesturing: false,
  terminalStates: {},
  notes: {},
  previews: {},
  liveIds: new Set(),
  webglLost: new Set(),

  setPreview: (id, lines) => set((s) => ({ previews: { ...s.previews, [id]: lines } })),
  setLiveIds: (ids) => {
    const cur = get().liveIds;
    if (cur.size === ids.size && [...ids].every((id) => cur.has(id))) return;
    set({ liveIds: ids });
  },
  markWebglLost: (id) => set((s) => ({ webglLost: new Set([...s.webglLost, id]) })),

  addNote: async (frame) => {
    const ws = get().workspaceId;
    if (!ws) throw new Error("No workspace open.");
    const node = await ipc.nodeCreateNote(ws, frame);
    set((s) => ({ nodes: [...s.nodes, node], notes: { ...s.notes, [node.id]: { text: "", sync: "clean", loaded: true } } }));
    return node;
  },

  loadNote: async (id) => {
    const view = await ipc.noteRead(id);
    set((s) => ({ notes: { ...s.notes, [id]: { ...view, loaded: true } } }));
  },

  setNoteText: (id, text, sync) =>
    set((s) => ({ notes: { ...s.notes, [id]: { ...s.notes[id], text, sync, loaded: true } } })),

  setNoteConflict: (id, conflictPath) =>
    set((s) => ({ notes: { ...s.notes, [id]: { ...s.notes[id], sync: "conflict", conflictPath, loaded: true } } })),

  load: (ws) =>
    set({
      workspaceId: ws?.id ?? null,
      camera: ws?.camera ?? { x: 0, y: 0, zoom: 1 },
      nodes: ws?.nodes ?? [],
      links: ws?.links ?? [],
      focusedId: null,
      selection: null,
      group: [],
      linkingFrom: null,
      tool: "select",
    }),

  tool: "select",
  selection: null,
  links: [],
  attention: new Set(),
  rawNotes: new Set(),
  linkingFrom: null,
  terminalDialog: null,
  laneDialog: null,

  group: [],

  setTool: (tool) => set({ tool }),
  select: (selection) => set({ selection, group: [] }),
  setGroup: (ids) => {
    if (ids.length === 1) set({ selection: { kind: "node", id: ids[0] }, group: [], focusedId: null });
    else set({ selection: null, group: ids, focusedId: ids.length > 0 ? null : get().focusedId });
  },

  createLink: async (a, b) => {
    const ws = get().workspaceId;
    if (!ws || a === b) return;
    const link = await ipc.linkCreate(ws, a, b);
    set((s) => ({ links: [...s.links, link], linkingFrom: null }));
  },

  deleteLink: async (id) => {
    const ws = get().workspaceId;
    if (!ws) return;
    await ipc.linkDelete(ws, id);
    set((s) => ({
      links: s.links.filter((l) => l.id !== id),
      selection: s.selection?.kind === "link" && s.selection.id === id ? null : s.selection,
    }));
  },

  startLinking: (from) => set({ linkingFrom: from }),
  cancelLinking: () => set({ linkingFrom: null }),
  toggleRaw: (id) =>
    set((s) => {
      const next = new Set(s.rawNotes);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { rawNotes: next };
    }),
  openTerminalDialog: (terminalDialog) => set({ terminalDialog }),
  openLaneDialog: (laneDialog) => set({ laneDialog }),
  confirmClose: null,
  asks: {},
  browserStates: {},
  setBrowserState: (b) => set((s) => ({ browserStates: { ...s.browserStates, [b.nodeId]: b } })),
  addBrowser: async (frame) => {
    const ws = get().workspaceId;
    if (!ws) throw new Error("No workspace open.");
    const node = await ipc.nodeCreateBrowser(ws, frame);
    set((s) => ({ nodes: [...s.nodes, node], selection: { kind: "node", id: node.id }, focusedId: null }));
    return node;
  },
  setAsk: (e) =>
    set((s) => {
      const asks = { ...s.asks };
      if (e.state === "queued" || e.state === "waiting") asks[e.askId] = { from: e.from, to: e.to };
      else delete asks[e.askId];
      return { asks };
    }),
  applyNodesChanged: (workspaceId, nodes, links) => {
    if (get().workspaceId !== workspaceId) return;
    const ids = new Set(nodes.map((n) => n.id));
    set((s) => ({
      nodes,
      links,
      selection: s.selection && s.selection.kind === "node" && !ids.has(s.selection.id) ? null : s.selection,
      group: s.group.filter((id) => ids.has(id)),
    }));
  },
  setConfirmClose: (confirmClose) => set({ confirmClose }),
  confirmCloseMany: null,
  setConfirmCloseMany: (confirmCloseMany) => set({ confirmCloseMany }),

  pressNode: (id, kind, additive = false) => {
    const { linkingFrom, createLink, select, setFocus, group, selection, setGroup } = get();
    if (linkingFrom && linkingFrom !== id) {
      void createLink(linkingFrom, id).catch(() => set({ linkingFrom: null }));
      return true;
    }
    if (additive) {
      const current = group.length > 0 ? group : selection?.kind === "node" ? [selection.id] : [];
      setGroup(current.includes(id) ? current.filter((g) => g !== id) : [...current, id]);
      return true;
    }
    if (group.includes(id)) return false;
    select({ kind: "node", id });
    setFocus(kind === "terminal" ? id : null);
    return false;
  },

  deleteSelection: async () => {
    const group = get().group;
    if (group.length > 0) {
      const busy: string[] = [];
      for (const id of group) {
        const node = get().nodes.find((n) => n.id === id);
        if (!node) continue;
        try {
          await get().removeNode(id, "note" in node.content);
        } catch (err) {
          if ((err as { code?: string })?.code === "TerminalBusy") busy.push(id);
          else throw err;
        }
      }
      if (busy.length > 0) set({ group: busy, confirmCloseMany: busy });
      return;
    }
    // Cmd+W also closes the focused terminal when nothing is selected.
    const focused = get().focusedId;
    const sel = get().selection ?? (focused ? { kind: "node" as const, id: focused } : null);
    if (!sel) return;
    if (sel.kind === "link") return get().deleteLink(sel.id);
    const node = get().nodes.find((n) => n.id === sel.id);
    if (!node) return;
    if ("note" in node.content) return get().removeNode(sel.id, true);
    try {
      await get().removeNode(sel.id, false);
    } catch (err) {
      if ((err as { code?: string })?.code === "TerminalBusy") set({ confirmClose: sel.id });
      else throw err;
    }
  },
  replaceNode: (node) => set((s) => ({ nodes: replaceNode(s.nodes, node.id, () => node) })),

  setNoteStyle: async (id, style) => {
    set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, content: { note: { ...style } } })) }));
    await ipc.noteSetStyle(id, style);
  },

  setCamera: (camera) => {
    set({ camera });
    const id = get().workspaceId;
    clearTimeout(cameraTimer);
    if (id) cameraTimer = setTimeout(() => void ipc.workspaceSetCamera(id, camera), CAMERA_SAVE_DELAY_MS);
  },

  setGesturing: (gesturing) => set({ gesturing }),
  setFocus: (focusedId) =>
    set((s) => {
      if (!focusedId || !s.attention.has(focusedId)) return { focusedId };
      const attention = new Set(s.attention);
      attention.delete(focusedId);
      return { focusedId, attention };
    }),

  previewFrame: (id, frame) => set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, frame })) })),
  previewFrames: (frames) => set((s) => ({ nodes: s.nodes.map((n) => (frames[n.id] ? { ...n, frame: frames[n.id] } : n)) })),

  commitFrame: async (id, frame) => {
    const ws = get().workspaceId;
    if (!ws) return;
    const saved = await ipc.nodeUpdateFrame(ws, id, frame);
    set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, frame: saved ?? frame })) }));
  },

  bringToFront: async (id) => {
    const ws = get().workspaceId;
    const nodes = get().nodes;
    const max = Math.max(0, ...nodes.map((n) => n.z));
    if (!ws || nodes.find((n) => n.id === id)?.z === max) return;
    set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, z: max + 1 })) }));
    const res = await ipc.nodeBringToFront(ws, id);
    if (res) set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, z: res.z })) }));
  },

  addTerminal: async (frame, opts) => {
    const ws = get().workspaceId;
    if (!ws) throw new Error("No workspace open.");
    const node = await ipc.nodeCreateTerminal(ws, frame, opts);
    set((s) => ({ nodes: [...s.nodes, node], focusedId: node.id }));
    return node;
  },

  removeNode: async (id, confirmed) => {
    const ws = get().workspaceId;
    if (!ws) return;
    await ipc.nodeDelete(ws, id, confirmed);
    set((s) => ({
      nodes: s.nodes.filter((n) => n.id !== id),
      links: s.links.filter((l) => l.a !== id && l.b !== id),
      focusedId: s.focusedId === id ? null : s.focusedId,
      selection: s.selection?.id === id ? null : s.selection,
      group: s.group.filter((g) => g !== id),
    }));
  },

  renameTerminal: async (id, name) => {
    const saved = await ipc.terminalRename(id, name);
    set((s) => ({
      nodes: replaceNode(s.nodes, id, (n) =>
        "terminal" in n.content ? { ...n, content: { terminal: { ...n.content.terminal, name: saved ?? name } } } : n,
      ),
    }));
  },

  // A busy terminal that becomes idle while not focused asks for attention (FR-013).
  setTerminalState: (e) =>
    set((s) => {
      const prev = s.terminalStates[e.nodeId];
      const finished = prev?.busy === true && !e.busy && s.focusedId !== e.nodeId;
      const attention = finished ? new Set([...s.attention, e.nodeId]) : s.attention;
      return { terminalStates: { ...s.terminalStates, [e.nodeId]: e }, attention };
    }),
}));
