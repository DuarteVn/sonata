# Feature Specification: Device Node on the Canvas, Controllable by Linked Agents

**Feature Branch**: `007-device-node`

**Created**: 2026-10-23

**Status**: Draft

## Clarifications

### Session 2026-10-23

- Q: Can linked agents install app packages on the device? → A: Yes, any linked terminal, with
  `sonata device install`; uninstall stays out of scope.
- Q: Are physical Android phones over USB in scope? → A: No; emulators and simulators only. USB
  phones are a later feature.

**Input**: User description: "Device node on the canvas, like the browser node (004). Android emulator first (AVD from the Android SDK), iOS simulator if possible. I see the screen live in the node and interact with mouse and keyboard. Terminals joined by a link control the device with `sonata device ...`: screenshot, tap, swipe, type, key (back/home), launch app, read the accessibility tree with numbered elements (like `sonata browser read`). Detect the installed SDK and emulators; warn when missing, never install anything. Clean room: behavior only as reference, nothing copied."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run an Android emulator next to the agents (Priority: P1)

The user picks the Device tool, clicks on the canvas, and chooses one of the Android virtual
devices installed on the Mac. The emulator boots and its screen appears live inside the node.
The user clicks to tap, drags to swipe, scrolls, and types with the keyboard. Buttons on the node
send Back, Home, and Recent apps. No separate emulator window is needed.

**Why this priority**: Mobile work today means a separate emulator window outside the canvas.
Having the device on the canvas, beside the agents that build the app, is the core value.

**Independent Test**: With the Android SDK and one virtual device installed, add a device node,
boot it, open Settings on the device by clicking, type in its search field, and press Back.

**Acceptance Scenarios**:

1. **Given** the Android SDK and at least one virtual device are installed, **When** the user
   places a device node, **Then** a picker lists the installed virtual devices by name, with
   their Android version.
2. **Given** a device is chosen, **When** it boots, **Then** the node shows boot progress, then
   the live screen within 90 seconds on a cold boot.
3. **Given** the live screen, **When** the user clicks, drags, scrolls, or types, **Then** the
   device receives a tap, a swipe, a scroll, or text at the matching position, and the screen
   shows the result in under 300 ms.
4. **Given** the node is selected, **When** the user uses Back, Home, or Recent apps on the node,
   **Then** the device receives that key.
5. **Given** the node is resized, **When** the screen redraws, **Then** it keeps the device's
   aspect ratio and input positions stay correct.
6. **Given** the workspace is reopened after a restart, **When** it loads, **Then** the node
   remembers its device and offers to boot it again (it does not boot by itself).

---

### User Story 2 - Clear guidance when tools are missing (Priority: P1)

The Mac has no Android SDK, or the SDK has no virtual device. Sonata says what is missing and how
to get it, and installs nothing.

**Why this priority**: The maintainer's own Mac has neither SDK nor Xcode today; without this,
the feature fails silently for most first-time users.

**Independent Test**: On a Mac without the Android SDK, place a device node and confirm the
message and the instructions; then point Sonata to an SDK folder and confirm the devices appear.

**Acceptance Scenarios**:

1. **Given** no Android SDK is found, **When** the user places a device node, **Then** the node
   says "Android SDK not found", lists the places Sonata looked, links to the official install
   instructions, and offers "Choose SDK folder…".
2. **Given** the SDK exists but has no virtual device, **When** the picker opens, **Then** it says
   so and explains how to create one with the SDK's own tools.
3. **Given** any missing tool, **When** Sonata reports it, **Then** Sonata downloads, installs,
   and changes nothing on the system.
4. **Given** the user installs the SDK while Sonata runs, **When** they press "Check again",
   **Then** the devices appear without a restart.

---

### User Story 3 - Agents test the app through the device (Priority: P2)

A linked agent builds a mobile app and tests it: it launches the app, reads the screen as a
numbered list of elements, taps element 4, types into a field, swipes, presses Back, and takes a
screenshot to confirm the result.

**Why this priority**: This is what makes the device node more than a viewer: agents can close
the build-and-test loop without the user.

**Independent Test**: Link a terminal to a booted device and run the command sequence below; each
step changes the device as described, and the user sees it live in the node.

**Acceptance Scenarios**:

1. **Given** a terminal linked to a device node, **When** it runs `sonata list`, **Then** the
   device appears with its name, platform, and state (off, booting, ready).
