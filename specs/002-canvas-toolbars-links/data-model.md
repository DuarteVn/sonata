# Data Model: Canvas Toolbars, New Terminal Dialog, and Visual Links

**Feature**: 002-canvas-toolbars-links · Extends `specs/001-workspaces-canvas-terminals/data-model.md`.

## Workspace (schemaVersion 2)

New field:

| Field | Type | Rules |
|---|---|---|
| `links` | `Link[]` | Default `[]`. Migration v1→v2 adds it. |

## Link

| Field | Type | Rules |
|---|---|---|
| `id` | UUID | Immutable. |
| `a`, `b` | node UUID | Both nodes exist in this workspace; `a != b`; stored with `a < b`. |
| `createdAt` | time | |

Uniqueness: one link per unordered pair `{a, b}`. Errors: `LinkSelf`, `LinkExists`,
`NodeNotFound`. Deleting a node removes every link with that node. On load, links with unknown
nodes are dropped (warning).

## TerminalContent (new optional fields)

| Field | Type | Default | Rules |
|---|---|---|---|
| `accentColor` | `#RRGGBB`? | none | One of the 8 workspace palette colors. |
| `fontSize` | integer? | none | 9–24; none = app preference. |
| `role` | `{name?: string, text: string}`? | none | `text` 1–20,000 chars; `name` set when picked from the library. |
| `monitorActivity` | bool | `true` | |
| `orchestrator` | bool | `false` | Stored only; powers in feature 003. |

## NoteContent (new optional fields)

| Field | Type | Default | Rules |
|---|---|---|---|
| `color` | enum | `yellow` | `yellow`, `blue`, `green`, `pink`, `purple`, `gray`. |
| `textSize` | integer | `3` | 1–5. |

## Role library (`roles.json`, type `roles`, v1)

| Field | Type | Rules |
|---|---|---|
| `roles[].id` | UUID | |
| `roles[].name` | string | Trimmed, 1–60 chars, unique ignoring case (`RoleNameTaken`). |
| `roles[].text` | string | 1–20,000 chars. |
| `roles[].createdAt` | time | |

## Preset (built in, not persisted)

| id | Name | Command |
|---|---|---|
| `claude` | Claude Code | `claude` |
| `codex` | Codex | `codex` |
| `opencode` | OpenCode | `opencode` |
| `gemini` | Gemini CLI | `gemini` |
| `shell` | Shell | (none) |

Detection result: `{id, found: true | false | null}` (`null` = unknown, detection timed out).

## Note image (file)

`workspaces/<id>/files/<uuid>.<png|jpg|gif|webp>`, mode `0600`, ≤ 20 MB, referenced from note
Markdown as `../files/<uuid>.<ext>`.

## UI-only state

- `tool`: `select | terminal | note`.
- `selection`: `{kind: node | link, id} | null`.
- `attention`: set of terminal ids (busy → idle while unfocused).
- `rawNotes`: set of note ids shown as raw Markdown.
- `linking`: pending link source id (toolbar link action) or drag state.
