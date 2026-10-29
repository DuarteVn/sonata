# Contract: CLI internals (agent wrapper and hooks)

These commands are internal: agents and the wrapper call them, users never type them. All go over
the existing socket with the calling terminal's identity; the server decides the node from that
identity, never from arguments (constitution V).

## `_session <agent> <hook>` (changed)

Reply `data` gains an optional `resume`:

```json
{
  "agent": "claude",
  "instructionsFile": "…/instructions.md",
  "settingsFile": "…/settings.json",
  "codexNotify": "…",
  "resume": { "agent": "claude", "id": "7294d08d-…" }
}
```

`resume` is present only on the first `_session` of the terminal process and only when the stored
conversation's agent equals `<agent>` (research R3).

## `_conversation <agent> <id>` (new)

Records `<id>` as the calling terminal's conversation. Reply `ok` with empty text. Invalid `<agent>`
or `<id>` (data-model rules): exit 2, nothing stored.

## `_conversation <agent> none` (new)

Removes the calling terminal's conversation if it belongs to `<agent>`.

## Hooks (`sonata _hook …`)

| Hook | Called by | Reads | Reports |
|---|---|---|---|
| `session-start` (new) | Claude `SessionStart` | stdin JSON `session_id`, `source` | `resume` → `_conversation claude <id>`; `startup`/`clear` → `_conversation claude none`; `compact` → nothing |
| `turn-start` (changed) | Claude `UserPromptSubmit` | stdin JSON `session_id` | `_turn start` as today, then `_conversation claude <id>` |
| `codex-notify` (changed) | Codex `notify` | argv JSON `thread-id` | `_turn complete` as today, then `_conversation codex <id>` |

Hooks skip conversation reports when `SONATA_CONVERSATION=off` (print mode, research R9). Hooks
always exit 0, as today.

## `sonata run <agent> [args…]` (changed)

- With `resume` in the reply and arguments that do not already pick a conversation (research R3):
  run `claude --resume <id> [args…]` or `codex resume <id> [args…]` as a child, ignore
  `SIGINT`/`SIGQUIT` while waiting. Non-zero exit within 10 s → print
  `sonata: the previous conversation could not be resumed; starting a new one.` to stderr and
  `exec` the agent with the original arguments. Otherwise exit with the child's status.
- Without `resume`: unchanged (`exec`).
- Print mode (`claude -p`/`--print`): set `SONATA_CONVERSATION=off` for the child.
