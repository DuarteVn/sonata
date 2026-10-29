import { Copy } from "lucide-react";
import { useEffect, useState } from "react";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { ipc } from "../ipc/commands";
import { onEvent } from "../ipc/events";
import { isAppError, type KimiIntegration, type KimiStatus } from "../ipc/types";

const STATUS_TEXT: Record<KimiStatus, string> = {
  on: "On: Sonata's hooks are in Kimi's config.",
  outdated: "Out of date: Kimi's config has an older version of Sonata's hooks.",
  off: "Off: Kimi terminals use a quiet screen to tell when Kimi is done, and do not restore conversations.",
  unavailable: "Unavailable",
};

// What each hook in the block does (FR-009).
const HOOKS: [string, string][] = [
  ["SessionStart", "Tells Sonata which conversation the terminal resumed or started."],
  ["UserPromptSubmit", "Marks the terminal busy and records the conversation at each message."],
  ["Stop", "Tells Sonata that Kimi finished its turn, so asks return at once."],
  ["StopFailure", "Ends the turn when Kimi stops with an error."],
  ["Interrupt", "Ends the turn when you cancel Kimi."],
];

type Pending = "apply" | "remove" | null;

// Settings → Agents → Kimi Code: status, the exact lines, Apply and Remove (feature 010).
export function AgentSettings() {
  const [kimi, setKimi] = useState<KimiIntegration | null>(null);
  const [confirm, setConfirm] = useState<Pending>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    ipc.kimiIntegration().then(setKimi).catch((err) => setError(message(err)));
    void ipc.kimiWatch(true).catch(() => {});
    const off = onEvent("kimi://integration", setKimi);
    return () => {
      void ipc.kimiWatch(false).catch(() => {});
      void off.then((f) => f());
    };
  }, []);

  if (!kimi) return <p className="settings-dim">{error ?? "Loading…"}</p>;

  async function run(action: "apply" | "remove") {
    setConfirm(null);
    setBusy(true);
    setError(null);
    setDone(null);
    try {
      const next = await (action === "apply" ? ipc.kimiIntegrationApply() : ipc.kimiIntegrationRemove());
      setKimi(next);
      const backup = next.lastBackup ? `Backup saved to ${next.lastBackup}. ` : "";
      setDone(`${backup}Restart running Kimi terminals to use the change.`);
    } catch (err) {
      setError(message(err));
    } finally {
      setBusy(false);
    }
  }

  const copy = () =>
    navigator.clipboard
      .writeText(kimi.block)
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      })
      .catch((err) => setError(message(err)));

  const canRemove = kimi.status === "on" || kimi.status === "outdated";
  const canApply = kimi.status !== "on" && kimi.status !== "unavailable";

  return (
    <div className="settings-section">
      <h1>Agents</h1>
      <h2>Kimi Code</h2>
      <p className="settings-dim">
        Kimi reads hooks only from its own config file. With Sonata's hooks there, Sonata knows when Kimi is busy or
        done and restores Kimi conversations after a restart. The hooks do nothing when Kimi runs outside Sonata.
      </p>
      <p>{STATUS_TEXT[kimi.status]}</p>
      <div className="settings-folder">
        <code>{kimi.configPath}</code>
      </div>
      {kimi.message && <p className="settings-error">{kimi.message}</p>}
      {!kimi.validated && kimi.status !== "unavailable" && (
        <p className="settings-dim">
          Sonata cannot run `kimi doctor` to check the file after a change: `kimi` was not found on your PATH.
        </p>
      )}

      <div className="settings-subhead">
        <h2>Lines Sonata adds</h2>
        <button type="button" onClick={() => void copy()}>
          <Copy size={13} /> {copied ? "Copied" : "Copy lines"}
        </button>
      </div>
      <pre className="settings-code" aria-label="Lines Sonata adds">
        {kimi.block}
      </pre>
      <table className="settings-table">
        <tbody>
          {HOOKS.map(([event, text]) => (
            <tr key={event}>
              <td>
                <code>{event}</code>
              </td>
              <td className="settings-dim">{text}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="settings-subhead">
        <p className="settings-dim">
          Remove takes out only these lines. Every change first saves a backup next to the file.
        </p>
        <span className="settings-actions">
          {canApply && (
            <button type="button" className="primary" disabled={busy || !kimi.canApply} onClick={() => setConfirm("apply")}>
              Apply
            </button>
          )}
          {canRemove && (
            <button type="button" disabled={busy} onClick={() => setConfirm("remove")}>
              Remove
            </button>
          )}
        </span>
      </div>
      {done && <p className="settings-dim">{done}</p>}
      {error && <p className="error">{error}</p>}

      {confirm === "apply" && (
        <ConfirmDialog
          title="Set up Kimi"
          body={`Sonata will back up and change ${kimi.configPath}: it adds only the lines shown. Kimi outside Sonata is not affected.`}
          confirmLabel="Apply"
          onConfirm={() => void run("apply")}
          onCancel={() => setConfirm(null)}
        />
      )}
      {confirm === "remove" && (
        <ConfirmDialog
          title="Remove Kimi setup"
          body={`Sonata will back up ${kimi.configPath} and remove only its own lines.`}
          confirmLabel="Remove"
          danger
          onConfirm={() => void run("remove")}
          onCancel={() => setConfirm(null)}
        />
      )}
    </div>
  );
}

function message(err: unknown): string {
  return isAppError(err) ? err.message : String((err as Error)?.message ?? err);
}
