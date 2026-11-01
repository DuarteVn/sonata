import { env, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DAILY_LIMIT, HOURLY_LIMIT, Limiter, dayKey, limiterStub } from "../src/limiter";
import { call, form, freshIp, mockResend, post } from "./helpers";

afterEach(() => vi.restoreAllMocks());

// Durable Object storage is shared by the tests of one file; start each test with no counters.
beforeEach(() =>
  runInDurableObject(limiterStub(env), (_: Limiter, state) => {
    for (const [key] of [...state.storage.kv.list()]) state.storage.kv.delete(key);
  }),
);

async function keys(): Promise<string[]> {
  return runInDurableObject(limiterStub(env), (_: Limiter, state) => [...state.storage.kv.list()].map(([k]) => k));
}

describe("rate limits", () => {
  it("accepts five reports an hour from one address, then answers 429", async () => {
    const resend = mockResend();
    const ip = freshIp();
    const statuses: number[] = [];
    for (let i = 0; i < HOURLY_LIMIT + 1; i++) statuses.push((await call(post(form(), { ip }))).status);
    expect(statuses).toEqual([200, 200, 200, 200, 200, 429]);

    const res = await call(post(form(), { ip }));
    expect(await res.json()).toEqual({ ok: false, error: "address_limit" });
    const retry = Number(res.headers.get("Retry-After"));
    expect(retry).toBeGreaterThan(0);
    expect(retry).toBeLessThanOrEqual(3600);
    expect(resend).toHaveBeenCalledTimes(HOURLY_LIMIT);

    expect((await call(post(form(), { ip: freshIp() }))).status).toBe(200);
  });

  it("refuses everyone after the daily total", async () => {
    mockResend();
    await runInDurableObject(limiterStub(env), (_: Limiter, state) => {
      state.storage.kv.put(dayKey(Date.now()), DAILY_LIMIT - 1);
    });
    expect((await call(post(form()))).status).toBe(200);
    const res = await call(post(form()));
    expect(res.status).toBe(429);
    expect(await res.json()).toEqual({ ok: false, error: "daily_limit" });
    const retry = Number(res.headers.get("Retry-After"));
    expect(retry).toBeGreaterThan(0);
    expect(retry).toBeLessThanOrEqual(86_400);
  });

  it("lets exactly five of 50 parallel requests from one address through", async () => {
    const resend = mockResend();
    const ip = freshIp();
    const results = await Promise.all(Array.from({ length: 50 }, () => call(post(form(), { ip }))));
    const ok = results.filter((r) => r.status === 200).length;
    expect(ok).toBe(HOURLY_LIMIT);
    expect(results.filter((r) => r.status === 429)).toHaveLength(50 - HOURLY_LIMIT);
    expect(resend).toHaveBeenCalledTimes(HOURLY_LIMIT);
  });

  it("never uses a slot for invalid requests", async () => {
    mockResend();
    const ip = freshIp();
    for (let i = 0; i < 10; i++) {
      expect((await call(post(form({ name: "" }), { ip }))).status).toBe(400);
      expect(
        (await call(post(form(), { ip, headers: { "Content-Length": String(8 * 1024 * 1024) } }))).status,
      ).toBe(413);
    }
    for (let i = 0; i < HOURLY_LIMIT; i++) expect((await call(post(form(), { ip }))).status).toBe(200);
  });

  it("stores no raw address", async () => {
    mockResend();
    const ip = "198.51.100.77";
    await call(post(form(), { ip }));
    const stored = await keys();
    expect(stored.length).toBeGreaterThan(0);
    expect(stored.some((k) => k.startsWith("h:"))).toBe(true);
    for (const k of stored) {
      expect(k).not.toContain(ip);
      expect(k).toMatch(/^(h:[0-9a-f]{64}:\d{10}|d:\d{8})$/);
    }
  });

  it("deletes counters older than two days in its alarm", async () => {
    const now = Date.now();
    const old = now - 3 * 86_400_000;
    await runInDurableObject(limiterStub(env), (instance: Limiter, state) => {
      state.storage.kv.put(dayKey(old), 3);
      state.storage.kv.put(`h:${"a".repeat(64)}:${dayKey(old).slice(2)}00`, 1);
      return instance.reserve("b".repeat(64), now);
    });
    expect(await runDurableObjectAlarm(limiterStub(env))).toBe(true);
    const stored = await keys();
    expect(stored).not.toContain(dayKey(old));
    expect(stored).toContain(dayKey(now));
    expect(stored.filter((k) => k.startsWith("h:a"))).toEqual([]);
  });
});
