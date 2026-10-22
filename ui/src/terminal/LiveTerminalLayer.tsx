import { useEffect, useMemo, useRef, useState } from "react";
import { frameToScreen, type ScreenRect } from "../canvas/viewport";
import { ipc } from "../ipc/commands";
import { isTerminal, type CanvasNode } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { computeLiveSet, type Renderer } from "./liveSet";
import { XtermSession } from "./xtermSession";

export const TITLE_HEIGHT = 28;
export const BODY_PADDING = 6;
const SETTLE_MS = 150;
const SUBSCRIBE_DEBOUNCE_MS = 200;

/** Canvas-space rectangle of a terminal's text area. */
export function bodyFrame(node: CanvasNode) {
  return {
    x: node.frame.x + BODY_PADDING,
    y: node.frame.y + TITLE_HEIGHT + BODY_PADDING,
    width: Math.max(10, node.frame.width - BODY_PADDING * 2),
    height: Math.max(10, node.frame.height - TITLE_HEIGHT - BODY_PADDING * 2),
  };
}

// Renders live xterm.js views in screen space, above the scaled canvas (research R2). Only
// visible, legible terminals are live (R3); the rest show previews pushed by the core. During a
// pan or zoom gesture the layer follows with a CSS transform; once the camera settles, views
// snap to crisp positions and font sizes.
export function LiveTerminalLayer({ viewport }: { viewport: ScreenRect }) {
  const nodes = useCanvas((s) => s.nodes);
  const camera = useCanvas((s) => s.camera);
  const focusedId = useCanvas((s) => s.focusedId);
  const lost = useCanvas((s) => s.webglLost);
  const baseFontSize = useWorkspaces((s) => s.preferences.terminalFontSize);
  const [settled, setSettled] = useState(camera);

  useEffect(() => {
    const t = setTimeout(() => setSettled(camera), SETTLE_MS);
    return () => clearTimeout(t);
  }, [camera]);

  const terminals = useMemo(() => nodes.filter(isTerminal), [nodes]);
  const { live, previews } = useMemo(
    () =>
      computeLiveSet({
        terminals: terminals.map((n) => ({ id: n.id, rect: frameToScreen(settled, bodyFrame(n)) })),
        viewport,
        zoom: settled.zoom,
        baseFontSize,
        focusedId,
        lost,
      }),
    [terminals, settled, viewport, baseFontSize, focusedId, lost],
  );

  useEffect(() => useCanvas.getState().setLiveIds(new Set(live.map((l) => l.id))), [live]);

  const previewKey = previews.join(",");
  useEffect(() => {
    const t = setTimeout(() => void ipc.terminalPreviewSubscribe(previewKey ? previewKey.split(",") : []), SUBSCRIBE_DEBOUNCE_MS);
    return () => clearTimeout(t);
  }, [previewKey]);
  useEffect(() => () => void ipc.terminalPreviewSubscribe([]), []);

  const byId = new Map(terminals.map((n) => [n.id, n]));
  const k = camera.zoom / settled.zoom;
  const transform =
    settled === camera
      ? undefined
      : `translate(${(settled.x - camera.x) * camera.zoom}px, ${(settled.y - camera.y) * camera.zoom}px) scale(${k})`;

  return (
    <div className="live-layer" style={{ transform }}>
      {live.map(({ id, renderer }) => {
        const n = byId.get(id)!;
        return (
          <LiveTerminal
            key={id}
            node={n}
            rect={frameToScreen(settled, bodyFrame(n))}
            zoom={settled.zoom}
            baseFontSize={baseFontSize}
            focused={focusedId === id}
            renderer={renderer}
          />
        );
      })}
    </div>
  );
}

type LiveProps = {
  node: CanvasNode;
  rect: ScreenRect;
  zoom: number;
  baseFontSize: number;
  focused: boolean;
  renderer: Renderer;
};

function LiveTerminal({ node, rect, zoom, baseFontSize, focused, renderer }: LiveProps) {
  const ref = useRef<HTMLDivElement>(null);
  const session = useRef<XtermSession | null>(null);
  const { setFocus, bringToFront, setTerminalState, markWebglLost } = useCanvas.getState();

  useEffect(() => {
    const s = new XtermSession(node.id, ref.current!, {
      baseFontSize,
      zoom,
      useWebgl: renderer === "webgl",
      onState: setTerminalState,
      onWebglLost: () => markWebglLost(node.id),
    });
    session.current = s;
    void s.start();
    return () => {
      s.dispose();
      session.current = null;
    };
    // The session lives as long as this view; zoom, size, and renderer changes apply below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id]);

  useEffect(() => {
    if (renderer === "webgl") session.current?.enableWebgl();
    else session.current?.disableWebgl();
  }, [renderer]);

  useEffect(() => session.current?.setZoom(zoom), [zoom]);

  useEffect(() => {
    const t = setTimeout(() => session.current?.refit(), 120);
    return () => clearTimeout(t);
  }, [node.frame.width, node.frame.height]);

  useEffect(() => {
    if (focused) session.current?.focus();
    else session.current?.blur();
  }, [focused]);

  return (
    <div
      ref={ref}
      className="live-terminal"
      data-node-id={node.id}
      data-node-kind="terminal"
      data-renderer={renderer}
      style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height, zIndex: node.z }}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        setFocus(node.id);
        void bringToFront(node.id);
      }}
    />
  );
}
