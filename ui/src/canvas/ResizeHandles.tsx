import type { Frame } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { startDrag } from "./useDrag";

type Props = { nodeId: string; frame: Frame; minWidth: number; minHeight: number };

// Bottom-right resize handle. Sizes are in canvas units; the drag delta is divided by zoom.
export function ResizeHandles({ nodeId, frame, minWidth, minHeight }: Props) {
  const previewFrame = useCanvas((s) => s.previewFrame);
  const commitFrame = useCanvas((s) => s.commitFrame);

  return (
    <div
      className="resize-handle"
      aria-label="Resize"
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.stopPropagation();
        const zoom = useCanvas.getState().camera.zoom;
        const size = (dx: number, dy: number): Frame => ({
          ...frame,
          width: Math.max(minWidth, frame.width + dx / zoom),
          height: Math.max(minHeight, frame.height + dy / zoom),
        });
        startDrag(
          e,
          (dx, dy) => previewFrame(nodeId, size(dx, dy)),
          (dx, dy) => void commitFrame(nodeId, size(dx, dy)),
        );
      }}
    />
  );
}
