import { create } from "zustand";
import type { ReportImageMime } from "../ipc/types";

// The Report Bug draft (data-model.md "Draft"): kept in memory for the app session so closing
// the dialog loses nothing (FR-019); cleared after a successful send; never written to disk.

export type DraftImage = { bytes: Uint8Array; mime: ReportImageMime };

export type Draft = {
  name: string;
  description: string;
  image: DraftImage | null;
  includeLogs: boolean;
};

const empty: Draft = { name: "", description: "", image: null, includeLogs: false };

export const useDraft = create<Draft>(() => ({ ...empty }));

export const getDraft = (): Draft => useDraft.getState();
export const setDraft = (patch: Partial<Draft>): void => useDraft.setState(patch);
export const clearDraft = (): void => useDraft.setState({ ...empty }, true);
export const subscribeDraft = useDraft.subscribe;
