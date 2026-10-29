<!--
Sync Impact Report
- Version change: 1.1.1 → 1.2.0 (2026-10-29)
- Modified principles: VI. Free, Open Distribution — adds the Linux install paths (AppImage that
  updates itself, Debian package installed by hand); VII. Simplicity & Incremental Delivery —
  target platforms are macOS and Linux x86_64 (feature 011, research R16).
- Modified section: Technology & Platform Constraints — adds the Linux baseline (Ubuntu 22.04 or
  equivalent with WebKitGTK 4.1).
- Templates requiring updates: none.

Earlier report:
- Version change: 1.1.0 → 1.1.1 (2026-10-23)
- Modified principle: VI. Free, Open Distribution — names two install paths: the prebuilt
  universal disk image from GitHub Releases (recommended for users) and the build-from-source
  script (contributors). Zero cost, ad-hoc signing, and the own updater key are unchanged
  (feature 005, research R11).
- Templates requiring updates: none.

Earlier report:
- Version change: 1.0.0 → 1.1.0 (2026-10-22)
- Modified section: Technology & Platform Constraints — CLI channel is newline-delimited JSON over
  a Unix socket with tokio on the app side; `hyper`/HTTP removed (feature 003, research R1).
- Templates requiring updates: none.

Previous report:
- Version change: (template, unversioned) → 1.0.0
- Modified principles: none (initial ratification)
- Added principles:
  I. Clean-Room Originality
  II. Core/Shell Separation
  III. Test-First (NON-NEGOTIABLE)
  IV. Files Are the Source of Truth
  V. Server-Side Authorization & Local Security
  VI. Free, Open Distribution
  VII. Simplicity & Incremental Delivery
- Added sections: Technology & Platform Constraints; Development Workflow & Quality Gates; Governance
- Removed sections: none
- Templates requiring updates: none. plan-template.md reads "Constitution Check" gates from this
  file at runtime; spec-template.md and tasks-template.md contain no principle-specific text.
- Deferred TODOs: none
-->

# Sonata Constitution

Sonata is an open source (MIT) desktop app for macOS and Linux: an infinite spatial canvas holding AI-agent
terminals, notes, and the links between them, organized into multiple workspaces.

## Core Principles

### I. Clean-Room Originality

Sonata is built from a behavior-only reference (`maestri-arquitetura.md`, section 14). That
reference describes what a similar product does; it is not a source to copy.

- Code MUST be written from scratch. No code, UI text, icons, prompts, embedded skills, or
  product names (Maestri, Maestro, Ombro, Partitura, Maestri Wire) may be copied.
- Domain vocabulary MUST be neutral and technical (for example: Workspace, Terminal, Note,
  Link, Orchestrator, Lane, Template). Each term has one meaning across code, UI, and docs.
- Generic techniques (git worktrees, atomic writes, path-prefix retargeting) are allowed, because
  they are common knowledge, not product expression.
- The reverse-engineering reference MUST NOT be committed to the public repository.

Rationale: the project is open source and must be free of copyright and trademark risk.

### II. Core/Shell Separation

- The Rust `core` crate owns the model, persistence, PTY management, IPC server, git operations,
  and all business rules. It MUST NOT depend on Tauri.
- `src-tauri` is a thin layer: Tauri commands and events that call `core`. It MUST NOT contain
  business rules.
- The CLI is a separate crate that builds one self-contained binary. It talks to the app only
  through the IPC protocol.
- The React frontend renders state and sends intents. `core` is the single source of truth for
  state; the frontend never persists data on its own.

Rationale: logic in `core` is testable without a GUI, and the shell can change without a rewrite.

### III. Test-First (NON-NEGOTIABLE)

- Tests MUST be written before the feature code, and MUST fail before implementation starts
  (red, green, refactor).
- Unit tests cover pure logic (path retargeting, envelope and migrations, id remapping, path
  validation).
- Integration tests that touch git MUST run in a temporary repository created by the test, never
  in a developer's real repository.
- End-to-end tests for agent communication MUST use fake PTYs with scripted output.
- A change that breaks an existing test MUST NOT be merged.

Rationale: Sonata runs shells and git on the user's real files; regressions are expensive.

### IV. Files Are the Source of Truth

- All persisted state is human-readable files under `~/.sonata/`.
- Every JSON file MUST use the envelope `{"type", "schemaVersion", "payload"}`. Migrations run on
  raw JSON, step by step, before decoding. `schemaVersion` exists from the first release.
- Writes MUST be atomic: write a temporary file in the same directory, `fsync`, then `rename`.
- Notes are stored as `.md` files; the workspace JSON stores only their references.
- Loading MUST discover workspaces by listing directories, not by trusting an index file.
- A file that fails to load or migrate MUST be reported in the UI as a load failure. It MUST NOT
  crash the app or be silently overwritten.
