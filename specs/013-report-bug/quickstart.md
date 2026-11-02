# Quickstart: Report Bug (013)

Validation guide. Contracts: [report-service.md](contracts/report-service.md),
[app-commands.md](contracts/app-commands.md). Types: [data-model.md](data-model.md).

## Prerequisites

- Repository built as usual (`npm ci`, Rust toolchain).
- For the live service: free Cloudflare and Resend accounts; the Resend account email is the
  address that receives reports.
- Run dev builds next to the installed app with a throwaway data folder:
  `SONATA_HOME=$(mktemp -d) npx tauri dev --config '{"identifier":"dev.sonata.app.test013"}'`.

## 1. Automated checks

```sh
sh scripts/check.sh                 # cargo test/clippy/fmt, UI typecheck and tests
(cd worker && npm ci && npm run typecheck && npm test)
```

Expected: all green. Worker tests cover every row of the response table in report-service.md
and SC-005 (50 requests from one address give 5 Resend calls).

## 2. Deploy the report service (maintainer, once)

```sh
cd worker
npx wrangler login
npx wrangler deploy                 # prints https://sonata-report.<account>.workers.dev
npx wrangler secret put RESEND_API_KEY
npx wrangler secret put REPORT_TO
```

Test without the app:

```sh
curl -sS -F name=Test -F description='Hello from curl' -F appVersion=0.3.0 -F os=macos \
  -F osVersion=15.6 -F arch=aarch64 -F sentAt=2026-11-01T12:00:00Z -F logsRequested=false \
  -F image=@screenshot.png https://sonata-report.<account>.workers.dev/report
```

Expected: `{"ok":true}` and one email with `screenshot.png` attached within 1 minute.

## 2b. Scenario E — mail app, the default (no service, FR-020 to FR-022)

Build with no `SONATA_REPORT_*` variables. Help → Report Bug…:

1. The primary button reads `Open email`, stays disabled until name and description are filled,
   and `Include logs` is not shown.
2. No image: press it. The mail app opens to `vini.d.science@gmail.com`, with a subject that
   starts `Sonata <version> bug:` and a body with the name, description, and diagnostics. The
   dialog says `Your mail app opened with the report.` The clipboard is unchanged.
3. Paste a PNG screenshot and press it. The dialog says the image is in the clipboard. Paste
   into the email: the image appears.
4. Attach a JPEG and press it. Pasting into the email gives the image (now a PNG).
5. Type more than 1,500 characters with an image attached and press it. The body ends with
   `(cut, full text copied to clipboard)`. Pasting gives the whole text, not the image.
6. Double click the button: one email opens.
7. With no `mailto:` handler (Linux: `xdg-settings`), the dialog says the report was copied and
   shows the address.

Repeat steps 3 to 5 on Linux and Windows: clipboard support differs by web view, so `Attach the
image by hand.` is an accepted result where the image cannot be copied.

## 3. Scenario A — report with screenshot (US1, US2)

Scenarios A to D apply only to a build with a deployed service, which is optional for now.

1. Build with `SONATA_REPORT_URL=https://…/report SONATA_REPORT_FALLBACK_EMAIL=<alias>` set.
2. Help → Report Bug…: the headline reads `Nosso sistema não tem bugs, tem PONTOS DE MELHORIA`.
3. Send stays disabled until name and description are filled.
4. Take a screenshot to the clipboard and paste it into the dialog: preview appears.
5. Drag a JPG onto the dialog: it replaces the preview. Try a 6 MB file and a `.gif`: both
   rejected, preview kept.
6. Send: `Sending…`, then `Thanks! Your report was sent.`; the form is empty after reopening.
7. Email: subject `[Sonata bug] <name>: …`, body with version, OS, architecture,
   `Logs: not requested`, the image attached.

## 4. Scenario B — logs (US3)

Tick `Include logs`, send. Email has `sonata-log.txt` (≤ 1 MB, starts at a line start) and
`Logs: included`. Send again unticked: no log attachment.

## 5. Scenario C — failures (US4)

1. Turn off the network and send: unreachable message; text and image kept.
2. `Send by email instead`: mail app opens to the alias with subject and body; note about
   attaching the image by hand.
3. Send 6 reports within an hour: the 6th shows the rate limit message.

## 6. Scenario D — secrets stay out (US5, SC-004)

```sh
git grep -n "re_[A-Za-z0-9]\{10,\}" ; git grep -n "<destination address>"
strings <installed app binary> | grep -c "<destination address>"   # expect 0
```

## 7. Platforms

Repeat Scenario A steps 2–6 on Linux (AppImage) and Windows (installer): paste with Ctrl+V,
drag and drop, file chooser, Help menu present.

## Results (implementation, 2026-11-01)

- `sh scripts/check.sh` on macOS: green. Rust workspace tests pass (including the new
  `report::*`, `platform::tests`, and `crates/core/tests/report_send.rs`), 237 UI tests,
  release script tests including `no-resend-key.test.sh`.
- Worker (`cd worker && npm run typecheck && npm test`): green, 38 tests (every row of the
  response table, Resend body, log line, limits, and SC-005: 50 parallel requests from one
  address give exactly 5 Resend calls).
- CI run 41331174177 on `013-report-bug` (commit 290213e): `core-and-ui`, `linux-app`,
  `windows`, and `worker` all green. The Linux and Windows jobs build and test
  `platform::os_version()` on those systems.
- Local end to end: core `report::send` → `wrangler dev` (real `workerd`, fake Resend key):
  the Worker parsed the multipart request, passed validation and the limiter, and answered
  502 `email_failed` when Resend refused the fake key; core mapped it to `unreachable`. A
  bad `os` part gave 400 `invalid_field:os`.
- Not yet done (maintainer, T042): deploy with real Cloudflare and Resend accounts, set the
  GitHub variables, and run Scenarios A–D in a dev build on each platform.

## Results (mail app by default, 2026-11-02)

- `sh scripts/check.sh` on macOS: green. 475 Rust tests, 249 UI tests, release script tests.
- Worker (`cd worker && npm run typecheck && npm test`): green, 38 tests; `worker/` is unchanged.
- Clipboard rules probed in a real `WKWebView` (research.md).
- Not yet done: Scenario E by hand in a dev build on each platform.
