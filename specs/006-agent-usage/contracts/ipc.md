# Contract: Frontend IPC for Feature 006

Types are in [../data-model.md](../data-model.md). All commands are thin wrappers over
`sonata_core::App` usage methods.

## Commands

| Command | Window | Input | Output |
|---|---|---|---|
| `usage_state` | main, settings | — | `UsageState` |
| `usage_refresh` | main, settings | `{providerId?: string, reason: "panelOpen" \| "manual"}` | `UsageState` after the run(s) end; `panelOpen` respects the 1-minute rule, `manual` always runs; no `providerId` = all readable providers ("Refresh all") |
| `usage_set_enabled` | settings, main | `{enabled: boolean}` | `UsageState`; master switch (FR-016) |
| `usage_set_warnings` | settings | `{enabled: boolean}` | `UsageState` (FR-011) |
| `usage_set_provider_enabled` | settings | `{providerId, enabled}` | `UsageState`; turning on a provider without consent returns it with `consent: unasked`, and the UI shows the consent dialog |
| `usage_set_provider_color` | settings | `{providerId, color: "#rrggbb"}` | `UsageState` |
| `usage_answer_consent` | main, settings | `{providerId, allow: boolean, command: string[]}` | `UsageState`; `command` must equal the provider's current command or the call fails (`consent_mismatch`) |
| `usage_providers_folder` | settings | — | `{path: string}`; creates the folder and guide when missing. "Show folder" uses existing `app_reveal_path`; "Copy path" copies `path` in the UI |
| `usage_open_settings` | main | — | `void`; opens or focuses the Settings window (same as menu) |

Errors use the existing `CommandError` shape. No command input or output ever carries a
credential.

## Events

| Event | Payload | Sent when |
|---|---|---|
| `usage://state` | `UsageState` | any change: reading, refresh start/end, setting, definition reload |
| `usage://warning` | `UsageWarning` | a window crosses a level (data-model WarningLedger) and warnings are on; the main window sends the macOS notification |
| `usage://consent` | `ConsentRequest` | a provider would be read but has `consent: unasked` (start-up or switch turned on); main window shows the dialog unless the settings window is focused, which then shows it |
| `app://open-settings` | — | menu "Settings…" (Cmd+,); handled in `src-tauri` by opening the window |

## Types

```ts
type UsageState = {
  enabled: boolean;
  warnings: boolean;
  summary: UsageSummary;
  providers: ProviderState[];      // built-ins first, then custom by name
  providersFolder: string;
};

type ProviderState = {
  id: string;
  name: string;
  kind: "builtIn" | "custom";
  color: string;
  enabled: boolean;
  detection: "found" | "notFound" | "unknown";
  commandPath: string | null;
  consent: "unasked" | "granted" | "denied";
  command: string[];               // what consent approves; shown in the dialog
  definitionError: string | null;
  reading: Reading | null;
  stale: boolean;
  lastError: string | null;
  refreshing: boolean;
};

type Reading = {
  status: "ok" | "noPlanLimits" | "notLoggedIn" | "notInstalled" | "unavailable" | "off";
  plan: string | null;
  account: string | null;
  accountEmail: string | null;     // live only; null after restart until next auth check
  windows: { kind: "short" | "weekly" | "other"; label: string;
             usedPercent: number; resetsAt: string | null }[];
  readAt: string;
  message: string | null;          // e.g. "Run `claude auth login` in a terminal."
};

type UsageSummary = {
  state: "off" | "empty" | "normal" | "warning" | "full";
  percent: number | null;
  providerId: string | null;
  windowKind: "short" | "weekly" | "other" | null;
  resetsAt: string | null;
};

type UsageWarning = {
  providerId: string; providerName: string;
  windowLabel: string; level: 80 | 100; resetsAt: string | null;
};

type ConsentRequest = {
  providerId: string; providerName: string; kind: "builtIn" | "custom";
  command: string[];               // full argv incl. resolved path
  explanation: string;             // built-in: what the probe does and its cost; custom: file path
};
```

## Menu (Sonata app menu)

| Item | Accelerator | Action |
|---|---|---|
| Settings… | Cmd+, | opens or focuses the `settings` window |

## Capabilities

- `capabilities/default.json` (main): `+ allow-usage-state`, `allow-usage-refresh`,
  `allow-usage-set-enabled`, `allow-usage-answer-consent`, `allow-usage-open-settings`.
- `capabilities/settings.json` (new, window `settings`): `core:default`,
  `allow-usage-*` (all usage commands above), `allow-app-reveal-path`.
