# Tasks: Report Bug

**Input**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Required (constitution III). Each test task comes before its implementation task and
must fail first. Core tests never call the real service: they start a local listener in the
test. Worker tests run in the Workers pool with `fetch` to Resend mocked.

**Verification rule**: run `sh scripts/check.sh` before every commit that changes app code, and
`(cd worker && npm run typecheck && npm test)` before every commit that changes `worker/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: User story from spec.md (US1–US5)

## Phase 1: Setup

- [ ] T001 Add `reqwest = { version = "0.13", default-features = false, features = ["rustls", "multipart"] }` to workspace deps in Cargo.toml and as a dependency of crates/core/Cargo.toml; confirm `cargo tree -p sonata-app -d` adds no second `reqwest` version (use the features the lock already has if the names differ)
- [ ] T002 [P] Scaffold worker/: package.json (scripts `typecheck` = `tsc --noEmit`, `test` = `vitest run`, `deploy` = `wrangler deploy`; devDependencies `wrangler`, `vitest`, `@cloudflare/vitest-pool-workers`, `@cloudflare/workers-types`, `typescript` at exact versions), tsconfig.json, wrangler.toml (name `sonata-report`, `main = "src/index.ts"`, `REPORT_FROM = "Sonata Reports <onboarding@resend.dev>"`, Durable Object binding `LIMITER` → class `Limiter` with a `new_sqlite_classes` migration), vitest.config.ts (pool workers, miniflare bindings `RESEND_API_KEY = "test-key"`, `REPORT_TO = "dest@example.test"`), .gitignore (`node_modules`, `.wrangler`); commit package-lock.json
- [ ] T003 [P] Add a `worker` job to .github/workflows/ci.yml on `ubuntu-latest`: setup-node 22 with npm cache keyed on worker/package-lock.json, `npm ci`, `npm run typecheck`, `npm test`, all with `working-directory: worker`

## Phase 2: Foundational (blocks every story)

**Goal**: core types, limits, and validation exist; the app has a place to open the dialog.

- [ ] T004 Write failing unit tests in crates/core/src/report/mod.rs (`#[cfg(test)]`) for `BugReport::validate`: name "required; trimmed; 1–100 characters (Unicode scalar values)", description "required; trimmed; 1–10,000 characters", emoji and non-Latin names accepted unchanged, image "1 B – 5 MB (5 × 1024 × 1024)", mime "decided from the magic number, never from a file name" (PNG `89 50 4E 47 0D 0A 1A 0A`, JPEG `FF D8 FF`, GIF rejected as `unsupported_type`)
- [ ] T005 Implement crates/core/src/report/mod.rs: constants `MAX_NAME = 100`, `MAX_DESCRIPTION = 10_000`, `MAX_IMAGE = 5 * 1024 * 1024`, `MAX_LOG = 1024 * 1024`, `SEND_TIMEOUT = 30 s`; `BugReport`, `ReportImage`, `ImageMime`, `sniff_image(bytes) -> Result<ImageMime>`, `validate()`, `ReportError` with kinds `invalid { field, reason }`, `not_configured`, `unreachable`, `rate_limited { retry_after_secs }`, `refused { code }` (serde tag `kind`, camelCase, per contracts/app-commands.md); register `pub mod report;` in crates/core/src/lib.rs
- [ ] T006 [P] Write failing tests for `platform::os_version()` (returns non-empty, never panics; on Linux parses `PRETTY_NAME` from a fixture os-release string through a pure helper `linux_pretty_name(text)`) in crates/core/src/platform/mod.rs tests
- [ ] T007 [P] Implement `os_version()` per research R6 in crates/core/src/platform/macos.rs (`sw_vers -productVersion`), crates/core/src/platform/linux.rs (`PRETTY_NAME` + `uname -r`, form `Ubuntu 24.04.1 LTS (6.8.0-45-generic)`), crates/core/src/platform/windows.rs (`RtlGetVersion` via `windows-sys`, form `10.0.26100`; add the needed `windows-sys` feature in Cargo.toml), and the dispatcher in crates/core/src/platform/mod.rs; any failure gives `"unknown"`
- [ ] T008 Write failing tests then implement crates/core/src/report/diagnostics.rs: `Diagnostics { app_version, os, os_version, arch, sent_at }` with `os` in `macos | linux | windows` from `std::env::consts::OS`, `arch` from `std::env::consts::ARCH`, `sent_at` RFC 3339 UTC; `collect(app_version, now)`
- [ ] T009 Add a Help submenu on every platform with item id `report-bug`, label `Report Bug…`, emitting `REPORT_BUG_EVENT = "report-bug"`; on macOS append it last and call `set_as_help_menu_for_nsapp` in src-tauri/src/menu.rs (research R10)
- [ ] T010 [P] Add `onReportBug` listener for the `report-bug` event in ui/src/ipc/events.ts and add the `ReportConfigView`, `ImageView`, `ReportInput`, `ReportError`, `FallbackResult` types from contracts/app-commands.md to ui/src/ipc/types.ts

