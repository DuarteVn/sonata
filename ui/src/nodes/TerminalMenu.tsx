import { MoreHorizontal } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { ConfirmDialog, RESTART_CONFIRM } from "../dialogs/ConfirmDialog";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";

// Per-terminal options: history saving (FR-015a), lanes (008), Start fresh (009). The menu is rendered in a portal on top of
// everything: inside the scaled canvas it would sit under the live terminal layer.
export function TerminalMenu({ node }: { node: CanvasNode }) {
  const initial = "terminal" in node.content ? node.content.terminal.saveHistory : true;
  const lane = "terminal" in node.content ? node.content.terminal.lane : undefined;
  const laneState = useCanvas((s) => s.laneStatuses[node.id]?.state);
  const missing = laneState === "checkoutMissing";
  const activeLane = lane && !lane.landedAt && !missing ? lane : undefined;
  const conversation = "terminal" in node.content ? node.content.terminal.conversation : undefined;
  const [confirmFresh, setConfirmFresh] = useState(false);
  const [open, setOpen] = useState(false);
  const [saveHistory, setSaveHistory] = useState(initial);
  const ref = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ top: number; right: number } | null>(null);
  const addWarning = useWorkspaces((s) => s.addWarning);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));

  useEffect(() => setSaveHistory(initial), [initial]);

  async function startFresh(confirmed: boolean) {
    try {
      await ipc.terminalStartFresh(node.id, confirmed);
    } catch (err) {
      if (isAppError(err) && err.code === "TerminalBusy") setConfirmFresh(true);
      else report(err);
    }
  }

  useEffect(() => {
    if (!open) return;
    const close = (e: Event) => {
      const t = e.target as Node;
      if (!ref.current?.contains(t) && !menuRef.current?.contains(t)) setOpen(false);
    };
    const dismiss = () => setOpen(false);
    window.addEventListener("pointerdown", close);
    window.addEventListener("wheel", dismiss, { capture: true });
    window.addEventListener("resize", dismiss);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("wheel", dismiss, { capture: true });
      window.removeEventListener("resize", dismiss);
    };
  }, [open]);

  return (
    <div className="menu-anchor" ref={ref}>
      <button
        type="button"
        className="icon-button"
        aria-label="Terminal options"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={(e) => {
          const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
          setPos({ top: r.bottom + 4, right: window.innerWidth - r.right });
          setOpen((v) => !v);
        }}
      >
        <MoreHorizontal size={14} />
      </button>
      {open &&
        createPortal(
        <div role="menu" className="menu floating" ref={menuRef} style={pos ?? undefined}>
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
          {conversation && (
            <>
              <div className="menu-separator" role="separator" />
              <button
                type="button"
                role="menuitem"
                title="Forget this terminal's conversation and restart the agent in a new one."
                onClick={() => {
                  setOpen(false);
                  void startFresh(false);
                }}
              >
                <span className="menu-check" aria-hidden />
                Start fresh
              </button>
            </>
          )}
          {activeLane && (
            <>
              <div className="menu-separator" role="separator" />
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setOpen(false);
                  useCanvas.getState().openLaneDialog({ nodeId: node.id, action: "land" });
                }}
              >
                <span className="menu-check" aria-hidden />
                Land into {activeLane.baseBranch}…
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setOpen(false);
                  useCanvas.getState().openLaneDialog({ nodeId: node.id, action: "remove" });
                }}
              >
                <span className="menu-check" aria-hidden />
                Remove lane…
              </button>
            </>
          )}
          {lane && !lane.landedAt && missing && (
            <>
              <div className="menu-separator" role="separator" />
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setOpen(false);
                  ipc.laneForget(node.id).catch(report);
                }}
              >
                <span className="menu-check" aria-hidden />
                Forget lane
              </button>
            </>
          )}
        </div>,
          document.body,
        )}
      {confirmFresh &&
        createPortal(
          <ConfirmDialog
            {...RESTART_CONFIRM}
            danger
            onCancel={() => setConfirmFresh(false)}
            onConfirm={() => {
              setConfirmFresh(false);
              void startFresh(true);
            }}
          />,
          document.body,
        )}
    </div>
  );
}
