import { useEffect, useState } from "react";
import { ipc } from "../ipc/commands";
import { isAppError, type LanePreview } from "../ipc/types";
import type { LaneDialogState } from "../state/canvas";
import { Modal } from "./Modal";

type Props = { state: LaneDialogState; onClose: () => void };

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;

// Confirms a land or a lane removal (feature 008). Every land and removal goes through here,
// whether the user chose it in the menu or an agent asked for it with `sonata lane`.
export function LaneDialog({ state, onClose }: Props) {
  const { nodeId, action } = state;
  const [preview, setPreview] = useState<LanePreview | null>(state.request?.preview ?? null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (state.request) return;
    ipc
      .lanePreview(nodeId, action)
      .then(setPreview)
      .catch((err) => setError(isAppError(err) ? err.message : String(err)));
  }, [nodeId, action, state.request]);

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      await ipc.laneLand(nodeId, true);
      onClose();
    } catch (err) {
      setError(isAppError(err) ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  const blocked = !preview || preview.blockers.length > 0;
  const nothing = preview !== null && preview.blockers.length === 0 && preview.commits === 0;

  return (
    <Modal title="Land lane" onClose={onClose}>
      {preview && (
        <div className="dialog-body lane-dialog">
          <p>
            Merge <code>{preview.branch}</code> into {preview.baseBranch}: {plural(preview.commits, "commit")}. The lane
            folder is removed and the terminal stops; the branch is kept.
          </p>
          {preview.blockers.length > 0 && (
            <ul className="lane-blockers" aria-label="Why the land cannot run">
              {preview.blockers.map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
          )}
          {nothing && <p>Nothing to land: {preview.baseBranch} already has every commit of this lane.</p>}
        </div>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      <div className="actions">
        <button type="button" onClick={onClose}>
          Cancel
        </button>
        <button type="button" className="primary" disabled={blocked || nothing || busy} onClick={() => void confirm()}>
          Land
        </button>
      </div>
    </Modal>
  );
}
