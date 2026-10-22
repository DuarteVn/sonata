# Data Model: Agent Messaging

**Feature**: 003-agent-messaging · Extends 001 and 002 data models.

## Persisted

### TerminalContent (new optional fields, workspace schema stays v2)

| Field | Type | Default | Rules |
|---|---|---|---|
| `recruitedBy` | node UUID? | none | Set by `recruit`; used by `dismiss` (FR-022). |
| `sendInstructions` | bool? | none (= on for agents without session instructions, off for shells) | FR-019. |

Both are additive and optional; an older 002 build ignores unknown fields on load and would drop
them on save, which only affects dismiss rights and the typed-instructions option.

## Runtime only

### Session

| Field | Type |
|---|---|
| `terminalId` | node UUID |
| `token` | 32 random bytes, hex |
| `agentKind` | `claude` \| `codex` \| `other` (first word of the launch command) |
| `runDir` | `<data root>/run/<terminalId>/` (instructions.md, settings.json; `0700`/`0600`; removed on stop) |

### Ask

| Field | Type | Rules |
|---|---|---|
| `id` | UUID | |
| `from`, `to` | node UUID | Linked (FR-004). |
| `message` | string | ≤ 100 KB (FR-029). |
| `state` | `queued` → `waiting` → `answered` \| `replied` \| `timedOut` \| `failed` \| `cancelled` | |
| `sentAt`, `deadline` | time | Timeout 5 s–60 min, default 10 min. |
| `anchorRow` | integer | Absolute screen row at send (R5). |
| `result` | string | ≤ 1 MB, truncated with a note. |

Loop rule: an ask from X to Y is refused when Y is (transitively) waiting on an ask whose
asker chain includes X.

### Exchange log entry (last 50 per terminal)

`{at, direction: in|out, other, firstLine, outcome, resultChars}`.
