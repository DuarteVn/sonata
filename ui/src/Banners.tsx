import { X } from "lucide-react";
import { useWorkspaces } from "./state/workspaces";

// Startup and persistence warnings (crash guard, invalid settings, failed saves).
export function Banners() {
  const warnings = useWorkspaces((s) => s.warnings);
  const dismiss = useWorkspaces((s) => s.dismissWarning);
  if (warnings.length === 0) return null;
  return (
    <div className="banners">
      {warnings.map((w, i) => (
        <div key={`${i}-${w}`} role="status" aria-label="Warning" className="banner">
          <span>{w}</span>
          <button type="button" className="icon-button" aria-label="Dismiss" onClick={() => dismiss(i)}>
            <X size={14} />
          </button>
        </div>
      ))}
    </div>
  );
}
