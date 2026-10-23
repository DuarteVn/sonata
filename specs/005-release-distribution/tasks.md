---

description: "Task list for feature 005: release distribution and auto-update"
---

# Tasks: Release Distribution and Auto-Update

**Input**: Design documents from `specs/005-release-distribution/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ipc.md, contracts/release.md, quickstart.md

**Tests**: Required. Constitution III (test-first) applies: each test task is written and seen
failing before its implementation task.

**Organization**: by user story. Story order follows dependency: US1 (universal artifact) →
US2 (release automation) → US3 (auto-update) ; US4 (install.sh) is independent.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [X] T001 Add `tauri-plugin-updater = "2"` to `[workspace.dependencies]` in `Cargo.toml` and to `src-tauri/Cargo.toml`; run `cargo build -p sonata-app` to confirm it resolves
- [X] T002 [P] Create `scripts/release/tests/run.sh`: a POSIX sh runner that executes every `scripts/release/tests/*.test.sh`, prints PASS/FAIL per file, and exits non-zero on any failure
- [X] T003 [P] Add `scripts/release/` and `scripts/release/tests/run.sh` invocation to `scripts/check.sh` so local checks run the shell tests

---

## Phase 2: Foundational (blocks US3)

- [X] T004 Write failing tests in `crates/core/tests/preferences_update.rs`: `preferences.json` without `autoUpdateCheck` loads as `true`; `false` round-trips; `true` is omitted on write ("Missing field reads as `true` (serde default), so old files load unchanged. Written only when `false`")
- [X] T005 Add `auto_update_check: bool` (serde default `true`, skip when `true`) to `Preferences` in `crates/core/src/model/preferences.rs`; keep `PREFERENCES_SCHEMA_VERSION = 1`; make T004 pass

---

## Phase 3: User Story 1 - Friend installs Sonata from a download (P1) 🎯 MVP

**Goal**: one universal, ad-hoc signed disk image with a universal `sonata` CLI inside.

**Independent Test**: `npm run tauri build -- --target universal-apple-darwin --bundles app,dmg` locally; `lipo -archs` on `Sonata.app/Contents/MacOS/sonata-app` and `Contents/Resources/bin/sonata` both print `x86_64 arm64`; `codesign -dv` shows `Signature=adhoc`; install from the `.dmg` and run `sonata help` in a Sonata terminal.

### Tests

- [X] T006 [P] [US1] Write `scripts/release/tests/build-cli.test.sh`: with stub `cargo` and `lipo` on `PATH`, assert `scripts/build-cli.sh` builds host only when `TAURI_ENV_TARGET_TRIPLE` is unset, builds `aarch64-apple-darwin` and `x86_64-apple-darwin` and calls `lipo -create` when it is `universal-apple-darwin` or `SONATA_UNIVERSAL=1`, and writes `src-tauri/resources/bin/sonata`

### Implementation

- [X] T007 [US1] Create `scripts/build-cli.sh` per research R2 (host or universal `sonata-cli`, output `src-tauri/resources/bin/sonata`, mode 0755); make T006 pass
- [X] T008 [US1] Change `build:tauri` in `package.json` to `sh scripts/build-cli.sh && npm run build`; keep `dev:tauri` unchanged
- [X] T009 [US1] Run the universal build locally (`rustup target add aarch64-apple-darwin x86_64-apple-darwin` first); verify archs and `codesign -dv --verbose=2` ad-hoc signature for app and CLI; record the commands in `specs/005-release-distribution/quickstart.md` if they differ
- [X] T010 [US1] README `## Install` in `README.md`: download the `.dmg` from the latest release, drag to Applications, first launch on macOS 15 (System Settings → Privacy & Security → Open Anyway), alternative `xattr -dr com.apple.quarantine /Applications/Sonata.app` with one sentence on what it removes and why it is safe only for a download you trust; keep "Build from source" as a second section

**Checkpoint**: a hand-built universal `.dmg` installs and runs on both architectures.

---

## Phase 4: User Story 2 - Maintainer publishes a release with one action (P1)

**Goal**: tag push → checks → universal build → signed update archive → release with 4 assets.

**Independent Test**: quickstart Scenario A, including the `v9.9.9` negative case.

### Tests

- [X] T011 [P] [US2] Write `scripts/release/tests/check-version.test.sh` with temp copies of the three version files: all match → exit 0; one differs → exit 1 and the line `version mismatch: tag 0.2.0, src-tauri/tauri.conf.json 0.1.0`; tag `0.2` or `v0.2` → exit 2 (contracts/release.md "Version gate")
- [X] T012 [P] [US2] Write `ui/tests/latestJson.test.ts` for `buildLatestJson({version, notes, sig, url, date})` from `scripts/release/latest-json.mjs`: strips leading `v`; both `darwin-aarch64` and `darwin-x86_64` carry the same trimmed signature and URL; `pub_date` RFC 3339 UTC; throws on empty signature and on non-https URL

### Implementation

- [X] T013 [P] [US2] Create `scripts/release/check-version.sh <tag>` (reads `src-tauri/tauri.conf.json` `version`, `Cargo.toml` `[workspace.package].version`, `package.json` `version`; exit codes 0/1/2); make T011 pass
- [X] T014 [P] [US2] Create `scripts/release/latest-json.mjs`: export `buildLatestJson`; CLI `--version --notes-file --sig-file --url [--date]` prints JSON to stdout; make T012 pass
- [X] T015 [US2] Add `workflow_call:` to the `on:` block of `.github/workflows/ci.yml` (no other change)
- [X] T016 [US2] Create `.github/workflows/release.yml` per research R5/R6 and contracts/release.md: triggers `push.tags: ['v*.*.*']` and `workflow_dispatch` (input `tag`); job `checks` uses `./.github/workflows/ci.yml`; job `release` on `macos-14`, `needs: checks`, `permissions: contents: write`, steps: checkout tag → `check-version.sh` → fail if `TAURI_SIGNING_PRIVATE_KEY` empty (message from contract) → rust toolchain with both darwin targets + rust-cache → node 22 + `npm ci` → `npm run tauri build -- --target universal-apple-darwin --bundles app,dmg` with `SONATA_UNIVERSAL=1` and signing secrets in `env` → `lipo -archs` gate on app and CLI → `gh release create "$TAG" --verify-tag --generate-notes <dmg> <tar.gz> <sig>` → `gh release view --json body` → `latest-json.mjs` → `gh release upload "$TAG" latest.json`
- [X] T017 [US2] Add `bundle.createUpdaterArtifacts: true` and `plugins.updater` (`pubkey` placeholder `REPLACE_WITH_PUBLIC_KEY`, endpoint from contracts/release.md) to `src-tauri/tauri.conf.json`
- [X] T018 [US2] README `## Releasing` in `README.md`: generate the key (`npm run tauri signer generate -- -w ~/.tauri/sonata.key`), paste the public key into `tauri.conf.json`, `gh secret set` both secrets via stdin, bump the three versions, tag and push; warn that losing the private key strands installed apps
- [ ] T019 [US2] Maintainer step (needs the user): make the repository public, generate the key, set the secrets, replace the pubkey placeholder; then cut `v0.2.0` and run quickstart Scenario A

**Checkpoint**: a pushed tag produces a complete release with no manual upload.

---

## Phase 5: User Story 3 - App updates itself on confirmation (P2)

**Goal**: check at launch and daily, sidebar banner, download → quit check → install → restart.

**Independent Test**: quickstart Scenario C.

### Tests

- [X] T020 [P] [US3] Write `crates/core/tests/update_schedule.rs`: `due` is true at launch, false within 24 h of `last_check`, true after 24 h, always false when disabled; a manual check does not change `last_check`
- [X] T021 [P] [US3] Write `crates/core/tests/update_state.rs` covering every transition in data-model.md "UpdateState": available → dismissed → available on next due check; downloading → readyToInstall; downloading → failed keeps version; newer available replaces older; background failure leaves state unchanged; `pending` true for available, downloading, readyToInstall, failed, blocked
- [X] T022 [P] [US3] Write `ui/tests/updateBanner.test.tsx`: renders version and "Install and restart"/"Later" for `available`; progress for `downloading`; "Restart to update" for `readyToInstall`; reason for `failed` and `blocked`; nothing for `idle` and `dismissed`; "Later" calls `update_dismiss`
- [X] T023 [P] [US3] Extend `ui/tests/sidebar.test.tsx` (or the App-level test that renders the show-sidebar button): dot visible when the sidebar is hidden and update state is pending; absent otherwise

### Implementation

- [X] T024 [US3] Create `crates/core/src/update.rs` with `UpdateSchedule` and `UpdateState` (serde `kind` tag, camelCase, matching contracts/ipc.md `UpdateState`); export from `crates/core/src/lib.rs`; make T020 and T021 pass
- [X] T025 [US3] Hold `UpdateSchedule` and `UpdateState` in the core app facade (`crates/core/src/app.rs`), seeded from `preferences.auto_update_check`; add `set_auto_update_check(bool)` that saves preferences atomically and emits the new state
- [X] T026 [US3] Create `src-tauri/src/commands/update.rs`: commands `update_state`, `update_check {manual}`, `update_dismiss`, `update_download`, `update_install`, `update_set_auto_check {enabled}` using `tauri_plugin_updater::UpdaterExt`; emit `update://state` and `update://progress`; location check per research R9 (`/Volumes/` or `/AppTranslocation/` → `blocked` "Move Sonata to Applications to update"); `update_install` calls `install` then `app.restart()`; register the plugin in `src-tauri/src/lib.rs`
- [X] T027 [US3] Background ticker in `src-tauri/src/lib.rs`: every 10 minutes ask core `due(now)`; on due, check silently; log failures with `tracing` only (FR-014)
- [X] T028 [US3] Add the six `update_*` commands to `COMMANDS` in `src-tauri/build.rs` and `allow-update-*` permissions to `src-tauri/capabilities/default.json`
- [X] T029 [US3] Menu in `src-tauri/src/menu.rs`: "Check for Updates…" (emits `app://check-updates`) and `CheckMenuItem` "Check for Updates Automatically" (calls core `set_auto_update_check`), both in the Sonata app menu after About
- [X] T030 [P] [US3] Add types, commands, and events from contracts/ipc.md to `ui/src/ipc/types.ts`, `ui/src/ipc/commands.ts`, `ui/src/ipc/events.ts`
- [X] T031 [US3] Create `ui/src/state/update.ts` (zustand slice fed by `update://state` and `update://progress`, initial `update_state`)
- [X] T032 [US3] Create `ui/src/sidebar/UpdateBanner.tsx` and mount it at the bottom of `ui/src/sidebar/Sidebar.tsx`; styles in `ui/src/styles.css`; make T022 pass
- [X] T033 [US3] In `ui/src/App.tsx`: dot on the show-sidebar button when pending (make T023 pass); "Install and restart" flow = `update_download` → when `readyToInstall`, run the existing `app_quit_check` / `QuitDialog` path → on confirm `update_install`; on cancel keep `readyToInstall`
- [X] T034 [US3] Handle `app://check-updates` in `ui/src/App.tsx`: call `update_check {manual: true}` and show the result ("Sonata is up to date", banner, or the error) through the existing notice mechanism in `ui/src/Banners.tsx`
- [ ] T035 [US3] Run quickstart Scenario C end to end with two real releases (needs T019)

**Checkpoint**: installed apps find, download, verify, and install new releases on confirmation.

---

## Phase 6: User Story 4 - Build from source stays easy (P3)

**Goal**: `install.sh` offers rustup; local builds need no update key.

**Independent Test**: quickstart Scenario D.

- [X] T036 [P] [US4] Write `scripts/release/tests/install-prompt.test.sh`: with `PATH` lacking `cargo` and stubbed `curl`, answer `n` → exit 1 with instructions and no `curl` call; answer `y` (via a pseudo-terminal helper or `SONATA_ASSUME_TTY=1` test hook) → stub installer runs; stdin not a terminal → no prompt, exit 1
- [X] T037 [US4] Update `scripts/install.sh` per research R10 (prompt only when `[ -t 0 ]`; official rustup command; source `~/.cargo/env`), and add `--config '{"bundle":{"createUpdaterArtifacts":false}}'` to its `tauri build` call (research R7); make T036 pass

---

## Phase 7: Polish & Cross-Cutting

- [X] T038 [P] Constitution PATCH 1.1.0 → 1.1.1 in `.specify/memory/constitution.md` per research R11, with Sync Impact Report entry
- [X] T039 [P] README `## Updates` in `README.md`: automatic check at launch and daily, the menu toggle, the banner, data untouched, what the check contacts
- [X] T040 Bump version to `0.2.0` in `src-tauri/tauri.conf.json`, `Cargo.toml`, `package.json` (first release); run `scripts/release/check-version.sh v0.2.0`
- [X] T041 Run all gates: `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `npm run typecheck`, `npm test`, `sh scripts/release/tests/run.sh`
- [ ] T042 Update memory/state notes and open the PR; leave the local repo on `main`

---

## Dependencies & Execution Order

- Setup (T001–T003) → Foundational (T004–T005).
- US1 (T006–T010) needs Setup only. US2 needs US1's `build-cli.sh` (T007) for the workflow build.
- US3 code (T020–T034) needs Foundational and T017 (updater config); its end-to-end check T035
  needs real releases (T019).
- US4 (T036–T037) is independent of all other stories.
- Polish last; T040 before the first tag in T019.

## Parallel Opportunities

- T002, T003 together. T006, T011, T012, T020–T023, T036 are independent test files.
- T013 and T014 in parallel; T030 in parallel with T024–T029.
- US4 can run at any time alongside US1–US3.

## Implementation Strategy

1. MVP: Setup + Foundational + US1 → a hand-built universal `.dmg` friends can install.
2. US2 → automated releases (needs the maintainer step T019: public repo, key, secrets).
3. US3 → auto-update, verified across two real releases.
4. US4 and Polish → PR.
