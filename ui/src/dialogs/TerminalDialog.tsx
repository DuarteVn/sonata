import { open as openFolderPicker } from "@tauri-apps/plugin-dialog";
import { Bot, Brain, Code2, Sparkles, SquareTerminal, Trash2, type LucideIcon } from "lucide-react";
import { useEffect, useState, type FormEvent } from "react";
import { ipc } from "../ipc/commands";
import { isAppError, type PresetStatus, type Role, type RoleRef, type TerminalSettings } from "../ipc/types";
import { useCanvas, type TerminalDialogMode } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { Modal } from "./Modal";

type Preset = { id: string; label: string; name: string; command: string; icon: LucideIcon };

// Built-in quick starts; ids match sonata-core's presets module.
export const PRESETS: Preset[] = [
  { id: "claude", label: "Claude Code", name: "Claude Code", command: "claude", icon: Sparkles },
  { id: "codex", label: "Codex", name: "Codex", command: "codex", icon: Brain },
  { id: "opencode", label: "OpenCode", name: "OpenCode", command: "opencode", icon: Code2 },
  { id: "gemini", label: "Gemini CLI", name: "Gemini", command: "gemini", icon: Bot },
  { id: "shell", label: "Shell", name: "", command: "", icon: SquareTerminal },
];

export const ACCENTS = ["#3B82F6", "#10B981", "#F59E0B", "#EF4444", "#8B5CF6", "#EC4899", "#14B8A6", "#F97316"];

const TERMINAL_SIZE = { width: 640, height: 400 };
type Tab = "details" | "appearance" | "role";

type Props = { mode: TerminalDialogMode; onClose: () => void };

