# Contract: Shell commands, events, and UI

## Tauri commands

```ts
kimiIntegration(): Promise<KimiIntegration>;              // status, path, block (data-model)
kimiIntegrationApply(): Promise<KimiIntegration>;          // UI confirms before calling
kimiIntegrationRemove(): Promise<KimiIntegration>;         // UI confirms before calling
dismissKimiNotice(status: "off" | "outdated"): Promise<void>;
kimiWatch(on: boolean): Promise<void>;                     // Settings open/closed (research R9)
kimiOpenSettings(): Promise<void>;                         // notice's Set up: Settings at Agents
```

`kimiWatch(true)` keeps the config file watched while the Settings window shows the Kimi
section; the app also watches while any Kimi terminal exists.

Errors (`AppError` codes):

- `KimiUnavailable`: folder missing or file unreadable. Text says what to do ("Run Kimi once
  first", or the read error).
- `KimiHooksConflict`: the file defines `hooks` in a form Sonata cannot add to. Text names the
  line.
- `KimiConfigInvalid`: `kimi doctor` rejected the file before any change; nothing was written.
  Text includes Kimi's message.
- `KimiCheckFailed`: `kimi doctor` rejected the new file; the backup was restored. Text includes
  Kimi's message.

Every change writes a backup first; the returned value carries `lastBackup`.

## Event

`kimi://integration` (`KimiIntegrationChanged`, payload `KimiIntegration`): sent when the computed
status changes (after Apply/Remove, or when the file changes on disk while watched).
`settings://section` (payload `"usage" | "agents"`): sent to an open Settings window to switch
section.

## Types (`ui/src/ipc/types.ts`)

```ts
type KimiStatus = "on" | "outdated" | "off" | "unavailable";
interface KimiIntegration {
  status: KimiStatus;
  configPath: string;
  block: string;
  message?: string;
  canApply: boolean;
  lastBackup?: string;
  validated: boolean;
}
// Preferences
kimiNoticeDismissed?: "off" | "outdated";
// TerminalContent.conversation.agent (feature 009)
agent: "claude" | "codex" | "kimi";
```

## UI

- **New-terminal dialog**: preset "Kimi Code" (command `kimi`), with the same "not found" hint as
  other presets.
- **Terminal node notice** (in the title bar of Kimi terminals only, status `off` or `outdated`, not dismissed for
  that status): one line, "Kimi: busy/done tracking and conversation restore are off." (or "…are
  out of date.") with **Set up** (opens Settings › Agents › Kimi Code) and **Dismiss**.
- **Settings › Agents › Kimi Code** (new section in the Settings window):
  - Status line and config file path.
  - The block in a read-only code view, with one sentence per hook and a **Copy lines** button.
  - **Apply** (status `off` or `outdated`, `canApply`): confirmation "Sonata will back up and
    change <path>: it adds only the lines shown. Kimi outside Sonata is not affected." →
    Apply.
  - **Remove** (status `on` or `outdated`): confirmation "Sonata will back up <path> and remove only
    its own lines." → Remove.
  - After a change: "Backup saved to <path>. Restart running Kimi terminals to use the change."
  - `message` shown when `canApply` is false or status is `unavailable`.
