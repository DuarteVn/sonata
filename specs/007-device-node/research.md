# Research: Device Node

**Feature**: 007-device-node | **Date**: 2026-10-23 · Emulator gRPC API checked against the
public `emulator_controller.proto` (Android Studio tools repository, Apache 2.0). This Mac has no
Android SDK or Xcode yet: items marked **verify** are confirmed on a real AVD before the tasks that
depend on them (see tasks.md).

## R1. Running the emulator without a window

**Decision**: Sonata starts the emulator itself:

```text
<sdk>/emulator/emulator -avd <name> -port <console> -grpc <grpc-port> -grpc-use-token
                        -qt-hide-window -no-boot-anim
```

`-qt-hide-window` keeps the full emulator UI pipeline but never shows its window; the official
command-line page states that Android Studio uses `-qt-hide-window -grpc-use-token` to show the
emulator inside its own tool window, which is the same situation as a canvas node. Fallback if
it misbehaves on macOS: `-no-window` (documented headless mode). **verify** both on Apple Silicon.

- Ports: the first even console port in 5554–5682 not used by `adb devices` and bindable on
  localhost; adb serial is `emulator-<console>`; gRPC port is the first bindable port from 8554.
- Quick boot (SC-001, 15 s): no `-no-snapshot-*` flags, so the default snapshot is loaded and
  saved. Stop is `adb -s <serial> emu kill` (graceful, saves the snapshot); if the process is
  still alive after 20 s, Sonata kills it.
- State: `off` → `booting` (process started) → `ready` when `adb -s <serial> shell getprop
  sys.boot_completed` returns `1` (polled every 1 s); `stopped` when the process exits;
  `error{message}` with the emulator's last stderr lines (covers missing hardware acceleration).

**Alternatives**: showing the emulator's own window and positioning it over the canvas (rejected:
separate window, FR-004 says no window).

## R2. Streaming the screen to the node (≥ 15 fps, SC-002 < 300 ms)

**Decision**: gRPC `EmulatorController.streamScreenshot(ImageFormat) → stream Image`, format
`RGBA8888`, with `width`/`height` set to the node's screen area in physical pixels (capped at the
device's own size), so the emulator scales before sending. The proto says a new frame is delivered
whenever the device produces one, so an idle screen sends nothing.

Path to the UI: core reads the stream on a tokio task and keeps only the latest frame per device
(no queue). The shell forwards it through a Tauri `ipc::Channel` as raw bytes (12-byte header
`width, height, seq` as little-endian u32, then RGBA). The node draws it on a `<canvas>` with
`putImageData`. The shell sends the next frame only after the UI acknowledges the previous one
(`device_frame_ack`), so a slow UI drops frames instead of building latency.

Budget at 100% zoom on a Retina screen: node screen area about 360×800 CSS px = 720×1600 px =
4.6 MB per frame. The stream width is capped at 1 physical pixel per CSS pixel × 2 and at 1.2 MP;
measured fps and latency are recorded in quickstart results (**verify**). If raw RGBA cannot reach
15 fps through the channel, the fallback is to encode JPEG in core before sending (one small
encoder crate, added only then).

| Option | fps | Latency | Cost |
|---|---|---|---|
| **gRPC `streamScreenshot`** | device frame rate, scaled by emulator | low (frame on change) | gRPC client in core |
| `adb exec-out screencap` in a loop | ~2–5 | 200–500 ms per frame | none, but fails FR-004 |
| `adb shell screenrecord --output-format=h264 -` | 30+ | 100–300 ms | H.264 decoding in the UI, 3-minute limit per run |
| scrcpy-style server pushed to the device | 60 | ~50 ms | bundling a third-party server jar |
| emulator WebRTC module | 30+ | low | needs extra services, complex |

Card image (node not live): the last frame is written as PNG to
`<data root>/run/device/<node>.png` (mode 0600) and shown like browser cards.

## R3. Input

**Decision**: gRPC for every live and agent input; `adb` only as fallback (R7).

| Input | gRPC call |
|---|---|
| tap, long press, swipe, drag | `sendTouch` with one touch (`identifier 0`, `pressure > 0` while down, `0` on release); the UI sends pointer moves as they happen, so long press and swipe follow the mouse |
| scroll wheel | `injectWheel` (client stream) |
| typed text | `sendKey` with `text` |
| Enter, Backspace, Tab, arrows, Escape | `sendKey` with W3C `key` names |
| Back, Home, Recent apps | `sendKey` with `GoBack`, `GoHome`, `AppSwitch` (**verify**; fallback `adb shell input keyevent KEYCODE_BACK / KEYCODE_HOME / KEYCODE_APP_SWITCH`) |

