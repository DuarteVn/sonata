# Tasks: Windows Support

**Input**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: Required (constitution III). Each test task comes before its implementation task and
must fail first. Windows tests run on the `windows` CI job (this Mac cannot run Windows); a
Windows test "fails first" when the CI job is red before the implementation commit. Unix tests
run locally and on the existing CI job. Git tests create their own temporary repositories.

**Verification rule**: run `sh scripts/check.sh` before every commit that changes code; push
the branch and read the `windows` job log after each phase.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: User story from spec.md (US1–US6)

## Phase 1: Setup

- [x] T001 Amend the constitution to 1.2.0 per research R19 (Principles V, VI, VII, Technology & Platform Constraints; Sync Impact Report) in .specify/memory/constitution.md
- [x] T002 Add a `windows` job on `windows-latest` (rust toolchain with clippy, rust-cache, `cargo clippy -p sonata-core -p sonata-cli --all-targets -- -D warnings`, `cargo test -p sonata-core -p sonata-cli --no-fail-fast`, `cargo check -p sonata-app`; push trigger for `012-windows-support` while the feature is open) in .github/workflows/ci.yml
- [x] T003 [P] Add `[target.'cfg(windows)'.dependencies] windows-sys = { version = "0.60", features = [...] }` (features from research R2) to Cargo.toml workspace deps, crates/core/Cargo.toml, and crates/cli/Cargo.toml
- [x] T004 [P] Generate src-tauri/icons/icon.ico (16, 24, 32, 48, 64, 256 px) from src-tauri/icons/source.png

## Phase 2: Foundational (blocks every story)

**Goal**: the workspace compiles on Windows and the cross-platform API exists.

- [x] T005 Create crates/core/src/platform/{mod,unix,windows}.rs with the API of research R1 (`home_dir`, `make_private(path, Kind)`, `is_private(path, Kind)`, `open_default`, `is_executable`, `process_alive`, `simplify`, `user_sid` on Windows); move the Unix bodies unchanged from paths.rs, device/process.rs, device/tools.rs, usage/custom.rs; register `pub mod platform;` in crates/core/src/lib.rs
- [x] T006 [P] Write Unix tests that `make_private`/`is_private` keep `0700`/`0600` semantics and `home_dir` reads `HOME`, in crates/core/tests/permissions.rs (extend)
- [x] T007 [P] Write Windows tests in crates/core/tests/platform_windows.rs: `make_private` on a temp folder yields a protected DACL with only the user SID and SYSTEM (`OICI` on folders); a file created inside inherits a private DACL; a folder with an `Everyone` allow entry is not private; `simplify(r"\\?\C:\x")` → `C:\x`, `\\?\UNC\srv\s\x` → `\\srv\s\x`, long path keeps its prefix; `is_executable` accepts `.exe`/`.cmd`, rejects `.txt`; `process_alive` true for self, false for an exited child
- [x] T008 Implement the Windows bodies (research R4, R5, R6, R12, R14) in crates/core/src/platform/windows.rs
- [x] T009 Use `platform::{home_dir, make_private, is_private}` in crates/core/src/paths.rs (`DataRoot::from_env`, `ensure_private_dir`, `set_mode` callers, `enforce_private_tree`) without changing Unix behavior; keep `DIR_MODE`/`FILE_MODE` Unix-only
- [x] T010 Create log files private on both systems (mode `0600` on Unix, inherited private DACL on Windows) in crates/core/src/logging.rs
- [x] T011 Replace every remaining `var_os("HOME")` in crates/core/src (terminal/pty.rs, usage/system.rs, usage/codex.rs, device/tools.rs) and crates/core/examples/emulator_probe.rs with `platform::home_dir()`
- [x] T012 Gate Unix-only tests with `#![cfg(unix)]` or `#[cfg(unix)]` (files listed in research R0 "Tests"), and make crates/core/tests/common/mod.rs and common/fake_bin.rs compile on Windows (fake binaries as `.cmd` on Windows)
- [x] T013 Gate Unix-only code so `sonata-core` and `sonata-cli` compile on Windows with stubs that return a clear "not supported yet" error, to be replaced by later tasks: messaging/server.rs, cli/src/client.rs, cli/src/run.rs, terminal/pty.rs `login_shell`, terminal/busy.rs, usage/runner.rs, device/ios.rs (macOS only, research R14), lanes/git.rs discovery
- [x] T014 Push the branch and confirm the `windows` CI job compiles and runs the gated test set; record the result in this file (2026-10-29: compiles; first run showed folder fsync, ConPTY cursor request, and pipe accept issues, fixed in the next commit)

