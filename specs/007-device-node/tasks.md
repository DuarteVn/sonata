---
description: "Task list for feature 007: device node"
---

# Tasks: Device Node

**Input**: `specs/007-device-node/` (plan.md, spec.md, research.md, data-model.md, contracts/)

**Tests**: required (constitution III). Write each test task first and see it fail.

**Real emulator gate**: tasks marked ⛔ need Android Studio with one AVD on the Mac. Everything
else runs with fakes and fixtures. Stop before the first ⛔ task and ask the maintainer to install.

US2 (detection) runs before US1 although both are P1: it needs no emulator and US1's picker
depends on it.

## Phase 1: Setup

- [X] T001 Add `tonic` (client, no TLS; `server` feature in dev-dependencies only), `prost`, `quick-xml`, `png` to `Cargo.toml` workspace deps and `crates/core/Cargo.toml`; record versions and why in plan Complexity Tracking
- [X] T002 [P] Add error codes `DeviceInUse`, `DeviceNotRunning`, `ToolMissing` in `crates/core/src/error.rs` and `ui/src/ipc/types.ts`
- [ ] T003 [P] Create fixture tree `crates/core/tests/fixtures/device/`: SDK folders (valid, missing `adb`, empty), AVD home with two AVDs (`Pixel_8_API_34.ini` + `config.ini` with `avd.ini.displayname`, `image.sysdir.1=system-images/android-34/google_apis/arm64-v8a/`, `hw.lcd.width=1080`, `hw.lcd.height=2400`), discovery `pid_123.ini`, uiautomator XML dumps (Settings home, login form with EditText/password, empty hierarchy, status-line suffix), `query-activities --brief` output, `simctl list --json` output

## Phase 2: Foundational (blocks all stories)

- [X] T004 [P] Tests `crates/core/tests/device_model.rs`: `NodeContent::Device{platform, deviceId, name?, osVersion?}` round trip (empty optionals omitted, `platform` is `"android"` | `"ios"`); `create_device` default frame 380×820, minimum 200×300; links and deletion like other nodes; loading a workspace never boots (FR-007); preference `androidSdkPath` omitted when unset
- [X] T005 Implement `NodeContent::Device` in `crates/core/src/model/node.rs`, `App::create_device` in `crates/core/src/app.rs`, `androidSdkPath` in `crates/core/src/model/preferences.rs` (drop `Copy`, fix call sites)
- [X] T006 [P] Tests `crates/core/tests/device_grpc.rs` with a fake tonic `EmulatorController` server: `streamScreenshot` request carries format RGBA8888 + width/height and yields frames; `sendTouch` down/up, `sendKey` text and key, `getScreenshot` PNG, `setVmState`; every call sends the `authorization` token header; a wrong token is rejected
- [X] T007 Implement `crates/core/src/device/grpc.rs`: hand-written prost messages (`ImageFormat`, `Image`, `TouchEvent`, `Touch`, `KeyboardEvent`, `WheelEvent`, `VmRunState`, field tags from the public proto, Apache 2.0 attribution comment) and an `EmulatorClient` using tonic's generic `Grpc` client (research R10)
- [X] T008 [P] Tests `crates/core/tests/device_still.rs`: signature from a frame (64 px wide); still after 500 ms without frames; still when frames differ in < 0.5% of pixels; not still while frames change more; 5 s limit returns "not settled"
- [X] T009 Implement `crates/core/src/device/still.rs` (research R5)
- [X] T010 Implement `crates/core/src/device/mod.rs`: `DeviceBackend` trait (tools, list, boot, stop, state, stream, input, read, apps, launch, install, screenshot), runtime registry keyed by node (refuse a second node for the same device with `DeviceInUse`), states `off|booting|ready|stopped|error`, `startedBySonata`, `controller`; a `FakeBackend` in `crates/core/tests/common/` for later tests

## Phase 3: US2 Clear guidance when tools are missing (P1)

**Goal**: without the SDK or AVD, the node says what is missing and where Sonata looked, links to official pages, and changes nothing. **Independent test**: quickstart steps 1–2 on this Mac.

