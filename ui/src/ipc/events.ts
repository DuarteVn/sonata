import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AskEvent,
  LaneStatus,
  NodesChangedEvent,
  NotifyEvent,
  NoteChangedEvent,
  NoteConflictEvent,
  PreviewEvent,
  TerminalStateEvent,
  UpdateState,
  UsageState,
  UsageWarning,
  ConsentRequest,
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
  "device://state": import("./types").DeviceStateEvent;
  "messaging://notify": NotifyEvent;
  "workspace://nodes-changed": NodesChangedEvent;
  "update://state": UpdateState;
  "app://check-updates": null;
  "usage://state": UsageState;
  "usage://warning": UsageWarning;
  "usage://consent": ConsentRequest;
  "lane://status": LaneStatus;
  "lane://confirm": import("./types").LaneConfirmEvent;
  "lane://confirm-closed": { requestId: string };
  "kimi://integration": import("./types").KimiIntegration;
  "settings://section": string;
};

export function onEvent<K extends keyof EventMap>(
  name: K,
  handler: (payload: EventMap[K]) => void,
): Promise<UnlistenFn> {
  return listen<EventMap[K]>(name, (e) => handler(e.payload));
}
