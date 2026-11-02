# Research: Report Bug (013)

## R1. Where the report is sent from: core over HTTPS

- **Decision**: `crates/core/src/report` builds and sends the request with `reqwest` (async,
  `rustls`, `multipart`), 30 s total timeout. The UI never talks to the network.
- **Rationale**: constitution II puts rules in core; the webview has `csp: null` but sending
  from Rust keeps validation, size limits, and the timeout testable with `cargo test`.
  `reqwest` 0.13 is already in `Cargo.lock` through `tauri-plugin-updater`.
- **Alternatives**: `fetch` in the webview (logic in the UI, harder to test, CORS on the
  Worker); `ureq` (new crate in the tree); raw `hyper` (more code).

## R2. Request format: multipart/form-data

- **Decision**: fields `name`, `description`, `appVersion`, `os`, `arch`, `sentAt`,
  `logsRequested`; files `image` (PNG or JPEG) and `log` (text). See
  [contracts/report-service.md](contracts/report-service.md).
- **Rationale**: raw bytes, no base64 growth (a 5 MB image stays 5 MB, so the 7 MB cap holds);
  the Worker parses it with the built-in `request.formData()`.
- **Alternatives**: JSON with base64 (≈33% larger, 5 MB image becomes 6.7 MB).

## R3. Rate limits: one Durable Object

- **Decision**: a single `Limiter` Durable Object instance (SQLite storage) holds two counters:
  per address per UTC hour (key `h:<sha256(ip)>:<yyyymmddhh>`) and global per UTC day
  (`d:<yyyymmdd>`). The Worker asks it to reserve a slot before calling Resend; an alarm
  deletes keys older than two days. The client address comes from `CF-Connecting-IP`.
- **Rationale**: a Durable Object processes requests one at a time, so counts are exact under
  bursts (SC-005). Durable Objects with SQLite storage are on the Workers free plan. Hashing
  keeps raw addresses out of storage.
- **Alternatives**: Workers KV (eventually consistent, bursts pass the limit); the rate
  limiting binding (periods of 10 s or 60 s only); no global limit (a botnet could burn the
  Resend free quota of 100 emails a day).
- **Note**: a slot is reserved before sending; if Resend fails, the slot stays used. This keeps
  the logic simple and errs on the side of fewer emails.

## R4. Email: Resend HTTP API

- **Decision**: `POST https://api.resend.com/emails` with `Authorization: Bearer
  <RESEND_API_KEY>`; JSON `from`, `to` (`REPORT_TO`), `subject`, `text`, `attachments`
  (`filename`, base64 `content`). `from` is the Worker variable `REPORT_FROM`, default
  `Sonata Reports <onboarding@resend.dev>`.
- **Rationale**: Resend's free plan (100 emails a day, 3,000 a month) allows the shared
  `onboarding@resend.dev` sender to deliver to the account owner's own address, which is
  exactly the maintainer; a verified domain is optional later. Subject:
  `[Sonata bug] <name>: <first 60 chars of description>`.
- **Alternatives**: MailChannels (free Workers integration ended in 2024); SMTP from the Worker
  (no raw TCP SMTP with STARTTLS in a simple way).
- **Verify during implementation**: current Resend attachment field names and the
  onboarding-sender restriction against Resend's docs.

## R5. Secrets and build-time configuration

- **Decision**: Worker secrets `RESEND_API_KEY` and `REPORT_TO` set with `wrangler secret put`
  (prompted input). App build-time values `SONATA_REPORT_URL` and
  `SONATA_REPORT_FALLBACK_EMAIL` read with `option_env!` in `src-tauri/src/report.rs` and passed
  to core; in CI they come from GitHub repository variables (`vars.*`), not secrets, because
  both are public by nature. Unset values mean: no service (Send goes to the fallback), no
  alias (fallback offers copy only).
- **Rationale**: FR-015 and SC-004; a search for the key and the destination address in the
  repo and release files finds nothing.
