---
description: "Task list for 011 Linux support"
---

# Tasks: Linux Support

**Input**: Design documents from `specs/011-linux-support/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Required (constitution III, test-first). Pure functions are tested on every host;
Linux-only tests are `#[cfg(target_os = "linux")]` and run on the `linux-app` CI job.

**Verification**: this Mac cannot run Linux. Every Linux claim is checked in GitHub Actions logs
(push the branch, read `linux-app`). Run `scripts/check.sh` before every code commit.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US6 from spec.md

---

## Phase 1: Setup

- [X] T001 Amend `.specify/memory/constitution.md` to 1.2.0 (R16): principle VII targets macOS and Linux x86_64; principle VI adds the Linux AppImage (self-updating) and `.deb` (manual) install paths; Technology & Platform Constraints add "Linux: Ubuntu 22.04+ or equivalent with WebKitGTK 4.1"; update the Sync Impact Report
- [X] T002 Add a `linux-app` job to `.github/workflows/ci.yml` on `ubuntu-22.04`: apt install `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf file build-essential bubblewrap zsh xvfb imagemagick`; Rust stable with clippy; Node 22; `npm ci`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --no-fail-fast` (keep `core-and-ui` unchanged)
- [X] T003 Push the branch and read the `linux-app` log; record every Linux compile error or test failure as the baseline in `specs/011-linux-support/research.md` (section "CI baseline")

---

## Phase 2: Foundational (blocks all stories)

- [X] T004 Write failing tests in `crates/core/tests/platform_env.rs` for `platform::host_env` (data-model `EnvFix`): empty without `APPIMAGE`; removes `APPIMAGE`, `APPDIR`, `ARGV0`, `OWD`; drops `$APPDIR` entries from colon lists (`LD_LIBRARY_PATH`, `XDG_DATA_DIRS`, `GTK_PATH`); removes a variable left empty; keeps unrelated variables
- [X] T005 Create `crates/core/src/platform/mod.rs`, `macos.rs`, `linux.rs` (R1) with `EnvFix`, `host_env`, `EnvFix::apply_std(&mut std::process::Command)` and `EnvFix::apply_pty(&mut portable_pty::CommandBuilder)`, a cached `child_env()` from the real environment, and per-OS `default_shell()` (`/bin/zsh` macOS, `/bin/bash` Linux); register in `crates/core/src/lib.rs`
- [X] T006 Write failing tests in `crates/core/tests/platform_open.rs` for `platform::open_url_command(url)` and `platform::reveal_command(path)` returning the program and arguments per OS (contracts/platform.md rows "Open URL", "Reveal file")
- [X] T007 Implement `open_url_command`, `reveal_command`, and `platform::spawn_detached(program, args)` (applies `child_env()`) in `crates/core/src/platform/{mod,macos,linux}.rs`; replace `/usr/bin/open` in `crates/core/src/app.rs` (`reveal_in_finder`, help pages) and `src-tauri/src/browser/mod.rs` (`open_externally`)

**Checkpoint**: platform module exists; macOS behavior identical (scripts/check.sh green).

---

## Phase 3: User Story 1 - Friend installs Sonata on Linux (P1) 🎯 MVP

**Goal**: an AppImage and a `.deb` that start, show the canvas, and offer `sonata` in terminals.

**Independent Test**: `linux-app` builds both bundles; the smoke run under Xvfb shows the window,
creates the IPC socket, and copies the CLI to `~/.local/bin/sonata`.

### Tests

- [X] T008 [P] [US1] Write failing tests in `crates/core/tests/cli_copy.rs` for `platform::install_cli(bundled, target)` (rule kept in core, constitution II): writes when missing; skips when identical; overwrites only when the existing file contains `SONATA_TERMINAL_ID`; never overwrites another program
- [X] T009 [P] [US1] Write `scripts/release/tests/linux-smoke.sh`: start the AppImage with `xvfb-run` and a temporary `HOME`, wait up to 60 s for the socket under `$HOME/.sonata/run/`, check `$HOME/.local/bin/sonata` runs `help`, take `screenshot.png` with `import -window root`, check `dpkg-deb -c` lists `./usr/bin/sonata`, kill the app; print `socket ready`, `cli copied`, `deb has /usr/bin/sonata`

### Implementation

- [X] T010 [US1] Create `src-tauri/tauri.linux.conf.json` (R2): `bundle.targets` `["appimage","deb"]`, `bundle.linux.deb.files` `{"/usr/bin/sonata": "resources/bin/sonata"}`, deb `section` `devel`; keep `tauri.conf.json` unchanged for macOS
- [X] T011 [US1] Implement `platform::install_cli` in `crates/core/src/platform/mod.rs` and `install_cli_copy` in `src-tauri/src/lib.rs` (R7): copy `resource_dir()/bin/sonata` to `~/.local/bin/sonata` atomically with mode 0755 when `should_write` says so; log failures; call it from `src-tauri/src/lib.rs` setup under `#[cfg(target_os = "linux")]`
- [X] T012 [US1] Verify `cli_dir()` in `src-tauri/src/lib.rs` finds the CLI in the AppImage and `.deb` resource dirs (fix the lookup if the smoke run shows it missing)
- [X] T013 [US1] Use `platform::default_shell()` for the fallback in `crates/core/src/terminal/pty.rs` (`login_shell`) and `crates/core/src/app.rs:573`; apply `child_env()` to the PTY `CommandBuilder` in `pty.rs`
- [X] T014 [P] [US1] Append Linux font fallbacks (`"DejaVu Sans Mono", "Ubuntu Mono", "Liberation Mono"`, and `system-ui, "Ubuntu", "Cantarell", "Noto Sans"`) after the macOS fonts in `ui/src/styles.css` and `ui/src/terminal/xtermSession.ts`
- [X] T015 [US1] Extend `linux-app` in `.github/workflows/ci.yml`: `npm run tauri build -- --bundles appimage,deb --config '{"bundle":{"createUpdaterArtifacts":false}}'`, run `scripts/release/tests/linux-smoke.sh`, upload `screenshot.png` as artifact `linux-smoke`
- [X] T016 [US1] Push, read the log, download the screenshot, fix until the smoke run passes

