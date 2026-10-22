import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { TerminalDialog } from "../dialogs/TerminalDialog";
import { ipc } from "../ipc/commands";
import { isAppError, isBrowser, isTerminal } from "../ipc/types";
import { BROWSER_CHROME_HEIGHT, BrowserNode } from "../nodes/BrowserNode";
import { linkDragBus } from "../nodes/LinkHandle";
import { NoteNode } from "../nodes/NoteNode";
import { TerminalNode } from "../nodes/TerminalNode";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { LiveTerminalLayer } from "../terminal/LiveTerminalLayer";
import { routePointerDown, routeWheel, targetOf } from "./gestures";
import { routeKey } from "./keyboard";
import { LinkLayer, type LinkDrag } from "./LinkLayer";
import { NodeToolbar } from "./NodeToolbar";
import { ToolBar } from "./ToolBar";
import { planBrowsers } from "./browserLayout";
import { nodesInRect, rectFromPoints } from "./marquee";
import { frameToScreen, pan, screenToCanvas, zoomAt, type ScreenRect } from "./viewport";
import { startDrag } from "./useDrag";

const GESTURE_END_MS = 150;
const NOTE_SIZE = { width: 260, height: 200 };
const BROWSER_SIZE = { width: 1024, height: 720 };
/** Interface elements that a live browser page must never cover (004 FR-007). */
const COVERS = ".modal-backdrop, .menu.floating, .message-log, .node-toolbar, .tool-bar, .tool-hint, .banners, .linking-hint, .tb-palette";

function hasTextFocus() {
  const el = document.activeElement;
  return el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement || (el as HTMLElement | null)?.isContentEditable === true;
}

