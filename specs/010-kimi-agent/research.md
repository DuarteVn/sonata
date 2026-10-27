# Research: Kimi Code CLI as a Supported Agent

**Feature**: 010-kimi-agent | **Date**: 2026-10-25 · Checked against the Kimi Code CLI sources
(`MoonshotAI/kimi-code` at commit `743f140`, release `@moonshot-ai/kimi-code@2.3.2`, 2026-10-25),
its `docs/en/` pages, the public docs at kimi.com, issue #2455, and the current Sonata `core` and
`cli` sources (`messaging/instructions.rs`, `messaging/broker.rs`, `messaging/commands.rs`,
`app.rs`, `presets.rs`, `terminal/pty.rs`, `terminal/zsh.rs`, `cli/src/run.rs`,
`cli/src/hook.rs`). Kimi is not installed on the development Mac: every Kimi fact below comes
from reading its source and docs, not from running it, and is marked "unverified live" where the
quickstart has a live check. Paths starting with `kimi:` are in the Kimi repository.

## R1. Which program

**Decision**: Support Kimi Code CLI, npm package `@moonshot-ai/kimi-code`, command `kimi`, version
2.3 or later. The older Python `kimi-cli` and the unrelated unscoped npm package `kimi-code` are
not supported.

**Rationale**: It is the current CLI, and every mechanism below (hooks, agent files, sessions)
exists in it.

## R2. Session instructions without breaking resume

**Decision**: `sonata run kimi` passes `--agent-file <run dir>/kimi-agent.md` when it starts a
new conversation. The file is written by `core` next to the Claude instructions
(`<data root>/run/sessions/<node>/`), with this shape:

```markdown
---
name: sonata-session
description: Kimi's default agent, with Sonata session instructions.
---

${base_prompt}

${plugin_sections}

<Sonata session instructions, the same text Claude Code receives>
```

- `${base_prompt}` embeds Kimi's own default system prompt (or the user's `SYSTEM.md`), so Kimi
  behaves as usual plus Sonata's section (`kimi:docs/en/customization/agents.md`, "Agent file
  format"). `${plugin_sections}` keeps instructions from the user's Kimi plugins.
- No `tools` key: the agent keeps every tool (`kimi:…/agentFile.test.ts`, "tools undefined").
- The name `sonata-session` matches no built-in agent, so no `override` is needed.
- **Resume**: Kimi refuses `--agent-file` together with `--session` or `--continue`, because "the
  agent is bound at session creation and the bound agent is restored automatically on resume"
  (`kimi:apps/kimi-code/test/cli/options.test.ts:459`). The rendered system prompt is saved in the
  session and replayed. So the wrapper never adds `--agent-file` when it resumes, and a resumed
  conversation keeps the instructions it was created with, with no new message (FR-005).
- The wrapper does not add `--agent-file` when the user's own arguments already contain `--agent`,
  `--agent-file`, `--session`, `-S`, `-r`, `--resume`, `--continue`, `-c`, `-C`, `--prompt`, `-p`,
  or a subcommand.
- The instructions text must not contain `${`, because Kimi renders the body as a template. A
  unit test checks this.

Kimi terminals do not get the typed instruction message (feature 003 FR-019) by default, as for
Claude Code. The per-terminal "send instructions" switch still overrides it.

**Known gap**: `/new` and `/clear` inside the same Kimi process start a conversation with Kimi's
default agent, so that conversation has no Sonata instructions until the terminal restarts
(`kimi:apps/kimi-code/src/tui/kimi-tui.ts:3249`). Recorded in the quickstart and README; the
Restart action fixes it without the setup; with the setup, the new session is recorded and a
restart resumes it, so Start fresh (009) fixes it. Typing instructions after `/clear` was rejected: Kimi creates the session
when the user submits a prompt, so Sonata would type into the user's input.

