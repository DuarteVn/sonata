# Research: Browser Node

**Feature**: 004-browser-node | **Date**: 2026-10-22 · APIs checked in the local `tauri` 2.12.5
and `objc2-web-kit` 0.3.2 sources.

## R1. Native child webviews

**Decision**: Each live-capable browser node owns one child webview added to the main window with
`Window::add_child` (Tauri feature `unstable`). The shell positions it at the node's page rect in
window coordinates. Pages are never `hide()`n: a "card" page is moved off screen (x = -20000) so it
keeps rendering, keeps its state, and can still be captured (FR-009, FR-017).

**Rationale**: Iframes fail for most sites (`X-Frame-Options`, CSP). A child webview is a real
browser with its own process and cookies.

## R2. Sessions per workspace

**Decision**: `WebviewBuilder::data_store_identifier(workspace_uuid_bytes)` (macOS 14+, Sonata
requires 15). Deleting a workspace removes its store with
`WKWebsiteDataStore::removeDataStoreForIdentifier:completionHandler:` (objc2-web-kit).

## R3. Isolation from Sonata

**Decision**: Browser webviews get no Tauri capability (capabilities list only `main`), so pages
cannot invoke commands (FR-011). `on_navigation` allows only `http`, `https`, `about:blank`;
`on_new_window` navigates the same webview and denies the new window; `on_download` opens the URL
in the system browser and cancels. Permission requests (camera, mic, location) use WebKit's
default deny.

## R4. Reading and acting on pages

**Decision**: `Webview::eval_with_callback` runs a small script and returns its JSON result. Scripts
live in `src-tauri/src/browser/scripts/` (our own code): `read.js` (title, URL, visible text up to
100 KB, interactive elements with `data-sonata-id` numbers), `click.js`, `fill.js` (native value
setter + `input`/`change` events), `press.js` (keydown/keypress/keyup on the active element).
Element numbers are stored on the elements as an attribute, so they survive until the next
`read` or navigation.

## R5. Load waiting

**Decision**: `on_page_load` (Started/Finished) updates a per-node load state in the shell; commands
wait on it with a deadline. `click` waits for a Started event within 300 ms and then Finished, or
for 1 s without new resource activity measured by `performance.getEntriesByType('resource')`
count, capped at 10 s.

## R6. Screenshots and card images

**Decision**: `with_webview` gives the `WKWebView`; `takeSnapshotWithConfiguration:completionHandler:`
returns an `NSImage`, converted to PNG through `NSBitmapImageRep`. Card images are written to
`<data root>/run/browser/<node>.png` (0600) and shown through the asset protocol (already scoped
to the data root). Agent screenshots go to `<data root>/run/shots/` unless `--out`.

## R7. Who decides live vs card

**Decision**: The frontend decides, as it does for terminals (it knows zoom, gestures, overlaps,
and selection) and sends `browser_layout([{nodeId, rect | null}])` after each settle (debounced
50 ms). `null` means "card": the shell captures a fresh image, then moves the view off screen.
Overlap test: DOM rects of open modals, menus, floating toolbars, banners, and the tool bar.

## R8. Core/shell split

**Decision**: `core` defines a `BrowserHost` trait (open, back, forward, reload, read, click, fill,
press, screenshot, eval, close, url/title) and keeps browser nodes in the model and permissions in
`messaging::commands`. The shell implements `BrowserHost` with Tauri webviews and registers it after
the window exists (`App::set_browser_host`). Core tests use a fake host.

## R9. Persistence

**Decision**: New node content `browser: {url, title}`; additive, so the workspace stays at schema
v2 (an older build fails to decode an unknown content key and shows the workspace as a load
failure without writing it — acceptable and safe).

## R10. URL normalization

**Decision**: Pure function in `core`: trim; refuse inner spaces; add `http://` for
`localhost`, `127.0.0.1`, `[::1]` (with optional port/path); add `https://` when no scheme; allow
only `http`, `https`, `about:blank`.
