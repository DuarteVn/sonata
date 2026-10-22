---

description: "Task list for feature 001: workspaces, canvas, terminals, and notes"
---

# Tasks: Workspaces, Canvas, Terminals, and Notes

**Input**: Design documents from `/specs/001-workspaces-canvas-terminals/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ipc.md,
contracts/file-formats.md, quickstart.md

**Tests**: Required. Constitution principle III (Test-First, NON-NEGOTIABLE): every test task is
written first and must fail before its implementation tasks start.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested on
its own.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: User story from spec.md (US1–US5)
- Paths follow the structure in plan.md: `crates/core/`, `src-tauri/`, `ui/`, `e2e/`, `scripts/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Repository skeleton, toolchains, and quality gates

- [X] T001 Create Cargo workspace in `Cargo.toml` (members `crates/core`, `src-tauri`; `edition = "2024"`; `[workspace.dependencies]` pinned per plan.md Technical Context), plus `rust-toolchain.toml` (channel `stable`, targets `aarch64-apple-darwin`, `x86_64-apple-darwin`) and `rustfmt.toml`
- [X] T002 Create crate `sonata-core` in `crates/core/Cargo.toml` and `crates/core/src/lib.rs` with deps `portable-pty 0.9`, `vt100 0.18`, `tokio 1` (rt-multi-thread, sync, time, macros), `serde 1` (derive), `serde_json 1` (no `preserve_order`), `uuid 1` (v4, serde), `chrono 0.4` (serde), `tempfile 3`, `notify 8`, `notify-debouncer-full 0.7`, `thiserror 2`, `tracing`, `tracing-appender`, `libc`, `rand`; dev-deps `tempfile`, `pretty_assertions`; empty modules matching plan.md (`error`, `paths`, `model`, `persistence`, `workspace`, `terminal`, `notes`, `autosave`, `crash_guard`, `events`, `logging`)
- [X] T003 Scaffold Tauri 2 app in `src-tauri/` (`Cargo.toml` depending on `sonata-core`, `tauri 2.12`, `tauri-plugin-dialog`; `build.rs`; `src/main.rs` opening one window titled "Sonata"; `tauri.conf.json` with productName `Sonata`, identifier `dev.sonata.app`, `bundle.macOS.minimumSystemVersion = "15.0"`, `bundle.macOS.signingIdentity = "-"`, `build.devUrl = "http://localhost:5173"`, `build.frontendDist = "../ui/dist"`; `capabilities/default.json` granting only `core:default` and `dialog:allow-open`)
- [X] T004 Create frontend toolchain: root `package.json` (scripts `dev`, `build`, `typecheck`, `test`, `e2e`, `tauri`; deps `react 19`, `react-dom 19`, `zustand 5`, `@xterm/xterm 6`, `@xterm/addon-webgl 0.19`, `@xterm/addon-unicode11 0.9`, `react-markdown 10`, `remark-gfm 4`, `lucide-react`, `@tauri-apps/api 2.12`, `@tauri-apps/plugin-dialog`; devDeps `vite 8`, `@vitejs/plugin-react`, `typescript`, `@tauri-apps/cli 2.12`, `vitest 5`, `@testing-library/react 16`, `jsdom`, `@playwright/test`), `vite.config.ts` with `root: "ui"`, `tsconfig.json` (strict), `ui/index.html`, `ui/src/main.tsx`, `ui/src/App.tsx` placeholder
- [X] T005 [P] Configure test runners: `vitest.config.ts` (jsdom, setup file `ui/tests/setup.ts`), `playwright.config.ts` (webServer = Vite dev server), and `e2e/fixtures/mockIpc.ts` built on `@tauri-apps/api/mocks` with an in-memory fake of every command in `contracts/ipc.md`
- [X] T006 [P] Create `scripts/check.sh` running `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `npm run typecheck`, `npm test`
- [X] T007 [P] Create CI workflow `.github/workflows/ci.yml` on `ubuntu-latest` (free minutes) running `cargo test -p sonata-core`, `npm ci`, `npm run typecheck`, `npm test`; skip Tauri bundling in CI
- [X] T008 [P] Update `README.md` with development prerequisites (Xcode CLT, rustup, Node 22+), `npm run tauri dev`, `scripts/check.sh`, and the `SONATA_HOME` override for throwaway data

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Data model, persistence primitives, workspace creation and opening, app shell, and
sidebar. Every story needs a workspace to exist.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Tests (write first, must fail)

- [X] T009 [P] Envelope and migration tests in `crates/core/tests/persistence_envelope.rs`: round trip of `{type, schemaVersion, payload}`; keys written sorted; `None` fields omitted; wrong `type` → error; `schemaVersion` greater than supported → `NewerVersion{found, supported}`; a test-only chain v0→v1→v2 applies steps in order on raw JSON before decode; missing step → `MigrationFailed{step}`
- [X] T010 [P] Atomic write tests in `crates/core/tests/atomic_write.rs`: replacing a file yields a new inode and full new content; a failure before rename leaves the old file byte-identical and no temp file behind; written files have mode `0600`
- [X] T011 [P] Path tests in `crates/core/tests/paths.rs`: `SONATA_HOME` overrides `~/.sonata`; data root and created folders have mode `0700`; workspace folder path accepted only for a valid UUID; any path containing `..` rejected; deletion guard accepts only `$SONATA_HOME/workspaces/<uuid>` exactly
- [X] T012 [P] Workspace creation tests in `crates/core/tests/workspace_create.rs`: name "Trimmed, 1–80 chars, unique among workspaces ignoring case" → errors `NameEmpty`, `NameTaken` (e.g. " demo " vs "Demo"); directory "Must exist on create" → `DirectoryMissing`, a file path → `NotADirectory`, relative path rejected; icon is from the icon set; color is one of 8 palette colors and prefers an unused one; new camera is `{x: 0, y: 0, zoom: 1}`; `workspace.json` exists after create; no count limit (create 60)
- [X] T013 [P] Autosave tests in `crates/core/tests/autosave.rs`: a changed workspace (revision ≠ saved revision) is written on tick; an unchanged workspace is not rewritten (inode unchanged); `save_all` on quit writes every dirty workspace; interval comes from `autosaveIntervalSeconds` "integer, 5–600", default 30
- [X] T014 [P] Sidebar UI tests in `ui/tests/sidebar.test.tsx`: empty state shows "Create workspace"; create dialog shows the message of each `AppError` code (`NameEmpty`, `NameTaken`, `DirectoryMissing`, `NotADirectory`); created workspace appears in the list with its icon and color

### Implementation

- [X] T015 [P] Implement `AppError` with codes `NameEmpty`, `NameTaken`, `DirectoryMissing`, `NotADirectory`, `WorkspaceNotFound`, `NodeNotFound`, `WorkspaceNotLoaded`, `TerminalBusy`, `Io`, `Internal` and English messages, serializing as `{code, message}`, in `crates/core/src/error.rs`
- [X] T016 [P] Implement data root resolution, `0700`/`0600` creation helpers, `..` rejection, UUID folder validation, and the deletion guard in `crates/core/src/paths.rs`
- [X] T017 [P] Implement atomic write (`NamedTempFile::new_in` same dir → write → `sync_all` → `persist` → fsync dir, mode `0600`) in `crates/core/src/persistence/atomic.rs`
- [X] T018 Implement envelope encode/decode (sorted keys via `serde_json::Value`, 2-space indent) in `crates/core/src/persistence/envelope.rs` and the per-type migration runner (`fn(Value) -> Result<Value>` steps) in `crates/core/src/persistence/migrate.rs` (depends on T017)
- [X] T019 [P] Implement models in `crates/core/src/model/`: `workspace.rs` (`Workspace` fields per data-model.md; `Camera` with "`zoom` clamped to [0.1, 4.0]"), `node.rs` (`Node`, `Frame`, `NodeContent` with custom serde enforcing "Exactly one key: `terminal` or `note`. More or none → decode error"; `TerminalContent` with `name` "Trimmed, 1–60 chars. Default `Terminal N`", `launchCommand?`, `workingDirectory?`, `saveHistory` "Default `true`"; `NoteContent` empty), `app_state.rs` (`recentWorkspaceIds` "max 10", `workspaceOrder`, crash fields), `preferences.rs` (`autosaveIntervalSeconds` 5–600 default 30, `terminalFontSize` 9–24 default 13, `historyMaxRows` 1,000–100,000 default 10,000)
- [X] T020 Implement store in `crates/core/src/persistence/store.rs`: save/load `workspace.json`, discover workspaces by listing `workspaces/` (skip non-UUID folders and folders without `workspace.json`), load/save `app-state.json` and `preferences.json` with defaults when missing (depends on T018, T019)
- [X] T021 [P] Implement icon set (24 `lucide` icon ids) and 8-color palette with "random, preferring unused" selection in `crates/core/src/workspace/palette.rs`
- [X] T022 Implement `WorkspaceManager` in `crates/core/src/workspace/manager.rs`: `create` (validation order: name trimmed non-empty → unique case-insensitive → directory absolute, exists, is a directory), `list_summaries` (pinned first, then `workspaceOrder`, unknown ids ignored, missing ids appended), `open` (set active, update `lastOpenedAt`, push to recent max 10), revision counter for dirty tracking (depends on T016, T020, T021)
- [X] T023 Implement autosave task (tokio interval, saves only dirty workspaces, `save_all` for quit) in `crates/core/src/autosave.rs` (depends on T022)
- [X] T024 [P] Implement `EventSink` trait and `CoreEvent` enum matching the Events table of `contracts/ipc.md` in `crates/core/src/events.rs`
- [X] T025 [P] Implement logging (`tracing-appender` daily file `logs/sonata.log`, keep 7 days, never log terminal content) in `crates/core/src/logging.rs`
- [X] T026 Implement `core::App` facade (owns managers, preferences, event sink) and wire it in `src-tauri/src/main.rs` setup; implement `TauriEventSink` in `src-tauri/src/events.rs`; implement commands `app_bootstrap`, `workspace_create`, `workspace_open` in `src-tauri/src/commands/app.rs` and `src-tauri/src/commands/workspace.rs`; register them in `src-tauri/capabilities/default.json` (depends on T022–T025)
- [X] T027 [P] Implement typed IPC wrappers mirroring `contracts/ipc.md` (types, commands, event listeners) in `ui/src/ipc/types.ts`, `ui/src/ipc/commands.ts`, `ui/src/ipc/events.ts`
- [X] T028 Implement workspaces store (summaries, active id, load failures, preferences) in `ui/src/state/workspaces.ts` (depends on T027)
- [X] T029 Implement `ui/src/sidebar/Sidebar.tsx`, `ui/src/sidebar/WorkspaceItem.tsx`, `ui/src/sidebar/CreateWorkspaceDialog.tsx` (name field, folder picker via `@tauri-apps/plugin-dialog`, error display), empty state, and layout in `ui/src/App.tsx` (depends on T028)

**Checkpoint**: App launches, creates and opens workspaces, and lists them. T009–T014 pass.

---

## Phase 3: User Story 1 - Run terminals on a canvas in one workspace (Priority: P1) 🎯 MVP

**Goal**: Real shells (optional launch command) on a pannable, zoomable canvas; nodes move,
resize, rename, and stack; layout and camera persist.

**Independent Test**: Create one workspace, add three terminals (one plain, two with a launch
command), type in each, move and resize them, pan and zoom, restart: layout and camera unchanged.

### Tests for User Story 1 (write first, must fail)

- [X] T030 [P] [US1] R1 spike: add `scripts/capture-pty.sh` (records raw PTY output with `script -q`) and fixtures `crates/core/tests/fixtures/{vim,top}.bin` (plus synthetic agent-TUI redraw, wrapping, color, and alternate-screen cases); in `crates/core/tests/screen_replay.rs` feed each fixture into a `ScreenModel`, generate the replay, feed it into a fresh model of the same size, and assert equal screen contents, attributes, cursor, and scrollback rows. If it fails for `vt100`, switch the implementation to `alacritty_terminal` behind the same trait
- [X] T031 [P] [US1] PTY tests in `crates/core/tests/terminal_pty.rs` (real `/bin/sh` and the login shell): starts in the workspace directory (`pwd`); node `workingDirectory` overrides it; missing directory falls back to home and reports a warning; launch command `echo hello` runs once and the shell stays open; env has `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=Sonata`, `TERM_PROGRAM_VERSION`, `SONATA_TERMINAL_ID`; resize changes `stty size`; `exit 3` reports `Exited{code: 3}`; restart from `Exited` starts a new shell; `sleep 5` makes the terminal busy, idle prompt is not busy
- [X] T032 [P] [US1] Canvas node tests in `crates/core/tests/canvas_nodes.rs`: new terminal default name `Terminal N`; frame clamped to minimum "terminal 240×120"; bring to front sets "`max(z) + 1`"; deleting a busy terminal without `confirmed` → `TerminalBusy`; camera zoom clamped to [0.1, 4.0]; every change bumps the workspace revision
- [X] T033 [P] [US1] Attach tests in `crates/core/tests/terminal_attach.rs`: attach sends the replay first, then live bytes; output batches at most every 8 ms or 32 KB; detach keeps the process running and output keeps reaching the screen model; re-attach shows output produced while detached
- [X] T034 [P] [US1] Viewport math tests in `ui/tests/viewport.test.ts`: screen↔canvas conversion, zoom around the pointer keeps the point fixed, zoom clamped to [0.1, 4.0]
- [X] T035 [P] [US1] Gesture routing tests in `ui/tests/gestures.test.ts` covering FR-008a: wheel over focused terminal → terminal scroll; over unfocused terminal, note, or empty canvas → pan; pinch → zoom at pointer; middle-drag → pan anywhere; Space+drag → pan only when nothing has keyboard focus; click on empty canvas clears focus; Escape does not clear focus
- [X] T036 [P] [US1] Playwright test in `e2e/canvas.spec.ts` (mocked IPC): add terminal, drag, resize, bring to front, rename, pan and zoom; assert the expected `node_update_frame`, `node_bring_to_front`, `terminal_rename`, and debounced `workspace_set_camera` calls

### Implementation for User Story 1

- [X] T037 [US1] Implement `ScreenModel` trait (`process`, `resize`, `replay() -> Vec<u8>`, `preview_lines()`, `history_rows(max)`) and the `vt100` implementation with scrollback = `historyMaxRows` in `crates/core/src/terminal/screen.rs`
- [X] T038 [US1] Implement PTY spawn per research R5 (login shell from `getpwuid` → `$SHELL` → `/bin/zsh`, `-l`, cwd fallback to home with warning, env vars, launch command written as `<command>\r`, reader thread feeding the screen model and attached channel, writer, resize) in `crates/core/src/terminal/pty.rs` (depends on T037)
- [X] T039 [P] [US1] Implement busy detection (foreground process group from the PTY master vs shell pid; fallback `libc::tcgetpgrp` on the raw fd) in `crates/core/src/terminal/busy.rs`
- [X] T040 [US1] Implement `TerminalManager` in `crates/core/src/terminal/manager.rs`: registry by node id; start, stop, restart; run states `NotStarted → Starting → Running{pid} → Exited{code}`; attach (replay then live, batching 8 ms / 32 KB), detach, write, resize; emits `terminal://state` (depends on T038, T039)
- [X] T041 [US1] Add node operations to `crates/core/src/workspace/manager.rs`: `create_terminal`, `update_frame`, `bring_to_front`, `delete` (confirmation rule), `set_camera`, `rename_terminal`, `set_launch_command`; start a workspace's terminals on first open (`Unloaded → Loaded`) (depends on T040)
- [X] T042 [US1] Implement commands `node_create_terminal`, `node_update_frame`, `node_bring_to_front`, `node_delete`, `workspace_set_camera`, `terminal_attach`, `terminal_detach`, `terminal_write`, `terminal_resize`, `terminal_rename`, `terminal_set_launch_command`, `terminal_restart` in `src-tauri/src/commands/node.rs` and `src-tauri/src/commands/terminal.rs`; register in `src-tauri/capabilities/default.json` (depends on T041)
- [X] T043 [P] [US1] Implement viewport math in `ui/src/canvas/viewport.ts` and the FR-008a gesture router in `ui/src/canvas/gestures.ts`
- [X] T044 [US1] Implement `ui/src/canvas/Canvas.tsx` (transformed container, pan/pinch/wheel via the router, focus state), `ui/src/canvas/selection.ts`, `ui/src/canvas/ResizeHandles.tsx`, an "Add terminal" toolbar action with optional name and launch command, and camera persistence debounced ≥ 200 ms; canvas state store in `ui/src/state/canvas.ts` (depends on T043)
- [X] T045 [P] [US1] Implement `ui/src/dialogs/ConfirmDialog.tsx` (title, body, confirm/cancel, Enter/Escape handling)
- [X] T046 [US1] Implement `ui/src/terminal/xtermSession.ts`: create xterm.js with `addon-unicode11` and `addon-webgl`, attach through a `Channel`, `onData` → `terminal_write`, `terminal_resize` only when the node size changes (not on zoom), dispose on detach
- [X] T047 [US1] Implement the screen-space overlay `ui/src/terminal/LiveTerminalLayer.tsx` per research R2: position each live terminal at its node's screen rect with `fontSize = terminalFontSize × zoom`, apply the canvas transform during gestures, snap crisp ~150 ms after the gesture ends (depends on T044, T046)
- [X] T048 [US1] Implement `ui/src/nodes/TerminalNode.tsx`: frame and title bar with inline name editing, exited state with exit code and Restart button (FR-017), close button that calls `node_delete` and shows `ConfirmDialog` on `TerminalBusy` (FR-018) (depends on T045, T047)

