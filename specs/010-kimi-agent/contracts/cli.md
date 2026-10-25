# Contract: CLI internals (Kimi wrapper and hooks)

Internal commands: agents and the wrapper call them, users never type them. All go over the
existing socket with the calling terminal's identity; the server picks the node from that
identity, never from arguments (constitution V).

## `_session kimi <hook>` (changed)

Reply `data` for `kimi`:

```json
{
  "agent": "kimi",
  "agentFile": "…/run/sessions/<node>/kimi-agent.md",
  "resume": { "agent": "kimi", "id": "session_…" }
}
```

`agentFile` is always present. `resume` follows feature 009's rules (first `_session` of the
process, stored conversation of the same agent).

## `sonata run kimi [args…]` (changed)

Arguments that "choose a conversation or a mode": `-S`, `--session`, `-r`, `--resume`, `-c`,
`--continue`, `-C`, `--agent`, `--agent-file`, `-p`, `--prompt`, or a first argument that is a
subcommand (`login`, `acp`, `web`, `app`, `install-desktop`, `doctor`, `export`, `fork`,
`session`, `provider`, `vis`, `migrate`, `upgrade`, `update`).

| Case | Command |
|---|---|
| `resume` in reply, args do not choose | `kimi --session <id> [args…]` as a child, feature 009 fallback (non-zero exit within 10 s → notice, then `exec kimi --agent-file <f> [args…]`) |
| no `resume`, args do not choose | `exec kimi --agent-file <f> [args…]` |
| args choose | `exec kimi [args…]` unchanged; the `resume` offer is consumed |
| `-p` / `--prompt` in args | as above, plus `SONATA_CONVERSATION=off` in the child's environment |

Outside Sonata (no socket): `exec kimi [args…]` unchanged.

## `sonata _hook kimi <event>` (new)

Reads Kimi's hook payload (JSON) from stdin. Prints nothing on stdout or stderr. Always exits 0.
Does nothing when `SONATA_TERMINAL_ID` or the socket is missing.

| `<event>` | Kimi event | Reports |
|---|---|---|
| `prompt-submit` | `UserPromptSubmit` | `_turn start`, then `_conversation kimi <session_id>` |
| `stop` | `Stop` | `_turn complete` (no text) |
| `stop-failure` | `StopFailure` | `_turn complete` (no text) |
| `interrupt` | `Interrupt` | `_turn complete` (no text) |
| `session-start` | `SessionStart` | `source=resume` → `_conversation kimi <session_id>`; `source=startup` → `_conversation kimi none`; other → nothing |

With `SONATA_CONVERSATION=off`, `_conversation` reports are skipped; `_turn` reports are sent.
An unknown `<event>` or unreadable payload exits 0 without reporting.

## `_turn start` (changed)

Marks the calling terminal busy and its process "hooked" (data-model). Reply `ok`.

## `_turn complete`, `_conversation …` (changed)

Also mark the calling terminal's process "hooked". Otherwise as in features 003 and 009.

## `sonata recruit kimi …` (changed)

`kimi` is accepted like the other presets. Usage text lists it.
