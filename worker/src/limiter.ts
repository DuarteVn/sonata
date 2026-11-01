// Rate limits (research R3): one Durable Object instance with SQLite storage counts reports
// per client address per UTC hour and in total per UTC day. It handles one call at a time and
// uses only synchronous storage, so counts stay exact under bursts (SC-005).

import { DurableObject } from "cloudflare:workers";

export const HOURLY_LIMIT = 5;
export const DAILY_LIMIT = 100;
const HOUR = 3_600_000;
const DAY = 86_400_000;
/** Counters older than this are deleted by the alarm. */
const KEEP = 2 * DAY;

export type Verdict = { ok: true } | { ok: false; error: "address_limit" | "daily_limit"; retryAfter: number };

const pad = (n: number) => String(n).padStart(2, "0");

function ymd(ms: number): string {
  const d = new Date(ms);
  return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}`;
}

export const dayKey = (ms: number) => `d:${ymd(ms)}`;
export const hourKey = (ipHash: string, ms: number) => `h:${ipHash}:${ymd(ms)}${pad(new Date(ms).getUTCHours())}`;

/** Start time (UTC ms) encoded in a counter key, or `undefined` for a foreign key. */
function keyTime(key: string): number | undefined {
  const m = /^(?:d:|h:[0-9a-f]+:)(\d{4})(\d{2})(\d{2})(\d{2})?$/.exec(key);
  if (!m) return undefined;
  return Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4] ?? 0));
}

const secondsUntil = (ms: number, now: number) => Math.max(1, Math.ceil((ms - now) / 1000));

export class Limiter extends DurableObject {
  /** Takes a slot for `ipHash` at `now` (UTC ms) when both limits allow it. */
  reserve(ipHash: string, now: number): Verdict {
    const kv = this.ctx.storage.kv;
    const day = dayKey(now);
    const hour = hourKey(ipHash, now);
    const dayCount = kv.get<number>(day) ?? 0;
    const hourCount = kv.get<number>(hour) ?? 0;
    if (dayCount >= DAILY_LIMIT) {
      return { ok: false, error: "daily_limit", retryAfter: secondsUntil((Math.floor(now / DAY) + 1) * DAY, now) };
    }
    if (hourCount >= HOURLY_LIMIT) {
      return { ok: false, error: "address_limit", retryAfter: secondsUntil((Math.floor(now / HOUR) + 1) * HOUR, now) };
    }
    kv.put(day, dayCount + 1);
    kv.put(hour, hourCount + 1);
    void this.scheduleCleanup(now);
    return { ok: true };
  }

  private async scheduleCleanup(now: number): Promise<void> {
    if ((await this.ctx.storage.getAlarm()) === null) await this.ctx.storage.setAlarm(now + DAY);
  }

  /** Daily: deletes counters older than two days. */
  async alarm(): Promise<void> {
    const kv = this.ctx.storage.kv;
    const cutoff = Date.now() - KEEP;
    for (const [key] of kv.list()) {
      const t = keyTime(key);
      if (t !== undefined && t < cutoff) kv.delete(key);
    }
    const remaining = [...kv.list({ limit: 1 })].length > 0;
    if (remaining) await this.ctx.storage.setAlarm(Date.now() + DAY);
  }
}

export function limiterStub(env: { LIMITER: DurableObjectNamespace<Limiter> }): DurableObjectStub<Limiter> {
  return env.LIMITER.get(env.LIMITER.idFromName("global"));
}

/** Hex SHA-256 of the client address, so raw addresses never reach storage. */
export async function hashAddress(address: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(address));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}
