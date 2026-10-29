# Sonata

An open source app for macOS and Linux: an infinite canvas where AI-agent terminals, notes, and the links
between them live side by side, organized into as many workspaces as you need.

> **Status:** pre-alpha, under active development. Work follows
> [Spec Kit](https://github.com/github/spec-kit): see `.specify/memory/constitution.md` for project
> principles and `specs/` for features.

## What it does today

- As many workspaces as you want, each tied to a project folder, in a sidebar you can pin,
  rename, reorder, and delete from.
- An infinite canvas per workspace with real terminals (your login shell, optionally starting a
  command such as `claude` or `codex`) and Markdown sticky notes.
- Terminals keep running when you switch workspaces or scroll them off screen. Off-screen or
  zoomed-out terminals show a light text preview.
- A tool bar (Select `V`, Terminal `T`, Note `N`), a New Terminal dialog with quick starts for
  Claude Code, Codex, OpenCode, Gemini CLI, and Shell, plus accent color, font size, roles, and
  an Orchestrator flag.
- Links between terminals and notes: drag from the dot on a selected item onto another one, or use
  the link button in its floating toolbar. Select a link and press Delete to remove it.
- Floating toolbars for a selected note (color, size, Markdown formatting, pictures) or terminal
  (edit, link, restart, delete). `Cmd+W` closes the selected item; `Cmd+Q` quits.
- Agents talk to each other through links with the `sonata` command, available in every Sonata
  terminal: `sonata list`, `sonata ask <name> "<message>"` (waits for the answer),
  `sonata ask --all`, `sonata check <name>`, `sonata reply`, `sonata note read|write`, and
  `sonata notify`. A terminal marked **Orchestrator** can also `sonata recruit`, `sonata dismiss`,
  `sonata link`, `sonata unlink`, `sonata note new`, and `sonata browser new`. Run `sonata help`
  inside a terminal.
- Claude Code and Codex started from the presets run as `sonata run claude|codex`: they get
  Sonata instructions (and the terminal's role) for that session only, plus an end-of-turn
  signal so answers return as soon as the agent finishes. Your global agent settings are never
  changed. In zsh terminals, typing `claude` or `codex` by hand does the same (your own
  `.zshrc` still loads; Sonata only adds two shell functions inside its terminals).
- Agent terminals come back in their conversation after you quit and reopen Sonata or restart
  the terminal. See [Session restore](#session-restore).
- Browser nodes (tool `B`): real web pages on the canvas, with back/forward/reload, address bar,
  developer tools, and "open in default browser". Pages are live at a readable zoom and show a
  card with the last image otherwise. Browsers in the same workspace share logins. Linked agents
  drive them with `sonata browser open|read|click|fill|press|screenshot|back|forward|reload`
  (and `eval` for the Orchestrator).
- A usage meter at the top right of the canvas shows how much of each agent account's plan
  limits is used (short window and weekly, with reset times), warns once at 80% and at 100% of
  a window with a macOS notification, and has a Settings window (`Cmd+,`) to control it. See
  [Usage meter](#usage-meter).
- Device nodes (tool `D`): an Android emulator on the canvas, with no separate emulator window.
  Pick one of your Android virtual devices, boot it, and use its screen live in the node: click
  to tap, drag to swipe, scroll, and type; Back, Home, and Recent apps buttons sit under the
  screen. Linked agents test apps with `sonata device boot|stop|read|tap|swipe|type|key|launch|
  apps|install|screenshot`; `read` prints the screen as numbered elements, like `browser read`.
  iOS simulators (with Xcode) can be listed, booted, and viewed, and agents can take screenshots
  and launch or install apps; taps and keys are not available for them.
- Everything is saved as readable files in `~/.sonata/`, with crash-safe writes, autosave, and
  terminal history restored after a restart.

## Install

1. Download `Sonata_<version>_universal.dmg` from the
   [latest release](https://github.com/DuarteVn/sonata/releases/latest). It runs natively on
   Apple Silicon and Intel Macs with macOS 15 or later.
2. Open the disk image and drag Sonata to Applications.
3. Open Sonata. The first time, macOS blocks it, because Sonata is signed without a paid Apple
   developer account and is not notarized. Click **Done**, then open **System Settings →
   Privacy & Security**, scroll to Security, and click **Open Anyway** next to the Sonata
   message. Confirm with your password. Later launches open normally.

   Or, in Terminal:

   ```sh
   xattr -dr com.apple.quarantine /Applications/Sonata.app
   ```

   This removes the "downloaded from the internet" mark that makes macOS ask. Do it only for a
   copy you downloaded from this project's release page.

The `sonata` command ships inside the app and works in every Sonata terminal; there is nothing
else to install.

### Install on Linux

Sonata runs on x86_64 Linux with glibc 2.35 or later and WebKitGTK 4.1: Ubuntu 22.04 or later,
Debian 12, Fedora 38 or later, Arch, and similar distributions, on X11 or Wayland.

- **AppImage** (recommended, updates itself): download `Sonata_<version>_amd64.AppImage` from
  the [latest release](https://github.com/DuarteVn/sonata/releases/latest), put it in a folder
  you own (for example `~/Applications`), then:

  ```sh
  chmod +x ~/Applications/Sonata_*_amd64.AppImage
  ~/Applications/Sonata_*_amd64.AppImage
  ```

  If it does not start, install the WebKitGTK runtime (`sudo apt install libwebkit2gtk-4.1-0`
  on Ubuntu and Debian) and, on some distributions, FUSE 2 (`sudo apt install libfuse2`).
- **Debian package** (Ubuntu, Debian, and derivatives): download `Sonata_<version>_amd64.deb`
  and run `sudo apt install ./Sonata_<version>_amd64.deb`. Sonata appears in your application
  launcher and `sonata` is installed in `/usr/bin`. The package does not update itself: when a
  new version is out, Sonata says so and links to the release page.

On Linux Sonata also keeps a copy of the `sonata` command in `~/.local/bin/sonata`, so scripts
outside Sonata can call it (Ubuntu puts that folder on your `PATH` after the next login). It only
replaces a file there that is a Sonata CLI.

What differs on Linux:

- Shortcuts use Ctrl, and Ctrl+Shift where Ctrl alone belongs to the terminal: **Ctrl+,**
  Settings, **Ctrl+Shift+W** Close Item, **Ctrl+Shift+S** Toggle Sidebar, **Ctrl+Shift+Q** Quit,
  **Ctrl+Shift+C** / **Ctrl+Shift+V** copy and paste in terminals. Ctrl+C, Ctrl+D, Ctrl+W, and
  the other control keys go to the program in the terminal.
- Typing `claude` or `codex` by hand starts them through Sonata in bash and zsh terminals (other
  shells start normally; presets still work).
- Custom usage commands need bubblewrap (`sudo apt install bubblewrap`); without it they do not
  run, and Settings → Usage says so.
- Device nodes support Android emulators; iOS simulators need a Mac and are not shown.
- "Show in File Manager" opens the folder that holds the file.

### Updates

Sonata checks for a newer release when it starts and once a day while it runs. When one exists,
a banner at the bottom of the sidebar shows the version and its notes (a dot on the show-sidebar
button when the sidebar is hidden). Nothing downloads until you click **Install and restart**.
Sonata then downloads the update, verifies its signature, asks before closing live terminals,
and restarts on the new version. Your data in `~/.sonata/` is not touched.

Use **Sonata → Check for Updates…** to check now, and **Sonata → Check for Updates
Automatically** to turn the automatic check off. On Linux the AppImage updates the same way,
as long as its folder is writable; the Debian package and builds from source show the notice
with a link to the release page instead. The check downloads one small file from GitHub
and sends nothing about you or your workspaces.

### Build from source

```sh
git clone https://github.com/DuarteVn/sonata.git
cd sonata
scripts/install.sh        # builds Sonata.app and copies it to /Applications
```

On Linux the same script checks for the WebKitGTK development files (and prints the `apt`,
`dnf`, or `pacman` command when they are missing), builds an AppImage, and installs it for your
user: `~/.local/bin/sonata-app`, `~/.local/bin/sonata`, and a launcher entry. No `sudo` needed
beyond the system packages.

Run the same script again to update. If Rust is missing, the script offers to install it with
the official installer (rustup) and does nothing without your "yes". An app you build yourself
opens without any Gatekeeper prompt.

## Security notice

Sonata runs shells, `git`, and AI agents with your user permissions, without a sandbox. Only run
commands you trust. Terminal history is saved to disk in plain text, readable only by your user;
turn it off per terminal (terminal menu → Save history) for sessions that print secrets.

## Device nodes: requirements

- Android Studio is not needed. The SDK
  [command-line tools](https://developer.android.com/studio#command-line-tools-only) are
  enough: install `emulator`, `platform-tools`, and one system image with `sdkmanager`, then
  create a virtual device with `avdmanager` (an ARM image on Apple Silicon). `sdkmanager` and
  `avdmanager` need Java 17 or later; Sonata itself does not. Android Studio works too.
- Sonata looks for the SDK in `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `~/Library/Android/sdk`
  and Homebrew's `android-commandlinetools` on macOS, `~/Android/Sdk`, `/opt/android-sdk`, and
  `/usr/lib/android-sdk` on Linux, or a folder you choose in the node ("Choose SDK
  folder…"). Virtual devices come from
  `~/.android/avd` (or `ANDROID_AVD_HOME`).
- Sonata never downloads, installs, or changes SDKs, emulators, or Xcode. When something is
  missing, the node says what, where it looked, and links to the official page.
- The emulator's control port listens on localhost only and needs a per-device token, which
  Sonata reads from the emulator's own files (never from a command line). Devices you start
  from Android Studio are attached to, not copied, and Sonata never stops them.
- Physical phones over USB are not supported yet.

## Agent messaging and safety

- The `sonata` command talks to the app over a private Unix socket (`~/.sonata/run/`, readable
  only by you); there is no network listener.
- Each terminal start gets a secret identity, so one terminal cannot act as another. A terminal
  can only reach the terminals and notes it is linked to.
- Waiting loops (A waits on B while B asks A) are refused; asks time out after 10 minutes by
  default (`--timeout`, up to 60 minutes) and are never sent twice.

## Session restore

When a Claude Code or Codex terminal starts again (app reopen, workspace opened, Restart), the
agent resumes the conversation it was in (`claude --resume <id>`, `codex resume <id>`), with
the same folder, role, lane, and Sonata instructions. The remembered conversation follows the
agent: `/clear`, `/resume`, or a new session inside the agent update it. Codex conversations are
remembered after their first finished turn.

- **Start fresh** (terminal menu, shown when a conversation is remembered) forgets it and
  restarts the agent in a new conversation. The agent's own conversation files are not touched.
- If resuming fails (conversation files deleted, agent updated, folder changed), the terminal
  prints `sonata: the previous conversation could not be resumed; starting a new one.` and
  starts a new conversation. The failed one is not tried again.
- Your own arguments win: a launch command with `--resume`, `--continue`, `-p`, a subcommand,
  and similar starts exactly what you asked for.
- Sonata stores only the conversation id (in `workspace.json`). The conversation itself stays in
  the agent's own files.

## Usage meter

The Usage button (top right of the canvas) shows the highest share used across your agent
accounts; click it for the panel. Settings → Usage (`Cmd+,`) has a master switch, the warning
switch, and one row per agent with its color, command path, status, switch, and refresh.

Where the numbers come from. Sonata never reads your logins: each agent's own command uses
its login to answer.

- **Claude Code**: `claude auth status --json` (plan and account), then one tiny print-mode
  request on Haiku (about 600 tokens) whose `stream-json` output reports the 5-hour and weekly
  limits. It runs every 5 minutes while usage reading is on, when you open the panel and the
  value is older than a minute, and when you press refresh. Accounts that pay per use (API key,
  Bedrock, Vertex) have no plan limits and are not probed.
- **Codex**: `codex app-server`, asked for its rate limits (no model request), or, with older
  versions, the last limits Codex recorded in its newest session file.
- **Other agents**: add a definition file to `~/.sonata/usage-providers/` (Settings → Usage →
  Show folder). The `README.md` in that folder explains the format and has a prompt you can give
  an agent to write one.

Sonata asks before it first runs an agent's usage command and shows the exact command; it asks
again when a custom command changes. Custom commands run without a shell, stop after 10
seconds, and run inside a sandbox that blocks the keychain, the Claude Code and Codex login
files, and Sonata's data folder: `sandbox-exec` on macOS, bubblewrap on Linux (which also hides
the desktop keyrings and the session's D-Bus folder). Without the sandbox they do not run. The last readings are kept in `usage-cache.json`
without credentials or emails. To test it, follow `specs/006-agent-usage/quickstart.md`.

## Your data

```text
~/.sonata/
├── app-state.json, preferences.json, roles.json
├── usage-cache.json                    last usage readings (no credentials)
├── usage-providers/                    custom usage provider definitions + README.md
├── logs/sonata.log.<date>              kept 7 days, never contains terminal output
├── lanes/<workspace id>/<name>/        lane checkouts (git worktrees of your repository)
├── run/                                sockets, screenshots, and device card images (temporary)
└── workspaces/<id>/
    ├── workspace.json                  layout: nodes, camera, settings
    ├── notes/<node-id>.md              your notes, editable with any editor
    ├── files/                          pictures inserted into notes
    └── terminals/<node-id>.history     saved terminal output
```

Formats are documented in `specs/001-workspaces-canvas-terminals/contracts/file-formats.md`.

## Development

### Prerequisites

- macOS 15 or later with the Xcode Command Line Tools (`xcode-select --install`), or Linux
  with the WebKitGTK 4.1 development packages (see "Build from source")
- Rust via [rustup](https://rustup.rs)
- Node.js 22 or later

### Run

```sh
npm install
npm run tauri dev
```

Sonata stores its data in `~/.sonata/`. To try things without touching your real data, point it
at a throwaway folder:

```sh
SONATA_HOME="$(mktemp -d)/sonata" npm run tauri dev
```

### Test

```sh
scripts/check.sh                 # fmt, clippy, Rust tests, type check, frontend tests
npx playwright install webkit    # once
npm run e2e                      # Playwright against the frontend with mocked IPC
scripts/bench-terminals.sh 30    # real app with 30 streaming terminals in a temp data folder
```

### Layout

| Path | What |
|---|---|
| `crates/core` | Model, persistence, terminals, notes. No Tauri dependency. |
| `src-tauri` | Thin Tauri shell: commands and events that call `core`. |
| `ui` | React + TypeScript frontend (canvas, sidebar, xterm.js). |
| `e2e` | Playwright tests with mocked IPC. |
| `specs` | Feature specifications, plans, and task lists. |
| `scripts/release` | Release helpers (version check, update manifest) and their shell tests, including the Linux smoke run. |

### Releasing

Releases are built by GitHub Actions (`.github/workflows/release.yml`) when a version tag is
pushed. Nothing is paid: the app is signed ad hoc and updates are signed with the project's own
key.

One-time setup (maintainer):

```sh
npm run tauri signer generate -- -w ~/.tauri/sonata.key    # choose a password
# Paste the printed public key into plugins.updater.pubkey in src-tauri/tauri.conf.json.
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/sonata.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD            # type the password at the prompt
```

Keep `~/.tauri/sonata.key` and its password backed up. If the key is lost, installed copies can
no longer verify updates, and every user must download the next version by hand.

Each release:

1. Set the same version in `src-tauri/tauri.conf.json`, `Cargo.toml` (`[workspace.package]`),
   and `package.json`; `scripts/release/check-version.sh v0.2.0` must pass.
2. Commit, then `git tag v0.2.0 && git push origin v0.2.0`.
3. The workflow runs the checks, creates a draft release, builds the universal macOS app (disk
   image, update archive and signature) and the Linux x86_64 files (AppImage and its signature,
   Debian package) in parallel, then writes `latest.json` with `darwin-aarch64`,
   `darwin-x86_64`, and `linux-x86_64` entries and publishes the release. If a build fails, the
   draft stays unpublished and the previous release remains the latest. Installed apps see the
   new one on their next check.

## License

[MIT](LICENSE)
