# Contract: Release Artifacts and Manifest (macOS + Linux)

Extends `specs/005-release-distribution/contracts/release.md`. macOS items are unchanged.

## Trigger

Unchanged: tag `v*.*.*` push, or manual run with an existing tag.

## Jobs (`.github/workflows/release.yml`)

| Job | Runner | Needs | Does |
|---|---|---|---|
| `checks` | (ci.yml) | — | Same checks as CI, including `linux-app` |
| `draft` | ubuntu-latest | checks | Version check, key check, `gh release create --draft --generate-notes` |
| `macos` | macos-14 | draft | Unchanged build steps; uploads `.dmg`, `Sonata.app.tar.gz`, `.sig` to the draft |
| `linux` | ubuntu-22.04 | draft | Installs build packages; `tauri build --bundles appimage,deb`; uploads `.AppImage`, `.AppImage.sig`, `.deb` |
| `publish` | ubuntu-latest | macos, linux | Reads both `.sig` files from the draft, builds `latest.json`, uploads it, un-drafts, marks latest |

A failure in any build job stops `publish`; the draft stays unpublished and the previous release
remains "latest".

## Release files

| File | Platform | Purpose |
|---|---|---|
| `Sonata_<v>_universal.dmg` | macOS | Install (unchanged) |
| `Sonata.app.tar.gz` + `.sig` | macOS | Update (unchanged) |
| `Sonata_<v>_amd64.AppImage` | Linux x86_64 | Install and update |
| `Sonata_<v>_amd64.AppImage.sig` | Linux x86_64 | Update signature |
| `Sonata_<v>_amd64.deb` | Linux x86_64 | Manual install |
| `latest.json` | all | Update manifest |

## `latest.json`

```json
{
  "version": "0.3.0",
  "notes": "…",
  "pub_date": "2026-11-01T12:00:00Z",
  "platforms": {
    "darwin-aarch64": { "signature": "…", "url": "https://github.com/<repo>/releases/download/v0.3.0/Sonata.app.tar.gz" },
    "darwin-x86_64": { "signature": "…", "url": "https://github.com/<repo>/releases/download/v0.3.0/Sonata.app.tar.gz" },
    "linux-x86_64": { "signature": "…", "url": "https://github.com/<repo>/releases/download/v0.3.0/Sonata_0.3.0_amd64.AppImage" }
  }
}
```

## `scripts/release/latest-json.mjs`

- Existing flags unchanged: `--version --notes-file --sig-file --url [--date]` (macOS entries).
- New optional pair: `--linux-sig-file <path> --linux-url <https url>`. Both or neither; one alone
  is an error. Adds `linux-x86_64`.
- `buildLatestJson({ …, linux?: { sig, url } })`.

## Secrets

Unchanged: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, used by the `macos`
and `linux` build jobs only.
