import { X } from "lucide-react";
import { useCanvas } from "../state/canvas";
import { linkAnchors, linkPath, pointAnchor } from "./linkGeometry";

export type LinkDrag = { from: string; x: number; y: number } | null;

// Links between nodes, drawn in canvas space below the nodes (research R7).
export function LinkLayer({ drag }: { drag: LinkDrag }) {
  const links = useCanvas((s) => s.links);
  const nodes = useCanvas((s) => s.nodes);
  const selection = useCanvas((s) => s.selection);
  const frames = new Map(nodes.map((n) => [n.id, n.frame]));
  const { select, deleteLink } = useCanvas.getState();

  const dragFrom = drag ? frames.get(drag.from) : undefined;
  return (
    <svg className="link-layer" aria-label="Links">
      {links.map((l) => {
        const a = frames.get(l.a);
        const b = frames.get(l.b);
        if (!a || !b) return null;
        const [p, q] = linkAnchors(a, b);
        const d = linkPath(p, q);
        const selected = selection?.kind === "link" && selection.id === l.id;
        const mid = { x: (p.x + q.x) / 2, y: (p.y + q.y) / 2 };
        return (
          <g key={l.id} className={`link${selected ? " selected" : ""}`} data-link-id={l.id}>
            <path d={d} className="link-line" />
            <path
              d={d}
              className="link-hit"
              role="button"
              aria-label="Link"
              onPointerDown={(e) => {
                e.stopPropagation();
                select({ kind: "link", id: l.id });
              }}
            />
            {selected && (
              <foreignObject x={mid.x - 12} y={mid.y - 12} width={24} height={24}>
                <button
                  type="button"
                  className="link-delete"
                  aria-label="Delete link"
                  onPointerDown={(e) => e.stopPropagation()}
                  onClick={() => void deleteLink(l.id)}
                >
                  <X size={12} />
                </button>
              </foreignObject>
            )}
          </g>
        );
      })}
      {drag && dragFrom && (
        <path
          className="link-line preview"
          d={linkPath(linkAnchors(dragFrom, { x: drag.x, y: drag.y, width: 0, height: 0 })[0], pointAnchor(drag.x, drag.y))}
        />
      )}
    </svg>
  );
}
