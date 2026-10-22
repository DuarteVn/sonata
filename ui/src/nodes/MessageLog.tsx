import { X } from "lucide-react";
import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { ipc } from "../ipc/commands";
import type { ExchangeEntry } from "../ipc/types";
import { useCanvas } from "../state/canvas";

const OUTCOME: Record<string, string> = {
  answered: "answered",
  replied: "replied",
  timedOut: "timed out",
  failed: "failed",
  cancelled: "cancelled",
  queued: "queued",
  waiting: "waiting",
};

// Last exchanges of one terminal (003 FR-028). Kept in memory by the core.
export function MessageLog({ nodeId, onClose }: { nodeId: string; onClose: () => void }) {
  const [entries, setEntries] = useState<ExchangeEntry[] | null>(null);
  const nodes = useCanvas((s) => s.nodes);
  const nameOf = (id: string) => {
    const n = nodes.find((x) => x.id === id);
    return n && "terminal" in n.content ? n.content.terminal.name : "a removed terminal";
  };

  useEffect(() => {
    ipc.exchangeLog(nodeId).then(setEntries).catch(() => setEntries([]));
  }, [nodeId]);

  return createPortal(
    <div className="message-log" role="dialog" aria-label="Message log">
      <header>
        <strong>Messages</strong>
        <button type="button" className="icon-button" aria-label="Close" onClick={onClose}>
          <X size={14} />
        </button>
      </header>
      {entries === null ? (
        <p className="dim">Loading…</p>
      ) : entries.length === 0 ? (
        <p className="dim">No messages yet. Agents talk through links with the `sonata` command.</p>
      ) : (
        <ul>
          {entries.map((e, i) => (
            <li key={i}>
              <span className="dim">
                {new Date(e.at).toLocaleTimeString()} · {e.direction === "in" ? `from ${nameOf(e.other)}` : `to ${nameOf(e.other)}`} ·{" "}
                {OUTCOME[e.outcome] ?? e.outcome} · {e.resultChars} chars
              </span>
              <span>{e.firstLine}</span>
            </li>
          ))}
        </ul>
      )}
    </div>,
    document.body,
  );
}