2. **Given** a ready device, **When** the terminal runs `sonata device read Pixel`, **Then** it
   prints the foreground app, the screen size, and a numbered list of visible elements with their
   kind, label or text, and whether they can be tapped or edited.
3. **Given** the numbered list, **When** the terminal runs `sonata device tap Pixel 4`, **Then**
   the device taps the center of element 4; `sonata device tap Pixel 120 480` taps those screen
   coordinates.
4. **Given** a ready device, **When** the terminal runs `sonata device type Pixel "hello"`,
   **Then** the text goes to the focused field; `--into 6` taps element 6 first.
5. **Given** a ready device, **When** the terminal runs `sonata device swipe Pixel up` (or with
   start and end coordinates and a duration), **Then** the device performs that swipe.
6. **Given** a ready device, **When** the terminal runs `sonata device key Pixel back` (also
   `home`, `recents`, `enter`, `delete`), **Then** the device receives that key.
7. **Given** an installed app, **When** the terminal runs `sonata device launch Pixel
   com.example.app`, **Then** the app starts in the foreground; `sonata device apps Pixel` lists
   the installed apps that can be launched.
8. **Given** a ready device, **When** the terminal runs `sonata device screenshot Pixel`, **Then**
   a PNG of the screen is saved, readable only by the user, and its path is printed; it works
   when the node is off screen.
9. **Given** a device that is off, **When** the terminal runs `sonata device boot Pixel`, **Then**
   it boots and the command returns when the device is ready (default limit 3 minutes).
10. **Given** a terminal that is not linked to the device, **When** it runs any `sonata device`
    command on it, **Then** the command fails with the same "not linked" error as other nodes.

---

### User Story 4 - iOS simulator in the same node (Priority: P3)

On a Mac with Xcode, the picker also lists iOS simulators, and the node and the commands work the
same way, as far as the simulator allows.

**Why this priority**: The user asked for iOS "if possible". Android comes first; the maintainer's
Mac has no Xcode today, so iOS cannot be verified yet.

**Independent Test**: On a Mac with Xcode and one simulator, boot it in a node, tap and type in
Settings, and run `sonata device read` and `screenshot`.

**Acceptance Scenarios**:

1. **Given** Xcode with simulators, **When** the picker opens, **Then** it lists them with their
   iOS version, in a section apart from Android.
2. **Given** a command the iOS simulator cannot support with the tools available, **When** an
   agent runs it, **Then** it fails with "not supported on iOS simulator" and no other effect.
3. **Given** no Xcode, **When** the picker opens, **Then** iOS shows "Xcode not found" with the
   official instructions, and Android still works.

### Edge Cases

- The emulator crashes or is closed outside Sonata: the node shows "Device stopped" with a Boot
  button; commands fail with "device not running".
- The same virtual device is already running outside Sonata: Sonata attaches to it instead of
  starting a second copy, and does not stop it on node delete.
- Two device nodes choose the same virtual device: the second one is refused with a clear message.
- The Mac lacks hardware acceleration for the emulator: the node shows the emulator's own error.
- An element number from an earlier `read` no longer exists after the screen changed: the command
  fails and says to run `read` again.
- The accessibility tree is empty (for example a game or a video): `read` returns no elements and
  says so; coordinate taps and screenshots still work.
- A device node is off screen or zoomed out: the screen is not streamed live; the node shows the
  last screen image, as browser nodes do.
- The user quits Sonata with a booted device: the quit confirmation lists running devices, and
  Sonata stops the devices it started.

## Requirements *(mandatory)*

### Functional Requirements

**Node and live screen**

- **FR-001**: The tool bar MUST offer a Device tool (shortcut D) that places a device node sized
  to a phone screen, which opens a device picker.
- **FR-002**: The device picker MUST list the installed Android virtual devices (and iOS
  simulators when Xcode is present) with name, platform, and OS version, plus a "Check again"
  action.
- **FR-003**: The node MUST show the device name, state (off, booting, ready, stopped, error), and
  Boot, Stop, Back, Home, and Recent apps controls; its floating toolbar MUST offer link,
  screenshot, stop, and delete.
- **FR-004**: The device screen MUST run without a separate emulator window and MUST be shown live
  in the node at a minimum of 15 frames per second when the node is live.
