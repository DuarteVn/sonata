import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    include: ["ui/tests/**/*.test.{ts,tsx}"],
    setupFiles: ["ui/tests/setup.ts"],
  },
});
