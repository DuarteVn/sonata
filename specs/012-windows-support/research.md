# Research: Windows Support

**Feature**: 012-windows-support | **Date**: 2026-10-29

## R0. Inventory of macOS/Unix-only code paths

Mapped on `main` at `e3cd926` with a search for `cfg(unix)`, `cfg(target_os)`, `cfg!(…)`,
`std::os::unix`, `libc::`, `sandbox-exec`, `simctl`, `xcrun`, `xcode-select`, `chmod`,
`set_permissions`, `PermissionsExt`, `UnixListener`/`UnixStream`, `/Applications`, `"HOME"`,
`"SHELL"`, `/bin/zsh`, `/bin/sh`, `/usr/bin/…`, `darwin`, and `macos`. No `osakit` or
`osascript` use exists. Each item names what blocks a Windows build or what behaves wrong on
Windows.

### Core crate (`crates/core/src`)

| Area | Location | macOS/Unix dependency | Windows impact |
|------|----------|-----------------------|----------------|
| Data root | `paths.rs:6,44-54,180-192` | `DirBuilderExt::mode(0o700)`, `PermissionsExt::from_mode`, `enforce_private_tree` checks `mode() & 0o777` | Does not compile; needs an ACL-based "private to current user" equivalent |
| Data root | `paths.rs:66` | `$HOME` for the default root | `HOME` is usually unset on Windows; use the user profile folder |
| Logging | `logging.rs:6,17,32` | `OpenOptionsExt::mode(0o600)` for log files | Does not compile; files inherit the private folder ACL instead |
| IPC server | `messaging/server.rs:5,20-35,45-46,74-91` | `UnixListener`/`UnixStream`, socket file `0600`, `libc::getuid` in fallback temp path, `MAX_SOCKET_PATH` | Does not compile; std has no Unix sockets on Windows; needs a per-user local channel |
| IPC client | `crates/cli/src/client.rs:4,44` | `UnixStream::connect(SONATA_SOCKET)` | Does not compile; must match the server channel |
| Agent launcher | `crates/cli/src/run.rs:4` | `CommandExt::exec` replaces the `sonata` process with the agent | No `exec` on Windows; spawn the agent, forward exit code, let console Ctrl+C reach the child |
| PTY shell | `terminal/pty.rs:10-32` | `getpwuid` login shell, `$SHELL`, fallback `/bin/zsh` | Does not compile; needs Windows shell discovery (pwsh, powershell, cmd, Git Bash, WSL) |
| PTY env | `terminal/pty.rs:84,110-121` | `$HOME` as working-dir fallback; `ZDOTDIR` shell integration only for zsh | Needs profile folder fallback and a PowerShell integration |
| Shell integration | `terminal/zsh.rs` (whole file), `app.rs:649-652,1489-1491` | zsh startup files under `<root>/run/zsh` | Needs a PowerShell counterpart (and Git Bash can reuse a bash variant or none) |
| Busy detection | `terminal/busy.rs:6-17` | `libc::tcgetpgrp`, `process_group_leader`, `pid_t` | Does not compile; Windows has no foreground process group; needs a process-tree check of the shell's children |
| Terminal manager | `terminal/manager.rs:35` | Tests use `/bin/sh` as shell | Tests need a Windows shell |
| App: default shell | `app.rs:572-573` | `$SHELL`, fallback `/bin/zsh` | Same as PTY shell |
| App: PATH | `app.rs:664` | fallback `/usr/bin:/bin:/usr/sbin:/sbin` | Windows uses `;` and `Path`; fallback must be the system default |
| App: open | `app.rs:1213,2091` | `/usr/bin/open` to open files/URLs | Use the Windows default handler (`ShellExecute` or `explorer`/opener plugin) |
| Presets | `presets.rs:83` | `<shell> -l -c "command -v …"` to find agent CLIs | `-l -c` is POSIX-only; on Windows search `PATH` + `PATHEXT` directly |
| Usage: env detection | `usage/detect.rs:13,80-97,128` | Login-shell `-c` script prints env vars (`SHELL_VARS`) | POSIX-only script; read the process/user environment on Windows |
| Usage: runner | `usage/runner.rs:4,13,143,167-181` | `CommandExt::process_group`, `killpg(SIGKILL)`, `waitid`, `siginfo_t`; `ENV_ALLOWLIST` with `HOME`, `TMPDIR` | Does not compile; use a Job Object to kill the tree; allowlist needs `USERPROFILE`, `TEMP`, `SystemRoot`, `PATHEXT` … |
| Usage: custom providers | `usage/custom.rs:9,26,52,301-373,493-496` | `/usr/bin/sandbox-exec`, executable bit check, `#!/bin/sh` example | No sandbox-exec; executable bit does not exist; example script is POSIX |
| Usage: system/codex | `usage/system.rs:44`, `usage/codex.rs:178` | `$HOME` to find `~/.codex`, `~/.claude` | Use the profile folder |
| Lanes: git lookup | `lanes/git.rs:148-164` | `/usr/bin/git` shim check, login-shell `command -v git`, `/usr/bin/xcode-select` | Find `git.exe` on `PATH` (and Git for Windows default folders) |
| Device tools | `device/tools.rs:4-8,32,65-67,90,129-138` | `$HOME` for SDK default, executable bit check, `xcode-select`, `/Applications/Xcode.app`, `simctl` | SDK default is `%LOCALAPPDATA%\Android\Sdk`; binaries end in `.exe`; Xcode paths skipped |
| Device: iOS | `device/ios.rs` (whole file) | `xcrun simctl` | macOS-only; gate behind `cfg(target_os = "macos")` and hide in the UI |
| Device: process | `device/process.rs:113-117` | `libc::kill(pid, 0)` liveness check | Use `OpenProcess` + `GetExitCodeProcess` |
| Notes watcher | `notes/watcher.rs` | `notify` crate (FSEvents on macOS) | `notify` uses `ReadDirectoryChangesW` on Windows; no code change expected |
| Browser URL | `messaging/commands.rs:1089`, `browser.rs` | none (URL normalizing) | Check `file://` handling with drive letters |
| Example | `examples/emulator_probe.rs:19` | `$HOME` | Example only; fix or gate |

