# Quickstart Results: Feature 002

**Date**: 2026-10-22

## Automated

| Check | Result |
|---|---|
| `scripts/check.sh` (fmt, clippy, 90 Rust tests, typecheck, 57 Vitest tests) | pass |
| `npm run e2e` (9 Playwright tests on WebKit, mocked IPC) | pass |
| Real app launch (`tauri dev`, temp `SONATA_HOME`): frontend bootstraps through the new per-command permissions and the custom menu | pass |
| Release build installed to `/Applications/Sonata.app` | done |

## Needs a person (open)

Run the scenarios in [quickstart.md](quickstart.md) in the installed app, in particular:

- Preset detection marks programs you do not have (needs your real login shell).
- Cmd+W closes the selected item and never the window; Cmd+Q asks when programs run.
- Pictures show inside notes (asset protocol in the packaged app).
- Activity dot and attention badge with a real `sleep 5`.
- Opening a v2 workspace with an older (001) build reports "newer version".
