---

description: "Task list for feature 003: agent messaging over links, Sonata CLI, orchestrator"
---

# Tasks: Agent Messaging over Links, Sonata CLI, and Orchestrator Powers

**Input**: `/specs/003-agent-messaging/` (plan, spec, research, data-model, contracts)

**Tests**: Required (constitution III). Tests first; they must fail before implementation.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [X] T001 Create crate `crates/cli` (binary `sonata`, deps `serde`, `serde_json`) and add it to the workspace in `Cargo.toml`
- [X] T002 [P] Add `tauri-plugin-notification` and `tauri-plugin-single-instance` to `src-tauri`; register them; add `notification:default` to capabilities
- [X] T003 [P] Bundle the CLI: `beforeBuildCommand` builds `sonata` in release and copies it to `src-tauri/resources/bin/sonata`; `bundle.resources` maps it to `bin/`; `.gitignore` the copied binary

---

## Phase 2: Foundational (channel, identity, sessions)

### Tests

- [X] T004 [P] Socket tests in `crates/core/tests/socket.rs`: server listens at `<root>/run/sonata.sock` with mode `0600` in a `0700` folder; long roots fall back to `$TMPDIR/sonata-<uid>/`; a request with a wrong token gets exit code 3; malformed and oversized (> 128 KB) lines are rejected; the socket file is removed on quit
- [X] T005 [P] Session env tests in `crates/core/tests/terminal_pty.rs` (extend): a started terminal has `SONATA_TERMINAL_ID`, `SONATA_TOKEN` (64 hex chars, new on restart), `SONATA_SOCKET`, and the CLI folder on `PATH`
- [X] T006 [P] CLI end-to-end harness in `crates/core/tests/common/cli.rs`: build-or-locate `target/debug/sonata`, run it with a terminal's env, capture stdout and exit code; test `sonata` without env prints the "only inside Sonata" message and exits 6

### Implementation

- [X] T007 Generate per-start session tokens and env (`SONATA_TERMINAL_ID`, `SONATA_TOKEN`, `SONATA_SOCKET`, `PATH` prefix from `AppOptions.cli_dir`) in `crates/core/src/terminal/pty.rs` and `manager.rs`; expose `session_token(id)`
- [X] T008 Implement the socket server (`tokio` UnixListener on a dedicated runtime thread, JSON-lines request/progress/result, auth by token, size limits, cleanup) in `crates/core/src/messaging/server.rs`; start it from `App::start_with` when `AppOptions.socket` is enabled
- [X] T009 Implement the CLI skeleton (args parsing for every command in `contracts/cli.md`, client, `--json`, exit codes, `help`) in `crates/cli/src/`
- [X] T010 Pass the CLI folder to `AppOptions` from the shell (`Contents/Resources/bin` in the bundle, `target/debug` in dev) in `src-tauri/src/lib.rs`; single instance (FR-031)

**Checkpoint**: `sonata list` round-trips from a real terminal.

---

## Phase 3: User Story 1 - Ask and answer (P1) 🎯 MVP

### Tests

- [X] T011 [P] [US1] Extraction tests in `crates/core/tests/extract.rs`: rows after the anchor as plain text; anchor survives scrolling past the scrollback cap (absolute counter); trailing prompt line removed; alternate-screen fallback returns the visible screen with a note; ANSI stripped
- [X] T012 [P] [US1] Broker tests in `crates/core/tests/broker.rs`: FIFO per target; loop A→B→A refused with the chain; timeout returns partial and state `timedOut`; reply completes the oldest pending ask; target exit fails pending asks with partial output; asker close cancels; quit ends all
- [X] T013 [P] [US1] CLI e2e in `crates/core/tests/cli_ask.rs` with real shells: linked `ask B "echo hello"` prints `hello`; unlinked → exit 3 listing linked names; ambiguous name → exit 2; `--all` and comma list return labeled answers in parallel; `--keys ctrl-c` stops `cat`; `check` prints the screen; multi-line message arrives as one block; `reply` path; message over 100 KB refused; target in an unloaded workspace is started first

### Implementation

- [X] T014 [US1] Absolute row counter via `vt100` `on_scroll` and anchor/extract API in `crates/core/src/terminal/screen.rs`; `crates/core/src/messaging/extract.rs`
- [X] T015 [US1] Message delivery (bracketed paste when enabled, 80 ms, Enter) and key names in `crates/core/src/terminal/manager.rs` and `crates/core/src/messaging/commands.rs`
- [X] T016 [US1] Ask broker (`crates/core/src/messaging/broker.rs`): queues, wait graph, deadlines, heuristic finishing (3 s quiet after output, 30 s total silence, child-process check), reply, cancellation, progress lines, exchange log
- [X] T017 [US1] Commands `list`, `ask` (single, list, `--all`, `--keys`, `--timeout`), `check`, `reply` with link-based permissions and name resolution in `crates/core/src/messaging/commands.rs`; on-demand start of unloaded targets (R9)