**Checkpoint**: Linux app builds, starts, and exposes the CLI.

---

## Phase 4: User Story 2 - Agents work on Linux (P1)

**Goal**: bash and zsh helpers, messaging, lanes, presets, usage readers all work on Linux.

**Independent Test**: `bash_helpers` test on the runner types `claude` in a bash PTY and sees
`sonata run claude`; user's `~/.bashrc` variables load; lanes tests pass on Linux.

### Tests

- [X] T017 [P] [US2] Write failing unit tests in `crates/core/src/terminal/bash.rs` for the generated `bashrc` text: sources `/etc/profile`, then the first readable of `~/.bash_profile`, `~/.bash_login`, `~/.profile`; defines `claude` and `codex` only when `sonata` is on PATH
- [X] T018 [P] [US2] Write failing test `crates/core/tests/bash_helpers.rs` (skipped when `/bin/bash` is missing): spawn a bash terminal with a temp `HOME` holding `.profile` that sources `.bashrc`; check `$FROM_USER_RC` is set, `type -t claude` prints `function`, and the user's files are unchanged
- [X] T019 [P] [US2] Write failing unit test in `crates/core/tests/bash_helpers.rs` for `pty::shell_start(shell, helpers, user_zdotdir)`: bash → `["--rcfile", <bashrc>, "-i"]`; zsh → `["-l"]` plus `ZDOTDIR` env; fish → `["-l"]`
- [X] T020 [P] [US2] Write failing test for the Linux git hint in `crates/core/tests/lanes_git_hint.rs`: on Linux the missing-git message is "Git is not installed. Install it with your package manager, for example: sudo apt install git"; on macOS it stays the `xcode-select` text

