# Contract: On-Disk File Formats (schemaVersion 1)

Users and future tools read these files, so they are a public contract (constitution IV). Any
incompatible change needs a new `schemaVersion` and a migration.

## Layout

```text
~/.sonata/                         0700  (override: $SONATA_HOME)
├── app-state.json                 0600  type "appState"
├── preferences.json               0600  type "preferences"
├── logs/sonata.log.YYYY-MM-DD     0600
└── workspaces/<workspace-uuid>/   0700
    ├── workspace.json             0600  type "workspace"
    ├── notes/<node-uuid>.md       0600  UTF-8 Markdown, no envelope
    ├── notes/<node-uuid> (conflict).md   0600  written only on conflicts (FR-020a)
    └── terminals/<node-uuid>.history     0600  UTF-8 text with ANSI SGR sequences, no envelope
```

Rules:
- Folders under `workspaces/` whose names are not UUIDs, or that lack `workspace.json`, are
  ignored. Folders with an unreadable `workspace.json` are load failures.
- Every JSON file is written atomically with keys sorted and 2-space indentation.
- A missing `app-state.json` or `preferences.json` means defaults. An invalid one is renamed to
  `<name>.invalid-<timestamp>` and replaced by defaults, with a warning.

## Envelope

```json
{
  "payload": { },
  "schemaVersion": 1,
  "type": "workspace"
}
```

## workspace.json payload

```json
{
  "camera": { "x": 0, "y": 0, "zoom": 1 },
  "color": "#0A84FF",
  "createdAt": "2026-10-21T12:00:00Z",
  "icon": "terminal",
  "id": "6f1c…",
  "isPinned": false,
  "lastOpenedAt": "2026-10-21T12:00:00Z",
  "name": "Sonata",
  "nodes": [
    {
      "content": {
        "terminal": {
          "launchCommand": "claude",
          "name": "Agent 1",
          "saveHistory": true
        }
      },
      "createdAt": "2026-10-21T12:01:00Z",
      "frame": { "height": 420, "width": 640, "x": 100, "y": 80 },
      "id": "9b2e…",
      "modifiedAt": "2026-10-21T12:05:00Z",
      "z": 1
    },
    {
      "content": { "note": {} },
      "createdAt": "2026-10-21T12:02:00Z",
      "frame": { "height": 200, "width": 260, "x": 800, "y": 80 },
      "id": "c41a…",
      "modifiedAt": "2026-10-21T12:02:00Z",
      "z": 2
    }
  ],
  "workingDirectory": "/Users/me/Developer/Sonata"
}
```

## app-state.json payload

```json
{
  "activeWorkspaceId": "6f1c…",
  "cleanShutdown": true,
  "consecutiveEarlyCrashes": 0,
  "outlivedLaunchWindow": true,
  "recentWorkspaceIds": ["6f1c…"],
  "workspaceOrder": ["6f1c…"]
}
```

## preferences.json payload

```json
{
  "autosaveIntervalSeconds": 30,
  "historyMaxRows": 10000,
  "terminalFontSize": 13
}
```

## Terminal history file

- Rows of the terminal's scrollback followed by the visible screen at save time, oldest first,
  one row per line, with SGR color sequences preserved. At most `historyMaxRows` rows and 8 MB;
  the oldest rows are dropped first. Readers stop at 8 MB.
- On restore, the app feeds the file into the new terminal, then prints a dim separator line
  (`── restored history · <time> ──`) before starting the shell.
