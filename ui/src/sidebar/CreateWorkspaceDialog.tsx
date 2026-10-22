import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
import { useState, type FormEvent } from "react";
import { Modal } from "../dialogs/Modal";
import { isAppError } from "../ipc/types";
import { useWorkspaces } from "../state/workspaces";

export function CreateWorkspaceDialog({ onClose }: { onClose: () => void }) {
  const create = useWorkspaces((s) => s.create);
  const open = useWorkspaces((s) => s.open);
  const [name, setName] = useState("");
  const [folder, setFolder] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function chooseFolder() {
    const picked = await openFolderPicker({ directory: true, multiple: false });
    if (typeof picked === "string") setFolder(picked);
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const created = await create(name, folder);
      onClose();
      await open(created.id);
    } catch (err) {
      setError(isAppError(err) ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Modal title="New workspace" onClose={onClose}>
      <form onSubmit={submit} className="form">
        <label>
          Name
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="My project" />
        </label>
        <label htmlFor="ws-folder">Folder</label>
        <div className="row">
          <input
            id="ws-folder"
            value={folder}
            onChange={(e) => setFolder(e.target.value)}
            placeholder="/Users/you/Developer/project"
          />
          <button type="button" onClick={chooseFolder}>
            Choose…
          </button>
        </div>
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="actions">
          <button type="button" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="primary" disabled={busy}>
            Create
          </button>
        </div>
      </form>
    </Modal>
  );
}
