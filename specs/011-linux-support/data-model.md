# Data Model: Linux Support

No persisted schema changes. Workspace, preference, and usage files keep their envelopes and
`schemaVersion`. Linux adds generated files and caches only.

## Files on disk (Linux additions)

| Path | Kind | Written by | Notes |
|---|---|---|---|
| `~/.sonata/run/bash/bashrc` | generated | core at start (like `run/zsh/*`) | Rewritten each start; private dir (0700) |
| `~/.sonata/browser-data/<workspace-uuid>/` | cache | WebKitGTK | One per workspace; removed when the workspace is deleted |
| `~/.local/bin/sonata` | copy of bundled CLI | app at start (Linux) | Only replaced when it is a Sonata CLI (marker check), mode 0755 |

## In-memory types

### `platform::EnvFix` (core)

- `remove: Vec<String>` — variables to unset for child processes.
- `set: Vec<(String, String)>` — variables to override (filtered path lists).
- Built by `platform::host_env(vars: &[(String, String)]) -> EnvFix`. Empty when `APPIMAGE` is not
  set (always empty on macOS).
- Rule: remove `APPIMAGE`, `APPDIR`, `ARGV0`, `OWD`; for every other variable whose value contains
  `$APPDIR`, split on `:`, drop entries that start with `$APPDIR`, and set the rest joined by `:`,
  or remove the variable when nothing is left.

### `platform::Sandbox` (core)

- `MacSandboxExec { exec: PathBuf, profile: String }` (current behavior).
- `Bubblewrap { exec: PathBuf, args: Vec<String> }` — `args` from
  `platform::linux::bwrap_args(home, sonata_root, runtime_dir, exists)`, where `exists` is a
  predicate so the function is pure.
- `Unavailable { reason: String }` — the provider shows `reason`, nothing runs.

### `device::tools::ToolReport` (changed)

- `ios: Option<IosReport>` — `None` on Linux, serialized as `null`. macOS always `Some`.

### `UpdateState::Blocked` (unchanged shape)

- New `reason` texts on Linux (see [contracts/platform.md](contracts/platform.md)).

### Install type (shell, derived)

- From `tauri::utils::platform::bundle_type()`: `AppImage` → self-update allowed when the
  AppImage's folder is writable; `Deb` or none → blocked with the release-page reason.
