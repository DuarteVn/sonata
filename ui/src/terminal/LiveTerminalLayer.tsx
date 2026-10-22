import { useEffect, useMemo, useRef, useState } from "react";
import { frameToScreen, intersects, type ScreenRect } from "../canvas/viewport";
import { isTerminal, type Camera, type CanvasNode } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { XtermSession } from "./xtermSession";

export const TITLE_HEIGHT = 28;
export const BODY_PADDING = 6;
/** Below this effective font size a terminal is shown as a preview instead (research R3). */
export const MIN_LEGIBLE_FONT_PX = 7;
const SETTLE_MS = 150;

/** Canvas-space rectangle of a terminal's text area. */
export function bodyFrame(node: CanvasNode) {
  return {
    x: node.frame.x + BODY_PADDING,
    y: node.frame.y + TITLE_HEIGHT + BODY_PADDING,
    width: Math.max(10, node.frame.width - BODY_PADDING * 2),
    height: Math.max(10, node.frame.height - TITLE_HEIGHT - BODY_PADDING * 2),
  };
}

export function isLegible(camera: Camera, baseFontSize: number) {
  return baseFontSize * camera.zoom >= MIN_LEGIBLE_FONT_PX;
}

// Renders live xterm.js views in screen space, above the scaled canvas (research R2). During a
// pan or zoom gesture the whole layer follows with a CSS transform; once the camera settles,
// views snap to crisp positions and font sizes.
export function LiveTerminalLayer({ viewport }: { viewport: ScreenRect }) {
  const nodes = useCanvas((s) => s.nodes);
  const camera = useCanvas((s) => s.camera);
  const focusedId = useCanvas((s) => s.focusedId);
  const baseFontSize = useWorkspaces((s) => s.preferences.terminalFontSize);
  const [settled, setSettled] = useState(camera);

  useEffect(() => {
    const t = setTimeout(() => setSettled(camera), SETTLE_MS);
    return () => clearTimeout(t);
  }, [camera]);

  const live = useMemo(() => {
    if (!isLegible(settled, baseFontSize)) return [];
    return nodes.filter(
      (n) => isTerminal(n) && intersects(frameToScreen(settled, bodyFrame(n)), { ...viewport, left: 0, top: 0 }),
    );
  }, [nodes, settled, baseFontSize, viewport]);

  const k = camera.zoom / settled.zoom;
  const transform =
    settled === camera
      ? undefined
      : `translate(${(settled.x - camera.x) * camera.zoom}px, ${(settled.y - camera.y) * camera.zoom}px) scale(${k})`;

  return (
    <div className="live-layer" style={{ transform }}>
      {live.map((n) => (
        <LiveTerminal
          key={n.id}
          node={n}
          rect={frameToScreen(settled, bodyFrame(n))}
          zoom={settled.zoom}
          baseFontSize={baseFontSize}
          focused={focusedId === n.id}
        />
      ))}
    </div>
  );
}

type LiveProps = { node: CanvasNode; rect: ScreenRect; zoom: number; baseFontSize: number; focused: boolean };

function LiveTerminal({ node, rect, zoom, baseFontSize, focused }: LiveProps) {
  const ref = useRef<HTMLDivElement>(null);
  const session = useRef<XtermSession | null>(null);
  const setFocus = useCanvas((s) => s.setFocus);
  const bringToFront = useCanvas((s) => s.bringToFront);
  const setTerminalState = useCanvas((s) => s.setTerminalState);

  useEffect(() => {
    const s = new XtermSession(node.id, ref.current!, {
      baseFontSize,
      zoom,
      useWebgl: true,
      onState: setTerminalState,
    });
    session.current = s;
    void s.start();
    return () => {
      s.dispose();
      session.current = null;
    };
    // The session lives as long as this view; zoom and size changes are applied below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [node.id]);

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
      style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height, zIndex: node.z }}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        setFocus(node.id);
        void bringToFront(node.id);
      }}
    />
  );
}
