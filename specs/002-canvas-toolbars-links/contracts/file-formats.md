# Contract: File Format Changes for Feature 002

Extends `specs/001-workspaces-canvas-terminals/contracts/file-formats.md`.

## Layout additions

```text
~/.sonata/
├── roles.json                         0600  type "roles", schemaVersion 1
└── workspaces/<id>/
    └── files/<uuid>.<ext>             0600  note images
```

## workspace.json, schemaVersion 2

Migration v1→v2: add `"links": []`. Other new fields are optional and omitted when unset.

```json
{
  "payload": {
    "links": [
      { "a": "0b1e…", "b": "9f2c…", "createdAt": "2026-10-22T10:00:00Z", "id": "77aa…" }
    ],
    "nodes": [
      {
        "content": {
          "terminal": {
            "accentColor": "#8B5CF6",
            "fontSize": 14,
            "launchCommand": "claude",
            "monitorActivity": true,
            "name": "Lead",
            "orchestrator": true,
            "role": { "name": "Coordinator", "text": "You coordinate the workers." },
            "saveHistory": true
          }
        }
      },
      { "content": { "note": { "color": "blue", "textSize": 4 } } }
    ]
  },
  "schemaVersion": 2,
  "type": "workspace"
}
```

(Only the new fields are shown; the rest is unchanged from 001.)

## roles.json

```json
{
  "payload": {
    "roles": [
      { "createdAt": "2026-10-22T10:00:00Z", "id": "5d1c…", "name": "Coordinator", "text": "You coordinate the workers." }
    ]
  },
  "schemaVersion": 1,
  "type": "roles"
}
```
