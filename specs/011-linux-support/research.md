# Research: Linux Support

## Inventory of macOS-only code paths (2026-10-29)

Found with a search for `cfg(target_os)`, `sandbox-exec`, `simctl`, `objc2`, `/Applications`,
`xcode-select`, `Library/`, `/usr/bin/open`, zsh-only helpers, Cmd shortcuts, and macOS fonts.
Each row says what happens on Linux today and the area it belongs to.

| # | Location | macOS-only behavior | On Linux today |
|---|----------|---------------------|----------------|
| M1 | `src-tauri/src/browser/snapshot.rs` `capture` | WKWebView `takeSnapshot` through `objc2-web-kit` | `cfg(not(macos))` branch returns `false`: no card picture, snapshot command fails |
| M2 | `src-tauri/src/browser/snapshot.rs` `remove_data_store` | `WKWebsiteDataStore::removeDataStoreForIdentifier` | no-op: workspace browsing data is never removed |
| M3 | `src-tauri/src/browser/mod.rs` `ensure` | `WebviewBuilder::data_store_identifier(ws)` (macOS/iOS only in wry) | ignored: all workspaces share one browsing store |
| M4 | `src-tauri/src/browser/mod.rs` `open_externally` | `/usr/bin/open <url>` | binary missing: external links do nothing |
| M5 | `src-tauri/Cargo.toml` | `objc2*` dependencies under `cfg(target_os = "macos")` | already gated |
| M6 | `crates/core/src/app.rs` `reveal_in_finder` | `/usr/bin/open -R <path>` | fails |
| M7 | `crates/core/src/app.rs` help pages (`android`, `avd`, `xcode`) | `/usr/bin/open <url>` | fails |
| M8 | `crates/core/src/usage/custom.rs` | `/usr/bin/sandbox-exec` with an SBPL profile (`Library/Keychains` denies) | sandbox missing: provider shows "Cannot run safely: sandbox-exec not found." |
| M9 | `crates/core/src/lanes/git.rs` | `/usr/bin/git` shim check, `xcode-select -p`, Homebrew paths, message "Install the Command Line Tools with: xcode-select --install" | shim check already `cfg!(macos)`; message and Homebrew candidates are macOS-flavored |
| M10 | `crates/core/src/terminal/pty.rs` `default_shell` | fallback `/bin/zsh` | zsh is often absent on Linux; fallback should be `/bin/bash` or `/bin/sh` |
| M11 | `crates/core/src/terminal/zsh.rs`, `pty.rs` `zsh_dir`, `app.rs` | hand-typed agent helpers only for zsh (`ZDOTDIR`) | bash users (Linux default) get no helpers |
| M12 | `crates/core/src/app.rs:573` | login shell fallback `/bin/zsh` | same as M10 |
| M13 | `crates/core/src/device/tools.rs` | Android SDK default `~/Library/Android/sdk`; Homebrew prefixes; Xcode `developer_dir`, `/Applications/Xcode.app` | Linux default SDK is `~/Android/Sdk`; iOS lookup must be off |
| M14 | `crates/core/src/device/emulator.rs` | running-AVD discovery dir `~/Library/Caches/TemporaryItems/avd/running` | Linux uses `$XDG_RUNTIME_DIR/avd/running` (fallback `/tmp/android-$USER/avd/running`) |
| M15 | `crates/core/src/device/ios.rs` | `simctl`, `/usr/bin/plutil` | must be unreachable on Linux (UI hides iOS) |
| M16 | `src-tauri/src/commands/update.rs` `wrong_location` | `/Volumes/` and `AppTranslocation` checks | Linux needs AppImage (`$APPIMAGE`) vs package detection |
| M17 | `src-tauri/src/lib.rs` `cli_dir` | `resource_dir()/bin/sonata` inside `.app` | resource dir differs for AppImage and .deb (`/usr/lib/Sonata/...`); must verify |
| M18 | `src-tauri/tauri.conf.json` | bundle targets `app`, `dmg`; `macOS` block; resources `bin/` | needs `appimage`, `deb` targets for Linux builds (CLI override per platform) |
| M19 | `src-tauri/src/menu.rs` | accelerators `Cmd+,`, `Cmd+Ctrl+S`; app menu layout | Linux menus attach to the window; `Cmd` accelerators must become `Ctrl` |
| M20 | `ui/src/App.tsx`, `ui/src/state/canvas.ts`, `ui/src/terminal/xtermKeys.ts` | Cmd+W, Cmd+Q, Ctrl+Cmd+S, Cmd line-editing keys use `metaKey` | Ctrl collides with terminal control keys |
| M21 | `ui/src/App.tsx`, `ui/src/sidebar/Sidebar.tsx` | hints "⌃⌘S" | macOS glyphs on Linux |
| M22 | `ui/src/sidebar/LoadFailureItem.tsx` | "Reveal in Finder" | macOS name on Linux |
| M23 | `ui/src/styles.css`, `ui/src/terminal/xtermSession.ts` | `-apple-system`, `SF Mono`, `Menlo` | fall back to defaults; terminal font may not be monospace-tuned |
| M24 | `scripts/install.sh` | `xcode-select`, `/Applications` copy | macOS only |
| M25 | `scripts/build-cli.sh` | universal CLI when target is `universal-apple-darwin` | plain `cargo build` path works on Linux |
| M26 | `scripts/release/latest-json.mjs` (+ `.d.mts`) | manifest platforms `darwin-aarch64`, `darwin-x86_64` only | needs `linux-x86_64` |
| M27 | `.github/workflows/release.yml` | single `macos-14` job | needs a Linux build job and a merge step |
| M28 | `.github/workflows/ci.yml` | Linux job runs core, CLI, UI only (no `src-tauri`) | needs `src-tauri` build and clippy on Linux |
| M29 | `crates/core/tests/usage_sandbox.rs`, `zsh_helpers.rs`, `device_tools.rs`, `device_ios.rs` | tests assume macOS paths or skip without zsh | must stay green on Linux CI |
| M30 | `src-tauri/src/lib.rs:166` | Cmd+Q quit handling (`ExitRequested`) | Linux has no app menu Quit; window close path must reach the same check |

