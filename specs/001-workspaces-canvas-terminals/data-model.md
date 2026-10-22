# Data Model: Workspaces, Canvas, Terminals, and Notes

**Feature**: 001-workspaces-canvas-terminals | **Date**: 2026-10-21

Persisted fields use `camelCase` in JSON. Optional fields are omitted when empty. All ids are UUID
v4 strings. Times are ISO 8601 UTC strings. File formats are in
[contracts/file-formats.md](contracts/file-formats.md).

## Workspace

| Field | Type | Rules |
|---|---|---|
| `id` | UUID | Immutable. Equals the workspace folder name. |
| `name` | string | Trimmed, 1–80 chars, unique among workspaces ignoring case (FR-002). |
| `icon` | string | One of the icon set (`IconId`). Random on create (FR-003). |
| `color` | string | One of 8 palette colors (`#RRGGBB`). Random, preferring unused (FR-003). |
| `workingDirectory` | absolute path | Must exist on create (FR-002). May go missing later (warning only). |
| `isPinned` | bool | Default `false`. Pinned sort first (FR-004). |
| `createdAt`, `lastOpenedAt` | time | `lastOpenedAt` updated on switch. |
| `camera` | `{x, y, zoom}` | `zoom` clamped to [0.1, 4.0]. New workspace: `{x: 0, y: 0, zoom: 1}`. |
| `nodes` | `Node[]` | Unlimited. |

Sidebar order is not a workspace field; it lives in `AppState.workspaceOrder` so that reordering
writes one file.

**Validation on create**: name trimmed non-empty → name unique (case-insensitive) → directory is
absolute, exists, and is a directory. Errors: `NameEmpty`, `NameTaken`, `DirectoryMissing`,
`NotADirectory`.

**Lifecycle**: `Unloaded` (on disk, terminals not started) → `Loaded` (opened this session,
terminals running) → `Deleted` (folder removed after confirmation). A folder that fails to decode
is a `LoadFailure` instead (not a Workspace).

## Node

| Field | Type | Rules |
|---|---|---|
| `id` | UUID | Immutable. |
| `frame` | `{x, y, width, height}` | Canvas units. Minimum size: terminal 240×120, note 120×80. |
| `z` | integer | Higher draws on top. "Bring to front" sets `max(z) + 1`. |
| `createdAt`, `modifiedAt` | time | `modifiedAt` updated on any change. |
| `content` | `NodeContent` | Exactly one key: `terminal` or `note`. More or none → decode error. |

### NodeContent.terminal

| Field | Type | Rules |
|---|---|---|
| `name` | string | Trimmed, 1–60 chars. Default `Terminal N`. Not required to be unique in 001. |
| `launchCommand` | string? | Written to the shell after spawn (research R5). |
| `workingDirectory` | absolute path? | Defaults to the workspace directory when absent. |
| `saveHistory` | bool | Default `true` (FR-015a). When `false`, no history file exists. |

**Runtime state (not persisted)**: `TerminalRun` = `NotStarted` → `Starting` → `Running{pid}`
→ `Exited{code}`; plus `Busy`/`Idle` (research R6). `Exited` → `Starting` on restart (FR-017).
`Running` → removed on close (confirmation when `Busy`, FR-018).

### NodeContent.note

| Field | Type | Rules |
|---|---|---|
| (none) | | The text lives in `notes/<nodeId>.md`. The node stores no text. |

**Runtime state**: `NoteSync` = `Clean` | `Dirty` (unsaved in-app edits) | `Conflict{path}` |
`FileMissing`. Transitions per FR-020a and research R8.

## AppState (`app-state.json`)

| Field | Type | Rules |
|---|---|---|
| `activeWorkspaceId` | UUID? | Restored at launch unless the crash guard trips (FR-027). |
| `recentWorkspaceIds` | UUID[] | Most recent first, max 10 (FR-026). |
| `workspaceOrder` | UUID[] | Sidebar order; unknown ids ignored, missing ids appended (FR-004). |
| `cleanShutdown` | bool | Research R9. |
| `outlivedLaunchWindow` | bool | Research R9. |
| `consecutiveEarlyCrashes` | integer | Research R9. Guard at ≥ 2. |

## Preferences (`preferences.json`)

| Field | Type | Default |
|---|---|---|
| `autosaveIntervalSeconds` | integer, 5–600 | 30 |
| `terminalFontSize` | integer, 9–24 | 13 |
| `historyMaxRows` | integer, 1,000–100,000 | 10,000 |

## LoadFailure (runtime only)

| Field | Type |
|---|---|
| `folderName` | string (may not be a valid UUID) |
| `path` | absolute path of the file that failed |
| `reason` | enum: `InvalidJson`, `WrongType`, `NewerVersion{found, supported}`, `MigrationFailed{step}`, `DecodeFailed{detail}`, `IdMismatch` |

A failed folder is never written by the app (FR-025).

## Relationships

```text
AppState ──order/recent/active──▶ Workspace (by id)
Workspace 1 ──▶ * Node
Node.content.terminal 1 ──▶ 0..1 history file  terminals/<nodeId>.history
Node.content.note     1 ──▶ 1    note file     notes/<nodeId>.md
```
