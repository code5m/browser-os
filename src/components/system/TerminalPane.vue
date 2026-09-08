<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { bridge } from "../../bridge";
import { useSystemStore } from "../../stores/useSystemStore";
import { useTerminalResize } from "../../composables/useTerminalResize";

const system = useSystemStore();
const termEl = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;

// M3.c（WBS M3-4 · E2，沿用 M3.a F5 的真实行列上报）：
// - 前端 `fit()` **每次都调**，渲染实时跟随容器；
// - 后端 `term_resize` 走静默窗口：尺寸去重 + 140 ms 静默 + 500 ms 硬上界，
//   拖动窗口不再每帧 invoke，也不会"持续慢拖时终端一直不重排"；
// - 失败一律 `.catch(() => {})`，高频路径不产生任何用户可见噪声。
const resize = useTerminalResize((cols, rows) => {
  if (!term || !system.termId) return;
  bridge.termResize(system.termId, cols, rows).catch(() => {});
});

function onContainerResize() {
  if (!term || !fit) return;
  const dims = fit.proposeDimensions();
  // 容器过渡态（面板折叠/隐藏时宽或高为 0）：不上报也不 fit，避免 0 值打到 PTY。
  if (!dims || dims.cols <= 0 || dims.rows <= 0) return;
  if (term.cols !== dims.cols || term.rows !== dims.rows) fit.fit();
  resize.notify(dims.cols, dims.rows);
}

// ====== M0-0.b 终端吞吐测量钩子（契约 logs/m0-baseline-contract-v1.md §6.3） ======
// 发送负载前启动 rAF 采样；xterm 完成 write 回调后再解析 begin/end 标记与有效载荷，
// 收到 end 并完成下一次 animation frame 后经 m0_term_report 上报。
const M0_BEGIN = "__M0_TERM_BEGIN__";
const M0_END = "__M0_TERM_END__";
let m0Armed = false;
let m0ScanBuf = "";
let m0PayloadCarry = "";
let m0Sampling = false;
let m0FrameSampling = false;
let m0Frames: number[] = [];
let m0LastTs = 0;
let m0Bytes = 0;
let m0BeginSeen = 0;
let m0EndSeen = 0;
const m0Encoder = new TextEncoder();

function m0Prepare() {
  m0Armed = true;
  m0ScanBuf = "";
  m0PayloadCarry = "";
  m0Sampling = false;
  m0Frames = [];
  m0Bytes = 0;
  m0BeginSeen = 0;
  m0EndSeen = 0;
  m0LastTs = performance.now();
  m0FrameSampling = true;
  const loop = () => {
    if (!m0FrameSampling) return;
    const now = performance.now();
    m0Frames.push(now - m0LastTs);
    m0LastTs = now;
    requestAnimationFrame(loop);
  };
  requestAnimationFrame(loop);
}

function m0ReportAfterPaint() {
  requestAnimationFrame(() => {
    m0FrameSampling = false;
    const frame_gaps = m0Frames.slice();
    const consumed_bytes = m0Bytes;
    const begin_seen = m0BeginSeen;
    const end_seen = m0EndSeen;
    const end_ts_ms = Date.now();
    const start_ts_ms = system.m0StartTs;
    m0Frames = [];
    m0Bytes = 0;
    bridge
      .m0TermReport({
        begin_seen,
        end_seen,
        consumed_bytes,
        start_ts_ms,
        end_ts_ms,
        frame_gaps_ms: frame_gaps,
      })
      .catch(() => {});
  });
}

function m0ConsumePayload(data: string) {
  const combined = m0PayloadCarry + data;
  const endIndex = combined.indexOf(M0_END);
  if (endIndex >= 0) {
    m0Bytes += m0Encoder.encode(combined.slice(0, endIndex)).length;
    m0EndSeen += 1;
    m0PayloadCarry = "";
    m0Sampling = false;
    m0Armed = false;
    m0ReportAfterPaint();
    return;
  }

  const carryLength = Math.min(M0_END.length - 1, combined.length);
  const countable = combined.slice(0, combined.length - carryLength);
  m0Bytes += m0Encoder.encode(countable).length;
  m0PayloadCarry = combined.slice(combined.length - carryLength);
}

