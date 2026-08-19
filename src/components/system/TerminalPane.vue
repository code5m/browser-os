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

    // 后端 PTY 输出 → xterm
    system.bindTermWriter((data) => {
      term?.write(data);
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
