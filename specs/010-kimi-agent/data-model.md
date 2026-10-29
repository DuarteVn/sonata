# Data Model: Kimi Code CLI as a Supported Agent

## Kimi integration status (computed, not stored)

| Value | Meaning |
|---|---|
| `on` | Kimi's config file holds exactly the current set of Sonata hook tables. |
| `outdated` | It holds some Sonata hook tables, but not exactly the current set. |
| `off` | It holds no Sonata hook table. |
| `unavailable` | The Kimi folder does not exist, or the config file cannot be read. Carries a message. |

Computed by `core` from the file (research R4). A Sonata hook table is a `[[hooks]]` table whose
`command` contains `sonata _hook kimi`.

```text
KimiIntegration {
  status: "on" | "outdated" | "off" | "unavailable",
  configPath: string,          // resolved from the login shell's KIMI_CODE_HOME, else ~/.kimi-code
  block: string,               // exact lines Apply adds (for display and Copy)
  message?: string,            // why unavailable, or why Apply would be refused
  canApply: boolean,           // false when the file defines `hooks` in another form, or unavailable
  lastBackup?: string,         // path of the newest Sonata backup, if any
  validated: boolean,          // false when `kimi` is not on PATH, so `kimi doctor` could not check the file
}
```

### Transitions

```text
off ──Apply (confirmed)──► on
outdated ──Apply (confirmed)──► on
on | outdated ──Remove (confirmed)──► off
on ──Sonata updates its block version──► outdated
any ──user or Kimi edits the file──► recomputed on the next file change
```

Apply and Remove each write one backup first. A failed check after writing restores the backup,
and the status stays what it was.

## Preference: `kimiNoticeDismissed` (new, optional)

Field on `Preferences` in `preferences.json`. Omitted when unset (constitution IV). No schema
bump: the field is optional.

| Field | Type | Rule |
|---|---|---|
| `kimiNoticeDismissed` | `"off"` \| `"outdated"` | The status shown when the user dismissed the notice. The notice is hidden while the status equals this value. |

## Conversation (feature 009, extended)

`TerminalContent.conversation.agent` gains `"kimi"`. All other 009 rules are unchanged. Kimi ids
look like `session_<uuid>` and pass 009's id rule (`[A-Za-z0-9_-]{1,128}`).

## Session files (per running terminal, not persisted)

Under `<data root>/run/sessions/<node id>/`, written by `_session`, removed when the terminal
stops (feature 003):

| File | Agent | Content |
|---|---|---|
| `instructions.md` | Claude Code | existing |
| `settings.json` | Claude Code | existing |
| `kimi-agent.md` | Kimi | agent file: frontmatter `name: sonata-session`, `description`; body `${base_prompt}`, `${plugin_sections}`, Sonata instructions (research R2) |

## Hooked and turn state (in memory, per running terminal)

Per process generation, reset on Restart, reopen, and Start fresh (research R5):

| Field | Type | Rule |
|---|---|---|
| `hooked` | bool | Starts `false`; set to `true` by the first `_turn` or `_conversation` report from that process. Decides whether a Kimi target behaves as `Other` or `Hooked`. |
| `turn_active` | bool | Set by `_turn start`, cleared by `_turn complete`. For hooked Kimi terminals: drives `working`/`idle` in `sonata list` and the canvas activity indicator, and suspends the broker's quiet fallback. |

## Constants

- `PRESETS` gains `kimi` / "Kimi Code" / `kimi`.
- `WRAPPED_AGENTS` gains `kimi`.
- `AGENT_SESSION_MARKERS` (feature 009) gains `KIMI_PLUGIN_ROOT`, `KIMI_CODE_UPDATE_REEXEC`.
- `KIMI_BLOCK_VERSION = 1`.
