# Contract: Per-OS Behavior

| Area | macOS (unchanged) | Linux |
|---|---|---|
| Default shell fallback | `/bin/zsh` | `/bin/bash` |
| Shell start | `<shell> -l`; zsh with `ZDOTDIR` helpers | bash: `bash --rcfile ~/.sonata/run/bash/bashrc -i`; zsh: as macOS; others: `-l` |
| Open URL | `/usr/bin/open <url>` | `xdg-open <url>` |
| Reveal file | `/usr/bin/open -R <path>` | `xdg-open <parent folder>` |
| Reveal label | "Reveal in Finder" | "Show in File Manager" |
| Git missing | "Git is not installed. Install the Command Line Tools with: xcode-select --install" | "Git is not installed. Install it with your package manager, for example: sudo apt install git" |
| Git candidates | PATH, `/opt/homebrew/bin/git`, `/usr/local/bin/git` | PATH, `/usr/bin/git`, `/usr/local/bin/git` |
| Custom provider sandbox | `sandbox-exec` profile | `bwrap` (see below) |
| Android SDK defaults | `~/Library/Android/sdk`, Homebrew | `~/Android/Sdk`, `/opt/android-sdk`, `/usr/lib/android-sdk` |
| Emulator discovery | `~/Library/Caches/TemporaryItems/avd/running` | `$XDG_RUNTIME_DIR/avd/running`, `/tmp/android-$USER/avd/running` |
| iOS | shown | `ToolReport.ios = null`, section hidden |
| Browser data | `data_store_identifier(ws)` | `data_directory(~/.sonata/browser-data/<ws>)` |
| Browser snapshot | WKWebView snapshot | WebKitGTK snapshot → PNG |
| Child environment | inherited | AppImage variables cleaned (data-model `EnvFix`) |
| CLI | bundled `bin` on PATH | bundled `bin` on PATH + copy at `~/.local/bin/sonata` |

## Shortcuts

| Action | macOS | Linux |
|---|---|---|
| Settings | Cmd+, | Ctrl+, |
| Close Item | Cmd+W | Ctrl+Shift+W |
| Toggle Sidebar | Ctrl+Cmd+S (hint "⌃⌘S") | Ctrl+Shift+S |
| Quit | Cmd+Q | Ctrl+Shift+Q |
| Terminal copy / paste | Cmd+C / Cmd+V (Edit menu) | Ctrl+Shift+C / Ctrl+Shift+V |
| Terminal line editing | Cmd+Backspace/Left/Right | native Ctrl keys |

Linux menu binds no plain-Ctrl letter key and has no Edit menu, so Ctrl+A…Ctrl+Z reach the
terminal.

## bubblewrap arguments (Linux)

In order; entries marked "if exists" are added only when the path exists:

```text
/usr/bin/bwrap --die-with-parent --new-session --unshare-pid --unshare-ipc
  --dev-bind / / --proc /proc
  --tmpfs <root> --ro-bind <root>/usage-providers <root>/usage-providers   (if exists)
  --ro-bind /dev/null <home>/.claude/.credentials.json                     (if exists)
  --ro-bind /dev/null <home>/.codex/auth.json                              (if exists)
  --tmpfs <home>/.local/share/keyrings                                     (if exists)
  --tmpfs <home>/.local/share/kwalletd                                     (if exists)
  --tmpfs <runtime dir>                                                    (if set and exists)
  -- <argv…>
```

Messages: "Cannot run safely: bubblewrap (bwrap) is not installed. Install the bubblewrap
package." and "Cannot run safely: bubblewrap cannot start: <first stderr line>".

## Update blockers (Linux)

- Package or source build: "Sonata was installed with a package. Download the new version from
  https://github.com/DuarteVn/sonata/releases/latest"
- AppImage in a read-only folder: "Move the Sonata AppImage to a folder you can write to, such as
  ~/Applications, to update"