### Tauri shell (`src-tauri`)

| Area | Location | Dependency | Windows impact |
|------|----------|------------|----------------|
| Browser snapshot | `browser/snapshot.rs:16,53,66,85` | `WKWebView takeSnapshot` via `objc2`; non-macOS branch returns an error | Needs WebView2 `CapturePreview` (PNG) |
| Browser data stores | `browser/mod.rs`, `Cargo.toml:29-34` | `WKWebsiteDataStore` per workspace (macOS deps only) | WebView2 uses a per-webview user data folder / profile; check per-workspace login sharing |
| Open in default browser | `browser/mod.rs:89` | `/usr/bin/open` | Windows default handler |
| Menu | `menu.rs:30,66,93` | `Cmd+,`, `Cmd+Ctrl+S` accelerators; macOS app menu layout | Use `CmdOrCtrl`; Windows has no app menu (menus attach to the window) |
| Bundle config | `tauri.conf.json` `bundle.targets` `["app","dmg"]`, `macOS` section, `icon.icns` | macOS bundles only | Add `nsis`/`msi` target (per-platform config), `.ico` icon, WebView2 install mode |
| Single instance | `tauri-plugin-single-instance` | cross-platform | No change expected |

### Frontend (`ui/src`)

| Location | Dependency | Windows impact |
|----------|------------|----------------|
| `terminal/xtermKeys.ts:7-21` | Cmd+Backspace/Left/Right line editing via `metaKey` | Windows has no Cmd; map to Ctrl variants that PowerShell understands or skip |
| `terminal/xtermSession.ts:48` | `macOptionIsMeta` | No effect on Windows; Ctrl+C copy-on-selection and Ctrl+V paste needed |
| `App.tsx:89,96,129`, `state/canvas.ts:80,321` | Cmd+W, Ctrl+Cmd+S, Cmd+Q comments and handlers | Ctrl+W, Ctrl+Shift+S (or similar), Alt+F4 |
| `sidebar/Sidebar.tsx:90`, `App.tsx:189` | Tooltip text `⌃⌘S` | Platform-specific labels |
| `canvas/Canvas.tsx:132` | `metaKey \|\| ctrlKey` | Already OS-neutral |
| `nodes/DeviceNode.tsx:43,116` | iOS branch | Hide iOS on Windows |
| `nodes/devicePointer.ts:81` | `metaKey \|\| ctrlKey` | Already OS-neutral |

