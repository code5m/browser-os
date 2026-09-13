import { defineStore } from "pinia";
import { ref, reactive, computed } from "vue";
import { bridge, type M0Config } from "../bridge";
import type { TermMessage } from "../types";
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

  // ===== 终端（真实 PTY + xterm.js · 多实例宫格） =====
  const terminalOpen = ref(false);
  // 多终端实例：每个 = 一个独立 PTY。宫格模式时并排显示多个。
  const termPanes = ref<{ id: string; cwd?: string }[]>([]);
  const termGrid = ref(false); // 宫格模式（多终端并排）
  const termGridCount = ref(4); // 宫格数（默认 4 = 2×2）
  const activeTermId = ref(""); // 当前聚焦的终端
  // per-pane 运行时状态（替代旧的单 termId 全局态）
  const termWriters = new Map<string, (data: string) => void>();
  const termHistories = new Map<string, string[]>();
  const termBuffers = new Map<string, string[]>();
  const TERM_TEMP_HISTORY_LIMIT = 40;

  // M0-0.b 测量配置（契约 §6.3）：term-throughput 模式下 startShell 后自动驱动 10 MiB 负载
  const m0Cfg = ref<M0Config | null>(null);
  // 前端发送吞吐负载的时刻（epoch ms），随 m0_term_report 上报用于计算 elapsed
  const m0StartTs = ref(0);
  let m0ThroughputStart: (() => void) | null = null;
  async function loadM0Config() {
    try {
      m0Cfg.value = await bridge.m0Config();
    } catch {
      m0Cfg.value = null;
    }
  }
  // ===== M3.c（WBS M3-4 · E1）终端临时历史（per-pane） =====
  // 语义：最近 N 条**输出块**的内存快照，供终端面板**重建**时回放（切 Dock 到文件再
  // 切回、切模块视图——面板卸载即丢，但后端 PTY 会话还活着，不回放就是一片空白）。
  // 与 xterm `scrollback` 是两件事：scrollback 管"能往上翻多少行"，这里管"重建时能补
  // 回多少块"（fileterm `TEMPORARY_HISTORY_LIMIT=40` 的同名概念）。
  //
  // 红线：仅会话内内存持有——不落盘、不进 `audit.json`、不写 debug 日志正文，
  // 会话结束（killTerm / 重启）即清空，进程退出随内存一起消亡。
  function pushTermHistory(id: string, data: string) {
    if (!data) return;
    const h = termHistories.get(id) ?? [];
    h.push(data);
    // 超出上限丢最旧的：数组长度恒定 ≤ 40，不会随输出量无限增长。
    if (h.length > TERM_TEMP_HISTORY_LIMIT) {
      h.splice(0, h.length - TERM_TEMP_HISTORY_LIMIT);
    }
    termHistories.set(id, h);
  }
  function clearTermHistory(id: string) {
    termHistories.set(id, []);
  }
  /** 面板重建后把历史 + 早期缓冲补写给该 pane 的 xterm。 */
  function replayTermHistory(id: string) {
    const w = termWriters.get(id);
    if (!w) return;
    for (const d of termHistories.get(id) ?? []) w(d);
    for (const d of termBuffers.get(id) ?? []) w(d);
    termBuffers.set(id, []);
  }
  // M3.a 丢弃统计（F2）：队列满时后端丢弃的字节数，仅展示不阻断。
  const droppedChunks = ref(0);
  const droppedBytes = ref(0);
  function resetDroppedStats() {
    droppedChunks.value = 0;
    droppedBytes.value = 0;
  }

  function bindTermWriter(id: string, fn: ((data: string) => void) | null) {
    if (fn) termWriters.set(id, fn);
    else termWriters.delete(id);
  }

  function bindM0ThroughputStart(fn: (() => void) | null) {
    m0ThroughputStart = fn;
  }

  // 新建一个终端实例（PTY）。cwd 可选，传入则打开后 cd 到该目录。
  async function spawnTerm(cwd?: string): Promise<string> {
    try {
      // M3.a：每终端独立 Channel 单播（F4），替代全局 `term-data` 事件广播。
      const ch = bridge.createTermChannel((msg) => onTermChannelMsg(msg));
      const r = await bridge.termSpawnChannel(ch);
      const id = r.id;
      termPanes.value.push({ id, cwd });
      termHistories.set(id, []);
      termBuffers.set(id, []);
      if (!activeTermId.value) activeTermId.value = id;
      resetDroppedStats();
      if (cwd) await bridge.termWrite(id, "cd " + cwd + "\n").catch(() => {});
      maybeRunM0Throughput(id);
      return id;
    } catch (e: any) {
      layout.showToast("终端启动失败: " + (e?.message ?? e));
      return "";
    }
  }
  // 确保终端视图至少有一个实例（切到终端视图时调用）
  async function ensureTerm() {
    if (termPanes.value.length === 0) await spawnTerm();
  }
  function addTermPane() {
    spawnTerm();
  }
  async function killTerm(id: string) {
    await bridge.termKill(id).catch(() => {});
    termPanes.value = termPanes.value.filter((p) => p.id !== id);
    termWriters.delete(id);
    termHistories.delete(id);
    termBuffers.delete(id);
    if (activeTermId.value === id) activeTermId.value = termPanes.value[0]?.id ?? "";
  }

  // M0-0.b 终端吞吐（契约 §6.3）：term-throughput 模式下 shell 就绪后提交固定
  // 10 MiB ASCII 负载（含唯一 begin/end 标记）；计时起点为命令发送时刻。
  function maybeRunM0Throughput(id: string) {
    if (m0Cfg.value?.driver !== "term-throughput" || !id) return;
    setTimeout(() => {
      if (!m0ThroughputStart) return;
      m0ThroughputStart();
      m0StartTs.value = Date.now();
      bridge
        .termWrite(
          id,
          "stty -echo; p='__M0_TERM_'; printf '%s' \"${p}BEGIN__\"; head -c 10485760 /dev/zero | tr '\\0' 'x'; printf '%s' \"${p}END__\"; stty echo\n"
        )
        .catch(() => {});
    }, 1500);
  }

  async function termKeydown(e: KeyboardEvent, id?: string) {
    const tid = id ?? activeTermId.value;
    if (!tid) return;
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
    await bridge.termWrite(tid, data);
  }

  async function termWrite(id: string, data: string) {
    if (id) await bridge.termWrite(id, data);
  }

  function toggleTerminal() {
    terminalOpen.value = !terminalOpen.value;
    if (terminalOpen.value && termPanes.value.length === 0) spawnTerm();
  }
  function setTermGrid(on: boolean) {
    termGrid.value = on;
  }
  function setTermGridCount(n: number) {
    termGridCount.value = Math.min(9, Math.max(1, Math.round(n)));
  }
  function setActiveTerm(id: string) {
    activeTermId.value = id;
  }
  // 文件树右键"命令行终端打开"：切到终端视图，新建实例并 cd 到目标目录
  async function openTerminalAt(dir: string) {
    layout.setView("term");
    await spawnTerm(dir);
  }

  function onTermData(d: { id: string; data: string }) {
    const w = termWriters.get(d.id);
    if (w) {
      pushTermHistory(d.id, d.data);
      w(d.data);
    } else {
      const b = termBuffers.get(d.id) ?? [];
      b.push(d.data);
      termBuffers.set(d.id, b);
    }
  }

  // M3.a Channel 回调：按 `kind` 分发（data → xterm；flow → 丢弃统计；exit → 结束提示）。
  // 兼容：Event sink（`term_spawn`，仅 M0 内部驱动）不带 `kind`，按 data 处理。
  function onTermChannelMsg(msg: TermMessage) {
    const id = msg.id;
    const data = msg.data ?? "";
    if (msg.kind === "flow") {
      droppedChunks.value += msg.dropped_chunks ?? 0;
      droppedBytes.value += msg.dropped_bytes ?? 0;
      return;
    }
    if (msg.kind === "exit") {
      if (termWriters.has(id)) termWriters.get(id)!(data);
      return;
    }
    const w = termWriters.get(id);
    if (w) {
      pushTermHistory(id, data);
      w(data);
    } else {
      const b = termBuffers.get(id) ?? [];
      b.push(data);
      termBuffers.set(id, b);
    }
  }

  return {
    clipText,
    clipHistory,
    apps,
    appFilter,
    brokenIcons,
    filteredApps,
    terminalOpen,
    termPanes,
    termGrid,
    termGridCount,
    activeTermId,
    droppedChunks,
    droppedBytes,
    resetDroppedStats,
    m0Cfg,
    m0StartTs,
    loadM0Config,
    bindM0ThroughputStart,
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
    spawnTerm,
    ensureTerm,
    addTermPane,
    termKeydown,
    termWrite,
    killTerm,
    setTermGrid,
    setTermGridCount,
    setActiveTerm,
    toggleTerminal,
    onTermData,
    onTermChannelMsg,
    openTerminalAt,
    bindTermWriter,
    replayTermHistory,
    clearTermHistory,
  };
});
