import { PanelLeftOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { Banners } from "./Banners";
import { Canvas } from "./canvas/Canvas";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { QuitDialog } from "./dialogs/QuitDialog";
import { ipc } from "./ipc/commands";
import { onEvent } from "./ipc/events";
import type { BusyWorkspace } from "./ipc/types";
import { CreateWorkspaceDialog } from "./sidebar/CreateWorkspaceDialog";
import { clampSidebarWidth, Sidebar, SIDEBAR_DEFAULT_WIDTH } from "./sidebar/Sidebar";
import { useCanvas } from "./state/canvas";
import { updatePending, useUpdate } from "./state/update";
import { useWorkspaces } from "./state/workspaces";
import { UsageConsentDialog } from "./dialogs/UsageConsentDialog";
import { useUsage } from "./state/usage";
import { UsageButton } from "./usage/UsageButton";
import { resetText } from "./usage/meter";
import { shortcut } from "./platform";

const SIDEBAR_HIDDEN_KEY = "sonata.sidebarHidden";
const SIDEBAR_WIDTH_KEY = "sonata.sidebarWidth";

function readSidebarHidden(): boolean {
  try {
    return localStorage.getItem(SIDEBAR_HIDDEN_KEY) === "1";
  } catch {
    return false;
  }
}

function readSidebarWidth(): number {
  try {
    const saved = Number(localStorage.getItem(SIDEBAR_WIDTH_KEY));
    return saved > 0 ? clampSidebarWidth(saved) : SIDEBAR_DEFAULT_WIDTH;
  } catch {
    return SIDEBAR_DEFAULT_WIDTH;
  }
}

let lastNotify: { nodeId: string; at: number } | null = null;

export function App() {
  const ready = useWorkspaces((s) => s.ready);
  const workspaces = useWorkspaces((s) => s.workspaces);
  const active = useWorkspaces((s) => s.active);
  const [creating, setCreating] = useState(false);
  const [quitBusy, setQuitBusy] = useState<BusyWorkspace[] | null>(null);
  const [updateBusy, setUpdateBusy] = useState<BusyWorkspace[] | null>(null);
  const updateState = useUpdate((s) => s.state);
  const [sidebarHidden, setSidebarHidden] = useState(readSidebarHidden);
  const [sidebarWidth, setSidebarWidth] = useState(readSidebarWidth);
  const consent = useUsage((s) => s.consents[0]);

  useEffect(() => {
    try {
      localStorage.setItem(SIDEBAR_HIDDEN_KEY, sidebarHidden ? "1" : "0");
      localStorage.setItem(SIDEBAR_WIDTH_KEY, String(sidebarWidth));
    } catch {
      // Per-device convenience only; ignore storage errors.
    }
  }, [sidebarHidden, sidebarWidth]);

  useEffect(() => {
    const { init, setSummaries, addWarning } = useWorkspaces.getState();
    init().catch((err) => addWarning(String(err?.message ?? err)));
    void useUpdate.getState().load();
    void useUsage.getState().load();
    const unlisten = [
      onEvent("workspaces://changed", setSummaries),
      onEvent("persistence://warning", (p) => addWarning(p.message)),
      onEvent("terminal://state", (e) => useCanvas.getState().setTerminalState(e)),
      onEvent("messaging://ask", (e) => useCanvas.getState().setAsk(e)),
      onEvent("browser://state", (e) => useCanvas.getState().setBrowserState(e)),
      onEvent("device://state", (e) => useCanvas.getState().setDeviceState(e)),
      onEvent("workspace://nodes-changed", (e) => useCanvas.getState().applyNodesChanged(e.workspaceId, e.nodes, e.links)),
      // `sonata notify` (003 FR-026): system notification; focusing Sonata soon after selects the terminal.
      onEvent("messaging://notify", async (e) => {
        lastNotify = { nodeId: e.nodeId, at: Date.now() };
        try {
          const granted = (await isPermissionGranted()) || (await requestPermission()) === "granted";
          if (granted) sendNotification({ title: e.title, body: e.message });
        } catch {
          addWarning(`${e.title}: ${e.message}`);
        }
      }),
      onEvent("terminal://preview", (e) => useCanvas.getState().setPreview(e.nodeId, e.lines)),
      onEvent("note://changed", (e) => useCanvas.getState().setNoteText(e.nodeId, e.text, e.sync)),
      onEvent("note://conflict", (e) => useCanvas.getState().setNoteConflict(e.nodeId, e.conflictPath)),
      // Cmd+W: close the selected item (or the focused terminal), not the window.
      onEvent("app://close-selected", () => {
        useCanvas
          .getState()
          .deleteSelection()
          .catch((err) => addWarning(String(err?.message ?? err)));
      }),
      // View → Toggle Sidebar (Ctrl+Cmd+S).
      onEvent("app://toggle-sidebar", () => setSidebarHidden((h) => !h)),
      onEvent("update://state", (state) => useUpdate.getState().set(state)),
      onEvent("usage://state", (state) => useUsage.getState().set(state)),
      onEvent("usage://consent", (request) => useUsage.getState().askConsent(request)),
      // 80% and 100% of a plan limit, once per level per window period (006 FR-010).
      onEvent("usage://warning", async (w) => {
        const title =
          w.level === 100
            ? `${w.providerName}: ${w.windowLabel} limit reached`
            : `${w.providerName}: ${w.windowLabel} limit at ${w.level}%`;
        const reset = resetText(w.resetsAt, new Date());
        const body = reset ? `The limit ${reset}.` : "Check the Usage panel for details.";
        try {
          const granted = (await isPermissionGranted()) || (await requestPermission()) === "granted";
          if (granted) sendNotification({ title, body });
        } catch {
          addWarning(`${title}. ${body}`);
        }
      }),
      onEvent("lane://status", (status) => useCanvas.getState().setLaneStatus(status)),
      onEvent("lane://confirm", (e) => useCanvas.getState().showLaneRequest(e)),
      onEvent("lane://confirm-closed", (e) => useCanvas.getState().closeLaneRequest(e.requestId)),
      // Sonata → Check for Updates…: always report the result (FR-013).
      onEvent("app://check-updates", async () => {
        try {
          const state = await ipc.updateCheck(true);
          useUpdate.getState().set(state);
          if (!updatePending(state)) addWarning("Sonata is up to date.");
        } catch (err) {
          addWarning(`Could not check for updates: ${String((err as Error)?.message ?? err)}`);
        }
      }),
      // Cmd+Q or closing the window: quit at once unless programs are running (FR-018a).
      onEvent("app://quit-requested", async () => {
        const busy = await ipc.appQuitCheck();
        if (busy.length === 0) await ipc.appQuit(false);
        else setQuitBusy(busy);
      }),
    ];
    const onFocus = () => {
      if (lastNotify && Date.now() - lastNotify.at < 60_000) {
        const { nodeId } = lastNotify;
        lastNotify = null;
        const canvas = useCanvas.getState();
        if (canvas.nodes.some((n) => n.id === nodeId)) canvas.select({ kind: "node", id: nodeId });
      }
    };
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      unlisten.forEach((p) => p.then((off) => off()));
    };
  }, []);

  useEffect(() => useCanvas.getState().load(active), [active]);

  // "Install and restart": download and verify first (nothing stops yet), then the same
  // running-programs check as quitting, then install and restart (FR-010, FR-012).
  const installUpdate = async () => {
    const { set } = useUpdate.getState();
    try {
      let state = useUpdate.getState().state;
      if (state.kind !== "readyToInstall") {
        state = await ipc.updateDownload();
        set(state);
      }
      if (state.kind !== "readyToInstall") return;
      const busy = await ipc.appQuitCheck();
      if (busy.length === 0) await ipc.updateInstall();
      else setUpdateBusy(busy);
    } catch (err) {
      useWorkspaces.getState().addWarning(`Update failed: ${String((err as Error)?.message ?? err)}`);
    }
  };

  return (
    <div className="app">
      {!sidebarHidden && (
        <Sidebar
          onCreate={() => setCreating(true)}
          onHide={() => setSidebarHidden(true)}
          width={sidebarWidth}
          onResize={setSidebarWidth}
          onInstallUpdate={() => void installUpdate()}
        />
      )}
      <main className="main">
        {sidebarHidden && (
          <button
            type="button"
            className="icon-button sidebar-show"
            aria-label={updatePending(updateState) ? "Show sidebar (update available)" : "Show sidebar"}
            title={`Show sidebar (${shortcut("toggleSidebar")})`}
            onClick={() => setSidebarHidden(false)}
          >
            <PanelLeftOpen size={16} />
            {updatePending(updateState) && <span className="update-dot" aria-hidden="true" />}
          </button>
        )}
        <Banners />
        <UsageButton />
        {ready && workspaces.length === 0 && (
          <div className="empty-state">
            <p>No workspaces yet. A workspace is a canvas for one project folder.</p>
            <button type="button" className="primary" onClick={() => setCreating(true)}>
              Create workspace
            </button>
          </div>
        )}
        {active && <Canvas key={active.id} />}
      </main>
      {creating && <CreateWorkspaceDialog onClose={() => setCreating(false)} />}
      {consent && (
        <UsageConsentDialog
          key={consent.providerId}
          request={consent}
          onDone={() => useUsage.getState().dropConsent(consent.providerId)}
        />
      )}
      {quitBusy && (
        <QuitDialog
          busy={quitBusy}
          onCancel={() => setQuitBusy(null)}
          onQuit={() => {
            setQuitBusy(null);
            void ipc.appQuit(true);
          }}
        />
      )}
      {updateBusy && (
        <QuitDialog
          forUpdate
          busy={updateBusy}
          onCancel={() => setUpdateBusy(null)}
          onQuit={() => {
            setUpdateBusy(null);
            ipc.updateInstall().catch((err) =>
              useWorkspaces.getState().addWarning(`Update failed: ${String((err as Error)?.message ?? err)}`),
            );
          }}
        />
      )}
    </div>
  );
}
