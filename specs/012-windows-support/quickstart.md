# Quickstart: verifying Windows support

This Mac cannot run Windows. Automated checks run on GitHub Actions; a final manual pass runs
on a real Windows PC before the first public Windows release.

## 1. macOS has not regressed (local)

```sh
sh scripts/check.sh
```

Expected: every check passes, as before the feature.

## 2. Core and CLI on Windows (CI)

Push the branch; open the `ci` run, job `windows`.

Expected: `cargo clippy -p sonata-core -p sonata-cli --all-targets -- -D warnings`,
`cargo test -p sonata-core -p sonata-cli`, and `cargo check -p sonata-app` pass. The log lists
the Windows tests from research R18 (pipe, ACL, shells, PowerShell PTY, ask round trip, run).

## 3. Installer build (CI, manual trigger)

Run the `release` workflow with `workflow_dispatch` in dry-run mode (no publish) on the branch.

Expected: job `build-windows` produces `Sonata_<v>_x64-setup.exe` and its `.sig`; the manifest
step prints a `latest.json` with `darwin-aarch64`, `darwin-x86_64` unchanged and a
`windows-x86_64` entry (see [contracts/release-manifest.md](contracts/release-manifest.md)).

## 4. Manual pass on a Windows PC (before the first Windows release)

1. Download the installer artifact; run it; SmartScreen → "More info" → "Run anyway".
   Expected: installs without an administrator prompt; Start menu entry exists.
2. Open Sonata; add a workspace on `C:\Users\<you>\code\<repo>`; add a terminal.
   Expected: PowerShell prompt in that folder.
3. Type `claude`. Expected: Claude Code starts with Sonata instructions; `$PROFILE` still loaded.
4. Add a second terminal, link the two, run `sonata ask <name> "say hi"`. Expected: answer
   returns.
5. Open Command Prompt outside Sonata; run `sonata help`. Expected: help text (user PATH).
6. Create a lane in the git repo, commit in it, land it. Expected: same confirmations as macOS.
7. Add a browser node, scroll it off screen. Expected: card with page image (or the R13
   message).
8. Press Ctrl+W with a note selected (note closes) and inside a terminal (shell deletes a word).
9. Install an older build, publish a newer test release, accept the update. Expected: new
   version runs; workspaces kept.

## Results

- 2026-10-29: section 1 green (`scripts/check.sh`); section 2 green on GitHub Actions run
  40898348030 (`windows` job: clippy, `cargo check -p sonata-app`, all core and CLI tests,
  including pipe, ACL, PowerShell PTY, busy, `sonata ask`, Job Object, presets).
- Unix-only tests (POSIX shells, modes, Unix sockets, sandbox-exec) are gated with
  `#![cfg(unix)]`; their Windows counterparts are the `*_windows.rs`, `pipe.rs`, and
  `terminal_pwsh.rs` tests.
- 2026-10-29: section 3 green on release run 40899717110 (dry run). The installer is
  downloadable as the `windows` artifact of that run for the manual pass in section 4.
