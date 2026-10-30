# Feature Specification: Windows Support

**Feature Branch**: `012-windows-support`

**Created**: 2026-10-29

**Status**: Draft

## Clarifications

### Session 2026-10-29

Answers delegated by the user to the Orchestrator terminal.

- Q: How does the `sonata` command reach the app on Windows? → A: A named pipe whose access list allows only the current user, plus the existing per-terminal token; same newline-delimited JSON protocol.
- Q: Which CPU architectures ship? → A: x86_64 only; native arm64 later.
- Q: Which installer format? → A: NSIS `.exe`, per-user install (no administrator rights), wired to the updater (manifest key `windows-x86_64`); no MSI now; unsigned, with the SmartScreen "More info → Run anyway" step documented like Gatekeeper.
- Q: Minimum Windows version? → A: Windows 10 1809 or later and Windows 11; WebView2 installed through the bootstrapper when missing.
- Q: Which shells, and how are they chosen? → A: Default PowerShell (PowerShell 7 when installed, else Windows PowerShell); Git Bash and WSL selectable per terminal when detected; Command Prompt available.
- Q: Which shells get agent shell integration? → A: PowerShell first; the bash integration is reused for Git Bash; WSL is best effort.
- Q: Where is the data folder, and how is it kept private? → A: `%USERPROFILE%\.sonata` with the macOS layout; private-tree hardening uses user-only access lists instead of `chmod`.
- Q: How is the `sonata` command found outside Sonata terminals? → A: Bundled with the app and added to the user PATH by the installer.
- Q: Which path forms must work? → A: Separators are normalized; drive letters and long paths work in the data folder, lanes, and git calls.
- Q: How do Cmd shortcuts map on Windows? → A: Cmd maps to Ctrl; Ctrl keys that a focused terminal needs stay with the terminal; app shortcuts that clash use Ctrl+Shift.
- Q: What happens to custom usage providers on Windows? → A: Disabled with a clear message (no sandbox equivalent now); never run unsandboxed.
- Q: What if the browser page image cannot be captured on Windows? → A: Implement it for WebView2 if feasible; otherwise degrade gracefully with a message.
- Q: Which device nodes exist on Windows? → A: Android emulator supported; iOS hidden.
- Q: How does this feature relate to the Linux port? → A: Windows depends on Linux (011) for shared cross-platform groundwork and rebases on main after 011 merges.

**Input**: User description: "Feature 012: Windows support for Sonata. Goal: friends on Windows 10/11 can download a prebuilt Sonata installer from GitHub Releases and use it like on macOS: canvas, terminals with AI agents, notes, links, the sonata CLI and agent messaging, browser node, lanes, usage meter, session restore, and auto-update. Scope ideas to confirm in clarify: target x86_64 (arm64 optional); installer format (NSIS .exe or MSI) wired to the Tauri updater; terminals via ConPTY (portable-pty) with PowerShell as default shell and optional Git Bash / WSL; shell integration so hand-typed agents know Sonata in PowerShell; how the sonata CLI reaches the app (Unix sockets or file permissions used today may need named pipes or a localhost transport with a token); file permission hardening (enforce_private_tree chmod) via Windows ACLs; path handling (backslashes, drive letters, long paths) in data root, lanes, and git calls; WebView2 for the browser node and snapshot (src-tauri/src/browser/snapshot.rs is macOS-only today); custom usage providers sandbox (sandbox-exec is macOS-only; find an equivalent or disable with a clear message); device node: Android emulator works on Windows, iOS hidden; Cmd to Ctrl shortcut mapping; SmartScreen warning for an unsigned installer, documented like Gatekeeper. Start by mapping every macOS/Unix-only code path (grep cfg(unix), cfg(target_os), std::os::unix, sandbox-exec, simctl, osakit, chmod, Unix sockets, /Applications, HOME) and list them in research.md."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Friend installs Sonata on Windows from a download (Priority: P1)

A friend with a Windows 10 or Windows 11 PC and no developer tools downloads one installer
from the project's release page, runs it, follows the documented SmartScreen step once, and
starts Sonata from the Start menu. The `sonata` command works inside Sonata terminals with no
extra setup.

**Why this priority**: Today Sonata runs only on macOS. Without an installer, Windows friends
cannot use Sonata at all.

**Independent Test**: On a clean Windows PC without Rust, Node, or Visual Studio, download the
installer from a release, install it, open Sonata after the SmartScreen step, create a
terminal, and run `sonata help`.

