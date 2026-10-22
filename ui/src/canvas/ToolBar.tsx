import { Globe, MousePointer2, StickyNote, SquareTerminal, type LucideIcon } from "lucide-react";
import { useCanvas, type Tool } from "../state/canvas";

const TOOLS: { id: Tool; label: string; key: string; icon: LucideIcon }[] = [
  { id: "select", label: "Select", key: "V", icon: MousePointer2 },
  { id: "terminal", label: "Terminal", key: "T", icon: SquareTerminal },
  { id: "note", label: "Note", key: "N", icon: StickyNote },
  { id: "browser", label: "Browser", key: "B", icon: Globe },
];

const HINTS: Partial<Record<Tool, string>> = {
  terminal: "Click on the canvas to place the terminal, or press Enter to place it in the middle.",
  note: "Click on the canvas to place the note.",
  browser: "Click on the canvas to place a browser.",
};

// Bottom tool bar (FR-001, FR-002).
export function ToolBar({ onPlaceAtCenter }: { onPlaceAtCenter: () => void }) {
  const tool = useCanvas((s) => s.tool);
  const setTool = useCanvas((s) => s.setTool);
  return (
    <div className="tool-bar-wrap">
      {HINTS[tool] && <div className="tool-hint">{HINTS[tool]}</div>}
      <div className="tool-bar" role="toolbar" aria-label="Tools">
        {TOOLS.map((t) => {
          const Icon = t.icon;
          return (
            <button
              key={t.id}
              type="button"
              className={`tool${tool === t.id ? " active" : ""}`}
              aria-label={`${t.label} tool`}
              aria-pressed={tool === t.id}
              title={`${t.label} (${t.key})`}
              onClick={() => setTool(t.id)}
              onDoubleClick={() => t.id === "terminal" && onPlaceAtCenter()}
            >
              <Icon size={18} />
            </button>
          );
        })}
      </div>
    </div>
  );
}
