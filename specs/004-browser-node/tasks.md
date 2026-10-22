---
description: "Task list for feature 004: browser node"
---

# Tasks: Browser Node

Tests first (constitution III). Native webview behavior is verified manually (quickstart).

## Phase 1: Setup

- [ ] T001 Enable `tauri` features `unstable` and `devtools`; add `objc2`, `objc2-foundation`, `objc2-app-kit`, `objc2-web-kit` to `src-tauri/Cargo.toml`
- [ ] T002 [P] Add error code `InvalidAddress` in `crates/core/src/error.rs` and `ui/src/ipc/types.ts`

## Phase 2: Foundational

- [ ] T003 [P] Tests `crates/core/tests/browser_model.rs`: URL normalization table (R10); `NodeContent::Browser` round trip; `create_browser` default/min size; links and deletion like other nodes
- [ ] T004 Implement `crates/core/src/browser.rs` (normalize, `BrowserHost` trait, result types) and `App::create_browser`, `App::set_browser_host`, browser URL/title updates
- [ ] T005 Shell: `src-tauri/src/browser/host.rs` creates/positions/destroys child webviews per node (data store per workspace, scheme filter, new-window and download handlers, page-load state, devtools), implements `BrowserHost`

## Phase 3: US1 Browse (P1)

- [ ] T006 [P] [US1] UI tests `ui/tests/browserNode.test.tsx`: address normalization feedback, chrome buttons call `browser_action`, tool B places a node
- [ ] T007 [US1] Commands `node_create_browser`, `browser_navigate`, `browser_action`; event `browser://state`
- [ ] T008 [US1] `ui/src/nodes/BrowserNode.tsx` (chrome, card, loading, error card), Browser tool (B) in `ToolBar.tsx`, floating toolbar actions

## Phase 4: US2 Out of the way (P1)

- [ ] T009 [P] [US2] Tests `ui/tests/browserLayout.test.ts`: live only when legible, not covered, no gesture, ≤ 4 (selected first, then area)
- [ ] T010 [US2] `ui/src/canvas/browserLayout.ts` + `browser_layout` command; shell moves views off screen for cards and captures card images (`snapshot.rs`)

## Phase 5: US3 Agents (P2)

- [ ] T011 [P] [US3] Tests `crates/core/tests/cli_browser.rs` with a fake host: every subcommand, permissions (unlinked → 3, eval non-orchestrator → 3), name resolution, timeouts, list shows browsers
- [ ] T012 [P] [US3] DOM script tests `ui/tests/browserScripts.test.ts` (jsdom): read numbering and labels, click by number/selector, fill events, press
- [ ] T013 [US3] `browser` subcommands in `crates/core/src/messaging/commands.rs`; scripts in `src-tauri/src/browser/scripts/`; host methods read/click/fill/press/eval/screenshot/back/forward/reload with load waiting
- [ ] T014 [US3] "controlled by" marker and link glow during browser commands (events reuse `messaging://ask` shape)

## Phase 6: Polish

- [ ] T015 Delete workspace browsing data on workspace delete (objc2)
- [ ] T016 README, `scripts/check.sh`, e2e, install, quickstart results