**Checkpoint**: `cargo test -p sonata-core report` and `platform` tests green; Help menu visible.

## Phase 3: User Story 1 — Friend sends a bug report with a screenshot (P1) 🎯 MVP

**Goal**: name + description (+ optional image already in memory) reach the service with
diagnostics.

**Independent Test**: quickstart Scenario A steps 2, 3, 6, 7 against a local or deployed Worker.

### Tests

- [ ] T011 [P] [US1] Write failing tests in crates/core/tests/report_send.rs with a local `tokio::net::TcpListener` HTTP stub started by the test: (a) 200 `{"ok":true}` → `Ok`; the received multipart has parts `name`, `description`, `appVersion`, `os`, `osVersion`, `arch`, `sentAt`, `logsRequested=false`, `image` (`screenshot.png`, `image/png`) and `User-Agent: Sonata/<version>`; (b) 429 with `Retry-After: 120` → `rate_limited { retry_after_secs: 120 }`; (c) 400 `{"ok":false,"error":"invalid_field:name"}` → `refused { code: "invalid_field:name" }`; (d) 502 → `unreachable`; (e) stub that never answers → `unreachable` within `SEND_TIMEOUT` (make the timeout injectable for the test); (f) no URL → `not_configured`; (g) invalid report → `invalid` without any connection
- [ ] T012 [P] [US1] Write failing Worker tests in worker/test/report.test.ts: valid form → 200 and exactly one mocked `POST https://api.resend.com/emails` with `Authorization: Bearer test-key`, `to: ["dest@example.test"]`, subject `[Sonata bug] <name>: <first 60 chars of description, one line>`, text body per contracts/report-service.md "Email produced", attachment `screenshot.png` with base64 content; Resend non-2xx → 502 `email_failed`; responses never contain the key or the destination address
- [ ] T013 [P] [US1] Write failing UI tests in ui/tests/reportBugDialog.test.tsx with mocked commands: headline text exactly `Nosso sistema não tem bugs, tem PONTOS DE MELHORIA`; `Send` disabled while `Your name` or `What happened?` is empty; Send shows `Sending…` then `Thanks! Your report was sent.` and calls `report_send` once even on double click; draft survives close/reopen; form cleared after success; counter appears from 9,000 characters; `report-bug` event and the sidebar `Report Bug` button both open the dialog

### Implementation

