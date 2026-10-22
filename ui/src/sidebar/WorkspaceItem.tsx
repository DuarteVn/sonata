import type { WorkspaceSummary } from "../ipc/types";
import { WorkspaceIcon } from "./WorkspaceIcon";

type Props = { workspace: WorkspaceSummary; active: boolean; onOpen: () => void };

export function WorkspaceItem({ workspace, active, onOpen }: Props) {
  return (
    <li className={`ws-item${active ? " active" : ""}`}>
      <button type="button" className="ws-open" onClick={onOpen} title={workspace.workingDirectory}>
        <WorkspaceIcon icon={workspace.icon} color={workspace.color} size={20} />
        <span className="ws-name">{workspace.name}</span>
      </button>
    </li>
  );
}
