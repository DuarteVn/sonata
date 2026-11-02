import { open as openFile } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { X } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ClipboardEvent } from "react";
import { Modal } from "../dialogs/Modal";
import { ipc } from "../ipc/commands";
import type { ReportConfigView, ReportError, ReportImageMime, ReportInput } from "../ipc/types";
import { shortcut } from "../platform";
import { pngBlob, writeLater, writeTextNow } from "./clipboard";
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
  | { kind: "opening" }
  | { kind: "sent" }
  | { kind: "failed"; error: ReportError["kind"] }
  | { kind: "fallback"; message: string };

const FAILURE: Record<ReportError["kind"], string> = {
  invalid: "The report service did not accept this report.",
  unreachable: "Could not reach the report service. Check your connection and try again.",
  rate_limited: "Too many reports right now. Please try again later.",
  refused: "The report service did not accept this report.",
  not_configured: "This build cannot send reports directly.",
};

const MAIL_OPENED = "Your mail app opened. Attach the image by hand if you have one.";
const COPIED = "Report copied to the clipboard. Paste it in a message to the maintainer.";
const NOT_COPIED = "Could not copy the report. Select the text above and copy it by hand.";
const NOT_PREPARED = "Could not prepare the email. Please try again later.";

const EMAIL_OPENED = "Your mail app opened with the report.";
// The opener can finish between the two clicks of a double click; one press, one email.
const REOPEN_AFTER_MS = 1_000;

type EmailOutcome = { opened: boolean; truncated: boolean; hasImage: boolean; imageCopied: boolean; textCopied: boolean };

