# Quickstart: Validate Release Distribution and Auto-Update

## Prerequisites (maintainer, once)

1. Repository is public (Settings → General → Danger Zone → Change visibility). Done by hand.
2. Update key: `npm run tauri signer generate -- -w ~/.tauri/sonata.key`; copy the public key into
   `plugins.updater.pubkey`; store secrets with
   `gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/sonata.key` and
   `gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (typed at the prompt).

## Automated checks

```sh
cargo test -p sonata-core update            # schedule + banner state machine
npm test -- UpdateBanner latest-json        # banner UI + manifest generator
sh scripts/release/tests/run.sh             # version gate + install.sh prompt
```

## Scenario A — release (Story 2)

1. Bump the version to `0.2.0` in the three files; commit; `git tag v0.2.0 && git push --tags`.
2. Expect: workflow `release` passes; release `v0.2.0` holds the four assets of
   [contracts/release.md](contracts/release.md); log shows `lipo -archs` → `x86_64 arm64` for the
   app and the CLI; total time < 30 min.
3. Negative: tag `v9.9.9` without bumping → job fails at the version gate with mismatch lines.

## Scenario B — fresh install (Story 1)

1. On a Mac without Rust/Node, download the `.dmg` in a browser, drag Sonata to Applications.
2. Open: blocked → System Settings → Privacy & Security → Open Anyway (or
   `xattr -dr com.apple.quarantine /Applications/Sonata.app`).
3. New terminal → `sonata help` prints help. On Intel: Activity Monitor shows Kind "Intel".

## Scenario C — auto-update (Story 3)

1. Install `v0.2.0`; publish `v0.2.1`.
2. Launch `v0.2.0`: banner at sidebar bottom shows `0.2.1` and notes. Hide sidebar → dot on the
   show-sidebar button.
3. "Later" → banner hides; relaunch → banner returns.
4. Open a live terminal; "Install and restart" → progress → quit dialog lists the terminal →
   confirm → app restarts as `0.2.1`; workspaces intact (SC-006).
5. Menu → Check for Updates… → "Sonata is up to date".
6. Toggle off "Check for Updates Automatically"; relaunch with a newer release present → no
   banner; `~/.sonata/preferences.json` has `"autoUpdateCheck": false`.
7. Tamper test: serve a manifest whose signature is from another key (local endpoint override in
   a debug build) → banner shows the signature failure; app unchanged (SC-005).
8. Run Sonata from the mounted disk image → banner says to move Sonata to Applications.

## Scenario D — build from source (Story 4)

1. `PATH` without `cargo`: `scripts/install.sh` → prompt; `n` → instructions, exit 1.
2. `echo | scripts/install.sh` (no terminal) → no prompt, instructions, exit 1.
3. With Rust present: script builds and installs without needing the update key.
