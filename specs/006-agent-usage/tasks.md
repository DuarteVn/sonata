---

description: "Task list for feature 006: agent usage meter"
---

# Tasks: Agent Usage Meter

**Input**: Design documents from `specs/006-agent-usage/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ipc.md,
contracts/usage-provider.md, quickstart.md

**Tests**: Required. Constitution III (test-first) applies: each test task is written and seen
failing before its implementation task. No test runs the real `claude` or `codex`: tests use
recorded fixtures and fake executables written into a temp dir and put first on `PATH`.

**Organization**: by user story, in priority order: US1 (P1) → US2 (P2) → US4 (P2) → US3 (P3) →
US5 (P3). US2 and US4 both need US1's service; US3 and US5 only need the foundational phase plus
US1's service wiring.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [X] T001 Create module folder `crates/core/src/usage/` with `mod.rs` declaring submodules `window`, `schedule`, `claude`, `codex`, `custom`, `runner`, `consent`, `cache`, `detect`, `service` (empty files with a one-line `//!` doc each) and add `pub mod usage;` to `crates/core/src/lib.rs`; `cargo build -p sonata-core` passes
- [X] T002 [P] Create fixture folder `crates/core/tests/fixtures/usage/` and record fixtures: `claude-auth-team.json`, `claude-auth-logged-out.json`, `claude-auth-api-key.json` (shapes from research R1, with fake email `user@example.com` and fake org), `claude-stream-ok.jsonl` (system init line, assistant line, `rate_limit_event` with `unifiedWindows.five_hour {utilization 0.25, resetsAt 1792767000}` and `seven_day {utilization 0.04, resetsAt 1793343600}`, result line), `claude-stream-no-limits.jsonl` (no `rate_limit_event`), `claude-stream-extra-window.jsonl` (adds `seven_day_sonnet`)
- [X] T003 [P] Add shared test helper `crates/core/tests/common/fake_bin.rs` (and `mod fake_bin;` in `common/mod.rs`): `fake_bin(dir, name, script) -> PathBuf` writes an executable `#!/bin/sh` script with mode 0755 and returns its path

---

## Phase 2: Foundational (blocks all stories)

**Purpose**: types, process runner, detection, persistence, consent, schedule, service skeleton,
and IPC plumbing that every story uses.

### Tests

- [X] T004 [P] Write failing tests in `crates/core/tests/usage_types.rs` for `usage::Window` and `usage::Reading` serde (camelCase, RFC 3339 `resetsAt`/`readAt`); `Reading.accountEmail` is serialized for IPC but skipped by the cache type; window validation: "`usedPercent` finite and ≥ 0; drop windows whose `resetsAt` is in the past"
- [X] T005 [P] Write failing tests in `crates/core/tests/usage_runner.rs` using `fake_bin`: argv passed without a shell (an argument with spaces and `$HOME` arrives literally); cwd is a fresh empty dir; env holds only `HOME`, `USER`, `LOGNAME`, `LANG`, `TMPDIR`, `PATH` (a test var `SONATA_TEST_SECRET` set in the parent is absent); a script that runs `sleep 30 & sleep 30` is killed at the timeout and its child is gone (check with `kill -0` on the recorded pid); stdout over 64 KiB → `RunError::OutputTooLarge`; non-zero exit → `RunError::Exit(code)` with stderr capped at 4 KiB; stdout read concurrently (a script printing 200 KiB to stderr does not deadlock)
- [X] T006 [P] Write failing tests in `crates/core/tests/usage_detect.rs`: with a fake login shell script that prints known paths, `detect::detect_commands(shell, &["claude","codex"], timeout)` returns `found` with full path for present commands, `notFound` for absent, `unknown` on timeout; also returns the login shell `PATH`
- [X] T007 [P] Write failing tests in `crates/core/tests/usage_preferences.rs`: `preferences.json` without `usage` loads with `enabled = true`, `warnings = true`, empty `providers`; `enabled`/`warnings` written only when false; provider entry `{enabled?, color?, consent?}` round-trips; empty entries omitted; `PREFERENCES_SCHEMA_VERSION` unchanged
- [X] T008 [P] Write failing tests in `crates/core/tests/usage_consent.rs`: states `unasked | granted { approvedCommand } | denied`; `is_granted_for(argv)` true only on exact argv equality; `denied` forces the provider switch off; granting stores the argv
- [X] T009 [P] Write failing tests in `crates/core/tests/usage_cache.rs`: envelope `{"type":"usageCache","schemaVersion":1,"payload":{readings,warnings}}`; atomic save then load round-trips; only `ok`, `noPlanLimits`, `notLoggedIn` readings are stored; saved file contains no `accountEmail` and no `@`; a corrupt file loads as empty and is reported (returned error) and is renamed to `usage-cache.json.bad` on the next save, not overwritten in place
- [X] T010 [P] Write failing tests in `crates/core/tests/usage_schedule.rs` (data-model "Schedule"): automatic due after 5 min for `claude` and 1 min for others; panel-open due only when cached reading older than 1 min; manual always due; "no automatic or panel-open run within 1 min of the previous attempt"; a window whose `resetsAt` passed makes its provider due; master switch off → nothing due