**Checkpoint**: `windows` job compiles core and CLI; macOS `scripts/check.sh` green.

## Phase 3: User Story 2 — Terminals with AI agents (P1) 🎯 MVP

**Goal**: PowerShell terminals, agent helpers, busy detection, and `sonata` messaging on Windows.
**Independent Test**: two linked terminals exchange `sonata ask` on the Windows runner
(crates/core/tests/cli_ask_windows.rs); a request without a token is refused.

### Command channel

- [x] T015 [P] [US2] Write pipe tests per contracts/ipc-pipe.md in crates/core/tests/pipe.rs (`#![cfg(windows)]`): round trip of one JSON line; pipe DACL is protected with exactly user + SYSTEM; second server on the same name fails (first-instance flag); client gets "Sonata is not running" for a missing pipe; busy retry succeeds when the only instance frees within 5 s; server stop unblocks the accept thread
- [x] T016 [US2] Implement the named pipe server and client helpers (research R3) in crates/core/src/messaging/pipe.rs; split crates/core/src/messaging/server.rs so `socket_path` returns the pipe name on Windows and `Server::start` uses the pipe (Unix code unchanged)
- [x] T017 [US2] Use the pipe client with the server-owner check in crates/cli/src/client.rs (Unix code unchanged)
- [x] T018 [P] [US2] Write crates/core/tests/cli_ask_windows.rs: an `App` with fake terminals answers `list`, `ask`, `reply` through the pipe; a request with a bad token is `unauthorized`; an unlinked caller cannot reach the target

### Shells and PTY

- [x] T019 [P] [US2] Write shell discovery tests with fake folders and `PATH` in crates/core/tests/shells_windows.rs: pwsh on PATH wins over Windows PowerShell; Git Bash found beside `git.exe` and in `%ProgramFiles%\Git\bin`; `wsl --list --quiet` UTF-16 output parses to names (pure parser test, runs on every OS in crates/core/src/terminal/shells.rs `#[cfg(test)]`); default flag on the first PowerShell
- [x] T020 [US2] Implement `ShellChoice` discovery (research R7, data-model.md) in crates/core/src/terminal/shells.rs
- [x] T021 [P] [US2] Write persistence tests for `TerminalContent.shell`: `{ "kind": "wsl", "distro": "Ubuntu" }` round-trips; omitted when `None`; `kind` must be one of `"pwsh" | "powershell" | "cmd" | "gitBash" | "wsl"`; `distro` "required when `kind` is `"wsl"`, else omitted"; a file without the field still loads as schema 3, in crates/core/tests/shell_choice_persistence.rs
- [x] T022 [US2] Add `ShellKind`/`TerminalShell` and `TerminalContent.shell` in crates/core/src/model/node.rs; accept `shell` in `NewTerminal` and terminal edit in crates/core/src/app.rs
- [x] T023 [P] [US2] Write unit tests for the PowerShell helper command (contracts/shell-integration.md): base64 of UTF-16LE text; defines `claude` and `codex` only when `Get-Command sonata` succeeds; launch command appended after helpers; no `-NoProfile`, no `-ExecutionPolicy`, in crates/core/src/terminal/powershell.rs `#[cfg(test)]`
- [x] T024 [US2] Implement crates/core/src/terminal/powershell.rs (research R8)
- [x] T025 [P] [US2] Write unit tests for `sonata.bashrc` content (loads `/etc/profile`, then `~/.bash_profile` else `~/.bashrc`, defines helpers guarded by `command -v sonata`) in crates/core/src/terminal/bash.rs `#[cfg(test)]`
- [x] T026 [US2] Implement crates/core/src/terminal/bash.rs and write the file under `<root>/run/bash/` at startup next to the zsh files in crates/core/src/app.rs
- [x] T027 [P] [US2] Write crates/core/tests/terminal_pwsh.rs (`#![cfg(windows)]`): spawn Windows PowerShell through `TerminalManager` in ConPTY, `Write-Output hello` appears in output, resize does not fail, `SONATA_TERMINAL_ID` is set, `cmd` launch-command path works
- [x] T028 [US2] Build the command per shell kind (PowerShell args, Git Bash `--rcfile`, cmd `/K`, WSL `-d`/`--cd` + `WSLENV`) and the Windows default shell in crates/core/src/terminal/pty.rs; join `PATH` with `std::env::join_paths` in `session_env` and use the Windows default shell for git discovery in crates/core/src/app.rs
- [x] T029 [P] [US2] Write a Windows busy test in crates/core/tests/terminal_pwsh.rs: idle shell is not busy; while `ping -n 5 127.0.0.1` runs it is busy; afterwards idle again
- [x] T030 [US2] Implement the Windows process-tree busy check (research R10) in crates/core/src/terminal/busy.rs (Unix code unchanged)

