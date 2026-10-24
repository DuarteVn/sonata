# Quickstart Results: Feature 007

**Date**: 2026-10-23

## Automated (no SDK, no Xcode on this Mac)

- `scripts/check.sh` passes: fmt, clippy, all Rust tests, type check, Vitest, release script tests.
- Core, with fakes and fixtures: emulator gRPC client against a fake `EmulatorController` server
  (token header, touch, keys, wheel, screenshot, scaled RGBA stream); SDK/AVD/Xcode detection
  (reads files only, runs no program); emulator start/attach/stop with fake processes (hidden
  window flags, free ports, token from the discovery file, never on argv, physical phones
  ignored); device runtimes (boot → ready, crash → stopped, start error, stop only devices Sonata
  started, quit check); live stream (latest frame only, next frame after the UI's ack, private
  card PNG); stillness detection; uiautomator parsing and numbering; every `sonata device`
  command, permissions, stale element numbers, install path checks; adb parsing and the reduced
  adb-only mode; iOS via simctl (list, boot, attach, stop, screenshot, apps, launch, install,
  input refused with no effect).
- UI (Vitest): Device tool (D), picker and missing-tool cards, iOS list, live/card switching,
  Back/Home/Recents, boot/stop, "controlled by", screen geometry and input mapping, live rule
  (at most 2 devices), quit dialog with devices.

Known flaky test, unrelated to this feature: `multi_workspace` (busy-terminal timing) failed
once in a full run and passed on 3 reruns and the next full run.

## Not done yet

- Quickstart steps 1–2 in the real app (visual check of the missing-SDK card) — needs a person.
- Steps 3–10 need Android Studio with one AVD (tasks T024, T025, T032, T037).
