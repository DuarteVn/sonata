# Research: Workspaces, Canvas, Terminals, and Notes

**Feature**: 001-workspaces-canvas-terminals | **Date**: 2026-10-21

Versions were checked against npm and crates.io on 2026-10-21. Use caret ranges and commit the
lockfiles (`Cargo.lock`, `package-lock.json`).

## R1. Terminal screen model lives in Rust

**Decision**: `core` parses every PTY byte stream with the `vt100` crate (0.18) and keeps the
screen and a 10,000-row scrollback per terminal. The frontend creates an xterm.js instance only
while a terminal is legible on screen. On attach, `core` sends a replay (scrollback rows and the
current screen, as escape sequences from `rows_formatted` / `state_formatted`), then streams live
bytes. On detach, the xterm.js instance is disposed.

**Rationale**:
- Constitution II: `core` is the source of truth. The screen must exist without any view.
- Previews (FR-014), saved history (FR-015), and the "idle shell" check need the screen in Rust.
- Feature 002 (`check`/`ask`) reads terminal screens from the server; this model is reused.
- Disposing xterm.js instances bounds frontend memory and WebGL contexts, whatever the number of
  terminals (SC-003, SC-004).

**Alternatives considered**:
- *Keep one xterm.js instance alive per terminal in the frontend*: simpler, but 100 instances
  with 10k-row buffers live in the webview, and the screen is unavailable to `core`.
- *Raw byte ring buffer replayed into xterm.js*: exact colors, but a replay can start inside an
  escape sequence or after an alternate-screen switch, which corrupts the view.
- *`alacritty_terminal` (0.26)*: more complete emulator, but no built-in formatted output for
  replay. Kept as a fallback behind the `ScreenModel` trait (see Complexity Tracking).

**Spike (first task of US1)**: run `claude`, `vim`, and `htop` through `vt100`, detach and
re-attach, and compare the replayed view with a live view. If the replay is visibly wrong, switch
the trait implementation to `alacritty_terminal`.

## R2. Live terminals render in a screen-space overlay, not inside the scaled canvas

**Decision**: The canvas (nodes, notes, previews) is drawn inside one transformed container
(`translate` + `scale`). Live terminals (xterm.js) are drawn in a separate overlay layer that is
not scaled. Each live terminal is positioned at its node's screen rectangle and uses
`fontSize = baseFontSize × zoom`, so columns and rows do not change with zoom and the PTY is not
resized. During an active pan or zoom gesture, the overlay receives the same CSS transform as the
canvas (cheap, slightly blurry); about 150 ms after the gesture ends, the overlay snaps to crisp
positions and font sizes.

**Rationale**: xterm.js computes mouse positions and selection from unscaled geometry; under a
CSS `scale()` the text is blurry and mouse selection is offset. Rendering at native size avoids
both problems.

**Alternatives considered**: xterm.js inside scaled nodes (blurry text, wrong selection);
interactive terminals only at exactly 100% zoom (too restrictive).

## R3. Legibility, renderers, and WebGL contexts

**Decision**:
- A terminal is **live** when its screen rectangle intersects the viewport and the effective font
  size is at least 7 px (zoom ≥ 0.55 with a 13 px base font). Otherwise it shows a **preview**.
- Live terminals use the WebGL addon (`@xterm/addon-webgl` 0.19) while at most 12 are live; beyond
  that, or after `onContextLoss`, they use the DOM renderer. 12 stays below WebKit's limit of about
  16 contexts per page.
- Previews are plain text from `core` (visible screen only), pushed at most once per second per
  terminal and only while the preview is in or near the viewport. They render as a `<pre>` with a
  small fixed font (scaled with the canvas).

**Rationale**: bounds GPU contexts and DOM size independently of the terminal count.

**Alternatives considered**: image snapshots of xterm canvases (costly, impossible for terminals
never shown); one shared WebGL canvas for all terminals (large custom renderer, not needed).

## R4. Canvas implementation

**Decision**: Custom canvas component in React (one transformed container, absolutely positioned
nodes, pointer-event handlers for drag, resize, pan, and pinch). Viewport culling: nodes outside
the viewport plus a margin render as empty boxes.

**Rationale**: Gesture rules in FR-008a (scroll goes to the focused terminal, else pans; pinch
zooms around the pointer; middle-drag and Space+drag pan) and the overlay of R2 need full control
of events and coordinates. The node count is small (tens), so a library brings little.

**Alternatives considered**: `@xyflow/react` 12.12 (React Flow): has pan, zoom, drag, and edges
(useful for feature 002 links), but its node renderer conflicts with the R2 overlay and its wheel
handling must be overridden. Reconsider for links in feature 002 only if custom edges prove
costly.

## R5. Shell and launch command

**Decision**: Spawn the user's login shell (from `getpwuid`, fallback `$SHELL`, fallback
`/bin/zsh`) with `-l` as an interactive PTY via `portable-pty` 0.9, in the node's working folder
(fallback: home folder plus a warning, if the folder no longer exists). If the node has a launch
command, write `<command>\r` to the PTY right after spawn; the TTY buffers it until the shell
reads input. Environment additions: `TERM=xterm-256color`, `COLORTERM=truecolor`,
`TERM_PROGRAM=Sonata`, `SONATA_TERMINAL_ID=<node id>`.

