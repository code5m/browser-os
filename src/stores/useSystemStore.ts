// 系统杂项 Owner —— Phase 8E / Train D 之后**只**拥有 Clipboard 与 Apps 两个域。
//
// 历史：本文件曾是 Terminal + Clipboard + Apps 三域混居（Debt-7A-2）。
// Train D（SCR-20260920-terminal-owner-extraction）把 Terminal 域整体抽到
// `src/capabilities/terminal/state/useTerminalStore.ts`，本文件不再声明任何 terminal 状态
// （由 scripts/check-terminal-owners.mjs 机器强制）。
//
// 显式债务 Debt-8E-1：Clipboard 与 Apps 仍共处本文件（二者 owner 同为 useSystemStore）；
// 拆分不属 Train D 范围，未静默处理。

import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { bridge } from "../bridge";
import { useLayoutStore } from "./useLayoutStore";

export interface ClipItem {
  text: string;
  at: number;
}

export const useSystemStore = defineStore("system", () => {
  const layout = useLayoutStore();

  // ===== 剪贴板（事件驱动，无轮询） =====
  // B11-1 修复（P0 明文落盘）：默认**不持久化**剪贴板历史。
  // 历史只保留在会话内存、上限 CLIP_CAP 条；关闭应用即清空，绝不写入 localStorage / 磁盘。
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

  // ===== 系统应用 =====
  const apps = ref<AppEntry[]>([]);
  const appFilter = ref("");
  const brokenIcons = ref<Set<string>>(new Set());
  async function loadApps() {
    try {
      apps.value = await bridge.listApps();
      brokenIcons.value.clear();
    } catch (e: any) {
      layout.showToast("读取应用列表失败: " + (e?.message ?? e));
    }
  }
  const filteredApps = computed(() => {
    const f = appFilter.value.trim().toLowerCase();
    if (!f) return apps.value;
    return apps.value.filter((a) => a.name.toLowerCase().includes(f));
  });
  async function launchApp(app: AppEntry) {
    try {
      layout.showToast("正在启动: " + app.name);
      await bridge.launchApp(app.exec);
      layout.showToast("已启动: " + app.name);
    } catch (e: any) {
      layout.showToast("启动失败: " + app.name + " - " + (e?.message ?? e));
    }
  }
  function onAppImgError(exec: string) {
    brokenIcons.value.add(exec);
  }

  return {
    clipText,
    clipHistory,
    apps,
    appFilter,
    brokenIcons,
    filteredApps,
    loadClipHistory,
    clipReadSilent,
    clipCopy,
    clipPaste,
    useClipItem,
    copyClipItem,
    clearClipHistory,
    startClipWatch,
    loadApps,
    launchApp,
    onAppImgError,
  };
});
