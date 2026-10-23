import { create } from "zustand";
import { ipc } from "../ipc/commands";
import type { UpdateState } from "../ipc/types";

// Mirrors core's update state (feature 005); core sends every change as `update://state`.
type UpdateStore = {
  state: UpdateState;
  set: (state: UpdateState) => void;
  load: () => Promise<void>;
};

export const useUpdate = create<UpdateStore>((set) => ({
  state: { kind: "idle" },
  set: (state) => set({ state }),
  load: async () => {
    try {
      set({ state: await ipc.updateState() });
    } catch {
      // No updater (tests, dev builds without the command): keep the banner hidden.
    }
  },
}));

/** True while the banner, or the dot on the show-sidebar button, is visible. */
export function updatePending(state: UpdateState): boolean {
  return state.kind !== "idle" && state.kind !== "dismissed";
}
