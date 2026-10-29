# Implementation Plan: Linux Support

**Branch**: `011-linux-support` | **Date**: 2026-10-29 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/011-linux-support/spec.md`

## Summary

Sonata runs on Linux x86_64 (Ubuntu 22.04+ and equivalents with WebKitGTK 4.1) with the same
features and data format as macOS. Platform-specific behavior moves behind a new
`crates/core/src/platform/` module (one file per OS) and `cfg(target_os = "linux")` blocks in
`src-tauri`. Linux gets bash shell integration, a bubblewrap sandbox for custom usage providers,
WebKitGTK snapshots and per-workspace browser data folders, `xdg-open`, Ctrl-based shortcuts that
never steal terminal keys, Android-only device support, and a cleaned environment for programs
started from an AppImage. The release workflow builds an AppImage (self-updating, manifest key
`linux-x86_64`) and a `.deb` on `ubuntu-22.04` next to the unchanged macOS build, then publishes
one manifest. CI builds and smoke-tests the Linux app under Xvfb. `install.sh` gains a Linux path.

## Technical Context

**Language/Version**: Rust 2024 edition (stable), TypeScript 5 / React 19, POSIX sh, Node 22

**Primary Dependencies**: Tauri 2.12 (wry 0.57, WebKitGTK 4.1 on Linux), `tauri-plugin-updater`
2.14; new Linux-only `webkit2gtk` 2.0 and `cairo-rs` 0.18 (already in `Cargo.lock` through wry);
system `bwrap` (bubblewrap) at run time for custom providers

**Storage**: unchanged `~/.sonata/` layout; Linux adds `browser-data/<workspace>/` (browser
storage) and `run/bash/bashrc` (generated)

**Testing**: `cargo test` (pure platform functions on every host; real bwrap, bash helpers, and
AppImage environment tests on the Linux runner), vitest (shortcut labels, key overrides,
manifest script), shell tests (install script, smoke script), CI smoke run under Xvfb

**Target Platform**: macOS 15+ (unchanged) and Linux x86_64 with glibc ≥ 2.35 and WebKitGTK 4.1,
X11 or Wayland

**Project Type**: desktop app + CLI + release automation

**Performance Goals**: start-up and terminal latency equal to macOS (no Linux-only work on hot
paths); Linux release job under 30 minutes

**Constraints**: no macOS regression; zero paid services; platform code isolated so the Windows
branch (012) rebases cleanly; verification only through GitHub Actions logs

**Scale/Scope**: about 30 code locations (research inventory M1–M30), tens of users

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| I. Clean-Room Originality | Pass | Generic platform techniques (bubblewrap, xdg-open, AppImage); no reference material used. |
| II. Core/Shell Separation | Pass | Platform rules (shell choice, sandbox args, env fix-up, SDK paths) in `crates/core/src/platform`; `src-tauri` only calls WebKitGTK, menus, and the updater. |
| III. Test-First | Pass | Pure functions (bwrap args, env fix-up, bashrc text, shortcut labels, manifest) get failing tests first; Linux-only integration tests run on the Linux runner. |
| IV. Files Are the Source of Truth | Pass | Same files and envelopes; new folders are caches/generated files, not state. |
| V. Authorization & Local Security | Pass | Socket rules unchanged; providers never run unsandboxed; CLI copy never overwrites non-Sonata files; workspace browser data deletion checks the data-root prefix. |
| VI. Free, Open Distribution | Pass with MINOR amendment | Linux AppImage + `.deb` on GitHub Releases, same updater key; amendment 1.2.0 (R16). |
| VII. Simplicity | Violation, justified by amendment | "macOS only" is lifted by amendment 1.2.0 (the user asked for Linux). New Linux-only crates listed below. |

Post-design re-check: unchanged; the amendment ships in this branch before the code that needs it.

## Project Structure

### Documentation (this feature)

```text
specs/011-linux-support/
├── plan.md
├── research.md           # inventory M1–M30 and decisions R1–R16
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── release.md        # artifacts, manifest, workflow jobs
│   └── platform.md       # per-OS behavior table, shortcuts, sandbox, environment
└── tasks.md              # /speckit-tasks
```

### Source Code (repository root)

```text
.github/workflows/
├── ci.yml                      # + linux-app job (ubuntu-22.04): clippy, tests, bundle, smoke
└── release.yml                 # checks → draft → macos | linux → publish
scripts/
├── install.sh                  # + Linux path (~/.local, desktop entry)
└── release/
    ├── latest-json.mjs         # + --linux-sig-file / --linux-url → linux-x86_64
    ├── latest-json.d.mts
    └── tests/
        ├── linux-smoke.sh      # new: AppImage under xvfb-run, socket, CLI copy, screenshot
        └── install-linux.test.sh # new: Linux branch of install.sh with stubbed tools
