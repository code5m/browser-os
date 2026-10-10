// BrowserOS Shell shortcut transport: adapter boundary keeps App.vue free of native APIs.
// Browser-tabs plugin owns GTK keyboard capture; this module owns the event subscription.
import { bridge } from "../bridge";

export type ChildShellShortcut = { id: string; action: string };

export function onChildShellShortcut(cb: (event: ChildShellShortcut) => void): Promise<() => void> {
  return bridge.onChildShellShortcut(cb);
}