Portable today: data root `~/.sonata`, Unix socket IPC (`0600`), `portable-pty`, git through
process calls, Claude and Codex usage readers (they run the agent CLIs), Android adb and gRPC.

Decisions for each row follow in the plan phase.

## Decisions

### R1. Platform code layout

- **Decision**: New `crates/core/src/platform/` module with `mod.rs` (shared API), `macos.rs`,
  and `linux.rs`, selected with `#[cfg(target_os = …)]`. It owns: default shell, open URL,
  reveal file, git install hint, Android SDK defaults, emulator discovery folders, iOS support
  flag, custom-provider sandbox, and the child-process environment fix-up (R6). Shell-side
  (`src-tauri`) Linux code sits in `#[cfg(target_os = "linux")]` blocks next to the macOS ones in
  `browser/snapshot.rs`, `menu.rs`, `commands/update.rs`, and a new `cli_install.rs`.
- **Rationale**: One file per OS keeps the Windows branch (012) to adding `windows.rs` and its
  match arms, so both branches rebase without touching each other's lines. Logic stays in core
  (constitution II) and is unit-testable on any host because the pure parts take inputs.
- **Alternatives**: `cfg` checks scattered at each call site (hard to rebase, hard to review);
  a runtime `Platform` trait object (indirection with only one implementation per build).

### R2. Packages and the update manifest

- **Decision**: Linux bundles `appimage` and `deb`, configured in a new
  `src-tauri/tauri.linux.conf.json` that Tauri merges automatically on Linux (macOS config
  untouched). With `createUpdaterArtifacts: true`, the AppImage itself is the update file and
  gets `Sonata_<v>_amd64.AppImage.sig`. `latest.json` gains `platforms["linux-x86_64"]` pointing
  at the AppImage. The `.deb` installs `/usr/bin/sonata` through `bundle.linux.deb.files`.
