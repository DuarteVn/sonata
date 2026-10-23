# Feature Specification: Release Distribution and Auto-Update

**Feature Branch**: `005-release-distribution`

**Created**: 2026-10-23

**Status**: Draft

## Clarifications

### Session 2026-10-23

- Q: Where are releases hosted? → A: Make the source repository public and publish releases there.

**Input**: User description: "Distribution for Sonata (feature 005). Friends must install Sonata without Rust or Node: a GitHub Actions release workflow on a macOS runner builds a prebuilt universal .dmg (arm64 + x86_64) with the `sonata` CLI bundled, ad-hoc signed (no paid Apple Developer account). The app auto-updates through the Tauri updater: it checks for new releases, tells the user, and installs on confirmation. Docs explain Gatekeeper \"Open Anyway\" and `xattr -dr com.apple.quarantine`. `scripts/install.sh` (build from source) stays, and may offer to install rustup when missing. Open question: the repo is private, so friends need repo access to download releases and macOS Actions minutes count 10x; consider making the repo public."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Friend installs Sonata from a download (Priority: P1)

A friend with a Mac (Apple Silicon or Intel) and no developer tools downloads one disk image
from the project's release page, drags Sonata to Applications, follows the documented
Gatekeeper step once, and starts using the app. The `sonata` command works inside Sonata
terminals with no extra setup.

**Why this priority**: Today the only install path builds from source and needs Rust, Node, and
the Xcode command line tools. Friends cannot use Sonata without this story.

**Independent Test**: On a Mac without Rust, Node, or Xcode tools, download the disk image from
the latest release, install it, open it after the Gatekeeper step, create a terminal, and run
`sonata list`.

**Acceptance Scenarios**:

1. **Given** a Mac with Apple Silicon and no developer tools, **When** the friend downloads the
   disk image from the latest release and drags Sonata to Applications, **Then** Sonata opens
   after the documented Gatekeeper step and all features work.
2. **Given** an Intel Mac, **When** the friend installs the same disk image, **Then** Sonata runs
   natively (not through translation).
3. **Given** a fresh install, **When** the friend opens a terminal in Sonata and runs
   `sonata help`, **Then** the command runs, because it ships inside the app.
4. **Given** macOS blocks the first launch, **When** the friend follows the README section for
   "Open Anyway" (or the one-line quarantine removal command), **Then** the app opens and later
   launches show no prompt.

---

### User Story 2 - Maintainer publishes a release with one action (Priority: P1)

The maintainer marks a version (for example `v0.2.0`) and pushes it. Automation builds the
universal app, signs it ad hoc, packages the disk image, signs the update files with the
project's own update key, and publishes a release with the files and short notes. No paid
service is used.

**Why this priority**: Without automated releases, every update needs a manual build on the
maintainer's Mac, and Story 1 and Story 3 have nothing to download.

**Independent Test**: Push a version tag on a test version; confirm that a release appears with
the disk image, the update archive, its signature, and the update manifest, and that the build
log shows both architectures.

**Acceptance Scenarios**:

1. **Given** a version tag is pushed, **When** the release automation finishes, **Then** a
   release with that version exists and holds the disk image, the update archive, its update
   signature, and an update manifest that points to them.
2. **Given** the version in the tag differs from the app's declared version, **When** the
   automation runs, **Then** it fails before building and says which values differ.
3. **Given** the update signing key secret is missing, **When** the automation runs, **Then** it
   fails with a clear message and publishes nothing.
4. **Given** a normal push or pull request, **When** CI runs, **Then** no release is built (the
   costly macOS build runs only for version tags or a manual trigger).

---

### User Story 3 - App updates itself on confirmation (Priority: P2)

A friend running Sonata learns that a newer version exists. Sonata shows a small, non-blocking
notice with the new version number and its notes. On "Install and restart", Sonata downloads the
update, verifies its signature, replaces itself, and restarts. Live terminals are not killed
without warning.

**Why this priority**: Without it, friends stay on old versions and every fix needs a manual
reinstall. It depends on Story 2.

**Independent Test**: Install version N from a release; publish version N+1; open Sonata N and
confirm that the notice appears, that the install works, and that Sonata N+1 starts with all
workspaces intact.

**Acceptance Scenarios**:

1. **Given** Sonata is open and a newer release exists, **When** the update check runs, **Then**
   Sonata shows a notice with the new version and its notes, and nothing installs yet.
2. **Given** the notice is shown, **When** the user chooses "Later", **Then** the notice closes
   and returns no sooner than the next launch or the next daily check.
3. **Given** live terminals are running, **When** the user chooses "Install and restart",
   **Then** Sonata uses the same quit confirmation it uses on Cmd+Q before it restarts.
4. **Given** the downloaded update has a bad or missing signature, **When** Sonata verifies it,
   **Then** Sonata refuses to install it, keeps the current version, and tells the user.
5. **Given** the Mac is offline or the release server is unreachable, **When** the check runs,
   **Then** nothing is shown to the user and the failure goes only to the log.
6. **Given** the user wants to check now, **When** they choose "Check for Updates…" in the app
   menu, **Then** Sonata checks and reports "up to date", "update available", or the error.

---

### User Story 4 - Build from source stays easy (Priority: P3)

A contributor still installs with `scripts/install.sh`. When Rust is missing, the script offers
to install it with the official installer (only after an explicit "yes"), instead of exiting with
a hint.

**Why this priority**: The constitution keeps build from source as a supported path; this only
removes friction.

**Independent Test**: Run the script on a Mac where `cargo` is not on the path; answer "no" and
confirm it exits with instructions; answer "yes" and confirm it installs Rust and continues.

**Acceptance Scenarios**:

1. **Given** `cargo` is missing, **When** the script runs, **Then** it asks before installing
   anything and does nothing without "yes".
