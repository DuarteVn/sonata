# Contract: Tauri Commands and Events for Lanes

Argument names are camelCase in TypeScript and snake_case in Rust. Errors use the existing
`AppError { code, message }`.

## Commands

| Command | Arguments | Returns |
|---|---|---|
| `node_create_terminal` (changed) | existing + `lane?: boolean` | `CanvasNode` (with `content.terminal.lane`) |
| `lane_check` | `workspaceId` | `{ available: boolean, reason?: string, baseBranch?: string }` |
| `lane_preview` | `nodeId`, `action: "land" \| "remove"` | `LanePreview` |
| `lane_land` | `nodeId`, `confirmed: boolean` | `{ commits: number }` |
| `lane_remove` | `nodeId`, `discardChanges: boolean`, `deleteBranch: boolean`, `confirmed: boolean` | `void` |
| `lane_forget` | `nodeId` | `void` |
| `lane_confirm_answer` | `requestId`, `accepted: boolean`, `discardChanges?: boolean`, `deleteBranch?: boolean` | `void` |
| `node_delete` (changed) | existing + `laneRemoval?: { discardChanges, deleteBranch }` | `void` |
| `workspace_delete` (changed) | unchanged | fails with `LaneHasChanges` listing lanes |

`confirmed: false` returns `ConfirmationRequired` and changes nothing (same pattern as
`terminal_restart`).

```ts
type LanePreview = {
  branch: string;
  baseBranch: string;
  commits: number;          // base..lane
  changedFiles: string[];   // first 20
  changedTotal: number;
  unmergedCommits: number;  // commits on no other branch (branch deletion warning)
  blockers: string[];       // non-empty → the action button is disabled
};
```

New error codes: `LaneUnavailable`, `LaneBlocked`, `LaneConflict` (message lists files),
`LaneUnsafePath`, `LaneHasChanges`, `ConfirmationRequired`, `LaneConfirmationNeeded`.

## Events

| Event | Payload |
|---|---|
| `lane://status` | `LaneStatus` (data-model.md) |
| `lane://confirm` | `{ requestId, nodeId, action: "land" \| "remove", requestedBy: string, preview: LanePreview, deleteBranch: boolean }` |
| `lane://confirm-closed` | `{ requestId }` (timeout or the CLI went away; the UI closes the dialog) |

## Capabilities

New commands are added to `src-tauri/build.rs` `COMMANDS` and `capabilities/default.json` for the
`main` window only.
