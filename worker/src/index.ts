export interface Env {
  RESEND_API_KEY: string;
  REPORT_TO: string;
  REPORT_FROM: string;
}

export default {
  async fetch(_request: Request, _env: Env, _ctx: ExecutionContext): Promise<Response> {
    return Response.json({ ok: false, error: "not_found" }, { status: 404 });
  },
} satisfies ExportedHandler<Env>;
