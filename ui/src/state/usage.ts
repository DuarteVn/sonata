import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type { ConsentRequest, UsageState } from "../ipc/types";

// Mirrors core's usage state (feature 006); core sends every change as `usage://state`.
type UsageStore = {
  state: UsageState | null;
  /** Consent requests waiting for an answer, oldest first. */
  consents: ConsentRequest[];
  set: (state: UsageState) => void;
  load: () => Promise<void>;
  askConsent: (request: ConsentRequest) => void;
  dropConsent: (providerId: string) => void;
};

export const useUsage = create<UsageStore>((set) => ({
  state: null,
  consents: [],
  // An answer given in another window settles the request here too.
  set: (state) =>
    set((s) => ({
      state,
      consents: s.consents.filter((c) => state.providers.some((p) => p.id === c.providerId && p.consent === "unasked")),
    })),
  load: async () => {
    try {
      set({ state: await ipc.usageState() });
    } catch {
      // No usage commands (tests): keep the button hidden.
    }
  },
  askConsent: (request) =>
    set((s) => ({
      consents: [...s.consents.filter((c) => c.providerId !== request.providerId), request],
    })),
  dropConsent: (providerId) => set((s) => ({ consents: s.consents.filter((c) => c.providerId !== providerId) })),
}));
