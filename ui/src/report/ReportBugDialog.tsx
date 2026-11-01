import { open as openFile } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { X } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ClipboardEvent } from "react";
import { Modal } from "../dialogs/Modal";
import { ipc } from "../ipc/commands";
import type { ReportConfigView, ReportError, ReportImageMime, ReportInput } from "../ipc/types";
import { shortcut } from "../platform";
import { clearDraft, getDraft, setDraft, useDraft, type DraftImage } from "./draft";

// UI text is English (constitution), except this one sentence, which the maintainer chose on
// purpose and must stay exactly as written (spec FR-003, plan Complexity Tracking).
export const IMPROVEMENT_POINTS = "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA";

export const MAX_NAME = 100;
export const MAX_DESCRIPTION = 10_000;
export const MAX_IMAGE = 5 * 1024 * 1024;
const COUNTER_FROM = 9_000;
const IMAGE_TYPES: readonly string[] = ["image/png", "image/jpeg"];

const IMAGE_PROBLEM: Record<string, string> = {
  too_large: "That image is larger than 5 MB.",
  unsupported_type: "Only PNG or JPEG images can be attached.",
  unreadable: "Could not read that file.",
};

function imageProblem(err: unknown): string {
  const reason = (err as { reason?: unknown } | null)?.reason;
  return (typeof reason === "string" && IMAGE_PROBLEM[reason]) || IMAGE_PROBLEM.unreadable;
}

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

/** A preview address for the image, revoked when the image changes or the dialog closes. */
function usePreview(image: DraftImage | null): string | null {
  const url = useMemo(
    () => (image ? URL.createObjectURL(new Blob([image.bytes], { type: image.mime })) : null),
    [image],
  );
  useEffect(() => () => {
    if (url) URL.revokeObjectURL(url);
  }, [url]);
  return url;
}

export function ReportBugDialog({ onClose }: Props) {
  const { name, description, image, includeLogs } = useDraft();
  const [phase, setPhase] = useState<Phase>({ kind: "editing" });
  const [imageError, setImageError] = useState<string | null>(null);
  const config = useRef<Promise<ReportConfigView> | null>(null);
  const busy = useRef(false);
  const previewUrl = usePreview(image);

  useEffect(() => {
    config.current = ipc.reportConfig().catch(() => ({ serviceConfigured: false, fallbackEmail: null }));
  }, []);

  const attach = (bytes: Uint8Array<ArrayBuffer>, mime: ReportImageMime) => {
    setImageError(null);
    edit({ image: { bytes, mime } });
  };

  // A dropped or chosen file goes through core, which checks its size and type.
  const attachPath = async (path: string) => {
    try {
      const view = await ipc.reportReadImage(path);
      attach(new Uint8Array(view.bytes), view.mime);
    } catch (err) {
      setImageError(imageProblem(err));
    }
  };
  const attachPathRef = useRef(attachPath);
  attachPathRef.current = attachPath;

  // Tauri takes file drops before the page sees them; listen only while the dialog is open.
  useEffect(() => {
    let stop: (() => void) | null = null;
    let closed = false;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        if (e.payload.type === "drop" && e.payload.paths.length > 0) void attachPathRef.current(e.payload.paths[0]);
      })
      .then((off) => (closed ? off() : (stop = off)))
      .catch(() => {});
    return () => {
      closed = true;
      stop?.();
    };
  }, []);

  async function chooseFile() {
    const path = await openFile({
      multiple: false,
      directory: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
    }).catch(() => null);
    if (typeof path === "string") await attachPath(path);
  }

  // Pasting an image attaches it; pasting text stays normal.
  function onPaste(e: ClipboardEvent) {
    const items = Array.from(e.clipboardData?.items ?? []);
    const item = items.find((i) => i.kind === "file" && i.type.startsWith("image/"));
    if (!item) return;
    e.preventDefault();
    const file = item.getAsFile();
    if (!file || !IMAGE_TYPES.includes(item.type)) {
      setImageError(IMAGE_PROBLEM.unsupported_type);
      return;
    }
    if (file.size > MAX_IMAGE) {
      setImageError(IMAGE_PROBLEM.too_large);
      return;
    }
    file
      .arrayBuffer()
      .then((buf) => attach(new Uint8Array(buf), item.type as ReportImageMime))
      .catch(() => setImageError(IMAGE_PROBLEM.unreadable));
  }

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
      <div className="report-dialog" onPaste={onPaste}>
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
          <div className="report-image">
            {image && previewUrl ? (
              <div className="report-preview">
                <img src={previewUrl} alt="Attached image" />
                <button
                  type="button"
                  className="icon-button"
                  aria-label="Remove image"
                  onClick={() => {
                    setImageError(null);
                    edit({ image: null });
                  }}
                >
                  <X size={14} />
                </button>
              </div>
            ) : (
              <small className="field-label">
                Paste a screenshot ({shortcut("paste")}), drop a PNG or JPEG here, or attach one. Up to 5 MB.
              </small>
            )}
            <button type="button" onClick={() => void chooseFile()}>
              Attach image…
            </button>
            {imageError && (
              <p role="alert" className="error">
                {imageError}
              </p>
            )}
          </div>
          <label className="check">
            <input
              type="checkbox"
              checked={includeLogs}
              onChange={(e) => edit({ includeLogs: e.target.checked })}
            />
            <span>
              Include logs
              <small>Logs can contain folder paths and project names.</small>
            </span>
          </label>
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
