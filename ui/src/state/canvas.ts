import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type { Camera, CanvasNode, Frame, TerminalStateEvent, Workspace } from "../ipc/types";

type CanvasState = {
  workspaceId: string | null;
  camera: Camera;
  nodes: CanvasNode[];
  focusedId: string | null;
  /** True while a pan/zoom gesture is in progress (live terminals use a CSS transform). */
  gesturing: boolean;
  terminalStates: Record<string, TerminalStateEvent>;
  load: (ws: Workspace | null) => void;
  setCamera: (camera: Camera) => void;
  setGesturing: (on: boolean) => void;
  setFocus: (id: string | null) => void;
  previewFrame: (id: string, frame: Frame) => void;
  commitFrame: (id: string, frame: Frame) => Promise<void>;
  bringToFront: (id: string) => Promise<void>;
  addTerminal: (frame: Frame, opts: { name?: string; launchCommand?: string }) => Promise<CanvasNode>;
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

  load: (ws) =>
    set({
      workspaceId: ws?.id ?? null,
      camera: ws?.camera ?? { x: 0, y: 0, zoom: 1 },
      nodes: ws?.nodes ?? [],
      focusedId: null,
    }),

  setCamera: (camera) => {
    set({ camera });
    const id = get().workspaceId;
    clearTimeout(cameraTimer);
    if (id) cameraTimer = setTimeout(() => void ipc.workspaceSetCamera(id, camera), CAMERA_SAVE_DELAY_MS);
  },

  setGesturing: (gesturing) => set({ gesturing }),
  setFocus: (focusedId) => set({ focusedId }),

  previewFrame: (id, frame) => set((s) => ({ nodes: replaceNode(s.nodes, id, (n) => ({ ...n, frame })) })),

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
      focusedId: s.focusedId === id ? null : s.focusedId,
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

  setTerminalState: (e) => set((s) => ({ terminalStates: { ...s.terminalStates, [e.nodeId]: e } })),
}));
