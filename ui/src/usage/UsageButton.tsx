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
  const title =
    summary.state === "off"
      ? "Usage reading is off"
      : percent === null || !owner
        ? "Agent usage"
        : `${owner.name}: ${percentText(percent)} of the ${
            summary.windowKind === "weekly" ? "weekly" : "short window"
          } limit, ${resetText(summary.resetsAt, new Date())}`;

  return (
    <div className="usage-anchor" ref={ref}>
      <button
        type="button"
        className={`usage-button ${summary.state}`}
        aria-label="Usage"
        aria-expanded={open}
        title={title}
        onClick={() => setOpen((v) => !v)}
      >
        <Gauge size={15} />
        {percent !== null && summary.state !== "off" && <span>{percentText(percent)}</span>}
      </button>
      {open && <UsagePanel />}
    </div>
  );
}