- [ ] T014 [US1] Implement `report::send(url: Option<&str>, report, diagnostics, log: Option<LogExcerpt>, timeout)` in crates/core/src/report/mod.rs with `reqwest::multipart` per contracts/report-service.md (part names, `screenshot.png`/`screenshot.jpg`, `sonata-log.txt` as `text/plain`), status mapping per the Responses table, `Retry-After` parsing; validate before connecting
- [ ] T015 [US1] Implement worker/src/validate.ts (`parseReport(request)`: method/path → `Content-Length` ≤ 7 × 1024 × 1024 → `formData()` → field rules and regexes from contracts/report-service.md → image ≤ 5 MB with magic number check, declared type ignored → `log` ≤ 1 MB and only when `logsRequested` is `true`; returns typed report or `{status, error}`), worker/src/email.ts (`sendEmail(env, report)` building the Resend JSON from research R4), and worker/src/index.ts (`POST /report` only; 404/405 otherwise; JSON `{"ok": bool, "error"?}`; logs only status, error code, sizes)
- [ ] T016 [US1] Add src-tauri/src/report.rs: `ReportConfig` from `option_env!("SONATA_REPORT_URL")` and `option_env!("SONATA_REPORT_FALLBACK_EMAIL")` (empty string = absent); commands `report_config() -> {serviceConfigured, fallbackEmail}` and `report_send(input)` that converts `ReportInput` (image `bytes: number[]`, `mime`), collects diagnostics with the app's package version, and calls `core::report::send`; register both in src-tauri/src/lib.rs; add `cargo:rerun-if-env-changed` for both variables in src-tauri/build.rs
- [ ] T017 [P] [US1] Add typed wrappers `reportConfig`, `reportSend` in ui/src/ipc/commands.ts
- [ ] T018 [US1] Implement ui/src/report/draft.ts (module-level session store with subscribe/get/set/clear, per data-model.md "Draft") and ui/src/report/ReportBugDialog.tsx (uses `Modal`; exported constant `IMPROVEMENT_POINTS = "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA"` as the most prominent text; fields, labels, and messages exactly as contracts/app-commands.md; state machine from data-model.md; styles in ui/src/styles.css)
- [ ] T019 [US1] Open the dialog from the `report-bug` event in ui/src/App.tsx and add a footer `Report Bug` button in ui/src/sidebar/Sidebar.tsx next to the update banner (prop `onReportBug`); extend ui/tests/sidebar.test.tsx if its snapshot or roles change

**Checkpoint**: T011–T013 green; a report sent from a dev build reaches a local `wrangler dev`.

## Phase 4: User Story 5 — Maintainer runs the report service at no cost (P1)

**Goal**: limits, deploy docs, and build wiring; secrets stay out.

**Independent Test**: quickstart sections 2 and 6.

- [ ] T020 [P] [US5] Write failing Worker tests in worker/test/limits.test.ts: 6 reports from one `CF-Connecting-IP` in one hour → five 200 and one 429 `address_limit` with `Retry-After`; another address still accepted; 101st report of the day → 429 `daily_limit`; 50 parallel requests from one address → exactly 5 Resend calls (SC-005); invalid requests (400/413) never use a slot; storage keys contain no raw address
- [ ] T021 [P] [US5] Write failing Worker tests in worker/test/validate.test.ts for every 400/404/405/413 row of contracts/report-service.md (missing field, bad `appVersion`, bad `os`, GIF image, 5 MB + 1 image, log with `logsRequested=false`, `Content-Length` over 7 MB, GET, wrong path)
- [ ] T022 [US5] Implement worker/src/limiter.ts: Durable Object `Limiter` (SQLite storage) with `reserve(ipHash, now) -> "ok" | "address_limit" | "daily_limit"` using keys `h:<sha256(ip) hex>:<yyyymmddhh UTC>` and `d:<yyyymmdd UTC>`, limits 5/hour/address and 100/day, increment both only on `ok`, daily alarm deleting keys older than 48 h; call it from worker/src/index.ts after validation and before `sendEmail`; export the class from index.ts
- [ ] T023 [US5] Pass `SONATA_REPORT_URL: ${{ vars.SONATA_REPORT_URL }}` and `SONATA_REPORT_FALLBACK_EMAIL: ${{ vars.SONATA_REPORT_FALLBACK_EMAIL }}` to the build steps of the macos, linux, and windows jobs in .github/workflows/release.yml
- [ ] T024 [US5] Add README.md sections "Reporting bugs" (Help → Report Bug…, what is sent, logs opt-in, fallback) and "Running the report service" (free Cloudflare and Resend accounts, `npx wrangler deploy`, `wrangler secret put RESEND_API_KEY` / `REPORT_TO`, the curl test from quickstart.md section 2, setting the two GitHub repository variables, security note that the key and destination live only in Worker secrets)
- [ ] T025 [US5] Add a check in scripts/check.sh (or a test in scripts/release/tests/) that fails when the repository contains a Resend key pattern `re_[A-Za-z0-9_]{20,}` outside worker/test

