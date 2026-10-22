import type { PointerEvent as ReactPointerEvent } from "react";
import type { Frame } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { startDrag } from "./useDrag";

/** Drags a node by its title bar; when it is part of the selected group, the whole group moves. */
export function startNodeDrag(e: ReactPointerEvent, id: string) {
  const s = useCanvas.getState();
  const ids = s.group.includes(id) ? s.group : [id];
  const starts = new Map<string, Frame>();
  for (const n of s.nodes) if (ids.includes(n.id)) starts.set(n.id, n.frame);
  const zoom = s.camera.zoom;
  const moved = (dx: number, dy: number) => {
    const frames: Record<string, Frame> = {};
    for (const [nodeId, f] of starts) frames[nodeId] = { ...f, x: f.x + dx / zoom, y: f.y + dy / zoom };
    return frames;
  };
  startDrag(
    e,
    (dx, dy) => useCanvas.getState().previewFrames(moved(dx, dy)),
    (dx, dy) => {
      if (dx === 0 && dy === 0) return;
      const { commitFrame } = useCanvas.getState();
      for (const [nodeId, frame] of Object.entries(moved(dx, dy))) void commitFrame(nodeId, frame);
    },
  );
}