**Acceptance Scenarios**:

1. **Given** a Windows 11 PC with no developer tools, **When** the friend downloads the
   installer from the latest release and runs it, **Then** Sonata installs without
   administrator rights and appears in the Start menu.
2. **Given** Windows shows a SmartScreen warning for the unsigned installer, **When** the friend
   follows the README step ("More info" then "Run anyway"), **Then** the installer runs and
   later launches of Sonata show no warning.
3. **Given** a fresh install, **When** the friend opens a terminal in Sonata and runs
   `sonata help`, **Then** the command runs, because it ships with the app.
4. **Given** a Windows 10 PC (version 1809 or later), **When** the friend installs the same
   installer, **Then** Sonata runs with the same features as on Windows 11.
5. **Given** the WebView2 runtime is missing, **When** the friend runs the installer, **Then**
   the installer gets the runtime, or tells the friend how to get it, before Sonata starts.

---

### User Story 2 - Terminals with AI agents work on Windows (Priority: P1)

The friend creates terminals on the canvas. Each terminal runs a real Windows shell
(PowerShell by default) in the workspace folder. Quick starts for Claude Code, Codex, and other
agents start the agent with Sonata's instructions, and typing `claude` or `codex` by hand in a
Sonata PowerShell terminal does the same. Agents talk to each other through links with
`sonata ask`, `sonata check`, `sonata reply`, notes, and `sonata notify`.

**Why this priority**: Terminals with agents and agent messaging are the core of Sonata. A
Windows build without them has no value.

**Independent Test**: On Windows, create two linked terminals, start an agent in one, and run
`sonata ask <other> "hello"` from the other; the answer returns. Run `sonata list` from an
unlinked terminal; it does not show or reach the first one.

**Acceptance Scenarios**:

1. **Given** a workspace on a Windows folder (for example `C:\Users\Ana\code\app`), **When** the
   friend adds a terminal, **Then** a PowerShell prompt opens in that folder, with colors,
   resizing, copy, paste, and Ctrl+C working.
2. **Given** a terminal started from the Claude Code quick start, **When** the agent finishes a
   turn, **Then** Sonata detects the end of the turn the same way it does on macOS.
3. **Given** a Sonata PowerShell terminal, **When** the friend types `claude` by hand, **Then**
   the agent starts with Sonata's instructions for that session only, and the friend's own
   PowerShell profile still loads.
4. **Given** two linked terminals, **When** one runs `sonata ask`, **Then** the message reaches
   the other and the answer returns, exactly as on macOS.
5. **Given** a process outside Sonata (or another Windows user) tries to send commands to
   Sonata, **When** it connects without a valid terminal identity, **Then** Sonata refuses the
   request.
6. **Given** Git Bash or a WSL distribution is installed, **When** the friend picks it (or
   Command Prompt) for a terminal in the New Terminal dialog, **Then** that terminal starts in
   the chosen shell, and terminals without a choice start in PowerShell.
7. **Given** a Command Prompt or other terminal outside Sonata, **When** the friend runs
   `sonata help`, **Then** the command is found, because the installer added it to the user
   PATH.

---

### User Story 3 - Maintainer publishes Windows and macOS releases together (Priority: P1)

The maintainer pushes a version tag as today. Automation builds the macOS disk image as
before and, in addition, the Windows installer. It signs the Windows update files with the
same project update key and publishes one release whose update manifest serves both macOS and
Windows. No paid service is used.

**Why this priority**: Story 1 and Story 5 have nothing to download without this.

**Independent Test**: Push a test version tag on a fork or a test release; confirm that the
release holds the macOS files and the Windows installer with its update signature, and that the
update manifest lists both platforms.

**Acceptance Scenarios**:

1. **Given** a version tag is pushed, **When** the release automation finishes, **Then** the
   release holds the macOS disk image, the macOS update archive, the Windows installer, and the
   update signatures, and the update manifest lists macOS and Windows entries.
2. **Given** the Windows build fails, **When** the automation runs, **Then** no release is
   published with a partial update manifest, and the log says which step failed.
3. **Given** an installed macOS copy of a previous version, **When** it reads the new manifest,
   **Then** it finds its update exactly as before (no regression for existing macOS users).
4. **Given** a normal push or pull request, **When** CI runs, **Then** the core and CLI tests
   also run on a Windows runner, so Windows regressions show before a release.

