import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [
    cloudflareTest({
      wrangler: { configPath: "./wrangler.toml" },
      miniflare: {
        // Test values only; the real ones are Worker secrets.
        bindings: { RESEND_API_KEY: "test-key", REPORT_TO: "dest@example.test" },
      },
    }),
  ],
  test: { include: ["test/**/*.test.ts"] },
});
