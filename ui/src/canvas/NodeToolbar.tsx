import { open as openFilePicker } from "@tauri-apps/plugin-dialog";
import {
  AArrowDown,
  AArrowUp,
  Bold,
  ExternalLink,
  Code,
  FileCode2,
  Heading,
  Image,
  Italic,
  MessagesSquare,
  List,
  ListChecks,
  Pencil,
  RotateCcw,
  Spline,
  Strikethrough,
  Trash2,
  Type,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import { ConfirmDialog } from "../dialogs/ConfirmDialog";
import { ipc } from "../ipc/commands";
import { isAppError, isTerminal, noteStyle, type CanvasNode, type NoteColor } from "../ipc/types";
import { MessageLog } from "../nodes/MessageLog";
import { noteEditors } from "../notes/editorRegistry";
import type { FormatAction } from "../notes/markdownEdit";
import { useCanvas } from "../state/canvas";
import { useWorkspaces } from "../state/workspaces";
import { frameToScreen, type ScreenRect } from "./viewport";

const GAP = 10;
const MIN_ROOM_ABOVE = 56;
const MARGIN = 8;

/** Screen position for a node's toolbar: above, else below; clamped; null when off screen. */
export function placeToolbar(
  node: ScreenRect,
  viewport: { width: number; height: number },
  size: { width: number; height: number },
): { left: number; top: number; below: boolean } | null {
  const visible =
    node.left < viewport.width && node.left + node.width > 0 && node.top < viewport.height && node.top + node.height > 0;
  if (!visible) return null;
  const below = node.top < MIN_ROOM_ABOVE;
  const top = below ? node.top + node.height + GAP : node.top - size.height - GAP;
  const centered = node.left + node.width / 2 - size.width / 2;
  const left = Math.min(Math.max(centered, MARGIN), viewport.width - size.width - MARGIN);
  return { left, top, below };
}

export const NOTE_COLORS: { id: NoteColor; swatch: string }[] = [
  { id: "yellow", swatch: "#f5e6a8" },
  { id: "blue", swatch: "#a8c8f5" },
  { id: "green", swatch: "#b5e3b0" },
  { id: "pink", swatch: "#f5b8d0" },
  { id: "purple", swatch: "#cdb8f5" },
  { id: "gray", swatch: "#cfd2d8" },
];

const SIZES = {
  terminal: { width: 232, height: 40 },
  note: { width: 600, height: 40 },
  browser: { width: 180, height: 40 },
  device: { width: 120, height: 40 },
};

function BrowserActions({ node }: { node: CanvasNode }) {
  const addWarning = useWorkspaces((s) => s.addWarning);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));
  return (
    <>
      <LinkButton node={node} />
      <span className="tb-sep" />
      <Btn label="Reload page" onClick={() => void ipc.browserAction(node.id, "reload").catch(report)}>
        <RotateCcw size={15} />
      </Btn>
      <Btn label="Open in default browser" onClick={() => void ipc.browserAction(node.id, "external").catch(report)}>
        <ExternalLink size={15} />
      </Btn>
      <span className="tb-sep" />
      <Btn label="Delete browser" onClick={() => void useCanvas.getState().removeNode(node.id, true).catch(report)}>
        <Trash2 size={15} />
      </Btn>
    </>
  );
}

// Device nodes (feature 007): link and delete; screenshot and stop arrive with the live screen.
function DeviceActions({ node }: { node: CanvasNode }) {
  const addWarning = useWorkspaces((s) => s.addWarning);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));
  return (
    <>
      <LinkButton node={node} />
      <span className="tb-sep" />
      <Btn label="Delete device" onClick={() => void useCanvas.getState().removeNode(node.id, true).catch(report)}>
        <Trash2 size={15} />
      </Btn>
    </>
  );
}

