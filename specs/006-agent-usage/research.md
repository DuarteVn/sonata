# Research: Agent Usage Meter

Feature: `006-agent-usage` | Date: 2026-10-23

All findings below come from the CLIs' own help, documented output formats, public source code,
and two live test runs on the maintainer's Mac. No credential store (macOS Keychain) and no
credential file was opened during this research.

## R1. Claude Code: where the plan limits come from

**Decision**: Sonata reads Claude Code limits through the `claude` CLI itself, with two calls:

1. `claude auth status --json` for login state and plan name.
2. A minimal print-mode probe whose `stream-json` output carries a `rate_limit_event`.

Sonata never opens the Keychain or a credential file for Claude Code. The CLI uses its own
login, as it does for every request the user makes.

**Findings**

- `claude auth status --json` (Claude Code 2.1.309) prints `loggedIn`, `authMethod`
  (`claude.ai`, API key, …), `apiProvider` (`firstParty`, Bedrock, …), `email`, `orgName`,
  `subscriptionType` (for example `team`). It holds no secret. It does not report limits.
- `claude -p … --output-format stream-json --verbose` emits one line per event. After the first
  API response it emits:

  ```json
  {"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1792767000,
   "rateLimitType":"five_hour","overageStatus":"rejected","isUsingOverage":false,
   "unifiedWindows":{"five_hour":{"utilization":0.25,"resetsAt":1792767000},
                     "seven_day":{"utilization":0.04,"resetsAt":1793343600}}}, …}
  ```

  `utilization` is a fraction (0.25 = 25%). `resetsAt` is Unix epoch seconds. Verified live on a
  Team plan. `rate_limit_event` is a message type of the Agent SDK stream format, so it is part
  of a published output format, not a private file.
- Probe cost, measured: with `--model haiku`, a one-line `--system-prompt`, `--tools ""`,
  `--setting-sources ""`, `--strict-mcp-config`, `--disable-slash-commands`,
  `--no-session-persistence`, `--max-turns 1`, one probe used 2 input + 588 cache-write + 3 output
  tokens (about US$0.0001 at API prices) and took 2.3 s wall time. The probe itself adds a
  negligible amount to the 5-hour window.
- Accounts without plan limits (API key, Bedrock, Vertex) do not get `unifiedWindows`; the probe
  then reports "No plan limits" (FR-009). `authMethod`/`apiProvider` from `auth status` tells
  Sonata this before any probe, so Sonata skips the probe for them.

**Probe command** (argv, no shell; run in an empty temporary directory):

```text
<claude path> -p "." --model haiku --system-prompt "Reply with one dot." --tools ""
  --setting-sources "" --strict-mcp-config --disable-slash-commands
  --no-session-persistence --max-turns 1 --output-format stream-json --verbose
```

The exact flags are fixed in one constant in `crates/core/src/usage/claude.rs` and shown to the
user in the consent prompt (FR-012).

**Schedule (decided by the maintainer)**: automatic probe every 5 minutes while the app is open
and usage reading is on, plus on panel open when the cached value is older than 1 minute, plus on
manual refresh. Never more than once per minute per agent (FR-006). About 12 probes per hour.

**Alternatives considered**

- *Status line relay*: Claude Code passes `rate_limits.five_hour` / `seven_day`
  (`used_percentage`, `resets_at`) to a configured status line command. Documented and free, but
  Sonata would have to replace the user's `statusLine` setting and would only see data while a
  Claude session is active. Rejected by the maintainer.
- *Provider usage endpoint with the stored OAuth token*: the CLI binary calls an
  `/api/oauth/usage` endpoint. Undocumented, and Sonata would have to read the token from the
  Keychain. Rejected: higher risk (format may change without notice, Sonata holds a secret in
  memory) and not needed, because the CLI probe gives the same windows.
- *`/usage` in an interactive session*: requires driving a TUI in a hidden PTY and scraping text.
  Fragile; rejected.
- *Local files under `~/.claude/`*: `sessions/*.json` and the other state files hold no limit
  data. Rejected.

## R2. Codex: where the plan limits come from

**Decision**: Primary source is the Codex app server: `codex app-server` speaks JSON-RPC over
stdio; Sonata sends `initialize`, then `account/rateLimits/read`, then closes stdin. Fallback,
when the app server is unavailable (old Codex): the last `token_count` event in the newest
session file under `$CODEX_HOME/sessions` (default `~/.codex/sessions`). Sonata never opens
`~/.codex/auth.json`.

**Findings** (from the public `openai/codex` source; Codex is not installed on the maintainer's
Mac, so live verification is a task of the implement phase)

- `RateLimitSnapshot` has `primary` and `secondary` windows, `plan_type`, `credits`, and
  `rate_limit_reached_type`. `RateLimitWindow` has `used_percent` (0–100), `window_minutes`, and
  `resets_at` (Unix seconds). `PlanType` is a lowercase enum (`plus`, `pro`, `team`, …,
  `unknown`).
