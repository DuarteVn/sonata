# Tasks: Session Restore for Agent Terminals

**Input**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: Required (constitution III). Each test task comes before its implementation task and
must fail first. Agent tests use fake `claude`/`codex` scripts in a temporary
`SONATA_AGENT_BIN_DIR` (as in crates/core/tests/agent_hooks.rs) that record their arguments and
exit with a chosen status after a chosen delay. Hook tests feed fake JSON payloads on stdin or
argv. No test runs a real agent.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: User story from spec.md (US1–US5)

## Phase 1: Setup

- [x] T001 Add a fake-agent helper to crates/core/tests/common/mod.rs: `install_fake_agent(dir, name, exit_code, delay_ms)` writes an executable script that appends its argv (one line, space-joined) to `<dir>/<name>.args` and exits with `exit_code` after `delay_ms`; reuse the `install` pattern from crates/core/tests/agent_hooks.rs

## Phase 2: Foundational (blocks US1, US3, US4, US5)

- [x] T002 [P] Write unit tests for `Conversation` in crates/core/src/model/node.rs (`#[cfg(test)]`): JSON round-trip with keys `agent`, `id`, `recordedAt`; `TerminalContent` without a conversation omits the field; `agent` accepts only `"claude"` and `"codex"`; `validate_conversation_id` accepts `7294d08d-ac3c-4e7e-925e-16750c04c269` and rejects empty, 129 characters, and any character outside `[A-Za-z0-9_-]` (space, `;`, `/`, `$`, newline)
- [x] T003 Add `Conversation { agent, id, recorded_at }` and `TerminalContent.conversation: Option<Conversation>` (`skip_serializing_if = "Option::is_none"`, `#[serde(default)]`) plus `validate_conversation_id` ("1–128 characters of `[A-Za-z0-9_-]`") in crates/core/src/model/node.rs; set `conversation: None` in `TerminalContent::new`; no schema bump (research R6)
- [x] T004 [P] Write persistence tests in crates/core/tests/session_restore_persistence.rs: a workspace with a terminal conversation saves and loads unchanged with sorted keys; a file written before this feature (no field) loads with `conversation: None`
- [x] T005 [P] Write core tests in crates/core/tests/session_restore_record.rs for `App::record_conversation(node, agent, Some(id))` and `App::record_conversation(node, agent, None)`: a new value is stored and the workspace file on disk contains it immediately (no autosave tick); the same value again does not rewrite the file (compare mtime); `None` for the same agent removes the field; `None` for the other agent leaves it; an invalid id returns an error and stores nothing; a `NodesChanged` event is emitted only on change
- [x] T006 Implement `App::record_conversation` in crates/core/src/app.rs: validate, update the node through `update_node`, save that workspace at once with the existing atomic save, emit `NodesChanged` only on change (research R6)
- [x] T007 [P] Write CLI-command tests in crates/core/tests/session_restore_record.rs using the existing in-process socket helpers (`messaging_app`, `sonata_as`): `_conversation claude <id>` from a terminal stores on that terminal only; `_conversation codex none` removes a codex conversation; bad agent or bad id → exit 2 and nothing stored; a caller that is not a terminal is refused
- [x] T008 Add `["_conversation", agent, id]` to the command table in crates/core/src/messaging/commands.rs, mapping `none` to `None` and calling `App::record_conversation` for `ctx.me` (contracts/cli.md)

**Checkpoint**: conversations can be stored, cleared, and saved; nothing resumes yet.

## Phase 3: User Story 1 — Agents come back in their conversation (P1) 🎯 MVP

**Goal**: After reopen or Restart, the first agent started in a terminal resumes its stored
conversation.
**Independent Test**: quickstart A.

