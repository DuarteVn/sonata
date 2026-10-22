# Research: Canvas Toolbars, New Terminal Dialog, and Visual Links

**Feature**: 002-canvas-toolbars-links | **Date**: 2026-10-22

Builds on the decisions of `specs/001-workspaces-canvas-terminals/research.md` (R1–R14 there).
No new runtime dependencies are needed except enabling Tauri's asset protocol (R6).

## R1. Workspace schema version 2

**Decision**: Bump the workspace format to `schemaVersion: 2` with a v1→v2 migration that adds
`links: []`. New node fields (terminal settings, note style) are optional with defaults.

**Rationale**: An older Sonata build reading a v2 file reports "created by a newer version" and
never writes it (001 FR-025), instead of silently dropping links on its next save. It also
exercises the migration chain for the first time.

**Alternatives considered**: additive fields under v1 (old builds would load the file and erase the
links on save).

## R2. Links as a workspace-level list

**Decision**: `Workspace.links: Link[]`, each `{id, a, b, createdAt}` with `a < b` (node ids
sorted) so a pair has one canonical form; uniqueness and self-link checks in `core`. Deleting a
node removes its links in the same update. On load, links to unknown nodes are dropped and a
warning is recorded.

**Rationale**: Links belong to the canvas, not to either node; one list is simple to persist,
validate, and query (`links_of(node)`), and feature 003 needs exactly this query for permissions.

## R3. Role library file

**Decision**: `~/.sonata/roles.json` (envelope type `roles`, v1) with `[{id, name, text,
createdAt}]`. Terminals store a copy: `role: {name?, text}`.

**Rationale**: Shared by all workspaces; a terminal keeps working if its library entry is deleted.

## R4. Preset program detection

**Decision**: Command `presets_detect` runs the user's login shell once:
`<shell> -l -c 'for p in claude codex opencode gemini; do command -v "$p"; done'` with a 2-second
timeout, and reports which names resolved. On timeout or error, all presets are reported as
unknown (no "not found" mark).

**Rationale**: GUI apps on macOS do not inherit the shell `PATH`; asking the login shell gives
the same answer the terminal will get. One process per dialog opening is cheap.

## R5. Activity states and attention

**Decision**: `core` runs one activity thread (every 500 ms) over terminals with
`monitorActivity` on. When the busy/idle state of one changes, it emits `terminal://state`
(already defined in 001 with `busy`). The UI keeps an `attention` flag per terminal: set when an
event goes busy → idle while that terminal is not focused, cleared on focus.

**Rationale**: Busy detection exists (001 R6); only change notification is new. Focus is a UI
concept, so attention lives in the UI.

## R6. Note images

**Decision**: Command `note_insert_image(nodeId, sourcePath)` checks type (PNG, JPEG, GIF, WebP by
extension and magic bytes) and size (≤ 20 MB), copies the file to
`workspaces/<id>/files/<uuid>.<ext>` with mode `0600`, and returns the Markdown
`![](../files/<uuid>.<ext>)` (relative to `notes/`). `note_read` also returns `assetBase`, the
absolute `notes/` folder, so the UI resolves relative image paths with Tauri's `convertFileSrc`.
The asset protocol is enabled (`tauri` feature `protocol-asset`) with an empty static scope; at
startup the shell allows exactly the data root (`asset_protocol_scope().allow_directory(root,
true)`), which also covers a `SONATA_HOME` override.

**Rationale**: Notes stay plain Markdown that other viewers render (images sit next to the notes);
the webview can only read Sonata's own data folder.

**Alternatives considered**: base64 data URLs inside the `.md` (huge files, unreadable Markdown).

## R7. Drawing links

**Decision**: One SVG layer inside the scaled canvas layer, below nodes. Each link is a cubic
Bézier between the facing edge midpoints of its two nodes, `vector-effect: non-scaling-stroke`
for a constant 2 px line, with a transparent 12 px stroke on top for hit testing. While a node is
dragged, frames update in the store and the curves follow on the next render.

**Rationale**: Native SVG in the same transform as nodes stays aligned for free and scales to 40
links without measurable cost.

## R8. Floating toolbars and the tool bar

**Decision**: Rendered in screen space inside the canvas container, above the live-terminal layer.
The node toolbar is centered above the node's screen rect (below it when there is less than 56 px
above), clamped to the viewport, and hidden while a pan/zoom gesture is in progress. The bottom
tool bar is a fixed element at bottom center.

## R9. Selection, tools, and keyboard

**Decision**: Canvas store gains `selection: {kind: "node" | "link", id} | null` and `tool:
"select" | "terminal" | "note"`, separate from 001's keyboard `focusedId`. Shortcut keys and
Delete/Backspace/Escape are handled on `window` only when neither a terminal nor a text field has
focus (same `hasTextFocus` rule as 001 FR-008a).

## R10. Markdown editing actions

**Decision**: Pure functions in `ui/src/notes/markdownEdit.ts` take `{text, start, end}` and
return the new `{text, start, end}`: `wrap(marker)` (toggle), `prefixLine(kind)` (heading cycles
`# `→`## `→`### `→none; checklist `- [ ] `; bullet `- `), `codeBlock()`. Checkbox clicks in the
rendered view use the source line that `react-markdown` reports for each list item and flip
`[ ]`↔`[x]` on that line.

**Rationale**: Pure text transforms are easy to test against a sample table (SC-005).

## R11. Restart confirmation

**Decision**: `terminal_restart` gains `confirmed`; without it, a busy terminal returns
`TerminalBusy`, like `node_delete`.

## R12. Per-terminal font size

**Decision**: `fontSize` override is passed to the live xterm session and to `grid_for_frame`;
the frontend's base font for that node is `fontSize ?? preferences.terminalFontSize`.