**Checkpoint**: shells can talk to each other through links.

---

## Phase 4: User Story 2 - Agents know Sonata (P1)

### Tests

- [X] T018 [P] [US2] Instructions tests in `crates/core/tests/agent_hooks.rs`: `sonata run claude` writes `instructions.md` (name, workspace, role, commands) and `settings.json` (only Stop and UserPromptSubmit hooks) into the run folder and execs a fake `claude` script with `--append-system-prompt-file` and `--settings`; `~/.claude` untouched (HOME pointed at a temp folder); launch commands starting with `claude`/`codex` are typed as `sonata run …`
- [X] T019 [P] [US2] Hook tests in `crates/core/tests/agent_hooks.rs`: a fake Claude transcript JSONL + `sonata _hook claude-stop` completes the pending ask with the last assistant text; `sonata _hook codex-notify '<json>'` completes it with `last-assistant-message`; typed instructions are sent once when `sendInstructions` is on

### Implementation

- [X] T020 [US2] `crates/core/src/messaging/instructions.rs` (template, role, command list) and session run folder lifecycle
- [X] T021 [US2] `sonata run` (resolve real agent on `PATH`, build args, exec) and `_hook` subcommands in `crates/cli/src/run.rs` and `crates/cli/src/hook.rs`; server handlers for `turn_complete` and `turn_start`
- [X] T022 [US2] Rewrite launch commands for known agents in `crates/core/src/terminal/pty.rs`; typed instructions fallback with `sendInstructions` (FR-019) and its checkbox in `ui/src/dialogs/TerminalDialog.tsx`

---

## Phase 5: User Story 3 - Orchestrator (P2)

### Tests

- [X] T023 [P] [US3] Orchestrator tests in `crates/core/tests/orchestrator.rs`: recruit with preset, `--role` (library), `--role-text`, `--folder`; placement without overlap; unique name suffix; recruitedBy saved; dismiss own recruit only; link/unlink by name in the workspace; non-orchestrator gets exit 3 for all four; UI event `workspace://nodes-changed` emitted

### Implementation

- [X] T024 [US3] Placement (R10) and `recruit`/`dismiss`/`link`/`unlink` in `crates/core/src/messaging/commands.rs` using `App` node and link operations; `recruitedBy` field in `crates/core/src/model/node.rs`
- [X] T025 [US3] UI: apply `workspace://nodes-changed` in `ui/src/state/canvas.ts` and `ui/src/App.tsx`

---

## Phase 6: User Story 4 - Notes and notifications (P2)

### Tests

- [X] T026 [P] [US4] CLI tests in `crates/core/tests/cli_notes.rs`: read/write/append linked notes by title and by short id; unlinked → exit 3; canvas event `note://changed` emitted; conflict rule while dirty; `notify` emits `messaging://notify`

### Implementation

- [X] T027 [US4] `note read|write|--append` and `notify` handlers in `crates/core/src/messaging/commands.rs`; note titles and short ids
- [X] T028 [US4] Shell: show notifications with `tauri-plugin-notification`; UI selects the notifying terminal on next window focus within 60 s

---

## Phase 7: User Story 5 - Visibility (P3)

### Tests

- [X] T029 [P] [US5] UI tests in `ui/tests/messaging.test.tsx`: `messaging://ask` waiting highlights the link and shows "asked by"; terminal states clear them; the log panel lists entries from `exchange_log`

### Implementation

- [X] T030 [US5] Events from the broker; command `exchange_log`; link glow in `ui/src/canvas/LinkLayer.tsx`; "asked by" chip in `ui/src/nodes/TerminalNode.tsx`; log panel from the terminal toolbar in `ui/src/canvas/NodeToolbar.tsx`

---

## Phase 8: Polish

- [X] T031 [P] README: the `sonata` tool, `sonata run claude`, orchestrator commands, safety notes
- [X] T032 Run `scripts/check.sh`, `npm run e2e`; fix failures
- [X] T033 Build and install the app (`scripts/install.sh`); run quickstart steps that need no person; record the rest in `specs/003-agent-messaging/quickstart-results.md`

## Dependencies

Setup → Foundational → US1 → US2 → (US3, US4, US5) → Polish.
