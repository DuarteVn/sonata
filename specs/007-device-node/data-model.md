# Data Model: Device Node

## NodeContent.device (new, persisted)

| Field | Type | Rules |
|---|---|---|
| `platform` | `"android"` \| `"ios"` | Required. |
| `deviceId` | string | AVD name (Android) or simulator UDID (iOS). Empty until the picker is used. |
| `name` | string? | Display name at selection time (AVD display name or simulator name). |
| `osVersion` | string? | For example `Android 14`, `iOS 18.2`. |

Default frame 380×820 (phone screen plus node chrome); minimum 200×300. The screen area keeps the
device aspect ratio inside the node (letterboxed). Additive content key: the workspace stays at
its current schema version (same reasoning as 004 R9). Loading never boots a device (FR-007).

## Preferences (changed)

| Field | Type | Rules |
|---|---|---|
| `androidSdkPath` | path? | Folder chosen with "Choose SDK folder…"; omitted when unset. `Preferences` stops being `Copy`. |

## Tool locations (runtime, core)

`ToolReport { android: Found{sdk, emulator, adb} | Missing{looked: [path]}, avds: [Avd] | NoAvd{avdHome}, ios: Found{xcrun} | Missing }`

`Avd { name, displayName, apiLevel?, osVersion?, abi?, screen: (width, height)? }`

## Device runtime (in memory, core)

| Field | Notes |
|---|---|
| node id, platform, device id | One runtime per node; a second node choosing the same device is refused. |
| `state` | `off` → `booting` → `ready`; `stopped` (process exited); `error{message}` |
| `startedBySonata` | `false` when attached to an emulator found running (R7); then never stopped by Sonata. |
| serial, console port, gRPC port, token | Android only. Token is never logged or put on a command line. |
| `mode` | `full` (gRPC) \| `reduced` (adb only, R7) |
| `screen` | Device size in pixels (from the first frame or `wm size`). |
| `controller` | Terminal id while an agent command runs (FR-016). |
| last frame | Latest RGBA frame for the live stream; signature frame (64 px) for stillness and element maps. |

State transitions: `boot` from `off`/`stopped`/`error`; `stop` from `booting`/`ready`; process exit
from any state → `stopped`; boot timeout (3 min) → `error`.

## Element map (in memory, core)

`ElementMap { signature, screen, app, elements: [Element] }`

`Element { number (1-based), kind, label, resourceId?, bounds (x1,y1,x2,y2), tappable, editable, scrollable, password, focused }`

Replaced on each `read`; invalid when the current signature differs (R4, R5).

## Runtime files

| Item | Path |
|---|---|
| Card image | `<data root>/run/device/<node>.png` (0600) |
| Agent screenshots | `<data root>/run/shots/` or `--out` (0600) |

## Events

`device://state` `{nodeId, state, message?, controller?, mode, screen?, imageVersion}`
`device://tools` `ToolReport` (after "Check again" or a chosen SDK folder)
