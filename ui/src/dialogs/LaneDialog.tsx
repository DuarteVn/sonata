import { useEffect, useState } from "react";
import { ipc } from "../ipc/commands";
import { isAppError, type LaneAction, type LanePreview } from "../ipc/types";
import { useCanvas, type LaneDialogState } from "../state/canvas";
import { Modal } from "./Modal";

type Props = { state: LaneDialogState; onClose: () => void };

const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
const message = (err: unknown) => (isAppError(err) ? err.message : String(err));

// Confirms a land or a lane removal (feature 008). Every land and removal goes through here,
// whether the user chose it in the menu or an agent asked for it with `sonata lane`.
export function LaneDialog({ state, onClose }: Props) {
  const { nodeId, request, deleteNode } = state;
  const [action, setAction] = useState<LaneAction>(state.action);
  const [preview, setPreview] = useState<LanePreview | null>(request?.preview ?? null);
  const [discard, setDiscard] = useState(false);
  const [deleteBranch, setDeleteBranch] = useState(request?.deleteBranch ?? false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (request && action === state.action) return;
    setPreview(null);
    ipc
      .lanePreview(nodeId, action)
      .then(setPreview)
      .catch((err) => setError(message(err)));
  }, [nodeId, action, request, state.action]);

  async function confirm() {
    setBusy(true);
    setError(null);
    try {
      if (action === "land") await ipc.laneLand(nodeId, true);
      else if (deleteNode)
        await useCanvas.getState().removeNode(nodeId, true, { discardChanges: discard, deleteBranch });
      else await ipc.laneRemove(nodeId, discard, deleteBranch, true);
      onClose();
    } catch (err) {
      setError(message(err));
    } finally {
      setBusy(false);
    }
  }

  const blocked = !preview || preview.blockers.length > 0;
  const nothing = preview !== null && preview.blockers.length === 0 && preview.commits === 0;
  const dirty = (preview?.changedTotal ?? 0) > 0;
  const title = action === "land" ? "Land lane" : deleteNode ? "Delete terminal and lane" : "Remove lane";

  return (
    <Modal title={title} onClose={onClose}>
      {preview && (
        <div className="dialog-body lane-dialog">
          {action === "land" ? (
            <p>
              Merge <code>{preview.branch}</code> into {preview.baseBranch}: {plural(preview.commits, "commit")}. The lane
              folder is removed and the terminal stops; the branch is kept.
            </p>
          ) : (
            <p>
              Delete the folder of <code>{preview.branch}</code>.{" "}
              {deleteBranch ? "The branch is deleted too." : "The branch is kept, so no commit is lost."}{" "}
              {deleteNode ? "The terminal is closed." : "The terminal restarts in the workspace folder."}
            </p>
          )}
          {preview.blockers.length > 0 && (
            <ul className="lane-blockers" aria-label="Why this cannot run">
              {preview.blockers.map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
          )}
          {action === "land" && nothing && (
            <p>
              Nothing to land: {preview.baseBranch} already has every commit of this lane.{" "}
              <button type="button" className="link-button" onClick={() => setAction("remove")}>
                Remove lane…
              </button>
            </p>
          )}
          {action === "remove" && dirty && (
            <>
              <ul className="lane-files" aria-label="Uncommitted files">
                {preview.changedFiles.map((f) => (
                  <li key={f}>{f}</li>
                ))}
                {preview.changedTotal > preview.changedFiles.length && (
                  <li>and {preview.changedTotal - preview.changedFiles.length} more</li>
                )}
              </ul>
              <label className="check">
                <input type="checkbox" checked={discard} onChange={(e) => setDiscard(e.target.checked)} />
                <span>Discard these {plural(preview.changedTotal, "uncommitted file")}</span>
              </label>
            </>
          )}
          {action === "remove" && (
            <label className="check">
              <input type="checkbox" checked={deleteBranch} onChange={(e) => setDeleteBranch(e.target.checked)} />
              <span>
                Also delete the branch {preview.branch}
                {deleteBranch && preview.unmergedCommits > 0 && (
                  <small className="warning">
                    {plural(preview.unmergedCommits, "commit")} exist only on {preview.branch} and will be lost.
                  </small>
                )}
              </span>
            </label>
          )}
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
        {action === "land" ? (
          <button type="button" className="primary" disabled={blocked || nothing || busy} onClick={() => void confirm()}>
            Land
          </button>
        ) : (
          <button
            type="button"
            className="danger"
            disabled={!preview || (dirty && !discard) || busy}
            onClick={() => void confirm()}
          >
            {deleteNode ? "Delete terminal" : "Remove lane"}
          </button>
        )}
      </div>
    </Modal>
  );
}
