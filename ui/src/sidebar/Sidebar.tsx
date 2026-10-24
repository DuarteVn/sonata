import { PanelLeftClose, Plus } from "lucide-react";
import { useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { startDrag as startPointerDrag } from "../canvas/useDrag";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { isAppError, type WorkspaceSummary } from "../ipc/types";
import { useWorkspaces } from "../state/workspaces";
import { LoadFailureItem } from "./LoadFailureItem";
import { UpdateBanner } from "./UpdateBanner";
import { WorkspaceItem } from "./WorkspaceItem";

const DRAG_THRESHOLD_PX = 4;
export const SIDEBAR_DEFAULT_WIDTH = 232;
const SIDEBAR_MIN_WIDTH = 180;
const SIDEBAR_MAX_WIDTH = 480;
const SIDEBAR_KEY_STEP = 16;

export function clampSidebarWidth(width: number): number {
  return Math.round(Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, width)));
}

type Props = {
  onCreate: () => void;
  onHide: () => void;
  width: number;
  onResize: (width: number) => void;
  /** Starts the update install flow from the banner (feature 005). */
  onInstallUpdate?: () => void;
};

export function Sidebar({ onCreate, onHide, width, onResize, onInstallUpdate }: Props) {
  const workspaces = useWorkspaces((s) => s.workspaces);
  const activeId = useWorkspaces((s) => s.activeId);
  const loadFailures = useWorkspaces((s) => s.loadFailures);
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
    <nav className="sidebar" aria-label="Workspaces" style={{ width }}>
      <header className="sidebar-header">
        <span>Workspaces</span>
        <span className="sidebar-actions">
          <button type="button" className="icon-button" aria-label="New workspace" onClick={onCreate}>
            <Plus size={16} />
          </button>
          <button type="button" className="icon-button" aria-label="Hide sidebar" title="Hide sidebar (⌃⌘S)" onClick={onHide}>
            <PanelLeftClose size={16} />
          </button>
        </span>
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
        {loadFailures.map((f) => (
          <LoadFailureItem key={f.path} failure={f} />
        ))}
      </ul>
      {onInstallUpdate && <UpdateBanner onInstall={onInstallUpdate} />}
      <div
        className="sidebar-resize"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        aria-valuenow={width}
        aria-valuemin={SIDEBAR_MIN_WIDTH}
        aria-valuemax={SIDEBAR_MAX_WIDTH}
        tabIndex={0}
        title="Drag to resize · double-click to reset"
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          const start = width;
          startPointerDrag(e, (dx) => onResize(clampSidebarWidth(start + dx)), () => {});
        }}
        onDoubleClick={() => onResize(SIDEBAR_DEFAULT_WIDTH)}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft") onResize(clampSidebarWidth(width - SIDEBAR_KEY_STEP));
          else if (e.key === "ArrowRight") onResize(clampSidebarWidth(width + SIDEBAR_KEY_STEP));
        }}
      />
      {deleting && (
        <ConfirmDialog
          title={`Delete "${deleting.name}"?`}
          body={`This stops its terminals and removes its layout, notes, and history from Sonata. The folder ${deleting.workingDirectory} is not touched; lane folders are removed and their branches kept (lanes with uncommitted work block the deletion). This cannot be undone.`}
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