- **Alternatives**: a config file in `~/.sonata` (every friend would have to edit it); hard-coded
  URL in the repo (works, but forks would spam the maintainer's Worker).

## R6. Diagnostics

- **Decision**: `diagnostics::collect()` returns app version (`CARGO_PKG_VERSION` passed from
  the shell, the same value the updater uses), `std::env::consts::OS`/`ARCH`, and an OS version
  from `platform::os_version()`: macOS `sw_vers -productVersion`; Linux `PRETTY_NAME` from
  `/etc/os-release` plus `uname -r`; Windows `RtlGetVersion` through `windows-sys` (already a
  dependency) for major.minor.build. Failures give `unknown`, never an error.
- **Rationale**: no new crate (`os_info` and `tauri-plugin-os` rejected for that reason);
  one function per OS in the existing platform modules (constitution VII).

## R7. Log excerpt

- **Decision**: newest `sonata.log.*` in `paths.logs_dir()` that is not empty; read the last
  1 MB; if the read starts mid-file, drop bytes up to the first newline; lossy UTF-8. Missing
  or unreadable: no file, and the email body says "logs requested but unavailable".
- **Rationale**: matches the daily file layout in `crates/core/src/logging.rs` (7 days kept).

## R8. Image input in the webview

- **Decision**:
  - Paste: `paste` event on the dialog; the first `clipboardData.items` entry of kind `file`
    with type `image/png` or `image/jpeg` becomes the attachment; text pastes stay normal.
    Bytes go to core as an array (`Uint8Array` → `number[]`, at most 5 MB).
  - Drag and drop: Tauri intercepts file drops on all three webviews (`dragDropEnabled`
    default), so the dialog listens to `getCurrentWebview().onDragDropEvent` while open and
    takes the first path.
  - File chooser: `tauri-plugin-dialog` `open` with filters `png`, `jpg`, `jpeg`.
  - Paths go through `report_read_image(path)`; core checks size (≤ 5 MB) and magic number
    (`89 50 4E 47 0D 0A 1A 0A` or `FF D8 FF`) and returns bytes and type. Pasted bytes get the
    same check in `report_send`.
- **Rationale**: one validation in core for all three paths; drag and drop works the same way
  on WKWebView, WebKitGTK, and WebView2.
- **Risk**: the canvas may already use drag-drop events; the dialog handler only acts while the
  dialog is open and stops propagation to the canvas handler.

## R9. Fallback: mailto and copy

- **Decision**: `mailto::build(alias, report)` gives
  `mailto:<alias>?subject=…&body=…` with RFC 3986 percent-encoding; the body is cut to 1,500
  characters with a "(cut, full text copied to clipboard)" marker, and in that case the full
  text is also copied. Opened with the existing `platform::open_url_command`. If opening fails
  or no alias is configured, the UI copies the plain text with `navigator.clipboard.writeText`
  (already used in the app) and says so.
- **Rationale**: long `mailto:` URLs break on Windows mail handlers (about 2,000 characters).

## R10. Help menu and sidebar control

- **Decision**: new `Help` submenu on every platform with "Report Bug…" (id `report-bug`),
  emitting `REPORT_BUG_EVENT`; on macOS it is the last menu and is registered as the Help menu
  (`set_as_help_menu_for_nsapp`) so the system search field appears. The sidebar footer gets a
  small "Report Bug" button next to the update banner.
- **Rationale**: no Help menu exists today (`src-tauri/src/menu.rs`).

## R11. Worker tests and CI

- **Decision**: `@cloudflare/vitest-pool-workers` runs tests inside `workerd` with the Durable
  Object binding; `fetch` to Resend is mocked with `fetchMock`. A `worker` CI job on
  `ubuntu-latest` runs `npm ci && npm run typecheck && npm test` in `worker/`. Deploy stays
  manual (`npx wrangler deploy`) and documented, so CI never needs Cloudflare credentials.
- **Rationale**: tests run on the real runtime, including `formData()` limits and Durable
  Object behavior.

## R12. Portuguese message and the strings check

- **Decision**: the sentence lives in one exported constant `IMPROVEMENT_POINTS` in
  `ReportBugDialog.tsx`; `ui/tests/windowsStrings.test.ts` is unaffected (it scans for macOS
  terms, not language). The plan's Complexity Tracking records the deviation.

## Verification during implementation (2026-11-01)

- **R4 Resend** (resend.com/docs/api-reference/emails/send-email): `POST
  https://api.resend.com/emails`, `Authorization: Bearer <key>`; `from` accepts
  `Name <address>`; `to` is a string or an array (max 50); `text` is optional; `attachments`
  is an array of objects with `filename` and `content` (Buffer or Base64 string) or `path`;
  the content type is derived from `filename` when not set. Limit: 40 MB per email after
  Base64 encoding, far above our 5 MB image + 1 MB log. Success returns `{"id": "…"}`.
  The plan's field names (`filename`, base64 `content`) are confirmed. The onboarding-sender
  restriction (`onboarding@resend.dev` delivers only to the account owner's address) is
  a Resend account rule; the maintainer confirms it at deploy time (T042).
- **R3 Durable Objects** (developers.cloudflare.com/durable-objects/platform/pricing):
  available on Workers Free; only the SQLite storage backend is available there (so the
  `new_sqlite_classes` migration in `wrangler.toml` is required). Free limits: 100,000
  requests a day, 13,000 GB-s duration a day, 5 million rows read and 100,000 rows written a
  day, 5 GB stored; daily limits reset at 00:00 UTC. With at most 100 reports a day plus
  rejected bursts this stays far below every limit.
- **R11 test tooling**: `@cloudflare/vitest-pool-workers` is deprecated and renamed to
  `@cloudflare/vitest-plugin` (Vite plugin `cloudflareTest`, supports Vitest 4 and 5). The
  Worker uses the new package. `fetchMock` is gone; tests call the Worker's `fetch` handler
  directly and replace `globalThis.fetch` with a Vitest spy for the Resend call.
- **R1 `reqwest` features**: the `rustls` feature of `reqwest` 0.13 pulls `aws-lc-rs`, a new
  C-built crypto crate. Core instead enables `rustls-no-provider` + `multipart` and installs
  the `ring` provider that the updater already builds (same pattern as
  `tauri-plugin-updater`). The only new crates are `mime_guess` and `unicase`.

## Clipboard rules in WKWebView (2026-11-02)

Probed with a real `WKWebView` on macOS, in a secure context, starting each write through
`evaluateJavaScript`, which carries a user gesture as a click does. The probe could not use real
mouse clicks, because a command-line process could not become the active app, so the dialog
itself still needs a manual check (quickstart Scenario E).

| Case | Result | Clipboard |
|------|--------|-----------|
| `write` inside the gesture, PNG promise resolves 300 ms later | ok | PNG |
| `write` inside the gesture, text promise resolves 300 ms later | ok | text |
| `write` inside the gesture, promise rejects | `NotAllowedError` | unchanged |
| `write` 1.5 s after the gesture | `NotAllowedError` | unchanged |

**Decision**: start every write inside the click and pass its content as a promise
(`ui/src/report/clipboard.ts`). A rejected promise is a safe way to write nothing. A copy that
starts after an IPC answer can be refused on macOS, so `writeTextNow` is only a best effort.
Linux (WebKitGTK) and Windows (WebView2) were not probed; the dialog handles a refused write by
saying to attach the image by hand.
