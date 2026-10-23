# Research: Release Distribution and Auto-Update

## R1. Universal app build

- **Decision**: Build with `tauri build --target universal-apple-darwin --bundles app,dmg` on a
  GitHub-hosted macOS arm64 runner, after `rustup target add aarch64-apple-darwin
  x86_64-apple-darwin`. Tauri compiles both targets and joins them with `lipo`.
- **Rationale**: One disk image for both Mac kinds (FR-001); constitution VI already names
  `universal-apple-darwin`. One job is cheaper than two jobs plus a merge step.
- **Alternatives**: Two disk images (one per architecture) — friends must pick the right one,
  and the updater needs two archives. Rejected.

## R2. Universal `sonata` CLI inside the bundle

- **Decision**: Replace the inline `build:tauri` npm script with `scripts/build-cli.sh`. The
  script reads `TAURI_ENV_TARGET_TRIPLE` (set by the Tauri CLI for `beforeBuildCommand`). For
  `universal-apple-darwin` it builds `sonata-cli` for both targets and joins them with
  `lipo -create`; otherwise it builds for the host as today. Output goes to
  `src-tauri/resources/bin/sonata`, which `bundle.resources` already copies to
  `Contents/Resources/bin/`.
- **Rationale**: Without this, the bundle holds a host-only CLI, and `sonata` fails on the other
  architecture (FR-002, SC-002). Reading the env var keeps local `install.sh` builds unchanged.
- **Verification**: the release workflow runs `lipo -archs` on both the app binary and the CLI
  and fails unless both print `x86_64 arm64`.
- **Fallback**: if the env var is absent in some Tauri version, the release workflow sets
  `SONATA_UNIVERSAL=1`, which the script also honors.

## R3. Ad-hoc signing and Gatekeeper on macOS 15

- **Decision**: Keep `signingIdentity: "-"` (already in `tauri.conf.json`). Tauri signs the
  universal bundle ad hoc, nested CLI included. No notarization.
- **Findings**: A browser download sets the quarantine attribute. On macOS 15, an ad-hoc signed,
  non-notarized app is blocked on first launch; the Control-click → Open bypass no longer exists.
  The user opens System Settings → Privacy & Security → "Open Anyway", or runs
  `xattr -dr com.apple.quarantine /Applications/Sonata.app`. Files the updater downloads itself
  get no quarantine attribute, so updates open without the prompt.
- **Rationale**: FR-003, constitution VI (zero cost).

## R4. Updater mechanism

- **Decision**: `tauri-plugin-updater` (v2), called only from Rust. Config:
  `bundle.createUpdaterArtifacts: true`; `plugins.updater.pubkey` = project public key;
  `plugins.updater.endpoints` =
  `https://github.com/DuarteVn/sonata/releases/latest/download/latest.json`.
  The plugin checks the manifest, compares versions (semver), downloads, verifies the minisign
  signature against the built-in public key, and replaces the bundle.
- **Rationale**: Official plugin; signature check satisfies FR-011 and SC-005 without our own
  crypto. The `releases/latest/download/` URL always points at the newest non-draft,
  non-prerelease release (edge case "two releases close together").
- **Flow** (FR-010, FR-012): `check` → banner → user picks "Install and restart" → `download`
  with progress (nothing killed yet; failure keeps the current app) → existing quit check
  (live terminals, waiting asks) → `install` → `app.restart()`.
- **Alternatives**: Sparkle — needs an Objective-C framework, EdDSA keys, appcast hosting; more
  moving parts for the same result. Rejected.

## R5. Manifest and release publishing

- **Decision**: Own workflow steps, no third-party release action:
  1. `gh release create vX.Y.Z --generate-notes --verify-tag` with the disk image, the
     `Sonata.app.tar.gz` update archive, and its `.sig`.
  2. `scripts/release/latest-json.mjs` builds `latest.json` from the version, the release body
     (`gh release view --json body`), the date, the signature file contents, and the archive URL,
     for both `darwin-aarch64` and `darwin-x86_64` (same universal archive).
  3. `gh release upload vX.Y.Z latest.json`.
