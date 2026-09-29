// Clipboard 窄端口（PHASE 4）：包不直接 import bridge / useLayoutStore / shared/ui / utils/redact。
// Host 在 bootstrap 通过 app.provide(CLIPBOARD_PORTS_KEY, ...) 注入具体实现。
import type { InjectionKey, Component } from "vue";

export interface ClipboardNativePort {
  clipboardRead(): Promise<string>;
  clipboardWrite(text: string): Promise<void>;
}

export interface ClipboardUiPort {
  showToast(message: string): void;
  // 关闭剪贴板面板：clipOpen 归 Host useLayoutStore（视图态，非本能力域）。
  requestClose(): void;
  // 红密：纯函数，由 Host 注入其实现，避免包直连 utils/redact。
  redactSecrets(text: string): string;
}

export interface ClipboardPorts {
  native: ClipboardNativePort;
  ui: ClipboardUiPort;
}

export const CLIPBOARD_PORTS_KEY: InjectionKey<ClipboardPorts> = Symbol(
  "clipboardPorts",
);

export function assertClipboardPorts(ports: unknown): asserts ports is ClipboardPorts {
  if (!ports || typeof ports !== "object") {
    throw new Error("[clipboard] CLIPBOARD_PORTS_KEY 未提供（Host 必须注入端口）");
  }
  const p = ports as Partial<ClipboardPorts>;
  if (!p.native || typeof p.native.clipboardRead !== "function" || typeof p.native.clipboardWrite !== "function") {
    throw new Error("[clipboard] ClipboardNativePort 不完整");
  }
  if (!p.ui || typeof p.ui.showToast !== "function" || typeof p.ui.requestClose !== "function" ||
      typeof p.ui.redactSecrets !== "function") {
    throw new Error("[clipboard] ClipboardUiPort 不完整");
  }
}