- [x] T009 [P] [US1] Write tests for the resume offer in crates/core/tests/session_restore_offer.rs: a terminal whose node stores a claude conversation gets `data.resume = {agent:"claude", id}` on its first `_session claude <hook>`; the second `_session` has no `resume`; `_session codex` for a stored claude conversation has no `resume` and consumes the offer; after `restart_terminal` the offer is available again; a new `App` on the same data root (app reopen) offers it; a node without conversation never gets `resume`
- [x] T010 [US1] Track a one-time resume offer per terminal process generation in crates/core/src/terminal/manager.rs (set at start and restart from the node's conversation; `take_resume_offer(node)` returns it once) and set it from `start_terminal` in crates/core/src/app.rs (data-model.md "Resume offer")
- [x] T011 [US1] Return the offer from `prepare_agent_session` (or a new `App::take_resume_offer`) and add the optional `resume` object to the `session` reply in crates/core/src/messaging/commands.rs (contracts/cli.md `_session`)
- [x] T012 [P] [US1] Write wrapper tests in crates/core/tests/session_restore_run.rs with fake agents: with an offer, `sonata run claude --model x` runs `claude --resume <id> --model x` with the session `--settings` and `--append-system-prompt-file` still present; `sonata run codex` runs `codex resume <id>` plus the `-c notify=…` override; with user arguments `-r`, `--resume x`, `-c`, `--continue`, `--session-id u`, `--from-pr 1`, `--teleport`, `-p hi`, `--print`, or a subcommand (`claude mcp list`, `codex exec x`) no resume is added; without an offer the arguments are unchanged; the child exit status is passed through
- [x] T013 [US1] Implement resume in crates/cli/src/run.rs: read `data.resume`; skip it per research R3 rules; when used, insert `--resume <id>` (claude) or `resume <id>` as the first arguments (codex), run the agent as a child, ignore `SIGINT`/`SIGQUIT` with `libc::signal` while waiting and restore them after, and exit with the child's status (exit code 128+signal when killed); keep `exec` when no resume is used
- [x] T014 [US1] Add a test in crates/core/tests/session_restore_offer.rs that a resumed terminal still gets the session instructions typed once (feature 003 FR-019) and the same settings file as a fresh start, so resume changes nothing but the conversation; fix crates/core/src/app.rs if it fails

**Checkpoint**: US1 works with a conversation recorded through `_conversation` (US3 makes the
recording automatic; until then, test with the command).

## Phase 4: User Story 2 — Conversation saving works in every terminal (P1)

**Goal**: No terminal inherits agent session markers.
**Independent Test**: quickstart B. Independent of every other phase.

- [x] T015 [P] [US2] Write tests in crates/core/tests/session_restore_env.rs: with `CLAUDECODE=1`, `CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDE_CODE_MESSAGING_TOKEN=secret`, `CLAUDE_PID=1`, `CODEX_SANDBOX=seatbelt`, `CLAUDE_CODE_USE_BEDROCK=1`, and `ANTHROPIC_MODEL=x` set in the test process, a spawned terminal running `env` prints none of the markers and still prints `CLAUDE_CODE_USE_BEDROCK=1` and `ANTHROPIC_MODEL=x`; `SONATA_TERMINAL_ID` is still set. Run in one test function because it changes the process environment
- [x] T016 [US2] Add `pub const AGENT_SESSION_MARKERS: [&str; 12]` with exactly `CLAUDECODE`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_EXECPATH`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_SESSION_ATTENDED`, `CLAUDE_CODE_MESSAGING_SOCKET`, `CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_PID`, `CLAUDE_EFFORT`, `CODEX_SANDBOX`, `CODEX_SANDBOX_NETWORK_DISABLED` (research R5) in crates/core/src/terminal/pty.rs and call `cmd.env_remove` for each next to `SONATA_HOME`, before Sonata's own variables
- [x] T017 [P] [US2] Write a test that the usage runner's child does not see the markers (fake provider command that prints `env`) in crates/core/tests/session_restore_env.rs
- [x] T018 [US2] Remove `AGENT_SESSION_MARKERS` from the child environment in crates/core/src/usage/runner.rs

**Checkpoint**: quickstart B passes.

## Phase 5: User Story 3 — The remembered conversation follows the agent (P2)

**Goal**: Hooks report every conversation switch automatically.
**Independent Test**: quickstart C.

- [x] T019 [P] [US3] Write a unit test in crates/core/src/messaging/instructions.rs (`#[cfg(test)]`) that the settings file has a `SessionStart` hook running `'<sonata>' _hook session-start`, next to the existing `Stop` and `UserPromptSubmit` hooks
- [x] T020 [US3] Add the `SessionStart` hook to `write_session` in crates/core/src/messaging/instructions.rs
- [x] T021 [P] [US3] Write hook tests in crates/core/tests/session_restore_hooks.rs (fake terminal through `sonata_as`): `_hook session-start` with stdin `{"session_id":"a1","source":"resume"}` stores `claude/a1`; with `source` `startup` or `clear` removes a stored claude conversation; with `compact` changes nothing; `_hook turn-start` with `{"session_id":"b2"}` still reports turn start and stores `claude/b2`; `_hook codex-notify '{"type":"agent-turn-complete","thread-id":"c3","last-assistant-message":"x"}'` still reports completion and stores `codex/c3`; with `SONATA_CONVERSATION=off` none of them changes the conversation; malformed JSON changes nothing and every hook exits 0
- [x] T022 [US3] Implement `session-start`, the `session_id` read in `turn-start`, and the `thread-id` read in `codex-notify` in crates/cli/src/hook.rs, sending `_conversation <agent> <id|none>` after the existing `_turn` report and honoring `SONATA_CONVERSATION=off` (contracts/cli.md hooks table)
- [x] T023 [P] [US3] Write a wrapper test in crates/core/tests/session_restore_run.rs: `sonata run claude -p hi` and `--print` start the child with `SONATA_CONVERSATION=off` (fake agent prints its env); interactive runs do not set it
- [x] T024 [US3] Set `SONATA_CONVERSATION=off` for print-mode children in crates/cli/src/run.rs (research R9)

**Checkpoint**: quickstart A and C pass end to end without calling `_conversation` by hand.

## Phase 6: User Story 4 — Start fresh (P2)

**Goal**: The user can forget a terminal's conversation and restart in a new one.
**Independent Test**: quickstart D.

- [x] T025 [P] [US4] Write core tests in crates/core/tests/session_restore_record.rs for `App::start_fresh_conversation(node, confirmed)`: a busy terminal with `confirmed = false` returns `TerminalBusy` with the same message as `restart_terminal` and keeps the conversation; otherwise the conversation is removed from the node and from the file on disk, the terminal restarts, and the next `_session` has no `resume`; an unknown node returns `NodeNotFound`; the agent's files are untouched (a temp file standing for the transcript still exists)
- [x] T026 [US4] Implement `App::start_fresh_conversation` in crates/core/src/app.rs reusing the busy check of `restart_terminal` and the private `start_fresh` restart (research R7)
- [x] T027 [US4] Add the `terminal_start_fresh(node_id, confirmed)` command in src-tauri/src/commands/terminal.rs and register it in src-tauri/src/commands/mod.rs, src-tauri/src/lib.rs, src-tauri/build.rs, and src-tauri/capabilities/default.json
- [x] T028 [P] [US4] Add `conversation?: { agent: "claude" | "codex"; id: string; recordedAt: string }` to `TerminalContent` in ui/src/ipc/types.ts and `terminalStartFresh(nodeId, confirmed)` in ui/src/ipc/commands.ts (contracts/ipc.md)
- [x] T029 [P] [US4] Write tests in ui/tests/terminalMenu.test.tsx: "Start fresh" is shown only when the node has `conversation`; clicking it calls `terminalStartFresh(id, false)`; on a `TerminalBusy` error the existing restart confirmation appears and confirming calls `terminalStartFresh(id, true)`; the tooltip reads "Forget this terminal's conversation and restart the agent in a new one."
- [x] T030 [US4] Add the "Start fresh" item to ui/src/nodes/TerminalMenu.tsx, reusing the confirmation flow of Restart

**Checkpoint**: quickstart D passes.

## Phase 7: User Story 5 — Restore fails gracefully (P3)

**Goal**: A failed resume prints one notice and starts a fresh conversation, never a loop.
**Independent Test**: quickstart E.

- [x] T031 [P] [US5] Write wrapper tests in crates/core/tests/session_restore_run.rs: a fake `claude` that exits 1 at once when its first arguments are `--resume <id>` and stays 0 otherwise makes `sonata run claude` print exactly `sonata: the previous conversation could not be resumed; starting a new one.` once on stderr and then run `claude` with the original arguments (args file has two lines); a fake that exits 1 after 11 s with `--resume` is not retried and its status is returned; a fake that exits 0 within 10 s is not retried; same for `codex resume <id>`
- [x] T032 [US5] Implement the 10-second failure window and fallback `exec` in crates/cli/src/run.rs (research R4); the window defaults to 10 000 ms and is read from `SONATA_RESUME_WINDOW_MS` when set, so tests can shorten it (T031 uses 500 ms and a 1 s fake instead of 11 s)
- [x] T033 [P] [US5] Write an integration test in crates/core/tests/session_restore_offer.rs that after a failed resume the fresh start's `SessionStart` (`source=startup`) clears the conversation, so the next app reopen offers nothing (no second notice)
- [x] T034 [US5] Make T033 pass (expected to pass with T022 and T032; fix gaps if not)

**Checkpoint**: quickstart E passes.

## Phase 8: Polish & Cross-Cutting

- [x] T035 [P] Add a unit test that copying a terminal node for any future duplicate/template path clears `conversation` (research R8): add `TerminalContent::copy_for_new_node()` that returns a clone with `conversation: None` and `lane: None`, in crates/core/src/model/node.rs
- [x] T036 [P] Add a "Session restore" section to README.md: agents resume their conversation after reopen, Start fresh, what happens when resume fails, and that Sonata stores only the conversation id
- [x] T037 Update `--help` text if any user-visible CLI text changed, in crates/cli/src/main.rs (internal commands stay hidden)
- [ ] T038 Run `sh scripts/check.sh` (fmt, clippy, Rust tests, typecheck, Vitest, release tests) and fix every failure
- [ ] T039 Manual: run quickstart scenarios A–E in a dev build with throwaway `SONATA_HOME` and record results in specs/009-session-restore/quickstart.md
- [ ] T040 Manual, when Codex is installed: quickstart F, confirming `thread-id` works with `codex resume <id>` (research R2)

## Dependencies

```text
Setup (T001)
   └─► Foundational (T002–T008)
          ├─► US1 resume (T009–T014) ─► US5 failure (T031–T034)
          ├─► US3 hooks  (T019–T024)      (US5 T033 needs T022)
          └─► US4 Start fresh (T025–T030)
US2 environment (T015–T018): no dependency, can start first
Polish (T035–T040): after the stories
```

- US1 is usable alone with `_conversation` called by hand; US3 makes it automatic. Ship US1 and
  US3 together.
- US2 is independent and smallest; it can merge first as its own PR if wanted.

## Parallel Examples

- Phase 2: T002, T004, T005, T007 (different test files) at once; then T003 → T006 → T008.
- US1: T009 and T012 together; then T010 → T011 and T013.
- US2: T015 and T017 together; then T016 and T018.
- US3: T019, T021, T023 together; then T020, T022, T024.
- US4: T025, T028, T029 together; then T026 → T027 and T030.

## Implementation Strategy

1. **MVP**: Phases 1–2, US2, US1, US3 → quickstart A, B, C. This is the full daily value: agents
   come back after reopen and saving works everywhere.
2. **Next**: US4 Start fresh.
3. **Then**: US5 failure path.
4. **Finish**: Polish, `scripts/check.sh`, manual quickstart.
