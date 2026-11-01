# Contract: App Commands, Events, and UI (013)

## Menu event

- Menu: `Help` → `Report Bug…` (id `report-bug`) on macOS, Linux, and Windows.
- Event `report-bug` (no payload): the UI opens the dialog. The sidebar footer button opens it
  directly.

## Tauri commands (`src-tauri/src/report.rs`, thin wrappers over `core::report`)

### `report_config() -> ReportConfigView`

```json
{ "serviceConfigured": true, "fallbackEmail": "bugs@example.com" }
```

`fallbackEmail` is `null` when not set at build time. The service URL is not exposed.

### `report_read_image(path: string) -> ImageView`

Reads a dropped or chosen file. Returns `{ "bytes": number[], "mime": "image/png" | "image/jpeg" }`.
Errors (`{ "kind": "invalid", "field": "image", "reason": "too_large" | "unsupported_type" | "unreadable" }`).

### `report_send(input: ReportInput) -> ()`

```json
{
  "name": "Ana",
  "description": "The note vanished after I dragged it.",
  "image": { "bytes": [137, 80, 78, 71], "mime": "image/png" },
  "includeLogs": false
}
```

`image` may be `null`. Core re-validates everything, adds diagnostics and the log excerpt, and
posts to the service. Errors:

| `kind` | Meaning |
|--------|---------|
| `invalid` (`field`, `reason`) | input broke a rule |
| `not_configured` | no service URL in this build |
| `unreachable` | network error, timeout (30 s), or 5xx |
| `rate_limited` (`retryAfterSecs`) | 429 |
| `refused` (`code`) | other 4xx |

### `report_fallback(input: ReportInput) -> FallbackResult`

Builds the plain-text report (same body as the email, see report-service.md) and, when a
fallback alias exists, opens `mailto:`. Returns
`{ "opened": bool, "text": string, "truncated": bool }`. When `opened` is `false` or
`truncated` is `true`, the UI copies `text` to the clipboard and says so.

## Dialog UI contract (`ui/src/report/ReportBugDialog.tsx`)

| Element | Accessible name | Notes |
|---------|-----------------|-------|
| Dialog | `Report Bug` | `Modal`; Escape closes and keeps the draft |
| Headline | — | text exactly `Nosso sistema não tem bugs, tem PONTOS DE MELHORIA` |
| Name input | `Your name` | required, max 100 |
| Description textarea | `What happened?` | required, max 10,000; counter from 9,000 |
| Attach button | `Attach image…` | PNG/JPEG file chooser |
| Preview | `Attached image` | with `Remove image` button |
| Logs checkbox | `Include logs` | unticked; hint `Logs can contain folder paths and project names.` |
| Send button | `Send` | disabled while sending or required fields empty |
| Status | live region | `Sending…`, `Thanks! Your report was sent.`, failure messages |
| Failure actions | `Try again`, `Send by email instead` | shown in `failed` states |

Failure messages:

- `unreachable`: `Could not reach the report service. Check your connection and try again.`
- `rate_limited`: `Too many reports right now. Please try again later.`
- `refused`: `The report service did not accept this report.`
- `not_configured`: `This build cannot send reports directly.` (Send leads straight to the
  fallback actions)
- Fallback opened: `Your mail app opened. Attach the image by hand if you have one.`
- Fallback copied: `Report copied to the clipboard. Paste it in a message to the maintainer.`