**Checkpoint**: US1 independent test passes; T030–T036 green. MVP usable with one workspace.

---

## Phase 4: User Story 2 - Many workspaces, switch freely (Priority: P1)

**Goal**: Unlimited workspaces with rename, pin, reorder, switch (terminals keep running), delete
with confirmation, and quit confirmation when agents run.

**Independent Test**: Create five workspaces, start a long-running command in A, switch to B, C,
and back: the command kept running and its output is complete.

### Tests for User Story 2 (write first, must fail)

- [X] T049 [P] [US2] Workspace management tests in `crates/core/tests/workspace_manage.rs`: rename follows create rules; pin puts the workspace first; reorder persists in `app-state.json`; 8 workspaces get 8 different colors and the 9th reuses one; delete stops its terminals, removes only `$SONATA_HOME/workspaces/<uuid>`, and leaves the working directory untouched; delete refuses any path failing the deletion guard; `workingDirectoryMissing` is true after the directory is removed
- [X] T050 [P] [US2] Multi-workspace runtime tests in `crates/core/tests/multi_workspace.rs`: a terminal in A keeps producing output into its screen model while B is opened and active; `app_quit_check` returns `{workspaceId, workspaceName, count}` for each workspace with busy terminals and an empty list when only idle shells run
- [X] T051 [P] [US2] Playwright test in `e2e/workspaces.spec.ts` (mocked IPC): create five workspaces, see three distinct validation errors, pin and drag-reorder, switch, delete with cancel then confirm (dialog names the workspace), quit dialog lists busy counts per workspace