### Implementation

- [X] T011 Implement types in `crates/core/src/usage/mod.rs`: `ProviderId`, `ProviderKind { BuiltIn, Custom }`, `Detection { Found, NotFound, Unknown }`, `WindowKind { Short, Weekly, Other }`, `Window { kind, label, used_percent, resets_at }`, `ReadingStatus { Ok, NoPlanLimits, NotLoggedIn, NotInstalled, Unavailable, Off }`, `Reading { provider_id, status, plan, account, account_email, windows, read_at, message }` with validation per data-model; make T004 pass
- [X] T012 Implement `crates/core/src/usage/runner.rs` per contracts/usage-provider.md "Runner rules": `RunSpec { argv, env_path, extra_env, timeout, stdout_cap, sandbox: Option<String>, stdin: Option<Vec<u8>> }`, `run(spec) -> Result<Output, RunError>`; `Command::new(argv[0])`, `env_clear()` + allowlist, `current_dir(tempdir)`, `process_group(0)`, reader threads for stdout/stderr, `libc::killpg(pgid, SIGKILL)` on timeout; stderr redaction of token-like strings (`[A-Za-z0-9_-]{32,}` → `[redacted]`); make T005 pass
- [X] T013 Implement `crates/core/src/usage/detect.rs`: one login-shell call `shell -l -c 'for p in …; do command -v "$p" || echo "!$p"; done; echo "PATH=$PATH"'` with timeout (reuse the approach of `presets::detect`, but read stdout on a thread); return `HashMap<ProviderId, (Detection, Option<PathBuf>)>` and the `PATH`; make T006 pass
- [X] T014 Add `usage: UsagePreferences { enabled, warnings, providers: BTreeMap<String, ProviderPrefs { enabled: Option<bool>, color: Option<String>, consent: Option<Consent> }> }` to `Preferences` in `crates/core/src/model/preferences.rs` (serde default, skip when default); make T007 pass
- [X] T015 Implement `crates/core/src/usage/consent.rs` (`Consent` enum, `is_granted_for`) ; make T008 pass
- [X] T016 Add `usage_cache_file()` (`<root>/usage-cache.json`) and `usage_providers_dir()` (`<root>/usage-providers`, created 0700 on demand) to `DataRoot` in `crates/core/src/paths.rs`; implement `crates/core/src/usage/cache.rs` with the existing envelope and atomic write helpers in `crates/core/src/persistence/`; make T009 pass
- [X] T017 Implement `crates/core/src/usage/schedule.rs` (`UsageSchedule` per provider: `last_attempt`, `due(now, reason, cached_read_at, windows)`; constants `CLAUDE_INTERVAL = 5 min`, `DEFAULT_INTERVAL = 1 min`, `PANEL_FRESH = 1 min`, `FLOOR = 1 min`); make T010 pass
- [X] T018 Write failing test then implement `UsageService` skeleton in `crates/core/src/usage/service.rs` (test `crates/core/tests/usage_service.rs`): holds providers registry (built-ins `claude` "Claude Code" `#d97757`, `codex` "Codex" `#10a37f`), prefs, cache, schedule, in-flight set; `state() -> UsageState` per contracts/ipc.md types; `set_enabled`, `set_provider_enabled` (enabling without consent leaves `consent: unasked` and queues a `ConsentRequest`), `set_provider_color` (validates `#rrggbb`), `answer_consent(id, allow, command)` (fails `consent_mismatch` when `command` differs); `refresh(reason, provider)` dispatches to a `ProviderReader` trait object per provider (fake reader in tests) and never runs two reads of one provider at once; readers run outside the service lock
- [X] T019 Add `CoreEvent::UsageState(UsageState)` (`usage://state`), `CoreEvent::UsageWarning(UsageWarning)` (`usage://warning`), `CoreEvent::UsageConsent(ConsentRequest)` (`usage://consent`) to `crates/core/src/events.rs`; extend the event name/payload tests in `crates/core/tests/` accordingly (failing first)
- [X] T020 Own `UsageService` in `App` (`crates/core/src/app.rs`): created in `start_with` from preferences and cache; methods `usage_state`, `usage_refresh(reason, provider)`, `usage_set_enabled`, `usage_set_provider_enabled`, `usage_set_provider_color`, `usage_answer_consent`, `usage_tick(now)` (runs due automatic reads); every change saves preferences/cache and emits `usage://state`
- [X] T021 Create `src-tauri/src/commands/usage.rs` with `usage_state`, `usage_refresh`, `usage_set_enabled`, `usage_answer_consent` (thin, `spawn_blocking` for refresh); register in `src-tauri/src/commands/mod.rs` and `src-tauri/src/lib.rs` invoke handler; add names to `COMMANDS` in `src-tauri/build.rs` and `allow-usage-*` entries to `src-tauri/capabilities/default.json`
- [X] T022 Add `spawn_usage_loop(core)` in `src-tauri/src/lib.rs`: a dedicated thread that calls `core.usage_tick(Instant::now())` every 15 s (reads themselves run on a separate worker so terminals are never blocked, SC-004), started after bootstrap
- [X] T023 [P] Add TS types `UsageState`, `ProviderState`, `Reading`, `UsageSummary`, `UsageWarning`, `ConsentRequest` (exactly as contracts/ipc.md) to `ui/src/ipc/types.ts`; commands to `ui/src/ipc/commands.ts`; events `usage://state`, `usage://warning`, `usage://consent` to `ui/src/ipc/events.ts`
- [X] T024 [P] Create zustand slice `ui/src/state/usage.ts` fed by `usage://state` and an initial `usage_state` call

