# Contract: IPC for Device Nodes

| Command | Input | Output |
|---|---|---|
| `node_create_device` | `{workspaceId, frame}` | `Node` (empty `deviceId`; the node opens the picker) |
| `device_tools` | `{recheck: bool}` | `ToolReport` |
| `device_choose_sdk` | `{path}` | `ToolReport` (path saved in preferences when valid) |
| `device_select` | `{nodeId, platform, deviceId}` | `Node` or `DeviceInUse` |
| `device_boot` / `device_stop` | `{nodeId}` | — (progress through `device://state`) |
| `device_stream` | `{nodeId, width, height, channel}` | — starts or resizes the live stream; frames are raw bytes: 12-byte header (`width`, `height`, `seq`, u32 LE) + RGBA |
| `device_unstream` | `{nodeId}` | — (writes the card image) |
| `device_frame_ack` | `{nodeId, seq}` | — next frame may be sent |
| `device_input` | `{nodeId, input}` | — `input`: `{touch: {x, y, down}}` \| `{wheel: {x, y, dx, dy}}` \| `{text}` \| `{key}` (device pixels) |
| `device_layout` | `{items: {nodeId, live: bool}[]}` | — live set from the UI (R13) |

Events `device://state`, `device://tools` (see data-model.md). New error codes `DeviceInUse`,
`DeviceNotRunning`, `ToolMissing`.
