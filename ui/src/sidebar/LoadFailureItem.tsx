import { AlertOctagon } from "lucide-react";
import { useId } from "react";
import { ipc } from "../ipc/commands";
import type { LoadFailure, LoadFailureReason } from "../ipc/types";
import { revealLabel } from "../platform";

export function describeReason(r: LoadFailureReason): string {
  switch (r.kind) {
    case "invalidJson":
      return `The file is not valid JSON (${r.detail}).`;
    case "wrongType":
      return `The file is not a workspace (type "${r.found}").`;
    case "newerVersion":
      return `Created by a newer version of Sonata (format ${r.found}; this version supports up to ${r.supported}).`;
    case "migrationFailed":
      return `Upgrading from format ${r.step} failed: ${r.detail}`;
    case "decodeFailed":
      return `The file could not be read: ${r.detail}`;
    case "idMismatch":
      return `The id inside the file (${r.found}) does not match its folder.`;
    case "unreadable":
      return `The file could not be opened: ${r.detail}`;
  }
}

// A workspace folder that failed to load (FR-025). Sonata never writes to it.
export function LoadFailureItem({ failure }: { failure: LoadFailure }) {
  const labelId = useId();
  return (
    <li className="ws-failure" aria-labelledby={labelId}>
      <AlertOctagon size={16} className="ws-failure-icon" aria-hidden />
      <div className="ws-failure-text">
        <span id={labelId} className="ws-failure-title">
          Workspace could not be loaded
        </span>
        <span className="ws-failure-reason">{describeReason(failure.reason)}</span>
        <span className="ws-failure-path" title={failure.path}>
          {failure.path}
        </span>
        <button type="button" className="link-button" onClick={() => void ipc.appRevealPath(failure.path)}>
          {revealLabel()}
        </button>
      </div>
    </li>
  );
}
