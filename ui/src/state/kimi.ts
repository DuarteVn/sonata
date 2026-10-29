import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type { KimiIntegration, KimiNoticeDismissed } from "../ipc/types";

// Kimi integration status for the terminal notice (feature 010); core sends changes as
// `kimi://integration`.
type KimiStore = {
  integration: KimiIntegration | null;
  /** Status the user dismissed the notice for (preferences). */
  dismissed: KimiNoticeDismissed | undefined;
  loading: boolean;
  set: (integration: KimiIntegration) => void;
  load: () => Promise<void>;
  dismiss: (status: KimiNoticeDismissed) => Promise<void>;
};

export const useKimi = create<KimiStore>((set, get) => ({
  integration: null,
  dismissed: undefined,
  loading: false,
  set: (integration) => set({ integration }),
  load: async () => {
    if (get().integration || get().loading) return;
    set({ loading: true });
    try {
      set({ integration: await ipc.kimiIntegration() });
    } catch {
      // No Kimi commands (tests, older shell): no notice.
    } finally {
      set({ loading: false });
    }
  },
  dismiss: async (status) => {
    set({ dismissed: status });
    await ipc.dismissKimiNotice(status);
  },
}));