### Implementation for User Story 2

- [X] T052 [US2] Add `rename`, `set_pinned`, `reorder`, `delete` (stop terminals, rename folder to `.trash-<uuid>` inside `workspaces/`, then remove it, after the deletion guard) and the `workingDirectoryMissing` check to `crates/core/src/workspace/manager.rs`
- [X] T053 [US2] Implement quit check and quit flow (`quit_check`, `quit(confirmed)`: save all, set `cleanShutdown = true`, stop terminals) in `crates/core/src/app.rs`
- [X] T054 [US2] Implement commands `workspace_rename`, `workspace_set_pinned`, `workspace_reorder`, `workspace_delete`, `app_quit_check`, `app_quit` in `src-tauri/src/commands/workspace.rs` and `src-tauri/src/commands/app.rs`; intercept window close and `RunEvent::ExitRequested` (prevent exit, ask the UI) in `src-tauri/src/main.rs`; register in capabilities
- [X] T055 [US2] Sidebar features in `ui/src/sidebar/Sidebar.tsx` and `ui/src/sidebar/WorkspaceItem.tsx`: inline rename, pin toggle, drag reorder, delete via `ConfirmDialog` naming the workspace, missing-directory warning badge, busy-terminal count badge
- [X] T056 [US2] Implement `ui/src/dialogs/QuitDialog.tsx` listing busy terminals per workspace and its listener for the quit request event in `ui/src/App.tsx`
- [X] T057 [US2] On workspace switch, detach all live terminal sessions of the previous canvas and load the new canvas without stopping processes, in `ui/src/state/canvas.ts` and `ui/src/terminal/LiveTerminalLayer.tsx`

