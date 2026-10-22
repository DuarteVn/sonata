import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  NoteChangedEvent,
  NoteConflictEvent,
  PreviewEvent,
  TerminalStateEvent,
  WorkspaceSummary,
} from "./types";

export type EventMap = {
  "workspaces://changed": WorkspaceSummary[];
  "terminal://state": TerminalStateEvent;
  "terminal://preview": PreviewEvent;
  "note://changed": NoteChangedEvent;
  "note://conflict": NoteConflictEvent;
  "persistence://warning": { message: string };
  "app://quit-requested": null;
  "app://close-selected": null;
};

export function onEvent<K extends keyof EventMap>(
  name: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(name, (e) => handler(e.payload));
}
