# Implementation Plan: Workspaces, Canvas, Terminals, and Notes

**Branch**: `001-workspaces-canvas-terminals` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/001-workspaces-canvas-terminals/spec.md`

## Summary

First usable slice of Sonata: unlimited workspaces in a sidebar, each with an infinite canvas of
terminal nodes (real login shells, optional launch command) and Markdown note nodes, saved as
human-readable files under `~/.sonata/` with atomic writes, autosave, migrations, load-failure
isolation, and a crash-loop guard.

Technical approach: a Rust `core` crate owns every rule and all state, including a headless
terminal screen model (`vt100`) per PTY, so terminals run, preview, and save history without any
view. A thin Tauri 2 shell exposes `core` through commands, events, and per-terminal channels. A
React frontend draws a custom canvas; terminals that are legible on screen get a live xterm.js
instance in an unscaled overlay layer, and all others show a text preview pushed by `core`. See
[research.md](research.md).

## Technical Context

**Language/Version**: Rust stable 1.99 (edition 2024); TypeScript 7 for the frontend.

**Primary Dependencies**: Rust: `tauri` 2.12, `portable-pty` 0.9, `vt100` 0.18, `tokio` 1.54,
`serde`/`serde_json`, `uuid` 1, `tempfile` 3, `notify` 8 + `notify-debouncer-full` 0.7,
`thiserror` 2, `tracing` + `tracing-appender`, `libc`. Frontend: React 19, Vite 8, `zustand` 5,
`@xterm/xterm` 6 + `addon-webgl` 0.19 + `addon-unicode11`, `react-markdown` 10 + `remark-gfm` 4.

**Storage**: Files under `~/.sonata/` (override `SONATA_HOME`): JSON envelopes, `.md` notes,
`.history` text. Formats in [contracts/file-formats.md](contracts/file-formats.md).

**Testing**: `cargo test` (unit + integration with temp `SONATA_HOME`, real `/bin/sh` PTYs, git not
needed in 001), `vitest` + Testing Library, Playwright with mocked Tauri IPC, manual
[quickstart.md](quickstart.md) for full-app checks.

**Target Platform**: macOS 15+, Apple Silicon and Intel (universal build).

**Project Type**: Desktop app (Rust core + Tauri shell + React frontend).

**Performance Goals**: 60 fps pan/zoom with 30 streaming terminals in one workspace; keystroke
echo with no noticeable delay (< 50 ms) in the focused terminal; workspace switch < 1 s with 50
workspaces; external note edits shown < 2 s.

**Constraints**: ≤ 12 WebGL contexts at any time; frontend memory independent of terminal count
(live xterm.js only for legible terminals); files `0600`, folders `0700`; no data loss beyond one
autosave interval on crash; no file ever left partially written.

**Scale/Scope**: ≥ 50 workspaces; ≥ 100 running terminals across workspaces; 10,000 history rows
per terminal; single user, single window.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre-design | Post-design |
|---|---|---|---|
| I. Clean-room originality | Own names (Workspace, Node, Terminal, Note); no Maestri text, icons, or code; RE doc not committed (`.gitignore`). | Pass | Pass. Vocabulary and formats are our own (`camera`, `launchCommand`, `saveHistory`). |
| II. Core/shell separation | `core` has no Tauri dependency; `src-tauri` only maps commands/events; frontend never persists. | Pass | Pass. IPC contract forwards to `core` services; screen model in `core` (R1). CLI crate deferred to feature 002, where it is first needed. |
| III. Test-first | Tests before code; temp dirs; fake/scripted PTYs. | Pass | Pass. R12 lists unit, integration, crash-kill, UI, and Playwright suites; tasks will order tests first. |
| IV. Files are the source of truth | Envelope, migrations, atomic writes, `.md` notes, discovery by folder listing, failures isolated. | Pass | Pass. R7, R9, and the file-format contract implement each rule. |
| V. Security | Socket rules N/A (no IPC server in 001). Paths validated in code; destructive ops confirmed; no secrets in argv. | Pass | Pass. Workspace deletion only removes `$SONATA_HOME/workspaces/<uuid>` after UUID and prefix checks; delete/close/quit confirmations (FR-006, FR-018, FR-018a); `0600`/`0700` (FR-015a). |
| VI. Free distribution | Ad-hoc signing, universal build, no paid services. | Pass | Pass. `tauri.conf.json` sets `signingIdentity: "-"`. Install script and release workflow are out of 001 scope (no release yet). |
| VII. Simplicity | macOS only; smallest slice; justify new abstractions. | Pass | Pass with one justified abstraction (see Complexity Tracking). |

Result: **PASS**. No unjustified violations.

## Project Structure

### Documentation (this feature)

```text
specs/001-workspaces-canvas-terminals/
├── plan.md              # This file
├── research.md          # Phase 0: decisions R1–R13
├── data-model.md        # Phase 1: entities, validation, state transitions
├── quickstart.md        # Phase 1: build, test, and manual validation guide
├── contracts/
│   ├── file-formats.md  # On-disk formats (public contract)
│   └── ipc.md           # Tauri commands, events, channels
├── checklists/
│   └── requirements.md
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
Cargo.toml                    # workspace: crates/core, src-tauri
package.json                  # frontend scripts; Vite root = ui/
crates/core/
├── src/
│   ├── lib.rs
│   ├── error.rs              # AppError codes (contracts/ipc.md)
│   ├── paths.rs              # SONATA_HOME, path validation, 0700/0600 enforcement
│   ├── model/                # workspace.rs, node.rs, app_state.rs, preferences.rs
│   ├── persistence/          # envelope.rs, atomic.rs, migrate.rs, store.rs (load/save/discover)
│   ├── workspace/            # manager.rs (create/rename/pin/reorder/open/delete), palette.rs
│   ├── terminal/             # manager.rs, pty.rs, screen.rs (ScreenModel + vt100),
│   │                         # history.rs, busy.rs, preview.rs
│   ├── notes/                # store.rs, watcher.rs (notify + conflict copies)
│   ├── app.rs                # App facade: bootstrap, quit check, quit
│   ├── autosave.rs           # dirty revisions + interval task
│   └── crash_guard.rs
└── tests/                    # persistence.rs, workspaces.rs, terminals.rs, notes.rs,
                              # crash_kill.rs (SIGKILL loop), screen_replay.rs
