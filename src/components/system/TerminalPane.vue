<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { useSystemStore } from "../../stores/useSystemStore";
const system = useSystemStore();
const inputEl = ref<HTMLInputElement | null>(null);

// 终端视图激活时确保 shell 已启动并聚焦
watch(
  () => system.terminalOpen,
  (open) => {
    if (open) {
      nextTick(() => system.startShell());
      nextTick(() => inputEl.value?.focus());
    }
  },
  { immediate: true }
);
function ensureOpen() {
  if (!system.terminalOpen) system.toggleTerminal();
}
ensureOpen();
</script>

<template>
  <section class="terminal">
    <div class="term-head">
      <span>终端</span>
      <div>
        <button @click="system.startShell(true)" title="重启">↻</button>
        <button @click="system.killShell" title="关闭进程">⏹</button>
        <button @click="system.terminalOpen = false" title="隐藏">✕</button>
      </div>
    </div>
    <div id="termOut" class="term-out">{{ system.termLines.join("") }}</div>
    <div class="term-input">
      <span>$</span>
      <input
        ref="inputEl"
        v-model="system.termInput"
        @keydown="system.termKeydown"
        placeholder="真实终端：Tab 补全 / ↑↓ 历史 / Ctrl+C"
        autocomplete="off"
        spellcheck="false"
      />
    </div>
  </section>
</template>
