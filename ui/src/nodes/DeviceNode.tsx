import { Channel, convertFileSrc } from "@tauri-apps/api/core";
import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
import { Circle, ChevronLeft, Play, RefreshCw, Smartphone, Square, SquareStack, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startNodeDrag } from "../canvas/nodeDrag";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode, type DeviceContent, type Platform, type ToolReport } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { fitScreen, keyToInput, streamSize, toDevice, TouchTracker, wheelToDevice, type DeviceInput, type Size } from "./devicePointer";
import { LinkCount, LinkHandle } from "./LinkHandle";

export const DEVICE_SIZE = { width: 380, height: 820 };
export const DEVICE_HEADER_HEIGHT = 36;
export const DEVICE_NAV_HEIGHT = 40;
/** Node chrome above and below the screen area, in canvas units. */
export const DEVICE_CHROME = { top: DEVICE_HEADER_HEIGHT, bottom: DEVICE_NAV_HEIGHT };
const FALLBACK_SCREEN: Size = { width: 1080, height: 2400 };

const message = (e: unknown) => (isAppError(e) ? e.message : String(e));

// A phone or tablet emulator on the canvas (feature 007). Until a device is chosen the body
// shows the picker, which also explains what is missing (US2).
export function DeviceNode({ node }: { node: CanvasNode }) {
  const { frame } = node;
  const device: DeviceContent = "device" in node.content ? node.content.device : { platform: "android", deviceId: "" };
  const selected = useCanvas((s) => s.selection?.kind === "node" && s.selection.id === node.id);
  const grouped = useCanvas((s) => s.group.includes(node.id));
  const { bringToFront, pressNode, removeNode } = useCanvas.getState();
  const title = device.name || device.deviceId || "Device";
  const state = useCanvas((s) => s.deviceStates[node.id]);
  const kind = state?.state.kind ?? "off";
  const controlledBy = useCanvas((s) =>
    Object.values(s.asks)
      .filter((a) => a.to === node.id)
      .map((a) => s.nodes.find((n) => n.id === a.from))
      .map((n) => (n && "terminal" in n.content ? n.content.terminal.name : "an agent"))
      .join(", "),
  );
  const [error, setError] = useState<string | null>(null);
  const chosen = device.deviceId !== "";
  /** iOS simulators take no input without third-party tools (research R11). */
  const interactive = kind === "ready" && device.platform !== "ios";

  useEffect(() => {
    if (!chosen) return;
    ipc
      .deviceState(node.id)
      .then((s) => useCanvas.getState().setDeviceState(s))
      .catch(() => {});
  }, [node.id, chosen]);

  const run = (p: Promise<unknown>) => {
    setError(null);
    p.catch((e) => setError(message(e)));
  };
  // One input at a time, in order: a release must never overtake its press.
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  const send = (input: DeviceInput) => {
    queue.current = queue.current.then(() => ipc.deviceInput(node.id, input)).catch((e) => setError(message(e)));
  };

  return (
    <div
      className={`node device-node${selected || grouped ? " selected" : ""}`}
      data-node-id={node.id}
      data-node-kind="device"
      style={{ left: frame.x, top: frame.y, width: frame.width, height: frame.height, zIndex: node.z }}
      onPointerDown={(e) => {
        if (e.button !== 0 || (e.target as HTMLElement).closest("button, input, .link-handle, .resize-handle, .device-screen")) return;
        if (pressNode(node.id, "note", e.shiftKey)) e.preventDefault();
      }}
    >
      <div
        className="device-header"
        style={{ height: DEVICE_HEADER_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
          if (useCanvas.getState().linkingFrom || e.shiftKey) return;
          void bringToFront(node.id);
          startNodeDrag(e, node.id);
        }}
      >
        <Smartphone size={14} aria-hidden />
        <span className="device-title">{title}</span>
        {device.osVersion && <span className="device-sub">{device.osVersion}</span>}
        <span className="device-spacer" />
        {controlledBy && <span className="asked-by">controlled by {controlledBy}</span>}
        <LinkCount nodeId={node.id} />
        {chosen &&
          (kind === "booting" || kind === "ready" ? (
            <button type="button" className="icon-button" aria-label="Stop" title="Stop" onClick={() => run(ipc.deviceStop(node.id))}>
              <Square size={13} />
            </button>
          ) : (
            <button type="button" className="icon-button" aria-label="Boot" title="Boot" onClick={() => run(ipc.deviceBoot(node.id))}>
              <Play size={13} />
            </button>
          ))}
        <button type="button" className="icon-button" aria-label="Close device" onClick={() => void removeNode(node.id, true)}>
          <X size={14} />
        </button>
      </div>
      {(error || (kind === "error" && state?.state.message)) && (
        <p role="alert" className="device-error">
          {error ?? state?.state.message}
        </p>
      )}
      <div className="device-body">
        {!chosen ? (
          <DevicePicker nodeId={node.id} />
        ) : (
          <DeviceScreen nodeId={node.id} title={title} kind={kind} screen={state?.screen} onInput={interactive ? send : undefined} />
        )}
      </div>
      {chosen && device.platform === "ios" && kind === "ready" && (
        <p className="device-note">View only: Sonata cannot send taps or keys to iOS simulators without extra tools.</p>
      )}
      {chosen && state?.reduced && device.platform === "android" && kind === "ready" && (
        <p className="device-note">Started outside Sonata: reduced frame rate.</p>
      )}
      {chosen && (
        <div className="device-nav" style={{ height: DEVICE_NAV_HEIGHT }}>
          <button type="button" className="icon-button" aria-label="Back" title="Back" disabled={!interactive} onClick={() => send({ key: "back" })}>
            <ChevronLeft size={16} />
          </button>
          <button type="button" className="icon-button" aria-label="Home" title="Home" disabled={!interactive} onClick={() => send({ key: "home" })}>
            <Circle size={14} />
          </button>
          <button
            type="button"
            className="icon-button"
            aria-label="Recent apps"
            title="Recent apps"
            disabled={!interactive}
            onClick={() => send({ key: "recents" })}
          >
            <SquareStack size={14} />
          </button>
        </div>
      )}
      {selected && <LinkHandle nodeId={node.id} />}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={200} minHeight={300} />
    </div>
  );
}