**Alternatives considered**: `AGENTS.md` in the project or `$KIMI_CODE_HOME`: writes to user
files (FR-006). `$KIMI_CODE_HOME/SYSTEM.md`: permanent and global. A `UserPromptSubmit` hook that
prints the instructions: its output is added to the context and shown to the user on every
prompt (issue #2455). `SessionStart` hook output: discarded by Kimi. Typed message (feature 003):
adds a visible message, and repeats on every resume.

## R3. Hooks Sonata needs and what they do

Kimi runs each hook with `/bin/sh -c <command>` in the session folder, writes a JSON payload to
stdin, and inherits Kimi's environment, so `SONATA_TERMINAL_ID`, the socket path, and Sonata's
`PATH` entry reach the hook (`kimi:…/os/backends/node-local/hostProcessService.ts` `buildEnv`).
Every payload has `hook_event_name`, `session_id`, `cwd`, `client_type`. A failing or missing
command never breaks Kimi; a timeout or an exit code other than 0 and 2 means "allow". For every
event except `UserPromptSubmit`, stdout and stderr are discarded. For `UserPromptSubmit`, stdout
is added to the model's context and shown in the terminal, so Sonata's hook must print nothing.

**Decision**: Five hooks, all calling `sonata _hook kimi <event>`:

| Kimi event | Waits? | Sonata report |
|---|---|---|
| `UserPromptSubmit` | yes | `_turn start`, then `_conversation kimi <session_id>` |
| `Stop` | yes | `_turn complete` |
| `StopFailure` | no | `_turn complete` |
| `Interrupt` | no | `_turn complete` (Kimi sends `Interrupt` instead of `Stop` when cancelled) |
| `SessionStart` | yes | `source=resume` → `_conversation kimi <id>`; `source=startup` → `_conversation kimi none` |

- `Stop` carries no final message text, so `_turn complete` has no text and the broker uses the
  screen text, as for Codex turns without `last-assistant-message`.
- Kimi creates a session lazily: plain `kimi` fires `SessionStart startup` at the first prompt,
  just before `UserPromptSubmit`. `/new` and `/clear` fire `SessionStart startup` for the new
  session; `/sessions` fires `SessionStart resume` for the picked one. These map directly onto
  feature 009's rules (none on a new empty conversation, record at the first message, record on
  resume).
- Hooks are read when Kimi starts. After the user applies the integration, Kimi processes already
  running keep the old hooks until restarted. The setup view says so.

**Command shape** (one line per hook in the config file):

```sh
test -n "$SONATA_TERMINAL_ID" && sonata _hook kimi stop; exit 0
```

The `test` makes the hook a no-op outside a Sonata terminal without even starting `sonata`
(FR-012), and it keeps working after Sonata is uninstalled (`command not found` goes to stderr,
which Kimi discards for these events; `UserPromptSubmit` only reads stdout). `exit 0` keeps a
failing report from ever blocking a prompt (exit 2 would block). `timeout = 5` bounds the wait if
the socket hangs; `sonata _hook` already gives up quickly and always exits 0.

`sonata` is found through `PATH`, because Sonata terminals put the CLI folder on `PATH`. No
absolute path is written into the user's file, so moving or updating Sonata.app does not make
the lines out of date.

**Alternatives considered**: A Kimi plugin under `$KIMI_CODE_HOME/plugins` with `hooks` in
`kimi.plugin.json`: also writes into Kimi's folder, needs the plugin install flow, and plugin
hooks reload differently; no simpler for the user. `KIMI_CODE_HOME` pointing at a Sonata folder:
sessions, logins, and config live there too, so the user's history and login would split.

## R4. Editing Kimi's configuration file

Path: `$KIMI_CODE_HOME/config.toml`, else `~/.kimi-code/config.toml`
(`kimi:…/app/bootstrap/bootstrap.ts:173`). `KIMI_CODE_HOME` is read from the user's login shell
(`$SHELL -l -c`), as preset detection already does, because Sonata.app started from the Finder
does not see variables exported in shell startup files.

`[[hooks]]` entries are validated strictly: only `event`, `command`, `matcher`, `timeout`
(`kimi:…/features/externalHooks/configSection.ts`). One invalid entry makes Kimi drop the whole
`hooks` section (`configService.ts:748`, "Ignored invalid config section"), including the user's
own hooks. A TOML syntax error fails the whole file.

**Decision**: Line-based text editing, no TOML library:

- **Block**: Sonata's lines are one contiguous block between marker comments:
  ```toml
  # >>> Sonata integration (v1). Managed by Sonata: Settings > Agents > Kimi Code. >>>
  [[hooks]]
  event = "UserPromptSubmit"
  command = "test -n \"$SONATA_TERMINAL_ID\" && sonata _hook kimi prompt-submit; exit 0"
  timeout = 5
  …
  # <<< Sonata integration <<<
  ```
  The block is appended at the end of the file. A `[[hooks]]` array-of-tables header at the end of
  a TOML file is valid after any other table, so the user's content is untouched.
