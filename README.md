# Sonata

An open source macOS app: an infinite canvas where AI-agent terminals, notes, and the links
between them live side by side, organized into as many workspaces as you need.

> **Status:** pre-alpha. Nothing to install yet. Work follows [Spec Kit](https://github.com/github/spec-kit);
> see `.specify/memory/constitution.md` for project principles and `specs/` for features.

## Planned stack

Tauri 2, React + TypeScript + Vite, xterm.js, Rust (`portable-pty`, `tokio`, `hyper`). macOS only.

## Security notice

Sonata runs shells, `git`, and AI agents with your user permissions, without a sandbox.

## License

[MIT](LICENSE)
