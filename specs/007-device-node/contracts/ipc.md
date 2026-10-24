# Contract: IPC for Device Nodes

| Command | Input | Output |
|---|---|---|
| `node_create_device` | `{workspaceId, frame}` | `Node` (empty `deviceId`; the node opens the picker) |
| `device_tools` | — | `ToolReport` (reads files only; "Check again" calls it again) |
| `device_choose_sdk` | `{path}` | `ToolReport` (path saved in preferences when valid) |
| `device_open_help` | `{topic: "android" \| "avd" \| "xcode"}` | — opens one of three fixed official pages |
| `device_select` | `{nodeId, platform, deviceId}` | `Node` or `DeviceInUse` |
| `device_state` | `{nodeId}` | `DeviceStateEvent` (current state when a node mounts) |
| `device_boot` / `device_stop` | `{nodeId}` | — (progress through `device://state`) |
| `device_stream` | `{nodeId, width, height, channel}` | — starts or resizes the live stream; frames are raw bytes: 12-byte header (`width`, `height`, `seq`, u32 LE) + RGBA |
| `device_unstream` | `{nodeId}` | card image path or `null` (written from the last frame, 0600) |
| `device_frame_ack` | `{nodeId}` | — next frame may be sent |
| `device_input` | `{nodeId, input}` | — `input`: `{touch: {x, y, down}}` \| `{wheel: {x, y, dx, dy}}` \| `{text}` \| `{key}` (device pixels; key `back`/`home`/`recents` or a W3C name) |

Event `device://state` (see data-model.md). New error codes `DeviceInUse`,
`DeviceNotRunning`, `ToolMissing`.