- **Detection (FR-007)**: by content, not position. Each `[[hooks]]` table (from its header line
  to the next `[` header or end of file) whose `command` contains `sonata _hook kimi` is Sonata's.
  Status is **on** when the set of Sonata tables equals the expected set for the current block
  version, **out of date** when some Sonata tables exist but differ, and **off** when there are
  none. Markers are not needed for detection, because Kimi's own config writeback can drop
  comments when it falls back to re-serializing the file (`kimi:…/app/config/tomlWriteback.ts`).
- **Apply**: refuse with a clear message, changing nothing, when the file defines `hooks` in any
  other form (`hooks = …` or a `[hooks]` table), because adding `[[hooks]]` would then be invalid
  TOML. Otherwise: remove any Sonata tables (out-of-date block), append the current block, write.
- **Remove**: delete Sonata's tables and the marker lines; leave everything else byte for byte.
- **Backup (FR-010)**: before each write, copy the file to
  `config.toml.sonata-backup-<YYYYMMDD-HHMMSS>` in the same folder, keeping the original mode.
  The three newest Sonata backups are kept; older Sonata backups are deleted. No other file is
  touched.
- **Write**: atomic (temporary file in the same folder, `fsync`, `rename`), keeping the original
  file's permission bits, because the file can hold API keys. A missing file is created with mode
  `0600` only when the Kimi folder exists; when the folder is missing, Apply says "Run Kimi once
  first" and does nothing.
- **Check before write**: when `kimi` is on the login shell's `PATH`, run `kimi doctor`
  (validates `config.toml`) with a 10 s timeout before changing anything. A failure means the
  user's file is already invalid: Apply and Remove stop with `KimiConfigInvalid` and Kimi's
  message, and change nothing, so Sonata never appends to a broken file or blames its own lines.
- **Check after write**: run `kimi doctor` again. A failure restores the backup and reports
  `KimiCheckFailed` with Kimi's message. Without `kimi` on `PATH`, both checks are skipped and the
  status view says the file was not validated. Unverified live: `kimi doctor` exit codes; the
  quickstart checks them.

**Rationale**: The edit is append-or-remove of whole tables Sonata owns, which line editing does
safely; a TOML library would be a new direct dependency (constitution VII) and would reformat the
user's file on write.

**Alternatives considered**: `toml_edit` (already in `Cargo.lock` through Tauri, but a new direct
dependency of `core`, and Sonata would still need the marker logic). Copy-paste only: rejected in
the spec clarification (2026-10-25).

## R5. Busy and done

**Decision**: `kimi` joins `WRAPPED_AGENTS`, but the broker cannot assume the hooks exist: a Kimi
without the integration sends no end-of-turn signal, and the hooked fallback (20 s of quiet) would
make every ask 20 s slow. So the target kind of a Kimi terminal is decided per running process:

- New `TargetKind::Probing` for Kimi terminals: behaves like `Other` (3 s quiet ends the turn)
  until Sonata receives any Kimi hook report from that terminal's process.
- Any `_turn` or `_conversation` report from the terminal marks its process generation "hooked"
  (in memory, reset when the process restarts). From then on the terminal is `Hooked`.
- `UserPromptSubmit` waits for the hook, so `_turn start` arrives before the model starts, well
  before 3 s of quiet. An ask sent to a hooked Kimi therefore switches to `Hooked` within its
  first tick and waits for `Stop`, `StopFailure`, or `Interrupt`.
- **Turn state**: `_turn start` (today a no-op reply) sets `turn_active` for the calling
  terminal's process; `_turn complete` clears it. Today "busy" means "a program other than the
  shell owns the terminal" (`terminal/busy.rs`), which is always true while an agent runs. For
  hooked Kimi terminals, `turn_active` replaces that check in `sonata list` (`working`/`idle`)
  and in the canvas activity indicator (`App::check_activity`, feature 002). Claude Code and
  Codex keep today's rule; changing them is out of scope.
- **No quiet cut-off during a turn**: the hooked quiet fallback (`HOOKED_QUIET_FALLBACK`, 20 s)
  exists for agents whose end-of-turn signal may never arrive. For a `Probing` target that is
  hooked, the broker ignores the quiet fallback while `turn_active` is set, so a turn with a long
  silent tool run (SC-003) ends only on `Stop`, `StopFailure`, `Interrupt`, or the ask's timeout.
  The fallback applies again only if `turn_active` was never set for the ask (the prompt hook did
  not fire).