2. **Given** the script runs without an interactive terminal, **When** `cargo` is missing,
   **Then** it does not install anything and exits with instructions.
3. **Given** Node or the Xcode command line tools are missing, **When** the script runs, **Then**
   it exits with the same instructions as today (it does not install them).

### Edge Cases

- A friend installed from source (no update manifest pin of its own) receives an update notice:
  the notice works the same, because the update source is built into every build.
- A friend moves Sonata out of `/Applications` or runs it from the disk image: the update
  install fails without damage, and Sonata says to move the app to Applications.
- An update arrives while a `sonata ask` is waiting: the quit confirmation covers it, because
  restart follows the same path as quit.
- A release has no update manifest (failed upload): the check treats it as "no update".
- The new version needs a newer macOS than the user has: the update is not offered.
- Two releases are published close together: the check offers only the newest one.
- The update downloads partly and the network drops: nothing is replaced; the next check retries.

## Requirements *(mandatory)*

### Functional Requirements

**Release build**

- **FR-001**: The project MUST publish each version as a release that contains one disk image
  that runs natively on both Apple Silicon and Intel Macs with macOS 15 or later.
- **FR-002**: The app inside the disk image MUST contain the `sonata` command, and Sonata
  terminals MUST find it with no user setup.
- **FR-003**: The app MUST be signed ad hoc. The release process MUST NOT require the Apple
  Developer Program, notarization, or any other paid service.
- **FR-004**: A release MUST be produced by pushing a version tag (`vMAJOR.MINOR.PATCH`) or by a
  manual trigger, and by nothing else.
- **FR-005**: The release process MUST stop before building when the tag version and the app's
  declared version differ.
- **FR-006**: The release process MUST run the same checks as CI (format, lint, tests, type
  check) before it publishes.
- **FR-007**: Each release MUST include an update manifest, an update archive, and a signature
  made with the project's own update key. The private key MUST exist only as a repository secret
  and on the maintainer's machine, never in the repository or in build logs.

**Auto-update**

- **FR-008**: Sonata MUST check for a newer release at launch and then once every 24 hours while
  open.
- **FR-009**: When a newer release exists, Sonata MUST show a non-blocking notice with the
  version and release notes, and actions "Install and restart" and "Later".
- **FR-010**: Sonata MUST NOT download or install an update without the user's confirmation.
- **FR-011**: Sonata MUST verify the update signature against the public key built into the app
  and MUST refuse an update that fails verification.
- **FR-012**: Before it restarts for an update, Sonata MUST run the same quit confirmation it
  uses for Cmd+Q, so the user can cancel when terminals are live.
- **FR-013**: The app menu MUST offer "Check for Updates…", which reports the result even when
  no update exists or the check fails.
- **FR-014**: Failed background checks MUST be logged and MUST NOT interrupt the user.
- **FR-015**: User data in `~/.sonata/` MUST stay unchanged by an update (only the app bundle is
  replaced).
- **FR-016**: Releases MUST be published in the project's own repository, which becomes public,
  so every friend can download the disk image and the app can fetch updates with no token or
  account. The maintainer changes the repository visibility by hand; the release process never
  changes it.

**Documentation and build from source**

- **FR-017**: The README MUST document: download and install from a release, the Gatekeeper
  "Open Anyway" step, the alternative `xattr -dr com.apple.quarantine /Applications/Sonata.app`
  command and what it does, how updates work, and the existing security notice.
- **FR-018**: The README MUST document how the maintainer creates the update key, stores it as a
  secret, and cuts a release.
- **FR-019**: `scripts/install.sh` MUST offer to install Rust through the official installer when
  `cargo` is missing, and MUST act only after an explicit "yes" in an interactive terminal.

### Key Entities

- **Release**: one published version; holds the version number, release notes, the disk image,
  the update archive, its signature, and the update manifest.
- **Update manifest**: small public file that names the newest version, its notes, its date, and
  the download address and signature of the update archive.
- **Update key pair**: the private key signs update archives during the release; the public key
  is built into every app and verifies downloads.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A friend with no developer tools goes from the release page to a working Sonata in
  under 5 minutes, using only the README.
- **SC-002**: One disk image runs natively on both Apple Silicon and Intel Macs (verified by
  checking that the app binary and the `sonata` command each contain both architectures).
- **SC-003**: The maintainer publishes a release with one push of a version tag, and the release
  appears in under 30 minutes with no manual upload.
- **SC-004**: A user on version N is offered N+1 within one launch or 24 hours of its release,
  and installing it takes one confirmation and under 1 minute on a normal connection.
- **SC-005**: 100% of tampered or unsigned update archives are refused (verified with a test
  archive with a wrong signature).
- **SC-006**: After an update, 100% of workspaces, notes, and settings load as before.
- **SC-007**: Releasing costs nothing: no paid Apple service and no paid CI plan.

## Assumptions

- Friends use macOS 15 or later, as the rest of Sonata requires.
- Ad-hoc signed apps from the internet are blocked on first launch; one documented Gatekeeper
  step per install is acceptable to the users. Updates installed by the app itself do not need
  that step again.
- The repository becomes public before the first release (decided 2026-10-23). Public
  repositories get free CI minutes, macOS included; macOS builds still run only on version tags
  or manual triggers, to keep pushes fast.
- Release notes come from the tag message or the generated GitHub changelog; no separate notes
  file is needed.
- The update check contacts only the release host and sends no data about the user beyond a
  normal download request.
- Constitution principle VI names build from source as the primary install path. If the disk
  image becomes the recommended path for friends, the plan proposes a PATCH amendment that
  states both paths.
- Out of scope: Homebrew cask, delta updates, beta channels, notarization, Windows and Linux
  builds.
