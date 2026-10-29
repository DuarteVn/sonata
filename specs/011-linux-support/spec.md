# Feature Specification: Linux Support

**Feature Branch**: `011-linux-support`

**Created**: 2026-10-29

**Status**: Draft

**Input**: User description: "Linux support for Sonata. Goal: friends on Linux can download a prebuilt Sonata from GitHub Releases and use it like on macOS: canvas, terminals with AI agents, notes, links, the sonata CLI and agent messaging, browser node, lanes, usage meter, session restore, and auto-update. Scope ideas to confirm in clarify: target x86_64 (arm64 optional); package formats (AppImage for auto-update via Tauri updater, plus .deb); distro baseline (Ubuntu 22.04+ / webkit2gtk-4.1); shell integration for bash as well as zsh (agents typed by hand must know Sonata); data root and sonata CLI install path on Linux; browser node snapshot (src-tauri/src/browser/snapshot.rs is macOS-only today); custom usage providers run under sandbox-exec on macOS (find a Linux equivalent such as bubblewrap, or disable with a clear message); device node: Android emulator works on Linux, iOS hidden; keyboard shortcuts Cmd to Ctrl mapping; install-from-source script for Linux. macOS behavior must not regress. Verification happens on GitHub Actions Linux runners."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Friend installs Sonata on Linux from a download (Priority: P1)

A friend with a Linux desktop (for example Ubuntu 22.04 or later) and no developer tools
downloads one file from the project's release page, makes it runnable or installs it with the
system package tool, and starts Sonata. The canvas, workspaces, terminals, notes, and links work
as on macOS. The `sonata` command works inside Sonata terminals with no extra setup.

**Why this priority**: Today Sonata is macOS only. Friends on Linux cannot use it at all without
this story; every other story depends on a running app.

**Independent Test**: On a clean Linux machine without Rust or Node, download the release file,
start Sonata, create a workspace with two terminals and a note, link them, and run
`sonata list` in one terminal.

**Acceptance Scenarios**:

1. **Given** a supported Linux desktop with no developer tools, **When** the friend downloads the
   portable release file, marks it executable, and opens it, **Then** Sonata starts and shows the
   workspace list.
2. **Given** a Debian-based distribution, **When** the friend installs the release package with
   the system package tool, **Then** Sonata appears in the application launcher and starts.
3. **Given** a fresh install, **When** the friend opens a terminal in Sonata and runs
   `sonata help`, **Then** the command runs, because it ships with the app.
4. **Given** Sonata was used before on the same machine, **When** it starts again, **Then** all
   workspaces, nodes, links, and positions come back as they were (session restore).
5. **Given** a required system library is missing, **When** the friend starts the portable file,
   **Then** the README names the package to install for the supported distributions.

---

### User Story 2 - Agents work on Linux as on macOS (Priority: P1)

The friend starts AI agents (for example `claude` or `codex`) in Sonata terminals, both from the
terminal presets and by typing the command by hand. Agents can message linked terminals, read
and write linked notes, and drive linked browsers through the `sonata` command. Lanes give each
agent its own git checkout. This works with bash and with zsh as the login shell.

**Why this priority**: Agent orchestration is the main reason Sonata exists; an app that only
shows terminals is not useful.

**Independent Test**: In a Linux Sonata with bash as the login shell, start an agent by typing its
name, link it to a second terminal and a note, and run `sonata ask`, `sonata note write`, and a
lane create and land through the CLI.

**Acceptance Scenarios**:

1. **Given** bash is the login shell, **When** the user types `claude` by hand in a Sonata
   terminal, **Then** the agent starts with Sonata's identity and messaging available, and the
   user's own bash startup files still load.
2. **Given** zsh is the login shell, **When** the user types an agent name by hand, **Then** the
   behavior is the same as on macOS today.
3. **Given** two linked terminals, **When** one runs `sonata ask <other> "…"`, **Then** the other
   receives the message and the answer returns, as on macOS.
4. **Given** a git repository and git installed, **When** the user creates a lane for an agent,
   **Then** the agent works in its own checkout, and landing merges it back as on macOS.
5. **Given** git is not installed, **When** the user tries to create a lane, **Then** Sonata says
   how to install git with the system package tool (no macOS-only instructions).
