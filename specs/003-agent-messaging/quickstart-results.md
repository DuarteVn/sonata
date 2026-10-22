# Quickstart Results: Feature 003

**Date**: 2026-10-22

## Automated

| Check | Result |
|---|---|
| `scripts/check.sh`: fmt, clippy, 105 Rust tests (real socket, real shells, fake Claude Code and Codex), typecheck, 59 Vitest tests | pass |
| `npm run e2e` (9 Playwright tests) | pass |
| Dev app: a seeded terminal runs `sonata ask B "echo real-app-ok"` and `sonata list` on its own | pass, clean answer |
| Installed app `/Applications/Sonata.app` with the bundled CLI (`Contents/Resources/bin/sonata`): same check | pass |

Covered by tests: link permissions, ambiguous names, parallel asks, keys, check, reply, loop
refusal, timeout with partial answer, on-demand start of an exited terminal, forged tokens,
malformed and oversized requests, private socket, session environment, `sonata run` session
files and hooks without touching `~/.claude`, recruit/dismiss/link/unlink, notes by title and id,
conflict copy while the user edits, notifications.

## Needs a person (open)

- Real Claude Code: create a Claude Code terminal (preset), link a shell to it, run
  `sonata ask Claude "Reply with exactly: PONG"`. The first run in a folder may show Claude's
  "trust this folder" prompt; answer it by hand once.
- Real Codex (not installed on the author's machine): the `-c notify=…` override and the typed
  instructions.
- macOS notification permission prompt on the first `sonata notify`, and selecting the terminal
  after clicking the notification.
- Link glow and "asked by" marker during a real ask; message log from the terminal toolbar.