**Checkpoint**: US1 and US2 both pass their independent tests.

---

## Phase 5: User Story 3 - Work survives restarts and crashes (Priority: P1)

**Goal**: Isolated load failures, crash-loop guard, crash-safe saves, terminal history restore
with launch command re-run, private file permissions.

**Independent Test**: Arrange two workspaces, write notes and run commands, `kill -9` the app,
restart: layouts back (≤ one autosave interval lost) and each terminal shows prior output.

### Tests for User Story 3 (write first, must fail)

- [X] T058 [P] [US3] Load-failure tests in `crates/core/tests/load_failures.rs`: each reason `InvalidJson`, `WrongType`, `NewerVersion{found, supported}`, `MigrationFailed{step}`, `DecodeFailed{detail}`, `IdMismatch` (folder name ≠ payload id) produces a `LoadFailure` with path; other workspaces still load; the failed file stays byte-identical after autosave ticks and quit
- [X] T059 [P] [US3] Crash-guard tests in `crates/core/tests/crash_guard.rs` per research R9: unclean shutdown inside the 30 s window increments `consecutiveEarlyCrashes`; clean shutdown or `outlivedLaunchWindow` resets it to 0; at ≥ 2 the active workspace is not restored and `crashGuardTripped` is true
- [X] T060 [P] [US3] Crash-kill test in `crates/core/tests/crash_kill.rs` with a child process that is the test binary itself re-run with `SONATA_SAVE_LOOP` set (saves workspaces in a tight loop): spawn, `SIGKILL` at random times 100 times, then assert every `workspace.json` decodes (SC-005)
- [X] T061 [P] [US3] History tests in `crates/core/tests/history.rs`: history saved on autosave and quit, capped at `historyMaxRows` and 8 MB (oldest rows dropped); reading a larger file stops at 8 MB; `saveHistory = false` writes no file and deletes an existing one; clear deletes the file; on restart the history is fed before a `── restored history · <time> ──` separator and a fresh shell; a terminal with a launch command runs it again (FR-016)
- [X] T062 [P] [US3] Permission tests in `crates/core/tests/permissions.rs`: all created folders `0700` and files `0600`; at startup, looser modes are fixed and a `persistence://warning` event is emitted
- [X] T063 [P] [US3] App-state tests in `crates/core/tests/app_state.rs`: an invalid `app-state.json` or `preferences.json` is renamed to `<name>.invalid-<timestamp>` and replaced by defaults with a warning; active workspace restored at launch; recent list "max 10"
- [X] T064 [P] [US3] UI tests in `ui/tests/recovery.test.tsx`: load failure item shows reason and path; crash-guard banner text; persistence warning banner; terminal menu shows "Save history" toggle and "Clear saved history"