### Scripts and CI

| Location | Dependency | Windows impact |
|----------|------------|----------------|
| `scripts/build-cli.sh` | POSIX `sh`, `lipo`, `chmod 755`, output name `sonata` | Tauri `beforeBuildCommand` on Windows runs under `cmd`/PowerShell; needs a Windows path (Git Bash `sh` exists on runners) and `sonata.exe` |
| `scripts/install.sh` | `/Applications`, `.app` bundle | macOS-only build-from-source; Windows gets a separate note or script |
| `scripts/check.sh`, `bench-terminals.sh`, `capture-pty.sh` | POSIX `sh` | Developer tools on macOS; leave as is |
| `scripts/release/latest-json.mjs:18`, `.d.mts:13` | Manifest platforms only `darwin-aarch64`, `darwin-x86_64` | Add `windows-x86_64`; keep darwin keys unchanged |
| `.github/workflows/release.yml:23-76` | `macos-14` runner, `universal-apple-darwin` | Add a `windows-latest` build job; merge manifests after both succeed |
| `.github/workflows/ci.yml` | `ubuntu-latest` only | Add a `windows-latest` job for core + CLI tests |

### Tests (`crates/core/tests`) that assume Unix

`common/mod.rs`, `common/fake_bin.rs` (shell-script fake binaries with exec bit),
`socket.rs`, `permissions.rs`, `terminal_pty.rs`, `terminal_attach.rs`, `zsh_helpers.rs`,
`agent_hooks.rs`, `presets.rs`, `scale.rs`, `lanes_git.rs`, `lanes_guard.rs`, `lanes_cli.rs`,
`cli_ask.rs`, `cli_browser.rs`, `usage_*` (sandbox, runner, detect, custom consent,
no-secrets), `device_tools.rs`, `device_ios.rs`. Each needs either a Windows variant (fake
binaries as `.cmd`/`.exe`, PowerShell instead of `/bin/sh`) or a `cfg(unix)` gate with a
Windows counterpart test.

## Decisions

Each decision lists what was chosen, why, and what else was considered. Windows code lives in
`#[cfg(windows)]` items or `windows.rs` files next to the Unix code they replace, so the Linux
port (011) and this branch touch the same call sites only in small, mechanical ways.

### R1. Platform split in `core`

- **Decision**: A new `crates/core/src/platform/` module with `mod.rs` (one small cross-platform
  API), `unix.rs` (code moved from `paths.rs`, `terminal/pty.rs`, `terminal/busy.rs`,
  `device/process.rs`, `usage/runner.rs`), and `windows.rs`. Call sites use
  `platform::home_dir()`, `platform::make_private(path, kind)`, `platform::is_private(path,
  kind)`, `platform::open_default(target)`, `platform::is_executable(path)`,
  `platform::process_alive(pid)`. Larger Windows pieces get their own files:
  `messaging/pipe.rs`, `terminal/shells.rs`, `terminal/powershell.rs`, `terminal/bash.rs`.
- **Rationale**: Principle II keeps all logic in `core`; one module per platform keeps `cfg`
  out of business code. Unix code moves unchanged, so macOS behavior does not change.
- **Alternatives**: `cfg` blocks inline at every call site (harder to rebase against 011); a
  separate `sonata-platform` crate (new crate for a few hundred lines, rejected by VII).

### R2. Windows API bindings

- **Decision**: `windows-sys` 0.60 (already in `Cargo.lock` through other crates) as a
  `cfg(windows)` dependency of `core` and `cli`, with only the features needed
  (`Win32_Foundation`, `Win32_Security`, `Win32_Security_Authorization`, `Win32_Storage_FileSystem`,
  `Win32_System_Pipes`, `Win32_System_Threading`, `Win32_System_JobObjects`,
  `Win32_System_Diagnostics_ToolHelp`, `Win32_System_Console`, `Win32_UI_Shell`,
  `Win32_System_IO`). In `src-tauri`, `webview2-com` 0.39 and `windows` 0.62 (both already
  pulled in by wry) for the browser snapshot only.
