# Implementation Plan: Windows Support

**Branch**: `012-windows-support` | **Date**: 2026-10-29 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/012-windows-support/spec.md`

## Summary

Ship Sonata for x86_64 Windows 10 (1809+) and 11 as a per-user NSIS installer from the same
release run as the macOS disk image, with the same updater key and one update manifest. In
`core`, a `platform` module isolates the Unix-only calls (modes, home, process groups, `open`)
and adds Windows counterparts: user-only access lists, the profile folder, Job Objects, process
snapshots, `ShellExecuteW`. The CLI channel becomes a per-user named pipe on Windows with the
same JSON protocol and token. Terminals run PowerShell (pwsh first) through ConPTY, with Git
Bash, WSL, and Command Prompt per terminal and shell helpers so hand-typed agents get Sonata's
instructions. The shell adds a WebView2 page capture, per-workspace browser data folders, and
Windows shortcuts. CI runs core and CLI tests on a Windows runner. See
[research.md](research.md) (R0 inventory, R1–R20 decisions).

## Technical Context

**Language/Version**: Rust 2024 edition (toolchain from `rust-toolchain.toml`), TypeScript /
React 19.

**Primary Dependencies**: existing (`portable-pty` 0.9 with ConPTY, Tauri 2.12, `notify`).
New, Windows-only: `windows-sys` 0.60 (`core`, `cli`), `webview2-com` 0.39 and `windows` 0.62
(`src-tauri`); all three are already in `Cargo.lock` through wry/tao.

**Storage**: unchanged files under the data root (`%USERPROFILE%\.sonata` on Windows); one new
optional field `TerminalContent.shell` (no schema bump, see data-model.md).

**Testing**: `cargo test` on macOS (local and CI) and on `windows-latest` (CI); Vitest for the
UI keyboard and label helpers. Windows-only behavior is verified through GitHub Actions job
logs, because this Mac cannot run Windows.

**Target Platform**: macOS 15+ (unchanged) and Windows 10 1809+ / Windows 11, x86_64.

**Project Type**: desktop app (Tauri) + CLI.

**Performance Goals**: `sonata ask` round trip overhead ≤ 1 s (SC-005); terminal start within
the same budget as macOS (PowerShell startup itself excluded).

**Constraints**: no macOS regression (FR-030); no paid signing (FR-006); no network listener
(FR-014); custom usage providers never run unsandboxed (FR-025).

**Scale/Scope**: same as macOS (tens of terminals per workspace).

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Result |
|---|---|
| I Clean room | Pass: own design and names; named pipes, ACLs, ConPTY, NSIS are generic platform techniques. |
| II Core/shell | Pass: platform code, pipe server, shells, ACLs in `core`; `src-tauri` only adds the WebView2 capture and per-platform config. |
| III Test-first | Pass: Windows tests (R18) are written first and must fail (or not compile) on the Windows runner before the code lands; macOS tests stay. |
| IV Files | Pass: same envelope and layout; one optional field; a file from another OS that does not load is reported, never overwritten. |
| V Security | **Amend**: text names a Unix socket `0600`. Windows uses a named pipe with a user-only access list, remote clients rejected, server-owner check, same token authorization (R3). Same protection; wording amended in R19. |
| VI Distribution | **Amend**: text names only the universal disk image. Windows adds an unsigned per-user installer and the SmartScreen step; still zero cost (R16, R19). |
| VII Simplicity | **Amend**: "macOS only" becomes "macOS and Windows (x86_64)". New dependencies are justified below. |
| Tech constraints | **Amend**: CLI channel "Unix socket" becomes "Unix socket (macOS) or named pipe (Windows)"; app shell adds Windows 10 1809+. |

Gate result: pass with the constitution amendment to 1.2.0 as the first implementation task
(R19). Re-checked after design: no further change.

## Project Structure

### Documentation (this feature)

```text
specs/012-windows-support/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/ipc-pipe.md  contracts/release-manifest.md  contracts/shell-integration.md
├── checklists/requirements.md
└── tasks.md            # /speckit-tasks
```

### Source Code

```text
.specify/memory/constitution.md          # 1.2.0 amendment (R19)
crates/core/Cargo.toml                   # cfg(windows) windows-sys
crates/core/src/platform/{mod,unix,windows}.rs   # R1, R4, R5, R6, R12, R14
crates/core/src/paths.rs                 # uses platform::{make_private,is_private,home_dir}
crates/core/src/logging.rs               # private log files on both OSes
crates/core/src/messaging/server.rs      # transport split; Unix code unchanged
crates/core/src/messaging/pipe.rs        # Windows named pipe server + client helpers (R3)
crates/core/src/terminal/pty.rs          # default shell per OS, spawn args per shell kind
crates/core/src/terminal/shells.rs       # Windows shell discovery (R7)
crates/core/src/terminal/powershell.rs   # PowerShell helpers (R8)
crates/core/src/terminal/bash.rs         # Git Bash rc file (R8)
crates/core/src/terminal/busy.rs         # Windows process-tree check (R10)
crates/core/src/model/node.rs            # TerminalContent.shell
crates/core/src/app.rs                   # PATH join, open_default, shell choice, git lookup
crates/core/src/presets.rs               # PATH+PATHEXT agent lookup on Windows
crates/core/src/lanes/git.rs             # git.exe discovery, core.longpaths
crates/core/src/usage/{runner,detect,custom,system,codex}.rs   # R11
crates/core/src/device/{tools,process,mod}.rs, device/ios.rs   # R14
crates/core/tests/*_windows.rs, pipe.rs, platform_windows.rs   # R18; cfg(unix) gates elsewhere
crates/cli/Cargo.toml, src/client.rs, src/run.rs               # pipe client, spawn-and-wait (R9)
src-tauri/Cargo.toml                     # cfg(windows) webview2-com, windows
src-tauri/src/browser/{mod,snapshot}.rs  # data_directory, CapturePreview (R13)
src-tauri/src/menu.rs                    # per-platform accelerators (R15)
src-tauri/src/commands/*                 # shell list command for the New Terminal dialog
src-tauri/tauri.windows.conf.json        # NSIS, icon, WebView2, updater mode (R16)
src-tauri/windows/installer-hooks.nsh    # user PATH add/remove
src-tauri/icons/icon.ico
scripts/build-cli.sh                     # sonata.exe for Windows targets
scripts/release/latest-json.mjs (+ .d.mts, tests)   # windows-x86_64 entry (R17)
.github/workflows/ci.yml, release.yml    # Windows jobs (R17)
ui/src/platform.ts                       # isMac, keyLabel (R15)
ui/src/terminal/xtermKeys.ts, App.tsx, state/canvas.ts, sidebar/Sidebar.tsx
ui/src/dialogs/TerminalDialog.tsx        # shell picker on Windows
ui/src/nodes/DeviceNode.tsx, settings/UsageSettings.tsx
ui/tests/platform*.test.ts
README.md                                # Windows install, SmartScreen, shortcuts
```

**Structure Decision**: existing layout; one new `core` module folder (`platform/`) and new
files next to the code they extend. Windows code sits behind `cfg(windows)`; Unix code moves
without behavior change so the Linux branch (011) reuses it.

## Phases

1. **Setup**: constitution 1.2.0; Windows CI job (expected red); dependencies; `icon.ico`.
2. **Foundation**: `platform` module (home, private ACLs, open, executable, process alive,
   simplify); `paths.rs` and `logging.rs` on top of it; test gating so the workspace compiles
   and tests run on Windows.
3. **US2 terminals + messaging** (P1): named pipe server/client; shells discovery; PTY spawn per
   shell; PowerShell and Git Bash helpers; busy detection; `sonata run` spawn-and-wait; agent
   lookup with `PATHEXT`; usage runner Job Object (needed by built-in providers).
4. **US1 + US3 install and release** (P1): Tauri Windows config, installer hooks, build-cli,
   release job, manifest, README install section.
5. **US4 notes, links, browser, lanes** (P2): git discovery + long paths, path simplify in
   lanes, WebView2 data folder and capture, open default.
6. **US5 update** (P2): updater passive mode, manifest check on Windows.
7. **US6 conventions** (P2): shortcuts, labels, Android-only device node, custom providers
   message, Windows notifications.
8. **Polish**: README, quickstart run on CI, `scripts/check.sh`, rebase on `main` after 011 and
   009 merge; FR-027 check.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `windows-sys` dependency (core, cli) | Pipes with a security descriptor, ACLs, Job Objects, process snapshot, `ShellExecuteW` have no std API. | Process calls to `icacls`/`taskkill` are slow, locale-dependent, and untestable for security properties. Already in `Cargo.lock`. |
| `webview2-com` + `windows` (src-tauri, Windows only) | Page capture needs `ICoreWebView2::CapturePreview`. | Window screen grabs capture overlapping UI and need the node on screen. Already in `Cargo.lock` through wry. |
| Second IPC transport | Windows has no Unix sockets in std. | TCP localhost lets other local users connect (R3). |
| `TerminalContent.shell` field | Per-terminal shell choice (clarification). | A global setting only does not meet the clarification. |
