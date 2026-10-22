import { convertFileSrc } from "@tauri-apps/api/core";
import { ArrowLeft, ArrowRight, Code2, ExternalLink, Globe, RotateCw, X } from "lucide-react";
import { useEffect, useState } from "react";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startDrag } from "../canvas/useDrag";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { LinkCount, LinkHandle } from "./LinkHandle";

export const BROWSER_CHROME_HEIGHT = 36;

function imageUrl(path: string | undefined, version: number) {
  if (!path) return undefined;
  try {
    return `${convertFileSrc(path)}?v=${version}`;
  } catch {
    return undefined;
  }
}

// A web page on the canvas (feature 004). The page itself is a native view placed over the
// node's body by the shell while live; otherwise the body shows a card.
export function BrowserNode({ node }: { node: CanvasNode }) {
  const { frame } = node;
  const saved = "browser" in node.content ? node.content.browser : { url: "about:blank" };
  const state = useCanvas((s) => s.browserStates[node.id]);
  const selected = useCanvas((s) => s.selection?.kind === "node" && s.selection.id === node.id);
  const controlledBy = useCanvas((s) =>
    Object.values(s.asks)
      .filter((a) => a.to === node.id)
      .map((a) => s.nodes.find((n) => n.id === a.from))
      .map((n) => (n && "terminal" in n.content ? n.content.terminal.name : "an agent"))
      .join(", "),
  );
  const { previewFrame, commitFrame, bringToFront, pressNode, removeNode } = useCanvas.getState();
  const addWarning = useWorkspaces((s) => s.addWarning);
  const url = state?.url ?? saved.url;
  const title = state?.title || saved.title || (url === "about:blank" ? "New page" : url);
  const [address, setAddress] = useState(url === "about:blank" ? "" : url);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!editing) setAddress(url === "about:blank" ? "" : url);
  }, [url, editing]);

  const act = (action: Parameters<typeof ipc.browserAction>[1]) =>
    ipc.browserAction(node.id, action).catch((e) => addWarning(isAppError(e) ? e.message : String(e)));

  async function go() {
    try {
      setError(null);
      const normalized = await ipc.browserNavigate(node.id, address);
      setAddress(normalized);
      setEditing(false);
    } catch (e) {
      setError(isAppError(e) ? e.message : String(e));
    }
  }

  const live = state?.live ?? false;
  const img = imageUrl(state?.image, state?.imageVersion ?? 0);

  return (
    <div
      className={`node browser-node${selected ? " selected" : ""}`}
      data-node-id={node.id}
      data-node-kind="browser"
      style={{ left: frame.x, top: frame.y, width: frame.width, height: frame.height, zIndex: node.z }}
      onPointerDown={(e) => {
        if (e.button !== 0 || (e.target as HTMLElement).closest("button, input, .link-handle, .resize-handle")) return;
        if (pressNode(node.id, "note")) e.preventDefault();
      }}
    >
      <div
        className="browser-chrome"
        style={{ height: BROWSER_CHROME_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || (e.target as HTMLElement).closest("button, input")) return;
          if (useCanvas.getState().linkingFrom) return;
          void bringToFront(node.id);
          const zoom = useCanvas.getState().camera.zoom;
          const moved = (dx: number, dy: number) => ({ ...frame, x: frame.x + dx / zoom, y: frame.y + dy / zoom });
          startDrag(
            e,
            (dx, dy) => previewFrame(node.id, moved(dx, dy)),
            (dx, dy) => {
              if (dx !== 0 || dy !== 0) void commitFrame(node.id, moved(dx, dy));
            },
          );
        }}
      >
        <button type="button" className="icon-button" aria-label="Back" onClick={() => void act("back")}>
          <ArrowLeft size={14} />
        </button>
        <button type="button" className="icon-button" aria-label="Forward" onClick={() => void act("forward")}>
          <ArrowRight size={14} />
        </button>
        <button
          type="button"
          className={`icon-button${state?.loading ? " spinning" : ""}`}
          aria-label={state?.loading ? "Stop" : "Reload"}
          onClick={() => void act(state?.loading ? "stop" : "reload")}
        >
          {state?.loading ? <X size={14} /> : <RotateCw size={14} />}
        </button>
        <input
          className="browser-address"
          aria-label="Address"
          value={address}
          placeholder="Type an address, e.g. localhost:3000"
          spellCheck={false}
          autoFocus={url === "about:blank"}
          onFocus={(e) => {
            setEditing(true);
            e.currentTarget.select();
          }}
          onBlur={() => setEditing(false)}
          onChange={(e) => setAddress(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") void go();
            if (e.key === "Escape") e.currentTarget.blur();
          }}
        />
        {controlledBy && <span className="asked-by">controlled by {controlledBy}</span>}
        <LinkCount nodeId={node.id} />
        <button type="button" className="icon-button" aria-label="Open in default browser" onClick={() => void act("external")}>
          <ExternalLink size={14} />
        </button>
        <button type="button" className="icon-button" aria-label="Inspect" onClick={() => void act("inspect")}>
          <Code2 size={14} />
        </button>
        <button type="button" className="icon-button" aria-label="Close browser" onClick={() => void removeNode(node.id, true)}>
          <X size={14} />
        </button>
      </div>
      {error && (
        <p role="alert" className="browser-error">
          {error}
        </p>
      )}
      <div className="browser-body">
        {!live && (
          <div className="browser-card">
            {img ? <img src={img} alt={`Last view of ${title}`} draggable={false} /> : <Globe size={40} aria-hidden />}
            <div className="browser-card-caption">
              <strong>{title}</strong>
              <span>{url}</span>
            </div>
          </div>
        )}
      </div>
      {selected && <LinkHandle nodeId={node.id} />}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={320} minHeight={240} />
    </div>
  );
}
