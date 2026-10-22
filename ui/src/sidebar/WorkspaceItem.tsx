import { AlertTriangle, Pin, PinOff, Trash2 } from "lucide-react";
import { useState, type PointerEvent as ReactPointerEvent } from "react";
import type { WorkspaceSummary } from "../ipc/types";
import { WorkspaceIcon } from "./WorkspaceIcon";

type Props = {
  workspace: WorkspaceSummary;
  active: boolean;
  dragging: boolean;
  onOpen: () => void;
  onRename: (name: string) => Promise<void>;
  onTogglePin: () => void;
  onDelete: () => void;
  onDragStart: (e: ReactPointerEvent) => boolean;
};

export function WorkspaceItem(props: Props) {
  const { workspace: w, active, dragging } = props;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(w.name);

  async function commit() {
    setEditing(false);
    if (draft.trim() && draft.trim() !== w.name) {
      try {
        await props.onRename(draft);
      } catch {
        setDraft(w.name);
      }
    }
  }

  return (
    <li className={`ws-item${active ? " active" : ""}${dragging ? " dragging" : ""}`} data-ws-id={w.id}>
      {editing ? (
        <div className="ws-open">
          <WorkspaceIcon icon={w.icon} color={w.color} size={20} />
          <input
            className="ws-name-input"
            aria-label="Workspace name"
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === "Enter") void commit();
              if (e.key === "Escape") {
                setDraft(w.name);
                setEditing(false);
              }
            }}
          />
        </div>
      ) : (
        <button
          type="button"
          className="ws-open"
          title={w.workingDirectory}
          onPointerDown={(e) => {
            if (e.button === 0 && props.onDragStart(e)) e.preventDefault();
          }}
          onClick={(e) => {
            if (e.detail === 1) props.onOpen();
          }}
          onDoubleClick={() => {
            setDraft(w.name);
            setEditing(true);
          }}
        >
          <WorkspaceIcon icon={w.icon} color={w.color} size={20} />
          <span className="ws-name">{w.name}</span>
          {w.workingDirectoryMissing && (
            <span className="badge warning" title={`Folder not found: ${w.workingDirectory}`}>
              <AlertTriangle size={12} aria-label="Folder not found" />
            </span>
          )}
          {w.busyTerminalCount > 0 && (
            <span className="badge busy" title={`${w.busyTerminalCount} running`}>
              {w.busyTerminalCount}
            </span>
          )}
        </button>
      )}
      <div className="ws-actions">
        <button
          type="button"
          className="icon-button"
          aria-label={`${w.isPinned ? "Unpin" : "Pin"} ${w.name}`}
          aria-pressed={w.isPinned}
          onClick={props.onTogglePin}
        >
          {w.isPinned ? <PinOff size={13} /> : <Pin size={13} />}
        </button>
        <button type="button" className="icon-button" aria-label={`Delete ${w.name}`} onClick={props.onDelete}>
          <Trash2 size={13} />
        </button>
      </div>
      {w.isPinned && <Pin size={11} className="ws-pinned-mark" aria-hidden />}
    </li>
  );
}