- [ ] T011 [P] [US2] Tests `crates/core/tests/device_tools.rs`: SDK order preference → `ANDROID_HOME` → `ANDROID_SDK_ROOT` → `~/Library/Android/sdk`; valid only with executable `emulator/emulator` and `platform-tools/adb`; `looked` lists every candidate; AVD home order `ANDROID_AVD_HOME` → `ANDROID_EMULATOR_HOME/avd` → `ANDROID_USER_HOME/avd` → `~/.android/avd`; AVD parsing (display name, API 34 → "Android 14", screen size); no AVD → `NoAvd{avdHome}`; Xcode missing when `xcrun --find simctl` fails (injected runner); detection runs no install or SDK management command (assert the runner's call log)
- [ ] T012 [US2] Implement `crates/core/src/device/tools.rs` and `crates/core/src/device/avd.rs` (research R6)
- [ ] T013 [US2] Shell commands `device_tools{recheck}`, `device_choose_sdk{path}` and event `device://tools` in `src-tauri/src/commands/device.rs`; register in `src-tauri/build.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`; IPC wrappers in `ui/src/ipc/`
- [ ] T014 [P] [US2] UI tests `ui/tests/deviceNode.test.tsx`: tool D places a device node; "Android SDK not found" card with looked paths, official link (`https://developer.android.com/studio/install`), "Choose SDK folder…", "Check again"; no-AVD message with link `https://developer.android.com/studio/run/managing-avds`; iOS section "Xcode not found" with `https://developer.apple.com/xcode/` while Android still lists
- [ ] T015 [US2] `ui/src/nodes/DeviceNode.tsx` (picker and missing-tool cards), Device tool (D) in `ui/src/canvas/ToolBar.tsx` and `ui/src/canvas/keyboard.ts`, `node_create_device` command

## Phase 4: US1 Run an Android emulator next to the agents (P1)

**Goal**: pick an AVD, boot it, see and use the live screen in the node. **Independent test**: quickstart steps 3–5, 8, 9.

- [ ] T016 [P] [US1] Tests `crates/core/tests/device_emulator.rs` (fake process runner): emulator argv contains `-avd <name> -port <even console> -grpc <port> -grpc-use-token -qt-hide-window -no-boot-anim` and never the token; port choice skips used ports; token and ports read from discovery `pid_<pid>.ini`; boot polls `getprop sys.boot_completed` until `1`, 3-minute timeout → `error`; process exit → `stopped`; stop runs `adb -s <serial> emu kill`, kills after 20 s; attach to a running AVD sets `startedBySonata=false` and never stops it; missing token → `reduced` mode; physical serials (not `emulator-*`) ignored (FR-018)
- [ ] T017 [US1] Implement `crates/core/src/device/emulator.rs` and the boot/stop/state parts of `crates/core/src/device/adb.rs`; Android `DeviceBackend` in `crates/core/src/device/mod.rs`
- [ ] T018 [P] [US1] UI tests `ui/tests/devicePointer.test.ts`: node rect + zoom + letterbox → device pixels at several sizes and zooms (FR-006, AS5); click → touch down/up; hold ≥ 500 ms → long press; drag → move stream; wheel → wheel event; printable keys → text, Enter/Backspace/arrows → key names
- [ ] T019 [P] [US1] UI tests `ui/tests/liveLayout.test.ts`: generalized live rule with limit; devices ≤ 2 live (selected first, then area); browsers keep their 004 behavior (move `browserLayout.test.ts` cases)
- [ ] T020 [US1] `ui/src/nodes/devicePointer.ts` and `ui/src/canvas/liveLayout.ts` (browserLayout uses it with limit 4)
- [ ] T021 [US1] Shell: `device_select`, `device_boot`, `device_stop`, `device_stream` (raw `ipc::Channel`, 12-byte header + RGBA, next frame only after `device_frame_ack`), `device_unstream` (card PNG at `<data root>/run/device/<node>.png`, 0600), `device_input`, `device_layout`, event `device://state` in `src-tauri/src/commands/device.rs`
- [ ] T022 [US1] `DeviceNode.tsx` live screen (`<canvas>` + `putImageData`, ack per frame), state chrome (off/booting/ready/stopped/error), Boot/Stop/Back/Home/Recents, floating toolbar link/screenshot/stop/delete, card image when not live; restart shows "Boot" without booting (AS6)
- [ ] T023 [US1] Delete node stops a device Sonata started; quit confirmation lists running devices and stops Sonata-started ones (FR-008) in `crates/core/src/app.rs` and the existing quit flow
- [ ] T024 [US1] ⛔ Real AVD: confirm `-qt-hide-window` works headless (else switch to `-no-window`), discovery key names for gRPC port/token, token header form, key names `GoBack`/`GoHome`/`AppSwitch` (else adb keyevent fallback); fix code and fixtures to match; note results in `research.md`
- [ ] T025 [US1] ⛔ Real AVD: measure fps and click-to-screen latency at 100% and 200% zoom; if < 15 fps or ≥ 300 ms, apply the JPEG fallback from research R2; record in `specs/007-device-node/quickstart-results.md`

## Phase 5: US3 Agents test the app through the device (P2)

**Goal**: linked terminals run every `sonata device` command. **Independent test**: quickstart steps 6–7.

- [ ] T026 [P] [US3] Tests `crates/core/tests/device_ui_tree.rs`: fixture XML → numbered elements (document order, filter rules, text-only children merged into tappable parent, kinds, labels, bounds, password, editable); app and screen size; status-line suffix stripped; empty hierarchy → "no elements" message; 100 KB cap with "… N more elements"; "could not get idle state" → "screen never settled" error
- [ ] T027 [US3] Implement `crates/core/src/device/ui_tree.rs` and element map with signature (research R4)
- [ ] T028 [P] [US3] Tests `crates/core/tests/cli_device.rs` with `FakeBackend`: every subcommand in `contracts/cli.md`; unlinked → exit 3; name and `d-xxxx` resolution; `sonata list` shows name, platform, state; `tap N` uses element center; stale map → "the screen changed; run read again"; unknown number → "element 7 not found; run read again"; `type --into N` taps first; swipe directions 70% → 30%; `key` names map; boot waits for ready with `--timeout`; screen-changing commands wait for stillness (≤ 5 s); `screenshot` file mode 0600 and printed path; `install` rejects non-files, `..` paths, wrong extensions; not-running → "device not running"
- [ ] T029 [P] [US3] Tests `crates/core/tests/device_adb.rs` (fake runner + fixtures): `apps` parsing and sorting; `launch` "No activities found" → "not installed or not launchable"; `install` success detection, app id from `aapt2`, then package diff, then "update of an existing app"; 3-minute limit
- [ ] T030 [US3] `device` subcommands in `crates/core/src/messaging/commands.rs`; `adb.rs` apps/launch/install/dump; screenshot via gRPC `getScreenshot` PNG to `<data root>/run/shots/`
- [ ] T031 [US3] "controlled by <terminal>" marker and link glow during device commands (FR-016), reusing the browser event shape
- [ ] T032 [US3] ⛔ Real AVD: run the login test of SC-003 on a sample app with an agent; fix parsing gaps found on real dumps (add them as fixtures)

## Phase 6: US4 iOS simulator, detect and list plus simctl basics (P3)

**Goal**: decided 2026-10-23: list simulators; boot/stop/screenshot/launch/apps/install via simctl; other commands "not supported on iOS simulator". **Independent test**: needs Xcode (not on this Mac); unit tests only until then.

- [ ] T033 [P] [US4] Tests `crates/core/tests/device_ios.rs` (fake runner + fixture JSON): list available simulators with iOS version; boot/shutdown/screenshot/launch/listapps/install argv; tap/swipe/type/key/read → "not supported on iOS simulator" with no runner call; `.app` must be a directory
- [ ] T034 [US4] Implement `crates/core/src/device/ios.rs` and wire iOS into the picker (separate section) and the node (screenshot poll 2–4 fps, input disabled with a note)

## Phase 7: Polish

- [ ] T035 README section: device node, requirements (Android Studio + AVD), what Sonata never installs, `sonata device` commands; update `scripts/check.sh` if new suites need it
- [ ] T036 Run `sh scripts/check.sh`; quickstart without SDK (steps 1–2) and record in `quickstart-results.md`
- [ ] T037 ⛔ Full quickstart with a real AVD, including SC-005 pan test; record results

## Dependencies

- Setup → Foundational → US2 → US1 → US3 → US4 → Polish.
- US3 core tests (T026, T028, T029) need only Foundational + `FakeBackend`; they can run in parallel with US1 UI work.
- ⛔ tasks need the maintainer's AVD; T024 should run as soon as US1 code exists, since its findings can change `emulator.rs` and `grpc.rs`.

## Parallel examples

- Foundational: T004, T006, T008 together (different test files).
- US1: T016, T018, T019 together.
- US3: T026, T028, T029 together.

## Implementation strategy

MVP = Phases 1–4 without ⛔ (code complete and tested with fakes), then T024–T025 on the real AVD.
Next US3 (agent control), then iOS basics, then polish. Commit per task or small group.
