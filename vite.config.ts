import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed port and does not want Vite to clear the Rust output.
export default defineConfig({
  root: "ui",
  plugins: [react()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: { outDir: "dist", emptyOutDir: true, target: "safari17" },
});