### Implementation for User Story 3

- [X] T065 [US3] Return `LoadFailure{folderName, path, reason}` from discovery in `crates/core/src/persistence/store.rs` and mark failed folders read-only in `crates/core/src/workspace/manager.rs` so nothing writes them
- [X] T066 [US3] Implement crash guard in `crates/core/src/crash_guard.rs` and wire it in `src-tauri/src/main.rs` (check at startup before restore, 30 s timer sets `outlivedLaunchWindow`)
- [X] T067 [US3] Implement history save/restore in `crates/core/src/terminal/history.rs` and call it from `TerminalManager` (save on autosave and quit; restore before spawning on workspace open)
- [X] T068 [US3] Add the startup permission sweep with warning events to `crates/core/src/paths.rs`
- [X] T069 [US3] Handle invalid `app-state.json`/`preferences.json` (rename to `.invalid-<timestamp>`, defaults, warning) in `crates/core/src/persistence/store.rs`
- [X] T070 [US3] On save failure keep data in memory, emit `persistence://warning`, and retry on the next tick, in `crates/core/src/autosave.rs`
- [X] T071 [US3] Implement commands `terminal_set_save_history` and `terminal_clear_history` in `src-tauri/src/commands/terminal.rs`; register in capabilities
- [X] T072 [US3] Implement `ui/src/sidebar/LoadFailureItem.tsx` (reason, path, Reveal in Finder), crash-guard and persistence-warning banners in `ui/src/App.tsx`, and the terminal menu entries in `ui/src/nodes/TerminalNode.tsx`

