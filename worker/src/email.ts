// The email sent through Resend (research R4, contracts/report-service.md "Email produced").

import type { Env } from "./index";
import type { Report } from "./validate";

const RESEND_URL = "https://api.resend.com/emails";
const SUBJECT_SUMMARY = 60;

export type Attachment = { filename: string; content: string };

export type Email = {
  from: string;
  to: string[];
  subject: string;
  text: string;
  attachments: Attachment[];
};

const oneLine = (s: string) => s.replace(/\s+/g, " ").trim();

export function base64(bytes: Uint8Array): string {
  let binary = "";
  const CHUNK = 0x8000;
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK));
  }
  return btoa(binary);
}

export function logsLine(report: Report): string {
  if (report.log) return "included";
  return report.logsRequested ? "requested but unavailable" : "not requested";
}

export function buildEmail(env: Env, report: Report): Email {
  const summary = [...oneLine(report.description)].slice(0, SUBJECT_SUMMARY).join("");
  const text = [
    `From: ${report.name}`,
    `Sent: ${report.sentAt}`,
    `Sonata ${report.appVersion} · ${report.os} ${report.osVersion} · ${report.arch}`,
    `Logs: ${logsLine(report)}`,
    "",
    report.description,
  ].join("\n");
  const attachments: Attachment[] = [];
  if (report.image) {
    const filename = report.image.kind === "png" ? "screenshot.png" : "screenshot.jpg";
    attachments.push({ filename, content: base64(report.image.bytes) });
  }
  if (report.log) attachments.push({ filename: "sonata-log.txt", content: base64(report.log) });
  return {
    from: env.REPORT_FROM,
    to: [env.REPORT_TO],
    subject: `[Sonata bug] ${oneLine(report.name)}: ${summary}`,
    text,
    attachments,
  };
}

/** Whether Resend accepted the email. Its error bodies are never passed on. */
export async function sendEmail(env: Env, report: Report): Promise<boolean> {
  try {
    const res = await fetch(RESEND_URL, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${env.RESEND_API_KEY}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(buildEmail(env, report)),
    });
    return res.ok;
  } catch {
    return false;
  }
}