const STATE_TEXT: Record<string, string> = {
  off: "Device is off",
  booting: "Booting…",
  stopped: "Device stopped",
  error: "Device could not start",
};

/**
 * The device screen: streamed live into a canvas while the node is live, else the last image.
 * Mouse and keyboard on the live screen become touches, wheel, text, and keys (FR-006).
 */
function DeviceScreen({
  nodeId,
  title,
  kind,
  screen,
  onInput = ignore,
}: {
  nodeId: string;
  title: string;
  kind: string;
  screen?: [number, number];
  /** Absent when the device takes no input (iOS simulators). */
  onInput?: (input: DeviceInput) => void;
}) {
  const live = useCanvas((s) => s.deviceLive.has(nodeId)) && kind === "ready";
  const device: Size = screen ? { width: screen[0], height: screen[1] } : FALLBACK_SCREEN;
  const bodyRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [body, setBody] = useState<Size>({ width: 0, height: 0 });
  const [card, setCard] = useState<{ path: string; version: number } | null>(null);
  const tracker = useRef(new TouchTracker());
  /** Set when a live stream just ended: its cleanup already fetched the card. */
  const justStopped = useRef(false);

  useLayoutEffect(() => {
    const el = bodyRef.current!;
    const update = () => setBody({ width: el.clientWidth, height: el.clientHeight });
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const rect = fitScreen(body.width > 0 ? body : { width: DEVICE_SIZE.width, height: DEVICE_SIZE.height - DEVICE_HEADER_HEIGHT - DEVICE_NAV_HEIGHT }, device);

  // Live: stream frames into the canvas, acknowledging each one. Not live: show the card.
  useEffect(() => {
    // Ending a stream (or asking with none open) writes and returns the card image.
    const refreshCard = () =>
      ipc
        .deviceUnstream(nodeId)
        .then((path) => path && setCard((c) => ({ path, version: (c?.version ?? 0) + 1 })))
        .catch(() => {});
    if (!live) {
      if (justStopped.current) justStopped.current = false;
      else void refreshCard();
      return;
    }
    const zoom = useCanvas.getState().camera.zoom;
    const size = streamSize({ width: rect.width * zoom, height: rect.height * zoom }, window.devicePixelRatio || 1, device);
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (buf) => {
      drawFrame(canvasRef.current, buf);
      void ipc.deviceFrameAck(nodeId).catch(() => {});
    };
    void ipc.deviceStream(nodeId, size.width, size.height, channel).catch(() => {});
    return () => {
      channel.onmessage = () => {};
      justStopped.current = true;
      void refreshCard();
    };
    // Restart the stream when the screen area changes size.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [live, nodeId, rect.width, rect.height, device.width, device.height]);

  // Wheel must reach the device, not pan the canvas: a native listener stops it first.
  useEffect(() => {
    const el = canvasRef.current;
    if (!el || !live) return;
    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const at = toDevice({ x: e.clientX, y: e.clientY }, el.getBoundingClientRect(), device);
      if (at) onInput(wheelToDevice(at, e.deltaX, e.deltaY));
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    return () => el.removeEventListener("wheel", onWheel);
  }, [live, device, onInput]);

  const at = (e: React.PointerEvent) => toDevice({ x: e.clientX, y: e.clientY }, e.currentTarget.getBoundingClientRect(), device);
  const img = card ? imageSrc(card.path, card.version) : undefined;
  const box = { left: rect.left, top: rect.top, width: rect.width, height: rect.height };

  return (
    <div ref={bodyRef} className="device-screen-area">
      {live ? (
        <canvas
          ref={canvasRef}
          className="device-screen"
          style={box}
          tabIndex={0}
          data-captures-keys="true"
          aria-label={`Screen of ${title}`}
          onPointerDown={(e) => {
            if (e.button !== 0) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            e.currentTarget.focus();
            const p = at(e);
            if (p) onInput(tracker.current.down(p));
          }}
          onPointerMove={(e) => {
            if (!tracker.current.pressed) return;
            const input = tracker.current.move(at(e));
            if (input) onInput(input);
          }}
          onPointerUp={(e) => {
            const input = tracker.current.up(at(e));
            if (input) onInput(input);
          }}
          onPointerCancel={() => {
            const input = tracker.current.up(null);
            if (input) onInput(input);
          }}
          onKeyDown={(e) => {
            const input = keyToInput(e);
            if (!input) return;
            e.preventDefault();
            e.stopPropagation();
            onInput(input);
          }}
        />
      ) : (
        <div className="device-card" style={box}>
          {img ? <img src={img} alt={`Last screen of ${title}`} draggable={false} /> : <Smartphone size={40} aria-hidden />}
          {kind !== "ready" && <span className="device-card-state">{STATE_TEXT[kind] ?? kind}</span>}
        </div>
      )}
    </div>
  );
}

function ignore() {}

function imageSrc(path: string, version: number) {
  try {
    return `${convertFileSrc(path)}?v=${version}`;
  } catch {
    return undefined;
  }
}

/** Draws one frame: a 12-byte header (width, height, seq; little-endian u32) then RGBA. */
function drawFrame(canvas: HTMLCanvasElement | null, buf: ArrayBuffer) {
  if (!canvas || buf.byteLength < 12) return;
  const view = new DataView(buf);
  const width = view.getUint32(0, true);
  const height = view.getUint32(4, true);
  if (buf.byteLength < 12 + width * height * 4) return;
  if (canvas.width !== width) canvas.width = width;
  if (canvas.height !== height) canvas.height = height;
  const ctx = canvas.getContext("2d");
  ctx?.putImageData(new ImageData(new Uint8ClampedArray(buf, 12, width * height * 4), width, height), 0, 0);
}

/** Lists installed virtual devices, or says what is missing and where to get it (FR-002, FR-010). */
function DevicePicker({ nodeId }: { nodeId: string }) {
  const [report, setReport] = useState<ToolReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  const check = useCallback(async () => {
    setChecking(true);
    try {
      setReport(await ipc.deviceTools());
      setError(null);
    } catch (e) {
      setError(message(e));
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    void check();
  }, [check]);

  async function chooseSdk() {
    const path = await openFolderPicker({ directory: true, title: "Choose the Android SDK folder" });
    if (typeof path !== "string") return;
    try {
      setReport(await ipc.deviceChooseSdk(path));
    } catch (e) {
      setError(message(e));
    }
  }

  async function pick(platform: Platform, deviceId: string, name: string, osVersion?: string) {
    try {
      const updated = await ipc.deviceSelect(nodeId, platform, deviceId, name, osVersion);
      useCanvas.getState().replaceNode(updated);
    } catch (e) {
      setError(message(e));
    }
  }

  const help = (topic: "android" | "avd" | "xcode") => void ipc.deviceOpenHelp(topic).catch((e) => setError(message(e)));
  const android = report?.android;

  return (
    <div className="device-picker">
      {error && (
        <p role="alert" className="device-error">
          {error}
        </p>
      )}
      <section aria-label="Android">
        <h3>Android</h3>
        {!report ? (
          <p className="device-dim">Looking for the Android SDK…</p>
        ) : !android?.sdk ? (
          <>
            <p className="device-missing">Android SDK not found</p>
            <p className="device-dim">Sonata looked in:</p>
            <ul className="device-paths">
              {android?.looked.map((p) => (
                <li key={p}>{p}</li>
              ))}
            </ul>
            <p className="device-dim">Install Android Studio, which includes the SDK and the emulator. Sonata never installs it for you.</p>
            <div className="device-actions">
              <button type="button" onClick={() => help("android")}>
                Open install instructions
              </button>
              <button type="button" onClick={() => void chooseSdk()}>
                Choose SDK folder…
              </button>
            </div>
          </>
        ) : android.avds.length === 0 ? (
          <>
            <p className="device-missing">No virtual devices yet</p>
            <p className="device-dim">Create one in Android Studio's Device Manager. Sonata looks in {android.avdHome}.</p>
            <div className="device-actions">
              <button type="button" onClick={() => help("avd")}>
                How to create one
              </button>
            </div>
          </>
        ) : (
          <ul className="device-list">
            {android.avds.map((a) => (
              <li key={a.name}>
                <button type="button" onClick={() => void pick("android", a.name, a.displayName, a.osVersion)}>
                  <strong>{a.displayName}</strong> <span>{a.osVersion ?? a.name}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </section>
      <section aria-label="iOS">
        <h3>iOS</h3>
        {report && !report.ios.developerDir ? (
          <>
            <p className="device-missing">Xcode not found</p>
            <p className="device-dim">iOS simulators need Xcode from Apple.</p>
            <div className="device-actions">
              <button type="button" onClick={() => help("xcode")}>
                Open Xcode page
              </button>
            </div>
          </>
        ) : report && report.ios.simulators.length === 0 ? (
          <p className="device-dim">Xcode found, but it has no simulators. Add one in Xcode's Devices and Simulators window.</p>
        ) : (
          report && (
            <ul className="device-list">
              {report.ios.simulators.map((sim) => (
                <li key={sim.udid}>
                  <button type="button" onClick={() => void pick("ios", sim.udid, sim.name, sim.osVersion)}>
                    <strong>{sim.name}</strong> <span>{sim.osVersion}</span>
                  </button>
                </li>
              ))}
            </ul>
          )
        )}
      </section>
      <button type="button" className="device-recheck" onClick={() => void check()} disabled={checking}>
        <RefreshCw size={12} aria-hidden /> Check again
      </button>
    </div>
  );
}
