# Implementation Plan: Browser Node on the Canvas, Controllable by Linked Agents

**Branch**: `004-browser-node` | **Date**: 2026-10-22 | **Spec**: [spec.md](spec.md)

## Summary

Browser nodes are native child webviews positioned over the canvas by the shell, live only when
the frontend says so (legible, uncovered, within the limit of 4), and otherwise shown as cards with
the last page image. Sessions are per workspace. Linked terminals control them with `sonata
browser …`; `core` checks permissions and delegates page work to a `BrowserHost` implemented by
the shell. See [research.md](research.md).

## Technical Context

**Language/Version**: Rust 1.99, TypeScript 7. **Dependencies**: `tauri` features `unstable` and
`devtools`; `objc2`, `objc2-web-kit`, `objc2-app-kit`, `objc2-foundation` in `src-tauri` (macOS
only) for snapshots and data-store removal. **Storage**: new node content `browser`; runtime images
in `<data root>/run/`. **Testing**: core with a fake `BrowserHost` (permissions, CLI parsing, URL
normalization, persistence); shell scripts unit-tested with Vitest+jsdom (read/click/fill on test
DOM); manual real-app checks for native views (quickstart). **Target**: macOS 15+.
**Performance**: ≤ 4 live webviews; layout updates debounced 50 ms. **Constraints**: pages cannot
reach Sonata (no capability, scheme filter).

## Constitution Check

| Principle | Result |
|---|---|
| I Clean room | Pass: own scripts and wording. |
| II Core/shell | Pass: permissions and model in core; webview work behind `BrowserHost` in the shell. |
| III Test-first | Pass: fake host for core; DOM scripts tested in jsdom; native parts manual (no macOS WebDriver). |
| IV Files | Pass: additive node content. |
| V Security | Pass: no capability for pages, scheme filter, eval only for Orchestrator, private screenshots. |
| VI / VII | Pass: no new services; `unstable` Tauri feature justified below. |

## Project Structure

```text
crates/core/src/browser.rs            # BrowserHost trait, URL normalization, element-map types
crates/core/src/messaging/commands.rs # `browser` subcommands, permissions
crates/core/src/model/node.rs         # NodeContent::Browser
src-tauri/src/browser/                # host.rs (webviews), snapshot.rs (objc2), scripts/*.js
src-tauri/src/commands/browser.rs     # node_create_browser, browser_navigate/action/layout
ui/src/nodes/BrowserNode.tsx          # chrome + card
ui/src/canvas/browserLayout.ts        # live set + overlap → browser_layout
```

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Tauri `unstable` feature (child webviews) | Only way to embed real pages in the window. | Iframes break on most sites; separate windows cannot sit on the canvas. |
| objc2 calls in the shell | Snapshots of hidden pages and per-workspace data removal have no Tauri API. | No alternative for FR-017 and FR-010 deletion. |
