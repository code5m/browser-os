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
  // termId 设置前的 PTY 输出缓存：termSpawnChannel 异步返回前 shell 已开始输出，
  // 若直接丢弃会导致终端空白/无提示符。缓存后 termId 就绪时一次性写入。
  const termBuffer: string[] = [];
  // ===== M3.c（WBS M3-4 · E1）终端临时历史 =====
  // 语义：最近 N 条**输出块**的内存快照，供终端面板**重建**时回放（切 Dock 到文件再
  // 切回、切模块视图——面板卸载即丢，但后端 PTY 会话还活着，不回放就是一片空白）。
  // 与 xterm `scrollback` 是两件事：scrollback 管"能往上翻多少行"，这里管"重建时能补
  // 回多少块"（fileterm `TEMPORARY_HISTORY_LIMIT=40` 的同名概念）。
  //
  // 红线：仅会话内内存持有——不落盘、不进 `audit.json`、不写 debug 日志正文，
  // 会话结束（killShell / 重启）即清空，进程退出随内存一起消亡。
  const TERM_TEMP_HISTORY_LIMIT = 40;
  const termHistory: string[] = [];
  function pushTermHistory(data: string) {
    if (!data) return;
    termHistory.push(data);
    // 超出上限丢最旧的：数组长度恒定 ≤ 40，不会随输出量无限增长。
    if (termHistory.length > TERM_TEMP_HISTORY_LIMIT) {
      termHistory.splice(0, termHistory.length - TERM_TEMP_HISTORY_LIMIT);
    }
  }
  function clearTermHistory() {
    termHistory.length = 0;
  }
  /** 面板重建后把历史补写给新的 xterm。回放本身不写回历史（避免自喂导致翻倍）。 */
  function replayTermHistory() {
    if (!termWriter) return;
    for (const data of termHistory) termWriter(data);
  }
  // M3.a 丢弃统计（F2）：队列满时后端丢弃的字节数，仅展示不阻断。
  const droppedChunks = ref(0);
  const droppedBytes = ref(0);
  function resetDroppedStats() {
    droppedChunks.value = 0;
    droppedBytes.value = 0;
  }

  function bindTermWriter(fn: ((data: string) => void) | null) {
    termWriter = fn;
  }

  function bindM0ThroughputStart(fn: (() => void) | null) {
    m0ThroughputStart = fn;
  }

  async function startShell(force = false) {
    if (termId.value && !force) return;
    if (termId.value) await bridge.termKill(termId.value);
    // 新会话：旧会话的输出不得回放到新终端（否则会看到上一段会话的残影）。
    clearTermHistory();
    try {
      // M3.a：每终端独立 Channel 单播（F4），替代全局 `term-data` 事件广播。
      const ch = bridge.createTermChannel((msg) => onTermChannelMsg(msg));
      const r = await bridge.termSpawnChannel(ch);
      termId.value = r.id;
      termLines.value = [];
      resetDroppedStats();
      termWriter?.("$ 终端已就绪（xterm.js + PTY · Channel）\r\n");
      // 写入缓存的早期输出（shell 欢迎信息/提示符）
      for (const data of termBuffer) termWriter?.(data);
      termBuffer.length = 0;
      maybeRunM0Throughput();
    } catch (e: any) {
      termWriter?.("❌ 终端启动失败: " + (e?.message ?? e) + "\r\n");
    }
  }

  // M0-0.b 终端吞吐（契约 §6.3）：term-throughput 模式下 shell 就绪后提交固定
  // 10 MiB ASCII 负载（含唯一 begin/end 标记）；计时起点为命令发送时刻。
  function maybeRunM0Throughput() {
    if (m0Cfg.value?.driver !== "term-throughput" || !termId.value) return;
    setTimeout(() => {
      if (!termId.value || !m0ThroughputStart) return;
      m0ThroughputStart();
      m0StartTs.value = Date.now();
      bridge
        .termWrite(
          termId.value,
          "stty -echo; p='__M0_TERM_'; printf '%s' \"${p}BEGIN__\"; head -c 10485760 /dev/zero | tr '\\0' 'x'; printf '%s' \"${p}END__\"; stty echo\n"
        )
        .catch(() => {});
    }, 1500);
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
    // 会话结束，临时历史随之消亡（不保留到下一个会话）。
    clearTermHistory();
  }

  function toggleTerminal() {
    terminalOpen.value = !terminalOpen.value;
    if (terminalOpen.value) nextTick(() => startShell());
  }

  function onTermData(d: { id: string; data: string }) {
    if (d.id === termId.value) {
      pushTermHistory(d.data);
      termWriter?.(d.data);
    } else if (!termId.value) {
      // termId 还没设置（异步返回前），缓存输出待启动后写入
      pushTermHistory(d.data);
      termBuffer.push(d.data);
    }
  }

  // M3.a Channel 回调：按 `kind` 分发（data → xterm；flow → 丢弃统计；exit → 结束提示）。
  // 兼容：Event sink（`term_spawn`，仅 M0 内部驱动）不带 `kind`，按 data 处理。
  function onTermChannelMsg(msg: TermMessage) {
    const data = msg.data ?? "";
    if (msg.kind === "flow") {
      droppedChunks.value += msg.dropped_chunks ?? 0;
      droppedBytes.value += msg.dropped_bytes ?? 0;
      return;
    }
    if (msg.kind === "exit") {
      if (msg.id === termId.value || !termId.value) termWriter?.(data);
      return;
    }
    if (msg.id === termId.value) {
      pushTermHistory(data);
      termWriter?.(data);
    } else if (!termId.value) {
      // termId 还没设置（termSpawnChannel 异步返回前），缓存输出待启动后写入
      pushTermHistory(data);
      termBuffer.push(data);
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
    termId,
    termLines,
    termHistory,
    droppedChunks,
    droppedBytes,
    resetDroppedStats,
    replayTermHistory,
    clearTermHistory,
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
    startShell,
    termKeydown,
    termWrite,
    killShell,
    toggleTerminal,
    onTermData,
    onTermChannelMsg,
    bindTermWriter,
  };
});
