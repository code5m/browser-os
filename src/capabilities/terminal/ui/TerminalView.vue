<script setup lang="ts">
// Terminal 主视图（Phase 8E / Train D）——原 Shell（MainArea.vue）终端模块视图整体迁入能力包。
//
// 关键机制（C3 ABSENT 的支柱）：**PTY 的出生点在这一层**。
//   - 只有本组件被挂载（= Terminal 能力已注册且贡献已 activate）时，`ensureTerm()` 才可能被调用；
//   - Capability 未注册 → 无贡献 → 本组件不存在 → 不创建 PTY / 子进程 / 后端初始化。
//
// 显隐由本组件自管（v-show 绑定 layout.mainView），保持「切走不卸载 xterm」的既有行为
// （PTY 会话与 xterm 实例都保持存活；Dock 终端仍为 v-if 语义，行为不变）。
import { computed, watch } from "vue";
import { useLayoutStore } from "../../../stores/useLayoutStore";
import { useTerminalStore } from "../state/useTerminalStore";
import TerminalPane from "./TerminalPane.vue";

const layout = useLayoutStore();
const terminal = useTerminalStore();

// 终端宫格布局 class（单 / 2 / 4 / 9）
const gridClass = computed(() => {
  if (!terminal.termGrid) return "grid-off";
  return "grid-on cols-" + terminal.termGridCount;
});

// 进入终端视图时确保至少有一个终端实例（多实例宫格共享 termPanes）。
// 与迁移前 Shell 的 `watch(mainView, v => v==='term' && ensureTerm(), {immediate:true})` 语义一致。
watch(
  () => layout.mainView,
  (v) => {
    if (v === "term") terminal.ensureTerm();
  },
  { immediate: true },
);
</script>

<template>
  <div v-show="layout.mainView === 'term'" class="modview term-mod">
    <div class="term-toolbar">
      <span class="term-toolbar-title">终端</span>
      <button :class="{ active: !terminal.termGrid }" @click="terminal.setTermGrid(false)">单</button>
      <button
        :class="{ active: terminal.termGrid && terminal.termGridCount === 2 }"
        @click="terminal.setTermGridCount(2); terminal.setTermGrid(true)"
      >2</button>
      <button
        :class="{ active: terminal.termGrid && terminal.termGridCount === 4 }"
        @click="terminal.setTermGridCount(4); terminal.setTermGrid(true)"
      >4</button>
      <button
        :class="{ active: terminal.termGrid && terminal.termGridCount === 9 }"
        @click="terminal.setTermGridCount(9); terminal.setTermGrid(true)"
      >9</button>
      <span class="term-toolbar-spacer"></span>
      <button @click="terminal.addTermPane()" title="新建终端（追加一个实例）">＋ 终端</button>
    </div>
    <div v-if="terminal.termPanes.length" class="term-grid" :class="gridClass">
      <TerminalPane v-for="pane in terminal.termPanes" :key="pane.id" :paneId="pane.id" />
    </div>
    <div v-else class="term-empty">
      <p>还没有终端</p>
      <button @click="terminal.addTermPane()">＋ 新建终端</button>
    </div>
  </div>
</template>

<style scoped>
.term-mod {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.term-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  background: #252526;
  border-bottom: 1px solid #1b1b1b;
  flex-shrink: 0;
}
.term-toolbar-title {
  color: #ccc;
  font-size: 12px;
  margin-right: 4px;
}
.term-toolbar-spacer {
  flex: 1;
}
.term-toolbar button {
  border: none;
  background: #3a3a3a;
  color: #ccc;
  cursor: pointer;
  padding: 2px 10px;
  border-radius: 3px;
  font-size: 12px;
  margin-left: 2px;
}
.term-toolbar button.active {
  background: #0e639c;
  color: #fff;
}
.term-toolbar button:hover {
  background: #4a4a4a;
}
.term-toolbar button.active:hover {
  background: #1177bb;
}
.term-grid {
  flex: 1;
  min-height: 0;
  display: grid;
  gap: 1px;
  padding: 1px;
  background: #252526;
}
.term-grid.grid-off {
  grid-template-columns: 1fr;
  grid-template-rows: 1fr;
}
.term-grid.grid-on.cols-2 {
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr;
}
.term-grid.grid-on.cols-4 {
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr 1fr;
}
.term-grid.grid-on.cols-9 {
  grid-template-columns: 1fr 1fr 1fr;
  grid-template-rows: 1fr 1fr 1fr;
}
.term-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
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
.term-empty button:hover {
  background: #3a3a3a;
}
</style>