6. **Given** the user's login shell is neither bash nor zsh (for example fish), **When** a
   terminal opens, **Then** the shell starts normally, presets still launch agents through
   Sonata, and only hand-typed agent names lack Sonata integration.

---

### User Story 3 - Maintainer publishes macOS and Linux in one release (Priority: P1)

The maintainer pushes a version tag as today. Automation builds the macOS disk image as before
and also builds the Linux files, signs the Linux update file with the same project update key,
and publishes one release whose update manifest serves both macOS and Linux. Existing macOS
installs keep updating with no change.

**Why this priority**: Without automated Linux builds there is nothing for Story 1 to download,
and this Mac cannot build Linux files.

**Independent Test**: Run the release automation on a test tag; confirm the release holds the
macOS disk image, the Linux portable file, the Linux package, their update signatures, and one
update manifest with entries for both systems; confirm a macOS install of the previous version
still finds the update.

**Acceptance Scenarios**:

1. **Given** a version tag is pushed, **When** the release automation finishes, **Then** one
   release holds the macOS files (unchanged) and the Linux files, and the update manifest lists
   both platforms.
2. **Given** the Linux build fails, **When** the automation runs, **Then** no release is
   published and the macOS-only manifest of the previous release stays in place.
3. **Given** a normal push or pull request, **When** CI runs, **Then** the app builds and the
   platform tests run on Linux, so a change that breaks Linux fails before merge.
4. **Given** a macOS install of the previous version, **When** it checks for updates against the
   new manifest, **Then** it finds and installs the macOS update exactly as before.

---

### User Story 4 - Linux app updates itself on confirmation (Priority: P2)

A friend running the portable Linux file sees the same update notice as on macOS, chooses
"Install and restart", and Sonata replaces itself with the new version. A friend who installed
the system package is told where to download the new package, because the package tool owns
that install.

**Why this priority**: Without updates, friends stay on old versions. It depends on Story 3.

**Independent Test**: Run version N of the portable file; publish N+1; confirm the notice
appears, the install works, and N+1 starts with all workspaces intact.

**Acceptance Scenarios**:

1. **Given** the portable file of version N runs and N+1 is published, **When** the update check
   runs, **Then** the notice shows N+1 and its notes.
2. **Given** live terminals run, **When** the user chooses "Install and restart", **Then** the
   same quit confirmation as on macOS appears first.
3. **Given** Sonata was installed with the system package, **When** an update exists, **Then**
   the notice says a new version exists and links to the release page instead of installing.
4. **Given** the downloaded update has a bad signature, **When** Sonata verifies it, **Then** it
   refuses to install and keeps the current version.

---

### User Story 5 - Platform features degrade clearly on Linux (Priority: P2)

Features that depend on macOS-only system parts either work through a Linux equivalent or say
plainly that they are not available on Linux. Nothing fails silently and nothing macOS-specific
(Finder, Xcode, Cmd key names) appears in the Linux interface.

**Why this priority**: Friends must trust what they see. A blank browser card or a usage meter
stuck on an error without explanation looks like a broken app.

**Independent Test**: On Linux, open each of: a browser node, the usage meter with a custom
provider, the device node, the load-failure "reveal" action, and the shortcut hints; confirm each
works or shows a Linux-specific explanation.

**Acceptance Scenarios**:

1. **Given** a browser node on Linux, **When** the page loads, **Then** the page is usable live,
   and the canvas card shows a picture of the page or, if pictures are not supported, a clear
   placeholder with the page title and address.
2. **Given** a browser node is deleted with its workspace, **When** the workspace is deleted,
   **Then** that workspace's browsing data is removed from disk, as on macOS.
3. **Given** a custom usage provider on Linux, **When** the meter reads it, **Then** the command
   runs inside a restricted environment equivalent in intent to the macOS one, or the provider
   shows "not available on Linux" with the reason; it never runs unrestricted.
4. **Given** the Android SDK and emulator are installed on Linux, **When** the user adds a device
   node, **Then** Android devices work as on macOS, and iOS options are not shown.
5. **Given** a workspace file fails to load, **When** the user chooses the reveal action, **Then**
   the system file manager opens at that file, and the action is not labeled "Finder".
