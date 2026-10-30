import { Copy, FolderOpen, RefreshCw } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { ipc } from "../ipc/commands";
import type { ProviderState, UsageState } from "../ipc/types";
import { useUsage } from "../state/usage";
import { timeAgo } from "../usage/meter";
import { Switch } from "./Switch";

type Props = {
  /** Fixed clock for tests. */
  now?: Date;
};

function apply(promise: Promise<UsageState | unknown>, onError: (message: string) => void) {
  promise
    .then((s) => {
      if (s && typeof s === "object" && "providers" in s) useUsage.getState().set(s as UsageState);
    })
    .catch((err) => onError(String((err as Error)?.message ?? err)));
}

/** One line that says where this agent stands (FR-017). */
export function providerStatus(p: ProviderState, now: Date): { text: string; error: boolean } {
  if (p.definitionError) return { text: p.definitionError, error: true };
  if (p.detection === "notFound") return { text: "Command not found", error: true };
  if (p.detection === "unknown") return { text: "Not checked yet", error: false };
  if (p.consent === "denied") return { text: "Not allowed", error: false };
  if (p.consent === "unasked") return { text: "Waiting for permission", error: false };
  if (p.refreshing) return { text: "Reading…", error: false };
  const r = p.reading;
  if (r?.status === "notLoggedIn") return { text: "Not logged in", error: true };
  if (r?.status === "noPlanLimits") return { text: "No plan limits", error: false };
  if (p.lastError) return { text: p.lastError, error: true };
  if (r) return { text: `Read ${timeAgo(r.readAt, now)}`, error: false };
  return { text: "No reading yet", error: false };
}

// Settings → Usage: master switch, warnings, one row per agent, custom providers folder.
export function UsageSettings({ now: fixedNow }: Props) {
  const state = useUsage((s) => s.state);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [now, setNow] = useState(() => fixedNow ?? new Date());

  useEffect(() => {
    if (fixedNow) return;
    const id = setInterval(() => setNow(new Date()), 30_000);
    return () => clearInterval(id);
  }, [fixedNow]);

  if (!state) return <p className="settings-dim">Loading…</p>;
  const clock = fixedNow ?? now;

  const showFolder = () =>
    ipc
      .usageProvidersFolder()
      .then(({ path }) => ipc.appRevealPath(path))
      .catch((err) => setError(String((err as Error)?.message ?? err)));
  const copyPath = () =>
    ipc
      .usageProvidersFolder()
      .then(({ path }) => navigator.clipboard.writeText(path))
      .then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1500);
      })
      .catch((err) => setError(String((err as Error)?.message ?? err)));

  return (
    <div className="settings-section">
      <h1>Usage</h1>
      <p className="settings-dim">
        Sonata reads each agent's plan limits through the agent's own command. It never reads or stores your logins.
      </p>

      <div className="settings-row">
        <div>
          <div>Read agent usage</div>
          <div className="settings-dim">When off, Sonata runs no usage command and contacts no provider.</div>
        </div>
        <Switch label="Read agent usage" checked={state.enabled} onChange={(v) => apply(ipc.usageSetEnabled(v), setError)} />
      </div>
      <div className="settings-row">
        <div>
          <div>Warn at 80% and 100%</div>
          <div className="settings-dim">A system notification once per level, each time a limit window starts over.</div>
        </div>
        <Switch
          label="Warn at 80% and 100%"
          checked={state.warnings}
          disabled={!state.enabled}
          onChange={(v) => apply(ipc.usageSetWarnings(v), setError)}
        />
      </div>

      <div className="settings-subhead">
        <h2>Agents</h2>
        <button type="button" disabled={!state.enabled} onClick={() => apply(ipc.usageRefresh("manual"), setError)}>
          Refresh all
        </button>
      </div>
      <table className="settings-table">
        <tbody>
          {state.providers.map((p) => (
            <ProviderRow key={p.id} provider={p} now={clock} disabled={!state.enabled} onError={setError} />
          ))}
        </tbody>
      </table>

      <h2>Custom agents</h2>
      <p className="settings-dim">
        Add a definition file to this folder to read another agent. The README.md in the folder explains the format.
      </p>
      <div className="settings-folder">
        <code>{state.providersFolder}</code>
        <button type="button" onClick={() => void showFolder()}>
          <FolderOpen size={13} /> Show folder
        </button>
        <button type="button" onClick={() => void copyPath()}>
          <Copy size={13} /> {copied ? "Copied" : "Copy path"}
        </button>
      </div>
      {error && <p className="error">{error}</p>}
    </div>
  );
}

function ProviderRow({
  provider: p,
  now,
  disabled,
  onError,
}: {
  provider: ProviderState;
  now: Date;
  disabled: boolean;
  onError: (message: string) => void;
}) {
  const status = providerStatus(p, now);
  const broken = p.definitionError !== null;
  // The native picker reports every step of a drag; save once it settles.
  const [color, setColor] = useState(p.color);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => setColor(p.color), [p.color]);
  useEffect(() => () => clearTimeout(timer.current), []);
  const pickColor = (value: string) => {
    setColor(value);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => apply(ipc.usageSetProviderColor(p.id, value), onError), 300);
  };
  return (
    <tr aria-label={p.name}>
      <td>
        <input
          type="color"
          className="settings-color"
          aria-label={`${p.name} color`}
          value={color}
          disabled={broken}
          onChange={(e) => pickColor(e.target.value)}
        />
      </td>
      <td className="settings-agent">
        <div>
          <strong>{p.name}</strong> <span className="settings-tag">{p.kind === "custom" ? "Custom" : "Built-in"}</span>
        </div>
        <div className="settings-dim settings-path">{p.commandPath ?? (broken ? "" : "Command not found")}</div>
        <div className={status.error ? "settings-error" : "settings-dim"}>{status.text}</div>
      </td>
      <td className="settings-actions">
        <button
          type="button"
          className={`icon-button${p.refreshing ? " spinning" : ""}`}
          aria-label={`Refresh ${p.name}`}
          title="Refresh"
          disabled={disabled || broken || p.detection === "notFound"}
          onClick={() => apply(ipc.usageRefresh("manual", p.id), onError)}
        >
          <RefreshCw size={13} />
        </button>
        <Switch
          label={`Read ${p.name}`}
          checked={p.enabled}
          disabled={disabled || broken}
          onChange={(v) => apply(ipc.usageSetProviderEnabled(p.id, v), onError)}
        />
      </td>
    </tr>
  );
}
