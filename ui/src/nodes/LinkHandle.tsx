import type { PointerEvent as ReactPointerEvent } from "react";
import { screenToCanvas } from "../canvas/viewport";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { isAppError } from "../ipc/types";

/** Shared drag state for the link preview (the LinkLayer reads it). */
export const linkDragBus = {
  listener: null as null | ((drag: { from: string; x: number; y: number } | null) => void),
  emit(drag: { from: string; x: number; y: number } | null) {
    this.listener?.(drag);
  },
};

function canvasPoint(e: PointerEvent) {
  const canvas = document.querySelector<HTMLElement>(".canvas");
  const r = canvas?.getBoundingClientRect() ?? { left: 0, top: 0 };
  return screenToCanvas(useCanvas.getState().camera, { x: e.clientX - r.left, y: e.clientY - r.top });
}

// Drag from this handle onto another node to link them (FR-021).
export function LinkHandle({ nodeId }: { nodeId: string }) {
  function start(e: ReactPointerEvent<HTMLDivElement>) {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    const el = e.currentTarget;
    el.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => linkDragBus.emit({ from: nodeId, ...canvasPoint(ev) });
    const up = (ev: PointerEvent) => {
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      linkDragBus.emit(null);
      const target = document
        .elementsFromPoint(ev.clientX, ev.clientY)
        .map((x) => x.closest<HTMLElement>("[data-node-id]"))
        .find((x) => x && x.dataset.nodeId !== nodeId);
      const id = target?.dataset.nodeId;
      if (!id) return;
      useCanvas
        .getState()
        .createLink(nodeId, id)
        .catch((err) => {
          if (!(isAppError(err) && err.code === "LinkExists")) {
            useWorkspaces.getState().addWarning(isAppError(err) ? err.message : String(err));
          }
        });
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
  }

  return <div className="link-handle" role="button" aria-label="Drag to link" title="Drag to another item to link" onPointerDown={start} />;
}

export function LinkCount({ nodeId }: { nodeId: string }) {
  const count = useCanvas((s) => s.links.filter((l) => l.a === nodeId || l.b === nodeId).length);
  if (count === 0) return null;
  return (
    <span className="link-count" title={`${count} link${count === 1 ? "" : "s"}`} aria-label={`${count} links`}>
      {count}
    </span>
  );
}
