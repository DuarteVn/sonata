import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
import { RefreshCw, Smartphone, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startNodeDrag } from "../canvas/nodeDrag";
import { ipc } from "../ipc/commands";
import { isAppError, type CanvasNode, type DeviceContent, type ToolReport } from "../ipc/types";
import { useCanvas } from "../state/canvas";
import { LinkCount, LinkHandle } from "./LinkHandle";

export const DEVICE_SIZE = { width: 380, height: 820 };
export const DEVICE_HEADER_HEIGHT = 36;

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
        <LinkCount nodeId={node.id} />
        <button type="button" className="icon-button" aria-label="Close device" onClick={() => void removeNode(node.id, true)}>
          <X size={14} />
        </button>
      </div>
      <div className="device-body">{device.deviceId ? null : <DevicePicker nodeId={node.id} />}</div>
      {selected && <LinkHandle nodeId={node.id} />}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={200} minHeight={300} />
    </div>
  );
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

  async function pick(deviceId: string, name: string, osVersion?: string) {
    try {
      const updated = await ipc.deviceSelect(nodeId, "android", deviceId, name, osVersion);
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
                <button type="button" onClick={() => void pick(a.name, a.displayName, a.osVersion)}>
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
        ) : (
          report && <p className="device-dim">Xcode found.</p>
        )}
      </section>
      <button type="button" className="device-recheck" onClick={() => void check()} disabled={checking}>
        <RefreshCw size={12} aria-hidden /> Check again
      </button>
    </div>
  );
}
