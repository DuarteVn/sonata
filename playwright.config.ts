import { defineConfig, devices } from "@playwright/test";

// Runs the Vite frontend with a mocked Tauri IPC (see e2e/fixtures/mockIpc.ts).
// The full Tauri app cannot be driven by WebDriver on macOS; see quickstart.md.
export default defineConfig({
  testDir: "e2e",
  use: { baseURL: "http://localhost:1420" },
  projects: [{ name: "webkit", use: { ...devices["Desktop Safari"] } }],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: true,
  },
});
