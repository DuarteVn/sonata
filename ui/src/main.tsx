import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@xterm/xterm/css/xterm.css";
import "./styles.css";
import { App } from "./App";

async function start() {
  // Playwright runs the frontend without Tauri; `?mock` installs an in-memory core.
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("mock")) {
    const { installMockIpc } = await import("../../e2e/fixtures/mockIpc");
    installMockIpc();
  }
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

void start();
