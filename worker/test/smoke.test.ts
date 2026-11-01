import { createExecutionContext, env } from "cloudflare:test";
import { expect, it } from "vitest";
import worker from "../src/index";

it("answers 404 on an unknown path", async () => {
  const res = await worker.fetch(new Request("https://x/nope"), env, createExecutionContext());
  expect(res.status).toBe(404);
});
