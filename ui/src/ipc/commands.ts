import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ExchangeEntry,
  LaneAvailability,
  Link,
  NoteStyle,
  PresetStatus,
  Role,
  SettingsUpdate,
  TerminalSettings,
  NoteView,
  Bootstrap,
  BusyWorkspace,
  UpdateState,
  UsageState,
  RefreshReason,
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
    opts: { name?: string; launchCommand?: string; workingDirectory?: string; settings?: TerminalSettings; lane?: boolean } = {},
  ) => invoke<CanvasNode>("node_create_terminal", { workspaceId, frame, ...opts }),
  laneCheck: (workspaceId: string) => invoke<LaneAvailability>("lane_check", { workspaceId }),
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
  terminalRestart: (nodeId: string, confirmed = false) => invoke<void>("terminal_restart", { nodeId, confirmed }),
  terminalUpdateSettings: (nodeId: string, settings: TerminalSettings) =>
    invoke<SettingsUpdate>("terminal_update_settings", { nodeId, settings }),
  nodeCreateBrowser: (workspaceId: string, frame: Frame, url?: string) =>
    invoke<CanvasNode>("node_create_browser", { workspaceId, frame, url }),
  browserNavigate: (nodeId: string, input: string) => invoke<string>("browser_navigate", { nodeId, input }),
  browserAction: (nodeId: string, action: "back" | "forward" | "reload" | "stop" | "inspect" | "external") =>
    invoke<void>("browser_action", { nodeId, action }),
  browserLayout: (items: { nodeId: string; rect: { x: number; y: number; width: number; height: number } | null }[]) =>
    invoke<void>("browser_layout", { items }),
  exchangeLog: (nodeId: string) => invoke<ExchangeEntry[]>("exchange_log", { nodeId }),
  presetsDetect: () => invoke<PresetStatus[]>("presets_detect"),
  rolesList: () => invoke<Role[]>("roles_list"),
  roleSave: (name: string, text: string) => invoke<Role>("role_save", { name, text }),
  roleDelete: (id: string) => invoke<void>("role_delete", { id }),
  linkCreate: (workspaceId: string, a: string, b: string) => invoke<Link>("link_create", { workspaceId, a, b }),
  linkDelete: (workspaceId: string, linkId: string) => invoke<void>("link_delete", { workspaceId, linkId }),
  noteSetStyle: (nodeId: string, style: NoteStyle) => invoke<void>("note_set_style", { nodeId, style }),
  noteInsertImage: (nodeId: string, sourcePath: string) =>
    invoke<{ markdown: string }>("note_insert_image", { nodeId, sourcePath }),
  terminalSetSaveHistory: (nodeId: string, enabled: boolean) =>
    invoke<void>("terminal_set_save_history", { nodeId, enabled }),
  terminalPreviewSubscribe: (nodeIds: string[]) => invoke<void>("terminal_preview_subscribe", { nodeIds }),
  terminalClearHistory: (nodeId: string) => invoke<void>("terminal_clear_history", { nodeId }),
  updateState: () => invoke<UpdateState>("update_state"),
  updateCheck: (manual: boolean) => invoke<UpdateState>("update_check", { manual }),
  updateDismiss: () => invoke<UpdateState>("update_dismiss"),
  updateDownload: () => invoke<UpdateState>("update_download"),
  updateInstall: () => invoke<void>("update_install"),
  updateSetAutoCheck: (enabled: boolean) => invoke<void>("update_set_auto_check", { enabled }),
  usageState: () => invoke<UsageState>("usage_state"),
  usageRefresh: (reason: RefreshReason, providerId?: string) =>
    invoke<UsageState>("usage_refresh", { reason, providerId: providerId ?? null }),
  usageSetEnabled: (enabled: boolean) => invoke<UsageState>("usage_set_enabled", { enabled }),
  usageSetWarnings: (enabled: boolean) => invoke<UsageState>("usage_set_warnings", { enabled }),
  usageSetProviderEnabled: (providerId: string, enabled: boolean) =>
    invoke<UsageState>("usage_set_provider_enabled", { providerId, enabled }),
  usageSetProviderColor: (providerId: string, color: string) =>
    invoke<UsageState>("usage_set_provider_color", { providerId, color }),
  usageAnswerConsent: (providerId: string, allow: boolean, command: string[]) =>
    invoke<UsageState>("usage_answer_consent", { providerId, allow, command }),
  usageOpenSettings: () => invoke<void>("usage_open_settings"),
  usageProvidersFolder: () => invoke<{ path: string }>("usage_providers_folder"),
};

export type Ipc = typeof ipc;
