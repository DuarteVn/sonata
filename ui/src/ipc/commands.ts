import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  NoteView,
  Bootstrap,
  BusyWorkspace,
  CanvasNode,
  Camera,
  Frame,
  TerminalStateEvent,
  Workspace,
  WorkspaceSummary,
} from "./types";

// Typed wrappers for the Tauri commands in contracts/ipc.md. Argument names are camelCase;
// Tauri maps them to the Rust snake_case parameters.
export const ipc = {
  appBootstrap: () => invoke<Bootstrap>("app_bootstrap"),
  workspaceCreate: (name: string, workingDirectory: string) =>
    invoke<WorkspaceSummary>("workspace_create", { name, workingDirectory }),
  workspaceOpen: (id: string) => invoke<Workspace>("workspace_open", { id }),
  workspaceRename: (id: string, name: string) => invoke<WorkspaceSummary>("workspace_rename", { id, name }),
  workspaceSetPinned: (id: string, pinned: boolean) => invoke<void>("workspace_set_pinned", { id, pinned }),
  workspaceReorder: (orderedIds: string[]) => invoke<void>("workspace_reorder", { orderedIds }),
  workspaceDelete: (id: string) => invoke<void>("workspace_delete", { id, confirmed: true }),
  appQuitCheck: () => invoke<BusyWorkspace[]>("app_quit_check"),
  appQuit: (confirmed: boolean) => invoke<BusyWorkspace[]>("app_quit", { confirmed }),
  appRevealPath: (path: string) => invoke<void>("app_reveal_path", { path }),
  workspaceSetCamera: (id: string, camera: Camera) => invoke<Camera>("workspace_set_camera", { id, ...camera }),

  nodeCreateTerminal: (
    workspaceId: string,
    frame: Frame,
    opts: { name?: string; launchCommand?: string; workingDirectory?: string } = {},
  ) => invoke<CanvasNode>("node_create_terminal", { workspaceId, frame, ...opts }),
  nodeUpdateFrame: (workspaceId: string, nodeId: string, frame: Frame) =>
    invoke<Frame>("node_update_frame", { workspaceId, nodeId, frame }),
  nodeBringToFront: (workspaceId: string, nodeId: string) =>
    invoke<{ z: number }>("node_bring_to_front", { workspaceId, nodeId }),
  nodeDelete: (workspaceId: string, nodeId: string, confirmed: boolean) =>
    invoke<void>("node_delete", { workspaceId, nodeId, confirmed }),

  nodeCreateNote: (workspaceId: string, frame: Frame) => invoke<CanvasNode>("node_create_note", { workspaceId, frame }),
  noteRead: (nodeId: string) => invoke<NoteView>("note_read", { nodeId }),
  noteWrite: (nodeId: string, text: string, flush: boolean) => invoke<void>("note_write", { nodeId, text, flush }),

  terminalAttach: (nodeId: string, channel: Channel<ArrayBuffer>, cols: number, rows: number) =>
    invoke<TerminalStateEvent | null>("terminal_attach", { nodeId, channel, cols, rows }),
  terminalDetach: (nodeId: string) => invoke<void>("terminal_detach", { nodeId }),
  terminalWrite: (nodeId: string, data: string) => invoke<void>("terminal_write", { nodeId, data }),
  terminalResize: (nodeId: string, cols: number, rows: number) =>
    invoke<void>("terminal_resize", { nodeId, cols, rows }),
  terminalRename: (nodeId: string, name: string) => invoke<string>("terminal_rename", { nodeId, name }),
  terminalSetLaunchCommand: (nodeId: string, launchCommand: string | null) =>
    invoke<void>("terminal_set_launch_command", { nodeId, launchCommand }),
  terminalRestart: (nodeId: string) => invoke<void>("terminal_restart", { nodeId }),
  terminalSetSaveHistory: (nodeId: string, enabled: boolean) =>
    invoke<void>("terminal_set_save_history", { nodeId, enabled }),
  terminalPreviewSubscribe: (nodeIds: string[]) => invoke<void>("terminal_preview_subscribe", { nodeIds }),
  terminalClearHistory: (nodeId: string) => invoke<void>("terminal_clear_history", { nodeId }),
};

export type Ipc = typeof ipc;
