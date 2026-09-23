// src/capabilities/clipboard/state/useClipboardStore.ts
// Clipboard 能力语义 owner（Capability Library Expansion v1，STAGE H）。
//
// 从 src/stores/useSystemStore.ts 拆出（解除 Debt-8E-1：Clipboard 与 Apps 曾共居一个 store，
// 二者 owner 同名）。拆分后：Clipboard 域归本 store，Apps 域归 capabilities/apps/state。
//
// 红线（承 B11-1 修复）：
//   - 剪贴板历史**不落盘**：仅会话内存、上限 CLIP_CAP 条；关闭应用即清空，绝不写 localStorage / 磁盘。
//   - 原生调用一律经 bridge.ts；事件驱动（无轮询）。
import { defineStore } from "pinia";
import { reactive, ref } from "vue";
import { bridge } from "../../../bridge";
import { useLayoutStore } from "../../../stores/useLayoutStore";

export interface ClipItem {
  text: string;
  at: number;
}

export const useClipboardStore = defineStore("clipboard", () => {
  const layout = useLayoutStore();

  // ===== 剪贴板（事件驱动，无轮询） =====
  const clipText = ref("");
  const clipHistory = reactive<ClipItem[]>([]);
  const CLIP_CAP = 30;
  let clipFocusBound = false;

  // 历史不跨重启留存：保留为无副作用函数，供 App.vue 启动期调用而无需改动其调用点。
  function loadClipHistory() {
    /* intentionally no-op：不读取 localStorage，避免明文剪贴板历史落盘 */
  }
  // 仅做内存上限裁剪，绝不调用 localStorage.setItem（B11-1 红线）。
  function saveClipHistory() {
    if (clipHistory.length > CLIP_CAP) clipHistory.splice(CLIP_CAP);
  }
  async function clipReadSilent() {
    try {
      const t = await bridge.clipboardRead();
      if (!t || t === clipText.value) return;
      clipText.value = t;
      const idx = clipHistory.findIndex((c) => c.text === t);
      if (idx >= 0) clipHistory.splice(idx, 1);
      clipHistory.unshift({ text: t, at: Date.now() });
      saveClipHistory();
    } catch {}
  }
  async function clipCopy() {
    if (!clipText.value.trim()) {
      layout.showToast("没有可复制的内容");
      return;
    }
    await bridge.clipboardWrite(clipText.value);
    const idx = clipHistory.findIndex((c) => c.text === clipText.value);
    if (idx >= 0) clipHistory.splice(idx, 1);
    clipHistory.unshift({ text: clipText.value, at: Date.now() });
    saveClipHistory();
    layout.showToast("已复制到系统剪贴板");
  }
  async function clipPaste() {
    try {
      clipText.value = await bridge.clipboardRead();
      layout.showToast("已从剪贴板粘贴");
    } catch (e: any) {
      layout.showToast("读取剪贴板失败: " + (e?.message ?? e));
    }
  }
  function useClipItem(item: ClipItem) {
    clipText.value = item.text;
  }
  async function copyClipItem(item: ClipItem) {
    clipText.value = item.text;
    await bridge.clipboardWrite(item.text);
    const idx = clipHistory.findIndex((c) => c.text === item.text);
    if (idx >= 0) clipHistory.splice(idx, 1);
    clipHistory.unshift({ text: item.text, at: Date.now() });
    saveClipHistory();
    layout.showToast("已复制到系统剪贴板");
  }
  function clearClipHistory() {
    clipHistory.splice(0, clipHistory.length);
    saveClipHistory();
  }
  function bindClipFocus() {
    if (clipFocusBound) return;
    clipFocusBound = true;
    try {
      (window as any).__TAURI_INTERNALS__ &&
        (window as any).__TAURI__?.event?.listen("tauri://focus", () => clipReadSilent());
    } catch {}
    window.addEventListener("focus", () => clipReadSilent());
  }
  function startClipWatch() {
    bindClipFocus();
    clipReadSilent();
  }

  return {
    clipText,
    clipHistory,
    loadClipHistory,
    clipReadSilent,
    clipCopy,
    clipPaste,
    useClipItem,
    copyClipItem,
    clearClipHistory,
    startClipWatch,
  };
});
