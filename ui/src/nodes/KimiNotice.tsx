import { useEffect } from "react";
import { ipc } from "../ipc/commands";
import { useKimi } from "../state/kimi";
import { useWorkspaces } from "../state/workspaces";

/** A terminal that starts Kimi Code (feature 010). */
export function isKimiCommand(command: string | undefined): boolean {
  return command?.trim().split(/\s+/)[0] === "kimi";
}

// One line in the title bar of Kimi terminal nodes while Sonata's hooks are off or out of date
// (FR-008). The live terminal covers the node body, so the title bar is the visible place.
export function KimiNotice() {
  const integration = useKimi((s) => s.integration);
  const saved = useWorkspaces((s) => s.preferences.kimiNoticeDismissed);
  const dismissed = useKimi((s) => s.dismissed) ?? saved;

  useEffect(() => {
    void useKimi.getState().load();
  }, []);

  const status = integration?.status;
  if (status !== "off" && status !== "outdated") return null;
  if (dismissed === status) return null;
  const state = status === "off" ? "off" : "out of date";
  const text = `Kimi: busy/done tracking and conversation restore are ${state}.`;

  return (
    <div className="kimi-notice" role="status">
      <span title={text}>{text}</span>
      <button type="button" className="link-button" onClick={() => void ipc.kimiOpenSettings().catch(() => {})}>
        Set up
      </button>
      <button type="button" className="link-button" onClick={() => void useKimi.getState().dismiss(status).catch(() => {})}>
        Dismiss
      </button>
    </div>
  );
}
