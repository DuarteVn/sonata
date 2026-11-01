import { afterEach, describe, expect, it, vi } from "vitest";
import { JPEG, PNG, base64, call, form, mockResend, post, resendBody } from "./helpers";

afterEach(() => vi.restoreAllMocks());

describe("POST /report", () => {
  it("emails a valid report with its screenshot through Resend", async () => {
    const resend = mockResend();
    const res = await call(post(form({ image: new Blob([PNG], { type: "text/plain" }) })));
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ ok: true });

    expect(resend).toHaveBeenCalledTimes(1);
    const [input, init] = resend.mock.calls[0];
    const req = new Request(input as RequestInfo, init as RequestInit);
    expect(req.url).toBe("https://api.resend.com/emails");
    expect(req.method).toBe("POST");
    expect(req.headers.get("Authorization")).toBe("Bearer test-key");
    expect(req.headers.get("Content-Type")).toContain("application/json");

    const body = await resendBody(resend);
    expect(body.from).toBe("Sonata Reports <onboarding@resend.dev>");
    expect(body.to).toEqual(["dest@example.test"]);
    expect(body.subject).toBe("[Sonata bug] Ana: The note vanished after I dragged it.");
    expect(body.text).toBe(
      [
        "From: Ana",
        "Sent: 2026-11-01T12:00:00Z",
        "Sonata 0.3.0 · macos 15.6.1 · aarch64",
        "Logs: not requested",
        "",
        "The note vanished after I dragged it.",
      ].join("\n"),
    );
    expect(body.attachments).toEqual([{ filename: "screenshot.png", content: base64(PNG) }]);
  });

  it("names a JPEG screenshot.jpg whatever its declared type", async () => {
    const resend = mockResend();
    const res = await call(post(form({ image: new Blob([JPEG], { type: "image/png" }) })));
    expect(res.status).toBe(200);
    const body = await resendBody(resend);
    expect(body.attachments).toEqual([{ filename: "screenshot.jpg", content: base64(JPEG) }]);
  });

  it("sends no attachments without an image", async () => {
    const resend = mockResend();
    expect((await call(post(form()))).status).toBe(200);
    expect((await resendBody(resend)).attachments ?? []).toEqual([]);
  });

  it("keeps the subject on one line and cuts the description to 60 characters", async () => {
    const resend = mockResend();
    const description = "First line\nsecond   line " + "x".repeat(100);
    await call(post(form({ name: " Bob\n", description })));
    const { subject, text } = await resendBody(resend);
    const summary = "First line second line " + "x".repeat(100);
    expect(subject).toBe(`[Sonata bug] Bob: ${summary.slice(0, 60)}`);
    expect(subject).not.toContain("\n");
    expect(text.endsWith(description)).toBe(true);
  });

  it("answers 502 email_failed when Resend refuses, without leaking secrets", async () => {
    mockResend(422);
    const res = await call(post(form()));
    expect(res.status).toBe(502);
    const raw = await res.text();
    expect(JSON.parse(raw)).toEqual({ ok: false, error: "email_failed" });
    expect(raw).not.toContain("test-key");
    expect(raw).not.toContain("dest@example.test");
  });

  it("answers 502 email_failed when the Resend call throws", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new Error("network down test-key"));
    const res = await call(post(form()));
    expect(res.status).toBe(502);
    const raw = await res.text();
    expect(raw).not.toContain("test-key");
    expect(raw).not.toContain("network");
  });
});
