# Implementation Plan: Session Restore for Agent Terminals

**Branch**: `009-session-restore` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

## Summary

Sonata learns each agent terminal's conversation id from hooks it already installs: Claude Code's
`SessionStart` (new) and `UserPromptSubmit` (extended), and Codex's `notify` payload. `core`
stores the id on the terminal node and saves the workspace when it changes. When a terminal
process starts, the first `sonata run` in it gets a one-time resume offer and starts
`claude --resume <id>` or `codex resume <id>`. If the agent exits with an error within 10 s, the
wrapper prints one notice and starts a fresh conversation. "Start fresh" in the terminal menu
clears the id and restarts. Every terminal's environment loses an explicit list of agent session
markers, which fixes the "Transcript saving is off" warning and a token leak. See
[research.md](research.md).

## Technical Context

**Language/Version**: Rust 2024 edition, TypeScript / React 19.
**Primary Dependencies**: none new. `libc` (already a dependency) for signal handling in the
wrapper.
**Storage**: optional `TerminalContent.conversation` in the workspace file, no schema bump
(research R6). **Testing**: `cargo test` with fake agents (scripts in a temp `SONATA_AGENT_BIN_DIR`
that record their arguments and exit with chosen codes and delays) and fake hook payloads on
stdin; core tests through the existing in-process socket helpers; Vitest + Testing Library for the
menu item. **Target Platform**: macOS 15+.
**Project Type**: desktop app (Tauri) + CLI. **Performance Goals**: conversation recorded within
5 s of a switch (FR-003); resume adds at most 10 s to a terminal start (SC-002).
**Constraints**: nothing written to agent configuration or project folders (FR-005); the
conversation id is validated before it becomes an argument. **Scale/Scope**: a handful of agent
terminals per workspace.

## Constitution Check

| Principle | Result |
|---|---|
| I Clean room | Pass: own design and text; uses the agents' public flags and hooks. |
| II Core/shell | Pass: storage, offer, validation, and busy rule in `core`; the CLI wrapper only maps the offer to flags; `src-tauri` forwards one command. |
| III Test-first | Pass: fake agents and fake hook payloads; tests written and failing before code. |
| IV Files | Pass: optional field, omitted when empty, envelope unchanged, immediate atomic save on change. |
| V Security | Pass: node chosen by the caller's identity, never by arguments; id validated to `[A-Za-z0-9_-]{1,128}`; removes the outer agent's messaging token from terminal environments. Start fresh deletes nothing. |
| VI Distribution | Pass: no new service or dependency. |
| VII Simplicity | Pass: no new module folder; one new field, one new socket command, one new hook, one new shell command. |

Re-checked after design: no change.

## Project Structure

### Documentation (this feature)

```text
specs/009-session-restore/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/cli.md  contracts/ipc.md
└── tasks.md            # /speckit-tasks
```

### Source Code

```text
crates/core/src/terminal/pty.rs          # AGENT_SESSION_MARKERS removed at spawn (R5)
crates/core/src/usage/runner.rs          # same list for the usage probe child
crates/core/src/model/node.rs            # Conversation, TerminalContent.conversation, id validation
crates/core/src/terminal/manager.rs      # resume offer per process generation (R3)
crates/core/src/app.rs                   # record/clear conversation + immediate save, offer, start_fresh_conversation
crates/core/src/messaging/commands.rs    # `_conversation`, `_session` resume field
crates/core/src/messaging/instructions.rs # SessionStart hook in the settings file
crates/core/tests/session_restore_*.rs   # env, record, offer, persistence, start fresh
crates/cli/src/run.rs                    # resume flags, child + 10 s fallback, print mode (R3, R4, R9)
crates/cli/src/hook.rs                   # session-start, turn-start session_id, codex thread-id
crates/cli/tests/run_resume.rs           # fake agents
src-tauri/src/commands/terminal.rs       # terminal_start_fresh
src-tauri/build.rs, capabilities/default.json, src/lib.rs
ui/src/ipc/{commands,types}.ts
ui/src/nodes/TerminalMenu.tsx            # Start fresh
ui/tests/terminalMenu.test.tsx
README.md                                # Session restore section
```

**Structure Decision**: Existing layout; no new module folder.

## Phases

1. **US2 clean environment** (P1, independent, smallest): marker list, spawn and usage runner,
   tests.
2. **Foundation**: `Conversation` model and validation, `_conversation` command, immediate save.
3. **US1 restore** (P1): resume offer, `_session` field, wrapper resume, hook reports.
4. **US3 follow** (P2): `SessionStart` hook, `turn-start` id, Codex `thread-id`, print-mode guard.
5. **US4 Start fresh** (P2): core method, shell command, menu item.
6. **US5 failure** (P3): child + 10 s fallback and notice.
7. **Polish**: README, quickstart run, `scripts/check.sh`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Wrapper runs the agent as a child (not `exec`) when resuming | Detects a failed resume and starts fresh in the same terminal (FR-009). | Checking the agent's files first needs its private layout and misses the folder-change case (research R4). |
