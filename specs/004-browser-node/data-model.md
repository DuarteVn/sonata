# Data Model: Browser Node

## NodeContent.browser (new, persisted)

| Field | Type | Rules |
|---|---|---|
| `url` | string | Normalized (R10); default `about:blank`. |
| `title` | string? | Last page title. |

Default frame 1024×720; minimum 320×240. Links, selection, deletion as for other nodes.

## Runtime (shell)

| Item | Fields |
|---|---|
| Browser view | node id, workspace id, webview label `browser-<node>`, load state (`loading` \| `loaded` \| `failed{reason}`), current URL/title, live rect or off screen, element map version |
| Card image | `<data root>/run/browser/<node>.png` |

## Events

`browser://state` `{nodeId, url, title, loading, canGoBack, canGoForward, error?, imageVersion}`
