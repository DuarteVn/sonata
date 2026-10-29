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