**Checkpoint**: T020–T021 green; quickstart section 2 works on a real account (manual, maintainer).

## Phase 5: User Story 2 — Attach an image in the most natural way (P2)

**Goal**: paste, drag and drop, file chooser, preview, remove, replace.

**Independent Test**: quickstart Scenario A steps 4–5.

- [ ] T026 [P] [US2] Write failing tests for `report::read_image(path)` in crates/core/src/report/mod.rs tests (tempdir files: PNG ok, JPEG ok, 5 MB + 1 → `too_large`, GIF → `unsupported_type`, missing → `unreadable`; the size check reads metadata first and never loads more than 5 MB + 1 bytes)
- [ ] T027 [P] [US2] Write failing UI tests in ui/tests/reportBugDialog.test.tsx: pasting a `image/png` clipboard file sets the preview `Attached image`; pasting text into the description inserts text and no image; `Attach image…` uses the mocked dialog `open` and `report_read_image`; a second image replaces the first; a rejected image shows a message naming "5 MB" or "PNG or JPEG" and keeps the earlier preview; `Remove image` clears it
- [ ] T028 [US2] Implement `read_image` in crates/core/src/report/mod.rs and the `report_read_image(path)` command in src-tauri/src/report.rs (registered in src-tauri/src/lib.rs); wrapper `reportReadImage` in ui/src/ipc/commands.ts
- [ ] T029 [US2] Implement paste (first `clipboardData.items` file of type `image/png` or `image/jpeg`; client-side size check before reading bytes), file chooser (`@tauri-apps/plugin-dialog` `open` with filters `png`, `jpg`, `jpeg`), drag and drop (`getCurrentWebview().onDragDropEvent` subscribed only while the dialog is open, first path, `drop` handled so the canvas ignores it), preview via object URL revoked on replace/remove/unmount in ui/src/report/ReportBugDialog.tsx; check the canvas drag-drop handlers in ui/src/canvas/ do not also react while the dialog is open

**Checkpoint**: T026–T027 green.

## Phase 6: User Story 3 — Send logs when asked (P2)

**Goal**: opt-in log excerpt.

**Independent Test**: quickstart Scenario B.

- [ ] T030 [P] [US3] Write failing tests in crates/core/src/report/log_tail.rs (tempdir): newest non-empty `sonata.log.*` chosen; file under 1 MB returned whole; file over 1 MB returns at most 1 MB starting right after a newline; invalid UTF-8 replaced, not an error; no log dir or only empty files → `None`
- [ ] T031 [US3] Implement `log_tail::read(logs_dir) -> Option<LogExcerpt>` in crates/core/src/report/log_tail.rs per research R7, using the same `sonata.log.` prefix as crates/core/src/logging.rs (expose the prefix as `pub(crate)` there instead of duplicating it)
- [ ] T032 [US3] In src-tauri/src/report.rs `report_send`, read the excerpt only when `includeLogs` is true and send `logsRequested`; extend crates/core/tests/report_send.rs: ticked with log → `log` part present; ticked without log → no `log` part, `logsRequested=true`; unticked → no `log` part and `logsRequested=false`
- [ ] T033 [P] [US3] Worker: body line `Logs: included | requested but unavailable | not requested` and `sonata-log.txt` attachment; tests in worker/test/report.test.ts
- [ ] T034 [P] [US3] UI: `Include logs` checkbox unticked by default with the hint `Logs can contain folder paths and project names.`; test in ui/tests/reportBugDialog.test.tsx

