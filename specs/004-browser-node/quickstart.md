# Quickstart: Validate Feature 004

1. Press B, click the canvas, type `localhost:3000` (run `npx serve` in a terminal first) → page
   loads; back/forward/reload/inspect/open externally work.
2. Pan and zoom: pages switch to cards during gestures and at small zoom; open the New Terminal
   dialog over a browser → the page never covers the dialog.
3. Six browsers on screen → at most four live.
4. Log in to a site in one browser; a second browser in the same workspace is logged in; a browser
   in another workspace is not.
5. Link a shell to the browser: `sonata browser open App localhost:3000`, `read`, `fill 1 "a@b.c"`,
   `click 3`, `screenshot` → each works; from an unlinked shell → exit 3.
6. Orchestrator: `sonata browser eval App "document.title"` → JSON; non-orchestrator → exit 3.
7. Restart the app → browser reopens the last URL.