- Optional empty fields are omitted on write; keys are written in sorted order.

Rationale: users can read, back up, diff, and repair their data without the app.

### V. Server-Side Authorization & Local Security

- The CLI IPC socket is a Unix socket with permission `0600` inside the user's data directory.
  Any TCP listener binds to `localhost` only and requires a random per-terminal token.
- Authorization MUST be enforced in the server from the calling terminal's identity (terminal
  id → node → flags). The client is never trusted.
- Agent-to-agent commands MUST work only between nodes joined by a Link.
- Secrets MUST NOT appear in command-line arguments; use environment, stdin, or headers.
- Paths MUST be validated in code, not only in shell scripts: reject `..`, require exact
  directory prefixes, and remove only directories proven to be Sonata-managed checkouts.
- Destructive operations (delete, land/merge, overwrite on restore) MUST require explicit human
  confirmation in the UI and in the CLI.
- Archive extraction MUST reject unsafe paths (zip-slip).

Rationale: the app is unsandboxed and runs commands with the user's full permissions.

### VI. Free, Open Distribution

- Building, signing, and distributing Sonata MUST NOT require any paid service, including the
  Apple Developer Program and notarization.
- macOS binaries use ad-hoc signing (`signingIdentity: "-"`) and a universal build
  (`universal-apple-darwin`).
- macOS install paths: a prebuilt universal disk image from GitHub Releases (recommended for
  users) and a script that builds from source (contributors).
- Linux install paths: a prebuilt x86_64 AppImage from GitHub Releases (recommended; it updates
  itself), a Debian package from the same release (updated by hand), and the same build-from-source
  script.
- Auto-update uses the project's own updater key on every platform.
- The README MUST document the one-command install, the Gatekeeper "Open Anyway" step for
  browser downloads, and a security notice that the app runs shells without a sandbox.

Rationale: friends and contributors must be able to install and ship Sonata at zero cost.

### VII. Simplicity & Incremental Delivery

- Target platforms are macOS (Apple Silicon and Intel) and Linux x86_64. Platform-specific code
  lives behind `cfg(target_os)` in one module per OS. Other platforms are out of scope until this
  constitution is amended.
- Build the smallest slice that works, in phases. Multiple workspaces are part of the first phase
  because they are the main reason the project exists.
- New dependencies, abstractions, or configuration options MUST be justified in the plan's
  Complexity Tracking section.
- Features from the reference that no phase requires (remote companion, device simulators,
  local AI assistant) are out of scope until specified.

Rationale: a small team ships a working tool faster than a complete one.

## Technology & Platform Constraints

- **App shell**: Tauri 2, macOS 15 or later; Linux with glibc 2.35 or later and WebKitGTK 4.1
  (Ubuntu 22.04 or an equivalent distribution).
- **Frontend**: React, TypeScript, Vite. Terminal rendering with xterm.js. The WebGL renderer is
  used only for visible, legible terminals; all others use a lighter renderer or a static preview.
- **Core**: Rust, Cargo workspace with `crates/core`, `crates/cli`, and `src-tauri`; frontend in
  `ui/`.
- **PTY**: `portable-pty`. **Async and IPC**: `tokio`; the CLI channel is newline-delimited JSON
  over a Unix socket (no HTTP, no network listener).
- **Git**: the system `git` binary through process calls. No libgit2.
- **Language**: English for code, identifiers, UI text, commit messages, and documentation.
- **License**: MIT.

Changing an item in this section requires a constitution amendment (MINOR bump at least).

## Development Workflow & Quality Gates

- Work follows Spec Kit: specify → (clarify) → plan → tasks → implement, one feature per branch.
- Every plan MUST pass a Constitution Check against principles I to VII before Phase 0 research
  and again after design.
- Before merge: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and the
  frontend type check and tests MUST pass.
- Commits are small and scoped to one task or phase, using Conventional Commits.
- Each phase ends with a README update that explains how to run and test what was delivered.

## Governance

- This constitution supersedes other practices and guidance files in the repository.
- Amendments are made by a pull request that edits this file, updates the Sync Impact Report,
  and states the version bump. A maintainer approves before merge.
- Versioning follows semantic versioning:
  - MAJOR: a principle is removed or redefined in an incompatible way.
  - MINOR: a principle or section is added or materially expanded.
  - PATCH: clarifications and wording fixes with no change in meaning.
- Every code review checks compliance with the principles. A deviation MUST be recorded in the
  plan's Complexity Tracking section with its reason, or the change is rejected.

**Version**: 1.2.0 | **Ratified**: 2026-10-21 | **Last Amended**: 2026-10-29