6. **Given** the Linux app, **When** the user reads menus, tooltips, and shortcut hints, **Then**
   they show Ctrl-based shortcuts that work, and no shortcut clashes with terminal control keys
   that agents need (for example Ctrl+C, Ctrl+D, Ctrl+W inside a terminal).
7. **Given** a link to an external page, **When** the user opens it, **Then** it opens in the
   system default browser.

---

### User Story 6 - Contributor builds from source on Linux (Priority: P3)

A contributor on Linux clones the repository and runs one install script. The script checks for
the build tools and system libraries, says which packages to install when one is missing, builds
Sonata, and installs it for the current user.

**Why this priority**: The constitution keeps build from source as a supported path; Linux
contributors need it too, but friends use Story 1.

**Independent Test**: On a Linux runner, run the script with the documented packages installed;
confirm it builds and installs Sonata and the `sonata` command.

**Acceptance Scenarios**:

1. **Given** the system libraries are present, **When** the script runs, **Then** it builds and
   installs Sonata for the current user without root.
2. **Given** a system library is missing, **When** the script runs, **Then** it stops before
   building and prints the install command for the detected distribution family.
3. **Given** the script runs on macOS, **When** it runs, **Then** it behaves exactly as today.

### Edge Cases

- The friend runs on Wayland instead of X11: the app starts and works on both display systems
  of the supported distributions.
- The friend's distribution is older than the baseline: the README states the baseline; the app
  is not expected to start, and that is documented.
- The portable file runs from a read-only location or the user has no write access to it: the
  update install fails without damage and Sonata says where to put the file.
- The data folder from a macOS install is copied to Linux: workspaces load, because the files are
  the same format; paths that do not exist on Linux show as missing, not as a crash.
- The login shell is set in `$SHELL` but missing from the system user database (common in
  containers): Sonata falls back as today.
- The restricted environment for custom usage providers is not installed: the provider shows the
  reason and the package name to install.
- An agent sends a browser snapshot command on Linux where pictures are not supported: the
  command fails with a clear message, not a hang.
- A shortcut hint shown on macOS as ⌘ is shown on Linux as Ctrl, and on macOS the hints stay as
  today.

## Requirements *(mandatory)*

### Functional Requirements

**Distribution**

- **FR-001**: Each release MUST include a portable Linux file that runs without installation on
  the supported distributions for the x86_64 architecture.
- **FR-002**: Each release MUST include a Debian package for x86_64 that installs Sonata, its
  launcher entry, its icon, and the `sonata` command.
- **FR-003**: The release automation MUST publish one update manifest that holds entries for the
  existing macOS platforms (unchanged) and for Linux x86_64.
- **FR-004**: The release MUST NOT be published when any platform build fails.
- **FR-005**: The macOS release files, their names, and the update entries macOS installs read
  MUST stay compatible with existing installs.
- **FR-006**: The Linux update file MUST be signed with the same project update key as macOS.
- **FR-007**: Building and distributing the Linux files MUST NOT need any paid service.

**Updates**

- **FR-008**: The portable Linux app MUST support the same update check, notice, confirmation,
  install, and restart flow as macOS.
- **FR-009**: When Sonata runs from the Debian package, the update notice MUST point to the
  release page instead of installing in place.
- **FR-010**: When the update cannot replace the running file (no write access), Sonata MUST
  keep the current version and say why.

**Core behavior parity**

- **FR-011**: Workspaces, canvas, terminals, notes, links, groups, lanes, the usage meter, the
  browser node, and session restore MUST work on Linux with the same data format as macOS.
- **FR-012**: The data folder on Linux MUST be the same location relative to the user's home as
  on macOS, so data can move between systems.
- **FR-013**: The `sonata` command MUST be available inside every Sonata terminal on Linux with
  no user setup, for both the portable file and the Debian package.
- **FR-014**: The messaging channel between the CLI and the app MUST keep the same local-only,
  owner-only access rules on Linux as on macOS.
- **FR-015**: Typing an agent name by hand MUST start it through Sonata in bash and in zsh
  terminals; the user's own startup files for that shell MUST still load and MUST NOT be changed.
- **FR-016**: Messages that name macOS tools (Xcode command line tools, Finder, Homebrew-only
  paths) MUST be replaced on Linux with Linux equivalents (system package tool, file manager).

**Platform-specific features**

