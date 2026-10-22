# Quickstart Results: Feature 004

**Date**: 2026-10-22

## Automated

- `scripts/check.sh`: Rust tests (URL rules, persistence, every `sonata browser` command with a
  fake host, permissions), Vitest (page scripts in jsdom, live-set planning, browser node UI).
- Playwright e2e on the mocked frontend.

## Real app (dev build, temp data, local test page)

A linked shell ran, by itself: `open` → title and URL; `read` → text and numbered elements
(Email, Password, Sign in); `fill 1`, `fill 2`, `click 3` → page showed "Wrong password for
me@x.dev"; `screenshot --out` → 32 KB PNG; `eval` from a non-orchestrator → refused (exit 3);
`list` → shows the browser with its URL. Card images are captured (26 snapshots in one run).

Found and fixed: launch commands longer than the terminal's 1024-byte input line were cut when
typed before the shell was ready; they are now typed once the prompt appears.

## Needs a person

Visual checks: pages follow nodes while dragging, switch to cards during pan/zoom and under
dialogs, at most 4 live; shared login inside a workspace and not across workspaces; devtools.