- **Rationale**: `tauri-plugin-updater` 2.14 looks up `{os}-{arch}-{bundle}` then `{os}-{arch}`;
  for an AppImage it replaces the file at `$APPIMAGE`. A `.deb` install also matches
  `linux-x86_64` but would try `dpkg -i` with AppImage bytes, so the app checks its bundle type
  first (R3) and never installs on a `.deb`.
- **Alternatives**: a `linux-x86_64-deb` entry with pkexec install (system-wide root install from
  a GUI prompt; rejected for this feature per clarification); Flatpak/Snap (store accounts,
  sandbox conflicts with PTYs).

### R3. Update behavior per install type

- **Decision**: `commands/update.rs` replaces `wrong_location()` with a per-OS
  `install_blocker() -> Option<String>`. macOS keeps its current rule and text. Linux:
  `tauri::utils::platform::bundle_type()` is `Deb` (or none, a source build) → "Sonata was
  installed with a package. Download the new version from the release page:
  https://github.com/DuarteVn/sonata/releases/latest"; AppImage whose folder is not writable →
  "Move the Sonata AppImage to a folder you can write to, such as ~/Applications, to update".
  The check and the banner work everywhere.
- **Rationale**: reuses the existing `Blocked` banner state; no new IPC type.
- **Alternatives**: a new `Manual` state with a link button (more UI for little gain).

### R4. Release workflow shape

- **Decision**: `release.yml` becomes: `checks` (reuses `ci.yml`) → `draft` (creates the draft
  release with generated notes) → `macos` and `linux` build jobs in parallel (each uploads its
  files to the draft) → `publish` (needs all builds; downloads the `.sig` files, runs
  `latest-json.mjs`, uploads `latest.json`, marks the release latest). The macOS build steps are
  unchanged except that they upload instead of creating the release. Linux builds on
  `ubuntu-22.04`. `latest-json.mjs` keeps its macOS flags and adds `--linux-sig-file` and
  `--linux-url`; when given, the `linux-x86_64` entry is added.
- **Rationale**: a failed platform job leaves only a draft; the previous release stays "latest",
  so existing `latest.json` consumers see no change (FR-004, FR-005). The Windows branch adds one
  build job and one pair of flags.
- **Alternatives**: a matrix job (macOS steps differ too much); `tauri-action` (third-party code
  with a write token, rejected in 005 R5).

### R5. glibc and system library baseline

- **Decision**: build on `ubuntu-22.04` (glibc 2.35, WebKitGTK 4.1). Build packages:
  `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf file build-essential`.
  `.deb` depends on `libwebkit2gtk-4.1-0` and `libgtk-3-0` (Tauri's default) plus `git` as a
  recommendation only.
- **Rationale**: binaries built against older glibc run on newer distributions; Tauri 2 needs
  WebKitGTK 4.1.

### R6. Child-process environment inside an AppImage

- **Decision**: when `APPIMAGE` is set, every program Sonata starts for the user (PTY shells,
  preset detection, usage readers, git, adb/emulator, `xdg-open`) gets a cleaned environment:
  `APPIMAGE`, `APPDIR`, `ARGV0`, `OWD` removed, and in every variable whose value is a
  colon-separated list, entries under `$APPDIR` are dropped (the variable is removed when nothing
  is left). A pure function `platform::host_env(vars) -> EnvFix` computes it; spawn sites apply it.
- **Rationale**: the AppImage runtime prepends its own GTK, GIO, and library paths
  (`LD_LIBRARY_PATH`, `XDG_DATA_DIRS`, `GTK_PATH`, `GIO_MODULE_DIR`, …). Inherited by terminals,
  they break system programs and GUI apps started from a Sonata terminal. The app itself still
  needs them, so they are only removed for children.
- **Alternatives**: clearing the app's own environment after GTK starts (WebKit's helper
  processes still need it).

### R7. CLI availability

- **Decision**: keep prepending the bundled `bin` folder to `PATH` in Sonata terminals (all OSes).
  On Linux the app also copies the bundled CLI to `~/.local/bin/sonata` at start when it is
  missing or differs, but only overwrites a file that is a Sonata CLI (contains the marker string
  `SONATA_TERMINAL_ID`); written atomically with mode 0755. Failures only log.
