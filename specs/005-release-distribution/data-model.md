# Data Model: Release Distribution and Auto-Update

## Preferences (changed)

File `~/.sonata/preferences.json`, envelope type `preferences`, schemaVersion stays 1.

| Field | Type | Default | Rule |
|---|---|---|---|
| `autoUpdateCheck` | bool | `true` | Missing field reads as `true` (serde default), so old files load unchanged. Written only when `false` (optional fields omitted when default). |

## UpdateSchedule (core, in memory)

| Field | Type | Meaning |
|---|---|---|
| `enabled` | bool | Mirrors `autoUpdateCheck`. |
| `last_check` | `Option<Instant>` | Time of the last automatic check (success or failure). |

- `due(now)`: `enabled && (last_check.is_none() || now - last_check >= 24 h)`.
- Launch: `last_check = None`, so the first tick after startup checks.
- Tick: the shell asks every 10 minutes; a manual check does not reset `last_check`.

## UpdateState (core, in memory) — banner state machine

```text
Idle ──check found newer──▶ Available{version, notes, date}
Available ──Later──▶ Dismissed{version}        (hidden until next due check)
Dismissed ──next due check finds newer──▶ Available
Available ──Install──▶ Downloading{version, received, total?}
Downloading ──ok + signature valid──▶ ReadyToInstall{version}
Downloading ──error / bad signature──▶ Failed{version, reason}   (current app kept)
ReadyToInstall ──quit check confirmed──▶ Installing ──▶ (restart)
ReadyToInstall ──quit check cancelled──▶ ReadyToInstall           (banner: "Restart to update")
Available|Failed ──wrong location (R9)──▶ Blocked{version, "Move Sonata to Applications"}
any ──manual check: none newer──▶ unchanged + menu reports "up to date"
```

Rules:
- A background check failure never changes state; it is logged (FR-014).
- A manual check result is always reported (FR-013): up to date, available, or error text.
- Only the newest version is kept; a newer `Available` replaces an older one.
- `pending` (banner visible or dot on the show-sidebar button) = state in
  `Available | Downloading | ReadyToInstall | Failed | Blocked`.

## Release (GitHub, external)

| Item | Name |
|---|---|
| Tag | `vMAJOR.MINOR.PATCH` |
| Disk image | `Sonata_<version>_universal.dmg` |
| Update archive | `Sonata.app.tar.gz` |
| Signature | `Sonata.app.tar.gz.sig` |
| Manifest | `latest.json` — see [contracts/release.md](contracts/release.md) |
