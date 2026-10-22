# Contract: IPC for Browser Nodes

| Command | Input | Output |
|---|---|---|
| `node_create_browser` | `{workspaceId, frame, url?}` | `Node` |
| `browser_navigate` | `{nodeId, input}` | normalized URL or `InvalidAddress` |
| `browser_action` | `{nodeId, action: "back"\|"forward"\|"reload"\|"stop"\|"inspect"\|"external"}` | — |
| `browser_layout` | `{items: {nodeId, rect: {x,y,width,height} \| null}[]}` | — |

Event `browser://state` (see data-model.md). New error code `InvalidAddress`.
