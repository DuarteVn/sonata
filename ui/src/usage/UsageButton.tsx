import { Gauge } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useUsage } from "../state/usage";
import { percentText, resetText } from "./meter";
import { UsagePanel } from "./UsagePanel";

// Top-right of the canvas area: the highest plan usage at a glance; opens the panel (FR-001).
export function UsageButton() {
  const state = useUsage((s) => s.state);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  if (!state) return null;
  const { summary } = state;
  const owner = state.providers.find((p) => p.id === summary.providerId);
  const percent = summary.percent;
  const windowName = summary.windowKind === "weekly" ? "weekly" : "short window";
  const reset = resetText(summary.resetsAt, new Date());
  const title =
    summary.state === "off"
      ? "Usage reading is off"
      : percent === null || !owner
        ? "Agent usage"
        : summary.state === "full"
          ? `${owner.name}: ${windowName} limit reached${reset ? `, ${reset}` : ""}`
          : `${owner.name}: ${percentText(percent)} of the ${windowName} limit${reset ? `, ${reset}` : ""}`;
  const showRing = percent !== null && summary.state !== "off" && summary.state !== "empty";

  return (
    <div className="usage-anchor" ref={ref}>
      <button
        type="button"
        className={`usage-button ${summary.state}`}
        aria-label={summary.state === "off" ? "Usage (off)" : "Usage"}
        aria-expanded={open}
        title={title}
        onClick={() => setOpen((v) => !v)}
      >
        {showRing ? (
          <Ring percent={percent} color={summary.state === "normal" ? (owner?.color ?? "currentColor") : "currentColor"} />
        ) : (
          <Gauge size={15} />
        )}
        {showRing && <span>{percentText(percent)}</span>}
      </button>
      {open && <UsagePanel />}
    </div>
  );
}

// Circular meter: the stroke covers the highest share used.
function Ring({ percent, color }: { percent: number; color: string }) {
  const r = 6;
  const length = 2 * Math.PI * r;
  const filled = (Math.min(100, Math.max(0, percent)) / 100) * length;
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true" className="usage-ring">
      <circle cx="8" cy="8" r={r} fill="none" stroke="var(--bg-hover)" strokeWidth="2.5" />
      <circle
        data-testid="usage-ring"
        cx="8"
        cy="8"
        r={r}
        fill="none"
        stroke={color}
        strokeWidth="2.5"
        strokeLinecap="round"
        strokeDasharray={`${filled} ${length}`}
        transform="rotate(-90 8 8)"
      />
    </svg>
  );
}
