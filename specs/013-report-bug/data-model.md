# Data Model: Report Bug (013)

Nothing is persisted by the app. These types exist in memory (UI draft, core request) and on
the wire (see [contracts/report-service.md](contracts/report-service.md)).

## BugReport (core, `report::BugReport`)

| Field | Type | Rules |
|-------|------|-------|
| `name` | string | required; trimmed; 1–100 characters (Unicode scalar values) |
| `description` | string | required; trimmed; 1–10,000 characters |
| `image` | `ReportImage?` | at most one |
| `include_logs` | bool | default `false` |

Validation errors (`ReportError::Invalid { field, reason }`): `name` empty or too long,
`description` empty or too long, image too large or wrong type.

## ReportImage

| Field | Type | Rules |
|-------|------|-------|
| `bytes` | bytes | 1 B – 5 MB (5 × 1024 × 1024) |
| `mime` | `image/png` \| `image/jpeg` | decided from the magic number, never from a file name |
| `file_name` | string | `screenshot.png` or `screenshot.jpg` on the wire |

## Diagnostics (`report::diagnostics::Diagnostics`)

| Field | Example |
|-------|---------|
| `app_version` | `0.3.0` |
| `os` | `macos` \| `linux` \| `windows` |
| `os_version` | `15.6.1`; `Ubuntu 24.04.1 LTS (6.8.0-45-generic)`; `10.0.26100`; or `unknown` |
| `arch` | `aarch64` \| `x86_64` |
| `sent_at` | RFC 3339 UTC time, set when sending |

## LogExcerpt

| Field | Rules |
|-------|-------|
| `text` | ≤ 1 MB, starts at a line start, lossy UTF-8 |
| `source` | file name, for example `sonata.log.2026-11-01` |
| state | `included` \| `requested but unavailable` \| `not requested` |

## ReportConfig (build time, `src-tauri/src/report.rs`)

| Field | Source | When absent |
|-------|--------|-------------|
| `service_url` | `SONATA_REPORT_URL` | Send goes straight to the fallback with a note |
| `fallback_email` | `SONATA_REPORT_FALLBACK_EMAIL` | fallback offers copy only |

## Draft (UI, `ui/src/report/draft.ts`)

`{ name, description, image?: { bytes, mime, previewUrl }, includeLogs }`. Lives in a module
store for the app session (FR-019); cleared after a successful send; never written to disk.

## Dialog state machine

```text
editing ──Send (valid)──▶ sending ──200──▶ sent (form cleared; Close)
   ▲                         │
   │                         ├─429──▶ failed(rate_limited)
   │                         ├─4xx──▶ failed(refused)
   │                         └─network/timeout/no service──▶ failed(unreachable | not_configured)
   └──────── Try again / edit ◀── failed ──Send by email instead──▶ fallback(opened | copied)
```

Send is disabled in `sending` and while `name` or `description` is empty.

## Worker limiter (Durable Object storage)

| Key | Value | Lifetime |
|-----|-------|----------|
| `h:<sha256(ip) hex>:<yyyymmddhh UTC>` | count | deleted by the daily alarm after 48 h |
| `d:<yyyymmdd UTC>` | count | deleted by the daily alarm after 48 h |

`reserve(ipHash, now)` → `ok` \| `address_limit` \| `daily_limit`; increments both counters only
on `ok`. Limits: 5 per address per hour, 100 per day.
