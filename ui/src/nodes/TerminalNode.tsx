import { RotateCcw, X } from "lucide-react";
import { useState } from "react";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startDrag } from "../canvas/useDrag";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode, type TerminalContent } from "../ipc/types";
import { TITLE_HEIGHT } from "../terminal/LiveTerminalLayer";
import { TerminalMenu } from "./TerminalMenu";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";

type Props = { node: CanvasNode & { content: { terminal: TerminalContent } } };

export function TerminalNode({ node }: Props) {
  const { frame } = node;
  const term = node.content.terminal;
  const state = useCanvas((s) => s.terminalStates[node.id]);
  const focused = useCanvas((s) => s.focusedId === node.id);
  const previewFrame = useCanvas((s) => s.previewFrame);
  const commitFrame = useCanvas((s) => s.commitFrame);
  const bringToFront = useCanvas((s) => s.bringToFront);
  const setFocus = useCanvas((s) => s.setFocus);
  const removeNode = useCanvas((s) => s.removeNode);
  const renameTerminal = useCanvas((s) => s.renameTerminal);
  const addWarning = useWorkspaces((s) => s.addWarning);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(term.name);
  const [confirmClose, setConfirmClose] = useState(false);

  async function close(confirmed: boolean) {
    try {
      await removeNode(node.id, confirmed);
    } catch (err) {
      if (isAppError(err) && err.code === "TerminalBusy") setConfirmClose(true);
      else addWarning(isAppError(err) ? err.message : String(err));
    }
  }

  async function commitName() {
    setEditing(false);
    if (draft.trim() && draft.trim() !== term.name) {
      try {
        await renameTerminal(node.id, draft);
      } catch (err) {
        addWarning(isAppError(err) ? err.message : String(err));
        setDraft(term.name);
      }
    } else {
      setDraft(term.name);
    }
  }

  const exited = state?.run === "exited";

  return (
    <div
      className={`node terminal-node${focused ? " focused" : ""}`}
      data-node-id={node.id}
      data-node-kind="terminal"
      style={{ left: frame.x, top: frame.y, width: frame.width, height: frame.height, zIndex: node.z }}
    >
      <div
        className="node-title"
        style={{ height: TITLE_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || editing) return;
          if ((e.target as HTMLElement).closest("button")) return;
          setFocus(node.id);
          void bringToFront(node.id);
          const zoom = useCanvas.getState().camera.zoom;
          const moved = (dx: number, dy: number) => ({ ...frame, x: frame.x + dx / zoom, y: frame.y + dy / zoom });
          startDrag(
            e,
            (dx, dy) => previewFrame(node.id, moved(dx, dy)),
            (dx, dy) => {
              if (dx !== 0 || dy !== 0) void commitFrame(node.id, moved(dx, dy));
            },
          );
        }}
        onDoubleClick={() => {
          setDraft(term.name);
          setEditing(true);
        }}
      >
        {editing ? (
          <input
            className="node-name-input"
            autoFocus
            value={draft}
            aria-label="Terminal name"
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commitName}
            onKeyDown={(e) => {
              if (e.key === "Enter") void commitName();
              if (e.key === "Escape") {
                setDraft(term.name);
                setEditing(false);
              }
            }}
          />
        ) : (
          <span className="node-name">{term.name}</span>
        )}
        {term.launchCommand && <span className="node-sub">{term.launchCommand}</span>}
        {exited && (
          <span className="node-exit">
            exited ({state?.exitCode ?? "?"})
            <button
              type="button"
              className="icon-button"
              aria-label="Restart"
              title="Restart"
              onClick={() => void ipc.terminalRestart(node.id)}
            >
              <RotateCcw size={13} />
            </button>
          </span>
        )}
        <span className="node-spacer" />
        <TerminalMenu node={node} />
        <button type="button" className="icon-button node-close" aria-label="Close terminal" onClick={() => void close(false)}>
          <X size={14} />
        </button>
      </div>
      <div className="node-body" />
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={240} minHeight={120} />
      {confirmClose && (
        <ConfirmDialog
          title="Close terminal?"
          body={`A program is still running in "${term.name}". Closing the terminal stops it.`}
          confirmLabel="Close"
          danger
          onCancel={() => setConfirmClose(false)}
          onConfirm={() => {
            setConfirmClose(false);
            void close(true);
          }}
        />
      )}
    </div>
  );
}
