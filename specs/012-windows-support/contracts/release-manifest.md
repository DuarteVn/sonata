# Contract: release files and update manifest

## Release assets (per version `vX.Y.Z`)

| Asset | Platform | Notes |
|---|---|---|
| `Sonata_X.Y.Z_universal.dmg` | macOS | unchanged |
| `Sonata.app.tar.gz`, `.sig` | macOS updater | unchanged |
| `Sonata_X.Y.Z_x64-setup.exe` | Windows installer and updater | NSIS, per user |
| `Sonata_X.Y.Z_x64-setup.exe.sig` | Windows updater | signed with the project updater key |
| `latest.json` | all | one manifest |

## `latest.json`

```json
{
  "version": "X.Y.Z",
  "notes": "…",
  "pub_date": "…",
  "platforms": {
    "darwin-aarch64": { "signature": "…", "url": ".../Sonata.app.tar.gz" },
    "darwin-x86_64":  { "signature": "…", "url": ".../Sonata.app.tar.gz" },
    "windows-x86_64": { "signature": "…", "url": ".../Sonata_X.Y.Z_x64-setup.exe" }
  }
}
```

- `darwin-*` entries keep exactly today's shape and values.
- `windows-x86_64` is present only when both Windows files exist; `latest-json.mjs` fails if
  only one of the two exists.
- The publish step runs only after the macOS and Windows build jobs both succeed, so no
  release has a partial manifest.