**Checkpoint**: `cargo test --workspace` green; the app starts with an empty usage state and no reads.

---

## Phase 3: User Story 1 - See how much of my plan limits I used (P1) 🎯 MVP

**Goal**: Usage button and panel show Claude Code plan, 5-hour and weekly percentages with reset
times, refresh, and all non-ok states; first read is gated by consent.

**Independent Test**: quickstart scenarios 1–4; panel values match `/usage` in Claude Code
within 1 point and 1 minute.

### Tests

- [X] T025 [P] [US1] Write failing tests in `crates/core/tests/usage_claude.rs` for `claude::parse_auth_status`: team fixture → plan "Team", account org name, email kept in memory; logged-out → `NotLoggedIn` with message "Run `claude auth login` in a terminal."; API key / non-`firstParty` → `NoPlanLimits` and `needs_probe = false`
- [X] T026 [P] [US1] Write failing tests in `crates/core/tests/usage_claude.rs` for `claude::parse_probe`: ok fixture → windows short "5-hour" 25.0 and weekly "Weekly" 4.0 with `resetsAt` from epoch; "`usedPercent = utilization × 100` (rounded to 0.1)"; last `rate_limit_event` wins; `seven_day_sonnet` → `other` "Weekly (Sonnet)"; unknown keys ignored; no event + exit 0 → `NoPlanLimits`; garbage → `Unavailable`
- [X] T027 [P] [US1] Write failing test in `crates/core/tests/usage_claude.rs` for `ClaudeReader` with a fake `claude` on `PATH` that answers `auth status --json` and the probe from fixtures: the probe argv equals `claude::PROBE_ARGS` exactly (fake script records `$@`), it runs in an empty dir, and the result is the combined `Reading`
- [X] T028 [P] [US1] Write failing tests in `crates/core/tests/usage_stale.rs`: `ok` reading older than 15 min → `stale = true` in `ProviderState`; failed refresh keeps the last good reading and sets `lastError`; `notInstalled` when detection says `notFound`
- [X] T029 [P] [US1] Write failing vitest `ui/tests/usagePanel.test.tsx`: renders plan, account, both windows with percent and "resets in 2 h 10 min"; renders states "Not logged in" (with hint, no percentages), "No plan limits", "Usage not available" (with reason), "Stale · last read 10:42", "Limit reached"; empty state lists readable agents ("Claude Code, Codex") when nothing is installed (US1-4); refresh button calls `usage_refresh {reason: "manual"}` and shows a spinner while `refreshing`
- [X] T030 [P] [US1] Write failing vitest `ui/tests/usageConsent.test.tsx`: dialog shows provider name, the exact command joined with shell-style quoting, the explanation; "Allow" and "Don't allow" call `usage_answer_consent` with the same `command` array

