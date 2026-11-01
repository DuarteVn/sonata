# Contract: Report Service HTTP API (013)

The app (core `report::send`) is the only intended client. The Worker treats every request
as untrusted.

## `POST /report`

**Request**: `Content-Type: multipart/form-data`, total body ≤ 7 MB (7 × 1024 × 1024 bytes;
checked with `Content-Length` first, then on the parsed form). `User-Agent: Sonata/<version>`.

| Part | Kind | Required | Rules |
|------|------|----------|-------|
| `name` | text | yes | 1–100 characters after trim |
| `description` | text | yes | 1–10,000 characters after trim |
| `appVersion` | text | yes | `^\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?$` |
| `os` | text | yes | `macos`, `linux`, or `windows` |
| `osVersion` | text | yes | 1–200 characters |
| `arch` | text | yes | 1–20 characters, `[a-z0-9_]` |
| `sentAt` | text | yes | RFC 3339 |
| `logsRequested` | text | yes | `true` or `false` |
| `image` | file | no | ≤ 5 MB; first bytes `89 50 4E 47 0D 0A 1A 0A` (PNG) or `FF D8 FF` (JPEG); the declared type is ignored |
| `log` | file | no | ≤ 1 MB; only allowed when `logsRequested` is `true`; sent as `text/plain` |

Unknown parts are ignored.

**Order of checks**: method and path → size (`Content-Length`) → parse → fields → files →
limiter reservation → Resend call. A request that fails before the limiter never uses a slot.

## Responses

All bodies are JSON `{"ok": bool, "error"?: string}`.

| Status | `error` | When | App state |
|--------|---------|------|-----------|
| 200 | — | email accepted by Resend | `sent` |
| 400 | `invalid_field:<name>` | missing or out-of-rule field or file | `failed(refused)` |
| 404 | `not_found` | other path | `failed(refused)` |
| 405 | `method_not_allowed` | not POST | `failed(refused)` |
| 413 | `too_large` | body over 7 MB | `failed(refused)` |
| 429 | `address_limit` \| `daily_limit` | limiter refused; `Retry-After` header in seconds | `failed(rate_limited)` |
| 502 | `email_failed` | Resend answered non-2xx or the call failed | `failed(unreachable)` |

The Worker never echoes request content, secrets, or Resend error bodies in its responses.
It logs only status, error code, and sizes.

## Email produced

- **To**: secret `REPORT_TO`. **From**: variable `REPORT_FROM` (default
  `Sonata Reports <onboarding@resend.dev>`).
- **Subject**: `[Sonata bug] <name>: <description, first 60 characters, one line>`.
- **Text body**:

  ```text
  From: <name>
  Sent: <sentAt>
  Sonata <appVersion> · <os> <osVersion> · <arch>
  Logs: included | requested but unavailable | not requested

  <description>
  ```

- **Attachments**: `screenshot.png` or `screenshot.jpg` when present; `sonata-log.txt` when
  present.

## Worker configuration

| Name | Kind | Purpose |
|------|------|---------|
| `RESEND_API_KEY` | secret | Resend API key |
| `REPORT_TO` | secret | destination address |
| `REPORT_FROM` | variable (`wrangler.toml`) | sender |
| `LIMITER` | Durable Object binding | class `Limiter` |