export function Canvas() {
  const ref = useRef<HTMLDivElement>(null);
  const nodes = useCanvas((s) => s.nodes);
  const camera = useCanvas((s) => s.camera);
  const tool = useCanvas((s) => s.tool);
  const linkingFrom = useCanvas((s) => s.linkingFrom);
  const dialog = useCanvas((s) => s.terminalDialog);
  const confirmClose = useCanvas((s) => s.confirmClose);
  const [viewport, setViewport] = useState<ScreenRect>({ left: 0, top: 0, width: 0, height: 0 });
  const [linkDrag, setLinkDrag] = useState<LinkDrag>(null);
  const [marquee, setMarquee] = useState<ScreenRect | null>(null);
  const confirmCloseMany = useCanvas((s) => s.confirmCloseMany);
  const spaceHeld = useRef(false);

  const report = (err: unknown) => useWorkspaces.getState().addWarning(isAppError(err) ? err.message : String(err));

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

  useEffect(() => {
    linkDragBus.listener = setLinkDrag;
    return () => {
      linkDragBus.listener = null;
    };
  }, []);

  const center = () =>
    screenToCanvas(useCanvas.getState().camera, { x: viewport.width / 2, y: viewport.height / 2 });

  const placeTerminalAtCenter = () => {
    const c = center();
    useCanvas.getState().openTerminalDialog({ kind: "create", at: { x: c.x - 320, y: c.y - 200 } });
  };

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

  // Keyboard: Space+drag panning, tool shortcuts, Delete, Escape (FR-002, FR-004, FR-005).
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      const s = useCanvas.getState();
      if (e.code === "Space" && !s.focusedId && !hasTextFocus()) {
        spaceHeld.current = true;
        e.preventDefault();
        return;
      }
      if (document.querySelector(".modal-backdrop")) return;
      if (e.key === "Enter" && s.tool === "terminal" && !hasTextFocus()) {
        e.preventDefault();
        placeTerminalAtCenter();
        return;
      }
      const action = routeKey({
        key: e.key,
        textFocus: hasTextFocus(),
        terminalFocus: false,
        hasSelection: s.selection !== null || s.group.length > 0,
        linking: s.linkingFrom !== null,
        meta: e.metaKey || e.ctrlKey || e.altKey,
      });
      if (!action) return;
      e.preventDefault();
      if (action === "tool:select") s.setTool("select");
      else if (action === "tool:terminal") s.setTool("terminal");
      else if (action === "tool:note") s.setTool("note");
      else if (action === "tool:browser") s.setTool("browser");
      else if (action === "delete") s.deleteSelection().catch(report);
      else if (action === "cancelLink") s.cancelLinking();
      else {
        s.select(null);
        s.setTool("select");
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [viewport]);

  // Places live browser pages over their nodes; everything else shows cards (004 research R7).
  useEffect(() => {
    const container = ref.current!;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let shown = new Set<string>();
    const run = () => {
      const s = useCanvas.getState();
      const box = container.getBoundingClientRect();
      const pages = s.nodes.filter(isBrowser).map((n) => ({
        id: n.id,
        rect: frameToScreen(s.camera, {
          x: n.frame.x,
          y: n.frame.y + BROWSER_CHROME_HEIGHT,
          width: n.frame.width,
          height: n.frame.height - BROWSER_CHROME_HEIGHT,
        }),
      }));
      const covers = [...document.querySelectorAll(COVERS)].map((el) => {
        const r = el.getBoundingClientRect();
        return { left: r.left - box.left, top: r.top - box.top, width: r.width, height: r.height };
      });
      const selectedId = s.selection?.kind === "node" ? s.selection.id : null;
      const plan = planBrowsers({ pages, viewport: { left: 0, top: 0, width: box.width, height: box.height }, zoom: s.camera.zoom, gesturing: s.gesturing, covers, selectedId });
      const items = plan.map((p) => ({
        nodeId: p.id,
        rect: p.rect && { x: p.rect.left + box.left, y: p.rect.top + box.top, width: p.rect.width, height: p.rect.height },
      }));
      const ids = new Set(items.map((i) => i.nodeId));
      for (const gone of shown) if (!ids.has(gone)) items.push({ nodeId: gone, rect: null });
      shown = ids;
      void ipc.browserLayout(items).catch(() => {});
    };
    const schedule = () => {
      clearTimeout(timer);
      timer = setTimeout(run, 50);
    };
    const unsubscribe = useCanvas.subscribe(schedule);
    const bodyObserver = new MutationObserver(schedule);
    bodyObserver.observe(document.body, { childList: true });
    const canvasObserver = new MutationObserver(schedule);
    canvasObserver.observe(container, { childList: true });
    window.addEventListener("resize", schedule);
    // The canvas also moves and resizes without a window resize (sidebar hidden or shown).
    const resizeObserver = new ResizeObserver(schedule);
    resizeObserver.observe(container);
    schedule();
    return () => {
      clearTimeout(timer);
      unsubscribe();
      bodyObserver.disconnect();
      canvasObserver.disconnect();
      window.removeEventListener("resize", schedule);
      resizeObserver.disconnect();
      void ipc.browserLayout([...shown].map((nodeId) => ({ nodeId, rect: null }))).catch(() => {});
    };
  }, []);

  const layerTransform = `translate(${-camera.x * camera.zoom}px, ${-camera.y * camera.zoom}px) scale(${camera.zoom})`;
  const grid = 24 * camera.zoom;
  const closing = confirmClose ? nodes.find((n) => n.id === confirmClose) : undefined;

  return (
    <div className="canvas-wrap">
      <div
        ref={ref}
        className={`canvas tool-${tool}${linkingFrom ? " linking" : ""}`}
        role="application"
        aria-label="Canvas"
        style={{
          backgroundSize: `${grid}px ${grid}px`,
          backgroundPosition: `${-camera.x * camera.zoom}px ${-camera.y * camera.zoom}px`,
        }}
        onPointerDownCapture={(e) => {
          const s = useCanvas.getState();
          const target = targetOf(e.target as Element);
          const action = routePointerDown({
            button: e.button,
            spaceHeld: spaceHeld.current,
            keyboardFocus: s.focusedId !== null || hasTextFocus(),
            target,
          });
          if (action === "pan") {
            e.preventDefault();
            e.stopPropagation();
            const start = s.camera;
            startDrag(e, (dx, dy) => useCanvas.getState().setCamera(pan(start, dx, dy)), () => {});
            return;
          }
          if (target.kind !== "empty" || e.button !== 0) return;
          if ((e.target as Element).closest(".link-layer g, .node-toolbar, .tool-bar-wrap")) return;
          const r = ref.current!.getBoundingClientRect();
          const at = screenToCanvas(s.camera, { x: e.clientX - r.left, y: e.clientY - r.top });
          if (s.tool === "terminal") {
            s.openTerminalDialog({ kind: "create", at });
          } else if (s.tool === "browser") {
            s.addBrowser({ x: at.x, y: at.y, ...BROWSER_SIZE }).catch(report);
            s.setTool("select");
          } else if (s.tool === "note") {
            s.addNote({ x: at.x, y: at.y, ...NOTE_SIZE })
              .then((n) => s.select({ kind: "node", id: n.id }))
              .catch(report);
            s.setTool("select");
          } else {
            // Select tool on empty canvas: clear the selection, and dragging draws a selection
            // rectangle. Shift adds to the current selection.
            const base = !e.shiftKey ? [] : s.group.length > 0 ? s.group : s.selection?.kind === "node" ? [s.selection.id] : [];
            s.cancelLinking();
            if (!e.shiftKey) s.select(null);
            s.setFocus(null);
            (document.activeElement as HTMLElement | null)?.blur?.();
            const origin = { x: e.clientX - r.left, y: e.clientY - r.top };
            startDrag(
              e,
              (dx, dy) => {
                const end = { x: origin.x + dx, y: origin.y + dy };
                setMarquee(rectFromPoints(origin, end));
                const st = useCanvas.getState();
                const area = rectFromPoints(screenToCanvas(st.camera, origin), screenToCanvas(st.camera, end));
                st.setGroup([...new Set([...base, ...nodesInRect(st.nodes, area)])]);
              },
              () => setMarquee(null),
            );
          }
        }}
      >
        <div className="canvas-layer" style={{ transform: layerTransform }}>
          <LinkLayer drag={linkDrag} />
          {nodes.map((n) =>
            isTerminal(n) ? (
              <TerminalNode key={n.id} node={n} />
            ) : isBrowser(n) ? (
              <BrowserNode key={n.id} node={n} />
            ) : (
              <NoteNode key={n.id} node={n} />
            ),
          )}
        </div>
        <LiveTerminalLayer viewport={viewport} />
        <NodeToolbar viewport={viewport} />
        {marquee && (
          <div
            className="marquee"
            style={{ left: marquee.left, top: marquee.top, width: marquee.width, height: marquee.height }}
          />
        )}
      </div>
      <ToolBar onPlaceAtCenter={placeTerminalAtCenter} />
      {linkingFrom && <div className="linking-hint">Click another item to link it · Esc to cancel</div>}
      {dialog && <TerminalDialog mode={dialog} onClose={() => useCanvas.getState().openTerminalDialog(null)} />}
      {closing && (
        <ConfirmDialog
          title="Close terminal?"
          body={`A program is still running in "${"terminal" in closing.content ? closing.content.terminal.name : ""}". Closing the terminal stops it.`}
          confirmLabel="Close"
          danger
          onCancel={() => useCanvas.getState().setConfirmClose(null)}
          onConfirm={() => {
            const s = useCanvas.getState();
            s.setConfirmClose(null);
            s.removeNode(closing.id, true).catch(report);
          }}
        />
      )}
      {confirmCloseMany && (
        <ConfirmDialog
          title={`Close ${confirmCloseMany.length === 1 ? "terminal" : `${confirmCloseMany.length} terminals`}?`}
          body={`A program is still running in ${confirmCloseMany
            .map((id) => nodes.find((n) => n.id === id))
            .map((n) => `"${n && "terminal" in n.content ? n.content.terminal.name : "a terminal"}"`)
            .join(", ")}. Closing stops it.`}
          confirmLabel="Close"
          danger
          onCancel={() => useCanvas.getState().setConfirmCloseMany(null)}
          onConfirm={() => {
            const s = useCanvas.getState();
            s.setConfirmCloseMany(null);
            for (const id of confirmCloseMany) s.removeNode(id, true).catch(report);
          }}
        />
      )}
    </div>
  );
}
