# Implementation Plan: Agent Usage Meter

**Branch**: `006-agent-usage` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-agent-usage/spec.md`

## Summary

Sonata shows how much of each agent account's plan limits is used (short window and weekly, with
reset times) in a Usage button with a compact meter and a panel, warns once per window period at
80% and 100% with a macOS notification, and adds a Settings window (Cmd+,) with a Usage section.

Data comes from the agents' own CLIs, never from credential stores read by Sonata (research R1,
R2): for Claude Code, `claude auth status --json` plus a minimal print-mode probe whose
`stream-json` output carries `rate_limit_event`; for Codex, `codex app-server`
`account/rateLimits/read`, with the newest session file as fallback. Custom agents come from
definition files whose command prints Sonata's normalized usage JSON and runs sandboxed with a
10 s limit (R4, R5).

Core owns the rules: provider registry, output parsers, schedule, window periods and warning
levels, stale rule, consent, cache without credentials, definition validation, and the command
runner. `src-tauri` starts the background loop, creates the Settings window and menu item, and
forwards core events. The frontend renders the meter, panel, and Settings section, and sends
notifications.

## Technical Context

**Language/Version**: Rust 2024 edition (stable), TypeScript 5 / React 19

**Primary Dependencies**: existing only: Tauri 2.12, `tauri-plugin-notification`, `tokio`,
`serde_json`, `chrono`, `libc`. No new crate.

**Storage**: `~/.sonata/preferences.json` gains `usage` (serde defaults, no schema bump);
new `~/.sonata/usage-cache.json` (envelope `usageCache` v1); new folder
`~/.sonata/usage-providers/` (definitions + `README.md`)

**Testing**: `cargo test` in core: parsers with recorded fixtures (Claude stream-json and auth
status, Codex app-server and rollout lines, custom output), window/warning state machine, stale
rule, schedule, definition validation, consent matching, cache round trip and "no credential"
check, runner tests with fake scripts in a temp dir (timeout kills the process group, output cap,
env allowlist, sandbox denies a test path). Vitest: meter states, panel states, Settings rows.
No test runs the real `claude` or `codex` (fake executables on a temp `PATH`).

**Target Platform**: macOS 15+, Apple Silicon and Intel

**Project Type**: desktop app (Tauri) with Rust core

**Performance Goals**: panel shows cached values < 2 s after open (SC-001); probes run off the
UI thread and off the terminal I/O tasks (SC-004); refresh result within 5 s for Claude
(measured probe 2.3 s)

**Constraints**: no credential in files, logs, argv, or events (SC-005, FR-013); at most one
automatic reading per minute per agent, Claude every 5 minutes (R1); custom command limit 10 s
(FR-019); no new network listener

**Scale/Scope**: 2 built-in providers (Claude Code, Codex), a few custom providers, 1–3 windows
each

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|---|---|---|
| I. Clean-Room Originality | Pass | Behavior from the spec only; own layout, wording, and icons. Reference screenshots are not copied. No reference material in the repo. |
| II. Core/Shell Separation | Pass | All rules and the command runner in `crates/core/src/usage/`; `src-tauri` only wires the loop, window, menu, and commands. |
| III. Test-First | Pass | Every core module starts with failing tests on fixtures and fake executables; UI components with vitest. |
| IV. Files Are the Source of Truth | Pass | Cache and definitions use the envelope; atomic writes; invalid definition files are reported, never overwritten; guide file written only when missing. |
| V. Authorization & Local Security | Pass | Sonata reads no credential store. Probes get a scrubbed env and argv without secrets. Custom commands need consent bound to the exact approved argv and run sandboxed away from credential stores (R5). Settings window gets only the commands it needs. |
| VI. Free, Open Distribution | Pass | No paid service; no change to signing or release. |
| VII. Simplicity | Pass with justification | No new crate; one new window and one cache file, justified below. No field-mapping language for custom providers. |

Post-design re-check (after data-model and contracts): unchanged, all pass.

## Project Structure

### Documentation (this feature)

```text
specs/006-agent-usage/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── ipc.md               # commands, events, types for main and settings windows
│   └── usage-provider.md    # definition file, normalized output, guide, runner rules
└── tasks.md                 # /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/
├── usage/
│   ├── mod.rs               # public types: ProviderId, Reading, Window, ReadingStatus
│   ├── window.rs            # window periods, warning levels, stale rule, meter summary
│   ├── schedule.rs          # due checks per provider (5 min Claude, 1 min floor, on-open)
│   ├── claude.rs            # probe argv, auth-status and stream-json parsers
│   ├── codex.rs             # app-server request/response, rollout tail parser
│   ├── custom.rs            # definition validation, normalized output parser, guide text
│   ├── runner.rs            # process runner: argv, env allowlist, pgroup kill, caps, sandbox
│   ├── consent.rs           # consent state, approved-argv matching
│   ├── cache.rs             # usage-cache.json load/save (envelope, atomic)
│   ├── detect.rs            # login-shell command paths + PATH (extends presets approach)
│   └── service.rs           # UsageService: registry, refresh, events; no Tauri
├── model/preferences.rs     # + usage settings
├── paths.rs                 # + usage_cache_file(), usage_providers_dir()
├── events.rs                # + UsageState, UsageWarning events
└── app.rs                   # + owns UsageService, exposes usage_* methods
src-tauri/
├── build.rs                 # + usage_* commands
├── capabilities/default.json   # + allow-usage-* for main
├── capabilities/settings.json  # new: settings window permissions
└── src/
    ├── lib.rs               # + usage loop task, settings window creation
    ├── menu.rs              # + "Settings…" (Cmd+,)
    └── commands/usage.rs    # new: thin commands over core
ui/src/
├── usage/
│   ├── UsageButton.tsx      # compact meter, opens panel
│   ├── UsagePanel.tsx       # per-agent sections, states, refresh
│   ├── meter.ts             # pure helpers: level, colors, reset text
│   └── *.test.tsx
├── settings/
│   ├── SettingsApp.tsx      # settings window root (?view=settings)
│   └── UsageSettings.tsx    # master switch, warnings, rows, Refresh all, folder actions
├── dialogs/UsageConsentDialog.tsx  # consent with exact command
├── state/usage.ts           # zustand slice fed by usage events
├── ipc/commands.ts, events.ts, types.ts
├── main.tsx                 # route ?view=settings
└── App.tsx                  # mounts UsageButton, warning notifications
README.md                    # Usage section: sources, consent, custom providers
```

**Structure Decision**: Existing Cargo workspace and `ui/`. One new core module folder
(`usage/`), one new Tauri command file, two new UI folders.

**UX decisions taken (not in the spec, reversible)**:

- The Usage button sits at the top-right corner of the canvas area, so it stays visible when
  the sidebar is hidden; the panel opens as a popover under it.
- The compact meter shows the highest percentage as a ring around a gauge icon, tinted with
  that agent's color; amber at the warning level, red with a full ring at 100%; a slashed icon
  when usage reading is off.
- Consent appears the first time a provider would be read (app start with the master switch on,
  or turning an agent on), one dialog per agent, showing what runs and why.

## Complexity Tracking

| Addition | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Settings window (second webview) | FR-015 asks for a Settings window; later features add sections | Modal in main window contradicts the spec |
| `sandbox-exec` profile for custom commands | FR-019: a definition must not make Sonata read credential stores | Consent alone does not enforce it; full sandboxing of built-in probes impossible because the CLI must read its own login |
| Usage cache file | FR-014, SC-001 (values after restart) and warnings not repeated after restart (R7) | In-memory only: loses warning levels on restart and shows an empty panel at start |