- **FR-005**: The live rule MUST match browser nodes: live only when visible at a legible zoom,
  with nothing on top and no gesture running, and at most 2 live devices at a time; otherwise the
  node shows the last screen image.
- **FR-006**: Mouse and keyboard input on the live screen MUST map to device taps, long presses
  (press and hold 500 ms or more), swipes, scrolls, text, and keys, with correct positions at any
  node size and zoom.
- **FR-007**: The node MUST save the chosen device with the workspace and MUST NOT boot it on load
  without a user or agent action.
- **FR-008**: Deleting a device node MUST stop the device if Sonata started it; quitting Sonata
  MUST list running devices in the quit confirmation and stop the ones Sonata started.

**Detection**

- **FR-009**: Sonata MUST find the Android SDK from `ANDROID_HOME`, `ANDROID_SDK_ROOT`, the default
  Android Studio location, or a folder the user chose; the chosen folder persists in preferences.
- **FR-010**: When a required tool is missing, Sonata MUST say which tool, where it looked, and
  link to the official instructions. Sonata MUST NOT download, install, or change system tools.

**Agent control**

- **FR-011**: `sonata device` commands (list, boot, stop, read, tap, swipe, type, key, launch,
  apps, install, screenshot) MUST work only for terminals linked to the device node; names resolve among
  linked device nodes by name or by the short id shown in `sonata list`.
- **FR-012**: `read` MUST return the foreground app, the screen size, and a numbered list of
  visible elements (kind, label or text, identifier when present, bounds, tappable, editable),
  at most 100 KB; numbers stay valid until the next `read` or screen change detected by Sonata.
- **FR-013**: `tap` and `type --into` MUST accept an element number from the last `read` or screen
  coordinates; `swipe` MUST accept a direction or start and end points with a duration.
- **FR-014**: Commands that change the screen MUST wait until the screen is still for 500 ms, at
  most 5 seconds, before returning, so a following `read` sees the result.
- **FR-015**: `screenshot` MUST save a PNG readable only by the user, in a temporary folder or at
  `--out`, and print the path; it MUST work when the node is not live.
- **FR-016**: While an agent controls a device, the node MUST show "controlled by <terminal>" and
  the link MUST be highlighted, as with browser nodes.
- **FR-017**: Any linked terminal MUST be able to install an app package with `sonata device
  install <device> <file>` (Android `.apk`; iOS `.app` for simulators). The command waits for the
  install to finish (at most 3 minutes) and prints the installed app id. Uninstalling is out of
  scope.

**Scope**

- **FR-018**: Only emulators and simulators are in scope. Physical phones connected by USB MUST
  NOT appear in the picker in this feature.

### Key Entities

- **Device node**: a canvas node bound to one virtual device; holds the device id, platform, and
  frame; saved with the workspace.
- **Device runtime**: the running emulator or simulator, with its state and whether Sonata started
  it; in memory only.
- **Element map**: the numbered elements from the last `read` of a device, in memory.
- **Tool locations**: the SDK and Xcode paths Sonata found or the user chose; saved in preferences.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From placing a node to a usable screen takes under 90 seconds on a cold boot and
  under 15 seconds when the device was booted before (quick boot).
- **SC-002**: A click on the live screen shows its effect in under 300 ms on an Apple Silicon Mac.
- **SC-003**: An agent completes a login test on a sample app (launch, read, type two fields, tap,
  read, screenshot) with no help from the user.
- **SC-004**: On a Mac without the SDK, 100% of users who follow the node's message reach the
  official instructions in one click, and Sonata makes zero changes to the system.
- **SC-005**: With one live device and 8 live terminals, the canvas keeps panning smoothly (no
  visible stutter in a 30-second pan test).

## Assumptions

- The user installs the Android SDK (usually with Android Studio) and creates at least one
  virtual device before testing; Sonata only guides.
- Apple Silicon Macs use ARM system images; Intel Macs use x86 images with hardware acceleration.
- Android virtual devices are the first and fully supported platform; iOS simulators are best
  effort and need Xcode.
- Clean room: device control uses the platforms' own public developer tools; behavior is inspired
  by the reference, but no names, text, or code are copied.
- Constitution principle VII lists device simulators as out of scope "until specified"; this
  specification is that step, and the plan records it.
- Out of scope: recording videos, GPS and sensor simulation, network shaping, rotating the device,
  multi-touch gestures such as pinch, uninstalling apps, and physical phones over USB.