---

### User Story 4 - Notes, links, browser nodes, lanes, and session restore on Windows (Priority: P2)

On Windows, the friend uses every canvas feature available on macOS: notes with pictures,
links, browser nodes driven by agents, lanes (git worktrees for agents), and session restore
after a restart. Data lives in the friend's user profile and is private to that Windows user.

**Why this priority**: These features make Sonata more than a terminal grid, but terminals and
messaging (Story 2) come first.

**Independent Test**: On Windows, create a note with a picture, a browser node, and a terminal
in a lane on a git repository; quit Sonata and start it again; all items return, and
`sonata browser read` and `sonata lane status` work.

**Acceptance Scenarios**:

1. **Given** a workspace folder with spaces or non-ASCII letters in its path, **When** the
   friend creates terminals, notes, and lanes, **Then** everything works and the files save
   correctly.
2. **Given** a git repository on a drive other than `C:`, **When** an agent creates a lane and
   later the friend lands it, **Then** the worktree is created, landed, and removed with the
   same confirmations as on macOS.
3. **Given** a browser node, **When** a linked agent runs `sonata browser screenshot`, or the
   node goes off screen, **Then** Sonata captures an image of the page as on macOS.
4. **Given** Sonata's data folder, **When** another Windows user on the same PC tries to read
   it, **Then** access is denied.
5. **Given** a session with terminals running, **When** the friend restarts Sonata, **Then**
   the terminals, their history, and agent sessions return as on macOS.
6. **Given** a link in a terminal or the "open in default browser" button, **When** the friend
   clicks it, **Then** the page opens in the default Windows browser.

---

### User Story 5 - Windows app updates itself on confirmation (Priority: P2)

A friend running Sonata on Windows learns that a newer version exists through the same notice
as on macOS, confirms, and Sonata installs the update and restarts.

**Why this priority**: Without updates, friends stay on old builds; but a first install
(Story 1) must exist first.

**Independent Test**: Install an older Windows build, publish a newer test release, start the
old build, confirm the update notice, and check that the newer version runs afterwards with all
data kept.

**Acceptance Scenarios**:

1. **Given** a Windows install of an older version, **When** a newer release exists, **Then**
   Sonata shows the update notice as on macOS.
2. **Given** the friend confirms the update while terminals run programs, **When** Sonata
   installs it, **Then** Sonata asks before it stops the running programs, as on macOS.
3. **Given** a downloaded update with a signature that does not match the project key, **When**
   Sonata checks it, **Then** Sonata refuses to install it.

---

### User Story 6 - Windows keyboard and platform conventions (Priority: P2)

Shortcuts that use Cmd on macOS use Ctrl on Windows (Ctrl+W closes the selected item, Ctrl+,
opens Settings), labels and tooltips show Windows key names, and terminal shortcuts follow
Windows terminal habits (Ctrl+C copies when text is selected, else interrupts; Ctrl+V pastes).
Features that cannot exist on Windows are hidden, not broken.

**Why this priority**: Wrong shortcuts make the app feel broken, but they do not block use.

**Independent Test**: On Windows, check every documented shortcut and every tooltip that names
a key; none mentions Cmd or a macOS-only symbol, and each works with Ctrl.

**Acceptance Scenarios**:

1. **Given** Windows, **When** the friend presses Ctrl+W with a note selected, **Then** the
   note closes, not the window.
2. **Given** Windows, **When** the friend opens the device node, **Then** only Android devices
   are offered; iOS is not shown.
3. **Given** Windows, **When** the friend opens the usage Settings, **Then** custom usage
   providers are disabled and a clear message says why they are not available on Windows.
4. **Given** Windows, **When** a usage limit warning fires, **Then** it appears as a Windows
   notification.
5. **Given** a focused terminal, **When** the friend presses a Ctrl key the shell needs (for
   example Ctrl+W, Ctrl+R), **Then** the key goes to the terminal; the app action that uses it
   is reachable with Ctrl+Shift.

---

### Edge Cases

- The user's profile path has spaces, non-ASCII letters, or is longer than 260 characters.
- The workspace folder is on a network drive, a mapped drive letter, or a WSL path
  (`\\wsl$\...`).
- Git for Windows is not installed: lanes and git features show a clear message, as when git is
  missing on macOS.
- The repository has files with Windows line endings, or `core.autocrlf` is set.
- PowerShell 7 is installed next to Windows PowerShell 5.1: Sonata picks one in a predictable
  way.