### Implementation

- [X] T031 [US1] Implement `crates/core/src/usage/claude.rs`: `PROBE_ARGS` constant (contracts/usage-provider.md), `parse_auth_status`, `parse_probe`, `ClaudeReader` (auth status then probe through `runner`, 20 s timeout each, temp cwd); make T025–T027 pass
- [X] T032 [US1] Wire stale rule, `lastError`, and `notInstalled` into `UsageService::state()` in `crates/core/src/usage/service.rs`; make T028 pass
- [X] T033 [US1] Emit `usage://consent` from `UsageService` when a readable-except-consent provider is due (at start, master switch on, provider switched on), once per provider per app run until answered
- [X] T034 [P] [US1] Create `ui/src/usage/meter.ts`: pure helpers `percentText`, `resetText(resetsAt, now)` ("resets in 2 h 10 min", "resets at 18:00 tomorrow"), `statusText(reading, stale)`; unit tests in `ui/tests/usageMeter.test.ts` first
- [X] T035 [US1] Create `ui/src/usage/UsagePanel.tsx` (popover; one section per enabled provider with color bar, plan, account/email, windows as horizontal bars, reading time, refresh button); calls `usage_refresh {reason: "panelOpen"}` on open; make T029 pass. Original layout and wording (clean room)
- [X] T036 [US1] Create `ui/src/usage/UsageButton.tsx` (top-right of the canvas area, gauge icon from `lucide-react`, opens/closes the panel, closes on Escape and outside click) and mount it in `ui/src/App.tsx`; style in `ui/src/styles.css`
- [X] T037 [US1] Create `ui/src/dialogs/UsageConsentDialog.tsx` on top of `Modal.tsx`, shown from `usage://consent` in `ui/src/App.tsx`; make T030 pass
- [ ] T038 [US1] Manual check with the real Claude Code: run quickstart scenarios 1–3 with a throwaway `SONATA_HOME`; record result in `specs/006-agent-usage/quickstart.md` only if steps differ

**Checkpoint**: MVP — Claude Code limits visible in the panel, gated by consent.

---

## Phase 4: User Story 2 - Get warned before a limit runs out (P2)

**Goal**: compact meter shows the highest usage; one macOS notification per level (80, 100) per
window period; warnings switch.

**Independent Test**: quickstart scenarios 6–7 (fake provider readings crossing 80 and 100).

### Tests

