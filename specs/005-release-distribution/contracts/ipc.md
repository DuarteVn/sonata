# Contract: Frontend IPC Changes for Feature 005

## New commands

| Command | Input | Output |
|---|---|---|
| `update_state` | — | `UpdateState` (current banner state) |
| `update_check` | `{manual: boolean}` | `UpdateState`; with `manual`, the result is also reported by the caller (up to date / available / error) |
| `update_dismiss` | — | `UpdateState` (`Available` → `Dismissed`) |
| `update_download` | — | `UpdateState` after the download ends; progress arrives as `update://state` with `kind: "downloading"` (about one event per percent) |
| `update_install` | — | never returns on success (app restarts); error → `Failed` |
| `update_set_auto_check` | `{enabled: boolean}` | `void`; saves `autoUpdateCheck`, updates the menu check mark |

`update_install` is called by the frontend only after the existing quit check
(`app_quit_check`) passed or the user confirmed the quit dialog.

## New events

| Event | Payload |
|---|---|
| `update://state` | `UpdateState` |
| `app://check-updates` | — (menu "Check for Updates…"; frontend calls `update_check {manual: true}` and shows the result) |

## Types

```ts
type UpdateState =
  | { kind: "idle" }
  | { kind: "available"; version: string; notes: string; date: string | null }
  | { kind: "dismissed"; version: string }
  | { kind: "downloading"; version: string; received: number; total: number | null }
  | { kind: "readyToInstall"; version: string }
  | { kind: "installing"; version: string }
  | { kind: "failed"; version: string; reason: string }
  | { kind: "blocked"; version: string; reason: string };
```

## Menu (Sonata app menu)

| Item | Kind | Action |
|---|---|---|
| Check for Updates… | normal | emits `app://check-updates` |
| Check for Updates Automatically | check item | toggles `autoUpdateCheck` |

## Changed

`Preferences` gains `autoUpdateCheck: boolean`.