### Agents

- [x] T031 [P] [US2] Write crates/cli/tests/run_windows.rs (`#![cfg(windows)]`): `find_real` finds `claude.cmd` through `PATHEXT` and skips the folder holding `sonata.exe`; the child's exit code is returned; Claude hook command uses a quoted forward-slash path
- [x] T032 [US2] Implement spawn-and-wait with Ctrl+C ignored in the parent, `PATHEXT` lookup, and the hook path form (research R9) in crates/cli/src/run.rs and crates/cli/src/hook.rs
- [x] T033 [P] [US2] Write a test that preset detection on Windows finds `codex.cmd` on `PATH` without running a shell, in crates/core/tests/presets.rs (`#[cfg(windows)]` case)
- [x] T034 [US2] Implement Windows agent lookup (PATH + PATHEXT) in crates/core/src/presets.rs
- [x] T035 [P] [US2] Write Windows runner tests in crates/core/tests/usage_runner.rs (`#[cfg(windows)]`): output captured; timeout kills a child and its grandchild (Job Object); env contains only the Windows allowlist of research R11
- [x] T036 [US2] Implement the Job Object runner and Windows allowlist in crates/core/src/usage/runner.rs; skip the login-shell probe on Windows in crates/core/src/usage/detect.rs
- [x] T037 [US2] Add the `terminal_shells` command (returns `ShellChoice[]`, empty on macOS) in src-tauri/src/commands/node.rs and register it in src-tauri/src/lib.rs, src-tauri/build.rs, src-tauri/capabilities/default.json; add the type and call in ui/src/ipc/types.ts and ui/src/ipc/commands.ts
- [x] T038 [P] [US2] Write UI tests: the New Terminal dialog shows a shell picker only when `terminalShells` returns entries, preselects the default, and sends `shell` (omitted for the default), in ui/tests/terminalDialog.test.tsx
- [x] T039 [US2] Add the shell picker to ui/src/dialogs/TerminalDialog.tsx
- [x] T040 [US2] Push; confirm pipe, shells, pwsh PTY, busy, ask, run tests pass on the `windows` job (2026-10-29, ci run 40898348030: green)

**Checkpoint**: US2 green on Windows CI; macOS unchanged.

## Phase 4: User Story 1 — Install from a download (P1)

**Goal**: an NSIS per-user installer with `sonata.exe` on the user PATH.
**Independent Test**: CI artifact `Sonata_<v>_x64-setup.exe` builds; a friend installs it.

- [x] T041 [P] [US1] Write a test in scripts/release/tests/build-cli.test.sh that with `TAURI_ENV_TARGET_TRIPLE=x86_64-pc-windows-msvc` the script targets that triple and writes `src-tauri/resources/bin/sonata.exe` (cargo stubbed as in the existing tests)
- [x] T042 [US1] Write `sonata.exe` for Windows targets in scripts/build-cli.sh (macOS paths unchanged)
- [x] T043 [US1] Add src-tauri/tauri.windows.conf.json (research R16: `bundle.targets: ["nsis"]`, icons with `icons/icon.ico`, `webviewInstallMode: downloadBootstrapper`, `nsis.installMode: "currentUser"`, `installerHooks`, updater `installMode: "passive"`)
- [x] T044 [US1] Add src-tauri/windows/installer-hooks.nsh: `NSIS_HOOK_POSTINSTALL` adds `$INSTDIR\bin` to the user Path and `NSIS_HOOK_POSTUNINSTALL` removes it, through `[Environment]::SetEnvironmentVariable(..., 'User')`, without duplicates
- [x] T045 [US1] Make the Windows CLI folder (`<install>\bin`) the `cli_dir` passed to `AppOptions` in src-tauri/src/lib.rs (same lookup as macOS `Resources/bin`, `sonata.exe` name)
- [x] T046 [US1] Add a manual-dispatch `windows-bundle` CI job (or the release dry run of T049) that builds `npm run tauri build -- --bundles nsis` and uploads the installer as an artifact; run it and record the result (Done as the release dry run of T049/T050.)