**Rationale**: A login shell loads the user's `PATH` (GUI apps launched from Finder have a minimal
environment). Typing the command keeps it in shell history, and the user returns to a shell when
the agent exits.

**Alternatives considered**: `$SHELL -lic '<cmd>; exec $SHELL -l'` (the command is not in history;
quoting risks).

## R6. "Idle shell" detection (FR-018, FR-018a)

**Decision**: A terminal is busy when the PTY's foreground process group (`tcgetpgrp` on the
master, exposed by `portable-pty` as `process_group_leader()`) differs from the shell's process
id. Idle otherwise.

**Rationale**: Standard job-control check; cheap; no polling of process lists.

**To confirm in implementation**: the exact `portable-pty` 0.9 accessor; if missing, call
`libc::tcgetpgrp` on the master's raw file descriptor.

## R7. Persistence mechanics

**Decision**:
- Data root `~/.sonata/` (override with env `SONATA_HOME` for tests). Directories `0700`, files
  `0600` (FR-015a), enforced on every create and checked at startup.
- Atomic write: `tempfile::NamedTempFile::new_in(same dir)` → write → `sync_all` → `persist`
  (rename) → `fsync` of the directory.
- Envelope `{type, schemaVersion, payload}`, `serde_json` with keys sorted (serialize through
  `serde_json::Value`, whose map is a `BTreeMap` without the `preserve_order` feature), optional
  `None` fields skipped.
- Migrations: a list of functions `fn(Value) -> Result<Value>` per file type, applied from the
  file's version to the current one before typed decoding. A file with a newer version is a load
  failure and is never written.
- Dirty tracking: each workspace has a revision counter; the autosave task (tokio interval,
  default 30 s) saves workspaces whose revision differs from the last saved one.
- Terminal history: on save and on quit, `core` writes each terminal's scrollback and screen as
  formatted text (capped at 10,000 rows) to `terminals/<nodeId>.history`. On restart, the history
  is fed into the new terminal's model before the shell starts, followed by a separator line.

**Alternatives considered**: SQLite (not human-readable, violates constitution IV); the
`atomicwrites` crate (less maintained; `tempfile` covers it).

## R8. Watching note files (FR-020a)

**Decision**: `notify` 8.2 with `notify-debouncer-full` 0.7 (FSEvents backend) on each loaded
workspace's `notes/` folder, debounce 300 ms. `core` keeps, per note, the content hash of the last
write it made. Events whose content hash equals that hash are ignored (own writes). Otherwise:
no unsaved in-app edits → reload and emit an event; unsaved edits → write the external text to
`<nodeId> (conflict).md`, keep in-app text, and emit a warning event.

**Rationale**: Meets the 2-second target with margin; hash comparison avoids loops from own
atomic writes.

## R9. Crash-loop guard

**Decision**: `app-state.json` holds `cleanShutdown`, `launchedAt`, and `consecutiveEarlyCrashes`.
At launch: if the previous run did not shut down cleanly and did not outlive the 30-second window,
increment the counter, else reset it to 0. Then set `cleanShutdown = false`. A timer at 30 s sets
`outlivedLaunchWindow = true` and resets the counter. Normal quit sets `cleanShutdown = true`. If
the counter is ≥ 2, the active workspace is not restored and the UI shows why.

## R10. Frontend ↔ core transport

**Decision**: Tauri 2.12 commands for requests, Tauri events for state changes
(`workspace://changed` etc.), and a `tauri::ipc::Channel` per live terminal for PTY output
(raw bytes, batched every 8 ms or 32 KB). Keyboard input goes through a `terminal_write` command.

**Rationale**: Channels are the documented high-throughput path; events are fine for low-rate
state updates.

## R11. Frontend libraries

**Decision**: React 19, TypeScript, Vite, `zustand` 5 for the view state cache,
`react-markdown` 10 + `remark-gfm` 4 for rendered notes, a plain `<textarea>` for note editing,
`@xterm/xterm` 6 with `addon-webgl`, `addon-unicode11`.

**Rationale**: Small and common. A rich editor (CodeMirror) is not required by the spec.

## R12. Testing strategy

**Decision**:
- `crates/core`: `cargo test` unit tests and integration tests in `crates/core/tests/` using
  temporary `SONATA_HOME` directories (`tempfile`). PTY tests use real `/bin/sh` with scripted
  input and also a fake `ScreenModel` feed for replay tests.
- Crash safety (SC-005): a test binary that is killed (`SIGKILL`) in a loop during saves; the
  test then checks that every file decodes.
- Frontend: `vitest` + `@testing-library/react` for components and the gesture router; Tauri IPC
  mocked with `@tauri-apps/api/mocks`.
- End to end: Playwright 1.64 against the Vite dev server with mocked IPC (layout, gestures,
  previews). Tauri's WebDriver does not support macOS, so full-app checks follow `quickstart.md`
  by hand.
- Performance (SC-003/004): a benchmark script that opens 30 and 100 terminals running
  `yes | head -c` loops and records frame times in the dev build.

## R13. Logging

**Decision**: `tracing` + `tracing-appender` daily file at `~/.sonata/logs/sonata.log`, kept 7
days. Log load failures, migrations, save errors, and PTY spawn errors. Never log terminal
content.