- [X] T039 [P] [US2] Write failing tests in `crates/core/tests/usage_warnings.rs` for `window::WarningLedger` (data-model "WarningLedger"): 79 → 85 emits warn80 once; 85 → 90 emits nothing; 90 → 100 emits full100; 100 → 95 → 100 emits nothing more; new `resetsAt` with 85 emits warn80 again; warnings off updates `sent` without emitting; ledger pruned of past `resetsAt`; ledger survives cache save/load (no repeat after restart, SC-003)
- [X] T040 [P] [US2] Write failing tests in `crates/core/tests/usage_summary.rs` for `window::summary`: highest `usedPercent` across readable providers wins; `warning` at ≥ 80, `full` at ≥ 100; stale readings still count; `off` when master switch off; `empty` with no readings
- [X] T041 [P] [US2] Write failing vitest `ui/tests/usageButton.test.tsx`: meter ring states `normal | warning | full | off | empty`, tinted with the provider color; title text names agent, window and reset time

### Implementation

- [X] T042 [US2] Implement `WarningLedger` and `summary` in `crates/core/src/usage/window.rs`; persist ledger in the cache (`warnings` array) via `crates/core/src/usage/cache.rs`; make T039–T040 pass
- [X] T043 [US2] Call the ledger in `UsageService` after each `ok` reading; emit `usage://warning` when it returns a new level and warnings are on; add `usage_set_warnings` to `App`, `src-tauri/src/commands/usage.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `ui/src/ipc/commands.ts`
- [X] T044 [US2] In `ui/src/App.tsx`, on `usage://warning` send a notification through `@tauri-apps/plugin-notification` (same permission flow as `messaging://notify`): title "<Agent>: <window> at 80%" / "<Agent>: <window> limit reached", body with reset time
- [X] T045 [US2] Render the meter ring in `ui/src/usage/UsageButton.tsx` from `summary`; make T041 pass

**Checkpoint**: warnings fire exactly once per level per period; meter reflects the highest usage.

---

## Phase 5: User Story 4 - Control usage reading in Settings (P2)

**Goal**: Settings window (Cmd+,) with the Usage section: master switch, warnings switch, one row
per agent (tag, color, path, status, switch, refresh), Refresh all.

**Independent Test**: quickstart scenarios 8 and 12; master switch off stops all probes; one
agent off stops only that agent.

### Tests

- [X] T046 [P] [US4] Write failing tests in `crates/core/tests/usage_settings.rs`: master off → `usage_tick` runs no reader and `summary.state = off` (fake reader counts calls); provider off → skipped and omitted from panel providers; `notFound` provider switch has no effect on reads until detection finds it (US4-3); color change validated (`#rrggbb`) and reflected in state; "Refresh all" runs every readable provider once
- [X] T047 [P] [US4] Write failing vitest `ui/tests/usageSettings.test.tsx`: master switch, warnings switch, a row per provider with name, "Built-in"/"Custom" tag, color input, path or "Command not found", status ("Read 3 min ago", "Not logged in", error), switch, refresh; "Refresh all" calls `usage_refresh` without `providerId`

### Implementation

- [X] T048 [US4] Make T046 pass in `crates/core/src/usage/service.rs` (re-detect commands on "Refresh all" and on a `notFound` read)
- [X] T049 [US4] Add commands `usage_set_provider_enabled`, `usage_set_provider_color`, `usage_open_settings` to `src-tauri/src/commands/usage.rs`, `src-tauri/build.rs`, capabilities, and `ui/src/ipc/commands.ts`
- [X] T050 [US4] Add menu item "Settings…" with accelerator `Cmd+,` to the app menu in `src-tauri/src/menu.rs`; handler opens or focuses a `WebviewWindow` with label `settings` (`index.html?view=settings`, 640×520, resizable) in `src-tauri/src/lib.rs`
- [X] T051 [US4] Create `src-tauri/capabilities/settings.json` (window `settings`: `core:default`, every `allow-usage-*`, `allow-app-reveal-path`)
- [X] T052 [US4] Route `?view=settings` in `ui/src/main.tsx` to `ui/src/settings/SettingsApp.tsx` (left section list with only "Usage", right content); create `ui/src/settings/UsageSettings.tsx`; the consent dialog also mounts here when this window is focused; make T047 pass
- [X] T053 [US4] Panel footer link "Usage settings…" in `ui/src/usage/UsagePanel.tsx` calls `usage_open_settings`

