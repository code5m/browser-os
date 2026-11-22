// Terminal Capability — Domain Owner（Phase 8E / Train D）
//
// Terminal 域的**唯一 owner**：PTY 生命周期（spawn/write/resize/kill）、面板注册表、
// 宫格布局参数、聚焦 id、CLI 自动确认偏好、对账 probe 开关、M0 吞吐测量配置与计时、
// 背压丢弃遥测、会话内临时历史。
//
// 语义来源：Phase 4 Terminal Lifecycle（INV-4-1 termPanes 唯一面板注册表；
// INV-4-2 activeTermId 必须 ∈ termPanes.id）+ M3.c（临时历史 40 条、仅会话内存）。
// owner 最初由 useSystemStore 迁入本能力包。本轮补充启动/关闭状态、隐藏缓冲与
// 重建历史的独立字节/条数预算；输出仍只存在于当前会话内存。
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
import { registerTerminalLifecycleBinding } from "../lifecycle";

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
  const TERM_TEMP_HISTORY_LIMIT = 40;
  const TERM_HISTORY_MAX_BYTES = 256 * 1024;
  const TERM_BUFFER_MAX_CHUNKS = 128;
  const TERM_BUFFER_MAX_BYTES = 128 * 1024;
  const TERM_SESSION_LIMIT = 32;
  type OutputChunk = { text: string; bytes: number };
  type BoundedOutput = {
    chunks: OutputChunk[];
    bytes: number;
    droppedChunks: number;
    droppedBytes: number;
  };
  type OutputSession = {
    phase: "starting" | "active" | "closing" | "closed";
    id: string | null;
    buffer: BoundedOutput;
    history: BoundedOutput;
    exit: TermMessage | null;
    droppedChunks: number;
    droppedBytes: number;
  };
  const termHistories = new Map<string, BoundedOutput>();
  const termBuffers = new Map<string, BoundedOutput>();
  const outputSessions = new Map<string, OutputSession>();
  // A Channel closes over its own startup slot before invoke can deliver data.
  // Closing that slot rejects late callbacks without an ever-growing ID set.
  const startingSessions = new Set<OutputSession>();
  const termReplayNotices = ref<Record<string, string>>({});
  const outputDecoder = new TextDecoder("utf-8", { fatal: true });

  function emptyOutput(): BoundedOutput {
    return { chunks: [], bytes: 0, droppedChunks: 0, droppedBytes: 0 };
  }
  function resetOutput(buffer: BoundedOutput) {
    buffer.chunks = [];
    buffer.bytes = 0;
    buffer.droppedChunks = 0;
    buffer.droppedBytes = 0;
  }
  function saturatingCount(a: number, b: number): number {
    return Math.min(Number.MAX_SAFE_INTEGER, a + b);
  }
  function utf8ByteLength(text: string): number {
    let bytes = 0;
    for (const character of text) {
      const code = character.codePointAt(0)!;
      bytes += code <= 0x7f ? 1 : code <= 0x7ff ? 2 : code <= 0xffff ? 3 : 4;
    }
    return bytes;
  }
  function utf8TailWithin(text: string, limit: number): OutputChunk {
    let start = text.length;
    let bytes = 0;
    while (start > 0) {
      let from = start - 1;
      const code = text.charCodeAt(from);
      let size = code <= 0x7f ? 1 : code <= 0x7ff ? 2 : 3;
      if (code >= 0xdc00 && code <= 0xdfff && from > 0) {
        const previous = text.charCodeAt(from - 1);
        if (previous >= 0xd800 && previous <= 0xdbff) { from -= 1; size = 4; }
      }
      if (bytes + size > limit) break;
      start = from;
      bytes += size;
    }
    // Materialize only the retained suffix: a JS sliced string may otherwise
    // keep the entire oversized parent allocation alive behind a small tail.
    const retained = outputDecoder.decode(probeEncoder.encode(text.slice(start)));
    return { text: retained, bytes };
  }
  function appendOutput(buffer: BoundedOutput, text: string, maxBytes: number, maxChunks: number) {
    if (!text) return;
    const originalBytes = utf8ByteLength(text);
    const chunk = originalBytes > maxBytes
      ? utf8TailWithin(text, maxBytes) : { text, bytes: originalBytes };
    if (chunk.bytes < originalBytes) {
      buffer.droppedBytes = saturatingCount(buffer.droppedBytes, originalBytes - chunk.bytes);
      buffer.droppedChunks = saturatingCount(buffer.droppedChunks, 1);
    }
    while (buffer.chunks.length >= maxChunks || buffer.bytes + chunk.bytes > maxBytes) {
      const old = buffer.chunks.shift();
      if (!old) break;
      buffer.bytes -= old.bytes;
      buffer.droppedBytes = saturatingCount(buffer.droppedBytes, old.bytes);
      buffer.droppedChunks = saturatingCount(buffer.droppedChunks, 1);
    }
    if (chunk.text) { buffer.chunks.push(chunk); buffer.bytes += chunk.bytes; }
  }
  function termBufferNotice(id: string): string {
    return termReplayNotices.value[id] ?? "";
  }
  function updateReplayNotice(id: string, slot: OutputSession) {
    const bytes = saturatingCount(slot.history.droppedBytes, slot.buffer.droppedBytes);
    if (bytes > 0) {
      termReplayNotices.value[id] = "回放内容已截断，已省略较早的 " + bytes + " 字节。";
    } else {
      delete termReplayNotices.value[id];
    }
  }
  function closeOutputSlot(slot: OutputSession) {
    slot.phase = "closed";
    resetOutput(slot.buffer);
    resetOutput(slot.history);
    slot.exit = null;
    slot.droppedChunks = 0;
    slot.droppedBytes = 0;
  }

  // Input delivery belongs to this session owner, including hidden/remounted
  // panes. Bound both payload bytes and request count; never chain an unbounded
  // series of closures retaining pasted text.
  const TERM_INPUT_CHUNK_BYTES = 16 * 1024;
  const TERM_INPUT_PENDING_BYTES = 1024 * 1024;
  const TERM_INPUT_PENDING_REQUESTS = 128;
  const TERM_INPUT_RETRY_MS = 2_000;
  const TERM_INPUT_RETRY_PAUSE_MS = 25;
  type PendingInput = {
    encoded: Uint8Array;
    resolve: (accepted: boolean) => void;
  };
  type InputQueue = {
    items: PendingInput[];
    bytes: number;
    running: boolean;
    stopped: boolean;
    cancelWait?: () => void;
  };
  const inputQueues = new Map<string, InputQueue>();
  const inputClosed = new Set<string>();
  const pendingKills = new Map<string, Promise<boolean>>();
  const termNotices = ref<Record<string, string>>({});
  const closingTerms = ref<Record<string, boolean>>({});
  const inputEncoder = new TextEncoder();
  const inputDecoder = new TextDecoder("utf-8", { fatal: true });

  function termNotice(id: string): string {
    return termNotices.value[id] ?? "";
  }
  function isTermClosing(id: string): boolean {
    return closingTerms.value[id] === true;
  }
  function setTermNotice(id: string, message: string, notify = true) {
    const previous = termNotices.value[id];
    termNotices.value[id] = message;
    if (notify && previous !== message) layout.showToast(message);
  }
  function cancelTermInput(id: string) {
    inputClosed.add(id);
    const queue = inputQueues.get(id);
    if (!queue) return;
    queue.stopped = true;
    queue.cancelWait?.();
    queue.cancelWait = undefined;
    for (const item of queue.items) item.resolve(false);
    queue.items = [];
    queue.bytes = 0;
  }
  function pauseInput(queue: InputQueue, ms: number): Promise<boolean> {
    if (queue.stopped) return Promise.resolve(false);
    return new Promise((resolve) => {
      let done = false;
      const finish = (running: boolean) => {
        if (done) return;
        done = true;
        window.clearTimeout(timer);
        queue.cancelWait = undefined;
        resolve(running);
      };
      const timer = window.setTimeout(() => finish(!queue.stopped), ms);
      queue.cancelWait = () => finish(false);
    });
  }
  function inputFailure(id: string, error: unknown) {
    const detail = String((error as { message?: unknown })?.message ?? error);
    cancelTermInput(id);
    if (isTermClosing(id)) return;
    if (detail.includes("TERM_INPUT_CLOSED") || detail.includes("终端不存在")) {
      setTermNotice(id, "终端已停止，本次输入未完整发送。可以重启终端后重试。");
    } else {
      setTermNotice(id, "输入未完整发送，后续输入已暂停。请核对终端内容并重启后重试。");
    }
  }
  async function drainInput(id: string, queue: InputQueue): Promise<void> {
    if (queue.running) return;
    queue.running = true;
    try {
      while (!queue.stopped && queue.items.length > 0) {
        const item = queue.items[0];
        let offset = 0;
        while (!queue.stopped && offset < item.encoded.length) {
          let end = Math.min(offset + TERM_INPUT_CHUNK_BYTES, item.encoded.length);
          // A UTF-8 continuation byte cannot start the next native IPC chunk.
          while (end < item.encoded.length && (item.encoded[end] & 0xc0) === 0x80) end -= 1;
          const chunk = inputDecoder.decode(item.encoded.subarray(offset, end));
          const deadline = Date.now() + TERM_INPUT_RETRY_MS;
          for (;;) {
            if (queue.stopped) return;
            try {
              await bridge.termWrite(id, chunk);
              break;
            } catch (error) {
              if (queue.stopped) return;
              const detail = String((error as { message?: unknown })?.message ?? error);
              const remaining = deadline - Date.now();
              if (!detail.includes("TERM_INPUT_QUEUE_FULL") || remaining <= 0) throw error;
              if (!await pauseInput(queue, Math.min(TERM_INPUT_RETRY_PAUSE_MS, remaining))) return;
            }
          }
          offset = end;
        }
        if (queue.stopped) return;
        queue.items.shift();
        queue.bytes -= item.encoded.length;
        item.resolve(true);
      }
    } catch (error) {
      // Already accepted chunks are never replayed: retry only the chunk for
      // which the backend explicitly refused queue admission.
      inputFailure(id, error);
    } finally {
      queue.running = false;
    }
  }

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
    const history = termHistories.get(id);
    if (history) appendOutput(history, data, TERM_HISTORY_MAX_BYTES, TERM_TEMP_HISTORY_LIMIT);
  }
  function clearTermHistory(id: string) {
    const slot = outputSessions.get(id);
    if (!slot) return;
    resetOutput(slot.history);
    updateReplayNotice(id, slot);
  }
  /** Replaying never feeds old history back into itself. Hidden output moves
   * into bounded history once, so another remount does not lose that output. */
  function replayTermHistory(id: string) {
    const slot = outputSessions.get(id);
    const writer = termWriters.get(id);
    if (!slot || slot.phase === "closed" || !writer) return;
    updateReplayNotice(id, slot);
    for (const chunk of slot.history.chunks) writer(chunk.text);
    const pending = slot.buffer.chunks;
    slot.buffer.chunks = [];
    slot.buffer.bytes = 0;
    // Keep truncation counters for this session. Clearing the queue must not
    // erase the explanation of output that was already omitted.
    for (const chunk of pending) {
      pushTermHistory(id, chunk.text);
      writer(chunk.text);
    }
    updateReplayNotice(id, slot);
    // Exit is a control frame, never saved as terminal byte history. Its
    // separate notice survives remounting even when it arrived before a writer.
  }
  // M3.a 丢弃统计（F2）：队列满时后端丢弃的字节数，仅展示不阻断。
  const droppedChunks = ref(0);
  const droppedBytes = ref(0);
  function resetDroppedStats() {
    droppedChunks.value = 0;
    droppedBytes.value = 0;
  }

  function bindTermWriter(id: string, fn: ((data: string) => void) | null) {
    if (fn && outputSessions.has(id)) termWriters.set(id, fn);
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
    window.setTimeout(() => { void termWrite(id, "\r"); }, 120);
  }

  function bindM0ThroughputStart(fn: (() => void) | null) {
    m0ThroughputStart = fn;
  }

  // 新建一个终端实例（PTY）。cwd 可选，传入则打开后 cd 到该目录。
  async function spawnTerm(cwd?: string): Promise<string> {
    if (!isTerminalResourceAllowed()) return "";
    if (outputSessions.size + startingSessions.size >= TERM_SESSION_LIMIT) {
      layout.showToast("终端数量已达上限，部分终端可能仍在启动或关闭。请稍后重试。");
      return "";
    }
    const slot: OutputSession = {
      phase: "starting", id: null, buffer: emptyOutput(), history: emptyOutput(),
      exit: null, droppedChunks: 0, droppedBytes: 0,
    };
    startingSessions.add(slot);
    try {
      const ch = bridge.createTermChannel((msg) => receiveSessionMessage(slot, msg));
      const r = await bridge.termSpawnChannel(ch);
      const id = r.id;
      const probe = !!r.probe;
      if (slot.phase === "closed" || !isTerminalResourceAllowed()
        || (slot.id !== null && slot.id !== id) || outputSessions.has(id)) {
        closeOutputSlot(slot);
        // The returned native session is still owned by its cleanup registry.
        // An aborted startup must not publish a new pane after suspension.
        try { await bridge.termKill(id); }
        catch { layout.showToast("终端启动已取消，资源仍在关闭。请稍后重试。"); }
        return "";
      }
      slot.id = id;
      slot.phase = "active";
      outputSessions.set(id, slot);
      // Preserve the exact buffers captured before termSpawnChannel resolved.
      termHistories.set(id, slot.history);
      termBuffers.set(id, slot.buffer);
      termPanes.value.push({ id, cwd });
      // Count this owner once after publication, even if cwd input awaits
      // bounded native admission while another pane is being opened.
      startingSessions.delete(slot);
      termProbeOn.value = probe;
      if (!activeTermId.value) activeTermId.value = id;
      resetDroppedStats();
      droppedChunks.value = slot.droppedChunks;
      droppedBytes.value = slot.droppedBytes;
      slot.droppedChunks = 0;
      slot.droppedBytes = 0;
      updateReplayNotice(id, slot);
      if (slot.exit) showExitNotice(id, slot.exit);
      if (cwd && !slot.exit) await termWrite(id, "cd " + cwd + "\n");
      if (!slot.exit) maybeRunM0Throughput(id);
      return id;
    } catch (error: unknown) {
      const ownedId = slot.id;
      if (ownedId && outputSessions.get(ownedId) === slot) {
        // Publication may already have happened before cwd/driver/notice
        // handling failed. Keep the pane and retry action if cleanup is pending.
        await killTerm(ownedId);
      } else {
        closeOutputSlot(slot);
        if (ownedId && !outputSessions.has(ownedId)) {
          try { await bridge.termKill(ownedId); }
          catch { layout.showToast("终端未能完成启动，资源仍在关闭。请稍后重试。"); }
        }
      }
      const detail = String((error as { message?: unknown })?.message ?? error);
      layout.showToast(detail.includes("TERM_SESSION_LIMIT")
        ? "终端数量已达上限，部分终端可能仍在关闭。请关闭现有终端或稍后重试。"
        : (detail.includes("TERM_SHUTTING_DOWN") || detail.includes("TERM_APPLICATION_SHUTTING_DOWN"))
          ? "应用正在关闭，未启动新的终端。"
          : "终端启动失败: " + detail);
      return "";
    } finally {
      startingSessions.delete(slot);
    }
  }
  // 确保终端视图至少有一个实例（终端视图组件挂载时调用）
  async function ensureTerm() {
    if (termPanes.value.length === 0) await spawnTerm();
  }
  function addTermPane() {
    spawnTerm();
  }
  function removeClosedTerm(id: string) {
    const slot = outputSessions.get(id);
    if (slot) closeOutputSlot(slot);
    outputSessions.delete(id);
    termPanes.value = termPanes.value.filter((pane) => pane.id !== id);
    termWriters.delete(id);
    termHistories.delete(id);
    termBuffers.delete(id);
    probeRecv.delete(id);
    confirmScans.delete(id);
    lastAutoConfirmAt.delete(id);
    delete termReplayNotices.value[id];
    inputQueues.delete(id);
    inputClosed.delete(id);
    delete termNotices.value[id];
    delete closingTerms.value[id];
    if (activeTermId.value === id) activeTermId.value = termPanes.value[0]?.id ?? "";
  }
  function killTerm(id: string): Promise<boolean> {
    const pending = pendingKills.get(id);
    if (pending) return pending;
    const slot = outputSessions.get(id);
    if (!slot) return Promise.resolve(true);
    slot.phase = "closing";
    // Stop accepting input before waiting for the native close command.
    // This also wakes any queue-full retry timer.
    cancelTermInput(id);
    closingTerms.value[id] = true;
    setTermNotice(id, "终端正在关闭…", false);
    const operation = (async () => {
      try {
        await bridge.termKill(id);
        removeClosedTerm(id);
        return true;
      } catch (error) {
        const detail = String((error as { message?: unknown })?.message ?? error);
        if (detail.includes("TERM_CLEANUP_COMPLETED_WITH_ERRORS:")) {
          // The native owner explicitly confirmed every resource is reaped.
          // Keep the warning visible without treating a historical error as a
          // permanently pending terminal.
          removeClosedTerm(id);
          layout.showToast("终端已关闭，关闭过程中曾出现异常。可以重新打开终端。");
          return true;
        }
        setTermNotice(id, detail.includes("TERM_CLEANUP_PENDING")
          ? "终端正在关闭，资源回收尚未完成。稍后可点击“检查关闭状态”。"
          : "终端关闭尚未完成。请再次检查关闭状态。");
        // Preserve the pane and its retry action until actual cleanup finishes.
        return false;
      } finally {
        pendingKills.delete(id);
      }
    })();
    pendingKills.set(id, operation);
    return operation;
  }

  // M0-0.b 终端吞吐（契约 §6.3）：term-throughput 模式下 shell 就绪后提交固定
  // 10 MiB ASCII 负载（含唯一 begin/end 标记）；计时起点为命令发送时刻。
  function maybeRunM0Throughput(id: string) {
    if (m0Cfg.value?.driver !== "term-throughput" || !id) return;
    setTimeout(() => {
      if (!m0ThroughputStart) return;
      m0ThroughputStart();
      m0StartTs.value = Date.now();
      void termWrite(
          id,
          "stty -echo; p='__M0_TERM_'; printf '%s' \"${p}BEGIN__\"; head -c 10485760 /dev/zero | tr '\\0' 'x'; printf '%s' \"${p}END__\"; stty echo\n"
        );
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
    await termWrite(tid, data);
  }

  function termWrite(id: string, data: string): Promise<boolean> {
    if (!id || !termPanes.value.some((pane) => pane.id === id)) return Promise.resolve(false);
    if (inputClosed.has(id) || isTermClosing(id)) return Promise.resolve(false);
    if (!data) return Promise.resolve(true);
    // Reject huge text before encoding; UTF-8 cannot be shorter than the
    // original UTF-16 code-unit count for these strings.
    if (data.length > TERM_INPUT_PENDING_BYTES) {
      setTermNotice(id, "有一段输入未发送：内容过长，请分成较小的段落后重试。");
      return Promise.resolve(false);
    }
    const encoded = inputEncoder.encode(data);
    let queue = inputQueues.get(id);
    if (!queue) {
      queue = { items: [], bytes: 0, running: false, stopped: false };
      inputQueues.set(id, queue);
    }
    if (queue.stopped) return Promise.resolve(false);
    if (encoded.length > TERM_INPUT_PENDING_BYTES
      || queue.bytes + encoded.length > TERM_INPUT_PENDING_BYTES
      || queue.items.length >= TERM_INPUT_PENDING_REQUESTS) {
      setTermNotice(id, "有一段输入未发送：等待发送的内容过多，请稍后分段重试。");
      return Promise.resolve(false);
    }
    const accepted = new Promise<boolean>((resolve) => {
      queue!.items.push({ encoded, resolve });
      queue!.bytes += encoded.length;
    });
    void drainInput(id, queue);
    return accepted;
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

  function onTermData(data: { id: string; data: string }) {
    // Legacy Event delivery may only address a live session owned by this
    // store. Unknown/closed IDs never allocate a buffer, notice or input tombstone.
    onTermChannelMsg({ ...data, kind: "data" });
  }

  function showExitNotice(id: string, msg: TermMessage) {
    cancelTermInput(id);
    if (!isTermClosing(id)) {
      const reason = String(msg.reason ?? "");
      setTermNotice(id, reason === "input_failed"
        ? "输入传输失败，部分输入可能未发送。请核对终端内容并重启后重试。"
        : reason === "eof" ? "终端进程已退出，可以重启终端。"
        : "终端输出连接已断开，请重启后重试。", reason !== "eof");
    }
  }

  function receiveSessionMessage(slot: OutputSession, msg: TermMessage) {
    if (slot.phase === "closed" || slot.phase === "closing") return;
    if (!msg.id || (slot.id !== null && slot.id !== msg.id)) return;
    slot.id ??= msg.id;
    const id = slot.id;
    if (slot.phase === "active" && outputSessions.get(id) !== slot) return;
    if (msg.kind === "flow") {
      const chunks = Math.max(0, Number.isFinite(msg.dropped_chunks) ? msg.dropped_chunks! : 0);
      const bytes = Math.max(0, Number.isFinite(msg.dropped_bytes) ? msg.dropped_bytes! : 0);
      if (slot.phase === "starting") {
        slot.droppedChunks = saturatingCount(slot.droppedChunks, chunks);
        slot.droppedBytes = saturatingCount(slot.droppedBytes, bytes);
      } else {
        droppedChunks.value = saturatingCount(droppedChunks.value, chunks);
        droppedBytes.value = saturatingCount(droppedBytes.value, bytes);
      }
      return;
    }
    if (slot.exit) return;
    const data = msg.data ?? "";
    if (msg.kind === "exit") {
      slot.exit = msg;
      if (slot.phase === "active") {
        showExitNotice(id, msg);
        probeFeRecv(id, data, "exit");
        termWriters.get(id)?.(data);
      }
      return;
    }
    const writer = slot.phase === "active" ? termWriters.get(id) : undefined;
    if (writer) {
      probeFeRecv(id, data, "xterm");
      pushTermHistory(id, data);
      writer(data);
      maybeAutoConfirmCli(id, data);
    } else {
      if (slot.phase === "active") probeFeRecv(id, data, "buffer");
      appendOutput(slot.buffer, data, TERM_BUFFER_MAX_BYTES, TERM_BUFFER_MAX_CHUNKS);
      if (slot.phase === "active") updateReplayNotice(id, slot);
    }
  }

  // Public/legacy routing only resolves already published owners. Startup data
  // goes through the private Channel closure's slot, not an unbounded ID map.
  function onTermChannelMsg(msg: TermMessage) {
    const slot = outputSessions.get(msg.id);
    if (slot) receiveSessionMessage(slot, msg);
  }

  registerTerminalLifecycleBinding({
    canSuspend: () => termPanes.value.length === 0 && startingSessions.size === 0,
    cleanup: () => {
      for (const slot of startingSessions) closeOutputSlot(slot);
      terminalOpen.value = false;
      activeTermId.value = "";
      resetDroppedStats();
      if (layout.mainView === "term") layout.setView("home");
    },
  });

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
    termNotice,
    termBufferNotice,
    isTermClosing,
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