function Btn({ label, onClick, children, active }: { label: string; onClick: () => void; children: ReactNode; active?: boolean }) {
  return (
    <button
      type="button"
      className={`tb-btn${active ? " active" : ""}`}
      aria-label={label}
      title={label}
      aria-pressed={active}
      // Keep keyboard focus (and the text selection) in the note editor.
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

function LinkButton({ node }: { node: CanvasNode }) {
  const count = useCanvas((s) => s.links.filter((l) => l.a === node.id || l.b === node.id).length);
  const linking = useCanvas((s) => s.linkingFrom === node.id);
  return (
    <Btn label={linking ? "Click another item to link it" : "Link to another item"} active={linking} onClick={() => useCanvas.getState().startLinking(node.id)}>
      <Spline size={15} />
      {count > 0 && <span className="tb-count">{count}</span>}
    </Btn>
  );
}

export function NodeToolbar({ viewport }: { viewport: ScreenRect }) {
  const selection = useCanvas((s) => s.selection);
  const node = useCanvas((s) => (s.selection?.kind === "node" ? s.nodes.find((n) => n.id === s.selection!.id) : undefined));
  const camera = useCanvas((s) => s.camera);
  const gesturing = useCanvas((s) => s.gesturing);
  if (!selection || !node || gesturing) return null;
  const kind = isTerminal(node) ? "terminal" : "browser" in node.content ? "browser" : "device" in node.content ? "device" : "note";
  const labels = { terminal: "Terminal actions", browser: "Browser actions", device: "Device actions", note: "Note actions" };
  const pos = placeToolbar(frameToScreen(camera, node.frame), viewport, SIZES[kind]);
  if (!pos) return null;
  return (
    <div
      className="node-toolbar"
      role="toolbar"
      aria-label={labels[kind]}
      style={{ left: pos.left, top: pos.top }}
    >
      {kind === "terminal" ? (
        <TerminalActions node={node} />
      ) : kind === "browser" ? (
        <BrowserActions node={node} />
      ) : kind === "device" ? (
        <DeviceActions node={node} />
      ) : (
        <NoteActions node={node} />
      )}
    </div>
  );
}

function TerminalActions({ node }: { node: CanvasNode }) {
  const { openTerminalDialog, removeNode } = useCanvas.getState();
  const addWarning = useWorkspaces((s) => s.addWarning);
  const [confirm, setConfirm] = useState<"restart" | "delete" | null>(null);
  const [log, setLog] = useState(false);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));

  async function restart(confirmed: boolean) {
    try {
      await ipc.terminalRestart(node.id, confirmed);
    } catch (err) {
      if (isAppError(err) && err.code === "TerminalBusy") setConfirm("restart");
      else report(err);
    }
  }

  async function remove(confirmed: boolean) {
    try {
      await removeNode(node.id, confirmed);
    } catch (err) {
      if (isAppError(err) && err.code === "TerminalBusy") setConfirm("delete");
      else report(err);
    }
  }

  return (
    <>
      <Btn label="Edit terminal" onClick={() => openTerminalDialog({ kind: "edit", nodeId: node.id })}>
        <Pencil size={15} />
      </Btn>
      <Btn label="Message log" active={log} onClick={() => setLog((v) => !v)}>
        <MessagesSquare size={15} />
      </Btn>
      <span className="tb-sep" />
      <LinkButton node={node} />
      <span className="tb-sep" />
      <Btn label="Restart terminal" onClick={() => void restart(false)}>
        <RotateCcw size={15} />
      </Btn>
      <span className="tb-sep" />
      <Btn label="Delete terminal" onClick={() => void remove(false)}>
        <Trash2 size={15} />
      </Btn>
      {log && <MessageLog nodeId={node.id} onClose={() => setLog(false)} />}
      {confirm && (
        <ConfirmDialog
          title={confirm === "restart" ? "Restart terminal?" : "Close terminal?"}
          body={
            confirm === "restart"
              ? "A program is still running. Restarting stops it and starts a fresh shell with the terminal's command."
              : "A program is still running. Closing the terminal stops it."
          }
          confirmLabel={confirm === "restart" ? "Restart" : "Close"}
          danger
          onCancel={() => setConfirm(null)}
          onConfirm={() => {
            const which = confirm;
            setConfirm(null);
            void (which === "restart" ? restart(true) : remove(true));
          }}
        />
      )}
    </>
  );
}

