import { Plus, StickyNote } from "lucide-react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { AddTerminalDialog } from "../dialogs/AddTerminalDialog";
import { isTerminal } from "../ipc/types";
import { NoteNode } from "../nodes/NoteNode";
import { TerminalNode } from "../nodes/TerminalNode";
import { useCanvas } from "../state/canvas";
import { LiveTerminalLayer } from "../terminal/LiveTerminalLayer";
import { routePointerDown, routeWheel, targetOf } from "./gestures";
import { pan, screenToCanvas, zoomAt, type ScreenRect } from "./viewport";
import { startDrag } from "./useDrag";

const GESTURE_END_MS = 150;

function hasTextFocus() {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement;
}

export function Canvas() {
  const ref = useRef<HTMLDivElement>(null);
  const nodes = useCanvas((s) => s.nodes);
  const camera = useCanvas((s) => s.camera);
  const [viewport, setViewport] = useState<ScreenRect>({ left: 0, top: 0, width: 0, height: 0 });
  const [adding, setAdding] = useState(false);
  const spaceHeld = useRef(false);

  useLayoutEffect(() => {
    const el = ref.current!;
    const update = () => {
      const r = el.getBoundingClientRect();
      setViewport({ left: r.left, top: r.top, width: r.width, height: r.height });
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Wheel and pinch (FR-008a). Capture phase, so unfocused terminals never scroll themselves.
  useEffect(() => {
    const el = ref.current!;
    let gestureTimer: ReturnType<typeof setTimeout> | undefined;
    const onWheel = (e: WheelEvent) => {
      const { camera, focusedId, setCamera, setGesturing } = useCanvas.getState();
      const action = routeWheel({
        ctrlKey: e.ctrlKey,
        deltaX: e.deltaX,
        deltaY: e.deltaY,
        target: targetOf(e.target as Element),
        focusedId,
      });
      if (action.type === "scrollTerminal") return;
      e.preventDefault();
      e.stopPropagation();
      setGesturing(true);
      clearTimeout(gestureTimer);
      gestureTimer = setTimeout(() => setGesturing(false), GESTURE_END_MS);
      if (action.type === "pan") setCamera(pan(camera, action.dx, action.dy));
      else {
        const r = el.getBoundingClientRect();
        setCamera(zoomAt(camera, { x: e.clientX - r.left, y: e.clientY - r.top }, action.factor));
      }
    };
    el.addEventListener("wheel", onWheel, { capture: true, passive: false });
    return () => el.removeEventListener("wheel", onWheel, { capture: true });
  }, []);

  // Space + drag pans only when nothing has keyboard focus, so typing a space never pans.
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.code === "Space" && !useCanvas.getState().focusedId && !hasTextFocus()) {
        spaceHeld.current = true;
        e.preventDefault();
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "Space") spaceHeld.current = false;
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
    };
  }, []);

  const addTerminal = async (opts: { name?: string; launchCommand?: string }) => {
    const { camera, addTerminal } = useCanvas.getState();
    const center = screenToCanvas(camera, { x: viewport.width / 2, y: viewport.height / 2 });
    const offset = (nodes.length % 6) * 24;
    await addTerminal({ x: center.x - 320 + offset, y: center.y - 200 + offset, width: 640, height: 400 }, opts);
  };

  const addNote = async () => {
    const { camera, addNote } = useCanvas.getState();
    const center = screenToCanvas(camera, { x: viewport.width / 2, y: viewport.height / 2 });
    await addNote({ x: center.x - 130, y: center.y - 100, width: 260, height: 200 });
  };

  const layerTransform = `translate(${-camera.x * camera.zoom}px, ${-camera.y * camera.zoom}px) scale(${camera.zoom})`;
  const grid = 24 * camera.zoom;

  return (
    <div className="canvas-wrap">
      <div
        ref={ref}
        className="canvas"
        role="application"
        aria-label="Canvas"
        style={{
          backgroundSize: `${grid}px ${grid}px`,
          backgroundPosition: `${-camera.x * camera.zoom}px ${-camera.y * camera.zoom}px`,
        }}
        onPointerDownCapture={(e) => {
          const { focusedId, setFocus } = useCanvas.getState();
          const action = routePointerDown({
            button: e.button,
            spaceHeld: spaceHeld.current,
            keyboardFocus: focusedId !== null || hasTextFocus(),
            target: targetOf(e.target as Element),
          });
          if (action === "pan") {
            e.preventDefault();
            e.stopPropagation();
            const start = useCanvas.getState().camera;
            startDrag(
              e,
              (dx, dy) => useCanvas.getState().setCamera(pan(start, dx, dy)),
              () => {},
            );
          } else if (action === "clearFocus") {
            setFocus(null);
            (document.activeElement as HTMLElement | null)?.blur?.();
          }
        }}
      >
        <div className="canvas-layer" style={{ transform: layerTransform }}>
          {nodes.map((n) => (isTerminal(n) ? <TerminalNode key={n.id} node={n} /> : <NoteNode key={n.id} node={n} />))}
        </div>
        <LiveTerminalLayer viewport={viewport} />
      </div>
      <div className="canvas-toolbar">
        <button type="button" onClick={() => setAdding(true)} aria-label="Add terminal">
          <Plus size={14} /> Terminal
        </button>
        <button type="button" onClick={() => void addNote()} aria-label="Add note">
          <StickyNote size={14} /> Note
        </button>
      </div>
      {adding && <AddTerminalDialog onAdd={addTerminal} onClose={() => setAdding(false)} />}
    </div>
  );
}