crates/core/src/
├── platform/
│   ├── mod.rs                  # new: shared API, EnvFix, re-exports
│   ├── macos.rs                # new: current macOS behavior moved here
│   └── linux.rs                # new: Linux behavior
├── terminal/
│   ├── pty.rs                  # default shell via platform; bash rcfile; env fix-up
│   ├── zsh.rs                  # unchanged
│   └── bash.rs                 # new: generated bashrc
├── usage/custom.rs             # sandbox selection via platform (sandbox-exec | bwrap)
├── usage/runner.rs             # env fix-up
├── presets.rs                  # env fix-up
├── lanes/git.rs                # install hint and candidates via platform; env fix-up
├── device/{tools,emulator,process}.rs # SDK defaults, discovery dirs, ios null; env fix-up
└── app.rs                      # open/reveal via platform; bash helper files
crates/core/tests/              # + platform_linux.rs, usage_bwrap.rs, bash_helpers.rs
src-tauri/
├── Cargo.toml                  # + [target.'cfg(target_os = "linux")'.dependencies]
├── tauri.linux.conf.json       # new: appimage + deb targets, deb files/depends
└── src/
    ├── browser/{mod,snapshot}.rs # Linux data_directory, WebKitGTK snapshot, data removal
    ├── commands/update.rs      # install_blocker per OS
    ├── cli_install.rs          # new: ~/.local/bin/sonata copy (Linux)
    ├── menu.rs                 # Linux menu without terminal-key accelerators
    └── lib.rs                  # wire cli_install on Linux
ui/src/
├── platform.ts                 # new: isMac, shortcut labels, file manager name
├── terminal/xtermKeys.ts       # Linux Ctrl+Shift+C / Ctrl+Shift+V
├── App.tsx, sidebar/Sidebar.tsx, sidebar/LoadFailureItem.tsx # labels from platform.ts
├── nodes/DeviceNode.tsx        # hide iOS when report.ios is null
├── ipc/types.ts                # ios: IosReport | null
└── styles.css, terminal/xtermSession.ts # Linux font fallbacks appended
.specify/memory/constitution.md # MINOR 1.2.0
README.md                       # Linux install, packages, updates, differences
```

**Structure Decision**: Existing Cargo workspace and `ui/`. Platform differences live in
`crates/core/src/platform/<os>.rs` and in `cfg` blocks inside the few `src-tauri` files that call
OS APIs.

## Complexity Tracking

| Addition | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Linux target (constitution VII) | User-requested feature | Staying macOS-only does not meet the goal |
| `webkit2gtk`, `cairo-rs` (Linux only) | Browser card pictures and `sonata browser screenshot` (FR-018) | Placeholder only loses a working feature; crates are already compiled through wry |
| `platform/` module | Keeps OS code in one file per OS for clean rebases with 012 | Inline `cfg` at ~20 call sites conflicts with the Windows branch |
| Env fix-up for AppImage children | System programs break with the AppImage's library paths (R6) | Clearing the app's own env breaks WebKit helper processes |
| Release `draft`/`publish` split | One manifest for several build jobs; failed platform keeps previous release latest | A single job cannot build on two runners |
| bwrap probe and argument builder | Never run providers unsandboxed (FR-019) | Disabling providers on Linux drops a feature |
