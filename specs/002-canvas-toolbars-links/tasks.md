---

description: "Task list for feature 002: canvas toolbars, New Terminal dialog, and visual links"
---

# Tasks: Canvas Toolbars, New Terminal Dialog, and Visual Links

**Input**: Design documents from `/specs/002-canvas-toolbars-links/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ipc.md,
contracts/file-formats.md, quickstart.md

**Tests**: Required (constitution III). Test tasks come first and must fail before implementation.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [X] T001 Enable Tauri `protocol-asset` in `src-tauri/Cargo.toml` and add `"assetProtocol": {"enable": true, "scope": []}` under `app.security` in `src-tauri/tauri.conf.json`; at setup in `src-tauri/src/lib.rs` allow exactly the data root with `asset_protocol_scope().allow_directory(root, true)` (research R6)
- [X] T002 [P] Add new error codes `LinkSelf`, `LinkExists`, `RoleNameTaken`, `ImageTooLarge`, `ImageUnsupported` to `crates/core/src/error.rs` and `ui/src/ipc/types.ts`

---

## Phase 2: Foundational

### Tests (write first)

- [X] T003 [P] Migration tests in `crates/core/tests/migration_v2.rs`: a v1 workspace file loads, gains `links: []`, saves as `schemaVersion: 2`; a v2 file with links to unknown nodes loads with those links dropped and a warning; a v3 file is a `NewerVersion` failure and stays byte-identical
- [X] T004 [P] Model defaults tests in `crates/core/tests/terminal_settings.rs` (first part): a 001 terminal decodes with `monitorActivity: true`, `orchestrator: false`, no accent, font, or role; a 001 note decodes with `color: yellow`, `textSize: 3`; unset optional fields are omitted on write
- [X] T005 [P] Toolbar placement tests in `ui/tests/toolbarPlacement.test.ts`: centered above the node; below when fewer than 56 px above; clamped inside the viewport horizontally; hidden when the node is off screen

### Implementation

- [X] T006 Bump `WORKSPACE_SCHEMA_VERSION` to 2 and add the v1→v2 step (adds `links: []`) in `crates/core/src/model/workspace.rs` and `crates/core/src/persistence/store.rs`; add `Workspace.links` with `#[serde(default)]`
- [X] T007 [P] Add `Link {id, a, b, created_at}` with canonical ordering `a < b` in `crates/core/src/model/link.rs`
- [X] T008 [P] Extend `TerminalContent` (`accent_color?`, `font_size?` 9–24, `role?: {name?, text}`, `monitor_activity` default true, `orchestrator` default false) and `NoteContent` (`color` enum default yellow, `text_size` 1–5 default 3) in `crates/core/src/model/node.rs`
- [X] T009 Drop links to unknown nodes on load with a warning in `crates/core/src/workspace/manager.rs`
- [X] T010 [P] Mirror the new types (`Link`, `TerminalSettings`, `NoteStyle`, extended contents, `WorkspaceView.links`) in `ui/src/ipc/types.ts`
- [X] T011 Add `tool`, `selection`, `links`, `attention`, `rawNotes`, `linking` state and actions to `ui/src/state/canvas.ts`; load links in `load(ws)`
- [X] T012 [P] Implement `placeToolbar(nodeRect, viewport, toolbarSize)` in `ui/src/canvas/NodeToolbar.tsx` (pure function exported for tests) and an empty `NodeToolbar` shell rendered in screen space above the live-terminal layer, hidden during gestures

**Checkpoint**: 001 data loads and saves as v2; T003–T005 pass.

---

## Phase 3: User Story 1 - Start an agent in two clicks (P1) 🎯 MVP

**Independent Test**: Start one terminal per preset and a plain shell using only the tool bar and dialog.

### Tests

- [X] T013 [P] [US1] Preset tests in `crates/core/tests/presets.rs`: the built-in list is Claude Code `claude`, Codex `codex`, OpenCode `opencode`, Gemini CLI `gemini`, Shell (none); detection with a fake login shell script reports found/not found; a shell that sleeps past 2 s yields `found: null` for all
- [X] T014 [P] [US1] Selection/tool keyboard tests in `ui/tests/keyboard.test.ts`: V/T/N switch tools only without terminal or text focus; Escape clears selection and cancels pending link; Delete/Backspace request deletion of the selection only without focus
- [X] T015 [P] [US1] Dialog tests in `ui/tests/terminalDialog.test.tsx`: picking a preset fills name and command; not-found presets show a hint and stay selectable; empty command allowed; Browse… sets the folder; Create calls `node_create_terminal` with the settings
- [X] T016 [P] [US1] Playwright `e2e/toolbar.spec.ts`: tool bar visible with Select active; `T` + canvas click + preset + Create places the terminal at the click point and returns to Select; `N` + click places a note

### Implementation

