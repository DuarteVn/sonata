# Sonata

An open source macOS app: an infinite canvas where AI-agent terminals, notes, and the links
between them live side by side, organized into as many workspaces as you need.

> **Status:** pre-alpha, under active development. Work follows
> [Spec Kit](https://github.com/github/spec-kit): see `.specify/memory/constitution.md` for project
> principles and `specs/` for features.

## What it does today

- As many workspaces as you want, each tied to a project folder, in a sidebar you can pin,
  rename, reorder, and delete from.
- An infinite canvas per workspace with real terminals (your login shell, optionally starting a
  command such as `claude` or `codex`) and Markdown sticky notes.
- Terminals keep running when you switch workspaces or scroll them off screen. Off-screen or
  zoomed-out terminals show a light text preview.
- A tool bar (Select `V`, Terminal `T`, Note `N`), a New Terminal dialog with quick starts for
  Claude Code, Codex, OpenCode, Gemini CLI, and Shell, plus accent color, font size, roles, and
  an Orchestrator flag.
- Links between terminals and notes: drag from the dot on a selected item onto another one, or use
  the link button in its floating toolbar. Select a link and press Delete to remove it.
- Floating toolbars for a selected note (color, size, Markdown formatting, pictures) or terminal
  (edit, link, restart, delete). `Cmd+W` closes the selected item; `Cmd+Q` quits.
- Everything is saved as readable files in `~/.sonata/`, with crash-safe writes, autosave, and
  terminal history restored after a restart.

## Install (build from source)

```sh
git clone https://github.com/DuarteVn/sonata.git
cd sonata
scripts/install.sh        # builds Sonata.app and copies it to /Applications
```

Run the same script again to update.

An app you build yourself opens without any Gatekeeper prompt. A packaged download comes later.

## Security notice

Sonata runs shells, `git`, and AI agents with your user permissions, without a sandbox. Only run
commands you trust. Terminal history is saved to disk in plain text, readable only by your user;
turn it off per terminal (terminal menu → Save history) for sessions that print secrets.

## Your data

```text
~/.sonata/
├── app-state.json, preferences.json, roles.json
├── logs/sonata.log.<date>              kept 7 days, never contains terminal output
└── workspaces/<id>/
    ├── workspace.json                  layout: nodes, camera, settings
    ├── notes/<node-id>.md              your notes, editable with any editor
    ├── files/                          pictures inserted into notes
    └── terminals/<node-id>.history     saved terminal output
```

Formats are documented in `specs/001-workspaces-canvas-terminals/contracts/file-formats.md`.

## Development

### Prerequisites

- macOS 15 or later
- Xcode Command Line Tools: `xcode-select --install`
- Rust via [rustup](https://rustup.rs)
- Node.js 22 or later

### Run

```sh
npm install
npm run tauri dev
```

Sonata stores its data in `~/.sonata/`. To try things without touching your real data, point it
at a throwaway folder:

```sh
SONATA_HOME="$(mktemp -d)/sonata" npm run tauri dev
```

### Test

```sh
scripts/check.sh                 # fmt, clippy, Rust tests, type check, frontend tests
npx playwright install webkit    # once
npm run e2e                      # Playwright against the frontend with mocked IPC
scripts/bench-terminals.sh 30    # real app with 30 streaming terminals in a temp data folder
```

### Layout

| Path | What |
|---|---|
| `crates/core` | Model, persistence, terminals, notes. No Tauri dependency. |
| `src-tauri` | Thin Tauri shell: commands and events that call `core`. |
| `ui` | React + TypeScript frontend (canvas, sidebar, xterm.js). |
| `e2e` | Playwright tests with mocked IPC. |
| `specs` | Feature specifications, plans, and task lists. |

## License

[MIT](LICENSE)
