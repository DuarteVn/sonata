import { useEffect, useRef, useState } from "react";
import { Modal } from "../dialogs/Modal";
import { ipc } from "../ipc/commands";
import type { ReportConfigView, ReportError, ReportInput } from "../ipc/types";
import { clearDraft, getDraft, setDraft, useDraft } from "./draft";

// UI text is English (constitution), except this one sentence, which the maintainer chose on
// purpose and must stay exactly as written (spec FR-003, plan Complexity Tracking).
export const IMPROVEMENT_POINTS = "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA";

export const MAX_NAME = 100;
export const MAX_DESCRIPTION = 10_000;
const COUNTER_FROM = 9_000;

type Phase =
  | { kind: "editing" }
  | { kind: "sending" }
  | { kind: "sent" }
  | { kind: "failed"; error: ReportError["kind"] };

const FAILURE: Record<ReportError["kind"], string> = {
  invalid: "The report service did not accept this report.",
  unreachable: "Could not reach the report service. Check your connection and try again.",
  rate_limited: "Too many reports right now. Please try again later.",
  refused: "The report service did not accept this report.",
  not_configured: "This build cannot send reports directly.",
};

function errorKind(err: unknown): ReportError["kind"] {
  const kind = (err as { kind?: unknown } | null)?.kind;
  return typeof kind === "string" && kind in FAILURE ? (kind as ReportError["kind"]) : "unreachable";
}

function toInput(): ReportInput {
  const d = getDraft();
  return {
    name: d.name,
    description: d.description,
    image: d.image ? { bytes: Array.from(d.image.bytes), mime: d.image.mime } : null,
    includeLogs: d.includeLogs,
  };
}

const count = (s: string) => [...s].length;

type Props = { onClose: () => void };

export function ReportBugDialog({ onClose }: Props) {
  const { name, description } = useDraft();
  const [phase, setPhase] = useState<Phase>({ kind: "editing" });
  const config = useRef<Promise<ReportConfigView> | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    config.current = ipc.reportConfig().catch(() => ({ serviceConfigured: false, fallbackEmail: null }));
  }, []);

  const ready = name.trim() !== "" && description.trim() !== "";
  const sending = phase.kind === "sending";

  async function send() {
    if (busy.current || !ready) return;
    busy.current = true;
    setPhase({ kind: "sending" });
    try {
      const cfg = await (config.current ?? ipc.reportConfig());
      if (!cfg.serviceConfigured) {
        setPhase({ kind: "failed", error: "not_configured" });
        return;
      }
      await ipc.reportSend(toInput());
      clearDraft();
      setPhase({ kind: "sent" });
    } catch (err) {
      setPhase({ kind: "failed", error: errorKind(err) });
    } finally {
      busy.current = false;
    }
  }

  const edit = (patch: Parameters<typeof setDraft>[0]) => {
    setDraft(patch);
    if (phase.kind === "sent") setPhase({ kind: "editing" });
  };

  const status =
    phase.kind === "sending"
      ? "Sending…"
      : phase.kind === "sent"
        ? "Thanks! Your report was sent."
        : phase.kind === "failed"
          ? FAILURE[phase.error]
          : "";

  const chars = count(description);

  return (
    <Modal title="Report Bug" onClose={onClose}>
      <div className="report-dialog">
        <p className="report-headline">{IMPROVEMENT_POINTS}</p>
        <form
          className="form"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <label>
            Your name
            <input
              value={name}
              maxLength={MAX_NAME}
              autoComplete="name"
              onChange={(e) => edit({ name: e.target.value })}
            />
          </label>
          <label>
            What happened?
            <textarea
              value={description}
              maxLength={MAX_DESCRIPTION}
              rows={6}
              onChange={(e) => edit({ description: e.target.value })}
            />
          </label>
          {chars >= COUNTER_FROM && (
            <small className="report-counter">
              {chars.toLocaleString("en-US")} / {MAX_DESCRIPTION.toLocaleString("en-US")}
            </small>
          )}
          <p role="status" className="report-status" data-phase={phase.kind}>
            {status}
          </p>
          <div className="actions">
            <button type="button" onClick={onClose}>
              Close
            </button>
            <button type="submit" className="primary" disabled={sending || !ready}>
              Send
            </button>
          </div>
        </form>
      </div>
    </Modal>
  );
}
