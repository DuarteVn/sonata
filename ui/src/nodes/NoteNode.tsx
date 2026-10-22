import { convertFileSrc } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { useEffect, useRef, useState, type ComponentProps } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { NOTE_COLORS } from "../canvas/NodeToolbar";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startNodeDrag } from "../canvas/nodeDrag";
import { ipc } from "../ipc/commands";
import { noteStyle, type CanvasNode } from "../ipc/types";
import { noteEditors } from "../notes/editorRegistry";
import { applyAction, toggleCheckbox, type Edit } from "../notes/markdownEdit";
import { useCanvas } from "../state/canvas";
import { LinkCount, LinkHandle } from "./LinkHandle";

const NOTE_TITLE_HEIGHT = 22;
const SAVE_DEBOUNCE_MS = 500;
export const NOTE_TEXT_PX = [11, 12, 13, 15, 18];

/** Resolves a note image path (relative to notes/) to a URL the webview may load. */
function imageUrl(src: string | undefined, assetBase: string | undefined) {
  if (!src || /^(https?:|data:|asset:)/.test(src) || !assetBase) return src;
  try {
    return convertFileSrc(`${assetBase}/${src}`);
  } catch {
    return src;
  }
}

// Markdown sticky note (FR-019, feature 002 US4). Text lives in notes/<id>.md; the core keeps
// it in sync with outside edits (FR-020a).
export function NoteNode({ node }: { node: CanvasNode }) {
  const { frame } = node;
  const style = noteStyle(node);
  const note = useCanvas((s) => s.notes[node.id]);
  const selected = useCanvas((s) => s.selection?.kind === "node" && s.selection.id === node.id);
  const grouped = useCanvas((s) => s.group.includes(node.id));
  const raw = useCanvas((s) => s.rawNotes.has(node.id));
  const { loadNote, setNoteText, bringToFront, pressNode, removeNode } = useCanvas.getState();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const editor = useRef<HTMLTextAreaElement>(null);
  const pendingSelection = useRef<[number, number] | null>(null);

  useEffect(() => {
    if (!note?.loaded) void loadNote(node.id);
  }, [node.id, note?.loaded, loadNote]);

  // Outside edits arrive while not editing; keep the draft in sync when nothing was typed.
  useEffect(() => {
    if (!editing && note) setDraft(note.text);
  }, [note?.text, editing, note]);

  // Restore the selection after a toolbar action changed the text.
  useEffect(() => {
    const sel = pendingSelection.current;
    if (sel && editor.current) {
      editor.current.focus();
      editor.current.setSelectionRange(sel[0], sel[1]);
      pendingSelection.current = null;
    }
  });

  function change(text: string) {
    setDraft(text);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void ipc.noteWrite(node.id, text, false), SAVE_DEBOUNCE_MS);
  }

  function finish() {
    clearTimeout(timer.current);
    setEditing(false);
    setNoteText(node.id, draft, "clean");
    void ipc.noteWrite(node.id, draft, true);
  }

  // Toolbar actions (US4): edit at the cursor, or at the end when not editing.
  useEffect(() => {
    const current = (): Edit => {
      const el = editor.current;
      const text = el ? el.value : (note?.text ?? "");
      return el && editing ? { text, start: el.selectionStart, end: el.selectionEnd } : { text, start: text.length, end: text.length };
    };
    const commit = (e: Edit) => {
      if (!editing) setEditing(true);
      change(e.text);
      pendingSelection.current = [e.start, e.end];
    };
    noteEditors.set(node.id, {
      apply: (action) => commit(applyAction(action, current())),
      insert: (text) => {
        const e = current();
        const lead = e.start > 0 && e.text[e.start - 1] !== "\n" ? "\n" : "";
        const piece = `${lead}${text}\n`;
        commit({ text: e.text.slice(0, e.start) + piece + e.text.slice(e.end), start: e.start + piece.length, end: e.start + piece.length });
      },
    });
    return () => {
      noteEditors.delete(node.id);
    };
  });

  function toggleTask(line: number) {
    const text = toggleCheckbox(note?.text ?? "", line);
    setNoteText(node.id, text, "clean");
    void ipc.noteWrite(node.id, text, true);
  }

  const warning =
    note?.sync === "conflict"
      ? `This note was changed outside Sonata while you were editing. Your text was kept; the other version is in ${note.conflictPath ?? "a conflict copy"}.`
      : note?.sync === "fileMissing"
        ? "The note file was deleted outside Sonata. It will be written again on the next save."
        : null;

  const colors = NOTE_COLORS.find((c) => c.id === style.color);
  const fontSize = NOTE_TEXT_PX[style.textSize - 1] ?? 13;

  const components = {
    img: ({ src, alt }: ComponentProps<"img">) => (
      <img src={imageUrl(typeof src === "string" ? src : undefined, note?.assetBase)} alt={alt ?? ""} draggable={false} />
    ),
    input: ({ checked, type }: ComponentProps<"input">) =>
      type === "checkbox" ? <input type="checkbox" checked={!!checked} onChange={() => {}} /> : null,
    li: ({ node: hast, children, className }: ComponentProps<"li"> & { node?: { position?: { start: { line: number } } } }) => (
      <li
        className={className}
        onClick={(e) => {
          if ((e.target as HTMLElement).tagName === "INPUT" && hast?.position) toggleTask(hast.position.start.line);
        }}
      >
        {children}
      </li>
    ),
  };

  return (
    <div
      className={`node note-node color-${style.color}${selected || grouped ? " selected" : ""}`}
      data-node-id={node.id}
      data-node-kind="note"
      style={{
        left: frame.x,
        top: frame.y,
        width: frame.width,
        height: frame.height,
        zIndex: node.z,
        fontSize,
        ["--note-swatch" as string]: colors?.swatch,
      }}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        if ((e.target as HTMLElement).closest("button, .link-handle, .resize-handle")) return;
        if (pressNode(node.id, "note", e.shiftKey)) e.preventDefault();
      }}
    >
      <div
        className="note-title"
        style={{ height: NOTE_TITLE_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
          if (useCanvas.getState().linkingFrom || e.shiftKey) return;
          void bringToFront(node.id);
          startNodeDrag(e, node.id);
        }}
      >
        <span className="note-label">Note</span>
        <span className="node-spacer" />
        <LinkCount nodeId={node.id} />
        <button type="button" className="icon-button" aria-label="Delete note" onClick={() => void removeNode(node.id, true)}>
          <X size={12} />
        </button>
      </div>
      {warning && (
        <p role="alert" className="note-warning">
          {warning}
        </p>
      )}
      {editing ? (
        <textarea
          ref={editor}
          className="note-editor"
          aria-label="Note text"
          autoFocus
          value={draft}
          spellCheck
          style={{ fontSize }}
          onChange={(e) => change(e.target.value)}
          onBlur={finish}
          onKeyDown={(e) => {
            if (e.key === "Escape") (e.target as HTMLTextAreaElement).blur();
          }}
        />
      ) : raw ? (
        <pre className="note-raw" data-testid="note-view" onDoubleClick={() => setEditing(true)}>
          {note?.text}
        </pre>
      ) : (
        <div
          className="note-view markdown"
          data-testid="note-view"
          onDoubleClick={() => {
            setDraft(note?.text ?? "");
            setEditing(true);
          }}
        >
          {note?.text ? (
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
              {note.text}
            </ReactMarkdown>
          ) : (
            <span className="note-placeholder">Double-click to write Markdown…</span>
          )}
        </div>
      )}
      {selected && <LinkHandle nodeId={node.id} />}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={120} minHeight={80} />
    </div>
  );
}
