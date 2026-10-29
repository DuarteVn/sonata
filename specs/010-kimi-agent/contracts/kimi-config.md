# Contract: Sonata block in Kimi's config file

File: `$KIMI_CODE_HOME/config.toml`, else `~/.kimi-code/config.toml` (research R4). This is the
exact text Apply appends (block version 1). Kimi's `[[hooks]]` schema allows only `event`,
`command`, `matcher`, `timeout`; the block uses no other key.

```toml
# >>> Sonata integration (v1). Managed by Sonata: Settings > Agents > Kimi Code. >>>
# These hooks only act inside Sonata terminals; elsewhere they exit at once.
[[hooks]]
event = "SessionStart"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi session-start; exit 0"
timeout = 5

[[hooks]]
event = "UserPromptSubmit"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi prompt-submit; exit 0"
timeout = 5

[[hooks]]
event = "Stop"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi stop; exit 0"
timeout = 5

[[hooks]]
event = "StopFailure"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi stop-failure; exit 0"
timeout = 5

[[hooks]]
event = "Interrupt"
command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi interrupt; exit 0"
timeout = 5
# <<< Sonata integration <<<
```

## Rules

- **Placement**: appended at the end of the file, after one blank line. A missing final newline
  in the user's file is added first.
- **Ownership**: a `[[hooks]]` table is Sonata's when its `command` value contains
  `sonata _hook kimi`. The table spans from its header line up to the next line that starts a
  table (`[` after optional spaces) or the end of the file. Marker comments directly before or
  after Sonata's tables belong to the block.
- **Status**: `on` = Sonata's tables are exactly the five above (same `event`, `command`,
  `timeout`, compared after trimming spaces); `outdated` = at least one Sonata table, not exactly
  these; `off` = none.
- **Conflict**: Apply is refused when any line outside Sonata's tables matches `^\s*hooks\s*=` or
  `^\s*\[hooks\]`, because `[[hooks]]` would then be invalid TOML.
- **Remove**: deletes Sonata's tables, the marker comments, and blank lines left at the end of the
  file. Every other byte stays the same.
- **Backup**: `config.toml.sonata-backup-<YYYYMMDD-HHMMSS>` next to the file, same permission bits;
  the three newest are kept.
- **Write**: atomic, same permission bits as the original (new file: `0600`).