### Implementation

- [X] T021 [US2] Implement `crates/core/src/terminal/bash.rs` (`write_files(dir)` → `bashrc`, private dir, atomic write) and register in `crates/core/src/terminal/mod.rs`
- [X] T022 [US2] Implement `shell_args` in `crates/core/src/terminal/pty.rs`; replace `zsh_dir` with `shell_helpers: Option<PathBuf>` (the `run/` folder holding `zsh/` and `bash/`) in `SpawnOptions`, `crates/core/src/terminal/manager.rs`, and `crates/core/src/app.rs` (write `run/bash` at start like `run/zsh`); update test helpers that build `SpawnOptions` (`crates/core/tests/terminal_pty.rs`, `terminal_attach.rs`, `zsh_helpers.rs`)
- [X] T023 [US2] Move git candidates and the install hint to `platform::{git_candidates, git_install_hint}` and use them in `crates/core/src/lanes/git.rs`; apply `child_env()` to git commands
- [X] T024 [P] [US2] Apply `child_env()` in `crates/core/src/presets.rs`, `crates/core/src/usage/runner.rs`, and `crates/core/src/device/process.rs`
- [X] T025 [US2] Push and confirm `bash_helpers`, `zsh_helpers`, lanes, messaging, and usage tests pass on `linux-app`

**Checkpoint**: agents typed by hand get Sonata in bash and zsh; lanes work.

---

## Phase 5: User Story 3 - One release for macOS and Linux (P1)

**Goal**: tag → draft → macOS + Linux builds → one `latest.json`.

