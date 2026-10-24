// Pure helpers for the usage meter and panel (feature 006).

export type MeterLevel = "normal" | "warning" | "full";

/** Share at which the meter and the notifications warn (FR-010). */
export const WARNING_PERCENT = 80;

export function meterLevel(percent: number): MeterLevel {
  if (percent >= 100) return "full";
  if (percent >= WARNING_PERCENT) return "warning";
  return "normal";
}

export function percentText(percent: number): string {
  if (percent > 0 && percent < 1) return "<1%";
  return `${Math.round(Math.min(percent, 100))}%`;
}

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

function span(ms: number): string {
  if (ms >= DAY) {
    const d = Math.floor(ms / DAY);
    const h = Math.floor((ms % DAY) / HOUR);
    return h > 0 ? `${d} d ${h} h` : `${d} d`;
  }
  if (ms >= HOUR) {
    const h = Math.floor(ms / HOUR);
    const m = Math.floor((ms % HOUR) / MINUTE);
    return m > 0 ? `${h} h ${m} min` : `${h} h`;
  }
  return `${Math.max(1, Math.floor(ms / MINUTE))} min`;
}

export function resetText(resetsAt: string | null | undefined, now: Date): string {
  if (!resetsAt) return "";
  const ms = new Date(resetsAt).getTime() - now.getTime();
  return ms <= 0 ? "resetting now" : `resets in ${span(ms)}`;
}

export function timeAgo(at: string, now: Date): string {
  const ms = now.getTime() - new Date(at).getTime();
  if (ms < MINUTE) return "just now";
  if (ms >= DAY) return `${Math.floor(ms / DAY)} d ago`;
  if (ms >= HOUR) return `${Math.floor(ms / HOUR)} h ago`;
  return `${Math.floor(ms / MINUTE)} min ago`;
}

export function clockTime(at: string): string {
  return new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Shows a command the way a shell would read it, so the user sees exactly what runs. */
export function shellQuote(argv: string[]): string {
  return argv.map((a) => (/^[\w@%+=:,./-]+$/.test(a) ? a : `'${a.replace(/'/g, `'\\''`)}'`)).join(" ");
}