## Phase 5: User Story 3 — Release both platforms (P1)

**Goal**: one release with macOS and Windows files and one manifest.
**Independent Test**: release dry run produces both builds and a manifest with three platforms.

- [x] T047 [P] [US3] Extend scripts/release/tests (latest-json tests) per contracts/release-manifest.md: darwin entries unchanged; `windows-x86_64` added when exe and sig exist; failure when only one exists; no Windows key when neither exists
- [x] T048 [US3] Implement the Windows entry in scripts/release/latest-json.mjs and scripts/release/latest-json.d.mts
- [x] T049 [US3] Add the `build-windows` job and make the publish step depend on both builds (research R17) in .github/workflows/release.yml; add a dry-run input that skips publishing; macOS steps unchanged
- [x] T050 [US3] Run the release dry run on the branch through `workflow_dispatch`; record both job results and the printed manifest here (2026-10-29, release run 40899717110: checks, build-macos, build-windows, publish green; `Sonata_0.2.0_x64-setup.exe` 5.5 MB with `.sig`; manifest has `darwin-aarch64`, `darwin-x86_64` unchanged and `windows-x86_64`; the v0.2.0 release was not changed.)

## Phase 6: User Story 4 — Notes, links, browser, lanes on Windows (P2)

**Goal**: lanes with Git for Windows, browser node on WebView2, default-app opening.
**Independent Test**: lanes tests pass on the Windows job; browser capture verified in the manual pass.

- [x] T051 [P] [US4] Make crates/core/tests/lanes_git.rs, lanes_guard.rs, lanes_cli.rs run on Windows (temporary repos, `git.exe`), add cases for a path with spaces and non-ASCII letters, and a guard case with `\\?\` and different letter case
- [x] T052 [US4] Implement git discovery on Windows (`git.exe` on PATH, then `%ProgramFiles%\Git\cmd\git.exe`), `-c core.longpaths=true`, and `platform::simplify` before git and cwd use in crates/core/src/lanes/git.rs, crates/core/src/lanes/ops.rs, crates/core/src/lanes/guard.rs
- [x] T053 [P] [US4] Write a test that `browser::normalize_url` rejects Windows drive paths and `file:` URLs (only http and https open, feature 004 R10) in crates/core/tests/browser_model.rs
- [x] T054 [US4] Use `platform::open_default` in crates/core/src/app.rs and src-tauri/src/browser/mod.rs
- [x] T055 [US4] Use `data_directory(<root>\browser\<ws>)` on Windows and delete the folder on store removal in src-tauri/src/browser/mod.rs and src-tauri/src/browser/snapshot.rs
- [x] T056 [US4] Implement the WebView2 `CapturePreview` PNG snapshot with the fallback message (research R13) in src-tauri/src/browser/snapshot.rs; add `webview2-com` and `windows` as `cfg(windows)` deps in src-tauri/Cargo.toml
- [x] T057 [US4] Push; confirm lanes tests and `cargo check -p sonata-app` pass on the `windows` job (2026-10-29, ci run 40898348030: lanes tests and `cargo check -p sonata-app` green)

## Phase 7: User Story 5 — Update on Windows (P2)

**Goal**: the existing update flow installs NSIS updates.
**Independent Test**: manual pass step 9 (quickstart).

- [x] T058 [P] [US5] Write a test that the update check reads `windows-x86_64` from a manifest fixture and rejects a bad signature (if the existing update tests in crates/core/src/update.rs cover only parsing, extend them with a Windows fixture) (Done as: the manifest's `windows-x86_64` entry is tested in ui/tests/latestJson.test.ts; signature checks are the updater plugin's, unchanged from macOS.)
- [x] T059 [US5] Confirm the quit-guard prompt runs before the updater restarts on Windows (passive NSIS install closes the app) in src-tauri/src/lib.rs; adjust if the Windows updater exits without the guard (Found: the Windows updater ends the process inside `install`; Sonata now saves and stops before it, in src-tauri/src/commands/update.rs.)

## Phase 8: User Story 6 — Windows conventions (P2)

**Goal**: Ctrl shortcuts, Windows labels, Android-only devices, custom providers message.
**Independent Test**: UI tests with a Windows user agent; string scan finds no macOS terms.

- [x] T060 [P] [US6] Write tests for ui/src/platform.ts (`isMac` from user agent, `keyLabel("mod+W")` → `⌘W` / `Ctrl+W`) in ui/tests/platform.test.ts
- [x] T061 [US6] Implement ui/src/platform.ts
- [x] T062 [P] [US6] Write tests for Windows terminal keys (Ctrl+C with selection copies, without selection passes through; Ctrl+V and Ctrl+Shift+V paste; Cmd mappings inactive) in ui/tests/xtermKeys.test.ts
- [x] T063 [US6] Implement Windows key handling in ui/src/terminal/xtermKeys.ts and ui/src/terminal/xtermSession.ts
- [x] T064 [P] [US6] Write tests: on Windows Ctrl+W closes the selected item only when no terminal has focus, Ctrl+Shift+W closes it from a terminal, Ctrl+Shift+S toggles the sidebar, tooltips use `keyLabel`, in ui/tests/shortcuts.test.tsx
- [x] T065 [US6] Implement the shortcuts and labels in ui/src/App.tsx, ui/src/state/canvas.ts, ui/src/sidebar/Sidebar.tsx; per-platform accelerators in src-tauri/src/menu.rs
- [x] T066 [P] [US6] Write a test that on Windows custom providers read as unavailable with the research R11 message and never start a process, in crates/core/tests/usage_custom_consent.rs (`#[cfg(windows)]` case)
- [x] T067 [US6] Disable custom providers on Windows in crates/core/src/usage/custom.rs and show the message in ui/src/settings/UsageSettings.tsx; make the notification text platform-neutral
- [x] T068 [P] [US6] Write Windows device tests: SDK default `%LOCALAPPDATA%\Android\Sdk`, `emulator.exe`/`adb.exe` detection, no iOS in the device list, in crates/core/tests/device_tools.rs (`#[cfg(windows)]` cases)
- [x] T069 [US6] Implement the Windows SDK lookup and iOS-less device list in crates/core/src/device/tools.rs and crates/core/src/device/mod.rs; hide the iOS section in ui/src/nodes/DeviceNode.tsx when unsupported
- [x] T070 [US6] Add a UI string scan test (no `Cmd`, `⌘`, `Finder`, `macOS` in texts shown on Windows, via `keyLabel` and platform branches) in ui/tests/windowsStrings.test.ts

