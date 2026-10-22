import { invoke } from "@tauri-apps/api/core";
import type { Bootstrap, Workspace, WorkspaceSummary } from "./types";

// Typed wrappers for the Tauri commands in contracts/ipc.md. Argument names are camelCase;
// Tauri maps them to the Rust snake_case parameters.
export const ipc = {
  appBootstrap: () => invoke<Bootstrap>("app_bootstrap"),
  workspaceCreate: (name: string, workingDirectory: string) =>
    invoke<WorkspaceSummary>("workspace_create", { name, workingDirectory }),
  workspaceOpen: (id: string) => invoke<Workspace>("workspace_open", { id }),
};

export type Ipc = typeof ipc;
