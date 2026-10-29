# Data Model: Session Restore

## Conversation (new, on `TerminalContent`)

Field `conversation` on `TerminalContent` in the workspace file. Omitted when empty (constitution
IV). No schema bump (research R6).

| Field | Type | Rule |
|---|---|---|
| `agent` | `"claude"` \| `"codex"` | One of the agents in `WRAPPED_AGENTS`. |
| `id` | string | The agent's conversation id. 1–128 characters of `[A-Za-z0-9_-]`; anything else is rejected, because it becomes a command-line argument. |
| `recordedAt` | RFC 3339 timestamp | Time of the last change. For display and debugging only. |

```json
"conversation": {
  "agent": "claude",
  "id": "7294d08d-ac3c-4e7e-925e-16750c04c269",
  "recordedAt": "2026-10-24T13:05:00Z"
}
```

### Rules

- A terminal has at most one conversation.
- A report with a different agent or id replaces the stored value; the same value is a no-op.
- A "no conversation" report (Claude `startup` or `clear`) removes the field.
- Start fresh removes the field.
- Any copy of a terminal node (templates, clone, import, when they exist) has no conversation.
- Deleting the terminal removes the field with the node; the agent's files stay.

## Resume offer (in memory, per running terminal)

Not persisted. Set when the terminal's process starts and the node has a conversation. Consumed by
the first `_session` request from that process (research R3). A new process generation (Restart,
reopen, Start fresh) sets it again from the stored value.

```text
process start ──► offer = node.conversation (may be none)
_session (1st) ──► reply.resume = offer; offer = consumed
_session (later) ──► no resume
```

## Lifecycle

```text
        (none) ──first user message / resume──► stored(id)
stored(id) ──/clear, new session──► (none) ──first message──► stored(id2)
stored(id) ──/resume other──► stored(id3)
stored(id) ──app reopen──► offer ──resume ok──► stored(id)   (SessionStart resume)
                                  └─resume failed──► fresh start ──► (none)
stored(id) ──Start fresh──► (none) + restart
```

## Agent session markers (constant)

`core::terminal::AGENT_SESSION_MARKERS`: the explicit list in research R5. Removed from every
terminal's environment and from the usage runner's child environment.