- **FR-017**: The browser node on Linux MUST render pages live and MUST keep each workspace's
  browsing data separate and removable, as on macOS.
- **FR-018**: The browser card picture and the agent snapshot command on Linux MUST either work
  or fail with a clear "not available on Linux" message; the card MUST show a placeholder with
  the page title and address when no picture exists.
- **FR-019**: Custom usage providers on Linux MUST run only inside a restricted environment that
  blocks writes outside allowed folders and hides the same sensitive folders as on macOS; if
  that environment is not available, the provider MUST NOT run and MUST show the reason.
- **FR-020**: The device node on Linux MUST support Android emulators and MUST hide iOS options.
- **FR-021**: Opening external links and revealing files MUST use the system default browser and
  file manager on Linux.

**Input and interface**

- **FR-022**: On Linux, every app shortcut that uses Cmd on macOS MUST use Ctrl (with Shift where
  Ctrl alone is a terminal control key), and every hint in menus and tooltips MUST show the Linux
  key names.
- **FR-023**: On Linux, terminal control keys (Ctrl+C, Ctrl+D, Ctrl+Z, Ctrl+W, Ctrl+U, and
  similar) MUST reach the terminal program when a terminal has focus.
- **FR-024**: The interface MUST use fonts that exist on common Linux desktops, falling back to
  the system sans-serif and monospace fonts.

**Build from source and verification**

- **FR-025**: The install script MUST build and install Sonata on Linux for the current user,
  after checking build tools and system libraries and printing missing package names.
- **FR-026**: CI MUST build the Linux app and run the platform-specific tests on a Linux runner
  for every push and pull request.
- **FR-027**: macOS behavior, files, and tests MUST NOT regress; all existing macOS checks MUST
  still pass.
- **FR-028**: The README MUST document the Linux install (portable file and package), the
  supported distributions, required system packages, update behavior per install type, and the
  features that differ on Linux.

### Key Entities

- **Release**: one version with files per platform (macOS disk image and update archive; Linux
  portable file, Debian package, and update signature) and one update manifest.
- **Update manifest**: the list of platforms, each with a download address and signature, read
  by every installed app to find updates.
- **Install type**: how Sonata was installed on Linux (portable file, Debian package, or from
  source); it decides whether the app can update itself.
- **Shell integration**: Sonata's per-shell startup files that make hand-typed agent names start
  through Sonata while loading the user's own startup files.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A friend with a supported Linux desktop goes from the release page to a running
  Sonata terminal with an agent in under 5 minutes, without installing developer tools.
- **SC-002**: 100% of the existing feature areas (canvas, terminals, notes, links, CLI, agent
  messaging, lanes, usage meter, browser node, session restore, updates) work on Linux, or show a
  documented Linux-specific explanation.
- **SC-003**: One tag push produces a release with macOS and Linux files and one manifest, with
  no manual steps.
- **SC-004**: A macOS install of the previous version updates to the new release with no change
  in behavior.
- **SC-005**: The Linux CI job passes on every merge to the main branch, and the macOS checks keep
  passing.
- **SC-006**: A portable Linux install of version N updates to N+1 in under 2 minutes on a normal
  connection, with all workspaces intact.
- **SC-007**: No text that names macOS-only tools or keys appears in the Linux interface.

## Assumptions

- Target architecture is x86_64 only for this feature; arm64 Linux is a later addition.
- Supported baseline is Ubuntu 22.04 or later and distributions of the same age (the WebKitGTK
  4.1 family); both X11 and Wayland sessions are supported.
- Formats are a portable file (self-updating) and a Debian package (updated by the user); other
  package formats (RPM, Flatpak, Snap, AUR) are out of scope.
- The data folder stays `~/.sonata` on Linux, matching the constitution and allowing data to move
  between systems.
- Builds and tests for Linux run on GitHub Actions Linux runners; the maintainer's Mac cannot run
  Linux, so CI logs are the verification record.
- Shell integration covers bash and zsh; other shells start normally without hand-typed agent
  integration.
- The constitution currently limits Sonata to macOS (principle VII and the platform constraints);
  the plan amends it to add Linux. A parallel Windows feature may amend the same text.
- iOS simulators need macOS and are not offered on Linux.
- Paid services (signing certificates, distribution stores) are not used.
