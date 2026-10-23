# Data Model: Agent Usage Meter

All types live in `crates/core/src/usage/`. JSON names are camelCase. Times on the wire are RFC
3339 strings in UTC.

## ProviderId

String. Built-in: `claude`, `codex`. Custom: the definition `id` (`^[a-z0-9][a-z0-9-]{0,31}$`),
which must not equal a built-in id.

## Provider (registry entry)

| Field | Type | Notes |
|---|---|---|
| `id` | ProviderId | |
| `name` | string | "Claude Code", "Codex", or the definition `name` |
| `kind` | `builtIn` \| `custom` | Settings tag (FR-017) |
| `color` | `#rrggbb` | preference override, else definition color, else built-in default |
| `enabled` | bool | per-agent switch (preference) |
| `commandPath` | string? | absolute path found by detection (built-in) or `command[0]` (custom) |
| `detection` | `found` \| `notFound` \| `unknown` | `unknown` when detection timed out |
| `consent` | Consent | see below |
| `definitionError` | string? | custom only: why the file was rejected |

Rule: a provider is **readable** when master switch on, `enabled`, `detection = found`,
`consent = granted` (matching current argv for custom), and no `definitionError`.

## Consent

`unasked` \| `granted { approvedCommand: string[] }` \| `denied`.

- Built-in: `approvedCommand` holds the fixed probe argv (without the resolved path), so a
  change of probe flags in a future release asks again.
- Custom: holds the definition `command`. A definition whose `command` differs from
  `approvedCommand` is treated as `unasked`.
- `denied` keeps `enabled = false`; turning the switch on asks again (FR-012).

## Window

| Field | Type | Notes |
|---|---|---|
| `kind` | `short` \| `weekly` \| `other` | from source: Claude `five_hour` → short, `seven_day` → weekly; Codex by `window_minutes` (≤ 1 day short, 6–8 days weekly, else other); custom as declared |
| `label` | string | "5-hour", "Weekly", or custom label |
| `usedPercent` | number | 0–100, may exceed 100 only from custom/spend sources; shown clamped to 100 |
| `resetsAt` | time? | absent when the source gives none |

Validation: `usedPercent` finite and ≥ 0; drop windows whose `resetsAt` is in the past (the
period is over; the next reading brings the new one).

## Reading

| Field | Type | Notes |
|---|---|---|
| `providerId` | ProviderId | |
| `status` | ReadingStatus | |
| `plan` | string? | "Team", "Pro", … (FR-002) |
| `account` | string? | Claude org name or "Personal"; never email in the cache |
| `accountEmail` | string? | memory only, never cached or logged (`#[serde(skip)]` in cache type) |
| `windows` | Window[] | empty unless `status = ok` |
| `readAt` | time | when the source produced the values (Codex rollout: file mtime) |
| `message` | string? | user-facing reason for non-ok statuses |

`ReadingStatus`: `ok` \| `noPlanLimits` \| `notLoggedIn` \| `notInstalled` \| `unavailable`
\| `off`.

Derived, not stored:

- **stale** = `status = ok` and `now - readAt > 15 min` (FR-004, US1 scenario 5). The panel keeps
  showing the last good windows and marks them stale; a failed refresh does not replace a good
  reading in the cache, it sets `lastError` on the provider state.
- **limitReached** = any window `usedPercent ≥ 100`.

## ProviderState (what the UI gets)

`{ provider: Provider, reading: Reading?, lastError: string?, refreshing: bool }`

## UsageSummary (compact meter, FR-005)

`{ state: off | empty | normal | warning | full, percent: number?, providerId?, windowKind?,
resetsAt? }` — the window with the highest `usedPercent` among readable providers with an `ok`
(including stale) reading. `warning` at ≥ 80, `full` at ≥ 100, `off` when the master switch is
off, `empty` when nothing has a reading.

## WarningLedger (window periods, R7)

Key: `(providerId, windowKind, resetsAt)`. Value: `sent: none | warn80 | full100`.

Transitions on each `ok` reading, per window:

```text
level(p) = full100 if p ≥ 100, warn80 if p ≥ 80, else none
if level(p) > sent[key]:  sent[key] = level(p); emit warning (if warnings on)
otherwise:                no change (no re-arm on drops within the period)
new resetsAt → new key, starts at none; keys with resetsAt in the past are pruned
```

When warnings are off, `sent` is still updated, so turning warnings on later does not fire for
crossings that already happened (avoids a burst). Windows without `resetsAt` use key
`resetsAt = none` and warn at most once per level until a reading with a `resetsAt` arrives.

## Schedule (R1, FR-006)

Per provider: `lastAttempt: Instant?`.

- Automatic: due when `now - lastAttempt ≥ interval`; interval 5 min for `claude`, 1 min for
  `codex` (app server makes no model request) and custom.
- Panel open: due when the cached reading is older than 1 min.
- Manual refresh / Refresh all: always runs, but never two runs of one provider at once
  (an in-flight run is reused).
- Reset passed while open: when a window's `resetsAt` passes, that provider becomes due
  (spec edge case).
- Floor: no automatic or panel-open run within 1 min of the previous attempt.

## UsagePreferences (in `preferences.json`, key `usage`)

| Field | Type | Default | Written |
|---|---|---|---|
| `enabled` | bool | `true` | only when false |
| `warnings` | bool | `true` | only when false |
| `providers` | map ProviderId → `{enabled?, color?, consent?}` | `{}` | omit empty |

`enabled` on by default; no reading happens before the provider's consent (FR-016).

## Usage cache file (`~/.sonata/usage-cache.json`)

```json
{
  "type": "usageCache",
  "schemaVersion": 1,
  "payload": {
    "readings": { "claude": { "status": "ok", "plan": "Team", "account": "Athom8",
                              "windows": [ … ], "readAt": "…" } },
    "warnings": [ { "providerId": "claude", "windowKind": "short",
                    "resetsAt": "…", "sent": "warn80" } ]
  }
}
```

Only `ok`, `noPlanLimits`, and `notLoggedIn` readings are cached. Load failure → reported as a
persistence warning, start with an empty cache, do not overwrite the bad file until the next
successful save renames it aside as `usage-cache.json.bad` (constitution IV).

## Custom definition file

See [contracts/usage-provider.md](contracts/usage-provider.md).
