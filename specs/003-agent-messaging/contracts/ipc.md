# Contract: Frontend IPC Changes for Feature 003

## New commands

| Command | Input | Output |
|---|---|---|
| `exchange_log` | `{nodeId}` | `ExchangeEntry[]` (last 50) |

## New events

| Event | Payload |
|---|---|
| `messaging://ask` | `{askId, from, to, state: "queued" \| "waiting" \| "answered" \| "replied" \| "timedOut" \| "failed" \| "cancelled"}` |
| `messaging://notify` | `{nodeId, message}` (UI remembers it to select the terminal on next focus) |
| `workspace://nodes-changed` | `{workspaceId, nodes, links}` (recruit, dismiss, link, unlink from the CLI) |

## Changed

`TerminalSettings` gains `sendInstructions?: boolean | null`.
