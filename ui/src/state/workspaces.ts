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
  setSummaries: (summaries: WorkspaceSummary[]) => void;
  addWarning: (message: string) => void;
  dismissWarning: (index: number) => void;
};

let initStarted = false;

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

  setSummaries: (workspaces) => set({ workspaces }),
  addWarning: (message) => set((s) => ({ warnings: [...s.warnings, message] })),
  dismissWarning: (index) => set((s) => ({ warnings: s.warnings.filter((_, i) => i !== index) })),
}));