- The app server method `account/rateLimits/read` asks the Codex backend for the current limits
  without sending a model request (openai/codex PR #5302). The message schema is marked unstable;
  `codex app-server generate-json-schema` (or `codex generate-ts`) prints the schema of the
  installed version. Field names in app-server responses are camelCase.
- Session files are JSONL (`rollout-*.jsonl`, date-sharded folders). Lines of
  `{"type":"event_msg","payload":{"type":"token_count","info":…,"rate_limits":{…}}}` carry the
  snapshot after each turn. Files can be very large (hundreds of MB), so Sonata reads only the
  last 256 KiB of the newest file. Values from files are as old as the last Codex turn; the panel
  shows the file time as the reading time, so the stale rule (FR-004) applies.
- The primary window is usually 5 hours (300 minutes) and the secondary 7 days; Sonata labels
  windows from `window_minutes`, not from position.

**Alternatives considered**: `/status` in the Codex TUI (TUI scraping, rejected); reading
`auth.json` and calling the backend (credential handling, rejected).

## R3. Other agents (Gemini CLI, OpenCode)

**Decision**: Not built in. No reliable, CLI-exposed source of plan limits was found. They appear
only through a custom definition (R4). FR-007 and FR-008 hold.

## R4. Custom provider definition format

**Decision**: One JSON file per agent in `~/.sonata/usage-providers/`, with the standard envelope
(constitution IV). The definition names an absolute program path and its arguments. The program
must print one JSON object in Sonata's normalized usage format (see
`contracts/usage-provider.md`). Sonata does no field mapping: a user who needs mapping writes a
small wrapper script (the guide shows one with `jq`).

**Rationale**: A fixed output format keeps the parser small and testable, and an agent can write
a wrapper from the guide in one step. Field-path mapping (JSONPath, regex) adds a mini-language
for little gain.

**Validation**: `id` matches `^[a-z0-9][a-z0-9-]{0,31}$` and does not collide with a built-in id
or another file; `command[0]` is an absolute path to an existing executable file (no `PATH`
lookup, so the consent prompt shows exactly what runs); at most 16 arguments, each under 1 KiB;
`color` is `#rrggbb` (optional). An invalid file yields an error row in Settings and affects no
other agent (FR-018, US5 scenario 2).

**Guide file**: Sonata writes `README.md` into the folder when it is missing and never
overwrites it, so user edits survive. Deleting it brings back the current version at the next
start or "Refresh all".

## R5. Running provider commands safely

**Decision**: One runner in core for every provider command (built-in and custom):

- `std::process::Command` with argv (no shell), `stdin` null, current directory a fresh empty
  temporary directory, and a cleared environment plus an allowlist: `HOME`, `USER`, `LOGNAME`,
  `PATH` (from the login-shell detection), `LANG`, `TMPDIR`. Secrets in Sonata's own environment
  (for example `ANTHROPIC_API_KEY`) are not passed.
- New process group (`process_group(0)`); on timeout Sonata sends `SIGKILL` to the group, so
  child processes die too. Timeout: 10 s for custom commands (FR-019) and 20 s for built-in
  probes.
- stdout read on a thread into a 64 KiB buffer (larger output is an error); stderr kept to 4 KiB
  for the log, after redaction of anything that looks like a token.
- Custom commands only: run under `/usr/bin/sandbox-exec` with a profile that allows everything
  except reading or writing `~/Library/Keychains`, `/Library/Keychains`,
  `~/.claude/.credentials.json`, `~/.codex/auth.json`, and Sonata's data folder, and except
  Mach lookups of the security services. Tested on this Mac: Keychain folder reads are denied,
  the `security` tool fails, network access works. If `sandbox-exec` is missing, Sonata refuses
  to run custom commands and says why (fail closed). Built-in probes do not run sandboxed,
  because the agent CLI must read its own login.
- Consent for a custom command stores the approved argv; a definition whose argv differs asks
  again (FR-019). Plain equality, no hash, no new dependency.

**Alternatives considered**: running through the login shell (`sh -l -c`) like preset detection
(rejected: shell quoting and the exact command shown in consent would differ from what runs);
no sandbox and consent only (rejected: FR-019 asks that a definition cannot make Sonata read
credential stores; consent alone does not enforce it).

## R6. Command detection

**Decision**: Reuse the login-shell `command -v` approach of `presets::detect`, extended to print
full paths (FR-017 shows the path). Detection runs at startup, on "Refresh all", and when a probe
fails with "not found". The `PATH` printed by the same login shell is cached and used as the
`PATH` allowlist entry of R5.

## R7. Warnings and window periods

**Decision**: A window period is identified by `(provider id, window kind, resetsAt)`. Core keeps,
per period, the highest warning level already sent (`none`, `warn80`, `full100`). A reading sends
a notification only when its level is above the stored level. When `resetsAt` changes (new
period) the stored level starts at `none` again. Stored levels persist in the usage cache, so a
restart does not repeat a warning (FR-010, SC-003). A drop from a higher level within the same
period (for example a percentage corrected by the provider) never re-arms a level.

Notifications are sent by the frontend through the existing notification plugin path (as in
feature 003), triggered by a new core event. The master warning switch is a preference
(FR-011).

## R8. Settings window

**Decision**: A second Tauri window with label `settings`, created on demand from the app menu
item "Settings…" (accelerator `Cmd+,`), loading the same frontend bundle with
`index.html?view=settings`. A single instance: a second open focuses the existing window. The
capability file grants the `settings` window only the `usage_*` and `app_reveal_path` commands
it needs.

**Alternative considered**: a modal inside the main window. Rejected: the spec asks for a
Settings window, and later features add sections to it.

## R9. Persistence

**Decision**

- `~/.sonata/preferences.json` gains `usage` settings: master switch, warnings switch, and per
  provider `{enabled, color, consent}` (consent: `granted` with the approved argv, or `denied`).
  Serde defaults; no schema bump (same approach as feature 005).
- `~/.sonata/usage-cache.json` (envelope `usageCache`, schemaVersion 1): last good reading per
  provider and warning levels per window period. No credential, no email (only plan name and an
  account label such as the org name). Atomic writes.
- `~/.sonata/usage-providers/` (mode 0700): custom definitions and `README.md`.

**Account label**: Claude: `orgName` when present, else "Personal". The panel also shows the
login email from the latest `auth status` call, kept in memory only, so the user can tell which
account is active (spec edge case "several accounts"). The email is never written to the cache
or the log.