src-tauri/
├── Cargo.toml
├── tauri.conf.json           # ad-hoc signing, macOS 15 minimum
├── capabilities/default.json # only the commands in contracts/ipc.md
└── src/
    ├── main.rs               # setup: core init, crash guard, autosave, quit handling
    ├── commands/             # app.rs, workspace.rs, node.rs, terminal.rs, note.rs
    └── events.rs             # core events → Tauri events
ui/
├── index.html
├── src/
│   ├── main.tsx, App.tsx
│   ├── ipc/                  # typed wrappers for commands/events (mirrors contracts/ipc.md)
│   ├── state/                # zustand stores: workspaces, active canvas, focus
│   ├── sidebar/              # Sidebar, WorkspaceItem, CreateWorkspaceDialog, LoadFailureItem
│   ├── canvas/               # Canvas.tsx, viewport.ts, gestures.ts (FR-008a router),
│   │                         # culling.ts, selection.ts, ResizeHandles.tsx
│   ├── nodes/                # TerminalNode.tsx (frame + preview), NoteNode.tsx
│   ├── terminal/             # LiveTerminalLayer.tsx (overlay, R2), liveSet.ts (R3),
│   │                         # xtermSession.ts (attach/detach/WebGL budget)
│   └── dialogs/              # ConfirmDialog (delete, close, quit)
└── tests/                    # vitest unit/component tests
e2e/                          # Playwright specs with mocked IPC
scripts/bench-terminals.sh    # SC-003/SC-004 load generator
```

**Structure Decision**: Cargo workspace with `crates/core` and `src-tauri`, plus a Vite frontend
in `ui/`, as fixed by the constitution's Technology & Platform Constraints. `crates/cli` is not
created in this feature because nothing uses it until feature 002 (YAGNI); it is added there
without changing this layout.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| `ScreenModel` trait over the terminal emulator | R1 spike may show `vt100` replay is wrong for some programs; the trait lets us swap to `alacritty_terminal` without touching the terminal manager, history, or previews. | Calling `vt100` directly everywhere would spread the dependency across five modules and make the fallback a rewrite. |
| Overlay layer for live terminals (two render layers) | xterm.js inside a CSS-scaled container renders blurry and mis-maps mouse selection (R2). | Single scaled layer breaks text selection and readability, which are core to a terminal app. |
