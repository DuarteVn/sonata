# Performance Notes: Feature 001

**Date**: 2026-10-21 · **Machine**: Apple Silicon (arm64), macOS 27.0.1

## Core (measured)

`cargo test --release -p sonata-core --test scale -- --ignored --nocapture`: 100 terminals in
4 workspaces, each printing a line every 50 ms.

| Metric | Result |
|---|---|
| All 100 terminals producing output | yes (100/100) |
| Terminals still updating after 10 s | 100/100 |
| Core CPU (Sonata process only, shells excluded) | 1.7 % of one core |
| Peak RSS of the test process | 31 MB |
| History save of 100 terminals (autosave tick) | 0.79 s, off the UI thread |

Conclusion: the core side of SC-004 holds with wide margin.

## Frontend (automated, mocked IPC)

`e2e/scale.spec.ts` with 30 seeded terminals: zoomed out, every terminal is a preview and no
xterm.js instance exists; zoomed in, terminals become live; never more than 12 use WebGL.

## Frontend frame times (not yet measured)

SC-003 (smooth pan/zoom with 30 streaming terminals, immediate echo) needs the real app on
screen. Run `scripts/bench-terminals.sh 30` and `scripts/bench-terminals.sh 100 4`, then record
frame times with Safari Web Inspector (Timelines → Rendering Frames) attached to the Sonata
webview. Results go here.

## Follow-ups

- History save time grows with scrollback size. If autosave ticks show up in profiles, save
  histories in smaller batches or only for terminals whose output changed (already the case)
  on a separate, lower-frequency timer.
