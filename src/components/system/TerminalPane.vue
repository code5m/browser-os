<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { bridge } from "../../bridge";
import { useSystemStore } from "../../stores/useSystemStore";
import { useTerminalResize } from "../../composables/useTerminalResize";

const props = withDefaults(defineProps<{ paneId: string; small?: boolean }>(), { small: false });
const system = useSystemStore();
const termEl = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;

// 终端对账 probe（默认关闭，开关经 spawn 响应下发）：只记元数据，不记终端正文。
const probeEncoder = new TextEncoder();
let probeWriteSeq = 0;
let probeSubmitBytes = 0;
let probeDoneSeq = 0;
let probeDoneBytes = 0;

// M3.c（WBS M3-4 · E2，沿用 M3.a F5 的真实行列上报）：
// - 前端 `fit()` **每次都调**，渲染实时跟随容器；
// - 后端 `term_resize` 走静默窗口：尺寸去重 + 140 ms 静默 + 500 ms 硬上界。
const resize = useTerminalResize((cols, rows) => {
  if (!term || !props.paneId) return;
  if (system.termProbeOn) {
    bridge.debugLog(`[TERM_PROBE] termResize.req pane=${props.paneId} cols=${cols} rows=${rows}`);
  }
  bridge.termResize(props.paneId, cols, rows).catch(() => {});
});

function onContainerResize() {
  if (!term || !fit) return;
  const dims = fit.proposeDimensions();
  if (system.termProbeOn) {
    bridge.debugLog(`[TERM_PROBE] resize.evt pane=${props.paneId} cw=${termEl.value?.clientWidth ?? -1} ch=${termEl.value?.clientHeight ?? -1} proposed=${dims ? `${dims.cols}x${dims.rows}` : "null"} term=${term.cols}x${term.rows} buf=${term.buffer.active.type}`);
  }
  if (!dims || dims.cols <= 0 || dims.rows <= 0) return;
  if (term.cols !== dims.cols || term.rows !== dims.rows) fit.fit();
  resize.notify(dims.cols, dims.rows);
}

// ====== M0-0.b 终端吞吐测量钩子（契约 §6.3） ======
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
  bridge.debugLog(`[TerminalPane] mounted paneId=${props.paneId}`);
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
      system.termWrite(props.paneId, data);
    });

    // 后端 PTY 输出 → xterm（M0-0.b 吞吐钩子在此拦截）
    system.bindTermWriter(props.paneId, (data) => {
      if (system.m0Cfg?.run_id) {
        bridge.debugLog(`[TerminalProbe] received chars=${data.length}`);
      }
      if (system.termProbeOn) {
        probeWriteSeq += 1;
        const b = probeEncoder.encode(data).length;
        probeSubmitBytes += b;
        bridge.debugLog(`[TERM_PROBE] xterm.write pane=${props.paneId} seq=${probeWriteSeq} bytes=${b} cum=${probeSubmitBytes} cols=${term?.cols} rows=${term?.rows} buf=${term?.buffer.active.type}`);
      }
      term?.write(data, () => {
        m0TrackRendered(data);
        if (system.m0Cfg?.run_id) {
          bridge.debugLog(`[TerminalProbe] rendered chars=${data.length} buffer=${term?.buffer.active.type}`);
        }
        if (system.termProbeOn) {
          probeDoneSeq += 1;
          probeDoneBytes += probeEncoder.encode(data).length;
          bridge.debugLog(`[TERM_PROBE] xterm.done pane=${props.paneId} seq=${probeDoneSeq} cum=${probeDoneBytes} cols=${term?.cols} rows=${term?.rows} buf=${term?.buffer.active.type}`);
        }
      });
    });
    system.bindM0ThroughputStart(m0Prepare);
    // M3.c（WBS M3-4 · E1）：面板重建（切 Dock / 切回）时回放最近 40 条输出，
    // 避免会话还活着但终端一片空白。历史仅会话内内存持有，不落盘。
    system.replayTermHistory(props.paneId);

    // 容器尺寸变化时 fit + 静默上报真实行列（M3.a F5 / M3.c E2）
    resizeObserver = new ResizeObserver(() => onContainerResize());
    resizeObserver.observe(termEl.value);

    bridge.debugLog("[TerminalPane] pane ready");
    if (system.termProbeOn) {
      bridge.debugLog(`[TERM_PROBE] pane.mounted pane=${props.paneId} cw=${termEl.value.clientWidth} ch=${termEl.value.clientHeight} cols=${term.cols} rows=${term.rows} buf=${term.buffer.active.type}`);
    }
    term.focus();
  } catch (e) {
    bridge.debugLog(`[TerminalPane] init error: ${e}`);
  }
});

function restart() {
  system.killTerm(props.paneId);
  system.addTermPane();
}
function closePane() {
  system.killTerm(props.paneId);
}

onBeforeUnmount(() => {
  if (system.termProbeOn) {
    bridge.debugLog(`[TERM_PROBE] pane.unmounted pane=${props.paneId} submitted=${probeSubmitBytes} completed=${probeDoneBytes}`);
  }
  m0FrameSampling = false;
  resize.dispose();
  resizeObserver?.disconnect();
  term?.dispose();
  system.bindTermWriter(props.paneId, null);
  system.bindM0ThroughputStart(null);
});
</script>

<template>
  <section
    class="terminal-xterm"
    @mousedown="system.setActiveTerm(props.paneId)"
    @focusin="system.setActiveTerm(props.paneId)"
  >
    <div class="term-head">
      <span>终端</span>
      <div>
        <label class="term-auto" title="仅自动确认 CodeArts CLI 的 True Color 兼容性提示">
          <input :checked="system.autoConfirmCli" type="checkbox" @change="system.setAutoConfirmCli(($event.target as HTMLInputElement).checked)" /> 自动确认 CLI
        </label>
        <span v-if="system.droppedBytes > 0" class="term-drop" title="输出过快，已丢弃的字节数">
          已丢弃 {{ system.droppedBytes }} B
        </span>
        <button v-if="!props.small" @click="restart" title="重启">↻</button>
        <button v-if="!props.small" @click="closePane" title="关闭进程">⏹</button>
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
  border: 1px solid #3a3a3a;
}
.term-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  padding: 4px 8px;
  background: #2d2d2d;
  color: #ccc;
  font-size: 12px;
  flex-shrink: 0;
  min-width: 0;
}
.term-head > span:first-child {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.term-head > div {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
}
.term-drop {
  color: #d19a66;
  font-size: 11px;
  margin-right: 6px;
  white-space: nowrap;
}
.term-auto { color: #bbb; font-size: 11px; margin-right: 6px; cursor: pointer; }
.term-auto input { vertical-align: middle; margin: 0 3px 0 0; }
.term-head button {
  border: none;
  background: transparent;
  color: #999;
  cursor: pointer;
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  margin-left: 2px;
  white-space: nowrap;
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