**Checkpoint**: US1–US3 pass. P1 scope complete.

---

## Phase 6: User Story 4 - Markdown notes beside the terminals (Priority: P2)

**Goal**: Markdown sticky notes stored as `.md` files, edited in place, synced with external edits.

**Independent Test**: Add a note, write Markdown, see it rendered, restart: note unchanged and
present as a readable `.md` file.

### Tests for User Story 4 (write first, must fail)

- [ ] T073 [P] [US4] Note tests in `crates/core/tests/notes.rs`: creating a note writes an empty `notes/<nodeId>.md`; minimum frame "note 120×80"; `note_write` marks dirty and the next save writes the file; deleting the node removes the file; external edit with no in-app edits emits `note://changed` within 2 s; external edit while dirty writes `<nodeId> (conflict).md`, keeps in-app text, emits `note://conflict`; the app's own writes do not trigger reloads (hash check); a deleted file sets `FileMissing` and is rewritten at the next save
- [ ] T074 [P] [US4] Note UI tests in `ui/tests/note-node.test.tsx`: double-click enters edit mode with a textarea; blur renders Markdown (heading, GFM list, table); conflict warning shown on `note://conflict`

### Implementation for User Story 4

- [ ] T075 [US4] Implement note file read/write/delete with last-written content hash and `NoteSync` states `Clean | Dirty | Conflict{path} | FileMissing` in `crates/core/src/notes/store.rs`
- [ ] T076 [US4] Implement the watcher per research R8 (`notify-debouncer-full`, 300 ms, per loaded workspace `notes/`) in `crates/core/src/notes/watcher.rs` (depends on T075)
- [ ] T077 [US4] Add `create_note` and note deletion to `crates/core/src/workspace/manager.rs`; implement commands `node_create_note`, `note_read`, `note_write` in `src-tauri/src/commands/note.rs`; register in capabilities (depends on T076)
- [ ] T078 [US4] Implement `ui/src/nodes/NoteNode.tsx` (`react-markdown` + `remark-gfm` render, textarea edit, `note_write` debounced and on blur, conflict and missing-file warnings) and an "Add note" toolbar action in `ui/src/canvas/Canvas.tsx`

