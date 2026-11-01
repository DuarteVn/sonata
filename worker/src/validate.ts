// Request checks for POST /report (contracts/report-service.md). Every request is untrusted.

export const MAX_BODY = 7 * 1024 * 1024;
export const MAX_IMAGE = 5 * 1024 * 1024;
export const MAX_LOG = 1024 * 1024;

export type ImageKind = "png" | "jpeg";

export type Report = {
  name: string;
  description: string;
  appVersion: string;
  os: "macos" | "linux" | "windows";
  osVersion: string;
  arch: string;
  sentAt: string;
  logsRequested: boolean;
  image?: { bytes: Uint8Array; kind: ImageKind };
  log?: Uint8Array;
};

export type Rejection = { status: 400 | 404 | 405 | 413; error: string };

export type Parsed = { ok: true; report: Report } | ({ ok: false } & Rejection);

const APP_VERSION = /^\d+\.\d+\.\d+([-+][0-9A-Za-z.-]+)?$/;
const ARCH = /^[a-z0-9_]{1,20}$/;
const RFC3339 = /^\d{4}-\d{2}-\d{2}[Tt]\d{2}:\d{2}:\d{2}(\.\d+)?([Zz]|[+-]\d{2}:\d{2})$/;
const OSES = ["macos", "linux", "windows"] as const;

const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG = [0xff, 0xd8, 0xff];

class Invalid extends Error {
  constructor(readonly field: string) {
    super(field);
  }
}

const reject = (status: Rejection["status"], error: string): Parsed => ({ ok: false, status, error });

/** Length in Unicode code points, the same unit the app counts. */
const length = (s: string) => [...s].length;

function text(form: FormData, field: string, min: number, max: number): string {
  const value = form.get(field);
  if (typeof value !== "string") throw new Invalid(field);
  const trimmed = value.trim();
  const n = length(trimmed);
  if (n < min || n > max) throw new Invalid(field);
  return trimmed;
}

function matching(form: FormData, field: string, pattern: RegExp): string {
  const value = form.get(field);
  if (typeof value !== "string" || !pattern.test(value)) throw new Invalid(field);
  return value;
}

function file(form: FormData, field: string): File | undefined {
  const value = form.get(field);
  if (value === null) return undefined;
  if (typeof value === "string") throw new Invalid(field);
  return value;
}

export function imageKind(bytes: Uint8Array): ImageKind | undefined {
  const starts = (magic: number[]) => bytes.length >= magic.length && magic.every((b, i) => bytes[i] === b);
  if (starts(PNG)) return "png";
  if (starts(JPEG)) return "jpeg";
  return undefined;
}

/** Checks method, path, size, then every field and file, in the contract's order. */
export async function parseReport(request: Request): Promise<Parsed> {
  const url = new URL(request.url);
  if (url.pathname !== "/report") return reject(404, "not_found");
  if (request.method !== "POST") return reject(405, "method_not_allowed");
  const declared = request.headers.get("Content-Length");
  if (declared !== null && !(Number(declared) <= MAX_BODY)) return reject(413, "too_large");

  let form: FormData;
  try {
    form = await request.formData();
  } catch {
    return reject(400, "invalid_field:form");
  }

  try {
    const name = text(form, "name", 1, 100);
    const description = text(form, "description", 1, 10_000);
    const appVersion = matching(form, "appVersion", APP_VERSION);
    const os = form.get("os");
    if (typeof os !== "string" || !(OSES as readonly string[]).includes(os)) throw new Invalid("os");
    const osVersion = text(form, "osVersion", 1, 200);
    const arch = matching(form, "arch", ARCH);
    const sentAt = matching(form, "sentAt", RFC3339);
    if (Number.isNaN(Date.parse(sentAt))) throw new Invalid("sentAt");
    const logs = form.get("logsRequested");
    if (logs !== "true" && logs !== "false") throw new Invalid("logsRequested");
    const logsRequested = logs === "true";

    const report: Report = {
      name,
      description,
      appVersion,
      os: os as Report["os"],
      osVersion,
      arch,
      sentAt,
      logsRequested,
    };

    const image = file(form, "image");
    if (image) {
      if (image.size === 0 || image.size > MAX_IMAGE) throw new Invalid("image");
      const bytes = new Uint8Array(await image.arrayBuffer());
      const kind = imageKind(bytes);
      if (!kind) throw new Invalid("image");
      report.image = { bytes, kind };
    }

    const log = file(form, "log");
    if (log) {
      if (!logsRequested || log.size > MAX_LOG) throw new Invalid("log");
      report.log = new Uint8Array(await log.arrayBuffer());
    }
    return { ok: true, report };
  } catch (err) {
    if (err instanceof Invalid) return reject(400, `invalid_field:${err.field}`);
    throw err;
  }
}