/** What to tell the user after pressing Open email (spec, session 2026-11-02). */
function emailMessage(o: EmailOutcome, address: string): string {
  if (!o.opened) return `${o.textCopied ? COPIED : NOT_COPIED} The address is ${address}.`;
  if (o.truncated) {
    if (!o.textCopied) return `${EMAIL_OPENED} It was cut because it is long. ${NOT_COPIED}`;
    const copied = "The report was long, so the full text is in the clipboard. Paste it into the email";
    return o.hasImage ? `${copied} and attach the image by hand.` : `${copied}.`;
  }
  if (!o.hasImage) return EMAIL_OPENED;
  return o.imageCopied
    ? `${EMAIL_OPENED} The image is in the clipboard: paste it into the email (${shortcut("paste")}).`
    : `${EMAIL_OPENED} Attach the image by hand.`;
}

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
  const [config, setConfig] = useState<ReportConfigView | null>(null);
  const busy = useRef(false);
  const lastOpened = useRef(0);
  const previewUrl = usePreview(image);

  useEffect(() => {
    let closed = false;
    ipc
      .reportConfig()
      // Without an answer the email flow still works; only the address in a message is missing.
      .catch((): ReportConfigView => ({ serviceConfigured: false, reportEmail: "", mailBodyRoom: 0 }))
      .then((view) => {
        if (!closed) setConfig(view);
      });
    return () => {
      closed = true;
    };
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
    // Read everything now: clipboard items stop working once the event ends.
    const mime = item.type;
    const file = item.getAsFile();
    if (!file || !IMAGE_TYPES.includes(mime)) {
      setImageError(IMAGE_PROBLEM.unsupported_type);
      return;
    }
    if (file.size > MAX_IMAGE) {
      setImageError(IMAGE_PROBLEM.too_large);
      return;
    }
    file
      .arrayBuffer()
      .then((buf) => attach(new Uint8Array(buf), mime as ReportImageMime))
      .catch(() => setImageError(IMAGE_PROBLEM.unreadable));
  }

  const ready = name.trim() !== "" && description.trim() !== "";
  const sending = phase.kind === "sending" || phase.kind === "opening";
  const useService = config?.serviceConfigured === true;

  async function send() {
    if (busy.current || !ready) return;
    busy.current = true;
    setPhase({ kind: "sending" });
    try {
      await ipc.reportSend(toInput());
      clearDraft();
      setPhase({ kind: "sent" });
    } catch (err) {
      setPhase({ kind: "failed", error: errorKind(err) });
    } finally {
      busy.current = false;
    }
  }

  // The default flow, with no report service (FR-020 to FR-022). Nothing may be awaited before
  // the clipboard writes start, or WebKit refuses them; see writeLater.
  async function openEmail(cfg: ReportConfigView) {
    if (busy.current || !ready || Date.now() - lastOpened.current < REOPEN_AFTER_MS) return;
    busy.current = true;
    setPhase({ kind: "opening" });
    const d = getDraft();
    const input: ReportInput = { name: d.name, description: d.description, image: null, includeLogs: false };
    const long = count(d.name.trim()) + count(d.description.trim()) > cfg.mailBodyRoom;
    const imageCopy = d.image && !long ? writeLater("image/png", pngBlob(d.image)) : null;
    const opening = imageCopy ? imageCopy.then(() => ipc.reportFallback(input)) : ipc.reportFallback(input);
    // The text comes from core and is wanted only sometimes; a rejected promise writes nothing.
    const textCopy = imageCopy
      ? null
      : writeLater(
          "text/plain",
          opening.then((r) => {
            if (r.opened && !r.truncated) throw new Error("no copy needed");
            return new Blob([r.text], { type: "text/plain" });
          }),
        );
    try {
      const result = await opening;
      const textWanted = !result.opened || result.truncated;
      const textCopied = textWanted && ((await textCopy) || (await writeTextNow(result.text)));
      setPhase({
        kind: "fallback",
        message: emailMessage(
          {
            opened: result.opened,
            truncated: result.truncated,
            hasImage: d.image !== null,
            imageCopied: (await imageCopy) === true,
            textCopied,
          },
          cfg.reportEmail,
        ),
      });
    } catch {
      setPhase({ kind: "fallback", message: NOT_PREPARED });
    } finally {
      lastOpened.current = Date.now();
      busy.current = false;
    }
  }

  function submit() {
    if (!config) return;
    void (config.serviceConfigured ? send() : openEmail(config));
  }

  // Mail app with the report filled in, or the clipboard when that is not possible (FR-011).
  async function sendByEmail() {
    if (busy.current) return;
    busy.current = true;
    try {
      const result = await ipc.reportFallback(toInput());
      const messages: string[] = [];
      if (result.opened) messages.push(MAIL_OPENED);
      if (!result.opened || result.truncated) {
        try {
          await navigator.clipboard.writeText(result.text);
          messages.push(COPIED);
        } catch {
          messages.push(NOT_COPIED);
        }
      }
      setPhase({ kind: "fallback", message: messages.join(" ") });
    } catch {
      setPhase({ kind: "fallback", message: NOT_PREPARED });
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
      : phase.kind === "opening"
        ? "Opening your mail app…"
        : phase.kind === "sent"
          ? "Thanks! Your report was sent."
          : phase.kind === "failed"
            ? FAILURE[phase.error]
            : phase.kind === "fallback"
              ? phase.message
              : "";
  const failed = phase.kind === "failed" ? phase.error : null;

  const chars = count(description);

  return (
    <Modal title="Report Bug" onClose={onClose}>
      <div className="report-dialog" onPaste={onPaste}>
        <p className="report-headline">{IMPROVEMENT_POINTS}</p>
        <form
          className="form"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
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
          {useService && (
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
          )}
          <p role="status" className="report-status" data-phase={phase.kind}>
            {status}
          </p>
          {failed && (
            <div className="report-fallback">
              {failed !== "not_configured" && (
                <button type="button" onClick={() => void send()}>
                  Try again
                </button>
              )}
              <button type="button" onClick={() => void sendByEmail()}>
                Send by email instead
              </button>
            </div>
          )}
          <div className="actions">
            <button type="button" onClick={onClose}>
              Close
            </button>
            <button type="submit" className="primary" disabled={sending || !ready || !config}>
              {useService ? "Send" : "Open email"}
            </button>
          </div>
        </form>
      </div>
    </Modal>
  );
}
