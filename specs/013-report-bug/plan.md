# Implementation Plan: Report Bug

**Branch**: `013-report-bug` | **Date**: 2026-11-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/013-report-bug/spec.md`

## Update 2026-11-02: mail app by default

The maintainer decided not to deploy Cloudflare or Resend for now (spec, session 2026-11-02).
Without `SONATA_REPORT_URL`, which is now the default, the dialog's primary button reads
"Open email": it opens `mailto:` to `REPORT_EMAIL` (`src-tauri/src/report.rs`, overridable with
`SONATA_REPORT_FALLBACK_EMAIL`) and hides "Include logs". With the URL set, everything below
still applies, and `worker/` is unchanged and optional.

Decisions:

- **Clipboard writes start inside the click.** WebKit (macOS, Linux) refuses a clipboard write
  once the click's task has ended, so `ReportBugDialog` awaits nothing before calling
  `navigator.clipboard.write`, and hands the content over as a promise in the `ClipboardItem`
  (`ui/src/report/clipboard.ts`). A rejected promise leaves the clipboard unchanged, which is
  how the no-copy-needed case writes nothing.
- **The UI knows about the cut before the click.** Choosing between image and text must happen
  inside the click too, so `report_config` returns `mailBodyRoom`, computed in core by
  `mailto::body_room`. `report_fallback` still reports the real `truncated`.
- **A real check that the mail app opened.** `open` and `xdg-open` start fine and exit with an
  error when nothing handles `mailto:`, so `Launch::spawn_checked` watches the opener for up to
  2 seconds. `rundll32` on Windows always exits with 0, so there a missing handler goes
  undetected and Windows shows its own "choose an app" prompt.
- **The image stays out of `report_fallback` in this mode.** The mail link cannot use it, and it
  would cost a large IPC message.

## Summary

A "Report Bug…" dialog (Help menu and sidebar footer) lets a user send a name, a description,
one PNG or JPEG image, and optionally a log excerpt, with diagnostics added by the app. `core`
validates the report, collects diagnostics and the log tail, and posts it as
`multipart/form-data` over HTTPS to a small Cloudflare Worker kept in `worker/`. The Worker
validates again, applies per-address and global limits in one Durable Object, and emails the
report with attachments through the Resend API. The Resend key and the destination address are
Worker secrets. The service URL and the public fallback alias are set at build time; when
sending fails, the app opens a `mailto:` to the alias or copies the report text.

## Technical Context

**Language/Version**: Rust 2024 edition (workspace toolchain), TypeScript 5 (UI and Worker)

**Primary Dependencies**: app: Tauri 2, React, `reqwest` 0.13 (already in the dependency tree
through `tauri-plugin-updater`; now a direct `core` dependency with `multipart` and
`rustls`), `tauri-plugin-dialog` (file chooser, existing). Worker: Cloudflare Workers runtime,
Durable Objects (SQLite storage, free plan), Resend HTTP API; dev only: `wrangler`,
`vitest`, `@cloudflare/vitest-pool-workers` in `worker/package.json` (not part of the app
build)

**Storage**: none in the app (the draft lives in UI memory until quit). Worker: hourly and daily
counters in the Durable Object, keyed by a SHA-256 of the client address; no report content is
stored

**Testing**: `cargo test` (core: validation, diagnostics, log tail, multipart body, mailto,
send against a local test HTTP listener), Vitest + Testing Library (dialog), Vitest in the
Workers pool (Worker validation, limits, Resend call with a mocked `fetch`)

**Target Platform**: macOS 15+, Linux x86_64 (WebKitGTK 4.1), Windows 10 1809+ / 11 (WebView2);
Worker on Cloudflare

**Project Type**: desktop app plus a small web service in the same repository

**Performance Goals**: report reaches the inbox within 1 minute (SC-002); dialog opens instantly

**Constraints**: request at most 7 MB; image at most 5 MB; log at most 1 MB; 30 s send timeout;
5 reports per address per hour and 100 per day in total; zero cost (constitution VI)

**Scale/Scope**: a few friends; up to 100 reports a day

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Clean-Room Originality | Pass | New feature, no reference behavior involved; own wording |
| II. Core/Shell Separation | Pass | Validation, diagnostics, log tail, request building, sending, and mailto building live in `crates/core/src/report/`; `src-tauri` adds a menu item and thin commands; the UI renders and sends intents |
| III. Test-First | Pass | Core tests use a local listener started by the test, never the real service; Worker tests mock `fetch` to Resend; UI tests mock the commands |
| IV. Files Are the Source of Truth | Pass | Nothing is persisted; the draft is session-only by spec (FR-019) |
| V. Server-Side Authorization & Local Security | Pass | No listener added; outbound HTTPS only, on an explicit user action. Image bytes checked by magic number in core and in the Worker. Log excerpt only on opt-in. Worker secrets set with `wrangler secret put` (stdin, not arguments) |
| VI. Free, Open Distribution | Pass | Cloudflare Workers + Durable Objects and Resend free plans; no paid service in the build. A build without the service URL still works (fallback) |
| VII. Simplicity & Incremental Delivery | Pass with tracking | `reqwest` becomes a direct core dependency; new `worker/` package. See Complexity Tracking |
| Technology constraint: English UI text | Deviation | One Portuguese sentence, chosen by the maintainer. See Complexity Tracking |

Post-design re-check (after Phase 1): unchanged; all principles pass and the tracked items stay
as listed.

## Project Structure

### Documentation (this feature)

```text
specs/013-report-bug/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── report-service.md   # HTTP contract between app and Worker
│   └── app-commands.md     # Tauri commands, menu event, UI contract
└── tasks.md                # /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/report/
├── mod.rs          # BugReport, ReportImage, limits, validation, send(), errors
├── diagnostics.rs  # app version, OS name/version, architecture
├── log_tail.rs     # last ≤1 MB of the newest sonata.log.* file, cut at a line start
└── mailto.rs       # fallback mailto URL and plain-text report
crates/core/src/platform/{macos,linux,windows}.rs  # os_version()
crates/core/tests/report_send.rs   # send() against a local test listener