Agent `tap` is touch down then up 50 ms later; `swipe` sends moves every 16 ms over the duration
(default 300 ms); a direction swipe goes from 70% to 30% of the screen on that axis, centered.

**Rationale**: a gRPC call is one localhost HTTP/2 message (a few ms). `adb shell input` starts a
new Java process on the device for every call; published measurements range from about 200 ms to
over 1 s per tap, which alone breaks SC-002 and makes swipes jerky.

## R4. Accessibility tree and element numbers

**Decision**: `adb -s <serial> exec-out uiautomator dump /dev/tty` prints the window hierarchy XML
to stdout (followed by a status line that is stripped). Parsed in core with `quick-xml`.

- Each `node` gives `class`, `text`, `content-desc`, `resource-id`, `package`, `bounds`
  (`[x1,y1][x2,y2]`), `clickable`, `long-clickable`, `checkable`, `scrollable`, `focused`,
  `password`, `enabled`.
- An element is numbered when its bounds have area and intersect the screen, and it is tappable
  (`clickable`, `long-clickable`, or `checkable`), editable (class ends in `EditText`), scrollable,
  or has `text` or `content-desc`. A text-only node inside a numbered tappable parent is not
  numbered; its text joins the parent's label.
- Numbers follow document order (depth first), starting at 1.
- Label: `text`, else `content-desc`, else joined child texts, else the `resource-id` part after
  `:id/`. Kind from class: `EditText` → input, `Button`/`ImageButton` → button, `CheckBox` →
  checkbox, `Switch` → switch, `ImageView` → image, `TextView` → text, scrollable → list, else view.
- Foreground app: `package` of the first node; screen size: bounds of the root node.
- Output capped at 100 KB (FR-012); the remainder is cut with a "… N more elements" line.
- uiautomator waits up to 10 s for the app to be idle and prints "could not get idle state" when
  animations never stop. Sonata reports that error as "screen never settled; try again or use
  coordinates".

The element map (number → center point, label) is kept in memory per device with the screen
signature (R5) at the time of `read`. `tap N` and `type --into N` fail with "the screen changed;
run read again" when the current signature differs.

## R5. Waiting until the screen is still (FR-014)

**Decision**: while an agent command runs, core keeps a second small `streamScreenshot` (64 px
wide, RGBA) open. The screen is still when 500 ms pass with no frame, or when every frame in the
last 500 ms differs from the previous one in fewer than 0.5% of pixels (ignores the blinking text
cursor). Limit 5 s. The 64-px frame is also the screen signature for R4. Pure comparison function,
unit tested.

## R6. SDK and virtual device detection (FR-009, FR-010)

**Decision**: SDK candidates in order: preference `androidSdkPath`, `$ANDROID_HOME`,
`$ANDROID_SDK_ROOT`, `~/Library/Android/sdk` (Android Studio default). A candidate is valid when
`emulator/emulator` and `platform-tools/adb` are executable files. Every looked-at path is kept for
the "where we looked" message.

AVDs: directory from `$ANDROID_AVD_HOME`, else `$ANDROID_EMULATOR_HOME/avd`, else
`$ANDROID_USER_HOME/avd`, else `~/.android/avd`. Each `<name>.ini` has `path=`; that directory's
`config.ini` gives `avd.ini.displayname`, `image.sysdir.1` (`system-images/android-34/...`),
`hw.lcd.width`, `hw.lcd.height`. API level maps to an Android version with a small table. Pure
parsing over fixture files; no process is run to list devices.

Missing-tool messages link to the official pages: Android Studio install
(`https://developer.android.com/studio/install`) and creating a virtual device
(`https://developer.android.com/studio/run/managing-avds`). Sonata never runs `sdkmanager` or
`avdmanager`. Note: running `adb` starts the adb server daemon if it is not running; Sonata does
not stop it.

## R7. Devices already running outside Sonata

