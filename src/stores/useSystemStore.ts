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
  const clipText = ref("");
  const clipHistory = reactive<ClipItem[]>([]);
  const CLIP_KEY = "browser-os-clipboard";
  let clipFocusBound = false;

  function loadClipHistory() {
    try {
      const raw = localStorage.getItem(CLIP_KEY);
      if (raw) clipHistory.splice(0, clipHistory.length, ...JSON.parse(raw));
    } catch {}
  }
  function saveClipHistory() {
    try {
      localStorage.setItem(CLIP_KEY, JSON.stringify(clipHistory.slice(0, 50)));
    } catch {}
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

  // ===== 终端（真实 PTY + xterm.js） =====
  const terminalOpen = ref(false);
  const termId = ref("");
  const termLines = ref<string[]>([]); // 兼容保留，不再用于渲染
  let termWriter: ((data: string) => void) | null = null;

  function bindTermWriter(fn: ((data: string) => void) | null) {
    termWriter = fn;
  }

  async function startShell(force = false) {
    if (termId.value && !force) return;
    if (termId.value) await bridge.termKill(termId.value);
    try {
      const r = await bridge.termSpawn();
      termId.value = r.id;
      termLines.value = [];
      termWriter?.("$ 终端已就绪（xterm.js + PTY）\r\n");
    } catch (e: any) {
      termWriter?.("❌ 终端启动失败: " + (e?.message ?? e) + "\r\n");
    }
  }

  async function termKeydown(e: KeyboardEvent) {
    if (!termId.value) return;
    let data = "";
    const k = e.key;
    if (k === "Enter") data = "\r";
    else if (k === "Backspace") data = "\x7f";
    else if (k === "Tab") data = "\t";
    else if (k === "ArrowUp") data = "\x1b[A";
    else if (k === "ArrowDown") data = "\x1b[B";
    else if (k === "ArrowRight") data = "\x1b[C";
    else if (k === "ArrowLeft") data = "\x1b[D";
    else if (k === "Escape") data = "\x1b";
    else if (e.ctrlKey && k.toLowerCase() === "c") data = "\x03";
    else if (e.ctrlKey && k.toLowerCase() === "d") data = "\x04";
    else if (e.ctrlKey && k.toLowerCase() === "l") data = "\x0c";
    else if (e.ctrlKey && k.toLowerCase() === "u") data = "\x15";
    else if (k.length === 1) data = k;
    else return;
    e.preventDefault();
    await bridge.termWrite(termId.value, data);
  }

  async function termWrite(data: string) {
    if (termId.value) await bridge.termWrite(termId.value, data);
  }

  async function killShell() {
    if (termId.value) await bridge.termKill(termId.value);
    termId.value = "";
  }

  function toggleTerminal() {
    terminalOpen.value = !terminalOpen.value;
    if (terminalOpen.value) nextTick(() => startShell());
  }

  function onTermData(d: { id: string; data: string }) {
    if (d.id === termId.value) termWriter?.(d.data);
  }

  return {
    clipText,
    clipHistory,
    apps,
    appFilter,
    brokenIcons,
    filteredApps,
    terminalOpen,
    termId,
    termLines,
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
    startShell,
    termKeydown,
    termWrite,
    killShell,
    toggleTerminal,
    onTermData,
    bindTermWriter,
  };
});