const FORMAT_BUTTONS: { action: FormatAction; label: string; icon: ReactNode }[] = [
  { action: "bold", label: "Bold", icon: <Bold size={15} /> },
  { action: "italic", label: "Italic", icon: <Italic size={15} /> },
  { action: "strike", label: "Strikethrough", icon: <Strikethrough size={15} /> },
  { action: "code", label: "Inline code", icon: <Code size={15} /> },
  { action: "heading", label: "Heading", icon: <Heading size={15} /> },
  { action: "checklist", label: "Checklist", icon: <ListChecks size={15} /> },
  { action: "bullet", label: "Bullet list", icon: <List size={15} /> },
  { action: "codeBlock", label: "Code block", icon: <FileCode2 size={15} /> },
];

function NoteActions({ node }: { node: CanvasNode }) {
  const style = noteStyle(node);
  const { setNoteStyle, toggleRaw, removeNode } = useCanvas.getState();
  const raw = useCanvas((s) => s.rawNotes.has(node.id));
  const addWarning = useWorkspaces((s) => s.addWarning);
  const [palette, setPalette] = useState(false);
  const report = (err: unknown) => addWarning(isAppError(err) ? err.message : String(err));
  const swatch = NOTE_COLORS.find((c) => c.id === style.color)?.swatch;

  async function insertImage() {
    const picked = await openFilePicker({
      multiple: false,
      filters: [{ name: "Pictures", extensions: ["png", "jpg", "jpeg", "gif", "webp"] }],
    });
    if (typeof picked !== "string") return;
    try {
      const { markdown } = await ipc.noteInsertImage(node.id, picked);
      noteEditors.get(node.id)?.insert(markdown);
    } catch (err) {
      report(err);
    }
  }

  return (
    <>
      <div className="tb-palette-anchor">
        <Btn label="Note color" onClick={() => setPalette((v) => !v)}>
          <span className="tb-swatch" style={{ background: swatch }} />
        </Btn>
        {palette && (
          <div className="tb-palette" role="radiogroup" aria-label="Note color">
            {NOTE_COLORS.map((c) => (
              <button
                key={c.id}
                type="button"
                role="radio"
                aria-checked={style.color === c.id}
                aria-label={c.id}
                className="swatch"
                style={{ background: c.swatch }}
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  setPalette(false);
                  setNoteStyle(node.id, { ...style, color: c.id }).catch(report);
                }}
              />
            ))}
          </div>
        )}
      </div>
      <span className="tb-sep" />
      <Btn label="Smaller text" onClick={() => style.textSize > 1 && void setNoteStyle(node.id, { ...style, textSize: style.textSize - 1 }).catch(report)}>
        <AArrowDown size={15} />
      </Btn>
      <Btn label="Larger text" onClick={() => style.textSize < 5 && void setNoteStyle(node.id, { ...style, textSize: style.textSize + 1 }).catch(report)}>
        <AArrowUp size={15} />
      </Btn>
      <span className="tb-sep" />
      {FORMAT_BUTTONS.map((b) => (
        <Btn key={b.action} label={b.label} onClick={() => noteEditors.get(node.id)?.apply(b.action)}>
          {b.icon}
        </Btn>
      ))}
      <Btn label="Insert picture" onClick={() => void insertImage()}>
        <Image size={15} />
      </Btn>
      <Btn label={raw ? "Show rendered note" : "Show raw Markdown"} active={raw} onClick={() => toggleRaw(node.id)}>
        <Type size={15} />
      </Btn>
      <span className="tb-sep" />
      <LinkButton node={node} />
      <span className="tb-sep" />
      <Btn label="Delete note" onClick={() => void removeNode(node.id, true).catch(report)}>
        <Trash2 size={15} />
      </Btn>
    </>
  );
}
