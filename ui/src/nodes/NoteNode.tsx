import { X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { ResizeHandles } from "../canvas/ResizeHandles";
import { startDrag } from "../canvas/useDrag";
import { ipc } from "../ipc/commands";
import type { CanvasNode } from "../ipc/types";
import { useCanvas } from "../state/canvas";

const NOTE_TITLE_HEIGHT = 22;
const SAVE_DEBOUNCE_MS = 500;

// Markdown sticky note (FR-019). Text lives in notes/<id>.md; the core keeps it in sync with
// edits made outside the app (FR-020a).
export function NoteNode({ node }: { node: CanvasNode }) {
  const { frame } = node;
  const note = useCanvas((s) => s.notes[node.id]);
  const focused = useCanvas((s) => s.focusedId === node.id);
  const { loadNote, setNoteText, previewFrame, commitFrame, bringToFront, setFocus, removeNode } =
    useCanvas.getState();
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    if (!note?.loaded) void loadNote(node.id);
  }, [node.id, note?.loaded, loadNote]);

  // Outside edits arrive while not editing; keep the draft in sync when nothing was typed.
  useEffect(() => {
    if (!editing && note) setDraft(note.text);
  }, [note?.text, editing, note]);

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

  const warning =
    note?.sync === "conflict"
      ? `This note was changed outside Sonata while you were editing. Your text was kept; the other version is in ${note.conflictPath ?? "a conflict copy"}.`
      : note?.sync === "fileMissing"
        ? "The note file was deleted outside Sonata. It will be written again on the next save."
        : null;

  return (
    <div
      className={`node note-node${focused ? " focused" : ""}`}
      data-node-id={node.id}
      data-node-kind="note"
      style={{ left: frame.x, top: frame.y, width: frame.width, height: frame.height, zIndex: node.z }}
    >
      <div
        className="note-title"
        style={{ height: NOTE_TITLE_HEIGHT }}
        onPointerDown={(e) => {
          if (e.button !== 0 || (e.target as HTMLElement).closest("button")) return;
          setFocus(null);
          void bringToFront(node.id);
          const zoom = useCanvas.getState().camera.zoom;
          const moved = (dx: number, dy: number) => ({ ...frame, x: frame.x + dx / zoom, y: frame.y + dy / zoom });
          startDrag(
            e,
            (dx, dy) => previewFrame(node.id, moved(dx, dy)),
            (dx, dy) => {
              if (dx !== 0 || dy !== 0) void commitFrame(node.id, moved(dx, dy));
            },
          );
        }}
      >
        <span className="note-label">Note</span>
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
          className="note-editor"
          aria-label="Note text"
          autoFocus
          value={draft}
          spellCheck
          onChange={(e) => change(e.target.value)}
          onBlur={finish}
          onKeyDown={(e) => {
            if (e.key === "Escape") (e.target as HTMLTextAreaElement).blur();
          }}
        />
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
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{note.text}</ReactMarkdown>
          ) : (
            <span className="note-placeholder">Double-click to write Markdown…</span>
          )}
        </div>
      )}
      <ResizeHandles nodeId={node.id} frame={frame} minWidth={120} minHeight={80} />
    </div>
  );
}