- **Rationale**: the AppImage mount path (`/tmp/.mount_*`) changes each launch; a stable path lets
  scripts outside Sonata call it. The marker check prevents clobbering an unrelated program.
- **Alternatives**: symlink into the mount (dangles when the app quits); `/usr/local/bin`
  (needs root).

### R8. Shell integration for bash

- **Decision**: for bash, Sonata starts `bash --rcfile <data>/run/bash/bashrc -i` instead of
  `bash -l`. The rcfile reproduces login startup — sources `/etc/profile`, then the first readable
  of `~/.bash_profile`, `~/.bash_login`, `~/.profile` — then defines the `claude` and `codex`
  helpers exactly as the zsh helpers do. User files are never changed. zsh keeps the current
  `ZDOTDIR` method. Other shells keep `-l` with no helpers. Default shell fallback on Linux is
  `/bin/bash` (macOS keeps `/bin/zsh`).
- **Rationale**: bash ignores `--rcfile` in login mode, and a login bash does not read
  `~/.bashrc` unless the profile sources it (Ubuntu's does). Emulating login startup gives the
  same environment as `bash -l` plus the helpers.
- **Alternatives**: `PROMPT_COMMAND` injection through the environment (fragile, visible to
  users); editing `~/.bashrc` (forbidden: user files must not change).

### R9. Custom usage provider sandbox (bubblewrap)

- **Decision**: on Linux the sandbox is `/usr/bin/bwrap` with: `--dev-bind / /` (same "allow
  default" stance as macOS), `--tmpfs <sonata root>` then `--ro-bind <root>/usage-providers` back,
  `--ro-bind /dev/null` over `~/.claude/.credentials.json` and `~/.codex/auth.json` when they
  exist, `--tmpfs` over `~/.local/share/keyrings`, `~/.local/share/kwalletd`, and
  `$XDG_RUNTIME_DIR` (hides the D-Bus secret service and keyring sockets) when they exist,
  `--unshare-pid --unshare-ipc --proc /proc --die-with-parent --new-session`. Before the first
  run, a probe (`bwrap --ro-bind / / /bin/true`) checks that it can start; a failure (missing
  binary, user namespaces blocked by the distribution) gives "Cannot run safely: bubblewrap
  (bwrap) is not installed. Install the bubblewrap package." or "Cannot run safely: bubblewrap
  cannot start: <first stderr line>". Nothing runs unsandboxed. The macOS `sandbox-exec` path is
  unchanged.
- **Rationale**: closest equivalent to the macOS profile (deny credential stores and Sonata data,
  allow the providers folder) without root. The argument list is built by a pure function, so it
  is unit-tested on any host; the real sandbox is integration-tested on the Linux runner.
- **Alternatives**: firejail (setuid, larger attack surface); landlock (kernel 5.13+, needs new
  dependency and cannot hide files, only deny); disabling providers on Linux (loses a feature).

### R10. Browser node on Linux

- **Decision**: per-workspace data uses `WebviewBuilder::data_directory(<data>/browser-data/<ws>)`
  on Linux (`data_store_identifier` stays on macOS). Deleting a workspace removes that folder
  after its webviews close (path checked to be under the data root). Snapshots use WebKitGTK
  `WebViewExt::snapshot(SnapshotRegion::Visible, SnapshotOptions::NONE, …)` on the
  `webkit2gtk::WebView` from `with_webview`, converted to PNG with `cairo::ImageSurface` and
  written atomically. On error the callback reports failure, so the card keeps its placeholder
  and `sonata browser screenshot` returns its existing error. New Linux-only dependencies:
  `webkit2gtk = "2.0"` and `cairo-rs = { version = "0.18", features = ["png"] }` — both already
  in `Cargo.lock` through wry, so no new crates are compiled.
- **Rationale**: same user-visible behavior as macOS with the APIs WebKitGTK exposes.
- **Alternatives**: card placeholder only (allowed fallback; kept if the snapshot API misbehaves
  on the runner).

### R11. Shortcuts and menus on Linux

- **Decision**: GTK menu accelerators fire before the webview sees a key, so the Linux menu must
  not bind any terminal control key. Linux menu: Sonata (Settings… `Ctrl+,`, Check for Updates…,
  Check for Updates Automatically, Quit `Ctrl+Shift+Q`), File (Close Item `Ctrl+Shift+W`), View
  (Toggle Sidebar `Ctrl+Shift+S`). No Edit menu on Linux (its predefined items would bind
  Ctrl+C/V/X/Z/A); text fields keep native Ctrl+C/V; terminals get `Ctrl+Shift+C` copy and
  `Ctrl+Shift+V` paste in `xtermKeys`. macOS menus and Cmd line-editing keys stay as they are.
  UI hints come from one `ui/src/platform.ts` (`isMac` from the user agent; `shortcut(name)`
  returns "⌃⌘S" or "Ctrl+Shift+S"). "Reveal in Finder" reads "Show in File Manager" on Linux.
- **Rationale**: FR-022/FR-023; follows common Linux terminal conventions.

### R12. Device node

- **Decision**: `ToolReport.ios` becomes `null` on Linux; the UI hides the iOS section when it is
  null. Android SDK candidates on Linux: preference, `ANDROID_HOME`, `ANDROID_SDK_ROOT`,
  `~/Android/Sdk` (Android Studio default), then `/opt/android-sdk` and `/usr/lib/android-sdk`.
  Emulator discovery on Linux: `$XDG_RUNTIME_DIR/avd/running`, `/tmp/android-$USER/avd/running`,
  then the existing env and `~/.android` folders.
- **Rationale**: the emulator writes its discovery files under the runtime folder on Linux.

### R13. Opening links and revealing files

- **Decision**: Linux uses `xdg-open <url>`; reveal opens the containing folder with
  `xdg-open <parent>` (same data-root check as today). Both use the cleaned environment (R6).
- **Alternatives**: D-Bus `org.freedesktop.FileManager1.ShowItems` (selects the file, but needs a
  D-Bus client dependency).

### R14. Install from source

- **Decision**: `scripts/install.sh` branches on `uname -s`. Linux: checks `cargo`, `npm`, and
  `pkg-config --exists webkit2gtk-4.1` (prints the `apt`/`dnf`/`pacman` command for the detected
  family from `/etc/os-release`), builds `--bundles appimage` without updater artifacts, installs
  `~/.local/bin/sonata-app` (the AppImage), `~/.local/bin/sonata`, a desktop entry in
  `~/.local/share/applications/sonata.desktop`, and the icon in
  `~/.local/share/icons/hicolor/128x128/apps/sonata.png`. macOS path unchanged.

### R15. Verification on GitHub Actions

- **Decision**: `ci.yml` gains a `linux-app` job on `ubuntu-22.04`: installs the build packages
  plus `bubblewrap zsh xvfb`, runs `cargo clippy --workspace --all-targets -D warnings` and
  `cargo test --workspace` (Linux-only tests run here), builds the AppImage and `.deb` without
  updater artifacts, then a smoke script (`scripts/release/tests/linux-smoke.sh`) starts the
  AppImage under `xvfb-run` with a temporary `HOME`, waits for the IPC socket, checks
  `~/.local/bin/sonata` and the `.deb` contents, takes a screenshot, and uploads it as an
  artifact. The existing `core-and-ui` job stays.
- **Rationale**: this Mac cannot run Linux; CI logs and the screenshot are the evidence.

### R16. Constitution

- **Decision**: MINOR amendment 1.2.0: principle VII names macOS and Linux (x86_64) as targets;
  principle VI adds the Linux install paths (AppImage self-updating, `.deb` manual) and states
  that macOS keeps ad-hoc signing and universal builds; Technology & Platform Constraints add
  "Linux: Ubuntu 22.04+ or equivalent with WebKitGTK 4.1". The Windows branch amends the same
  lines later; it rebases on this.