**Independent Test**: `latest-json` tests pass; workflow lint passes; a manual run on a test tag
(only with the user's OK) produces the files in contracts/release.md.

### Tests

- [X] T026 [P] [US3] Write failing vitest cases in `ui/tests/latestJson.test.ts`: `linux` option adds `linux-x86_64` with its signature and URL; macOS entries unchanged; CLI rejects `--linux-sig-file` without `--linux-url` and vice versa; non-https Linux URL rejected

### Implementation

- [X] T027 [US3] Extend `scripts/release/latest-json.mjs` and `scripts/release/latest-json.d.mts` with the optional `linux` entry and the `--linux-sig-file` / `--linux-url` flags (contracts/release.md)
- [X] T028 [US3] Restructure `.github/workflows/release.yml` into `checks` → `draft` → `macos` | `linux` → `publish` (R4, contracts/release.md); macOS build steps unchanged except upload to the draft; `linux` on `ubuntu-22.04` with signing env, `--bundles appimage,deb`, uploads `Sonata_<v>_amd64.AppImage`, `.AppImage.sig`, `Sonata_<v>_amd64.deb`; `publish` downloads both `.sig` files with `gh release download`, builds and uploads `latest.json`, un-drafts, marks latest
- [ ] T029 [US3] Validate the workflow with `actionlint` as a CI step (`docker://rhysd/actionlint`) and a dry `linux` build in CI with signing disabled; do not tag

**Checkpoint**: release pipeline ready; a real tag needs the user's OK.

---

## Phase 6: User Story 4 - Linux app updates itself (P2)

**Goal**: AppImage self-updates; `.deb` and source builds point to the release page.

**Independent Test**: unit tests for `install_blocker` per bundle type; banner shows Blocked text.

### Tests

- [X] T030 [P] [US4] Write failing unit tests in `src-tauri/src/commands/update.rs` for a pure `linux_blocker(bundle, appimage_dir_writable)`: `Deb`/none → package text; AppImage + read-only → move text; AppImage + writable → none (contracts/platform.md "Update blockers"); macOS `wrong_location` cases unchanged

### Implementation

- [X] T031 [US4] Replace `wrong_location()` with `install_blocker() -> Option<String>` in `src-tauri/src/commands/update.rs` (macOS rule and text unchanged; Linux uses `tauri::utils::platform::bundle_type()` and `$APPIMAGE`'s folder)

**Checkpoint**: update flow correct per install type.

---

## Phase 7: User Story 5 - Platform features degrade clearly (P2)

**Goal**: browser data and snapshots, bwrap sandbox, Android-only device, Linux shortcuts and labels.

**Independent Test**: `usage_bwrap` and `platform_linux` pass on the runner; vitest for shortcut
labels and key overrides; smoke screenshot shows no macOS glyphs.

### Tests

- [X] T032 [P] [US5] Write failing unit tests in `crates/core/tests/usage_bwrap.rs` for `platform::linux::bwrap_args(home, root, runtime_dir, exists)` matching contracts/platform.md exactly (order, "if exists" entries); and Linux-only integration tests: a provider can read `usage-providers/ok.txt`, cannot read `<root>/secret.json` or `~/.codex/auth.json`, and nothing runs when the bwrap path is missing (message "Cannot run safely: bubblewrap (bwrap) is not installed. Install the bubblewrap package.")
- [X] T033 [P] [US5] Write failing tests in `crates/core/tests/platform_linux.rs` for Linux SDK candidates (`~/Android/Sdk`, `/opt/android-sdk`, `/usr/lib/android-sdk`), emulator discovery dirs (`$XDG_RUNTIME_DIR/avd/running`, `/tmp/android-$USER/avd/running`), and `ToolReport.ios == None`; keep macOS cases in `crates/core/tests/device_tools.rs` behind `cfg(target_os = "macos")` where they assume macOS paths
- [X] T034 [P] [US5] Write failing vitest tests in `ui/tests/platform.test.ts` for `shortcut("toggleSidebar")` ("⌃⌘S" mac, "Ctrl+Shift+S" Linux), `shortcut("closeItem")`, `revealLabel()` ("Reveal in Finder" / "Show in File Manager"), and in `ui/tests/xtermKeys.test.ts` for Linux Ctrl+Shift+C / Ctrl+Shift+V handling and unchanged mac Cmd keys

### Implementation

- [X] T035 [US5] Implement the sandbox selection in `crates/core/src/platform/{mod,macos,linux}.rs` (`Sandbox` enum, data-model) with the bwrap probe (`bwrap --ro-bind / / /bin/true`, cached), and use it in `crates/core/src/usage/custom.rs` (`read_custom`; macOS `sandbox_profile` and messages unchanged)
- [X] T036 [US5] Move Android SDK candidates, emulator discovery dirs, and iOS support to `platform` and use them in `crates/core/src/device/tools.rs` and `crates/core/src/device/emulator.rs`; make `ToolReport.ios` an `Option<IosReport>`; iOS commands return "iOS simulators need macOS." on Linux
- [X] T037 [US5] Update `ui/src/ipc/types.ts` (`ios: IosReport | null`) and hide the iOS section in `ui/src/nodes/DeviceNode.tsx` when null
- [X] T038 [US5] Linux browser data: in `src-tauri/src/browser/mod.rs` use `data_directory(<data>/browser-data/<ws>)` under `cfg(target_os = "linux")` (keep `data_store_identifier` on macOS); in `src-tauri/src/browser/snapshot.rs` `remove_data_store` deletes that folder on Linux after checking it is under the data root
- [X] T039 [US5] Linux snapshot in `src-tauri/src/browser/snapshot.rs` via WebKitGTK `snapshot(SnapshotRegion::Visible, SnapshotOptions::NONE)` → `cairo::ImageSurface` → PNG → atomic write; add `[target.'cfg(target_os = "linux")'.dependencies]` `webkit2gtk = "2.0"`, `cairo-rs = { version = "0.18", features = ["png"] }` to `src-tauri/Cargo.toml`; on any error call `done(false)`
- [X] T040 [US5] Linux menu in `src-tauri/src/menu.rs` (R11): Sonata (Settings… `Ctrl+,`, update items, Quit `Ctrl+Shift+Q`), File (Close Item `Ctrl+Shift+W`), View (Toggle Sidebar `Ctrl+Shift+S`); no Edit menu, no services/hide items; macOS menu unchanged
- [X] T041 [US5] Create `ui/src/platform.ts` (`isMac`, `shortcut`, `revealLabel`) and use it in `ui/src/App.tsx`, `ui/src/sidebar/Sidebar.tsx`, and `ui/src/sidebar/LoadFailureItem.tsx` ("Reveal in Finder" / "Show in File Manager")
- [X] T042 [US5] Linux terminal copy/paste (Ctrl+Shift+C / Ctrl+Shift+V) in `ui/src/terminal/xtermKeys.ts` and its caller in `ui/src/terminal/xtermSession.ts`; mac Cmd keys only when `isMac`
- [X] T043 [US5] Push and confirm `usage_bwrap`, `platform_linux`, and the src-tauri Linux build pass; check the smoke screenshot

**Checkpoint**: every macOS-only feature works or explains itself on Linux.

---

## Phase 8: User Story 6 - Build from source on Linux (P3)

**Goal**: `scripts/install.sh` installs on Linux under `~/.local`.

**Independent Test**: `install-linux.test.sh` with stubbed tools; real run in `linux-app`.

- [X] T044 [P] [US6] Write failing `scripts/release/tests/install-linux.test.sh` (stub `uname`, `pkg-config`, `cargo`, `npm`, `/etc/os-release` via `SONATA_OS_RELEASE`): missing WebKitGTK prints the apt command for Ubuntu and the dnf command for Fedora and exits 1; with everything present it calls `tauri build --bundles appimage` without updater artifacts; register in `scripts/release/tests/run.sh`
- [X] T045 [US6] Add the Linux branch to `scripts/install.sh` (R14): checks, build, install `~/.local/bin/sonata-app`, `~/.local/bin/sonata`, `~/.local/share/applications/sonata.desktop`, `~/.local/share/icons/hicolor/128x128/apps/sonata.png`; macOS branch byte-for-byte the same behavior
- [ ] T046 [US6] Run `scripts/install.sh` in `linux-app` after the bundle build (or a separate step) and check the installed files

---

## Phase 9: Polish & Cross-Cutting

- [X] T047 [P] Update `README.md`: Linux install (AppImage, `.deb`), supported distributions, required packages (`libwebkit2gtk-4.1-0`, optional `bubblewrap`), update behavior per install type, Linux shortcuts, features that differ, build from source on Linux
- [X] T048 [P] Fill "Portable today" and the decision outcome per inventory row M1–M30 in `specs/011-linux-support/research.md`
- [ ] T049 Run `scripts/check.sh` on macOS and confirm a full green `ci` run (both jobs) on the branch; record run ids in `specs/011-linux-support/quickstart.md`
- [ ] T050 `sonata notify` the user: branch ready, ask for OK to open a PR and for a release dry run

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories. T003 baseline first, so Linux compile errors are known early.
- US1 (Phase 3) is the MVP and unblocks the smoke run used by US2, US5, US6.
- US2 needs T005 (`child_env`, `default_shell`).
- US3 needs US1's bundle config (T010). US4 needs US3's manifest entry to be meaningful, but its
  code and tests are independent.
- US5 tasks are independent of each other except T037 after T036, and T043 last.
- US6 needs T010.
- Polish last.

## Parallel Opportunities

- T008, T009, T014 in US1; T017–T020 in US2; T032–T034 in US5; T044, T047, T048.

## Implementation Strategy

1. MVP: Phases 1–3 → Linux app builds and starts in CI.
2. Add US2 (agents) → US3 (release) → US4 (updates) → US5 (degradation) → US6 (source install).
3. After each phase: `scripts/check.sh`, commit, push, read `linux-app`.
