# Sonata

An open source macOS app: an infinite canvas where AI-agent terminals, notes, and the links
between them live side by side, organized into as many workspaces as you need.

> **Status:** pre-alpha, under active development. Work follows
> [Spec Kit](https://github.com/github/spec-kit): see `.specify/memory/constitution.md` for project
> principles and `specs/` for features.

## Security notice

Sonata runs shells, `git`, and AI agents with your user permissions, without a sandbox. Only run
commands you trust.

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
scripts/check.sh      # fmt, clippy, Rust tests, type check, frontend tests
npm run e2e           # Playwright against the frontend with mocked IPC
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