function m0TrackRendered(data: string) {
  if (!m0Armed) return;
  if (!m0Sampling) {
    const combined = m0ScanBuf + data;
    const bi = combined.indexOf(M0_BEGIN);
    if (bi >= 0) {
      m0BeginSeen += 1;
      m0Sampling = true;
      m0ScanBuf = "";
      m0ConsumePayload(combined.slice(bi + M0_BEGIN.length));
    } else {
      m0ScanBuf = combined.slice(-(M0_BEGIN.length - 1));
    }
    return;
  }
  m0ConsumePayload(data);
}

onMounted(() => {
  bridge.debugLog(`[TerminalPane] mounted termEl=${!!termEl.value}`);
  if (!termEl.value) {
    bridge.debugLog("[TerminalPane] termEl is null");
    return;
  }
  try {
    term = new Terminal({
      cursorBlink: true,
      cursorStyle: "block",
      fontSize: 13,
      fontFamily: "Menlo, Monaco, 'Courier New', monospace",
      theme: {
        background: "#1e1e1e",
        foreground: "#d4d4d4",
        cursor: "#d4d4d4",
        selectionBackground: "#264f78",
      },
      allowProposedApi: true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(termEl.value);
    fit.fit();
    bridge.debugLog("[TerminalPane] xterm initialized");

    // 用户输入 → 后端 PTY
    term.onData((data) => {
      if (system.termId) system.termWrite(data);
    });

    // 后端 PTY 输出 → xterm（M0-0.b 吞吐钩子在此拦截）
    system.bindTermWriter((data) => {
      term?.write(data, () => m0TrackRendered(data));
    });
    system.bindM0ThroughputStart(m0Prepare);
    // M3.c（WBS M3-4 · E1）：面板重建（切 Dock / 切回）时回放最近 40 条输出，
    // 避免会话还活着但终端一片空白。历史仅会话内内存持有，不落盘。
    system.replayTermHistory();

    // 容器尺寸变化时 fit + 静默上报真实行列（M3.a F5 / M3.c E2）
    resizeObserver = new ResizeObserver(() => onContainerResize());
    resizeObserver.observe(termEl.value);

    // 新会话（首次启动 / ↻ 重启）：清去重态并立刻把当前真实行列报给新 PTY。
    // 新 PTY 默认 100×24，不补报会让 vim/top/less 一直按错误尺寸重排。
    watch(
      () => system.termId,
      (id) => {
        resize.reset();
        if (id) onContainerResize();
      }
    );

    // 启动 shell
    bridge.debugLog("[TerminalPane] calling startShell");
    system.startShell();
    term.focus();
  } catch (e) {
    bridge.debugLog(`[TerminalPane] init error: ${e}`);
  }
});

onBeforeUnmount(() => {
  m0FrameSampling = false;
  // 清掉在途的 resize 静默定时器，不留回调（用例 N5）。
  resize.dispose();
  resizeObserver?.disconnect();
  term?.dispose();
  system.bindTermWriter(null);
  system.bindM0ThroughputStart(null);
});
</script>

<template>
  <section class="terminal-xterm">
    <div class="term-head">
      <span>终端</span>
      <div>
        <span v-if="system.droppedBytes > 0" class="term-drop" title="输出过快，已丢弃的字节数">
          已丢弃 {{ system.droppedBytes }} B
        </span>
        <button @click="system.startShell(true)" title="重启">↻</button>
        <button @click="system.killShell" title="关闭进程">⏹</button>
        <button @click="system.terminalOpen = false" title="隐藏">✕</button>
      </div>
    </div>
    <div ref="termEl" class="term-container"></div>
  </section>
</template>

<style scoped>
.terminal-xterm {
  flex: 1;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: #1e1e1e;
}
.term-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  background: #2d2d2d;
  color: #ccc;
  font-size: 12px;
  flex-shrink: 0;
}
.term-drop {
  color: #d19a66;
  font-size: 11px;
  margin-right: 6px;
}
.term-head button {
  border: none;
  background: transparent;
  color: #999;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 12px;
  margin-left: 4px;
}
.term-head button:hover {
  background: #3d3d3d;
  color: #fff;
}
.term-container {
  flex: 1;
  height: 0;
  min-height: 0;
  padding: 2px;
  overflow: hidden;
}
</style>
