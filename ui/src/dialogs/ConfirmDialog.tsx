import { Modal } from "./Modal";

type Props = {
  title: string;
  body: string;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

/** Shown when a restart would stop a running program (Restart and Start fresh). */
export const RESTART_CONFIRM = {
  title: "Restart terminal?",
  body: "A program is still running. Restarting stops it and starts a fresh shell with the terminal's command.",
  confirmLabel: "Restart",
};

export function ConfirmDialog({ title, body, confirmLabel, danger, onConfirm, onCancel }: Props) {
  return (
    <Modal title={title} onClose={onCancel}>
      <p className="dialog-body">{body}</p>
      <div className="actions">
        <button type="button" onClick={onCancel}>
          Cancel
        </button>
        <button type="button" className={danger ? "danger" : "primary"} onClick={onConfirm}>
          {confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
