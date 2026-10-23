# Implementation Plan: Device Node on the Canvas, Controllable by Linked Agents

**Branch**: `007-device-node` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)

## Summary

Device nodes show an Android emulator (AVD) on the canvas. Sonata starts the emulator with its
window hidden and a localhost gRPC port protected by a per-instance token, streams scaled RGBA
frames into a `<canvas>` in the node, and sends touches, wheel, text, and keys over the same gRPC
channel. Linked terminals control the device with `sonata device …`; `read` uses `uiautomator
dump` and numbers elements like `sonata browser read`. Detection finds the SDK and AVDs and never
installs anything. iOS simulators are detected and listed; simctl basics (boot, stop, screenshot,
launch, apps, install, slow live view) come last and other commands report "not supported". See
[research.md](research.md).

This specification is the step principle VII asks for ("device simulators … out of scope until
specified"); no constitution amendment is needed (research R14).

## Technical Context

**Language/Version**: Rust 1.99, TypeScript 7, React 19.

**Primary Dependencies**: new in `crates/core`: `tonic` (gRPC client, no TLS), `prost` (hand-written
messages, no `protoc`), `quick-xml` (already in `Cargo.lock`), `png` (already in `Cargo.lock`).
System tools used, never installed: Android SDK `emulator`, `adb`, optional `aapt2`; `xcrun simctl`.

**Storage**: new node content `device`; preference `androidSdkPath`; runtime images in
`<data root>/run/`.

**Testing**: core with a fake `DeviceBackend` (permissions, CLI parsing, name resolution, waits,
errors) and fixtures (SDK folder trees, AVD `.ini` files, uiautomator XML, discovery `.ini`,
`query-activities` output); a fake emulator gRPC server in a test (tonic server feature in
dev-dependencies only) for frame streaming, touch and key messages, and token header; Vitest for
coordinate mapping, live rule, node UI, picker messages. Real emulator checks are manual
(quickstart) and gated on the maintainer installing Android Studio.

**Target Platform**: macOS 15+, Apple Silicon and Intel.

**Project Type**: desktop app (Tauri shell, Rust core, React UI) plus CLI.

**Performance Goals**: ≥ 15 fps live (FR-004); click to visible effect < 300 ms (SC-002); cold boot
< 90 s, quick boot < 15 s (SC-001); smooth pan with 1 live device and 8 live terminals (SC-005).

**Constraints**: at most 2 live devices; stream ≤ 1.2 MP per frame; latest-frame-only delivery with
UI acknowledgment; nothing installed or changed on the system; gRPC on localhost with token only.

**Scale/Scope**: a few devices per workspace; Android full, iOS partial (P3).

## Constitution Check

| Principle | Result |
|---|---|
| I Clean room | Pass: own code and wording; only public Android/Apple tool interfaces; proto field tags copied as interface facts with Apache 2.0 attribution in a comment. |
| II Core/shell | Pass: detection, processes, adb, gRPC, parsing, permissions in `core::device`; shell forwards frames and intents only. |
| III Test-first | Pass: fakes and fixtures for all logic; a fake gRPC server for the client; manual checks only for the real emulator. |
| IV Files | Pass: additive node content and one optional preference; runtime images under `run/`. |
| V Security | Pass: emulator gRPC on localhost with per-instance token, token read from file never argv; links required; screenshots 0600; install path validated as a regular `.apk`/`.app` file; Sonata stops only emulators it started. |
| VI Distribution | Pass: no paid service; no SDK bundled. |
| VII Simplicity | Pass with justification: this spec is the "specified" step; new dependencies below. |

Post-design re-check: unchanged, all pass.

## Project Structure

### Documentation (this feature)

```text
specs/007-device-node/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/cli.md, contracts/ipc.md
└── tasks.md            # /speckit-tasks
```

### Source Code

```text
crates/core/src/device/mod.rs         # DeviceBackend trait, runtime registry, states, errors
crates/core/src/device/tools.rs       # SDK, AVD, Xcode detection (R6)
crates/core/src/device/avd.rs         # .ini parsing, API level → Android version
crates/core/src/device/emulator.rs    # spawn/attach/stop, ports, discovery files (R1, R7)
crates/core/src/device/adb.rs         # adb calls: boot check, apps, launch, install, dump (R8)
crates/core/src/device/grpc.rs        # hand-written prost messages + tonic calls (R10)
crates/core/src/device/ui_tree.rs     # uiautomator XML → numbered elements, read text (R4)
crates/core/src/device/still.rs       # stillness and screen signature (R5)
crates/core/src/device/ios.rs         # simctl (R11)
crates/core/src/messaging/commands.rs # `device` subcommands, list entries, permissions
crates/core/src/model/node.rs         # NodeContent::Device
crates/core/src/model/preferences.rs  # androidSdkPath
crates/core/tests/device_*.rs, crates/core/tests/fixtures/device/
src-tauri/src/commands/device.rs      # IPC commands, frame Channel forwarding
ui/src/nodes/DeviceNode.tsx           # screen canvas, chrome, picker, missing-tool card
ui/src/nodes/devicePointer.ts         # node → device coordinates, pointer/wheel/key mapping
ui/src/canvas/liveLayout.ts           # browser rule generalized with a limit (R13)
ui/tests/device*.test.ts(x)
```

**Structure Decision**: existing workspace layout; new `core::device` module mirrors `core::browser`
but also owns the processes, because none of it needs Tauri.

## Complexity Tracking

| Addition | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `tonic` + `prost` in core | Emulator gRPC is the only channel with ≥ 15 fps frames and few-ms input (R2, R3). | `adb screencap`/`input` give 2–5 fps and 200 ms–1 s per tap; screenrecord needs H.264 decoding and restarts; scrcpy needs a bundled third-party server. |
| Hand-written prost messages | No `protoc` on contributor Macs. | `tonic-build` needs `protoc`; `protox` adds more crates than ~100 lines of structs. |
| `quick-xml`, `png` as direct deps | Parse uiautomator XML; write card images. | Both already in `Cargo.lock` through Tauri; a hand-written XML parser is error-prone. |
| Raw frame Channel with acknowledgment | Keeps latency bounded when the UI is slower than the device. | Fire-and-forget queues frames and adds seconds of lag. |
| Reduced adb mode for externally started emulators | Spec edge case: attach instead of a second copy. | Refusing to attach contradicts the spec; JWT auth support is more code for a rare case. |
