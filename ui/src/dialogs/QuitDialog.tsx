import type { BusyWorkspace } from "../ipc/types";
import { Modal } from "./Modal";

type Props = {
  busy: BusyWorkspace[];
  onQuit: () => void;
  onCancel: () => void;
  /** Restarting for an update stops programs the same way quitting does (feature 005). */
  forUpdate?: boolean;
};

export function QuitDialog({ busy, onQuit, onCancel, forUpdate = false }: Props) {
  const total = busy.reduce((n, b) => n + b.count, 0);
  return (
    <Modal title={forUpdate ? "Restart to update?" : "Quit Sonata?"} onClose={onCancel}>
      <p className="dialog-body">
        {total === 1 ? "A program is" : `${total} programs are`} still running.{" "}
        {forUpdate ? "Restarting" : "Quitting"} stops {total === 1 ? "it" : "them"}.
      </p>
      <ul className="quit-list">
        {busy.map((b) => (
          <li key={b.workspaceId}>
            <strong>{b.workspaceName}</strong>: {b.count} {b.count === 1 ? "terminal" : "terminals"}
          </li>
        ))}
      </ul>
      <div className="actions">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className="danger" onClick={onQuit}>
          {forUpdate ? "Restart" : "Quit"}
        </button>
      </div>
    </Modal>
  );
}
