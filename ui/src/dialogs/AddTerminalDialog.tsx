import { useState, type FormEvent } from "react";
import { Modal } from "./Modal";

type Props = {
  onAdd: (opts: { name?: string; launchCommand?: string }) => Promise<void>;
  onClose: () => void;
};

export function AddTerminalDialog({ onAdd, onClose }: Props) {
  const [name, setName] = useState("");
  const [launchCommand, setLaunchCommand] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function submit(e: FormEvent) {
    e.preventDefault();
    try {
      await onAdd({ name: name || undefined, launchCommand: launchCommand || undefined });
      onClose();
    } catch (err) {
      setError((err as { message?: string })?.message ?? String(err));
    }
  }

  return (
    <Modal title="New terminal" onClose={onClose}>
      <form onSubmit={submit} className="form">
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Terminal" />
        </label>
        <label>
          Launch command
          <input
            value={launchCommand}
            onChange={(e) => setLaunchCommand(e.target.value)}
            placeholder="optional, e.g. claude"
            spellCheck={false}
          />
        </label>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary">
            Add
          </button>
        </div>
      </form>
    </Modal>
  );
}