- [X] T017 [US1] Implement `crates/core/src/presets.rs` (built-in list; `detect(shell, timeout)` running `<shell> -l -c 'for p in …; do command -v "$p"; done'`) and `App::presets_detect`; command `presets_detect` in `src-tauri/src/commands/preset.rs`
- [X] T018 [US1] Accept `settings?: TerminalSettings` in `node_create_terminal` (`crates/core/src/app.rs`, `src-tauri/src/commands/node.rs`)
- [X] T019 [P] [US1] Implement `ui/src/canvas/keyboard.ts` (tool shortcuts, Escape, Delete routing) and wire it in `ui/src/canvas/Canvas.tsx`
- [X] T020 [US1] Implement `ui/src/canvas/ToolBar.tsx` (Select, Terminal, Note with tooltips and active state) and tool-aware canvas clicks in `ui/src/canvas/Canvas.tsx` (terminal tool: open dialog with the click point; note tool: create note at the click; then back to Select); remove the old top toolbar
- [X] T021 [US1] Implement `ui/src/dialogs/TerminalDialog.tsx` with the preset row and the Details tab (name, command, working folder with Browse…); replace `AddTerminalDialog`
- [X] T022 [US1] Click on a node selects it (`selection`) in `ui/src/nodes/TerminalNode.tsx`, `ui/src/nodes/NoteNode.tsx`, and `ui/src/terminal/LiveTerminalLayer.tsx`; click on empty canvas clears it

**Checkpoint**: US1 independent test passes.

---

## Phase 4: User Story 2 - Link nodes on the canvas (P1)

**Independent Test**: Star of links between three terminals and a note; move, delete, restart.

### Tests

- [X] T023 [P] [US2] Link tests in `crates/core/tests/links.rs`: create between two nodes (any kinds); reverse direction → `LinkExists`; self → `LinkSelf`; unknown node → `NodeNotFound`; delete link; deleting a node removes its links; links persist across reload; workspace marked dirty on each change
- [X] T024 [P] [US2] Geometry tests in `ui/tests/linkGeometry.test.ts`: anchors on facing edges for left/right/above/below layouts; path endpoints equal the anchors; overlapping nodes fall back to centers
- [X] T025 [P] [US2] Playwright `e2e/links.spec.ts` (mocked IPC): drag from link handle to another node creates a link and counts update; same pair again and self/empty drops create nothing; toolbar link action + click creates; click link + Delete removes; deleting a node removes its links

### Implementation

- [X] T026 [US2] Implement `crates/core/src/links.rs` (`create`, `delete`, `links_of`, node-deletion cleanup) and `App::create_link`/`delete_link`; remove links in `App::delete_node`
- [X] T027 [US2] Commands `link_create`, `link_delete` in `src-tauri/src/commands/link.rs`; register in `src-tauri/build.rs` and capabilities; add to `ui/src/ipc/commands.ts` and `e2e/fixtures/mockIpc.ts`
- [X] T028 [P] [US2] Implement `ui/src/canvas/linkGeometry.ts` (anchors, cubic Bézier path)
- [X] T029 [US2] Implement `ui/src/canvas/LinkLayer.tsx`: SVG below nodes in the scaled layer, non-scaling 2 px stroke, 12 px hit path, selected style, drag preview curve to the pointer
- [X] T030 [US2] Link handle on terminal and note nodes (`ui/src/nodes/LinkHandle.tsx`): drag starts linking, drop target found under the pointer (`[data-node-id]`, including live terminals); link count badge on nodes
- [X] T031 [US2] Toolbar link action flow (pending source, next node click links, Escape/empty click cancels) in `ui/src/state/canvas.ts` and `ui/src/canvas/Canvas.tsx`

**Checkpoint**: US1 and US2 pass.

---

## Phase 5: User Story 3 - Configure and edit a terminal (P2)

**Independent Test**: Create a terminal with every option, restart, edit; reuse a library role.

### Tests

- [X] T032 [P] [US3] Settings tests in `crates/core/tests/terminal_settings.rs` (second part): `terminal_update_settings` applies name, accent, font, role, flags at once; changing command or folder on a running terminal returns `appliesOnRestart: true` and the next restart uses them; font size outside 9–24 and unknown accent colors are rejected
- [X] T033 [P] [US3] Role library tests in `crates/core/tests/roles.rs`: save, list, unique name ignoring case (`RoleNameTaken`), delete; persists in `roles.json` (0600); terminals keep their copy after delete
- [X] T034 [P] [US3] Activity tests in `crates/core/tests/activity.rs`: a monitored terminal emits `terminal://state` when it goes busy and again when idle; an unmonitored one emits nothing on busy changes
- [X] T035 [P] [US3] UI tests in `ui/tests/terminalDialog.test.tsx` (extend): Appearance and Role tabs; save role to library; pick a role; edit mode pre-fills values and shows the restart notice; attention badge set on busy→idle while unfocused and cleared on focus (`ui/tests/attention.test.ts`)

### Implementation

