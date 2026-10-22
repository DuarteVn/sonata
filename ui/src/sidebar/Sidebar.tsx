import { Plus } from "lucide-react";
import { useWorkspaces } from "../state/workspaces";
import { WorkspaceItem } from "./WorkspaceItem";

export function Sidebar({ onCreate }: { onCreate: () => void }) {
  const workspaces = useWorkspaces((s) => s.workspaces);
  const activeId = useWorkspaces((s) => s.activeId);
  const open = useWorkspaces((s) => s.open);

  return (
    <nav className="sidebar" aria-label="Workspaces">
      <header className="sidebar-header">
        <span>Workspaces</span>
        <button type="button" className="icon-button" aria-label="New workspace" onClick={onCreate}>
          <Plus size={16} />
        </button>
      </header>
      <ul className="ws-list">
        {workspaces.map((w) => (
          <WorkspaceItem key={w.id} workspace={w} active={w.id === activeId} onOpen={() => open(w.id)} />
        ))}
      </ul>
    </nav>
  );
}
