import { Plus } from "lucide-react";
import { useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { isAppError, type WorkspaceSummary } from "../ipc/types";
import { useWorkspaces } from "../state/workspaces";
import { WorkspaceItem } from "./WorkspaceItem";

const DRAG_THRESHOLD_PX = 4;

export function Sidebar({ onCreate }: { onCreate: () => void }) {
  const workspaces = useWorkspaces((s) => s.workspaces);
  const activeId = useWorkspaces((s) => s.activeId);
  const { open, rename, setPinned, reorder, remove, addWarning } = useWorkspaces.getState();
  const [deleting, setDeleting] = useState<WorkspaceSummary | null>(null);
  const [draggingId, setDraggingId] = useState<string | null>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const suppressClick = useRef(false);

  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));

  // Pointer-based reordering (HTML5 drag and drop is unreliable inside the app's webview).
  function startDrag(e: ReactPointerEvent, id: string): boolean {
    const el = e.currentTarget as HTMLElement;
    const startY = e.clientY;
    let active = false;
    el.setPointerCapture(e.pointerId);
    const indexAt = (y: number) => {
      const items = [...(listRef.current?.querySelectorAll<HTMLElement>(".ws-item") ?? [])];
      const i = items.findIndex((item) => {
        const r = item.getBoundingClientRect();
        return y < r.top + r.height / 2;
      });
      return i === -1 ? items.length - 1 : i;
    };
    const move = (ev: PointerEvent) => {
      if (!active && Math.abs(ev.clientY - startY) > DRAG_THRESHOLD_PX) {
        active = true;
        setDraggingId(id);
      }
    };
    const up = (ev: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      if (!active) return;
      suppressClick.current = true;
      setTimeout(() => (suppressClick.current = false), 0);
      setDraggingId(null);
      const ids = useWorkspaces.getState().workspaces.map((w) => w.id);
      const from = ids.indexOf(id);
      const to = indexAt(ev.clientY);
      if (from === -1 || from === to) return;
      ids.splice(from, 1);
      ids.splice(to, 0, id);
      reorder(ids).catch(report);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    return false;
  }

  return (
    <nav className="sidebar" aria-label="Workspaces">
      <header className="sidebar-header">
        <span>Workspaces</span>
        <button type="button" className="icon-button" aria-label="New workspace" onClick={onCreate}>
          <Plus size={16} />
        </button>
      </header>
      <ul className="ws-list" ref={listRef}>
        {workspaces.map((w) => (
          <WorkspaceItem
            key={w.id}
            workspace={w}
            active={w.id === activeId}
            dragging={w.id === draggingId}
            onOpen={() => {
              if (!suppressClick.current) open(w.id).catch(report);
            }}
            onRename={(name) => rename(w.id, name).catch((err) => {
              report(err);
              throw err;
            })}
            onTogglePin={() => setPinned(w.id, !w.isPinned).catch(report)}
            onDelete={() => setDeleting(w)}
            onDragStart={(e) => startDrag(e, w.id)}
          />
        ))}
      </ul>
      {deleting && (
        <ConfirmDialog
          title={`Delete "${deleting.name}"?`}
          body={`This stops its terminals and removes its layout, notes, and history from Sonata. The folder ${deleting.workingDirectory} is not touched. This cannot be undone.`}
          confirmLabel="Delete"
          danger
          onCancel={() => setDeleting(null)}
          onConfirm={() => {
            const id = deleting.id;
            setDeleting(null);
            remove(id).catch(report);
          }}
        />
      )}
    </nav>
  );
}
