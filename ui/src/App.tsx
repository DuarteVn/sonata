import { useEffect, useState } from "react";
import { Banners } from "./Banners";
import { Canvas } from "./canvas/Canvas";
import { QuitDialog } from "./dialogs/QuitDialog";
import { ipc } from "./ipc/commands";
import { onEvent } from "./ipc/events";
import type { BusyWorkspace } from "./ipc/types";
import { CreateWorkspaceDialog } from "./sidebar/CreateWorkspaceDialog";
import { Sidebar } from "./sidebar/Sidebar";
import { useCanvas } from "./state/canvas";
import { useWorkspaces } from "./state/workspaces";

export function App() {
  const ready = useWorkspaces((s) => s.ready);
  const workspaces = useWorkspaces((s) => s.workspaces);
  const active = useWorkspaces((s) => s.active);
  const [creating, setCreating] = useState(false);
  const [quitBusy, setQuitBusy] = useState<BusyWorkspace[] | null>(null);

  useEffect(() => {
    const { init, setSummaries, addWarning } = useWorkspaces.getState();
    init().catch((err) => addWarning(String(err?.message ?? err)));
    const unlisten = [
      onEvent("workspaces://changed", setSummaries),
      onEvent("persistence://warning", (p) => addWarning(p.message)),
      onEvent("terminal://state", (e) => useCanvas.getState().setTerminalState(e)),
      onEvent("note://changed", (e) => useCanvas.getState().setNoteText(e.nodeId, e.text, e.sync)),
      onEvent("note://conflict", (e) => useCanvas.getState().setNoteConflict(e.nodeId, e.conflictPath)),
      // Cmd+Q or closing the window: quit at once unless programs are running (FR-018a).
      onEvent("app://quit-requested", async () => {
        const busy = await ipc.appQuitCheck();
        if (busy.length === 0) await ipc.appQuit(false);
        else setQuitBusy(busy);
      }),
    ];
    return () => unlisten.forEach((p) => p.then((off) => off()));
  }, []);

  useEffect(() => useCanvas.getState().load(active), [active]);

  return (
    <div className="app">
      <Sidebar onCreate={() => setCreating(true)} />
      <main className="main">
        <Banners />
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
    </div>
  );
}
