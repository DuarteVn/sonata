import type { FormatAction } from "./markdownEdit";

/** Lets the note toolbar act on a note's editor without owning its state. */
export type NoteEditor = {
  apply: (action: FormatAction) => void;
  insert: (text: string) => void;
};

export const noteEditors = new Map<string, NoteEditor>();