**Rationale**: Uses Kimi's own signal when present and costs nothing when absent; no config file
read at ask time.

**Alternatives considered**: Deciding by the config status: wrong for Kimi processes started
before Apply, and needs a file read per ask.

## R6. Session restore (feature 009)

**Decision**: Kimi plugs into feature 009 without new mechanisms:

- `Conversation.agent` gains `"kimi"`. Kimi ids are `session_<uuid>` (44 characters,
  `[a-z0-9_-]`, `kimi:…/kimi-tui.ts`), which pass 009's id rule.
- Resume command: `kimi --session <id> [args…]`, and never `--agent-file` (R2).
- The wrapper adds no resume when the user's arguments contain any flag listed in R2 or a
  subcommand (`login`, `acp`, `web`, `app`, `install-desktop`, `doctor`, `export`, `fork`,
  `session`, `provider`, `vis`, `migrate`, `upgrade`, `update`).
- Failure: `kimi --session <unknown id>` fails before the interface opens with
  `Session "<id>" not found.` and exit status 1 (`kimi:…/kimi-tui.ts:999`, `main.ts`). Sessions
  are stored per folder, so a folder change (lane land) fails the same way. Feature 009's 10 s
  child-and-fallback rule covers both. Unverified live: the exact exit status.
- Print mode `-p`/`--prompt`: Kimi fires hooks in print mode, so the wrapper sets
  `SONATA_CONVERSATION=off` (009 R9) and the Kimi hooks skip conversation reports.
- Without the integration, no conversation is ever recorded, so 009 never offers a resume and Kimi
  starts fresh (FR-018).

**Dependency**: this part needs 009 merged. The branch is rebased on `main` after 009 lands, and
phases are ordered so everything else is done first.

## R7. Environment markers

**Finding**: Kimi sets no session marker for its children. Its shell tool adds only `NO_COLOR`,
`TERM=dumb`, `GIT_TERMINAL_PROMPT=0`, and `SHELL` (`kimi:…/agent/tools/os/bash/bashTool.ts:165`).
Plugin hooks get `KIMI_PLUGIN_ROOT` (and `KIMI_CODE_HOME`); `KIMI_CODE_UPDATE_REEXEC` exists only
during self-update.

**Decision**: Add `KIMI_PLUGIN_ROOT` and `KIMI_CODE_UPDATE_REEXEC` to 009's
`AGENT_SESSION_MARKERS`. `TERM`, `NO_COLOR`, and `GIT_TERMINAL_PROMPT` are already set by Sonata's
spawn (`TERM`) or harmless to inherit; `NO_COLOR` from a Kimi shell would gray out a nested
Sonata's terminals, so it is not stripped (it is a common user setting too). `KIMI_CODE_HOME` is a
user setting and is kept (FR-019).

## R8. Preset, recruit, and typed agents

**Decision**:

- `PRESETS` gains `{ id: "kimi", name: "Kimi Code", command: "kimi" }`, before Shell. Preset
  detection already checks each command on the login shell's `PATH`.
- `sonata recruit kimi` works through the same preset table; the usage text and the Orchestrator
  instructions list `kimi`.
- zsh helpers gain `kimi() { command sonata run kimi "$@" }`, so a `kimi` typed by hand gets the
  agent file and hooks report to the right terminal.
- The terminal dialog's "send instructions" default treats `kimi` like `claude` (off by default).

## R9. Notice and setup view

**Decision**:

- **Status source**: `core` computes the status (R4) on request and when the file changes. The
  app already uses `notify` for file watching; it watches the Kimi folder only while a Kimi
  terminal exists or the Settings window is open.
- **Notice**: a one-line strip inside Kimi terminal nodes when the status is off or out of date:
  "Kimi: busy/done tracking and conversation restore are off. Set up · Dismiss". Set up opens
  Settings at the Kimi section.
- **Dismissal**: `Preferences.kimiNoticeDismissed` stores the status the user dismissed (`off` or
  `outdated`). The notice stays hidden while the status equals that value and shows again when it
  changes (FR-008).
- **Settings > Agents > Kimi Code**: status, config file path, the exact block with a one-line
  explanation per hook, Apply / Remove buttons (each behind the existing confirmation dialog),
  "Copy lines" for users who prefer to paste, a note that running Kimi terminals need Restart, and
  the backup path after each change.
