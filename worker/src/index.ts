// Sonata report service (feature 013): POST /report → validate → email through Resend.

import { sendEmail } from "./email";
import { parseReport } from "./validate";

export interface Env {
  RESEND_API_KEY: string;
  REPORT_TO: string;
  REPORT_FROM: string;
}

function answer(status: number, error?: string, headers?: HeadersInit): Response {
  const body = error ? { ok: false, error } : { ok: true };
  return Response.json(body, { status, headers });
}

/** Logs only status, error code, and sizes; never content, addresses, or secrets. */
function log(status: number, error?: string, sizes: { image?: number; log?: number } = {}) {
  console.log(JSON.stringify({ status, error: error ?? null, ...sizes }));
}

export default {
  async fetch(request: Request, env: Env, _ctx: ExecutionContext): Promise<Response> {
    const parsed = await parseReport(request);
    if (!parsed.ok) {
      log(parsed.status, parsed.error);
      return answer(parsed.status, parsed.error);
    }
    const { report } = parsed;
    const sizes = { image: report.image?.bytes.length ?? 0, log: report.log?.length ?? 0 };
    if (!(await sendEmail(env, report))) {
      log(502, "email_failed", sizes);
      return answer(502, "email_failed");
    }
    log(200, undefined, sizes);
    return answer(200);
  },
} satisfies ExportedHandler<Env>;
