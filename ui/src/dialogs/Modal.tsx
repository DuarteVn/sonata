import { useEffect, useId, useRef, type ReactNode } from "react";

type Props = {
  title: string;
  children: ReactNode;
  onClose: () => void;
};

// Minimal accessible modal: labelled dialog, Escape closes, focus moves inside on open.
export function Modal({ title, children, onClose }: Props) {
  const titleId = useId();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("input, button")?.focus();
  }, []);

  return (
    <div className="modal-backdrop" onMouseDown={onClose}>
      <div
        ref={ref}
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.stopPropagation();
            onClose();
          }
        }}
      >
        <h2 id={titleId}>{title}</h2>
        {children}
      </div>
    </div>
  );
}
