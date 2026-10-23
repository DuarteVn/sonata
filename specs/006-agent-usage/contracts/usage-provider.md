# Contract: Usage Providers

## Built-in: Claude Code (`claude`)

1. `"<claude>" auth status --json` (20 s limit).
   - `loggedIn: false` → `notLoggedIn`, message "Run `claude auth login` in a terminal."
   - `authMethod` not `claude.ai` or `apiProvider` not `firstParty` → `noPlanLimits`; no probe.
   - else `plan` = `subscriptionType` title-cased, `account` = `orgName` or "Personal",
     `accountEmail` = `email` (memory only).
2. Probe (argv fixed in `usage::claude::PROBE_ARGS`, run in an empty temp dir, 20 s limit):

   ```text
   -p . --model haiku --system-prompt "Reply with one dot." --tools ""
   --setting-sources "" --strict-mcp-config --disable-slash-commands
   --no-session-persistence --max-turns 1 --output-format stream-json --verbose
   ```

3. Parse stdout line by line; take the last line with `"type":"rate_limit_event"`.
   `rate_limit_info.unifiedWindows.five_hour` → window `short` "5-hour";
   `seven_day` → `weekly` "Weekly"; `usedPercent = utilization × 100` (rounded to 0.1);
   `resetsAt` from epoch seconds. Other keys in `unifiedWindows` (for example a per-model weekly
   window) → `other`, labelled from the key ("7-day sonnet" → "Weekly (Sonnet)") only when the
   key matches `seven_day_<word>`; unknown shapes are ignored.
   - No `rate_limit_event` and exit code 0 → `noPlanLimits`.
   - Non-zero exit with an auth error in the result line → `notLoggedIn`.
   - Anything else → `unavailable` with a short reason; stdout is not logged.

## Built-in: Codex (`codex`)

1. Primary: spawn `"<codex>" app-server`, write JSON lines to stdin:

   ```json
   {"id":1,"method":"initialize","params":{"clientInfo":{"name":"sonata","version":"<app version>"}}}
   {"method":"initialized"}
   {"id":2,"method":"account/rateLimits/read"}
   ```

   Read stdout lines until the response with `id: 2`, then close stdin and kill the group
   (20 s limit). Map `primary`/`secondary` → windows (`usedPercent`, `windowDurationMins` or
   `window_minutes`, `resetsAt`/`resets_at`), `planType` → `plan`. An auth error → `notLoggedIn`;
   no snapshot → `noPlanLimits`. The exact field names are confirmed against
   `codex app-server generate-json-schema` during implementation and recorded as fixtures.
2. Fallback (method unknown or app server fails to start): newest
   `$CODEX_HOME/sessions/**/rollout-*.jsonl` by mtime; read its last 256 KiB; last line with
   `payload.type == "token_count"` and a non-null `payload.rate_limits`. `readAt` = file mtime.
   No file → `unavailable` "Use Codex once to record limits."

## Custom definition file

Location: `~/.sonata/usage-providers/<anything>.json` (the file name is not the id). Files not
ending in `.json` and `README.md` are ignored.

```json
{
  "type": "usageProvider",
  "schemaVersion": 1,
  "payload": {
    "id": "my-agent",
    "name": "My Agent",
    "command": ["/Users/me/bin/my-agent-usage", "--json"],
    "color": "#3b82f6"
  }
}
```

| Field | Rule |
|---|---|
| `id` | `^[a-z0-9][a-z0-9-]{0,31}$`; not `claude`/`codex`; unique across files (duplicates: both rejected) |
| `name` | 1–40 characters |
| `command` | 1–17 strings; `command[0]` absolute path to an executable regular file; each ≤ 1 KiB; no NUL |
| `color` | optional `#rrggbb` |
| unknown keys | rejected (catches typos) |

## Normalized output (what a custom command prints on stdout)

```json
{
  "status": "ok",
  "plan": "Pro",
  "account": "work",
  "windows": [
    { "kind": "short", "label": "Session", "usedPercent": 42.5, "resetsAt": "2026-10-23T18:00:00Z" },
    { "kind": "weekly", "usedPercent": 12, "resetsAt": 1793343600 }
  ]
}
```

| Field | Rule |
|---|---|
| `status` | `ok` (default), `notLoggedIn`, `noPlanLimits`, `unavailable` |
| `message` | optional, ≤ 200 characters, shown for non-ok statuses |
| `plan`, `account` | optional strings ≤ 40 characters |
| `windows` | required when `ok`; 1–4 entries |
| `windows[].kind` | `short`, `weekly`, or `other` |
| `windows[].label` | optional (default "Short window" / "Weekly" / "Limit"), ≤ 24 characters |
| `windows[].usedPercent` | number ≥ 0 |
| `windows[].resetsAt` | optional; RFC 3339 string or Unix seconds |

Exit code non-zero, invalid JSON, more than 64 KiB, or more than 10 s → `unavailable` with the
reason ("exited with status 2", "timed out after 10 s", …).

## Runner rules (all providers)

- argv only, never a shell; `stdin` null (Codex: pipe); fresh empty temp dir as cwd.
- Environment cleared, then: `HOME`, `USER`, `LOGNAME`, `LANG`, `TMPDIR`, and `PATH` from
  login-shell detection. Codex also gets `CODEX_HOME` when set in the login shell.
- New process group; `SIGKILL` to the group on timeout (custom 10 s, built-in 20 s).
- stdout cap 64 KiB (Codex app server: 1 MiB, line by line); stderr cap 4 KiB, used only for a
  redacted log line.
- Custom only: wrapped as `/usr/bin/sandbox-exec -p <profile> <command…>`. Profile:

  ```scheme
  (version 1)
  (allow default)
  (deny file-read* file-write*
    (subpath "<HOME>/Library/Keychains") (subpath "/Library/Keychains")
    (literal "<HOME>/.claude/.credentials.json") (literal "<HOME>/.codex/auth.json")
    (subpath "<SONATA_ROOT>"))
  (deny mach-lookup (global-name-regex #"^com\.apple\.secur") (global-name "com.apple.SecurityServer"))
  (allow file-read* (subpath "<SONATA_ROOT>/usage-providers"))
  ```

  The last rule wins over the Sonata-folder deny, so helper scripts may live next to their
  definition file.

  Paths are canonicalized (the sandbox matches real paths, for example `/private/tmp`) and
  escaped for the profile string. Missing `sandbox-exec` → custom providers show
  "Cannot run safely: sandbox-exec not found" and do not run.
- Nothing from stdout, stderr, or env is written to `~/.sonata/` except the parsed reading.

## Guide (`~/.sonata/usage-providers/README.md`)

Written by Sonata when missing. Original text (English) covering: purpose of the folder, the
definition file with every field, the normalized output with every field, the 10 s limit and
sandbox, the consent prompt (re-asked when `command` changes), a worked wrapper example that
turns another tool's JSON into the normalized output with `jq`, and a short prompt the user can
give to an agent ("Write a Sonata usage provider for … following README.md in this folder").