**Checkpoint**: US4 passes independently.

---

## Phase 7: User Story 5 - Many terminals stay fast (Priority: P2)

**Goal**: Only legible terminals are live; others show previews; WebGL use is bounded; processes
never stop when hidden.

**Independent Test**: 30 terminals printing continuously in one workspace: smooth pan/zoom,
immediate echo, complete output when brought into view.

### Tests for User Story 5 (write first, must fail)

- [ ] T079 [P] [US5] Live-set tests in `ui/tests/liveSet.test.ts`: a terminal is live only if it intersects the viewport and `terminalFontSize × zoom ≥ 7`; at most 12 live terminals use WebGL and the rest use the DOM renderer; `onContextLoss` switches that terminal to DOM
- [ ] T080 [P] [US5] Preview tests in `crates/core/tests/preview.rs`: only subscribed nodes emit `terminal://preview`; at most 1 event per second per node; lines equal the visible screen text
- [ ] T081 [P] [US5] Playwright test in `e2e/scale.spec.ts` (mocked IPC, 30 terminals): zoom out → all previews and no xterm instances; zoom into one → it becomes live; never more than 12 WebGL contexts

### Implementation for User Story 5

- [ ] T082 [US5] Implement preview snapshots and subscriptions (≤ 1 Hz per node) in `crates/core/src/terminal/preview.rs`, wire into `TerminalManager`, and add command `terminal_preview_subscribe` in `src-tauri/src/commands/terminal.rs`
- [ ] T083 [US5] Implement `ui/src/terminal/liveSet.ts` (legibility, WebGL budget of 12, DOM fallback, context-loss handling) and use it in `ui/src/terminal/LiveTerminalLayer.tsx` to attach on enter and detach on leave
- [ ] T084 [US5] Implement viewport culling in `ui/src/canvas/culling.ts` and the preview `<pre>` in `ui/src/nodes/TerminalNode.tsx`; update the preview subscription set (debounced) as the viewport changes
- [ ] T085 [US5] Create `scripts/bench-terminals.sh N [workspaces]` that writes workspaces into a temp `SONATA_HOME` with N terminal nodes whose launch command prints continuously, then starts the dev app
- [ ] T086 [US5] Run the bench for SC-003 (30 terminals) and SC-004 (100 terminals in 4 workspaces) and record frame times and findings in `specs/001-workspaces-canvas-terminals/perf-notes.md`