export function TerminalDialog({ mode, onClose }: Props) {
  const editing = mode.kind === "edit" ? useCanvas.getState().nodes.find((n) => n.id === mode.nodeId) : undefined;
  const current = editing && "terminal" in editing.content ? editing.content.terminal : undefined;
  const workspaceDir = useWorkspaces((s) => s.active?.workingDirectory ?? "");
  const defaultFont = useWorkspaces((s) => s.preferences.terminalFontSize);

  const [tab, setTab] = useState<Tab>("details");
  const [presets, setPresets] = useState<PresetStatus[]>([]);
  const [name, setName] = useState(current?.name ?? "");
  const [command, setCommand] = useState(current?.launchCommand ?? "");
  const [folder, setFolder] = useState(current?.workingDirectory ?? workspaceDir);
  const [monitor, setMonitor] = useState(current?.monitorActivity ?? true);
  const [orchestrator, setOrchestrator] = useState(current?.orchestrator ?? false);
  const [accent, setAccent] = useState<string | null>(current?.accentColor ?? null);
  const [fontSize, setFontSize] = useState(String(current?.fontSize ?? defaultFont));
  const [roleText, setRoleText] = useState(current?.role?.text ?? "");
  const [roleName, setRoleName] = useState(current?.role?.name ?? "");
  const [library, setLibrary] = useState<Role[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [restartNotice, setRestartNotice] = useState(false);

  useEffect(() => {
    ipc.presetsDetect().then(setPresets).catch(() => setPresets([]));
    ipc.rolesList().then(setLibrary).catch(() => setLibrary([]));
  }, []);

  const report = (err: unknown) => setError(isAppError(err) ? err.message : String(err));
  const found = (id: string) => presets.find((p) => p.id === id)?.found;

  function pickPreset(p: Preset) {
    setName(p.name);
    setCommand(p.command);
  }

  async function browse() {
    const picked = await openFolderPicker({ directory: true, multiple: false, defaultPath: folder || undefined });
    if (typeof picked === "string") setFolder(picked);
  }

  function role(): RoleRef | null {
    if (!roleText.trim()) return null;
    const fromLibrary = library.find((r) => r.name === roleName && r.text === roleText);
    return fromLibrary ? { name: fromLibrary.name, text: roleText } : { text: roleText };
  }

  function font(): number | null {
    const n = Number(fontSize);
    return Number.isFinite(n) && n !== defaultFont ? n : null;
  }

  async function saveRole() {
    try {
      const saved = await ipc.roleSave(roleName, roleText);
      setLibrary((l) => [...l, saved]);
      setError(null);
    } catch (err) {
      report(err);
    }
  }

  async function deleteRole(r: Role) {
    try {
      await ipc.roleDelete(r.id);
      setLibrary((l) => l.filter((x) => x.id !== r.id));
    } catch (err) {
      report(err);
    }
  }

  async function submit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    try {
      if (mode.kind === "create") {
        const at = mode.at ?? { x: 0, y: 0 };
        await useCanvas.getState().addTerminal(
          { ...at, ...TERMINAL_SIZE },
          {
            name: name.trim() || undefined,
            launchCommand: command.trim() || undefined,
            workingDirectory: folder && folder !== workspaceDir ? folder : undefined,
            settings: {
              accentColor: accent,
              fontSize: font(),
              role: role(),
              monitorActivity: monitor,
              orchestrator,
            },
          },
        );
        useCanvas.getState().setTool("select");
        onClose();
      } else {
        const settings: TerminalSettings = {
          name,
          launchCommand: command.trim() || null,
          workingDirectory: folder && folder !== workspaceDir ? folder : null,
          accentColor: accent,
          fontSize: font(),
          role: role(),
          monitorActivity: monitor,
          orchestrator,
        };
        const res = await ipc.terminalUpdateSettings(mode.nodeId, settings);
        useCanvas.getState().replaceNode(res.node);
        if (res.appliesOnRestart) setRestartNotice(true);
        else onClose();
      }
    } catch (err) {
      report(err);
    }
  }

  return (
    <Modal title={mode.kind === "create" ? "New terminal" : "Edit terminal"} onClose={onClose}>
      <form onSubmit={submit} className="form terminal-dialog">
        {mode.kind === "create" && (
          <div className="presets" aria-label="Quick start">
            {PRESETS.map((p) => {
              const Icon = p.icon;
              const missing = found(p.id) === false;
              return (
                <button
                  key={p.id}
                  type="button"
                  className={`preset${missing ? " missing" : ""}`}
                  title={missing ? `"${p.command}" was not found in your shell's PATH` : undefined}
                  onClick={() => pickPreset(p)}
                >
                  <Icon size={22} aria-hidden />
                  <span>{p.label}</span>
                  {missing && <span className="preset-hint">not found</span>}
                </button>
              );
            })}
          </div>
        )}

        <div role="tablist" className="tabs">
          {(["details", "appearance", "role"] as Tab[]).map((t) => (
            <button key={t} type="button" role="tab" aria-selected={tab === t} onClick={() => setTab(t)}>
              {t[0].toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>

        {tab === "details" && (
          <div role="tabpanel" className="tab-panel">
            <label>
              Name
              <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Terminal" />
            </label>
            <label>
              Command
              <input
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="e.g. claude or codex; empty for a plain shell"
                spellCheck={false}
              />
            </label>
            <label htmlFor="term-folder">Working folder</label>
            <div className="row">
              <input id="term-folder" value={folder} onChange={(e) => setFolder(e.target.value)} spellCheck={false} />
              <button type="button" onClick={browse}>
                Browse…
              </button>
            </div>
            <label className="check">
              <input type="checkbox" checked={monitor} onChange={(e) => setMonitor(e.target.checked)} />
              <span>
                Monitor activity
                <small>Shows when a program runs or finishes, and flags terminals that finished while you were elsewhere.</small>
              </span>
            </label>
            <label className="check">
              <input type="checkbox" checked={orchestrator} onChange={(e) => setOrchestrator(e.target.checked)} />
              <span>
                Orchestrator
                <small>Marks the agent that coordinates the others. Its extra commands arrive with agent messaging.</small>
              </span>
            </label>
          </div>
        )}

        {tab === "appearance" && (
          <div role="tabpanel" className="tab-panel">
            <span className="field-label">Accent color</span>
            <div className="swatches" role="radiogroup" aria-label="Accent color">
              <button type="button" role="radio" aria-checked={accent === null} aria-label="None" className="swatch none" onClick={() => setAccent(null)} />
              {ACCENTS.map((c) => (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={accent === c}
                  aria-label={c}
                  className="swatch"
                  style={{ background: c }}
                  onClick={() => setAccent(c)}
                />
              ))}
            </div>
            <label>
              Font size
              <input type="number" min={9} max={24} value={fontSize} onChange={(e) => setFontSize(e.target.value)} />
            </label>
          </div>
        )}

        {tab === "role" && (
          <div role="tabpanel" className="tab-panel">
            {library.length > 0 && (
              <ul className="role-list">
                {library.map((r) => (
                  <li key={r.id}>
                    <button
                      type="button"
                      aria-label={`Use ${r.name}`}
                      className="role-use"
                      onClick={() => {
                        setRoleText(r.text);
                        setRoleName(r.name);
                      }}
                    >
                      <strong>{r.name}</strong>
                      <span>{r.text}</span>
                    </button>
                    <button type="button" className="icon-button" aria-label={`Delete ${r.name}`} onClick={() => deleteRole(r)}>
                      <Trash2 size={13} />
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <label>
              Role text
              <textarea
                rows={6}
                value={roleText}
                onChange={(e) => setRoleText(e.target.value)}
                placeholder="A reusable instruction for this agent, e.g. what it is responsible for."
              />
            </label>
            <div className="row">
              <input aria-label="Role name" value={roleName} onChange={(e) => setRoleName(e.target.value)} placeholder="Role name" />
              <button type="button" onClick={saveRole} disabled={!roleText.trim() || !roleName.trim()}>
                Save to library
              </button>
            </div>
          </div>
        )}

        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        {restartNotice && (
          <p role="status" className="notice">
            Saved. The new command or folder applies when the terminal restarts.
          </p>
        )}
        <div className="actions">
          <button type="button" onClick={onClose}>
            {restartNotice ? "Close" : "Cancel"}
          </button>
          {!restartNotice && (
            <button type="submit" className="primary">
              {mode.kind === "create" ? "Create" : "Save"}
            </button>
          )}
        </div>
      </form>
    </Modal>
  );
}
