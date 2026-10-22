# Contract: Frontend ↔ Core IPC (Tauri commands, events, channels)

The frontend calls `src-tauri` commands, which forward to `core` services. `src-tauri` holds no
business rules (constitution II). Names are `snake_case` commands; payloads are `camelCase` JSON.

## Errors

Every command returns `Result<T, AppError>`:

```ts
type AppError = {
  code:
    | "NameEmpty" | "NameTooLong" | "NameTaken" | "DirectoryMissing" | "NotADirectory"
    | "WorkspaceNotFound" | "NodeNotFound" | "WorkspaceNotLoaded"
    | "TerminalBusy" | "Io" | "Internal";
  message: string;       // English, user-facing
};
```

## Commands

### App and workspaces

| Command | Input | Output | Notes |
|---|---|---|---|
| `app_bootstrap` | — | `Bootstrap` | Called once at startup. |
| `workspace_create` | `{name, workingDirectory}` | `WorkspaceSummary` | FR-001–003. |
| `workspace_rename` | `{id, name}` | `WorkspaceSummary` | FR-002 rules. |
| `workspace_set_pinned` | `{id, pinned}` | — | FR-004. |
| `workspace_reorder` | `{orderedIds}` | — | FR-004. |
| `workspace_open` | `{id}` | `WorkspaceView` | Loads, starts terminals (first open), sets active. |
| `workspace_delete` | `{id, confirmed: true}` | — | UI shows confirmation first (FR-006). |
| `workspace_set_camera` | `{id, x, y, zoom}` | — | Debounced by UI (≥ 200 ms). |

```ts
type Bootstrap = {
  workspaces: WorkspaceSummary[];        // in sidebar order, pinned first
  loadFailures: LoadFailure[];           // FR-025
  activeWorkspaceId: string | null;      // null when crash guard tripped
  crashGuardTripped: boolean;            // FR-027 message
  recentWorkspaceIds: string[];
  preferences: Preferences;
};
type WorkspaceSummary = {
  id: string; name: string; icon: string; color: string;
  workingDirectory: string; workingDirectoryMissing: boolean;
  isPinned: boolean; isLoaded: boolean; busyTerminalCount: number;
};
type WorkspaceView = WorkspaceSummary & { camera: Camera; nodes: NodeView[] };
```

### Nodes

| Command | Input | Output | Notes |
|---|---|---|---|
| `node_create_terminal` | `{workspaceId, frame, name?, launchCommand?, workingDirectory?}` | `NodeView` | Starts the shell. |
| `node_create_note` | `{workspaceId, frame}` | `NodeView` | Creates an empty `.md`. |
| `node_update_frame` | `{workspaceId, nodeId, frame}` | — | Clamped to minimum size. |
| `node_bring_to_front` | `{workspaceId, nodeId}` | `{z}` | |
| `node_delete` | `{workspaceId, nodeId, confirmed}` | — | `TerminalBusy` if busy and not confirmed (FR-018). |

### Terminals

| Command | Input | Output | Notes |
|---|---|---|---|
| `terminal_attach` | `{nodeId, channel: Channel<bytes>, cols, rows}` | — | Sends replay, then live bytes on `channel`. |
| `terminal_detach` | `{nodeId}` | — | Process keeps running (FR-013). |
| `terminal_write` | `{nodeId, data: bytes}` | — | Keyboard input. |
| `terminal_resize` | `{nodeId, cols, rows}` | — | Only when node size changes, not on zoom (research R2). |
| `terminal_rename` | `{nodeId, name}` | — | |
| `terminal_set_launch_command` | `{nodeId, launchCommand?}` | — | |
| `terminal_set_save_history` | `{nodeId, enabled}` | — | FR-015a. |
| `terminal_clear_history` | `{nodeId}` | — | Deletes history file (FR-015a). |
| `terminal_restart` | `{nodeId}` | — | From `Exited` (FR-017). |
| `terminal_preview_subscribe` | `{nodeIds}` | — | Replaces the set of previews the UI wants (research R3). |

### Notes

| Command | Input | Output |
|---|---|---|
| `note_read` | `{nodeId}` | `{text, sync, conflictPath?}` |
| `note_write` | `{nodeId, text, flush}` | — (`flush: false` marks dirty for autosave; `flush: true` writes now, used on blur) |

### Quit

| Command | Input | Output |
|---|---|---|
| `app_reveal_path` | `{path}` | — (Finder; only paths inside the data folder) |
| `app_quit_check` | — | `{busy: {workspaceId, workspaceName, count}[]}` (FR-018a) |
| `app_quit` | `{confirmed}` | `BusyWorkspace[]`: without `confirmed`, returns the busy list and does not quit; otherwise saves all, writes `cleanShutdown`, stops terminals, exits |

## Events (core → frontend)

| Event | Payload |
|---|---|
| `workspaces://changed` | `WorkspaceSummary[]` |
| `terminal://state` | `{nodeId, run: "starting" \| "running" \| "exited", exitCode?, busy}` |
| `terminal://preview` | `{nodeId, lines: string[], cursor?: [row, col]}` (≤ 1 Hz per node) |
| `note://changed` | `{nodeId, text, sync}` (external edit reloaded) |
| `note://conflict` | `{nodeId, conflictPath}` |
| `persistence://warning` | `{message}` (save failures, permission fixes) |
| `app://quit-requested` | `null` (Cmd+Q or window close; the UI runs the quit check) |

## Channels

`terminal_attach` takes a `tauri::ipc::Channel` that carries raw PTY output bytes. The first
message is the replay (research R1). Messages are batched every 8 ms or 32 KB, whichever comes
first.