- The user's PowerShell execution policy blocks scripts: Sonata's shell integration still
  loads, or the agent helpers degrade with a message, and the user's policy is not changed.
- An antivirus program quarantines or slows the bundled `sonata` command.
- Two Windows users on the same PC run Sonata at the same time: each has its own data and its
  own command channel, and neither can reach the other.
- Sonata is already running and the friend starts it again from the Start menu.
- An update runs while the installer cannot replace files that are in use.
- The Android SDK is in its default Windows location (`%LOCALAPPDATA%\Android\Sdk`).
- A data folder copied from a Mac to a Windows PC (or back): files load, or a load failure is
  shown; nothing is silently overwritten.

## Requirements *(mandatory)*

### Functional Requirements

**Distribution**

- **FR-001**: The project MUST publish a Windows installer for x86_64 Windows 10 (version 1809
  or later) and Windows 11 with each release, next to the macOS files.
- **FR-002**: The installer MUST be a single `.exe` that installs Sonata for the current user
  without administrator rights, adds a Start menu entry, and offers a normal uninstall. No MSI
  package ships in this feature.
- **FR-003**: The installer MUST bundle the `sonata` command, add it to the user PATH (and
  remove it on uninstall), and the command MUST be available in every Sonata terminal without
  extra setup.
- **FR-004**: The release automation MUST build, sign (with the project's own update key), and
  publish the Windows update files in the same run as the macOS files, and MUST publish one
  update manifest that serves both platforms.
- **FR-005**: The update manifest MUST keep every field that existing macOS installs read
  today; adding Windows MUST NOT change how macOS installs find their update.
- **FR-006**: Building and distributing the Windows version MUST NOT require any paid service
  (no paid code-signing certificate).
- **FR-007**: The README MUST document the Windows install, the SmartScreen "Run anyway" step
  for an unsigned installer, uninstall, and the same security notice as for macOS.
- **FR-008**: CI MUST run the core and CLI test suites on a Windows runner for pull requests,
  in addition to the existing jobs.

**Terminals and shells**

- **FR-009**: Terminals on Windows MUST run a real interactive shell with full terminal
  behavior (colors, cursor, resize, Ctrl+C, wide characters).
- **FR-010**: The default shell on Windows MUST be PowerShell (PowerShell 7 when installed,
  otherwise Windows PowerShell). The user MUST be able to choose Command Prompt, and Git Bash or
  an installed WSL distribution when detected, per terminal.
- **FR-011**: Sonata terminals MUST receive the same Sonata identity and environment as on
  macOS, so the `sonata` command and agent presets work.
- **FR-012**: In Sonata PowerShell terminals, typing `claude` or `codex` by hand MUST start the
  agent with Sonata's instructions, as in zsh on macOS, while the user's own profile still
  loads and the user's settings, profile, and execution policy are never changed. Git Bash
  terminals MUST get the same helpers through a bash integration. WSL terminals get them on a
  best-effort basis.
- **FR-013**: Sonata MUST detect whether a terminal is busy (a program runs in the foreground)
  on Windows, so quit, restart, and update warnings and the shell-quiet logic work as on macOS.

**Command channel and security**

- **FR-014**: The `sonata` command MUST reach the running app through a local named channel
  whose access list allows only the current Windows user, using the same message protocol and
  per-terminal token as on macOS; no network listener is allowed.
- **FR-015**: The app MUST keep authorizing each request from the calling terminal's identity;
  a request without a valid identity MUST be refused.
- **FR-016**: Sonata's data folder MUST be readable and writable only by the current Windows
  user (and the system), set through user-only access lists, the Windows counterpart of the
  private permissions used on macOS.
- **FR-017**: All secrets MUST stay out of command-line arguments on Windows, as on macOS.

**Paths and files**

- **FR-018**: The data folder on Windows MUST default to `%USERPROFILE%\.sonata`, with the same
  layout as on macOS, and MUST honor `SONATA_HOME` as on macOS.
- **FR-019**: Sonata MUST handle Windows paths everywhere it stores, compares, or validates
  them: drive letters, backslashes, case-insensitive comparison, UNC paths, and paths longer
  than 260 characters. Path validation (no `..`, exact directory prefixes) MUST give the same
  protection as on macOS.
- **FR-020**: Lanes MUST work with Git for Windows: create, status, land, and remove worktrees,
  with the same confirmations and the same rule to remove only Sonata-managed checkouts.
- **FR-021**: Files written on one platform MUST load on the other platform, or be reported as
  a load failure; they MUST NOT crash the app or be silently overwritten.

**Other features**

- **FR-022**: Browser nodes MUST work on Windows. The page image used for off-screen nodes and
  `sonata browser screenshot` MUST work if the Windows web view allows it; otherwise the node
  keeps working without the image and `sonata browser screenshot` returns a clear message.
- **FR-023**: Opening a URL or a file "in the default app" MUST use the Windows default handler.
- **FR-024**: The usage meter MUST work on Windows for the built-in providers, and usage
  warnings MUST appear as Windows notifications.
- **FR-025**: Custom usage providers MUST be disabled on Windows, with a clear message in
  Settings that says why. Sonata MUST NEVER run them without a sandbox.
- **FR-026**: Device nodes on Windows MUST support Android emulators (with the SDK in its
  default Windows location or a configured one) and MUST NOT offer iOS.
- **FR-027**: Session restore MUST work on Windows as on macOS once that feature is part of the
  main line.
- **FR-028**: Every shortcut that uses Cmd on macOS MUST use Ctrl on Windows, and every visible
  key name (menus, tooltips, help, README) MUST show the Windows name on Windows. When a
  terminal has focus, Ctrl keys the shell needs MUST go to the terminal; app shortcuts that
  clash with them MUST use Ctrl+Shift.
- **FR-029**: A Windows user MUST be able to start only one Sonata instance; starting it again
  MUST bring the running window to the front.

**No regression**

- **FR-030**: macOS behavior, packaging, and update delivery MUST NOT change, and all existing
  macOS checks MUST still pass.

### Key Entities

- **Windows installer**: the file a friend downloads and runs; installs the app and the
  `sonata` command for one user; also used by the updater.
- **Update manifest**: one file per release that lists, per platform, the update file and its
  signature; read by every installed copy.
- **Default shell setting**: the shell new terminals start on Windows; one of the shells found
  on the PC.
- **Command channel**: the local, per-user channel between the `sonata` command and the app;
  carries terminal identity.
- **Platform capability**: whether a feature exists on the current OS (for example iOS devices
  or custom usage providers); drives what the UI shows.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A friend with a clean Windows 10 or 11 PC goes from the release page to a running
  agent in a Sonata terminal in under 5 minutes, with no developer tools installed.
- **SC-002**: 100% of the core and CLI automated tests that apply to Windows pass on a Windows
  runner, and 100% of the existing macOS checks still pass.
- **SC-003**: Every feature listed in the README for macOS either works on Windows or is
  documented as macOS-only (target: only iOS devices are macOS-only).
- **SC-004**: An installed Windows copy updates to a newer release in under 2 minutes after the
  friend confirms, with all workspaces, notes, and settings kept.
- **SC-005**: A `sonata ask` round trip between two linked terminals on Windows adds no more
  than 1 second over the agent's own answer time, as on macOS.
- **SC-006**: Zero requests from a process without a valid terminal identity, or from another
  Windows user, succeed in the security tests.
- **SC-007**: No visible text on Windows names Cmd, ⌘, Finder, or another macOS-only term,
  checked by an automated scan of the UI strings shown on Windows.

## Assumptions

- Target is x86_64 Windows 10 (1809 or later, which has the modern console host) and Windows 11.
  Windows on ARM runs the x86_64 build through emulation; a native arm64 build comes later.
- The installer is per-user and unsigned (no paid certificate), like the ad-hoc signed macOS
  build; SmartScreen shows a warning that the README explains.
- WebView2 is present on Windows 11 and on current Windows 10; the installer bootstraps it when
  missing.
- Git features need Git for Windows on the PATH; Sonata does not bundle git.
- AI agent CLIs (Claude Code, Codex, and others) are installed by the user and run natively on
  Windows; Sonata does not install them.
- Session restore (feature 009) and the Linux port (feature 011) land on the main line before
  this feature merges; Windows reuses 011's cross-platform groundwork and this branch rebases
  on main after 011 merges.
- This Mac cannot run Windows, so verification happens on GitHub Actions Windows runners, plus
  a manual check by a friend on a real PC before the first public Windows release.
- The project constitution currently limits the target to macOS; this feature amends it to add
  Windows (and coordinates with the Linux amendment).
