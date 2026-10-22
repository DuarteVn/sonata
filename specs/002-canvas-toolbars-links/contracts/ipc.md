# Contract: IPC Changes for Feature 002

Extends `specs/001-workspaces-canvas-terminals/contracts/ipc.md`. New error codes: `LinkSelf`,
`LinkExists`, `RoleNameTaken`, `ImageTooLarge`, `ImageUnsupported`.

## Changed

| Command | Change |
|---|---|
| `workspace_open` | `WorkspaceView` gains `links: Link[]`. |
| `node_create_terminal` | Accepts `settings?: TerminalSettings` (below). |
| `node_delete` | Also deletes the node's links; returns nothing new. |
| `terminal_restart` | Takes `confirmed: bool`; busy and not confirmed → `TerminalBusy`. |
| `note_read` | Returns `assetBase` (absolute `notes/` folder) besides `text`, `sync`. |

```ts
type TerminalSettings = {
  name?: string;
  launchCommand?: string | null;
  workingDirectory?: string | null;
  accentColor?: string | null;
  fontSize?: number | null;
  role?: { name?: string; text: string } | null;
  monitorActivity?: boolean;
  orchestrator?: boolean;
};
type Link = { id: string; a: string; b: string; createdAt: string };
type NoteStyle = { color: "yellow" | "blue" | "green" | "pink" | "purple" | "gray"; textSize: 1 | 2 | 3 | 4 | 5 };
```

## New

| Command | Input | Output | Notes |
|---|---|---|---|
| `terminal_update_settings` | `{nodeId, settings: TerminalSettings}` | `{node, appliesOnRestart: bool}` | `appliesOnRestart` when command or folder changed on a running terminal. |
| `presets_detect` | — | `{id, found: bool \| null}[]` | Login-shell lookup, 2 s limit. |
| `roles_list` | — | `Role[]` | |
| `role_save` | `{name, text}` | `Role` | `RoleNameTaken`, `NameEmpty`, `NameTooLong`. |
| `role_delete` | `{id}` | — | Terminals keep their copies. |
| `note_set_style` | `{nodeId, style: NoteStyle}` | — | |
| `note_insert_image` | `{nodeId, sourcePath}` | `{markdown}` | `ImageTooLarge`, `ImageUnsupported`. |
| `link_create` | `{workspaceId, a, b}` | `Link` | `LinkSelf`, `LinkExists`, `NodeNotFound`. |
| `link_delete` | `{workspaceId, linkId}` | — | |

## Events

`terminal://state` is now also emitted when a monitored terminal switches between busy and idle
(payload unchanged).