src-tauri/src/
├── menu.rs         # Help menu with "Report Bug…" (all platforms), emits the report-bug event
├── report.rs       # commands: report_config, report_read_image, report_send, report_fallback
└── lib.rs          # register commands

ui/src/report/
├── ReportBugDialog.tsx  # form, preview, paste/drop/picker, states
└── draft.ts             # session draft store
ui/src/sidebar/Sidebar.tsx   # footer "Report Bug" control
ui/src/ipc/commands.ts       # typed wrappers
ui/tests/reportBugDialog.test.tsx

worker/
├── package.json, tsconfig.json, wrangler.toml, vitest.config.ts
├── src/index.ts         # fetch handler: parse, validate, limit, send
├── src/validate.ts      # form fields, sizes, magic numbers
├── src/limiter.ts       # Durable Object: hourly per address, daily global
├── src/email.ts         # Resend request
└── test/*.test.ts

.github/workflows/release.yml  # passes SONATA_REPORT_URL and SONATA_REPORT_FALLBACK_EMAIL (repository variables)
.github/workflows/ci.yml       # worker job: npm ci, typecheck, test in worker/
README.md                      # "Reporting bugs" and "Running the report service"
```

**Structure Decision**: existing Cargo workspace and `ui/` layout; the Worker is a separate npm
package in `worker/`, so its tooling never enters the app build.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Portuguese sentence in an English UI (Technology constraint "English for UI text") | The maintainer chose this exact playful message (spec FR-003). It is one fixed string in one constant | Translating it removes the joke the maintainer asked for |
| `reqwest` as a direct `core` dependency | `core` must post a multipart HTTPS request with a timeout | Hand-written HTTP over TLS is error-prone; `reqwest` is already compiled into the app through the updater, so binary size and audit surface barely change |
| New `worker/` npm package with `wrangler` and the Workers test pool | The report service must hold the secrets, and its code must live in the repo (FR-015, FR-017) | A third-party form service either puts a key in the app or charges for attachments; `mailto:` cannot attach images |
| Durable Object for limits | Exact counts under parallel requests (SC-005) | Workers KV is eventually consistent, so a burst can pass the limit; the built-in rate limiting binding supports only 10 s or 60 s periods, not 1 hour or 1 day |