**Decision**: running emulators are found through the emulator's discovery files
`~/Library/Caches/TemporaryItems/avd/running/pid_<pid>.ini` (also `~/.android/avd/running/`), which
hold `key=value` lines including the AVD name, serial port, and gRPC port and token (**verify**
exact key names). If the chosen AVD is already running, Sonata attaches and marks it "not started
by Sonata" (never stopped on node delete or quit). If the discovery file has no gRPC token (for
example a JWT-only instance), the node runs in reduced mode: `adb exec-out screencap -p` every
300 ms and `adb shell input` for input, with a note "Started outside Sonata: reduced frame rate".

The token is read from the discovery file, never passed on a command line (constitution V).

## R8. Apps, launch, install

- `apps`: `adb shell cmd package query-activities --brief -a android.intent.action.MAIN -c
  android.intent.category.LAUNCHER`, keeping lines of the form `package/activity`, unique packages,
  sorted.
- `launch <package>`: `adb shell monkey -p <package> -c android.intent.category.LAUNCHER 1`;
  "No activities found" → error "not installed or not launchable".
- `install <file.apk>`: the path is resolved against the caller terminal's working directory and
  must be a regular `.apk` file. `adb -s <serial> install -r <file>`; success when output contains
  `Success`. App id: `aapt2 dump packagename <file>` from the newest `build-tools` when present;
  otherwise the difference of `pm list packages` before and after; otherwise "installed (update of
  an existing app)". Limit 3 minutes.

## R9. Screenshots for agents

**Decision**: gRPC `getScreenshot` with format `PNG` at full device size, written to
`<data root>/run/shots/` (or `--out`) with mode 0600, as in 004. Works when the node is not live.

## R10. gRPC client in Rust without `protoc`

**Decision**: `tonic` (HTTP/2 client only, no TLS, localhost) with `prost` messages written by hand
for the few messages Sonata uses (`ImageFormat`, `Image`, `TouchEvent`, `Touch`, `KeyboardEvent`,
`WheelEvent`, `VmRunState`), with field tags taken from the public proto. Calls use tonic's
generic client (`Grpc::unary`, `Grpc::server_streaming`, `Grpc::client_streaming`) with the
paths `/android.emulation.control.EmulatorController/<method>`. The token goes in the
`authorization: Bearer <token>` metadata (**verify** header form).

**Rationale**: no build-time code generation, so no `protoc` on contributors' Macs (we must not
ask them to install tools beyond Rust and Node); only about 100 lines of message structs.

**Alternatives**: `tonic-build` with `protoc` (rejected: extra system tool); `protox` pure-Rust
compiler (rejected: more dependencies than the hand-written structs); a raw HTTP/2 client (rejected:
reinventing gRPC framing).

## R11. iOS simulator (P3, decided 2026-10-23: detect and list only, plus simctl basics)

**Decision**: `xcrun simctl list devices available --json` lists simulators; "Xcode not found"
when `xcrun --find simctl` fails (link `https://developer.apple.com/xcode/`). As a last, optional
phase: `boot`, `stop` (`shutdown`), `screenshot` (`simctl io <udid> screenshot`), `launch`,
`apps` (`listapps`), `install` (`.app`). The live view is a screenshot poll (2–4 fps; below
FR-004, accepted for P3). `tap`, `swipe`, `type`, `key`, `read` fail with "not supported on iOS
simulator": there is no public tool for them without installing third-party software.

## R12. Who owns what (core/shell)

**Decision**: everything that does not need Tauri lives in `crates/core/src/device/`: detection,
AVD parsing, emulator processes, adb calls, gRPC client, uiautomator parsing, element map,
stillness, and the `DeviceBackend` trait (Android implementation, iOS implementation, fake for
tests). The shell only creates the frame `Channel`, forwards frames and node intents, and writes
nothing on its own. Coordinates from the node to device pixels are a pure TypeScript function
(the UI knows the node rect, zoom, and letterboxing).

## R13. Live rule

**Decision**: reuse the browser rule (004 R7) in one function parameterized by limit: device nodes
are live only when fully visible, zoom ≥ 0.55, uncovered, no gesture, and at most 2 live (selected
first, then area). Non-live devices stop their full stream and show the card image.

## R14. Constitution VII

Principle VII lists device simulators as out of scope "until specified". This specification is
that step; no amendment is needed. The emulator's gRPC port is a listener owned by the emulator,
bound to localhost and protected by a per-instance token, which matches principle V.
