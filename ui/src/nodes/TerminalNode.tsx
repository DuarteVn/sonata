import { Bell, Crown, RotateCcw, X } from "lucide-react";
import { useState } from "react";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startDrag } from "../canvas/useDrag";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode, type TerminalContent } from "../ipc/types";
import { TITLE_HEIGHT } from "../terminal/LiveTerminalLayer";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { LinkCount, LinkHandle } from "./LinkHandle";
import { TerminalMenu } from "./TerminalMenu";

type Props = { node: CanvasNode & { content: { terminal: TerminalContent } } };

function roleLabel(t: TerminalContent) {
  if (!t.role) return null;
  if (t.role.name) return t.role.name;
  const words = t.role.text.trim().split(/\s+/).slice(0, 4).join(" ");
  return words.length < t.role.text.trim().length ? `${words}…` : words;
}

export function TerminalNode({ node }: Props) {
  const { frame } = node;
  const term = node.content.terminal;
  const state = useCanvas((s) => s.terminalStates[node.id]);
  const focused = useCanvas((s) => s.focusedId === node.id);
  const selected = useCanvas((s) => s.selection?.kind === "node" && s.selection.id === node.id);
  const attention = useCanvas((s) => s.attention.has(node.id));
  const live = useCanvas((s) => s.liveIds.has(node.id));
  const preview = useCanvas((s) => s.previews[node.id]);
  const { previewFrame, commitFrame, bringToFront, pressNode, setConfirmClose, removeNode, renameTerminal } =
    useCanvas.getState();
  const addWarning = useWorkspaces((s) => s.addWarning);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(term.name);

  async function close() {
    try {
      await removeNode(node.id, false);
    } catch (err) {
      if (isAppError(err) && err.code === "TerminalBusy") setConfirmClose(node.id);
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
  const monitor = term.monitorActivity !== false;
  const activity = exited ? "exited" : state?.busy ? "running" : "idle";
  const role = roleLabel(term);

  return (
    <div
      className={`node terminal-node${focused ? " focused" : ""}${selected ? " selected" : ""}`}
      data-node-id={node.id}
      data-node-kind="terminal"
      style={{
        left: frame.x,
        top: frame.y,
        width: frame.width,
        height: frame.height,
        zIndex: node.z,
        ...(term.accentColor ? { ["--accent-node" as string]: term.accentColor } : {}),
      }}
    >
      <div
        className={`node-title${term.accentColor ? " accented" : ""}`}
        style={{ height: TITLE_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || editing) return;
          if ((e.target as HTMLElement).closest("button, .link-handle")) return;
          if (pressNode(node.id, "terminal")) return;
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
        {monitor && <span className={`activity-dot ${activity}`} title={`Activity: ${activity}`} aria-label={`Activity: ${activity}`} />}
        {term.orchestrator && (
          <span className="badge orchestrator" title="Orchestrator">
            <Crown size={11} aria-label="Orchestrator" />
          </span>
        )}
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
        {role && <span className="role-chip" title={term.role?.text}>{role}</span>}
        {!role && term.launchCommand && <span className="node-sub">{term.launchCommand}</span>}
        {exited && (
          <span className="node-exit">
            exited ({state?.exitCode ?? "?"})
            <button type="button" className="icon-button" aria-label="Restart" title="Restart" onClick={() => void ipc.terminalRestart(node.id, true)}>
              <RotateCcw size={13} />
            </button>
          </span>
        )}
        {monitor && attention && (
          <span className="badge attention" title="Finished while you were away">
            <Bell size={11} aria-label="Needs attention" />
          </span>
        )}
        <span className="node-spacer" />
        <LinkCount nodeId={node.id} />
        <TerminalMenu node={node} />
        <button type="button" className="icon-button node-close" aria-label="Close terminal" onClick={() => void close()}>
          <X size={14} />
        </button>
      </div>
      <div className="node-body">
        {!live && (
          <pre className="terminal-preview" aria-label={`Preview of ${term.name}`} style={term.fontSize ? { fontSize: term.fontSize } : undefined}>
            {(preview ?? []).join("\n")}
          </pre>
        )}
      </div>
      {(selected || focused) && <LinkHandle nodeId={node.id} />}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={240} minHeight={120} />
    </div>
  );
}