- **Rationale**: The release token stays in first-party steps (`gh` is preinstalled on runners);
  the manifest generator is a small pure function with unit tests (constitution III).
- **Alternatives**: `tauri-apps/tauri-action` — less code, but a third-party action receives a
  write token, and its manifest logic is not testable here. Rejected for supply-chain reasons.

## R6. Release triggers, checks, cost

- **Decision**: `.github/workflows/release.yml` runs on `push: tags: ['v*.*.*']` and
  `workflow_dispatch` (input: tag). Jobs: `checks` reuses `ci.yml` via `workflow_call` (Linux,
  free); `release` (`macos-14` or newer, `needs: checks`) does R1–R5. `ci.yml` gains a
  `workflow_call` trigger and is otherwise unchanged.
- **Version gate** (FR-005): `scripts/release/check-version.sh vX.Y.Z` compares the tag with
  `src-tauri/tauri.conf.json`, the workspace `Cargo.toml`, and `package.json`, and prints each
  mismatch. It runs first in the `release` job.
- **Secret gate** (Story 2, scenario 3): the job fails early when
  `TAURI_SIGNING_PRIVATE_KEY` is empty. Secrets go only through `env`, never arguments
  (constitution V).
- **Cost**: the repository becomes public, so GitHub-hosted runners are free (SC-007).

## R7. Update key handling

- **Decision**: The maintainer runs `npm run tauri signer generate -- -w ~/.tauri/sonata.key`
  once. The public key goes into `tauri.conf.json` (committed). The private key and its password
  go into repository secrets `TAURI_SIGNING_PRIVATE_KEY` and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` with `gh secret set` (stdin). The README documents this
  and warns that losing the key strands existing installs on the old key.
- **Local builds**: `install.sh` builds with `createUpdaterArtifacts` on; without the private key
  the Tauri CLI would fail to sign the archive. `install.sh` passes
  `--config '{"bundle":{"createUpdaterArtifacts":false}}'` so contributors need no key. Source
  builds still contain the public key, so they get update offers (spec edge case).

## R8. Where update logic lives (constitution II)

- **Decision**: `crates/core` owns the rules: the `autoUpdateCheck` preference, the schedule
  (`UpdateSchedule::due(now)` — at launch, then every 24 h, never while disabled, "Later" hides
  the banner until the next due check), and the banner state machine. `src-tauri` owns only the
  plugin calls (`check`, `download`, `install`, `restart`) and the menu items, and forwards
  results to core. The frontend renders the banner from events.
- **Rationale**: Schedule and state are unit-testable without Tauri.

## R9. Install location check

- **Decision**: Before `install`, the shell checks the running bundle path. If it is under
  `/Volumes/` (running from the disk image) or contains `/AppTranslocation/` (quarantined app
  moved by macOS), the banner shows "Move Sonata to Applications to update" and does not
  download.
- **Rationale**: The updater cannot replace a read-only or translocated bundle (spec edge case).

## R10. `install.sh` and Rust

- **Decision**: When `cargo` is missing and stdin is a terminal (`[ -t 0 ]`), ask
  "Install Rust with rustup (https://rustup.rs)? [y/N]". On `y`, run the official installer
  (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y`), then source
  `~/.cargo/env` and continue. Otherwise print instructions and exit 1. Node and Xcode tools keep
  today's messages.
- **Rationale**: FR-019 and Story 4. Only Rust is offered because rustup is the single
  official, user-local installer; Node has several (Homebrew, nvm, installer) and Xcode tools
  need a system dialog.

## R11. Constitution principle VI

- **Decision**: PATCH amendment 1.1.0 → 1.1.1 in this feature: "Two install paths: a prebuilt
  universal disk image from GitHub Releases (recommended for users) and a script that builds
  from source (contributors)". The `gh`-download wording becomes "GitHub Release".
- **Rationale**: The meaning (zero cost, ad-hoc, own updater key) stays; only which path is
  recommended changes. Recorded in the Sync Impact Report.
