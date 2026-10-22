import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type { LoadFailure, Preferences, Workspace, WorkspaceSummary } from "../ipc/types";

type WorkspacesState = {
  ready: boolean;
  workspaces: WorkspaceSummary[];
  loadFailures: LoadFailure[];
  activeId: string | null;
  active: Workspace | null;
  crashGuardTripped: boolean;
  preferences: Preferences;
  warnings: string[];
  init: () => Promise<void>;
  create: (name: string, workingDirectory: string) => Promise<WorkspaceSummary>;
  open: (id: string) => Promise<void>;
  rename: (id: string, name: string) => Promise<void>;
  setPinned: (id: string, pinned: boolean) => Promise<void>;
  reorder: (orderedIds: string[]) => Promise<void>;
  remove: (id: string) => Promise<void>;
  setSummaries: (summaries: WorkspaceSummary[]) => void;
  addWarning: (message: string) => void;
  dismissWarning: (index: number) => void;
};

let initStarted = false;

/** Pinned workspaces first, otherwise keeping the current order. */
function sortPinned(list: WorkspaceSummary[]) {
  return [...list.filter((w) => w.isPinned), ...list.filter((w) => !w.isPinned)];
}

/** Test helper: allows `init` to run again. */
export function resetInitForTests() {
  initStarted = false;
}

export const defaultPreferences: Preferences = {
  autosaveIntervalSeconds: 30,
  terminalFontSize: 13,
  historyMaxRows: 10000,
};

export const useWorkspaces = create<WorkspacesState>((set, get) => ({
  ready: false,
  workspaces: [],
  loadFailures: [],
  activeId: null,
  active: null,
  crashGuardTripped: false,
  preferences: defaultPreferences,
  warnings: [],

  init: async () => {
    // React StrictMode runs effects twice in development; bootstrap only once.
    if (initStarted) return;
    initStarted = true;
    const boot = await ipc.appBootstrap();
    set({
      ready: true,
      workspaces: boot.workspaces,
      loadFailures: boot.loadFailures,
      crashGuardTripped: boot.crashGuardTripped,
      preferences: boot.preferences,
      warnings: boot.warnings,
    });
    if (boot.activeWorkspaceId) await get().open(boot.activeWorkspaceId);
  },

  create: async (name, workingDirectory) => {
    const summary = await ipc.workspaceCreate(name, workingDirectory);
    set((s) => ({
      workspaces: s.workspaces.some((w) => w.id === summary.id)
        ? s.workspaces
        : [...s.workspaces, summary],
    }));
    return summary;
  },

  open: async (id) => {
    const ws = await ipc.workspaceOpen(id);
    set({ activeId: id, active: ws });
  },

  rename: async (id, name) => {
    const summary = await ipc.workspaceRename(id, name);
    set((s) => ({ workspaces: s.workspaces.map((w) => (w.id === id ? summary : w)) }));
  },

  setPinned: async (id, pinned) => {
    await ipc.workspaceSetPinned(id, pinned);
    set((s) => ({ workspaces: sortPinned(s.workspaces.map((w) => (w.id === id ? { ...w, isPinned: pinned } : w))) }));
  },

  reorder: async (orderedIds) => {
    set((s) => ({
      workspaces: [...s.workspaces].sort((a, b) => orderedIds.indexOf(a.id) - orderedIds.indexOf(b.id)),
    }));
    await ipc.workspaceReorder(orderedIds);
  },

  remove: async (id) => {
    await ipc.workspaceDelete(id);
    set((s) => ({
      workspaces: s.workspaces.filter((w) => w.id !== id),
      ...(s.activeId === id ? { activeId: null, active: null } : {}),
    }));
  },

  setSummaries: (workspaces) => set({ workspaces }),
  addWarning: (message) => set((s) => ({ warnings: [...s.warnings, message] })),
  dismissWarning: (index) => set((s) => ({ warnings: s.warnings.filter((_, i) => i !== index) })),
}));