- **Rationale**: no new code enters the build that is not already compiled for Windows; raw
  bindings are enough for the few calls needed.
- **Alternatives**: `interprocess` (named pipes, but no control over the security descriptor in
  its sync API), `windows-acl`/`windows-permissions` (unmaintained), `dunce`, `which` (small
  helpers we write in a few lines).

### R3. Command channel: named pipe

- **Decision**: On Windows the server creates `\\.\pipe\sonata-<16 hex of hash(user SID, data
  root)>` with `PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE`, `PIPE_TYPE_BYTE |
  PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS`, and a security descriptor from
  the SDDL `D:P(A;;GA;;;<user SID>)(A;;GA;;;SY)`. A thread loops: create instance →
  `ConnectNamedPipe` → hand the handle (as `std::fs::File`) to the existing per-request handler.
  `SONATA_SOCKET` carries the pipe name, so the CLI and the env contract keep their names. The
  CLI opens the pipe with `OpenOptions::new().read(true).write(true)`, retries on
  `ERROR_PIPE_BUSY` with `WaitNamedPipeW` (5 s), and checks with
  `GetNamedPipeServerProcessId` + the process token that the server runs as the same user. The
  protocol (newline-delimited JSON, `token` per request) is unchanged.
- **Rationale**: closest Windows counterpart to a `0600` Unix socket: per-user access list, no
  network, remote clients rejected; the first-instance flag and the server-owner check stop a
  squatting process from impersonating the app.
- **Alternatives**: AF_UNIX on Windows (std has no support; crates are old; folder ACL is the
  only protection); TCP on `127.0.0.1` with token (any local user can connect and try tokens).

### R4. Private files: access lists instead of modes

- **Decision**: `make_private` sets a protected DACL with `SetFileSecurityW`:
  folders `D:P(A;OICI;FA;;;<user SID>)(A;OICI;FA;;;SY)`, files `D:P(A;;FA;;;<user SID>)
  (A;;FA;;;SY)`. `is_private` reads the DACL and passes when it is protected and every allow
  entry names the user or SYSTEM. `enforce_private_tree` keeps its walk and lane exclusion and
  uses these two calls on Windows. Log files and the pipe use the same rule. The user SID comes
  from the process token (`TokenUser`, `ConvertSidToStringSidW`).
- **Rationale**: matches FR-016 ("current user and the system"); inheritable entries mean new
  files inside the root are private without extra calls.
- **Alternatives**: rely on the profile folder's default ACL (Administrators and the user only,
  but not guaranteed and not checked); `icacls` process calls (slow, locale-dependent output).

### R5. Home and data root

- **Decision**: `platform::home_dir()` = `std::env::home_dir()` (on Unix it reads `HOME`, as
  today; on Windows it returns the profile folder, `%USERPROFILE%`). Every `var_os("HOME")` in
  `core` (data root, PTY fallback cwd, usage `~/.codex`/`~/.claude`, Android SDK default, the
  probe example) uses it. `SONATA_HOME` keeps priority. Tests on Windows set `SONATA_HOME` (or
  pass explicit roots), never `HOME`.
- **Rationale**: one call, same result on macOS, correct on Windows.

### R6. Paths: verbatim prefixes, case, long paths

