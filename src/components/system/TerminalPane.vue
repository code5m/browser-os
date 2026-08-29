<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from "vue";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { bridge } from "../../bridge";
import { useSystemStore } from "../../stores/useSystemStore";

const system = useSystemStore();
const termEl = ref<HTMLElement | null>(null);
let term: Terminal | null = null;
let fit: FitAddon | null = null;
let resizeObserver: ResizeObserver | null = null;

// ====== M0-0.b 终端吞吐测量钩子（契约 logs/m0-baseline-contract-v1.md §6.3） ======
// 检测 __M0_TERM_BEGIN__/__M0_TERM_END__ 标记：begin 后启动 rAF 帧间隔采样，
// 收到 end 并完成下一次 animation frame 后经 m0_term_report 上报。日常输出无标记，
// 除一次 indexOf 外零开销。
let m0Buf = "";
let m0Sampling = false;
let m0Frames: number[] = [];
let m0LastTs = 0;
let m0Bytes = 0;
const m0Encoder = new TextEncoder();

function m0Track(data: string) {
  if (!m0Sampling) {
    m0Buf += data;
    const bi = m0Buf.indexOf("__M0_TERM_BEGIN__");
    if (bi >= 0) {
      m0Sampling = true;
      m0Bytes = m0Encoder.encode(m0Buf.slice(bi + "__M0_TERM_BEGIN__".length)).length;
      m0Frames = [];
      m0LastTs = performance.now();
      const loop = () => {
        if (!m0Sampling) return;
        const now = performance.now();
        m0Frames.push(now - m0LastTs);
        m0LastTs = now;
        requestAnimationFrame(loop);
      };
      requestAnimationFrame(loop);
    } else if (m0Buf.length > 4096) {
      m0Buf = m0Buf.slice(-4096);
    }
    return;
  }
  m0Bytes += m0Encoder.encode(data).length;
  m0Buf += data;
  const ei = m0Buf.indexOf("__M0_TERM_END__");
  if (ei >= 0) {
    m0Sampling = false;
    m0Buf = "";
    // 计时终点：收到 end 标记并完成下一次 animation frame（不是后端读完 PTY）
    requestAnimationFrame(() => {
      const frame_gaps = m0Frames.slice();
      const consumed_bytes = m0Bytes;
      const end_ts_ms = Date.now();
      const start_ts_ms = system.m0StartTs;
      m0Frames = [];
      m0Bytes = 0;
      bridge
        .m0TermReport({
          run_id: "",
          begin_seen: 1,
          end_seen: 1,
          consumed_bytes,
          start_ts_ms,
          end_ts_ms,
          frame_gaps_ms: frame_gaps,
        })
        .catch(() => {});
    });
  } else if (m0Buf.length > 4096) {
    m0Buf = m0Buf.slice(-4096);
  }
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
      term?.write(data);
      m0Track(data);
    });

    // 容器尺寸变化时自动 fit
    resizeObserver = new ResizeObserver(() => {
      fit?.fit();
    });
    resizeObserver.observe(termEl.value);

    // 启动 shell
    bridge.debugLog("[TerminalPane] calling startShell");
    system.startShell();
    term.focus();
  } catch (e) {
    bridge.debugLog(`[TerminalPane] init error: ${e}`);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  term?.dispose();
  system.bindTermWriter(null);
});
</script>

<template>
  <section class="terminal-xterm">
    <div class="term-head">
      <span>终端</span>
      <div>
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
  min-height: 0;
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
  min-height: 0;
  padding: 2px;
}
</style>
