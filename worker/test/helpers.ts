import { createExecutionContext, env } from "cloudflare:test";
import { vi } from "vitest";
import worker from "../src/index";

export const PNG = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1, 2, 3]);
export const JPEG = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 4, 5]);
export const GIF = new TextEncoder().encode("GIF89a\0\0");

export type Fields = Record<string, string | Blob | undefined>;

/** A valid report form; `overrides` replace or (with `undefined`) remove parts. */
export function form(overrides: Fields = {}): FormData {
  const base: Fields = {
    name: "Ana",
    description: "The note vanished after I dragged it.",
    appVersion: "0.3.0",
    os: "macos",
    osVersion: "15.6.1",
    arch: "aarch64",
    sentAt: "2026-11-01T12:00:00Z",
    logsRequested: "false",
  };
  const fd = new FormData();
  for (const [k, v] of Object.entries({ ...base, ...overrides })) {
    if (v === undefined) continue;
    if (typeof v === "string") fd.append(k, v);
    else fd.append(k, v, k === "image" ? "picture.bin" : "log.txt");
  }
  return fd;
}

let nextIp = 1;

/** A fresh client address, so tests do not share hourly slots. */
export function freshIp(): string {
  nextIp += 1;
  return `10.${Math.floor(nextIp / 250)}.${nextIp % 250}.1`;
}

export function post(
  body: BodyInit | null,
  init: { ip?: string; path?: string; method?: string; headers?: Record<string, string> } = {},
): Request {
  const headers = new Headers(init.headers);
  headers.set("CF-Connecting-IP", init.ip ?? freshIp());
  return new Request(`https://report.test${init.path ?? "/report"}`, {
    method: init.method ?? "POST",
    body,
    headers,
  });
}

export async function call(request: Request): Promise<Response> {
  return worker.fetch(request, env, createExecutionContext());
}

/** Replaces the global `fetch` (the Resend call) with a spy answering `status`. */
export function mockResend(status = 200) {
  return vi
    .spyOn(globalThis, "fetch")
    .mockImplementation(async () => Response.json({ id: "email-1" }, { status }));
}

export type ResendBody = {
  from: string;
  to: string[];
  subject: string;
  text: string;
  attachments?: { filename: string; content: string }[];
};

export async function resendBody(spy: ReturnType<typeof mockResend>, i = 0): Promise<ResendBody> {
  const [input, init] = spy.mock.calls[i];
  const req = new Request(input as RequestInfo, init as RequestInit);
  return (await req.json()) as ResendBody;
}

export function base64(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}