**Checkpoint**: T030–T034 green.

## Phase 7: User Story 4 — The report still reaches the maintainer when sending fails (P3)

**Goal**: failure states, Try again, mailto fallback, copy.

**Independent Test**: quickstart Scenario C.

- [ ] T035 [P] [US4] Write failing tests in crates/core/src/report/mailto.rs: `plain_text(report, diagnostics, logs_state)` matches the email body of contracts/report-service.md; `build(alias, …)` gives `mailto:<alias>?subject=…&body=…` with RFC 3986 percent-encoding (spaces as `%20`, newlines as `%0D%0A`, `&`, `?`, `#`, `+`, non-ASCII encoded); subject `Sonata <version> bug: <first words of description>`; body over 1,500 characters is cut with the marker `(cut, full text copied to clipboard)` and `truncated = true`
- [ ] T036 [US4] Implement crates/core/src/report/mailto.rs and the `report_fallback(input)` command in src-tauri/src/report.rs (opens with `sonata_core::platform::open_url_command`; returns `{opened, text, truncated}`; `opened = false` when no alias or the launch fails); wrapper `reportFallback` in ui/src/ipc/commands.ts
- [ ] T037 [P] [US4] Write failing UI tests in ui/tests/reportBugDialog.test.tsx for each failure message in contracts/app-commands.md (`unreachable`, `rate_limited`, `refused`, `not_configured`): content and image kept, `Try again` resends, `Send by email instead` calls `report_fallback`; `opened` → `Your mail app opened. Attach the image by hand if you have one.`; not opened or truncated → `navigator.clipboard.writeText(text)` and `Report copied to the clipboard. Paste it in a message to the maintainer.`; with `serviceConfigured = false`, Send goes straight to the fallback actions with `This build cannot send reports directly.`
- [ ] T038 [US4] Implement the failure and fallback states in ui/src/report/ReportBugDialog.tsx

**Checkpoint**: T035–T037 green.

## Phase 8: Polish & Cross-Cutting

- [ ] T039 [P] Add a test in ui/tests/windowsStrings.test.ts (or confirm the existing scan) so the dialog has no `Cmd`/`⌘` text on Windows and Linux (paste hint uses `keyLabel`)
- [ ] T040 [P] Update CLAUDE.md/AGENTS.md agent context if the repo keeps one, and record the Portuguese-string exception next to the language rule comment in ui/src/report/ReportBugDialog.tsx
- [ ] T041 Run `sh scripts/check.sh` and the worker checks; push the branch; confirm CI jobs `core-and-ui`, `linux-app`, `windows`, `worker` are green; record results in quickstart.md
- [ ] T042 Ask the user to create the Cloudflare and Resend accounts and the public alias, deploy (quickstart section 2), set the GitHub variables, and run Scenarios A–D in a dev build; do not open a PR, merge, tag, or release without their OK

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phase 3 (US1, MVP). US5 (Phase 4) needs the Worker from T015. US2, US3,
  US4 depend only on US1 and can run in any order after it.
- Inside each phase, test tasks come first and must fail before their implementation tasks.

## Parallel Opportunities

- T002 and T003 alongside T001.
- T006/T007 alongside T004/T005; T010 alongside T009.
- US1 tests T011, T012, T013 in parallel; then T014 (core), T015 (Worker), T017 (UI wrappers) in
  parallel.
- After US1: US2 core work (T026, T028) and US3 core work (T030, T031) in parallel with Worker
  work (T020–T022, T033).

## Implementation Strategy

1. MVP: Phases 1–3. A report with text and an in-memory image reaches a local Worker.
2. Phase 4 makes the service safe to deploy (limits, docs, release wiring).
3. Phases 5–7 add the attachment paths, logs, and fallback.
4. Phase 8 polishes and verifies on CI; the maintainer deploys and runs the quickstart.
