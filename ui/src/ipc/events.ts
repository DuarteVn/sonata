import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AskEvent,
  NodesChangedEvent,
  NotifyEvent,
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
  "app://toggle-sidebar": null;
  "messaging://ask": AskEvent;
  "browser://state": import("./types").BrowserState;
  "messaging://notify": NotifyEvent;
  "workspace://nodes-changed": NodesChangedEvent;
};

export function onEvent<K extends keyof EventMap>(
  name: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(name, (e) => handler(e.payload));
}