- **Decision**: `platform::simplify(path)` strips the `\\?\` (and `\\?\UNC\`) prefix that
  `fs::canonicalize` returns on Windows when the result is a plain drive or UNC path under 260
  characters (keeps it otherwise); used before a path is shown, stored, sent to git, or used as
  a cwd. Prefix checks (lane guard, workspace deletion) compare canonicalized paths, which on
  Windows have the on-disk case, so they stay exact. Git calls on Windows add
  `-c core.longpaths=true`. `PATH` is joined with `std::env::join_paths` (`;` on Windows).
  Stored paths are written as the OS gives them; a path from another OS that does not exist is
  reported as missing (lanes already show "folder missing").
- **Rationale**: Rust's std already handles long paths for file calls; the risk is the verbatim
  prefix reaching git or the UI.
- **Alternatives**: `dunce` crate (same logic, extra dependency).

### R7. Shells on Windows

- **Decision**: `terminal/shells.rs` discovers, in order: PowerShell 7 (`pwsh.exe` on `PATH`,
  then `%ProgramFiles%\PowerShell\7\pwsh.exe`), Windows PowerShell
  (`%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe`), Command Prompt
  (`%ComSpec%`), Git Bash (`bin\bash.exe` beside the `git.exe` found on `PATH`, then
  `%ProgramFiles%\Git\bin\bash.exe`, `%LOCALAPPDATA%\Programs\Git\bin\bash.exe`), and WSL
  distributions (`%SystemRoot%\System32\wsl.exe --list --quiet`, UTF-16 output). The default is
  the first PowerShell found. A terminal stores its choice in a new optional
  `TerminalContent.shell` (see data-model.md); missing means the default shell. On macOS the
  field is ignored and the login shell stays as today.
- **Rationale**: FR-010 and the clarification (per-terminal choice; pwsh first).
- **Alternatives**: a global default-shell setting only (clarification asked for per
  terminal); `where.exe` process calls (slower than a `PATH` scan).

### R8. Shell integration

- **Decision**:
  - PowerShell: start `pwsh.exe -NoLogo -NoExit -EncodedCommand <base64 UTF-16LE>` where the
    command defines `function claude { sonata run claude @args }` and the same for `codex`,
    guarded by `Get-Command sonata`. The profile still loads (no `-NoProfile`); `-Command`
    text is not a script file, so the execution policy does not block it and is not changed.
    A launch command runs after the helpers.
  - Git Bash: `bash.exe --rcfile <root>\run\bash\sonata.bashrc -i`; the file sources
    `/etc/profile`, then the user's `~/.bash_profile` (or `~/.bashrc`), then defines the two
    functions. The file content is shared with the Linux bash integration if 011 adds one
    (same function bodies).
  - WSL (best effort): `wsl.exe -d <distro> --cd <path>`, with `WSLENV` listing
    `SONATA_SOCKET/u:SONATA_TOKEN/u:SONATA_TERMINAL_ID/u` so `sonata.exe` called from inside
    WSL reaches the pipe; no hand-typed agent helpers.
  - Command Prompt: no helpers; presets still start agents through `sonata run`.
- **Rationale**: FR-012; never edits the user's profile or policy.

### R9. `sonata run` without `exec`

- **Decision**: On Windows `run.rs` spawns the agent with inherited stdio, ignores Ctrl+C in
  its own process (`SetConsoleCtrlHandler(None, TRUE)`; the child shares the console and still
  receives it), waits, and exits with the child's code. Agent lookup (`find_real`) tries each
  `PATHEXT` extension (`claude.exe`, `claude.cmd`, …); Rust's `Command` runs `.cmd` files with
  safe argument quoting. The end-of-turn hook command uses the CLI path with forward slashes in
  quotes (`"C:/…/sonata.exe" _hook …`), because Claude Code runs hook commands through Git Bash
  on Windows.
- **Rationale**: no `exec` on Windows; this keeps exit codes and Ctrl+C behavior.

### R10. Busy detection

- **Decision**: On Windows a terminal is busy when the shell process has a live child process
  other than console hosts (`conhost.exe`, `OpenConsole.exe`), found with
  `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`. The Unix code (`tcgetpgrp`) moves unchanged.
- **Rationale**: ConPTY has no foreground process group; a child of the shell is the closest
  signal and matches how agents and commands run.

### R11. Usage runner and environment

- **Decision**: On Windows the runner puts each command in a Job Object with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; timeout kills the job. The env allowlist on Windows is
  `USERPROFILE, USERNAME, APPDATA, LOCALAPPDATA, TEMP, TMP, PATH, PATHEXT, SystemRoot, ComSpec,
  HOMEDRIVE, HOMEPATH`. `detect.rs` skips the login-shell probe on Windows and reads the app's
  own environment (Start menu launches already get the user environment). Custom providers are
  disabled on Windows: `read_custom` returns "Custom usage providers are not available on
  Windows: there is no sandbox for them yet." and Settings shows that text; the provider is
  never started (FR-025).
- **Alternatives**: AppContainer or restricted tokens for custom providers (large, hard to test
  in CI; deferred).

### R12. Opening URLs and files

- **Decision**: `platform::open_default(target)` = `/usr/bin/open` on macOS (unchanged) and
  `ShellExecuteW(…, "open", target, …)` on Windows. Callers in `core` (`app.rs`) and the shell
  (`browser/mod.rs`) use it. Only `http`, `https`, `mailto`, and existing file paths are passed,
  as today.
- **Alternatives**: `cmd /c start` (quoting and injection risk), `explorer.exe` (odd for URLs).

### R13. Browser node on WebView2

- **Decision**: Per-workspace logins use `WebviewBuilder::data_directory(<root>\browser\<ws>)`
  on Windows instead of `data_store_identifier` (macOS). Removing a workspace's store deletes
  that folder after its webviews close. The page image uses
  `ICoreWebView2::CapturePreview(PNG)` into an in-memory `IStream` (`CreateStreamOnHGlobal`),
  reached through `Webview::with_webview(|w| w.controller())`. If capture fails, the node keeps
  its last image and `sonata browser screenshot` returns "Page images are not available on this
  system." (FR-022).
- **Alternatives**: a screen grab of the window region (captures overlapping UI, needs the node
  on screen).

### R14. Android tools on Windows

- **Decision**: SDK default `%LOCALAPPDATA%\Android\Sdk` (then `ANDROID_HOME`,
  `ANDROID_SDK_ROOT` as today); binaries `emulator.exe`, `adb.exe`. `is_executable` on Windows
  = file exists with an extension in `PATHEXT`. `process_alive` uses `OpenProcess` +
  `GetExitCodeProcess == STILL_ACTIVE`. `device/ios.rs` and the Xcode lookup compile only on
  macOS; the device list on other systems contains no iOS entries and the UI hides the iOS
  section when the list says iOS is unsupported.

### R15. Keyboard and labels

- **Decision**: A UI helper `platform.ts` exposes `isMac` (from `navigator.userAgent`) and
  `keyLabel(…)`. App shortcuts use `mod = isMac ? metaKey : ctrlKey`. On Windows: Ctrl+W closes
  the selected item unless a terminal has focus (then the terminal gets it; Ctrl+Shift+W closes
  the item); Ctrl+, opens Settings; Ctrl+Shift+S toggles the sidebar; quit is Alt+F4 and the
  window close button. In terminals on Windows: Ctrl+C copies when text is selected, else goes
  to the shell; Ctrl+V and Ctrl+Shift+V paste; Ctrl+Shift+C copies. Cmd line-editing mappings
  stay macOS-only. Menu accelerators are set per platform in `menu.rs` (`CmdOrCtrl` where safe,
  `Ctrl+Shift+…` where a terminal key clashes). Tooltips and help text use `keyLabel`.
- **Rationale**: FR-028 and the clarification.

### R16. Packaging, installer, updater

- **Decision**: `src-tauri/tauri.windows.conf.json` (merged by Tauri on Windows builds only)
  sets `bundle.targets: ["nsis"]`, `bundle.icon` with a new `icons/icon.ico`,
  `bundle.windows.webviewInstallMode: { "type": "downloadBootstrapper" }`,
  `bundle.windows.nsis.installMode: "currentUser"`, `bundle.windows.nsis.installerHooks:
  "windows/installer-hooks.nsh"`, and `plugins.updater.windows.installMode: "passive"`. The
  hooks add `$INSTDIR\bin` to the user `Path` on install and remove it on uninstall through a
  short PowerShell call (`[Environment]::SetEnvironmentVariable(..., 'User')`, which also
  broadcasts the change). `scripts/build-cli.sh` writes `sonata.exe` when the target triple
  contains `windows`. The updater artifact is the NSIS `-setup.exe` with its `.sig`.
- **Alternatives**: MSI (needs administrator rights; out of scope); editing `Path` in pure NSIS
  (string-length truncation risk).

### R17. Release and CI

- **Decision**: `release.yml` gets a `build-windows` job on `windows-latest` that builds
  `--bundles nsis` for `x86_64-pc-windows-msvc`, signs updater files with the same secrets, and
  uploads the installer and `.sig` as job artifacts. The publish step waits for both build jobs,
  and `latest-json.mjs` gains an optional Windows entry: `platforms` keeps `darwin-aarch64` and
  `darwin-x86_64` exactly as today and adds `windows-x86_64` only when the Windows files exist.
  `ci.yml` gets a `windows` job: `cargo clippy` and `cargo test` for `sonata-core` and
  `sonata-cli`, plus `cargo check -p sonata-app` (the Tauri crate) to catch shell-only compile
  errors. The repo is public, so Windows runner minutes cost nothing.
- **Rationale**: FR-004, FR-005, FR-008; macOS consumers read the same keys.

### R18. Tests on Windows

- **Decision**: Tests that need POSIX shells, fake shell-script binaries, Unix sockets, or
  modes get `#![cfg(unix)]` (file level) or `#[cfg(unix)]`. New Windows tests:
  `platform_windows.rs` (ACL private/not private, home dir, simplify), `pipe.rs` (round trip,
  wrong-user refusal by DACL inspection, busy retry), `shells_windows.rs` (discovery with fake
  folders), `terminal_pwsh.rs` (spawn PowerShell in ConPTY, echo, busy detection with a child
  `ping`), `cli_ask_windows.rs` (two fake terminals talk through the pipe), `run_windows.rs`
  (agent lookup with `PATHEXT`, exit code passthrough). Fake agent binaries on Windows are
  `.cmd` files. Integration tests that touch git keep using temporary repositories.

