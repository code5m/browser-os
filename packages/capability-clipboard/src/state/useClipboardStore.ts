import { defineStore } from "pinia";
import { ref, reactive, inject } from "vue";
import { CLIPBOARD_PORTS_KEY, assertClipboardPorts } from "../ports";
import { registerClipboardLifecycleBinding } from "../lifecycle";

export interface ClipItem {
  id: string;
  text: string;
  ts: number;
}

const CLIP_CAP = 30;

// B11-1 红线（冻结）：剪贴板内容不落盘。
// - loadClipHistory 不读取浏览器持久化 API 或文件系统。
// - saveClipHistory 不写入任何持久化层，仅内存上限裁剪。
export const useClipboardStore = defineStore("clipboard", () => {
  const clipText = ref("");
  const clipHistory = reactive<ClipItem[]>([]);

  // 宿主端口：Clipboard 包不直连 bridge / useLayoutStore（包边界 PHASE 3）。
  // 端口经 Host 在 bootstrap 注入（app.provide(CLIPBOARD_PORTS_KEY, ...)）。
  const injected = inject(CLIPBOARD_PORTS_KEY);
  assertClipboardPorts(injected);
  const ports = injected;

  function loadClipHistory() {
    // B11-1：剪贴板历史不写入任何持久化层。此处为无副作用空函数，保留签名供 App.vue 调用。
  }

  function saveClipHistory() {
    // B11-1：仅内存上限裁剪，绝不触碰持久化层、文件系统或 package cache。
    if (clipHistory.length > CLIP_CAP) clipHistory.splice(CLIP_CAP);
  }

  async function clipReadSilent() {
    try {
      const text = await ports.native.clipboardRead();
      if (text && text.length > 0) {
        clipText.value = text;
        const exists = clipHistory.find((i) => i.text === text);
        if (!exists) {
          clipHistory.unshift({ id: `clip-${Date.now()}`, text, ts: Date.now() });
          saveClipHistory();
        }
      }
    } catch (e) {
      // 读取失败静默忽略，不污染历史
    }
  }

  async function clipCopy(text?: string) {
    const value = text ?? clipText.value;
    if (!value) return;
    try {
      await ports.native.clipboardWrite(value);
      clipText.value = value;
      const exists = clipHistory.find((i) => i.text === value);
      if (!exists) {
        clipHistory.unshift({ id: `clip-${Date.now()}`, text: value, ts: Date.now() });
        saveClipHistory();
      }
      ports.ui.showToast("已复制到剪贴板");
    } catch (e) {
      ports.ui.showToast("复制失败");
    }
  }

  async function clipPaste() {
    try {
      const text = await ports.native.clipboardRead();
      if (text) {
        clipText.value = text;
        ports.ui.showToast("已粘贴到剪贴板");
      }
    } catch (e) {
      ports.ui.showToast("粘贴失败");
    }
  }

  function useClipItem(item: ClipItem) {
    clipText.value = item.text;
  }

  async function copyClipItem(item: ClipItem) {
    await clipCopy(item.text);
  }

  function clearClipHistory() {
    clipHistory.splice(0, clipHistory.length);
  }

  let focusHandler: (() => void) | null = null;
  let tauriUnlisten: (() => void) | null = null;
  let tauriBindSeq = 0;
  let lifecycleUnregister: (() => void) | null = null;

  async function bindClipFocus() {
    if (focusHandler) return;
    const seq = ++tauriBindSeq;
    focusHandler = () => {
      void clipReadSilent();
    };
    window.addEventListener("focus", focusHandler);
    try {
      // @ts-ignore tauri focus 事件（HMR/Node 场景可能未注入）
      const unlisten = await Promise.resolve(window.__TAURI__.event.listen("tauri://focus", () => void clipReadSilent()));
      // suspend/disable 可能发生在异步 listen resolve 之前；晚到的 unlisten 必须立即执行。
      if (seq !== tauriBindSeq || !focusHandler) {
        if (typeof unlisten === "function") unlisten();
        return;
      }
      if (typeof unlisten === "function") tauriUnlisten = unlisten;
    } catch (e) {
      // 非 Tauri 环境（如 node 校验）静默跳过
    }
  }

  function unbindClipFocus() {
    tauriBindSeq += 1;
    if (focusHandler) window.removeEventListener("focus", focusHandler);
    focusHandler = null;
    const unlisten = tauriUnlisten;
    tauriUnlisten = null;
    if (unlisten) unlisten();
  }

  function startClipWatch() {
    if (lifecycleUnregister) return;
    lifecycleUnregister = registerClipboardLifecycleBinding({
      start: () => void bindClipFocus(),
      stop: unbindClipFocus,
    });
  }

  function stopClipWatch() {
    const unregister = lifecycleUnregister;
    lifecycleUnregister = null;
    unregister?.();
  }

  return {
    clipText,
    clipHistory,
    loadClipHistory,
    saveClipHistory,
    clipReadSilent,
    clipCopy,
    clipPaste,
    useClipItem,
    copyClipItem,
    clearClipHistory,
    startClipWatch,
    stopClipWatch,
  };
});