**Checkpoint**: All five stories pass their independent tests.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T087 [P] Expand `README.md`: build-from-source install, data folder layout (`contracts/file-formats.md`), security notice (shells run unsandboxed with user permissions)
- [ ] T088 [P] Audit `src-tauri/capabilities/default.json` so it grants exactly the commands in `contracts/ipc.md` plus `dialog:allow-open`
- [ ] T089 [P] Add a test in `crates/core/tests/logging.rs` asserting that terminal output written during a session never appears in `logs/`
- [ ] T090 Run `scripts/check.sh` and fix every fmt, clippy, type, and test failure
- [ ] T091 Run every manual scenario in `specs/001-workspaces-canvas-terminals/quickstart.md` in `npm run tauri dev` and record pass/fail in `specs/001-workspaces-canvas-terminals/quickstart-results.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup. Blocks all stories.
- **US1 (Phase 3)**: after Foundational.
- **US2 (Phase 4)**: after Foundational; T050, T053, T057 use the `TerminalManager` from US1 (T040), so run US2 after US1.
- **US3 (Phase 5)**: after Foundational; T061 and T067 extend US1's terminal manager, so run after US1. Independent of US2.
- **US4 (Phase 6)**: after Foundational; uses the canvas from US1 (T044). Independent of US2, US3.
- **US5 (Phase 7)**: after US1 (extends the live terminal layer and terminal manager).
- **Polish (Phase 8)**: after the stories in scope.

### Story completion order

```text
Setup → Foundational → US1 ─┬─▶ US2
                            ├─▶ US3
                            ├─▶ US4
                            └─▶ US5
```

### Within each story

- Test tasks first; confirm they fail.
- Core (`crates/core`) before Tauri commands before UI.
- Commit after each task or small group (Conventional Commits).

### Parallel Opportunities

- Setup: T005–T008 in parallel after T001–T004.
- Foundational: all tests T009–T014 in parallel; T015, T016, T017, T019, T021, T024, T025, T027 in parallel.
- After US1: US2, US3, US4, and US5 can proceed in parallel by different people (they touch different modules, except shared edits to `workspace/manager.rs` and `TerminalNode.tsx`, which need coordination).
- Inside each story, every `[P]` test task runs in parallel.

---

## Parallel Example: User Story 1

```bash
# Tests first, all in parallel:
Task: "R1 spike + screen replay tests in crates/core/tests/screen_replay.rs"
Task: "PTY tests in crates/core/tests/terminal_pty.rs"
Task: "Canvas node tests in crates/core/tests/canvas_nodes.rs"
Task: "Attach tests in crates/core/tests/terminal_attach.rs"
Task: "Viewport math tests in ui/tests/viewport.test.ts"
Task: "Gesture routing tests in ui/tests/gestures.test.ts"
Task: "Playwright canvas test in e2e/canvas.spec.ts"

# Then independent implementation files in parallel:
Task: "Busy detection in crates/core/src/terminal/busy.rs"
Task: "Viewport math and gesture router in ui/src/canvas/"
Task: "ConfirmDialog in ui/src/dialogs/ConfirmDialog.tsx"
```

---

## Implementation Strategy

### MVP first (User Story 1)

1. Phase 1 Setup, Phase 2 Foundational.
2. Phase 3 US1, starting with the R1 spike (T030), because its result can change the screen model.
3. **Stop and validate** with the US1 independent test and quickstart scenario 1.

### Incremental delivery

1. US1 → usable with one workspace.
2. US2 → the core motivation: unlimited workspaces. First version to share with friends.
3. US3 → durable across crashes; required before daily use.
4. US4, US5 → notes and scale.

### Notes

- `[P]` = different files, no dependency on incomplete tasks.
- Every story ends at a checkpoint where its independent test must pass.
- Do not merge with failing gates (`scripts/check.sh`).