**Checkpoint**: Settings controls every read; master switch off means zero probe processes.

---

## Phase 6: User Story 3 - Other agents in the same panel: Codex (P3)

**Goal**: Codex limits from `codex app-server`, falling back to the newest session file.

**Independent Test**: with Codex installed and used once, panel values match Codex `/status`.

### Tests

- [X] T054 [US3] Record Codex fixtures (never install Codex): if `codex` is available on the maintainer's Mac, run `codex app-server generate-json-schema --out <tmp>` and record the `account/rateLimits/read` response and `initialize` params into `crates/core/tests/fixtures/usage/codex-*.json`; otherwise build fixtures from the public `openai/codex` source (research R2) and mark the task note "unverified live" in `specs/006-agent-usage/research.md`
- [X] T055 [P] [US3] Write failing tests in `crates/core/tests/usage_codex.rs`: app-server response → windows (`primary` 300 min → short "5-hour", `secondary` 10080 min → weekly "Weekly", other lengths → `other` labelled from minutes), `planType` → plan; auth error → `NotLoggedIn`; rollout tail parser: last `token_count` with non-null `rate_limits` in the last 256 KiB of a fixture file, `readAt` = file mtime; no session file → `Unavailable` "Use Codex once to record limits."
- [X] T056 [P] [US3] Write failing test with a fake `codex` script that reads JSON lines from stdin and answers `initialize` and `account/rateLimits/read`; reader sends exactly the three lines of contracts/usage-provider.md, stops after `id: 2`, and kills the group; fake that exits on unknown method → fallback to rollout

### Implementation

- [X] T057 [US3] Implement `crates/core/src/usage/codex.rs` (`CodexReader`: app-server via `runner` with piped stdin, 1 MiB line-by-line cap, 20 s timeout; `CODEX_HOME` honored; rollout fallback); register `codex` in the service registry; make T055–T056 pass
- [X] T058 [US3] Confirm the panel and Settings show Codex rows with no UI change (vitest case in `ui/tests/usagePanel.test.tsx` with two providers)

**Checkpoint**: Codex section shows its windows or an honest unavailable state.

---

## Phase 7: User Story 5 - Add another agent through a definition file (P3)

**Goal**: custom providers from `~/.sonata/usage-providers/*.json`, consent bound to the exact
command, sandboxed 10 s run, guide file, folder actions.

**Independent Test**: quickstart scenarios 9–11.

### Tests

- [X] T059 [P] [US5] Write failing tests in `crates/core/tests/usage_custom_definition.rs` (contracts "Custom definition file"): valid file loads; envelope type must be `usageProvider` v1; "`id` `^[a-z0-9][a-z0-9-]{0,31}$`; not `claude`/`codex`; unique across files (duplicates: both rejected)"; "`name` 1–40 characters"; "`command` 1–17 strings; `command[0]` absolute path to an executable regular file; each ≤ 1 KiB; no NUL"; "`color` optional `#rrggbb`"; unknown keys rejected; `README.md` and non-`.json` ignored; an invalid file yields a provider row with `definitionError` and does not affect others
- [X] T060 [P] [US5] Write failing tests in `crates/core/tests/usage_custom_output.rs` (contracts "Normalized output"): ok with 1–4 windows; `resetsAt` as RFC 3339 or Unix seconds; default labels "Short window" / "Weekly" / "Limit"; `status` `notLoggedIn`/`noPlanLimits`/`unavailable` with `message` ≤ 200 characters; `plan`/`account` ≤ 40, `label` ≤ 24; 0 or 5 windows when ok → `Unavailable`; non-zero exit → "exited with status 2"; timeout → "timed out after 10 s"
- [X] T061 [P] [US5] Write failing tests in `crates/core/tests/usage_sandbox.rs`: `custom::sandbox_profile(home, sonata_root)` matches the contract profile with canonicalized, escaped paths; with a fake definition script, reading a file under a temp "sonata root" is denied, reading under `<root>/usage-providers` is allowed, network not tested; when `sandbox-exec` path is missing → `Unavailable` "Cannot run safely: sandbox-exec not found" and nothing runs
- [X] T062 [P] [US5] Write failing tests in `crates/core/tests/usage_custom_consent.rs`: new definition → `usage://consent` with the full argv; after allow, editing the file to add an argument → `consent: unasked` again; denied → switch off
- [X] T063 [P] [US5] Write failing test: `usage_providers_folder` creates the folder (0700) and writes `README.md` when missing, never overwrites an existing one