- [X] T036 [US3] Implement `crates/core/src/roles.rs` and `App` role methods; commands `roles_list`, `role_save`, `role_delete` in `src-tauri/src/commands/role.rs`
- [X] T037 [US3] Implement `App::update_terminal_settings` (validation, `appliesOnRestart`, spec update in `TerminalManager`) and command `terminal_update_settings`
- [X] T038 [US3] Implement `crates/core/src/terminal/activity.rs` (500 ms thread over monitored terminals, emits on busy changes) started by `App`
- [X] T039 [US3] Add Appearance and Role tabs, Monitor activity and Orchestrator options, and edit mode to `ui/src/dialogs/TerminalDialog.tsx`
- [X] T040 [US3] Terminal node display in `ui/src/nodes/TerminalNode.tsx`: accent color, Orchestrator badge, role marker, activity indicator, attention badge; per-node font size in `ui/src/terminal/LiveTerminalLayer.tsx` and preview

**Checkpoint**: US3 passes.

---

## Phase 6: User Story 4 - Format notes from a toolbar (P2)

**Independent Test**: Apply every note toolbar action, restart, check the `.md` file.

### Tests

- [X] T041 [P] [US4] Markdown transform tests in `ui/tests/markdownEdit.test.ts`: wrap/unwrap bold, italic, strikethrough, inline code with and without selection; heading cycle 1→2→3→none; checklist and bullet prefixes; code block insertion; checkbox toggle by source line; a sample table checked against `remark` parsing (SC-005)
- [X] T042 [P] [US4] Image tests in `crates/core/tests/note_images.rs`: PNG/JPEG/GIF/WebP accepted by extension and magic bytes; other types → `ImageUnsupported`; over 20 MB → `ImageTooLarge`; copy lands in `files/` with 0600 and a unique name; the returned Markdown is `![](../files/<name>)`; the source file is unchanged
- [X] T043 [P] [US4] Note style tests in `crates/core/tests/notes.rs` (extend): `note_set_style` persists color and size; invalid size rejected; `note_read` returns `assetBase`

### Implementation

- [X] T044 [US4] Implement `ui/src/notes/markdownEdit.ts`
- [X] T045 [US4] Implement `crates/core/src/notes/images.rs`, `App::note_insert_image`, `App::note_set_style`, `assetBase` in `note_read`; commands in `src-tauri/src/commands/note.rs`
- [X] T046 [US4] Note toolbar in `ui/src/canvas/NodeToolbar.tsx` (color palette, size −/+, formatting buttons, image picker via `@tauri-apps/plugin-dialog` with image filters, raw toggle, link action with count, delete)
- [X] T047 [US4] Update `ui/src/nodes/NoteNode.tsx`: color and size from style, editor ref for formatting at the cursor, raw view, images resolved with `convertFileSrc(assetBase + src)`, broken-image marker, checkbox toggling in rendered view

**Checkpoint**: US4 passes.

---

## Phase 7: User Story 5 - Act on a selected terminal (P3)

### Tests

- [X] T048 [P] [US5] Restart confirmation test in `crates/core/tests/canvas_nodes.rs` (extend): restart of a busy terminal without `confirmed` → `TerminalBusy`; with it, a new shell starts with the current command
- [X] T049 [P] [US5] Playwright in `e2e/toolbar.spec.ts` (extend): terminal toolbar shows edit, link with count, restart, delete; restart of a busy terminal asks first

### Implementation

- [X] T050 [US5] Add `confirmed` to `terminal_restart` (`crates/core/src/app.rs`, `src-tauri/src/commands/terminal.rs`, `ui/src/ipc/commands.ts`)
- [X] T051 [US5] Terminal toolbar variant in `ui/src/canvas/NodeToolbar.tsx` (edit opens `TerminalDialog` in edit mode, link action, restart with confirmation, delete with confirmation); remove duplicated title-bar actions that the toolbar replaces, keeping close

**Checkpoint**: All stories pass.

---

## Phase 7b: User feedback

- [X] T055 [US5] Replace the default window menu with an app menu whose File → Close Item (Cmd+W) emits `app://close-selected`; the UI closes the selection or focused terminal with the busy confirmation (FR-028) in `src-tauri/src/menu.rs`, `src-tauri/src/lib.rs`, `ui/src/App.tsx`, `ui/src/state/canvas.ts`

## Phase 8: Polish

- [X] T052 [P] Update `README.md` (tool bar, shortcuts, links, roles) and the 001 contract cross-references
- [X] T053 Run `scripts/check.sh` and `npm run e2e`; fix all failures
- [X] T054 Run the manual scenarios in `specs/002-canvas-toolbars-links/quickstart.md` that can be run without a person (build, launch, logs) and list the rest for the user in `specs/002-canvas-toolbars-links/quickstart-results.md`

---

## Dependencies & Execution Order

- Setup → Foundational → US1 → US2 → (US3, US4, US5 in any order) → Polish.
- US2 needs the selection state and toolbar shell from Foundational, and the link action reuses
  the toolbar from US4/US5 only as a button (the drag flow does not).
- US5 depends on US3's edit dialog for its Edit action.

## Parallel Example: Foundational

```bash
Task: "Migration tests in crates/core/tests/migration_v2.rs"
Task: "Model defaults tests in crates/core/tests/terminal_settings.rs"
Task: "Toolbar placement tests in ui/tests/toolbarPlacement.test.ts"
```

## Implementation Strategy

MVP = US1 + US2 (tool bar, presets, links). Then US3–US5. Commit per story.
