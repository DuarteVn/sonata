import { useState } from "react";
import { ipc } from "../ipc/commands";
import { useUpdate } from "../state/update";

type Props = { onInstall: () => void };

// Bottom of the sidebar: a newer release, its download, or why it cannot install (feature 005).
export function UpdateBanner({ onInstall }: Props) {
  const state = useUpdate((s) => s.state);
  const [showNotes, setShowNotes] = useState(false);
  const later = () => {
    ipc.updateDismiss().then(useUpdate.getState().set).catch(() => {});
  };

  let body: React.ReactNode;
  switch (state.kind) {
    case "idle":
    case "dismissed":
      return null;
    case "available":
      body = (
        <>
          <p className="update-title">Sonata {state.version} is available</p>
          {state.notes && (
            <button type="button" className="link-button" aria-expanded={showNotes} onClick={() => setShowNotes((v) => !v)}>
              What's new
            </button>
          )}
          {showNotes && <p className="update-notes">{state.notes}</p>}
          <div className="update-actions">
            <button type="button" onClick={later}>
              Later
            </button>
            <button type="button" className="primary" onClick={onInstall}>
              Install and restart
            </button>
          </div>
        </>
      );
      break;
    case "downloading": {
      const percent = state.total ? Math.floor((state.received / state.total) * 100) : null;
      body = (
        <p className="update-title">
          Downloading Sonata {state.version}…{percent !== null && ` ${percent}%`}
        </p>
      );
      break;
    }
    case "readyToInstall":
      body = (
        <>
          <p className="update-title">Sonata {state.version} is ready</p>
          <div className="update-actions">
            <button type="button" className="primary" onClick={onInstall}>
              Restart to update
            </button>
          </div>
        </>
      );
      break;
    case "installing":
      body = <p className="update-title">Installing Sonata {state.version}…</p>;
      break;
    case "failed":
      body = (
        <>
          <p className="update-title">Sonata {state.version} could not be installed</p>
          <p className="update-notes">{state.reason}</p>
          <div className="update-actions">
            <button type="button" onClick={onInstall}>
              Try again
            </button>
          </div>
        </>
      );
      break;
    case "blocked":
      body = (
        <>
          <p className="update-title">Sonata {state.version} is available</p>
          <p className="update-notes">{state.reason}</p>
        </>
      );
      break;
  }
  return (
    <div className="update-banner" role="status" aria-label="Update">
      {body}
    </div>
  );
}