### Implementation

- [X] T064 [US5] Implement definition loading and validation, `CustomReader` (runner with 10 s timeout, sandbox wrap), and `sandbox_profile` in `crates/core/src/usage/custom.rs`; reload definitions at start and on "Refresh all"; make T059–T062 pass
- [X] T065 [US5] Write the guide text as a constant in `crates/core/src/usage/custom.rs` (original English text covering every item listed in contracts "Guide", including a `jq` wrapper example and an agent prompt); implement `App::usage_providers_folder`; make T063 pass
- [X] T066 [US5] Add command `usage_providers_folder` (src-tauri command, `build.rs`, capabilities, `ui/src/ipc/commands.ts`); in `ui/src/settings/UsageSettings.tsx` add "Show folder" (existing `app_reveal_path`) and "Copy path" (clipboard) and show `definitionError` in custom rows

**Checkpoint**: a fake definition appears in Settings and the panel after consent; invalid files are isolated.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T067 [P] Write failing test `crates/core/tests/usage_no_secrets.rs` (SC-005): run a full service cycle with fake `claude` whose auth output includes `user@example.com` and whose stderr includes a fake token `sk-ant-oat01-FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE`; afterwards no file under the temp data root and no captured log line contains the token or the email
- [X] T068 [P] README `## Usage meter` in `README.md`: what is shown, where data comes from (CLI probe and its cost, Codex app server), consent, master switch, custom providers folder and guide, the sandbox; constitution-required "how to run and test" pointing to `specs/006-agent-usage/quickstart.md`
- [ ] T069 Verify reads never block terminals (SC-004): run 8 shell terminals printing output while forcing "Refresh all" with a fake provider that sleeps 9 s; terminals keep scrolling (manual check, note result in quickstart)
- [X] T070 Run `sh scripts/check.sh` until it passes in full; run the "No credential check" from `specs/006-agent-usage/quickstart.md` after a dev session
- [X] T071 `git fetch && git rebase origin/main`; resolve conflicts in shared files (`README.md`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`, `ui/src/ipc/*`, `ui/src/App.tsx`) keeping both sides; rerun `sh scripts/check.sh`

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → US1 (Phase 3).
- US2 (Phase 4) and US4 (Phase 5) depend on US1's service wiring (T031–T033); they are
  independent of each other except shared files (`UsageButton.tsx`, `commands/usage.rs`).
- US3 (Phase 6) and US5 (Phase 7) depend only on Phase 2 and T018/T020; US5's folder actions
  (T066) need the Settings window from US4 (T052).
- Polish after the stories it covers.

Within each phase: tests first (must fail), then implementation in listed order.

## Parallel Opportunities

- Phase 2 tests T004–T010 touch separate files: run in parallel. T023/T024 (UI) in parallel
  with core work.
- US1: T025–T030 in parallel; T034 in parallel with T031–T033.
- US2: T039–T041 in parallel.
- US3 and US5 can proceed in parallel after US1, on separate files (`codex.rs` vs `custom.rs`).

## Implementation Strategy

1. MVP: Phases 1–3 (Claude Code panel with consent). Stop and validate against `/usage`.
2. Add US2 (warnings + meter), then US4 (Settings window).
3. Add US3 (Codex) and US5 (custom providers).
4. Polish, full checks, rebase. Commit per task group with Conventional Commits.
