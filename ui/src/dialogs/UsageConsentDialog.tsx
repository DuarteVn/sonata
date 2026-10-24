import { useState } from "react";
import { ipc } from "../ipc/commands";
import type { ConsentRequest } from "../ipc/types";
import { useUsage } from "../state/usage";
import { shellQuote } from "../usage/meter";
import { Modal } from "./Modal";

type Props = { request: ConsentRequest; onDone: () => void };

// Asked once per agent before Sonata runs its usage command (FR-012, FR-019).
export function UsageConsentDialog({ request, onDone }: Props) {
  const [error, setError] = useState<string | null>(null);
  const answer = async (allow: boolean) => {
    try {
      const state = await ipc.usageAnswerConsent(request.providerId, allow, request.command);
      if (state) useUsage.getState().set(state);
      onDone();
    } catch (err) {
      setError(String((err as Error)?.message ?? err));
    }
  };
  return (
    <Modal title={`Read ${request.providerName} usage?`} onClose={() => void answer(false)}>
      <p className="dialog-body">{request.explanation}</p>
      <p className="dialog-label">{request.kind === "custom" ? "Command from your definition file:" : "Command:"}</p>
      <pre className="consent-command">{shellQuote(request.command)}</pre>
      <p className="dialog-note">You can change this later in Settings → Usage.</p>
      {error && <p className="error">{error}</p>}
      <div className="actions">
        <button type="button" onClick={() => void answer(false)}>
          Don't allow
        </button>
        <button type="button" className="primary" onClick={() => void answer(true)}>
          Allow
        </button>
      </div>
    </Modal>
  );
}
