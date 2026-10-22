import { MoreHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode } from "../ipc/types";
import { useWorkspaces } from "../state/workspaces";

// Per-terminal options: history saving (FR-015a).
export function TerminalMenu({ node }: { node: CanvasNode }) {
  const initial = "terminal" in node.content ? node.content.terminal.saveHistory : true;
  const [open, setOpen] = useState(false);
  const [saveHistory, setSaveHistory] = useState(initial);
  const ref = useRef<HTMLDivElement>(null);
  const addWarning = useWorkspaces((s) => s.addWarning);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));

  useEffect(() => setSaveHistory(initial), [initial]);

  useEffect(() => {
    if (!open) return;
    const close = (e: PointerEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener("pointerdown", close);
    return () => window.removeEventListener("pointerdown", close);
  }, [open]);

  return (
    <div className="menu-anchor" ref={ref}>
      <button
        type="button"
        className="icon-button"
        aria-label="Terminal options"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <MoreHorizontal size={14} />
      </button>
      {open && (
        <div role="menu" className="menu">
          <button
            type="button"
            role="menuitemcheckbox"
            aria-checked={saveHistory}
            onClick={() => {
              const next = !saveHistory;
              setSaveHistory(next);
              setOpen(false);
              ipc.terminalSetSaveHistory(node.id, next).catch((err) => {
                setSaveHistory(!next);
                report(err);
              });
            }}
          >
            <span className="menu-check" aria-hidden>{saveHistory ? "✓" : ""}</span>
            Save history
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              ipc.terminalClearHistory(node.id).catch(report);
            }}
          >
            <span className="menu-check" aria-hidden />
            Clear saved history
          </button>
        </div>
      )}
    </div>
  );
}