### R19. Constitution amendment

- **Decision**: Amend to 1.2.0 (MINOR): Principle VII targets macOS and Windows (x86_64);
  Principle V and the Technology section allow a named pipe with a user-only access list as the
  Windows counterpart of the `0600` Unix socket; Principle VI adds the unsigned per-user Windows
  installer and the SmartScreen step. The Linux branch amends the same lines; whichever merges
  second rebases and merges the wording (one list of platforms).

### R20. Session restore and Linux groundwork

- **Decision**: Session restore (009) is not on `main` yet. Its Windows needs (terminal
  history files, agent resume commands typed into PowerShell) are covered by R5–R9; this branch
  verifies FR-027 after it rebases on a `main` that contains 009 and 011. Until then, tasks for
  FR-027 stay open.

## R21. Rebase on 009, 010, and 011 (2026-10-29)

Linux (011) landed first with its own `platform` module, bash helpers, menu, and release flow.
The rebase keeps one design per concern and adds Windows to it:

- **platform**: 011's per-OS API (`open_url_command`, `reveal_command`, git hints, Android
  folders, `IOS_SIMULATORS`, AppImage environment) gains `platform/windows.rs` next to
  `linux.rs` and `macos.rs`. The helpers from R1 (privacy, home, process checks, `PATHEXT`
  lookup) live in `mod.rs` with bodies in `unix.rs` (shared by Linux and macOS) and
  `windows.rs`. `ShellExecuteW` (R12) is replaced by 011's `Launch`: `rundll32
  url.dll,FileProtocolHandler <url>` and `explorer /select,<path>`.
- **Git Bash** reuses 011's `terminal/bash.rs` rc file (`<run>/bash/bashrc`); the Windows-only
  `bash.rs` of this branch is gone (R8).
- **Menu and keys**: Windows uses the Linux menu (Ctrl+Shift for Close Item, Toggle Sidebar,
  Quit; no Edit menu) and the Linux terminal clipboard keys, plus two Windows habits behind
  `isWindows`: plain Ctrl+W outside terminals closes the item, and Ctrl+C with a selection
  copies, Ctrl+V pastes (R15).
- **Browser data**: 011's per-workspace data folder (`browser-data/<ws>`) serves Windows too;
  Windows retries its removal while WebView2 lets go (R13).
- **iOS**: 011 reports `ios: null` where simulators cannot exist; the Windows `supported` flag is
  gone (R14).
- **Release**: 011's draft → platform jobs → publish flow gains a `windows` job; the dry-run
  input of R17 is gone (the flow creates a draft). CI's `windows` job builds the installer on
  every push instead and uploads it as `windows-installer` (R17).
- **Kimi (010)**: its hooks are POSIX shell commands; on Windows the integration reports
  "not available on Windows yet" and changes nothing. `sonata run kimi` itself works.
- **Session restore (009)**: `sonata run` resumes with the same logic; on Windows the wrapper
  runs the agent as a child (no `exec`) and ignores Ctrl+C through a console handler.
- **Constitution**: 011 took 1.2.0; Windows is 1.3.0 and lists macOS, Linux, and Windows.
