# Implementation Plan: Canvas Toolbars, New Terminal Dialog, and Visual Links

**Branch**: `002-canvas-toolbars-links` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/002-canvas-toolbars-links/spec.md`

## Summary

Adds a bottom tool bar (Select, Terminal, Note), a New Terminal dialog with presets and
Details/Appearance/Role tabs, floating toolbars for a selected note or terminal, Markdown
formatting actions with image insertion, activity indicators, and persisted visual links between
nodes. `core` gains links (workspace schema v2 with migration), terminal settings, note style, a
role library, preset detection, note images, and busy/idle change events. The UI gains tool,
selection, and link-drawing state, an SVG link layer, and screen-space toolbars. See
[research.md](research.md).

## Technical Context

**Language/Version**: Rust stable 1.99 (edition 2024); TypeScript 7.

**Primary Dependencies**: unchanged from 001. Tauri's `protocol-asset` feature is enabled for note
images (scope set at runtime to the data root).

**Storage**: workspace format v2 (`links`, terminal settings, note style); new `roles.json`; note
images in `workspaces/<id>/files/`. See [contracts/file-formats.md](contracts/file-formats.md).

**Testing**: `cargo test` (links, migration v1→v2, settings, roles, presets with a fake shell,
images, activity events), Vitest (Markdown edit functions, link geometry, toolbar placement,
dialog), Playwright with mocked IPC (tool bar, dialog, links, toolbars).

**Target Platform**: macOS 15+, Apple Silicon and Intel.

**Project Type**: Desktop app (existing structure from 001).

**Performance Goals**: 30 nodes and 40 links: links follow a dragged node at display rate;
New Terminal dialog opens in under 300 ms (preset detection runs in the background, ≤ 2 s).

**Constraints**: older builds must refuse, not damage, v2 files; the webview may read only the
data root through the asset protocol; images ≤ 20 MB.

**Scale/Scope**: up to ~100 links per workspace; role library up to a few hundred entries.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Pre-design | Post-design |
|---|---|---|---|
| I. Clean room | Own layout, icons (lucide), wording; "Orchestrator", not the reference name. | Pass | Pass. Toolbars and dialog are designed from the spec's behavior list, not from screenshots' pixels. |
| II. Core/shell | Links, settings, roles, presets, images, activity in `core`; shell maps commands. | Pass | Pass. UI-only state (tool, selection, attention, raw view) stays in the frontend. |
| III. Test-first | Tests before each task. | Pass | Pass. Pure Markdown transforms and link geometry are unit-testable; `presets_detect` is tested with a fake shell script. |
| IV. Files | Envelope, migration, atomic writes, failures never overwritten. | Pass | Pass. v2 migration (R1) keeps old builds from overwriting new files; roles and images are private files. |
| V. Security | Path validation; private files; destructive ops confirmed. | Pass | Pass. Asset scope limited to the data root; image source path only read; restart of a busy terminal confirmed (R11). |
| VI. Free distribution | Unchanged. | Pass | Pass. |
| VII. Simplicity | Smallest slice; out-of-scope tools excluded. | Pass | Pass. No new libraries; SVG for links. |

Result: **PASS**.

## Project Structure

### Documentation (this feature)

```text
specs/002-canvas-toolbars-links/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/ipc.md, contracts/file-formats.md
├── checklists/requirements.md
└── tasks.md
```

### Source Code (changes)

```text
crates/core/src/
├── model/workspace.rs        # links, schema v2 + migration
├── model/node.rs             # TerminalContent settings, NoteContent style
├── model/link.rs             # Link, canonical pair
├── links.rs                  # create/delete/validate, drop dangling on load
├── roles.rs                  # role library store (roles.json)
├── presets.rs                # built-in presets + login-shell detection
├── notes/images.rs           # image validation and copy
├── terminal/activity.rs      # busy/idle change thread
└── app.rs                    # new facade methods
crates/core/tests/            # links.rs, migration_v2.rs, terminal_settings.rs, roles.rs,
                              # presets.rs, note_images.rs, activity.rs
src-tauri/src/commands/       # link.rs, role.rs, preset.rs; terminal.rs, note.rs extended
src-tauri/                    # Cargo feature protocol-asset; asset scope at setup
ui/src/
├── canvas/ToolBar.tsx        # bottom tool bar + shortcuts
├── canvas/LinkLayer.tsx      # SVG links, hit areas, drag preview
├── canvas/linkGeometry.ts    # anchor points and Bézier paths
├── canvas/NodeToolbar.tsx    # placement logic + note/terminal variants
├── canvas/keyboard.ts        # V/T/N/Delete/Escape routing
├── dialogs/TerminalDialog.tsx# New/Edit terminal with tabs and presets
├── notes/markdownEdit.ts     # pure formatting transforms
├── nodes/NoteNode.tsx        # color, size, raw view, images, checkbox toggle
├── nodes/TerminalNode.tsx    # accent, badges, activity, link handle
└── state/canvas.ts           # tool, selection, links, attention, linking
ui/tests/                     # markdownEdit, linkGeometry, toolbarPlacement, terminalDialog
e2e/                          # toolbar.spec.ts, links.spec.ts
```

**Structure Decision**: Same layout as 001; new modules only where a concept is new (links,
roles, presets, images, activity).

## Complexity Tracking

No new violations. The two justified items from 001 (ScreenModel trait, live-terminal overlay)
are unchanged.
