# Implementation Plan: Agent Messaging over Links, Sonata CLI, and Orchestrator Powers

**Branch**: `003-agent-messaging` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/003-agent-messaging/spec.md`

## Summary

A `sonata` CLI inside every terminal talks to the app over a private Unix socket (JSON lines).
`core` gains a socket server, per-terminal session tokens, an ask broker (queues, deadlines, loop
detection, cancellation, exchange log), answer extraction from the screen model, orchestrator
operations (recruit, dismiss, link, unlink), note commands, and notifications. Known agents are
started through `sonata run`, which passes session-only instructions and end-of-turn hooks
(Claude Code: `--append-system-prompt-file`, `--settings` hooks; Codex: `-c notify`). See
[research.md](research.md).

## Technical Context

**Language/Version**: Rust stable 1.99; TypeScript 7.

**Primary Dependencies**: existing; new: `tauri-plugin-notification`,
`tauri-plugin-single-instance`; `rand` for tokens (already present). The CLI crate depends only on
`serde`/`serde_json`.

**Storage**: unchanged files; runtime folder `<data root>/run/` (socket and per-terminal session
files, removed on stop). Two optional terminal fields (`recruitedBy`, `sendInstructions`).

**Testing**: `cargo test` with the real socket and real `/bin/sh` terminals: CLI end to end (spawn
the built `sonata` binary with the terminal's env), broker unit tests (queue, loops, timeouts,
cancel), answer extraction from scripted output, fake agent hooks (a script that calls
`sonata _hook claude-stop` with a fake transcript), permission tests (unlinked, forged token,
non-orchestrator). Vitest for the log panel and link highlight; Playwright for UI events.

**Target Platform**: macOS 15+.

**Project Type**: Desktop app + bundled CLI.

**Performance Goals**: answer returned ≤ 2 s after a known agent's end-of-turn signal (SC-001);
CLI round trip for `list` under 50 ms.

**Constraints**: no network listener; socket `0600`; no writes to the user's agent
configuration; messages ≤ 100 KB, results ≤ 1 MB.

**Scale/Scope**: up to ~50 concurrent asks; 100 terminals.

## Constitution Check

| Principle | Gate | Pre | Post |
|---|---|---|---|
| I. Clean room | Own CLI name, commands, instructions text, protocol. | Pass | Pass. Reference timings informed the heuristic only; values and design are ours (hooks first). |
| II. Core/shell | Server, broker, permissions in `core`; CLI separate crate; shell wires plugins. | Pass | Pass. |
| III. Test-first | Tests before each task, real socket and PTYs. | Pass | Pass. |
| IV. Files | No new persistent formats beyond two optional fields. | Pass | Pass. Session files are runtime only. |
| V. Security | Socket private, tokens per terminal, link-based permissions, server-side checks, no secrets in argv. | Pass | Pass. Token travels in env and the request body, never in arguments; refused requests are logged without tokens. |
| VI. Distribution | CLI bundled in the app; nothing installed system-wide. | Pass | Pass. |
| VII. Simplicity | JSON lines instead of HTTP (amendment 1.1.0). | Pass | Pass. |

Result: **PASS**.

## Project Structure

```text
crates/cli/                     # new: `sonata` binary
├── Cargo.toml
└── src/{main.rs, args.rs, client.rs, run.rs, hook.rs}
crates/core/src/
├── messaging/                  # new
│   ├── mod.rs, server.rs (socket, request routing, auth)
│   ├── broker.rs (asks, queues, loops, deadlines, log)
│   ├── commands.rs (list, ask, check, reply, note, notify, recruit, dismiss, link, unlink)
│   ├── extract.rs (answer from screen rows, ANSI cleanup)
│   └── instructions.rs (instructions.md and settings.json per session)
├── terminal/manager.rs         # sessions (token, run dir), absolute row counter, paste/keys
└── app.rs                      # wires the server and broker
crates/core/tests/              # cli_e2e.rs, broker.rs, extract.rs, orchestrator.rs,
                                # agent_hooks.rs, permissions_cli.rs
src-tauri/                      # plugins, CLI in bundle resources, notify bridge
ui/src/                         # link glow, "asked by" marker, exchange log panel
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Second binary (`crates/cli`) | Agents call a command, not the app. | Constitution II already requires it; no simpler path. |
