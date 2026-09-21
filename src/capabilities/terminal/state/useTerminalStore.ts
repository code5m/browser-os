// Terminal Capability — Domain Owner（Phase 8E / Train D）
//
// Terminal 域的**唯一 owner**：PTY 生命周期（spawn/write/resize/kill）、面板注册表、
// 宫格布局参数、聚焦 id、CLI 自动确认偏好、对账 probe 开关、M0 吞吐测量配置与计时、
// 背压丢弃遥测、会话内临时历史。
//
// 语义来源：Phase 4 Terminal Lifecycle（INV-4-1 termPanes 唯一面板注册表；
// INV-4-2 activeTermId 必须 ∈ termPanes.id）+ M3.c（临时历史 40 条、仅会话内存）。
// 本次迁移（SCR-20260920-terminal-owner-extraction）**零语义变更**，只把归属从
// useSystemStore（Terminal+Clipboard+Apps 混居）换成专属 owner，并落到能力包内。
//
// 边界（硬约束）：
//   - Clipboard / Apps **不属**本 owner，留在 src/stores/useSystemStore.ts（Debt-8E-1）。
//   - mainView === "term" 的视图导航属 useLayoutStore（本文件只消费，不拥有）。
//   - 不落盘：除 `terminal-auto-confirm-cli` UI 偏好外，终端输出/历史绝不写 localStorage / 审计。

import { defineStore } from "pinia";
import { ref } from "vue";
import { bridge, type M0Config } from "../../../bridge";
import type { TermMessage } from "../../../types";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { isTerminalResourceAllowed } from "../resource/guard";

export const useTerminalStore = defineStore("terminal", () => {
  const layout = useLayoutStore();

  // ===== 终端（真实 PTY + xterm.js · 多实例宫格） =====
  const terminalOpen = ref(false);
  // 多终端实例：每个 = 一个独立 PTY。宫格模式时并排显示多个。
  const termPanes = ref<{ id: string; cwd?: string }[]>([]);
  const termGrid = ref(false); // 宫格模式（多终端并排）
  const termGridCount = ref(4); // 宫格数（默认 4 = 2×2）
  const activeTermId = ref(""); // 当前聚焦的终端
  const autoConfirmCli = ref(localStorage.getItem("terminal-auto-confirm-cli") !== "0");
  const confirmScans = new Map<string, string>();
  const lastAutoConfirmAt = new Map<string, number>();
  // 终端对账 probe（默认关闭）：开关由后端按 MVP_TERMINAL_PROBE=1 经 spawn 响应下发。
  // 只记录元数据（序号/UTF-8 字节数/累计/去向），绝不记录终端正文。
  const termProbeOn = ref(false);
  const probeRecv = new Map<string, { seq: number; bytes: number }>();
  const probeEncoder = new TextEncoder();
  function probeFeRecv(id: string, data: string, sink: "xterm" | "buffer" | "exit") {
    if (!termProbeOn.value) return;
    const s = probeRecv.get(id) ?? { seq: 0, bytes: 0 };
    s.seq += 1;
    const b = probeEncoder.encode(data).length;
    s.bytes += b;
    probeRecv.set(id, s);
    bridge.debugLog(`[TERM_PROBE] fe.recv id=${id} seq=${s.seq} bytes=${b} cum=${s.bytes} sink=${sink}`);
  }
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

  function setAutoConfirmCli(enabled: boolean) {
    autoConfirmCli.value = enabled;
    localStorage.setItem("terminal-auto-confirm-cli", enabled ? "1" : "0");
  }

  function maybeAutoConfirmCli(id: string, data: string) {
    const last = lastAutoConfirmAt.get(id) ?? 0;
    if (!autoConfirmCli.value || !id || Date.now() - last < 3000) return;
    const scan = ((confirmScans.get(id) ?? "") + data.replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, "")).slice(-1600);
    confirmScans.set(id, scan);
    const isCodeArtsCompatibility = scan.includes("兼容性提示") && scan.includes("True Color");
    // CodeArts 26.8 prints this warning unconditionally and continues after 1s.
    // Only an explicit compatibility confirmation may receive input.
    const asksToContinue = /(?:按.*回车.*继续|press.*enter.*continue)/i.test(scan);
    if (!isCodeArtsCompatibility || !asksToContinue) return;
    lastAutoConfirmAt.set(id, Date.now());
    confirmScans.set(id, "");
    window.setTimeout(() => bridge.termWrite(id, "\r").catch(() => {}), 120);
  }

  function bindM0ThroughputStart(fn: (() => void) | null) {
    m0ThroughputStart = fn;
  }

  // 新建一个终端实例（PTY）。cwd 可选，传入则打开后 cd 到该目录。
  async function spawnTerm(cwd?: string): Promise<string> {
    // H-G 同类防护：PTY 是 Terminal capability-owned 重资源，
    // 只能由 Terminal Capability 受控生命周期创建。Terminal absent → 0 PTY。
    // 与 Browser/Grid 的 buildGrid 闸同构（同一份编排真源，同一 fail-closed 口径）。
    if (!isTerminalResourceAllowed()) {
      return "";
    }
    try {
      // M3.a：每终端独立 Channel 单播（F4），替代全局 `term-data` 事件广播。
      const ch = bridge.createTermChannel((msg) => onTermChannelMsg(msg));
      const r = await bridge.termSpawnChannel(ch);
      const id = r.id;
      termProbeOn.value = !!r.probe;
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
  // 确保终端视图至少有一个实例（终端视图组件挂载时调用）
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
    // 会话结束即清空该 pane 的临时历史（禁止跨会话残影），随后移除桶。
    clearTermHistory(id);
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
  // canonical writer（hide）：面板内 ✕ 不得直写 terminalOpen（COMPONENT_WRITES_TERMINAL）。
  // 与 toggleTerminal 同为 terminalOpen 的 canonical writer（见 states.yaml）。
  function closeTerminal() {
    terminalOpen.value = false;
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
      maybeAutoConfirmCli(d.id, d.data);
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
      probeFeRecv(id, data, "exit");
      if (termWriters.has(id)) termWriters.get(id)!(data);
      return;
    }
    const w = termWriters.get(id);
    if (w) {
      probeFeRecv(id, data, "xterm");
      pushTermHistory(id, data);
      w(data);
      maybeAutoConfirmCli(id, data);
    } else {
      probeFeRecv(id, data, "buffer");
      const b = termBuffers.get(id) ?? [];
      b.push(data);
      termBuffers.set(id, b);
    }
  }

  return {
    terminalOpen,
    termPanes,
    termGrid,
    termGridCount,
    activeTermId,
    autoConfirmCli,
    termProbeOn,
    droppedChunks,
    droppedBytes,
    resetDroppedStats,
    m0Cfg,
    m0StartTs,
    loadM0Config,
    bindM0ThroughputStart,
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
    closeTerminal,
    onTermData,
    onTermChannelMsg,
    openTerminalAt,
    bindTermWriter,
    setAutoConfirmCli,
    replayTermHistory,
    clearTermHistory,
  };
});
