# Data Model: Windows Support

No new files and no schema bump. One optional field and some runtime-only values.

## TerminalContent.shell (persisted, optional)

| Field | Type | Rule |
|---|---|---|
| `shell` | object, optional | Omitted when the terminal uses the default shell (always omitted on macOS). |
| `shell.kind` | `"pwsh" \| "powershell" \| "cmd" \| "gitBash" \| "wsl"` | Which shell starts the terminal. |
| `shell.distro` | string, optional | WSL distribution name; required when `kind` is `"wsl"`, else omitted. |

- Written in `workspace.json` with the usual sorted keys and omitted empties.
- Older Sonata versions ignore the field (no `deny_unknown_fields`), so schema stays 3.
- On macOS the field is ignored and the login shell is used.
- If the chosen shell is not installed (for example the file moved to another PC), the terminal
  starts in the default shell and shows a warning line, like a missing working folder today.

## ShellChoice (runtime, from discovery)

| Field | Type | Meaning |
|---|---|---|
| `kind` | as above | Shell type. |
| `distro` | string, optional | WSL distribution. |
| `label` | string | UI text, for example "PowerShell 7", "Windows PowerShell", "Git Bash", "Ubuntu (WSL)", "Command Prompt". |
| `path` | path | Executable to start (`wsl.exe` for WSL). |
| `default` | bool | True for the shell used when a terminal has no `shell`. |

Returned by a new `terminal_shells` Tauri command; empty list on macOS (the dialog hides the
picker).

## Command channel address (runtime)

- `SONATA_SOCKET` keeps its name. Value: socket file path on Unix (unchanged), pipe name
  `\\.\pipe\sonata-<16 hex>` on Windows. The 16 hex digits hash the user SID and the data root,
  so two users or two data roots never share a pipe.

## Platform capabilities (runtime)

| Capability | macOS | Windows |
|---|---|---|
| iOS devices | yes (with Xcode) | no: device list has no iOS section, `ios: { supported: false }` |
| Custom usage providers | yes (sandboxed) | no: provider reading is "unavailable" with the R11 message |
| Browser page image | yes | yes when capture works, else unavailable message |
| Shell picker | no (login shell) | yes |
