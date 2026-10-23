import { RefreshCw, Settings } from "lucide-react";
import { useEffect, useState } from "react";
import { ipc } from "../ipc/commands";
import type { ProviderState, Reading, UsageWindow } from "../ipc/types";
import { useUsage } from "../state/usage";
import { clockTime, meterLevel, percentText, resetText, timeAgo } from "./meter";

type Props = {
  /** Fixed clock for tests; the panel ticks every 30 s otherwise. */
  now?: Date;
};

const STATUS_TEXT: Record<Reading["status"], string> = {
  ok: "",
  noPlanLimits: "No plan limits",
  notLoggedIn: "Not logged in",
  notInstalled: "Command not found",
  unavailable: "Usage not available",
  off: "Off",
};

function useClock(fixed?: Date): Date {
  const [now, setNow] = useState(() => fixed ?? new Date());
  useEffect(() => {
    if (fixed) return;
    const id = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(id);
  }, [fixed]);
  return fixed ?? now;
}

function report(err: unknown) {
  console.warn("usage:", err);
}

// Popover under the Usage button: one section per agent account (FR-002 to FR-004).
export function UsagePanel({ now: fixedNow }: Props) {
  const state = useUsage((s) => s.state);
  const now = useClock(fixedNow);

  useEffect(() => {
    ipc
      .usageRefresh("panelOpen")
      .then((s) => s && useUsage.getState().set(s))
      .catch(report);
  }, []);

  if (!state) return null;
  const refresh = () => {
    ipc
      .usageRefresh("manual")
      .then((s) => s && useUsage.getState().set(s))
      .catch(report);
  };
  const visible = state.providers.filter(
    (p) => p.enabled && p.detection !== "notFound" && p.definitionError === null,
  );
  const busy = state.providers.some((p) => p.refreshing);
  const latest = visible
    .map((p) => p.reading?.readAt)
    .filter((t): t is string => !!t)
    .sort()
    .at(-1);

  return (
    <div className="usage-panel" role="dialog" aria-label="Agent usage">
      <header className="usage-panel-header">
        <span>Plan usage</span>
        <button
          type="button"
          className={`icon-button${busy ? " spinning" : ""}`}
          aria-label="Refresh usage"
          title="Refresh"
          disabled={!state.enabled}
          onClick={refresh}
        >
          <RefreshCw size={14} />
        </button>
      </header>
      {!state.enabled ? (
        <div className="usage-empty">
          <p>Usage reading is off.</p>
          <button type="button" onClick={() => void ipc.usageSetEnabled(true).then((s) => s && useUsage.getState().set(s))}>
            Turn on
          </button>
        </div>
      ) : visible.length === 0 ? (
        <div className="usage-empty">
          <p>No agent with plan limits was found on this Mac.</p>
          <p className="usage-dim">Sonata can read: {state.providers.map((p) => p.name).join(", ")}.</p>
        </div>
      ) : (
        visible.map((p) => <ProviderSection key={p.id} provider={p} now={now} />)
      )}
      <footer className="usage-panel-footer">
        <span>{latest ? `Read ${timeAgo(latest, now)}` : ""}</span>
        <button type="button" className="link-button" onClick={() => void ipc.usageOpenSettings().catch(report)}>
          <Settings size={12} /> Usage settings…
        </button>
      </footer>
    </div>
  );
}

function ProviderSection({ provider: p, now }: { provider: ProviderState; now: Date }) {
  const r = p.reading;
  const account = [r?.account, r?.accountEmail].filter(Boolean).join(" · ");
  return (
    <section className="usage-provider" aria-label={p.name} aria-busy={p.refreshing}>
      <div className="usage-provider-head">
        <span className="usage-swatch" style={{ background: p.color }} aria-hidden="true" />
        <span className="usage-name">{p.name}</span>
        {r?.plan && <span className="usage-plan">{r.plan}</span>}
      </div>
      {account && <div className="usage-account">{account}</div>}
      <ProviderBody provider={p} now={now} />
    </section>
  );
}

function ProviderBody({ provider: p, now }: { provider: ProviderState; now: Date }) {
  const r = p.reading;
  if (p.consent !== "granted") {
    return (
      <div className="usage-status">
        <span>Sonata needs your permission to read this agent.</span>
        <button
          type="button"
          className="link-button"
          onClick={() => void ipc.usageSetProviderEnabled(p.id, true).then((s) => s && useUsage.getState().set(s))}
        >
          Review permission
        </button>
      </div>
    );
  }
  if (!r) {
    if (p.lastError)
      return (
        <div className="usage-status">
          <strong>Usage not available</strong>
          <span className="usage-dim">{p.lastError}</span>
        </div>
      );
    return <div className="usage-status usage-dim">{p.refreshing ? "Reading…" : "No reading yet."}</div>;
  }
  if (r.status !== "ok") {
    return (
      <div className="usage-status">
        <strong>{STATUS_TEXT[r.status]}</strong>
        {r.message && <span className="usage-dim">{r.message}</span>}
      </div>
    );
  }
  return (
    <>
      {(r.windows ?? []).map((w) => (
        <WindowRow key={`${w.kind}-${w.label}`} provider={p} window={w} now={now} />
      ))}
      {p.stale && <div className="usage-stale">Stale · last read {clockTime(r.readAt)}</div>}
      {p.lastError && !p.stale && <div className="usage-dim">Last refresh failed: {p.lastError}</div>}
    </>
  );
}

function WindowRow({ provider, window: w, now }: { provider: ProviderState; window: UsageWindow; now: Date }) {
  const level = meterLevel(w.usedPercent);
  const width = Math.min(100, Math.max(0, w.usedPercent));
  const reset = resetText(w.resetsAt, now);
  return (
    <div className={`usage-window ${level}`}>
      <div className="usage-window-head">
        <span>{w.label}</span>
        <span className="usage-percent">{percentText(w.usedPercent)}</span>
      </div>
      <div
        className="usage-bar"
        role="meter"
        aria-label={`${provider.name} ${w.label}`}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(width)}
      >
        <div className="usage-bar-fill" style={{ width: `${width}%`, background: level === "normal" ? provider.color : undefined }} />
      </div>
      <div className="usage-dim">{level === "full" ? `Limit reached${reset ? ` · ${reset}` : ""}` : reset}</div>
    </div>
  );
}
