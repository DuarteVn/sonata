import { afterEach, describe, expect, it, vi } from "vitest";
import { GIF, PNG, call, form, mockResend, post } from "./helpers";

afterEach(() => vi.restoreAllMocks());

async function expectRejected(req: Request, status: number, error: string) {
  const resend = mockResend();
  const res = await call(req);
  expect(res.status).toBe(status);
  expect(await res.json()).toEqual({ ok: false, error });
  expect(resend).not.toHaveBeenCalled();
}

const bigPng = (size: number) => {
  const b = new Uint8Array(size);
  b.set(PNG);
  return new Blob([b]);
};

describe("request checks", () => {
  it.each([
    ["name", { name: undefined }],
    ["name", { name: "   " }],
    ["name", { name: "a".repeat(101) }],
    ["description", { description: undefined }],
    ["description", { description: "d".repeat(10_001) }],
    ["appVersion", { appVersion: "v1" }],
    ["appVersion", { appVersion: "0.3" }],
    ["os", { os: "freebsd" }],
    ["osVersion", { osVersion: "" }],
    ["osVersion", { osVersion: "x".repeat(201) }],
    ["arch", { arch: "ARM 64" }],
    ["sentAt", { sentAt: "yesterday" }],
    ["logsRequested", { logsRequested: "yes" }],
    ["image", { image: new Blob([GIF], { type: "image/png" }) }],
    ["image", { image: bigPng(5 * 1024 * 1024 + 1) }],
    ["image", { image: "not a file" }],
    ["log", { log: new Blob(["line\n"]) }],
    ["log", { logsRequested: "true", log: new Blob([new Uint8Array(1024 * 1024 + 1)]) }],
  ])("rejects a bad %s part", async (field, overrides) => {
    await expectRejected(post(form(overrides)), 400, `invalid_field:${field}`);
  });

  it("accepts limits at their edges", async () => {
    const resend = mockResend();
    const res = await call(
      post(
        form({
          name: "😀".repeat(100),
          description: "é".repeat(10_000),
          appVersion: "1.2.3-beta.1",
          image: bigPng(5 * 1024 * 1024),
          logsRequested: "true",
          log: new Blob([new Uint8Array(1024 * 1024).fill(0x61)]),
          extra: "ignored",
        }),
      ),
    );
    expect(res.status).toBe(200);
    expect(resend).toHaveBeenCalledOnce();
  });

  it("rejects a body declared over 7 MB before reading it", async () => {
    await expectRejected(post(form(), { headers: { "Content-Length": String(7 * 1024 * 1024 + 1) } }), 413, "too_large");
  });

  it("rejects a body that is not a form", async () => {
    await expectRejected(post("{}", { headers: { "Content-Type": "application/json" } }), 400, "invalid_field:form");
  });

  it("answers 405 for other methods", async () => {
    await expectRejected(post(null, { method: "GET" }), 405, "method_not_allowed");
  });

  it("answers 404 for other paths", async () => {
    await expectRejected(post(form(), { path: "/" }), 404, "not_found");
    await expectRejected(post(form(), { path: "/report/x" }), 404, "not_found");
  });
});
