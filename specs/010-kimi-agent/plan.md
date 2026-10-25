# Implementation Plan: Kimi Code CLI as a Supported Agent

**Branch**: `010-kimi-agent` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/010-kimi-agent/spec.md`

## Summary

Kimi Code CLI (`kimi`) becomes the third agent `sonata run` wraps. Session instructions travel in
a per-session Kimi agent file (`--agent-file`, body `${base_prompt}` plus Sonata's text), which
Kimi binds to the conversation and restores on resume, so no message is ever typed and resume is
not blocked. Kimi reads hooks only from its user config file, so busy/done and conversation
tracking need a one-time, user-confirmed Apply in Settings that appends a marked block of five
`[[hooks]]` tables (with a backup, a `kimi doctor` check, and Remove). The hooks call
`sonata _hook kimi …` only inside Sonata terminals. Until a Kimi process proves its hooks work, the
broker treats it with the quiet-screen rule; after the first hook report it waits for Kimi's
`Stop`. Session restore reuses feature 009 with `kimi --session <id>`. See
[research.md](research.md).

## Technical Context

**Language/Version**: Rust 2024 edition, TypeScript / React 19.

**Primary Dependencies**: none new. Config editing is line-based (research R4); file watching
uses the existing `notify` crate.

**Storage**: optional `Preferences.kimiNoticeDismissed`; `Conversation.agent` gains `"kimi"`
(feature 009); per-session `kimi-agent.md` under `run/sessions/<node>/`. Kimi's `config.toml` is
the user's file, changed only on confirmed Apply/Remove.

**Testing**: `cargo test` with a fake `kimi` in `SONATA_AGENT_BIN_DIR` (records arguments, can
fail like a missing session), fake hook payloads on stdin, config fixtures in a temporary
`KIMI_CODE_HOME`; Vitest + Testing Library for the notice, Settings section, and preset.

**Target Platform**: macOS 15+.

**Project Type**: desktop app (Tauri) + CLI.

**Performance Goals**: hooks add no visible delay to Kimi (each is one local socket message;
outside Sonata they exit before starting `sonata`); an ask to a hooked Kimi returns within 2 s of
`Stop` (US3).

**Constraints**: nothing written to the user's project folder; Kimi's config changed only on
confirmed Apply/Remove, byte for byte outside Sonata's tables, original permission bits kept
(the file can hold API keys); hook stdout always empty (Kimi adds `UserPromptSubmit` stdout to
the model context).

**Scale/Scope**: one config file per user; a handful of Kimi terminals.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Result |
|---|---|
| I Clean room | Pass: own design and text; uses Kimi's public flags, agent-file format, and hook events. |
| II Core/shell | Pass: config parsing and editing, status, agent file, hooked state, and broker rule in `core`; the CLI maps the session reply to flags and hook payloads to reports; `src-tauri` forwards four commands and one event; the UI renders status and sends intents. |
| III Test-first | Pass: fake `kimi`, fake payloads, config fixtures; tests written and failing first. Config tests never touch `~/.kimi-code`. |
| IV Files | Pass: one optional preference, omitted when unset; atomic writes; Sonata's own files keep the envelope. Kimi's file is not Sonata state and keeps its format. |
| V Security | Pass with a recorded exception: changing the agent's global config is a user-confirmed action in the UI with a backup (spec clarification 2026-10-25; Complexity Tracking). Node chosen from caller identity. No secret in arguments. Config permission bits preserved. Hooks never block Kimi (`; exit 0`). |
| VI Distribution | Pass: no new service or dependency. |
| VII Simplicity | Pass: one new `core` module (`agents/kimi.rs`), no new crate; status computed, not stored. |

Re-checked after design: no change.

## Project Structure

### Documentation (this feature)

```text
specs/010-kimi-agent/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/cli.md  contracts/ipc.md  contracts/kimi-config.md
├── checklists/requirements.md
└── tasks.md            # /speckit-tasks
```

### Source Code

```text
crates/core/src/presets.rs                 # Kimi Code preset (R8)
crates/core/src/terminal/pty.rs            # WRAPPED_AGENTS += kimi; AGENT_SESSION_MARKERS += Kimi (R7, needs 009)
crates/core/src/terminal/zsh.rs            # kimi() helper
crates/core/src/agents/mod.rs              # new module root
crates/core/src/agents/kimi.rs             # config path, status, block, apply/remove, backup, doctor check (R4)
crates/core/src/messaging/instructions.rs  # kimi-agent.md writer; no `${` in instructions; recruit text lists kimi
crates/core/src/messaging/broker.rs        # TargetKind::Probing → Hooked once hooked (R5)
crates/core/src/messaging/commands.rs      # `_session kimi` reply, `_turn start` busy + hooked, recruit kimi
crates/core/src/terminal/manager.rs        # hooked flag per process generation
crates/core/src/model/preferences.rs       # kimiNoticeDismissed
crates/core/src/model/node.rs              # Conversation.agent kimi (needs 009)
crates/core/src/app.rs                     # Kimi integration methods, watcher, events
crates/core/tests/kimi_config.rs           # status, apply, remove, conflict, backup, permissions, doctor
crates/core/tests/kimi_session.rs          # agent file, `_session` reply, hooked switch in the broker
crates/core/tests/agent_hooks.rs           # `_hook kimi …` and wrapper flags with a fake kimi
crates/core/tests/fixtures/kimi/           # config.toml fixtures
crates/cli/src/run.rs                      # kimi flags: --agent-file, --session, choose detection, -p guard
crates/cli/src/hook.rs                     # kimi session-start | prompt-submit | stop | stop-failure | interrupt
src-tauri/src/commands/agents.rs           # kimi_integration, _apply, _remove, dismiss_kimi_notice
src-tauri/build.rs, capabilities/default.json, src/lib.rs, src/commands/mod.rs
ui/src/ipc/{commands,types,events}.ts
ui/src/settings/SettingsApp.tsx            # Agents section in the nav
ui/src/settings/AgentSettings.tsx          # Kimi Code status, block, Apply/Remove/Copy
ui/src/nodes/TerminalNode.tsx              # Kimi notice strip
ui/src/dialogs/TerminalDialog.tsx          # kimi default for "send instructions"
ui/tests/kimiSettings.test.tsx, ui/tests/kimiNotice.test.tsx, ui/tests/terminalDialog.test.tsx
README.md                                  # Kimi section: setup, what changes, known /clear gap
```

**Structure Decision**: Existing layout plus one `core` module folder `agents/` for agent setup
logic that is not messaging (Kimi config). Claude Code and Codex code stays where it is.

## Phases

Phases 1–4 do not need feature 009 and can merge on their own. Phases 5–6 start after 009 is on
`main` and this branch is rebased.

1. **US1 preset and instructions** (P1): preset, `WRAPPED_AGENTS`, zsh helper, recruit, agent
   file, `_session kimi`, wrapper `--agent-file` and choose detection, dialog default.
2. **US2 config core** (P1): `agents/kimi.rs` (path via login shell, status, block, conflict,
   apply, remove, backup rotation, permission bits, `kimi doctor` check and rollback).
3. **US2 setup UI** (P1): Tauri commands and event, Settings › Agents › Kimi Code, notice strip,
   `kimiNoticeDismissed`, watcher.
4. **US3 busy/done** (P2): `_hook kimi …` (empty stdout, exit 0), `_turn start` busy, hooked flag,
   `TargetKind::Probing`.
5. **US5 environment** (P3, needs 009's marker list): add Kimi markers.
6. **US4 restore** (P2, needs 009): `Conversation.agent` kimi, resume flags, `session-start`
   reports, print-mode guard, failure path with a fake `kimi`.
7. **Polish**: README, quickstart live run when Kimi is available, `scripts/check.sh`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Sonata changes the agent's global config file (exception to feature 003's "global agent configuration is never changed") | Kimi reads hooks only from `~/.kimi-code/config.toml`; without them there is no busy/done signal and no conversation id (FR-014, FR-017). | Copy-paste only: rejected by the user (spec clarification). `KIMI_CODE_HOME` redirect: splits the user's sessions and login. Kimi plugin: also writes into Kimi's folder and needs its install flow. Mitigations: explicit confirmation, backup, owned block only, Remove, `kimi doctor` check with rollback. |
| New `TargetKind::Probing` | A Kimi terminal may or may not have working hooks; the existing kinds would either wait 20 s on every unhooked ask or ignore `Stop`. | Status-based choice: wrong for Kimi processes started before Apply, and reads a file per ask (research R5). |
| New `core::agents` module folder | Kimi config editing is setup logic, not messaging or terminal code. | Putting it in `messaging/instructions.rs` mixes editing a user file with session text. |
