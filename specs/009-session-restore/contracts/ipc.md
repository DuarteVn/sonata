# Contract: Shell commands and UI

## Tauri command `terminal_start_fresh`

```ts
terminalStartFresh(nodeId: string, confirmed: boolean): Promise<void>
```

- Busy terminal and `confirmed = false`: error `TerminalBusy` with the same text as Restart; the UI
  shows the existing confirmation and calls again with `confirmed = true`.
- Otherwise: clears the node's conversation, saves the workspace, restarts the terminal.
- Unknown node: `NodeNotFound`.

## Node type (UI)

`TerminalContent` in `ui/src/ipc/types.ts` gains:

```ts
conversation?: { agent: "claude" | "codex"; id: string; recordedAt: string };
```

No new event: conversation changes arrive with the existing `NodesChanged`.

## Terminal menu

- "Start fresh" appears for terminals with `conversation`; hidden otherwise.
- Tooltip: "Forget this terminal's conversation and restart the agent in a new one."
