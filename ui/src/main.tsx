import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@xterm/xterm/css/xterm.css";
import "./styles.css";
import { App } from "./App";
import { SettingsApp } from "./settings/SettingsApp";

async function start() {
  // Playwright runs the frontend without Tauri; `?mock` installs an in-memory core.
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("mock")) {
    const { installMockIpc } = await import("../../e2e/fixtures/mockIpc");
    installMockIpc();
  }
  // The Settings window sets this before load (src-tauri/src/settings.rs).
  const view = (window as { __SONATA_VIEW__?: string }).__SONATA_VIEW__ ?? new URLSearchParams(location.search).get("view");
  const settings = view === "settings";
  createRoot(document.getElementById("root")!).render(
    <StrictMode>{settings ? <SettingsApp /> : <App />}</StrictMode>,
  );
}

void start();
