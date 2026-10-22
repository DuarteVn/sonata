# Quickstart: Validate Feature 002

Prerequisites, build, and automated checks are the same as
`specs/001-workspaces-canvas-terminals/quickstart.md`. Use a throwaway `SONATA_HOME`.

## Manual scenarios

### 1. Tool bar and New Terminal (US1)

1. Open a workspace. **Expect**: tool bar at the bottom with Select (active), Terminal, Note.
2. Press `T`, click the canvas, pick "Claude Code", Create. **Expect**: terminal at the click
   point running `claude`; tool back to Select (FR-003, SC-001).
3. Open the dialog again. **Expect**: presets for programs you do not have are marked not found
   (FR-007). Create a Shell terminal with Browse… to another folder; `pwd` shows it.
4. Press `N`, click the canvas. **Expect**: a note at the click point.

### 2. Links (US2)

1. Drag from a terminal's link handle onto another terminal and onto a note. **Expect**: curves,
   counts 2/1/1 (FR-021, FR-027).
2. Try linking the same pair again, a node to itself, and dropping on empty canvas. **Expect**:
   nothing new (FR-022).
3. Move and resize linked nodes. **Expect**: curves follow (FR-023).
4. Click a link, press Delete. Delete a linked node. **Expect**: links gone (FR-024, FR-025).
5. Restart. **Expect**: remaining links restored (FR-026).
6. Open the `workspace.json` with an older build of Sonata (001). **Expect**: load failure "newer
   version"; the file is unchanged.

### 3. Terminal settings (US3)

1. Create a terminal with Orchestrator on, a purple accent, font 16, and a role saved to the
   library as "Coordinator". **Expect**: badge, color, size, role marker (FR-012).
2. Create a second terminal and pick "Coordinator" from the role list.
3. Edit the first terminal: change its command. **Expect**: the dialog says it applies on restart
   (FR-011).
4. Run `sleep 5` in a monitored terminal, focus another node. **Expect**: running indicator, then
   idle with an attention badge; focusing clears it (FR-013).
5. Restart the app. **Expect**: every setting and the role library are kept (SC-004).

### 4. Note toolbar (US4)

1. Select a note; use each toolbar action. **Expect**: Markdown as in User Story 4; checkbox click
   toggles the item (FR-017, FR-019).
2. Insert a PNG. **Expect**: shown in the note; file in `workspaces/<id>/files/` (FR-018). A 30 MB
   image is refused.
3. Toggle raw/rendered; change color and size; restart. **Expect**: kept (FR-020).
4. Open the `.md` in another Markdown viewer. **Expect**: same rendering, image included (SC-005).

### 5. Terminal toolbar (US5)

1. Select a terminal running `sleep 100`; Restart. **Expect**: confirmation (FR-015).
2. Edit, link, delete from the toolbar.
