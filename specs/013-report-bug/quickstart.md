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

## 3. Scenario A — report with screenshot (US1, US2)

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
3. Build without `SONATA_REPORT_URL`: Send shows `This build cannot send reports directly.`
4. Build without the alias: the fallback copies the text and says so.
5. Send 6 reports within an hour: the 6th shows the rate limit message.

## 6. Scenario D — secrets stay out (US5, SC-004)

```sh
git grep -n "re_[A-Za-z0-9]\{10,\}" ; git grep -n "<destination address>"
strings <installed app binary> | grep -c "<destination address>"   # expect 0
```

## 7. Platforms

Repeat Scenario A steps 2–6 on Linux (AppImage) and Windows (installer): paste with Ctrl+V,
drag and drop, file chooser, Help menu present.
