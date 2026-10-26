# Tasks: Kimi Code CLI as a Supported Agent

**Input**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Required (constitution III). Each test task comes before its implementation task and
must fail first. No test runs the real `kimi` or touches `~/.kimi-code`: Kimi is a fake script in
`SONATA_AGENT_BIN_DIR`, and config tests use a temporary `KIMI_CODE_HOME`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: User story from spec.md (US1–US5)

## Phase 1: Setup

- [X] T001 Create the module `crates/core/src/agents/{mod,kimi}.rs` (empty `pub mod kimi;`) and register `pub mod agents;` in crates/core/src/lib.rs
- [X] T002 [P] Add config fixtures in crates/core/tests/fixtures/kimi/: `empty.toml` (0 bytes), `user.toml` (providers, a comment, a user `[[hooks]]` for `Notification`, no final newline), `on-v1.toml` (user.toml + the exact block of contracts/kimi-config.md), `outdated.toml` (user.toml + a Sonata `[[hooks]]` table with `event = "Stop"` and `timeout = 30`, no markers), `inline-hooks.toml` (`hooks = []`), `hooks-table.toml` (`[hooks]` header), `rewritten.toml` (on-v1.toml with the marker comments removed, as Kimi's full re-serialize would leave it)
- [X] T003 [P] Add a `FAKE_KIMI` script constant and an `install_fake_kimi(dir)` helper in crates/core/tests/common/mod.rs: records `"$@"` to `$FAKE_LOG/kimi.args`; when its args contain `--session session_missing` prints `error: failed to start shell: Session "session_missing" not found.` to stderr and exits 1; otherwise reads lines from stdin and for each line pipes `{"hook_event_name":"UserPromptSubmit","session_id":"session_test1","cwd":"."}` to `sonata _hook kimi prompt-submit`, echoes `kimi:<line>`, then pipes `{"hook_event_name":"Stop","session_id":"session_test1"}` to `sonata _hook kimi stop`; with `FAKE_KIMI_PAUSE=<seconds>` it sleeps that long, printing nothing, between the echo and the `stop` hook; with `FAKE_KIMI_NO_HOOKS=1` it only echoes

## Phase 2: Foundational (blocks every story)

- [X] T004 Add error codes `KimiUnavailable`, `KimiHooksConflict`, `KimiConfigInvalid`, `KimiCheckFailed` (contracts/ipc.md) in crates/core/src/error.rs and the matching union members in ui/src/ipc/types.ts
- [X] T005 [P] Write a unit test that `instructions()` for any `SessionInfo` (with role, lane, orchestrator) never contains `${`, because Kimi renders the agent-file body as a template (research R2), in crates/core/src/messaging/instructions.rs (`#[cfg(test)]`)

**Checkpoint**: module, fixtures, fake `kimi`, and error codes exist; T005 passes on today's text.

## Phase 3: User Story 1 — Start Kimi from a preset and have it know Sonata (P1) 🎯 MVP

**Goal**: A Kimi terminal starts from a preset, gets Sonata's instructions through an agent file,
and answers asks with the quiet-screen rule. No change to Kimi's config.
**Independent Test**: quickstart live check 1; automated: `cargo test -p sonata-core kimi_session`.

- [X] T006 [P] [US1] Write tests in crates/core/tests/presets.rs: `PRESETS` contains `{ id: "kimi", name: "Kimi Code", command: Some("kimi") }` placed before `shell`; `detect` reports `kimi` found when a fake `kimi` is on the test shell's `PATH`
- [X] T007 [US1] Add the Kimi Code preset in crates/core/src/presets.rs
- [X] T008 [P] [US1] Write tests in crates/core/src/terminal/pty.rs (`#[cfg(test)]`): `WRAPPED_AGENTS` contains `kimi`; `wrap_launch_command("kimi --yolo", true)` → `sonata run kimi --yolo`; `wrap_launch_command("kimi", false)` → `kimi`
- [X] T009 [US1] Add `kimi` to `WRAPPED_AGENTS` in crates/core/src/terminal/pty.rs (array length 3); add `kimi() { command sonata run kimi "$@" }` to `HELPERS` and the module comment in crates/core/src/terminal/zsh.rs; extend crates/core/tests/zsh_helpers.rs to assert the `kimi` helper
- [X] T010 [P] [US1] Write unit tests for `kimi_agent_file(info)` in crates/core/src/messaging/instructions.rs (`#[cfg(test)]`): starts with `---\nname: sonata-session\ndescription: ` frontmatter and `---`; body contains `${base_prompt}` and `${plugin_sections}` before the Sonata text; contains the role text and `sonata list`; has no `tools` key
- [X] T011 [US1] Implement `kimi_agent_file(info) -> String` and write `kimi-agent.md` in `write_session` (add `kimi_agent: PathBuf` to `SessionFiles`) in crates/core/src/messaging/instructions.rs; list `kimi` in the Orchestrator `sonata recruit <claude|codex|kimi|opencode|gemini|shell>` line
- [X] T012 [P] [US1] Write tests in crates/core/tests/kimi_hooks.rs (no environment changes, so several tests share the binary): `_session kimi <hook>` from a terminal replies with `data.agent == "kimi"` and `data.agentFile` pointing to an existing `run/sessions/<node>/kimi-agent.md` holding the terminal's role; the file is removed when the terminal is deleted (that is where 003 removes session files)
- [X] T013 [US1] Return `agentFile` for `kimi` in `session()` in crates/core/src/messaging/commands.rs (keep the existing fields for `claude`/`codex`)
- [X] T014 [P] [US1] Write wrapper tests in crates/core/tests/kimi_hooks.rs with the fake `kimi` (T003), running `sonata run kimi` with its own environment: a Kimi terminal with launch command `kimi --yolo` starts `kimi --agent-file <…/kimi-agent.md> --yolo`; for each of `-S x`, `--session x`, `-r x`, `--resume x`, `-c`, `--continue`, `-C`, `--agent a`, `--agent-file f`, `-p hi`, `--prompt hi`, and first-argument subcommands `login acp web app install-desktop doctor export fork session provider vis migrate upgrade update`, the args are passed unchanged with no `--agent-file`; outside Sonata (no socket) `sonata run kimi x` execs `kimi x` unchanged
- [X] T015 [US1] Implement the `kimi` branch in crates/cli/src/run.rs: `fn kimi_chooses(args: &[String]) -> bool` with the flag and subcommand lists of contracts/cli.md; add `--agent-file <data.agentFile>` only when it returns false; update the usage text to `sonata run <claude|codex|kimi> [args…]`
- [X] T016 [P] [US1] Write a test in crates/core/tests/kimi_hooks.rs that `sonata recruit kimi --name K` creates a terminal with launch command `kimi` linked to the caller
- [X] T017 [US1] Accept `kimi` in `recruit` and its usage text in crates/core/src/messaging/commands.rs and in recruit help in crates/cli/src/main.rs
- [X] T018 [P] [US1] Write a test in crates/core/tests/kimi_session.rs that a Kimi terminal does not get the typed instruction message by default, and does when `send_instructions = Some(true)`
- [X] T019 [US1] In `maybe_type_instructions` in crates/core/src/app.rs, keep `automatic = false` for `kimi` (only `codex` stays true in the `Hooked` branch); check after T009 that the broker still uses the quiet rule for Kimi until US3 (temporary: `target_kind` returns `Other` for `kimi`, replaced in T047)
- [X] T020 [P] [US1] Write tests in ui/tests/terminalDialog.test.tsx: the preset list shows "Kimi Code"; choosing it sets the command to `kimi`; the "send instructions" checkbox defaults to off for `kimi`
- [X] T021 [US1] Treat `kimi` like `claude` for the "send instructions" default in ui/src/dialogs/TerminalDialog.tsx (line with `["", "claude"]`)

**Checkpoint**: US1 works alone without any Kimi config change.

## Phase 4: User Story 2 — Set up the Kimi integration once (P1)

**Goal**: Status on/off/outdated/unavailable; confirmed Apply and Remove with backup, permission
bits kept, `kimi doctor` check and rollback; notice and Settings section.
**Independent Test**: quickstart live checks 2, 3, 6; automated: `cargo test -p sonata-core kimi_config`.

### Core

- [ ] T022 [P] [US2] Write unit tests for the table scanner in crates/core/src/agents/kimi.rs (`#[cfg(test)]`): splits a file into `[[hooks]]` tables from the header line to the next line starting with `[` after optional spaces or end of file; a table is Sonata's when its `command` contains `sonata _hook kimi`; marker lines directly before/after Sonata's tables belong to the block; `^\s*hooks\s*=` and `^\s*\[hooks\]` outside Sonata's tables are a conflict
- [ ] T023 [US2] Implement the scanner, `BLOCK_V1` (exact text of contracts/kimi-config.md), `KIMI_BLOCK_VERSION = 1`, and `status(text) -> KimiStatus` (`on` = Sonata's tables are exactly the five tables of the block, compared on `event`, `command`, `timeout` after trimming spaces; `outdated` = at least one Sonata table, not exactly these; `off` = none) in crates/core/src/agents/kimi.rs
- [ ] T024 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for `status` on every fixture of T002: `empty`/`user` → off, `on-v1` → on, `rewritten` → on, `outdated` → outdated, `inline-hooks`/`hooks-table` → off with `canApply == false` and a message naming the line. Also test the block as Kimi runs it: take each `command` value of `BLOCK_V1`, unescape it as a TOML basic string, and run it with `/bin/sh -c` and a JSON payload on stdin; without `SONATA_TERMINAL_ID` stdout and stderr are empty, exit status is 0, and a fake `sonata` on `PATH` is never called; with `SONATA_TERMINAL_ID` set, the fake `sonata` is called with `_hook kimi <event>` and receives the payload; when the fake `sonata` exits 1, the command still exits 0
- [ ] T025 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for `config_path`: `KIMI_CODE_HOME` from the login shell (a fake shell script printing a value) wins; empty value → `~/.kimi-code/config.toml`; folder missing → status `unavailable` with "Run Kimi once first"; unreadable file → `unavailable` with the read error
- [ ] T026 [US2] Implement `config_path(shell)` (login shell `-l -c 'printf %s "$KIMI_CODE_HOME"'`, 2 s timeout like `presets::detect`) and `integration(shell) -> KimiIntegration` (fields of data-model.md) in crates/core/src/agents/kimi.rs
- [ ] T027 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for `apply`: `user.toml` → final newline added, one blank line, block appended, every original byte unchanged before it; `outdated.toml` → old Sonata table removed and current block appended; conflict fixtures → `KimiHooksConflict` and file unchanged; existing file mode `0600` and `0644` kept; missing file with existing folder → created with mode `0600`; status after apply is `on`
- [ ] T028 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for `remove`: `on-v1.toml` → exactly `user.toml` plus a final newline; `rewritten.toml` → same; user `Notification` hook kept; status after remove is `off`
- [ ] T029 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for backups: each apply/remove writes `config.toml.sonata-backup-<YYYYMMDD-HHMMSS>` with the previous content and the same mode; only the three newest Sonata backups remain; files not matching the pattern are never deleted; `lastBackup` returns the newest
- [ ] T030 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for the checks before and after write (research R4), with a fake `kimi` on the login shell's `PATH`: `doctor` fails before any change → Apply and Remove return `KimiConfigInvalid` with Kimi's message, no backup is written, and the file is unchanged; `doctor` exits 0 → change kept; `doctor` passes before and exits 1 with a message after → backup restored byte for byte, error `KimiCheckFailed` carries the message; no `kimi` on `PATH` → change kept without either check and the integration says the file was not validated; `doctor` hangs → killed after 10 s, treated as failure
- [ ] T031 [US2] Implement `apply`, `remove`, backup rotation, atomic write keeping permission bits (reuse crates/core/src/persistence/atomic.rs, then `set_permissions` before `rename`), and the `kimi doctor` checks (before: stop with `KimiConfigInvalid`; after: roll back with `KimiCheckFailed`) in crates/core/src/agents/kimi.rs
- [ ] T032 [P] [US2] Write tests in crates/core/tests/preferences_update.rs: `kimiNoticeDismissed` round-trips as `"off"`/`"outdated"`, is omitted when `None`, and an unknown value fails to load as other invalid preferences do
- [ ] T033 [US2] Add `kimi_notice_dismissed: Option<KimiNoticeDismissed>` (`"off"` | `"outdated"`, `skip_serializing_if = "Option::is_none"`) in crates/core/src/model/preferences.rs
- [ ] T034 [P] [US2] Write tests in crates/core/tests/kimi_config.rs for `App`: `kimi_integration()`, `kimi_integration_apply()`, `kimi_integration_remove()`, `dismiss_kimi_notice(status)`; each change emits one `KimiIntegrationChanged` on the `RecordingSink`; an outside edit of the watched file emits `KimiIntegrationChanged` within 2 s; the watcher runs only while a Kimi terminal exists or `watch_kimi(true)` was called (Settings open)
- [ ] T035 [US2] Implement the `App` methods, the `KimiIntegrationChanged` event in crates/core/src/events.rs, and the watcher (existing `notify` debouncer, like `notes::watcher`) in crates/core/src/app.rs

### Shell and UI

- [ ] T036 [US2] Add src-tauri/src/commands/agents.rs with `kimi_integration`, `kimi_integration_apply`, `kimi_integration_remove`, `dismiss_kimi_notice`, `kimi_watch(on)`; register in src-tauri/src/commands/mod.rs, src-tauri/src/lib.rs, src-tauri/build.rs, src-tauri/capabilities/default.json; forward `KimiIntegrationChanged` as an event
- [ ] T037 [P] [US2] Add `KimiStatus`, `KimiIntegration`, `kimiNoticeDismissed`, the commands, and the `KimiIntegrationChanged` listener in ui/src/ipc/types.ts, ui/src/ipc/commands.ts, ui/src/ipc/events.ts
- [ ] T038 [P] [US2] Write tests in ui/tests/kimiSettings.test.tsx: Settings nav shows "Agents"; the Kimi Code section shows status, path, and the block; Apply opens the confirmation with the text of contracts/ipc.md and calls `kimiIntegrationApply` only on confirm; Cancel calls nothing; Remove likewise; after a change shows "Backup saved to <path>. Restart running Kimi terminals to use the change."; `canApply == false` disables Apply and shows `message`; Copy lines writes the block to the clipboard
- [ ] T039 [US2] Implement ui/src/settings/AgentSettings.tsx and the "Agents" nav entry in ui/src/settings/SettingsApp.tsx (reuse ui/src/dialogs/ConfirmDialog.tsx); call `kimiWatch(true)` while mounted
- [ ] T040 [P] [US2] Write tests in ui/tests/kimiNotice.test.tsx: a Kimi terminal node shows the off/outdated line with Set up and Dismiss; a Claude node never shows it; status `on` hides it; Dismiss calls `dismissKimiNotice(status)` and hides it; it shows again when the status changes to a different value; Set up opens Settings at the Agents section
- [ ] T041 [US2] Render the notice strip in ui/src/nodes/TerminalNode.tsx (Kimi = first word of the launch command is `kimi`), reading status from the `KimiIntegrationChanged` state and `kimiNoticeDismissed` from preferences

**Checkpoint**: Apply/Remove work end to end; nothing changes without confirmation.

## Phase 5: User Story 3 — Sonata knows exactly when Kimi is busy and done (P2)

**Goal**: With hooks, asks to Kimi end on `Stop`/`StopFailure`/`Interrupt`; without hooks, the
quiet rule stays.
**Independent Test**: quickstart live check 4; automated: `cargo test -p sonata-core agent_hooks kimi_session`.

- [ ] T042 [P] [US3] Write tests in crates/core/tests/agent_hooks.rs for `sonata _hook kimi <event>` with payloads on stdin: `prompt-submit` sends `_turn start` (and, once 009 is merged, `_conversation kimi <session_id>`); `stop`, `stop-failure`, `interrupt` send `_turn complete` with no text; stdout and stderr are empty in every case; exit status is 0 for unknown events, invalid JSON, and when `SONATA_TERMINAL_ID` is unset
- [ ] T043 [US3] Implement the `kimi` arm in crates/cli/src/hook.rs (`Some("kimi")` → match `args.get(1)`), reading stdin fully before reporting
- [ ] T044 [P] [US3] Write tests in crates/core/tests/kimi_session.rs for the hooked state: a new Kimi terminal process is not hooked; any `_turn start`, `_turn complete`, or `_conversation` from it marks it hooked; Restart resets it; once hooked, `sonata list` shows the Kimi terminal as `working` between `_turn start` and `_turn complete` and `idle` otherwise, and the activity check (`App::check_activity`) reports the same busy/idle changes; a Claude Code terminal that sends `_turn start` keeps today's foreground-process status
- [ ] T045 [US3] Add the per-process-generation `hooked` and `turn_active` flags in crates/core/src/terminal/manager.rs (reset on spawn); set them from `_turn start` / `_turn complete` (and `_conversation` when present) in crates/core/src/messaging/commands.rs; for Kimi terminals that are hooked, the `working`/`idle` status in `sonata list` (crates/core/src/messaging/commands.rs, the `RunStatus::Running` match) and the canvas activity check (`check_activity` in crates/core/src/app.rs) use `turn_active` instead of the foreground-process check; Claude Code and Codex keep the foreground-process check
- [ ] T046 [P] [US3] Write broker tests in crates/core/tests/kimi_session.rs with the fake `kimi`: unhooked (`FAKE_KIMI_NO_HOOKS=1`) ask returns after 3 s of quiet; hooked ask with `FAKE_KIMI_PAUSE=25` (silent 25 s, longer than the 20 s `HOOKED_QUIET_FALLBACK`) returns only after `stop`, within 2 s of it, with the full answer (SC-003); a hooked ask whose prompt hook never fires (no `_turn start`) still ends by the 20 s quiet fallback; an ask that started `Probing` switches to `Hooked` when `_turn start` arrives during it
- [ ] T047 [US3] Add `TargetKind::Probing` in crates/core/src/messaging/broker.rs (behaves like `Other` until the target is hooked, then like `Hooked`, checked each tick; while the target's `turn_active` is set, the quiet fallback is suspended and only the end-of-turn report or the ask timeout ends the ask, research R5); handle `Probing` in the `maybe_type_instructions` match in crates/core/src/app.rs (`automatic = false`); `target_kind` returns `Probing` for `kimi` in crates/core/src/messaging/commands.rs (replaces the temporary `Other` of T019)

**Checkpoint**: US1–US3 done; phases 1–5 can merge before 009.

## Phase 6: User Story 4 — Kimi conversations come back after Sonata reopens (P2) — needs 009

**Goal**: Feature 009's restore works for Kimi with `kimi --session <id>`.
**Independent Test**: quickstart live check 5. Start only after 009 is on `main` and this branch
is rebased (`git rebase main`).

- [ ] T048 [US4] Rebase `010-kimi-agent` on `main` with 009 merged; run `sh scripts/check.sh` and fix conflicts in crates/cli/src/run.rs, crates/cli/src/hook.rs, crates/core/src/messaging/commands.rs
- [ ] T049 [P] [US4] Write tests in 009's conversation model tests (crates/core/tests/session_restore_*.rs): `Conversation { agent: "kimi", id: "session_3f2a0c1e-…" }` round-trips; `_conversation kimi session_x` stores it; `_conversation kimi none` clears only a `kimi` conversation
- [ ] T050 [US4] Add `kimi` to the `Conversation.agent` enum (`"claude"` | `"codex"` | `"kimi"`) in crates/core/src/model/node.rs and in ui/src/ipc/types.ts
- [ ] T051 [P] [US4] Write tests in crates/core/tests/agent_hooks.rs: `_hook kimi session-start` with `source=resume` sends `_conversation kimi <id>`; `source=startup` sends `_conversation kimi none`; any other source sends nothing; `prompt-submit` sends `_conversation kimi <session_id>` after `_turn start`; with `SONATA_CONVERSATION=off` no `_conversation` is sent but `_turn` is
- [ ] T052 [US4] Add the `session-start` arm and the `_conversation` reports to the `kimi` arm in crates/cli/src/hook.rs
- [ ] T053 [P] [US4] Write wrapper tests in crates/core/tests/session_restore_run.rs with the fake `kimi`: a resume offer starts `kimi --session <id> [args…]` with no `--agent-file`; args that choose (T014 list) consume the offer and run unchanged; `--session session_missing` fails → stderr has `sonata: the previous conversation could not be resumed; starting a new one.` once and the next start is `kimi --agent-file <f> [args…]`; `-p` sets `SONATA_CONVERSATION=off` in the child
- [ ] T054 [US4] Map the 009 resume offer to `kimi --session <id>` (never with `--agent-file`) and the `-p`/`--prompt` guard in crates/cli/src/run.rs, reusing 009's child-and-fallback code
- [ ] T055 [P] [US4] Write a test in crates/core/tests/kimi_session.rs that Start fresh on a Kimi terminal clears its conversation and the restart gets `--agent-file` and no `--session`

**Checkpoint**: Kimi restore matches Claude Code and Codex.

## Phase 7: User Story 5 — Clean environment for Kimi (P3) — needs 009

**Goal**: Kimi's markers are stripped from every terminal; user Kimi settings are kept.
**Independent Test**: quickstart live check 8.

- [ ] T056 [P] [US5] Extend 009's environment test (crates/core/tests/session_restore_env.rs or the file 009 used): with `KIMI_PLUGIN_ROOT` and `KIMI_CODE_UPDATE_REEXEC` set in the parent, a new terminal's `env` has neither; `KIMI_CODE_HOME` and `NO_COLOR` set in the parent are kept
- [ ] T057 [US5] Add `KIMI_PLUGIN_ROOT` and `KIMI_CODE_UPDATE_REEXEC` to `AGENT_SESSION_MARKERS` in crates/core/src/terminal/pty.rs

## Phase 8: Polish

- [ ] T058 [P] Add a "Kimi Code" section to README.md: preset, what works without setup, what Apply changes (path, the five hooks, backup, Remove), that running Kimi terminals need Restart after Apply, and the known `/clear` gap (research R2)
- [ ] T059 [P] Update `sonata help` text that lists agents (`run`, `recruit`) in crates/cli/src/main.rs
- [ ] T060 Run `sh scripts/check.sh` and fix every failure
- [ ] T061 When Kimi Code CLI 2.3+ is available, run the quickstart live checks 1–8 and record results (including `kimi doctor` and `kimi --session session_missing` exit codes) in specs/010-kimi-agent/quickstart.md; update research R4/R6 if they differ

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories.
- US1 first: US2's notice and US3's hooks need Kimi terminals that start through `sonata run`.
- US2 core (T022–T035) does not depend on US1 code and can run beside it; US2 UI needs T035.
- US3 needs US1 (wrapper) and the block of US2 for live testing; automated tests call the hooks
  directly, so T042–T047 need only US1.
- US4 and US5 need 009 on `main` (T048 first). Phases 1–5 can merge as one PR before 009.
- Within a story: tests → core → CLI → shell commands → UI.

## Parallel Examples

- Phase 1: T002 and T003 together.
- US1: T006, T008, T010, T012, T014, T016, T018, T020 together; then implementations.
- US2: T022, T024, T025, T027–T030, T032, T034 (separate files or sections) together; T037,
  T038, T040 alongside core work once T035 exists.
- US3 tests T042, T044, T046 together.
- US4: T049, T051, T053, T055 together after T048.

## Implementation Strategy

1. MVP = Phases 1–3 (US1): Kimi usable on the canvas with instructions, no config change.
   Validate quickstart live check 1 when Kimi is available.
2. Add US2 (setup), then US3 (exact busy/done). Open the PR for phases 1–5.
3. After 009 merges: rebase, US4, US5, polish.
4. Commit after each task or small group; Conventional Commits; no push without the user's ask.
