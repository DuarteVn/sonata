# Contract: App Commands, Events, and UI (013)

## Menu event

- Menu: `Help` → `Report Bug…` (id `report-bug`) on macOS, Linux, and Windows.
- Event `report-bug` (no payload): the UI opens the dialog. The sidebar footer button opens it
  directly.

## Tauri commands (`src-tauri/src/report.rs`, thin wrappers over `core::report`)

### `report_config() -> ReportConfigView`

```json
{ "serviceConfigured": false, "reportEmail": "vini.d.science@gmail.com", "mailBodyRoom": 1412 }
```

`reportEmail` is `SONATA_REPORT_FALLBACK_EMAIL` when set at build time, otherwise the public
`REPORT_EMAIL`. The service URL is not exposed. `mailBodyRoom` is how many characters the name
and the description can share before a `mailto:` body is cut; the UI needs this before the
click ends, to choose what goes to the clipboard.

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
| `not_configured` | no service URL in this build; the UI does not call `report_send` then |
| `unreachable` | network error, timeout (30 s), or 5xx |
| `rate_limited` (`retryAfterSecs`) | 429 |
| `refused` (`code`) | other 4xx |

### `report_fallback(input: ReportInput) -> FallbackResult`

Builds the plain-text report (same body as the email, see report-service.md) and opens
`mailto:` to `reportEmail`. Returns `{ "opened": bool, "text": string, "truncated": bool }`.
`opened` is `false` when the opener cannot start or exits with an error within 2 seconds. When
`opened` is `false` or `truncated` is `true`, the UI copies `text` to the clipboard and says so.

## Dialog UI contract (`ui/src/report/ReportBugDialog.tsx`)

| Element | Accessible name | Notes |
|---------|-----------------|-------|
| Dialog | `Report Bug` | `Modal`; Escape closes and keeps the draft |
| Headline | — | text exactly `Nosso sistema não tem bugs, tem PONTOS DE MELHORIA` |
| Name input | `Your name` | required, max 100 |
| Description textarea | `What happened?` | required, max 10,000; counter from 9,000 |
| Attach button | `Attach image…` | PNG/JPEG file chooser |
| Preview | `Attached image` | with `Remove image` button |
| Logs checkbox | `Include logs` | only with a service; unticked; hint `Logs can contain folder paths and project names.` |
| Primary button | `Open email`, or `Send` with a service | disabled while working or required fields empty |
| Status | live region | `Opening your mail app…`, `Sending…`, `Thanks! Your report was sent.`, the messages below |
| Failure actions | `Try again`, `Send by email instead` | only with a service, in `failed` states |

Messages after `Open email` (no service, the default). `<paste>` is `⌘V` on macOS and `Ctrl+V`
elsewhere:

- No image: `Your mail app opened with the report.`
- Image copied as PNG: `Your mail app opened with the report. The image is in the clipboard: paste it into the email (<paste>).`
- Image copy failed: `Your mail app opened with the report. Attach the image by hand.`
- Body cut, full text copied instead of the image: `The report was long, so the full text is in the clipboard. Paste it into the email and attach the image by hand.` (ends at `email.` when there is no image)
- Body cut, copy failed: `Your mail app opened with the report. It was cut because it is long. Could not copy the report. Select the text above and copy it by hand.`
- Mail app did not open: `Report copied to the clipboard. Paste it in a message to the maintainer. The address is <reportEmail>.`
- Mail app did not open, copy failed: `Could not copy the report. Select the text above and copy it by hand. The address is <reportEmail>.`

The clipboard write must start inside the click, before any `await`. Pressing `Open email` again
within 1 second does nothing, so a double click opens one email. The draft stays after opening.

Failure messages with a service:

- `unreachable`: `Could not reach the report service. Check your connection and try again.`
- `rate_limited`: `Too many reports right now. Please try again later.`
- `refused`: `The report service did not accept this report.`
- `not_configured`: `This build cannot send reports directly.` (not reachable from the UI: a
  build without a service uses `Open email`)
- Fallback opened: `Your mail app opened. Attach the image by hand if you have one.`
- Fallback copied: `Report copied to the clipboard. Paste it in a message to the maintainer.`