## Phase 9: Polish & cross-cutting

- [x] T071 Update README.md: Windows install (installer, SmartScreen "More info → Run anyway", uninstall), shells, shortcuts table per platform, iOS macOS-only note, custom providers note, security notice
- [x] T072 Run `sh scripts/check.sh`; push; confirm `core-and-ui`, `windows` CI jobs are green; record results in quickstart.md (2026-10-29: `scripts/check.sh` green on macOS; ci run 40898348030 green on `core-and-ui` and `windows`)
- [ ] T073 After 011 (Linux) and 009 (session restore) merge into `main`: rebase, resolve the constitution wording into one platform list, re-run CI, and verify FR-027 (session restore on Windows) in the Windows job
- [ ] T074 Ask the user (sonata notify) to run the manual Windows pass of quickstart.md section 4 before the first public Windows release; do not open a PR, merge, tag, or release without their OK

## Dependencies

- Phase 1 → Phase 2 → all stories.
- US2 (Phase 3) before US1 (Phase 4): the installer must carry a working CLI and terminals.
- US1 → US3 (release needs the installer config) → US5 (updates need releases).
- US4 and US6 depend only on Phase 2 and can run in parallel with US1/US3.
- T073 waits for 011 and 009 on `main`.

## Parallel examples

- Phase 2: T006, T007 together; then T008 while T011 and T012 proceed.
- US2: T015, T019, T021, T023, T025 (tests in different files) together; T031, T033, T035
  together.
- US6: T060, T062, T064, T066, T068 together.

## Implementation strategy

MVP = Phase 1 + 2 + US2: Sonata core compiles and runs terminals and messaging on Windows, proven
on CI. Then US1 + US3 make it installable from a release, US4–US6 close feature parity, and
Polish documents it. Each phase ends with a push and a read of the `windows` job log.
