<script setup lang="ts">
// Terminal Dock 面板（Phase 8E / Train D）——原 Shell（MainArea.vue）Dock 终端分支整体迁入能力包。
// Dock 内终端：与终端主视图共享同一组 termPanes，竖向堆叠（避免小空间挤成宫格）。
// 保持迁移前的 v-if 语义：Dock 切到其它 Tab 即卸载 xterm；后端 PTY 会话仍活，切回时回放会话内历史。
import { useTerminalStore } from "../state/useTerminalStore";
import TerminalPane from "./TerminalPane.vue";

const terminal = useTerminalStore();
</script>

<template>
  <div class="dock-term-wrap">
    <div v-if="terminal.termPanes.length" class="term-grid dock-grid">
      <TerminalPane v-for="pane in terminal.termPanes" :key="pane.id" :paneId="pane.id" small />
    </div>
    <div v-else class="term-empty">
      <button @click="terminal.addTermPane()">＋ 新建终端</button>
    </div>
  </div>
</template>

<style scoped>
.dock-term-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.dock-term-wrap .term-grid.dock-grid {
  display: flex;
  flex-direction: column;
}
.dock-term-wrap .term-grid.dock-grid :deep(.terminal-xterm) {
  min-height: 120px;
}
.term-grid {
  flex: 1;
  min-height: 0;
  gap: 1px;
  padding: 1px;
  background: #252526;
}
.term-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #9aa4b2;
}
.term-empty button {
  border: 1px solid #3a3a3a;
  background: #2d2d2d;
  color: #ccc;
  padding: 6px 14px;
  border-radius: 4px;
  cursor: pointer;
}
</style>
